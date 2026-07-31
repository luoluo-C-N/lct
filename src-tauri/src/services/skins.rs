use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use image::{
    imageops::FilterType, DynamicImage, GenericImageView, ImageError, ImageFormat, ImageReader,
    Limits, Rgba,
};
use serde::Deserialize;
use thiserror::Error;
use zip::ZipArchive;

use crate::{
    domain::companion::{CompanionSkin, SkinSource, VisualPreset},
    repository::companion::{CompanionRepository, CompanionRepositoryError},
};

const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_ZIP_BYTES: u64 = 20 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 40 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 16;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MIN_DIMENSION: u32 = 128;
const MAX_DIMENSION: u32 = 4096;
const MAX_TEXTURE_DIMENSION: u32 = 1024;
const MAX_PREVIEW_DIMENSION: u32 = 256;
const MAX_DECODE_ALLOCATION: u64 = 32 * 1024 * 1024;
const MIN_VISIBLE_ALPHA: f32 = 0.2;
const FALLBACK_COLOR: &str = "#666666";
static IMPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum SkinImportError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] ImageError),
    #[error(transparent)]
    Repository(#[from] CompanionRepositoryError),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("unsupported skin image format: {0}")]
    UnsupportedFormat(String),
    #[error("source image is too large: {bytes} bytes (maximum {maximum} bytes)")]
    SourceSize { bytes: u64, maximum: u64 },
    #[error("image dimensions {width}x{height} must be between {minimum} and {maximum}")]
    Dimensions {
        width: u32,
        height: u32,
        minimum: u32,
        maximum: u32,
    },
    #[error("skin import failed ({operation}) and compensation failed: {cleanup}")]
    Compensation { operation: String, cleanup: String },
    #[error("invalid skin package: {0:?}")]
    Package(SkinImportErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinImportErrorKind {
    UnsafePath,
    Symlink,
    ArchiveTooLarge,
    TooManyEntries,
    ManifestTooLarge,
    UnsupportedEntry,
    UnreferencedEntry,
    UnsupportedVersion,
    RemoteUrl,
    InvalidColor,
    InvalidMotion,
    MissingTexture,
    InvalidDimensions,
    InvalidManifest,
}

impl SkinImportError {
    pub fn kind(&self) -> Option<SkinImportErrorKind> {
        match self {
            Self::Package(kind) => Some(*kind),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinPackageManifest {
    version: u32,
    name: String,
    texture: String,
    flow_colors: Vec<String>,
    flow_speed: serde_json::Value,
    flow_intensity: serde_json::Value,
}

pub fn import_zip_skin(
    path: &Path,
    data_dir: &Path,
    repository: &CompanionRepository,
) -> Result<CompanionSkin, SkinImportError> {
    validate_source_size(path, MAX_ZIP_BYTES).map_err(|error| match error {
        SkinImportError::SourceSize { .. } => package_error(SkinImportErrorKind::ArchiveTooLarge),
        other => other,
    })?;

    let entries = read_validated_zip_entries(path)?;
    let manifest_bytes = entries
        .get(Path::new("manifest.json"))
        .ok_or_else(|| package_error(SkinImportErrorKind::InvalidManifest))?;
    let manifest: SkinPackageManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|_| package_error(SkinImportErrorKind::InvalidManifest))?;
    validate_package_manifest(&manifest, &entries)?;

    let texture_path = PathBuf::from(&manifest.texture);
    let texture_bytes = entries
        .get(&texture_path)
        .ok_or_else(|| package_error(SkinImportErrorKind::MissingTexture))?;
    let decoded = decode_package_image(texture_bytes)?;
    validate_dimensions(decoded.dimensions())
        .map_err(|_| package_error(SkinImportErrorKind::InvalidDimensions))?;
    let square = center_crop_square(decoded);
    let flow_speed = manifest
        .flow_speed
        .as_f64()
        .ok_or_else(|| package_error(SkinImportErrorKind::InvalidMotion))?
        .clamp(0.5, 2.0) as f32;
    let flow_intensity = manifest
        .flow_intensity
        .as_f64()
        .ok_or_else(|| package_error(SkinImportErrorKind::InvalidMotion))?
        .clamp(0.0, 1.0) as f32;

    fs::create_dir_all(data_dir.join("skins"))?;
    let mut storage = FilesystemSkinImportStorage { repository };
    persist_normalized_skin_with_storage(
        square,
        [
            manifest.flow_colors[0].clone(),
            manifest.flow_colors[1].clone(),
        ],
        path,
        Some(&manifest.name),
        SkinSource::Package,
        flow_speed,
        flow_intensity,
        data_dir,
        &mut storage,
    )
}

pub fn import_image_skin(
    path: &Path,
    name: Option<&str>,
    data_dir: &Path,
    repository: &CompanionRepository,
) -> Result<CompanionSkin, SkinImportError> {
    validate_source_size(path, MAX_IMAGE_BYTES)?;
    let decoded = decode_source(path)?;
    validate_dimensions(decoded.dimensions())?;

    let square = center_crop_square(decoded);
    let colors = derive_flow_colors(&square);
    fs::create_dir_all(data_dir.join("skins"))?;
    let mut storage = FilesystemSkinImportStorage { repository };
    persist_normalized_skin_with_storage(
        square,
        colors,
        path,
        name,
        SkinSource::Image,
        1.0,
        0.7,
        data_dir,
        &mut storage,
    )
}

fn read_validated_zip_entries(path: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, SkinImportError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(package_error(SkinImportErrorKind::TooManyEntries));
    }

    let mut entries = BTreeMap::new();
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry
            .enclosed_name()
            .ok_or_else(|| package_error(SkinImportErrorKind::UnsafePath))?;
        if entry.is_symlink() {
            return Err(package_error(SkinImportErrorKind::Symlink));
        }
        if !entry.is_file() {
            return Err(package_error(SkinImportErrorKind::UnsupportedEntry));
        }
        if name.components().count() != 1 {
            return Err(package_error(SkinImportErrorKind::UnsupportedEntry));
        }
        if !is_supported_package_entry(&name) {
            return Err(package_error(SkinImportErrorKind::UnsupportedEntry));
        }
        if entries.contains_key(&name) {
            return Err(package_error(SkinImportErrorKind::UnsupportedEntry));
        }
        if name == Path::new("manifest.json") && entry.size() > MAX_MANIFEST_BYTES {
            return Err(package_error(SkinImportErrorKind::ManifestTooLarge));
        }

        total_uncompressed = total_uncompressed
            .checked_add(entry.size())
            .ok_or_else(|| package_error(SkinImportErrorKind::ArchiveTooLarge))?;
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(package_error(SkinImportErrorKind::ArchiveTooLarge));
        }

        let remaining = MAX_UNCOMPRESSED_BYTES - (total_uncompressed - entry.size());
        let mut contents = Vec::new();
        entry
            .by_ref()
            .take(remaining + 1)
            .read_to_end(&mut contents)?;
        if contents.len() as u64 > remaining {
            return Err(package_error(SkinImportErrorKind::ArchiveTooLarge));
        }
        if name == Path::new("manifest.json") && contents.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(package_error(SkinImportErrorKind::ManifestTooLarge));
        }
        entries.insert(name, contents);
    }
    Ok(entries)
}

