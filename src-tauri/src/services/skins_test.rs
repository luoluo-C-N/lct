use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use rusqlite::Connection;
use serde_json::json;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    domain::companion::{CompanionSkin, SkinSource},
    repository::companion::CompanionRepository,
    services::skins::{
        derive_flow_colors, import_image_skin, import_zip_skin,
        persist_normalized_skin_with_storage_for_test, SkinImportError, SkinImportErrorKind,
        SkinImportStorage,
    },
};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn imports_and_center_crops_a_rectangular_webp() {
    let fixture = SkinFixture::new();
    let source = fixture.write_image(
        "wide.webp",
        1600,
        800,
        ImageFormat::WebP,
        Rgba([128, 0, 128, 255]),
    );

    let skin = import_image_skin(
        &source,
        Some("Wide purple"),
        &fixture.data_directory,
        &fixture.repository,
    )
    .unwrap();

    assert_eq!(skin.name, "Wide purple");
    assert_eq!(skin.source, SkinSource::Image);
    assert!(skin.texture_path.as_ref().unwrap().ends_with("texture.png"));
    assert_eq!(
        image::open(skin.texture_path.unwrap())
            .unwrap()
            .dimensions(),
        (800, 800)
    );
    assert_eq!(
        image::open(skin.preview_path.unwrap())
            .unwrap()
            .dimensions(),
        (256, 256)
    );
    assert_eq!(fixture.repository.list_skins().unwrap().len(), 4);
}

#[test]
fn zip_imports_a_v1_package_after_full_validation() {
    let fixture = SkinFixture::new();
    let package = fixture.write_zip(
        "valid.zip",
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
    );

    let skin = import_zip_skin(&package, &fixture.data_directory, &fixture.repository).unwrap();

    assert_eq!(skin.name, "Lavender package");
    assert_eq!(skin.source, SkinSource::Package);
    assert_eq!(skin.flow_colors, vec!["#B79CFF", "#FFE4B5"]);
    assert_eq!(skin.flow_speed, 1.25);
    assert_eq!(skin.flow_intensity, 0.8);
    assert_eq!(
        image::open(skin.texture_path.unwrap())
            .unwrap()
            .dimensions(),
        (128, 128)
    );
    assert_eq!(fixture.repository.list_skins().unwrap().len(), 4);
}

#[test]
fn zip_rejects_path_traversal_before_persisting() {
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
            zip_file("../escape.png", valid_png_bytes(128, 128)),
        ],
        SkinImportErrorKind::UnsafePath,
    );
}

#[test]
fn zip_rejects_symlink_entries_before_persisting() {
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            TestZipEntry::Symlink("texture.webp".to_owned(), "../outside.webp".to_owned()),
        ],
        SkinImportErrorKind::Symlink,
    );
}

#[test]
fn zip_rejects_directory_entries_before_persisting() {
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
            TestZipEntry::Directory("nested/".to_owned()),
        ],
        SkinImportErrorKind::UnsupportedEntry,
    );
}

#[test]
fn zip_rejects_archives_over_the_compressed_size_limit() {
    let fixture = SkinFixture::new();
    let package = fixture.write_zip(
        "compressed-too-large.zip",
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
    );
    let file = OpenOptions::new().write(true).open(&package).unwrap();
    file.set_len(20 * 1024 * 1024 + 1).unwrap();

    fixture.assert_zip_rejected(&package, SkinImportErrorKind::ArchiveTooLarge);
}

#[test]
fn zip_rejects_archives_over_the_uncompressed_size_limit() {
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
            zip_file("unused.png", vec![0; 40 * 1024 * 1024]),
        ],
        SkinImportErrorKind::ArchiveTooLarge,
    );
}

#[test]
fn zip_rejects_archives_over_the_entry_count_limit() {
    let mut entries = vec![
        zip_file("manifest.json", valid_manifest("texture.webp")),
        zip_file("texture.webp", valid_webp_bytes(128, 128)),
    ];
    for index in 0..15 {
        entries.push(zip_file(
            &format!("unused-{index}.png"),
            valid_png_bytes(128, 128),
        ));
    }
    assert_zip_rejected(entries, SkinImportErrorKind::TooManyEntries);
}

