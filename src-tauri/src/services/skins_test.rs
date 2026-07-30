use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use rusqlite::Connection;

use crate::{
    domain::companion::CompanionSkin,
    repository::companion::CompanionRepository,
    services::skins::{
        derive_flow_colors, import_image_skin, persist_normalized_skin_with_storage_for_test,
        SkinImportError, SkinImportStorage,
    },
};

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
    assert!(skin.texture_path.as_ref().unwrap().ends_with("texture.png"));
    assert_eq!(
        image::open(skin.texture_path.unwrap())
            .unwrap()
            .dimensions(),
        (1024, 1024)
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
            "magic-image-library-skins-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