fn is_supported_package_entry(path: &Path) -> bool {
    if path == Path::new("manifest.json") {
        return true;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(extension) if extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("webp")
    )
}

fn validate_package_manifest(
    manifest: &SkinPackageManifest,
    entries: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), SkinImportError> {
    if manifest.version != 1 {
        return Err(package_error(SkinImportErrorKind::UnsupportedVersion));
    }
    if manifest.texture.contains("://") {
        return Err(package_error(SkinImportErrorKind::RemoteUrl));
    }
    let texture = PathBuf::from(&manifest.texture);
    if texture.is_absolute()
        || texture.components().count() != 1
        || !is_supported_package_entry(&texture)
        || texture == Path::new("manifest.json")
    {
        return Err(package_error(SkinImportErrorKind::UnsafePath));
    }
    if !entries.contains_key(&texture) {
        return Err(package_error(SkinImportErrorKind::MissingTexture));
    }
    let referenced = BTreeSet::from([PathBuf::from("manifest.json"), texture]);
    if entries.keys().any(|entry| !referenced.contains(entry)) {
        return Err(package_error(SkinImportErrorKind::UnreferencedEntry));
    }
    if manifest.flow_colors.len() != 2
        || manifest
            .flow_colors
            .iter()
            .any(|color| !is_valid_hex_color(color))
    {
        return Err(package_error(SkinImportErrorKind::InvalidColor));
    }
    if manifest.flow_speed.as_f64().is_none() || manifest.flow_intensity.as_f64().is_none() {
        return Err(package_error(SkinImportErrorKind::InvalidMotion));
    }
    Ok(())
}