#[test]
fn zip_rejects_manifests_over_the_size_limit() {
    let mut manifest = valid_manifest("texture.webp");
    manifest.resize(64 * 1024 + 1, b' ');
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", manifest),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
        SkinImportErrorKind::ManifestTooLarge,
    );
}

#[test]
fn zip_rejects_unsupported_and_unreferenced_entries() {
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
            zip_file("payload.html", b"<script>bad()</script>".to_vec()),
        ],
        SkinImportErrorKind::UnsupportedEntry,
    );
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
            zip_file("unused.png", valid_png_bytes(128, 128)),
        ],
        SkinImportErrorKind::UnreferencedEntry,
    );
}

#[test]
fn zip_rejects_unknown_manifest_versions_and_remote_textures() {
    let mut unknown_version = valid_manifest_value("texture.webp");
    unknown_version["version"] = json!(2);
    assert_zip_rejected(
        vec![
            zip_file(
                "manifest.json",
                serde_json::to_vec(&unknown_version).unwrap(),
            ),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
        SkinImportErrorKind::UnsupportedVersion,
    );

    assert_zip_rejected(
        vec![zip_file(
            "manifest.json",
            valid_manifest("https://attacker.invalid/texture.webp"),
        )],
        SkinImportErrorKind::RemoteUrl,
    );
}

#[test]
fn zip_rejects_invalid_colors_and_motion_fields() {
    let mut invalid_color = valid_manifest_value("texture.webp");
    invalid_color["flowColors"] = json!(["#B79CFF", "#NOTHEX"]);
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", serde_json::to_vec(&invalid_color).unwrap()),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
        SkinImportErrorKind::InvalidColor,
    );

    let mut invalid_speed = valid_manifest_value("texture.webp");
    invalid_speed["flowSpeed"] = json!("very fast");
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", serde_json::to_vec(&invalid_speed).unwrap()),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
        SkinImportErrorKind::InvalidMotion,
    );

    let mut invalid_intensity = valid_manifest_value("texture.webp");
    invalid_intensity["flowIntensity"] = json!(null);
    assert_zip_rejected(
        vec![
            zip_file(
                "manifest.json",
                serde_json::to_vec(&invalid_intensity).unwrap(),
            ),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
        SkinImportErrorKind::InvalidMotion,
    );
}

#[test]
fn zip_clamps_numeric_motion_fields_to_package_ranges() {
    let fixture = SkinFixture::new();
    let mut manifest = valid_manifest_value("texture.webp");
    manifest["flowSpeed"] = json!(9.0);
    manifest["flowIntensity"] = json!(-2.0);
    let package = fixture.write_zip(
        "clamped.zip",
        vec![
            zip_file("manifest.json", serde_json::to_vec(&manifest).unwrap()),
            zip_file("texture.webp", valid_webp_bytes(128, 128)),
        ],
    );

    let skin = import_zip_skin(&package, &fixture.data_directory, &fixture.repository).unwrap();

    assert_eq!(skin.flow_speed, 2.0);
    assert_eq!(skin.flow_intensity, 0.0);
}

#[test]
fn zip_rejects_missing_textures_and_invalid_decoded_dimensions() {
    assert_zip_rejected(
        vec![zip_file("manifest.json", valid_manifest("texture.webp"))],
        SkinImportErrorKind::MissingTexture,
    );
    assert_zip_rejected(
        vec![
            zip_file("manifest.json", valid_manifest("texture.webp")),
            zip_file("texture.webp", valid_webp_bytes(64, 64)),
        ],
        SkinImportErrorKind::InvalidDimensions,
    );
}

#[test]
fn rejects_oversized_or_too_small_images_without_leaving_files() {
    let fixture = SkinFixture::new();
    let too_small =
        fixture.write_image("64.png", 64, 64, ImageFormat::Png, Rgba([10, 20, 30, 255]));

    assert!(matches!(
        import_image_skin(
            &too_small,
            None,
            &fixture.data_directory,
            &fixture.repository
        ),
        Err(SkinImportError::Dimensions { .. })
    ));

    let oversized = fixture.root.join("oversized.png");
    fs::write(&oversized, vec![0; 10 * 1024 * 1024 + 1]).unwrap();
    assert!(matches!(
        import_image_skin(
            &oversized,
            None,
            &fixture.data_directory,
            &fixture.repository
        ),
        Err(SkinImportError::SourceSize { .. })
    ));

    fixture.assert_skin_directory_is_empty();
    assert_eq!(fixture.repository.list_skins().unwrap().len(), 3);
}

