use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use rusqlite::Connection;

use crate::{
    repository::companion::CompanionRepository,
    services::skins::{derive_flow_colors, import_image_skin, SkinImportError},
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

fn solid_image(color: Rgba<u8>) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(32, 32, color))
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