fn is_valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

fn decode_package_image(bytes: &[u8]) -> Result<DynamicImage, SkinImportError> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::WebP) => {}
        _ => return Err(package_error(SkinImportErrorKind::UnsupportedEntry)),
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    reader.limits(limits);
    Ok(reader.decode()?)
}

fn package_error(kind: SkinImportErrorKind) -> SkinImportError {
    SkinImportError::Package(kind)
}

pub fn derive_flow_colors(image: &DynamicImage) -> [String; 2] {
    let (red, green, blue, count) = alpha_aware_linear_samples(image).into_iter().fold(
        (0.0_f32, 0.0_f32, 0.0_f32, 0_u32),
        |(red, green, blue, count), sample| {
            (
                red + sample[0],
                green + sample[1],
                blue + sample[2],
                count + 1,
            )
        },
    );

    if count == 0 {
        return [FALLBACK_COLOR.to_owned(), "#999999".to_owned()];
    }

    let primary = Rgba([
        linear_to_srgb(red / count as f32),
        linear_to_srgb(green / count as f32),
        linear_to_srgb(blue / count as f32),
        255,
    ]);
    let (hue, saturation, luminance) = rgb_to_hsl(primary);
    let saturation = saturation.clamp(0.0, 0.85);
    let luminance = luminance.clamp(0.2, 0.75);
    let companion_luminance = (luminance + 0.2).min(0.9);

    [
        to_hex(hsl_to_rgb(hue, saturation, luminance)),
        to_hex(hsl_to_rgb(hue, saturation, companion_luminance)),
    ]
}

fn validate_source_size(path: &Path, maximum: u64) -> Result<(), SkinImportError> {
    let bytes = fs::metadata(path)?.len();
    if bytes > maximum {
        return Err(SkinImportError::SourceSize { bytes, maximum });
    }
    Ok(())
}

fn decode_source(path: &Path) -> Result<DynamicImage, SkinImportError> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::WebP) => {}
        _ => {
            return Err(SkinImportError::UnsupportedFormat(
                path.display().to_string(),
            ))
        }
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    reader.limits(limits);
    Ok(reader.decode()?)
}

fn validate_dimensions((width, height): (u32, u32)) -> Result<(), SkinImportError> {
    if width < MIN_DIMENSION
        || height < MIN_DIMENSION
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
    {
        return Err(SkinImportError::Dimensions {
            width,
            height,
            minimum: MIN_DIMENSION,
            maximum: MAX_DIMENSION,
        });
    }
    Ok(())
}

fn center_crop_square(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();
    let side = width.min(height);
    image.crop_imm((width - side) / 2, (height - side) / 2, side, side)
}