#[test]
fn derives_a_stable_safe_palette_for_purple() {
    assert_eq!(
        derive_flow_colors(&solid_image(Rgba([128, 0, 128, 255]))),
        ["#760A76".to_owned(), "#D511D5".to_owned()]
    );
}

#[test]
fn derives_a_stable_safe_palette_for_near_white() {
    assert_eq!(
        derive_flow_colors(&solid_image(Rgba([250, 250, 250, 255]))),
        ["#BFBFBF".to_owned(), "#E6E6E6".to_owned()]
    );
}

#[test]
fn derives_a_stable_safe_palette_for_near_black() {
    assert_eq!(
        derive_flow_colors(&solid_image(Rgba([4, 4, 4, 255]))),
        ["#333333".to_owned(), "#666666".to_owned()]
    );
}

#[test]
fn derives_a_stable_safe_palette_for_transparent_images() {
    assert_eq!(
        derive_flow_colors(&solid_image(Rgba([255, 0, 255, 0]))),
        ["#666666".to_owned(), "#999999".to_owned()]
    );
}

#[test]
fn ignores_transparent_saturated_pixels_when_deriving_flow_colors() {
    let mut pixels = RgbaImage::from_pixel(128, 128, Rgba([255, 0, 255, 0]));
    for y in 20..108 {
        for x in 20..108 {
            pixels.put_pixel(x, y, Rgba([0, 255, 0, 255]));
        }
    }

    assert_eq!(
        derive_flow_colors(&DynamicImage::ImageRgba8(pixels)),
        ["#13EC13".to_owned(), "#71F471".to_owned()]
    );
}

#[test]
fn rejects_header_dimensions_before_decoding_or_persisting() {
    let fixture = SkinFixture::new();
    let source = fixture.write_image(
        "header-too-wide.png",
        4097,
        128,
        ImageFormat::Png,
        Rgba([20, 40, 60, 255]),
    );

    assert!(matches!(
        import_image_skin(&source, None, &fixture.data_directory, &fixture.repository),
        Err(SkinImportError::Image(_))
    ));
    fixture.assert_skin_directory_is_empty();
    assert_eq!(fixture.repository.list_skins().unwrap().len(), 3);
}

#[test]
fn rejects_decodes_that_exceed_the_allocation_ceiling() {
    let fixture = SkinFixture::new();
    let source = fixture.write_image(
        "allocation-limit.png",
        4096,
        4096,
        ImageFormat::Png,
        Rgba([20, 40, 60, 255]),
    );

    assert!(matches!(
        import_image_skin(&source, None, &fixture.data_directory, &fixture.repository),
        Err(SkinImportError::Image(_))
    ));
    fixture.assert_skin_directory_is_empty();
    assert_eq!(fixture.repository.list_skins().unwrap().len(), 3);
}

#[test]
fn save_failure_leaves_no_skin_row_or_temporary_directory() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::Save);

    assert!(persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .is_err());

    storage.assert_empty();
}

#[test]
fn create_failure_leaves_no_skin_row_or_temporary_directory() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::Create);

    assert!(persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .is_err());

    storage.assert_empty();
}

#[test]
fn rename_failure_rolls_back_the_new_skin_row_and_temporary_directory() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::Rename);

    assert!(persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .is_err());

    storage.assert_empty();
}

#[test]
fn rename_failure_retries_a_transient_rollback_failure_before_returning() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::RollbackOnce);

    assert!(persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .is_err());

    storage.assert_empty();
}

#[test]
fn permanent_rollback_failure_returns_compensation_context() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::RollbackAlways);

    let error = persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .unwrap_err();

    assert_compensation(
        error,
        "injected rename failure",
        "injected rollback failure",
    );
}

