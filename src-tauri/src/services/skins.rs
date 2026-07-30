use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageError, Rgba};
use thiserror::Error;

use crate::{
    domain::companion::{CompanionSkin, SkinSource, VisualPreset},
    repository::companion::{CompanionRepository, CompanionRepositoryError},
};

const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MIN_DIMENSION: u32 = 128;
const MAX_DIMENSION: u32 = 4096;
const MAX_TEXTURE_DIMENSION: u32 = 1024;
const MAX_PREVIEW_DIMENSION: u32 = 256;
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
}

pub fn import_image_skin(
    path: &Path,
    name: Option<&str>,
    data_dir: &Path,
    repository: &CompanionRepository,
) -> Result<CompanionSkin, SkinImportError> {
    validate_source_size(path, MAX_IMAGE_BYTES)?;
    validate_source_format(path)?;
    let decoded = image::open(path)?;
    validate_dimensions(decoded.dimensions())?;

    let square = center_crop_square(decoded);
    let colors = derive_flow_colors(&square);
    persist_normalized_skin(square, colors, path, name, data_dir, repository)
}

pub fn derive_flow_colors(image: &DynamicImage) -> [String; 2] {
    let sampled = image.resize_exact(16, 16, FilterType::Triangle).to_rgba8();
    let (red, green, blue, count) = sampled.pixels().fold(
        (0.0_f32, 0.0_f32, 0.0_f32, 0_u32),
        |(red, green, blue, count), pixel| {
            if pixel[3] as f32 / 255.0 < MIN_VISIBLE_ALPHA {
                return (red, green, blue, count);
            }
            (
                red + srgb_to_linear(pixel[0]),
                green + srgb_to_linear(pixel[1]),
                blue + srgb_to_linear(pixel[2]),
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

fn validate_source_format(path: &Path) -> Result<(), SkinImportError> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension)
            if extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("webp") =>
        {
            Ok(())
        }
        _ => Err(SkinImportError::UnsupportedFormat(
            path.display().to_string(),
        )),
    }
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

fn persist_normalized_skin(
    square: DynamicImage,
    colors: [String; 2],
    source_path: &Path,
    name: Option<&str>,
    data_dir: &Path,
    repository: &CompanionRepository,
) -> Result<CompanionSkin, SkinImportError> {
    let skin_id = next_skin_id();
    let skin_directory = data_dir.join("skins");
    let final_directory = skin_directory.join(&skin_id);
    let temporary_directory = skin_directory.join(format!(".{skin_id}.tmp"));
    fs::create_dir_all(&skin_directory)?;
    fs::create_dir(&temporary_directory)?;

    let texture_path = temporary_directory.join("texture.png");
    let preview_path = temporary_directory.join("preview.png");
    let texture = square.resize(
        MAX_TEXTURE_DIMENSION,
        MAX_TEXTURE_DIMENSION,
        FilterType::Lanczos3,
    );
    let preview = square.resize(
        MAX_PREVIEW_DIMENSION,
        MAX_PREVIEW_DIMENSION,
        FilterType::Lanczos3,
    );

    if let Err(error) = texture
        .save(&texture_path)
        .and_then(|_| preview.save(&preview_path))
    {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error.into());
    }

    let skin = CompanionSkin {
        id: skin_id,
        name: name
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| default_skin_name(source_path)),
        source: SkinSource::Imported,
        visual_preset: VisualPreset::DeepInk,
        texture_path: Some(final_directory.join("texture.png")),
        preview_path: Some(final_directory.join("preview.png")),
        flow_colors: colors.into(),
        flow_speed: 1.0,
        flow_intensity: 0.7,
        created_at: Utc::now(),
    };

    if let Err(error) = repository.create_skin(&skin) {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error.into());
    }

    if let Err(error) = fs::rename(&temporary_directory, &final_directory) {
        let _ = repository.delete_skin(&skin.id);
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error.into());
    }

    Ok(skin)
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