fn persist_normalized_skin_with_storage(
    square: DynamicImage,
    colors: [String; 2],
    source_path: &Path,
    name: Option<&str>,
    source: SkinSource,
    flow_speed: f32,
    flow_intensity: f32,
    data_dir: &Path,
    storage: &mut impl SkinImportStorage,
) -> Result<CompanionSkin, SkinImportError> {
    let skin_id = next_skin_id();
    let skin_directory = data_dir.join("skins");
    let final_directory = skin_directory.join(&skin_id);
    let temporary_directory = skin_directory.join(format!(".{skin_id}.tmp"));
    storage.create_directory(&temporary_directory)?;

    let texture_path = temporary_directory.join("texture.png");
    let preview_path = temporary_directory.join("preview.png");
    let texture = resize_down_to_limit(&square, MAX_TEXTURE_DIMENSION);
    let preview = resize_down_to_limit(&square, MAX_PREVIEW_DIMENSION);

    if let Err(error) = storage
        .save_png(&texture, &texture_path)
        .and_then(|_| storage.save_png(&preview, &preview_path))
    {
        return return_after_compensation(
            storage,
            None,
            &temporary_directory,
            &final_directory,
            error,
        );
    }

    let skin = CompanionSkin {
        id: skin_id,
        name: name
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| default_skin_name(source_path)),
        source,
        visual_preset: VisualPreset::Custom,
        texture_path: Some(final_directory.join("texture.png")),
        preview_path: Some(final_directory.join("preview.png")),
        flow_colors: colors.into(),
        flow_speed,
        flow_intensity,
        created_at: Utc::now(),
    };

    if let Err(error) = storage.create_skin(&skin) {
        return return_after_compensation(
            storage,
            None,
            &temporary_directory,
            &final_directory,
            error,
        );
    }

    if let Err(error) = storage.rename_directory(&temporary_directory, &final_directory) {
        return return_after_compensation(
            storage,
            Some(&skin.id),
            &temporary_directory,
            &final_directory,
            error,
        );
    }

    Ok(skin)
}

fn resize_down_to_limit(image: &DynamicImage, maximum: u32) -> DynamicImage {
    if image.width() <= maximum && image.height() <= maximum {
        image.clone()
    } else {
        image.resize(maximum, maximum, FilterType::Lanczos3)
    }
}

#[cfg(test)]
pub(crate) fn persist_normalized_skin_with_storage_for_test(
    square: DynamicImage,
    source_path: &Path,
    data_dir: &Path,
    storage: &mut impl SkinImportStorage,
) -> Result<CompanionSkin, SkinImportError> {
    let colors = derive_flow_colors(&square);
    persist_normalized_skin_with_storage(
        square,
        colors,
        source_path,
        None,
        SkinSource::Image,
        1.0,
        0.7,
        data_dir,
        storage,
    )
}

pub(crate) trait SkinImportStorage {
    fn create_directory(&mut self, path: &Path) -> Result<(), SkinImportError>;
    fn save_png(&mut self, image: &DynamicImage, path: &Path) -> Result<(), SkinImportError>;
    fn create_skin(&mut self, skin: &CompanionSkin) -> Result<(), SkinImportError>;
    fn rollback_skin(&mut self, skin_id: &str) -> Result<(), SkinImportError>;
    fn rename_directory(
        &mut self,
        temporary_directory: &Path,
        final_directory: &Path,
    ) -> Result<(), SkinImportError>;
    fn remove_directory(&mut self, path: &Path) -> Result<(), SkinImportError>;
}

struct FilesystemSkinImportStorage<'a> {
    repository: &'a CompanionRepository,
}

impl SkinImportStorage for FilesystemSkinImportStorage<'_> {
    fn create_directory(&mut self, path: &Path) -> Result<(), SkinImportError> {
        fs::create_dir(path)?;
        Ok(())
    }

    fn save_png(&mut self, image: &DynamicImage, path: &Path) -> Result<(), SkinImportError> {
        image.save(path)?;
        Ok(())
    }

    fn create_skin(&mut self, skin: &CompanionSkin) -> Result<(), SkinImportError> {
        self.repository.create_skin(skin)?;
        Ok(())
    }

    fn rollback_skin(&mut self, skin_id: &str) -> Result<(), SkinImportError> {
        self.repository.rollback_imported_skin(skin_id)?;
        Ok(())
    }

    fn rename_directory(
        &mut self,
        temporary_directory: &Path,
        final_directory: &Path,
    ) -> Result<(), SkinImportError> {
        fs::rename(temporary_directory, final_directory)?;
        Ok(())
    }

    fn remove_directory(&mut self, path: &Path) -> Result<(), SkinImportError> {
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }
}