#[test]
fn temporary_directory_cleanup_failure_returns_compensation_context() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::TemporaryCleanup);

    let error = persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .unwrap_err();

    assert_compensation(
        error,
        "injected rename failure",
        "injected temporary cleanup failure",
    );
}

#[test]
fn final_directory_cleanup_failure_returns_compensation_context() {
    let fixture = SkinFixture::new();
    let mut storage = TestSkinImportStorage::failing(TestFailure::FinalCleanup);

    let error = persist_normalized_skin_with_storage_for_test(
        solid_image(Rgba([10, 20, 30, 255])),
        &fixture.root.join("source.png"),
        &fixture.data_directory,
        &mut storage,
    )
    .unwrap_err();

    assert_compensation(
        error,
        "injected rename failure",
        "injected final cleanup failure",
    );
}

fn solid_image(color: Rgba<u8>) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(32, 32, color))
}

fn assert_zip_rejected(entries: Vec<TestZipEntry>, expected: SkinImportErrorKind) {
    let fixture = SkinFixture::new();
    let package = fixture.write_zip("malicious.zip", entries);
    fixture.assert_zip_rejected(&package, expected);
}

fn zip_file(name: &str, contents: Vec<u8>) -> TestZipEntry {
    TestZipEntry::File(name.to_owned(), contents)
}

fn valid_manifest(texture: &str) -> Vec<u8> {
    serde_json::to_vec(&valid_manifest_value(texture)).unwrap()
}

fn valid_manifest_value(texture: &str) -> serde_json::Value {
    json!({
        "version": 1,
        "name": "Lavender package",
        "texture": texture,
        "flowColors": ["#B79CFF", "#FFE4B5"],
        "flowSpeed": 1.25,
        "flowIntensity": 0.8
    })
}

fn valid_webp_bytes(width: u32, height: u32) -> Vec<u8> {
    image_bytes(width, height, ImageFormat::WebP)
}

fn valid_png_bytes(width: u32, height: u32) -> Vec<u8> {
    image_bytes(width, height, ImageFormat::Png)
}

fn image_bytes(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        width,
        height,
        Rgba([183, 156, 255, 255]),
    ))
    .write_to(&mut bytes, format)
    .unwrap();
    bytes.into_inner()
}

enum TestZipEntry {
    File(String, Vec<u8>),
    Symlink(String, String),
    Directory(String),
}

fn assert_compensation(error: SkinImportError, operation: &str, cleanup: &str) {
    match error {
        SkinImportError::Compensation {
            operation: actual_operation,
            cleanup: actual_cleanup,
        } => {
            assert!(actual_operation.contains(operation));
            assert!(actual_cleanup.contains(cleanup));
        }
        unexpected => panic!("expected compensation error, got {unexpected}"),
    }
}

#[derive(Clone, Copy)]
enum TestFailure {
    Save,
    Create,
    Rename,
    RollbackOnce,
    RollbackAlways,
    TemporaryCleanup,
    FinalCleanup,
}

struct TestSkinImportStorage {
    failure: TestFailure,
    temporary_directories: BTreeSet<PathBuf>,
    final_directories: BTreeSet<PathBuf>,
    skin_ids: BTreeSet<String>,
    rollback_has_failed: bool,
}

impl TestSkinImportStorage {
    fn failing(failure: TestFailure) -> Self {
        Self {
            failure,
            temporary_directories: BTreeSet::new(),
            final_directories: BTreeSet::new(),
            skin_ids: BTreeSet::new(),
            rollback_has_failed: false,
        }
    }

    fn assert_empty(&self) {
        assert!(self.temporary_directories.is_empty());
        assert!(self.final_directories.is_empty());
        assert!(self.skin_ids.is_empty());
    }
}

impl SkinImportStorage for TestSkinImportStorage {
    fn create_directory(&mut self, path: &std::path::Path) -> Result<(), SkinImportError> {
        self.temporary_directories.insert(path.to_owned());
        Ok(())
    }

    fn save_png(
        &mut self,
        _image: &DynamicImage,
        path: &std::path::Path,
    ) -> Result<(), SkinImportError> {
        if matches!(self.failure, TestFailure::Save) {
            return Err(std::io::Error::other("injected save failure").into());
        }
        self.temporary_directories.insert(path.to_owned());
        Ok(())
    }

    fn create_skin(&mut self, skin: &CompanionSkin) -> Result<(), SkinImportError> {
        if matches!(self.failure, TestFailure::Create) {
            return Err(std::io::Error::other("injected create failure").into());
        }
        self.skin_ids.insert(skin.id.clone());
        Ok(())
    }

    fn rollback_skin(&mut self, skin_id: &str) -> Result<(), SkinImportError> {
        if matches!(self.failure, TestFailure::RollbackAlways) {
            return Err(std::io::Error::other("injected rollback failure").into());
        }
        if matches!(self.failure, TestFailure::RollbackOnce) && !self.rollback_has_failed {
            self.rollback_has_failed = true;
            return Err(std::io::Error::other("injected rollback failure").into());
        }
        self.skin_ids.remove(skin_id);
        Ok(())
    }

    fn rename_directory(
        &mut self,
        temporary_directory: &std::path::Path,
        final_directory: &std::path::Path,
    ) -> Result<(), SkinImportError> {
        if matches!(
            self.failure,
            TestFailure::Rename
                | TestFailure::RollbackOnce
                | TestFailure::RollbackAlways
                | TestFailure::TemporaryCleanup
                | TestFailure::FinalCleanup
        ) {
            return Err(std::io::Error::other("injected rename failure").into());
        }
        self.temporary_directories.remove(temporary_directory);
        self.final_directories.insert(final_directory.to_owned());
        Ok(())
    }

    fn remove_directory(&mut self, path: &std::path::Path) -> Result<(), SkinImportError> {
        let is_temporary_directory = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'));
        if matches!(self.failure, TestFailure::TemporaryCleanup) && is_temporary_directory {
            return Err(std::io::Error::other("injected temporary cleanup failure").into());
        }
        if matches!(self.failure, TestFailure::FinalCleanup) && !is_temporary_directory {
            return Err(std::io::Error::other("injected final cleanup failure").into());
        }
        self.temporary_directories
            .retain(|entry| !entry.starts_with(path));
        self.final_directories
            .retain(|entry| !entry.starts_with(path));
        Ok(())
    }
}

struct SkinFixture {
    root: PathBuf,
    data_directory: PathBuf,
    repository: CompanionRepository,
}

impl SkinFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "magic-image-library-skins-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self {
            data_directory: root.join("app-data"),
            repository: CompanionRepository::from_connection(Connection::open_in_memory().unwrap())
                .unwrap(),
            root,
        }
    }

    fn write_image(
        &self,
        name: &str,
        width: u32,
        height: u32,
        format: ImageFormat,
        color: Rgba<u8>,
    ) -> PathBuf {
        let path = self.root.join(name);
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, color))
            .save_with_format(&path, format)
            .unwrap();
        path
    }

    fn write_zip(&self, name: &str, entries: Vec<TestZipEntry>) -> PathBuf {
        let path = self.root.join(name);
        let file = fs::File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for entry in entries {
            match entry {
                TestZipEntry::File(name, contents) => {
                    writer.start_file(name, options).unwrap();
                    writer.write_all(&contents).unwrap();
                }
                TestZipEntry::Symlink(name, target) => {
                    writer.add_symlink(name, target, options).unwrap();
                }
                TestZipEntry::Directory(name) => {
                    writer.add_directory(name, options).unwrap();
                }
            }
        }
        writer.finish().unwrap();
        path
    }

    fn assert_zip_rejected(&self, package: &PathBuf, expected: SkinImportErrorKind) {
        let error = import_zip_skin(package, &self.data_directory, &self.repository).unwrap_err();
        assert_eq!(error.kind(), Some(expected), "{error}");
        self.assert_skin_directory_is_empty();
        assert_eq!(self.repository.list_skins().unwrap().len(), 3);
    }

    fn assert_skin_directory_is_empty(&self) {
        let skins_directory = self.data_directory.join("skins");
        assert!(
            !skins_directory.exists() || fs::read_dir(skins_directory).unwrap().next().is_none()
        );
    }
}

impl Drop for SkinFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