fn return_after_compensation(
    storage: &mut impl SkinImportStorage,
    skin_id: Option<&str>,
    temporary_directory: &Path,
    final_directory: &Path,
    operation_error: SkinImportError,
) -> Result<CompanionSkin, SkinImportError> {
    match compensate_import(storage, skin_id, temporary_directory, final_directory) {
        Ok(()) => Err(operation_error),
        Err(cleanup) => Err(SkinImportError::Compensation {
            operation: operation_error.to_string(),
            cleanup: cleanup.to_string(),
        }),
    }
}

fn compensate_import(
    storage: &mut impl SkinImportStorage,
    skin_id: Option<&str>,
    temporary_directory: &Path,
    final_directory: &Path,
) -> Result<(), SkinImportError> {
    let rollback_result = skin_id
        .map(|id| rollback_with_retry(storage, id))
        .transpose();
    let temporary_result = storage.remove_directory(temporary_directory);
    let final_result = storage.remove_directory(final_directory);
    rollback_result.and(temporary_result).and(final_result)
}

fn rollback_with_retry(
    storage: &mut impl SkinImportStorage,
    skin_id: &str,
) -> Result<(), SkinImportError> {
    match storage.rollback_skin(skin_id) {
        Ok(()) => Ok(()),
        Err(_) => storage.rollback_skin(skin_id),
    }
}

fn alpha_aware_linear_samples(image: &DynamicImage) -> Vec<[f32; 3]> {
    const PALETTE_SIZE: u32 = 16;
    let pixels = image.to_rgba8();
    let (width, height) = pixels.dimensions();
    let mut buckets = vec![[0.0_f32; 5]; (PALETTE_SIZE * PALETTE_SIZE) as usize];

    for (x, y, pixel) in pixels.enumerate_pixels() {
        let bucket_x = x * PALETTE_SIZE / width;
        let bucket_y = y * PALETTE_SIZE / height;
        let bucket = &mut buckets[(bucket_y * PALETTE_SIZE + bucket_x) as usize];
        let alpha = pixel[3] as f32 / 255.0;
        bucket[0] += srgb_to_linear(pixel[0]) * alpha;
        bucket[1] += srgb_to_linear(pixel[1]) * alpha;
        bucket[2] += srgb_to_linear(pixel[2]) * alpha;
        bucket[3] += alpha;
        bucket[4] += 1.0;
    }

    buckets
        .into_iter()
        .filter_map(|bucket| {
            let alpha = bucket[3] / bucket[4];
            (alpha >= MIN_VISIBLE_ALPHA && bucket[3] > 0.0).then(|| {
                [
                    bucket[0] / bucket[3],
                    bucket[1] / bucket[3],
                    bucket[2] / bucket[3],
                ]
            })
        })
        .collect()
}

fn default_skin_name(source_path: &Path) -> String {
    source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("Imported skin")
        .to_owned()
}

fn next_skin_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before UNIX_EPOCH")
        .as_nanos();
    let sequence = IMPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("imported-{timestamp:x}-{sequence:x}")
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round() as u8
}

fn rgb_to_hsl(color: Rgba<u8>) -> (f32, f32, f32) {
    let red = color[0] as f32 / 255.0;
    let green = color[1] as f32 / 255.0;
    let blue = color[2] as f32 / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let chroma = maximum - minimum;
    let luminance = (maximum + minimum) / 2.0;
    if chroma == 0.0 {
        return (0.0, 0.0, luminance);
    }

    let saturation = chroma / (1.0 - (2.0 * luminance - 1.0).abs());
    let hue = if maximum == red {
        60.0 * ((green - blue) / chroma).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * ((blue - red) / chroma + 2.0)
    } else {
        60.0 * ((red - green) / chroma + 4.0)
    };
    (hue, saturation, luminance)
}

fn hsl_to_rgb(hue: f32, saturation: f32, luminance: f32) -> Rgba<u8> {
    let chroma = (1.0 - (2.0 * luminance - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let secondary = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector as u32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let match_value = luminance - chroma / 2.0;
    Rgba([
        ((red + match_value).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((green + match_value).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((blue + match_value).clamp(0.0, 1.0) * 255.0).round() as u8,
        255,
    ])
}

fn to_hex(color: Rgba<u8>) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}
