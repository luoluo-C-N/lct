use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use rusqlite::Connection;

use crate::{
    domain::asset::AssetSource,
    repository::assets::AssetRepository,
    services::capture::{
        crop_image, encode_png_data_url, normalize_window_region, persist_captured_pixels,
        resolve_capture_target, validate_region, CaptureError, CaptureMode, CaptureTarget,
        CropRegion, WindowLocator,
    },
};

struct MockWindowLocator {
    rect: CropRegion,
}

impl WindowLocator for MockWindowLocator {
    fn foreground_window_rect(&self) -> Result<CropRegion, CaptureError> {
        Ok(self.rect)
    }
}

struct MissingWindowLocator;

impl WindowLocator for MissingWindowLocator {
    fn foreground_window_rect(&self) -> Result<CropRegion, CaptureError> {
        Err(CaptureError::NoFocusedWindow)
    }
}

#[test]
fn crops_a_screenshot_to_the_requested_region() {
    let image = image::RgbaImage::from_fn(4, 3, |x, y| image::Rgba([x as u8, y as u8, 0, 255]));

    let cropped = crop_image(
        image,
        CropRegion {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
    )
    .unwrap();

    assert_eq!(cropped.dimensions(), (2, 2));
    assert_eq!(cropped.get_pixel(0, 0), &image::Rgba([1, 1, 0, 255]));
}

#[test]
fn region_capture_requires_a_crop_region() {
    let error = validate_region(CaptureMode::Region, None).unwrap_err();

    assert_eq!(
        error.to_string(),
        "a crop region is required for region capture"
    );
}

#[test]
fn dispatches_fullscreen_region_and_window_capture_targets() {
    let window_region = CropRegion {
        x: 320,
        y: 80,
        width: 800,
        height: 600,
    };
    let locator = MockWindowLocator {
        rect: window_region,
    };

    assert_eq!(
        resolve_capture_target(CaptureMode::Fullscreen, None, &locator).unwrap(),
        CaptureTarget::Fullscreen
    );
    assert_eq!(
        resolve_capture_target(CaptureMode::Region, Some(window_region), &locator).unwrap(),
        CaptureTarget::PrimaryRegion(window_region)
    );
    assert_eq!(
        resolve_capture_target(CaptureMode::Window, None, &locator).unwrap(),
        CaptureTarget::VirtualDesktopRegion(window_region)
    );
}

#[test]
fn window_capture_propagates_a_missing_foreground_window() {
    assert!(matches!(
        resolve_capture_target(CaptureMode::Window, None, &MissingWindowLocator),
        Err(CaptureError::NoFocusedWindow)
    ));
}

#[test]
fn normalizes_negative_virtual_desktop_coordinates() {
    let region = normalize_window_region(-1600, 80, -800, 680, -1920, 0).unwrap();

    assert_eq!(
        region,
        CropRegion {
            x: 320,
            y: 80,
            width: 800,
            height: 600,
        }
    );
}

#[test]
fn capture_persistence_removes_temporary_image_after_creating_asset() {
    let temporary_directory = temporary_directory();
    let source_path = temporary_directory.join("capture.png");
    let data_directory = temporary_directory.join("app-data");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 255, 255]))
        .save(&source_path)
        .unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();

    let asset = crate::services::capture::persist_captured_image(
        &source_path,
        CaptureMode::Region,
        &data_directory,
        &repository,
    )
    .unwrap();

    assert!(!source_path.exists());
    assert!(asset.original_path.exists());
    assert_eq!(asset.source, AssetSource::Capture);
    assert_eq!(asset.capture_mode, Some(CaptureMode::Region));

    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn encodes_the_snapshot_preview_from_the_original_pixels() {
    let image = image::RgbaImage::from_pixel(2, 1, image::Rgba([17, 34, 51, 255]));

    let data_url = encode_png_data_url(&image).unwrap();

    let encoded = data_url
        .strip_prefix("data:image/png;base64,")
        .expect("PNG data URL prefix");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), (2, 1));
    assert_eq!(decoded.get_pixel(1, 0), &image::Rgba([17, 34, 51, 255]));
}

#[test]
fn persists_in_memory_pixels_as_a_region_capture_asset() {
    let temporary_directory = temporary_directory();
    let data_directory = temporary_directory.join("app-data");
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([90, 80, 70, 255]));

    let asset =
        persist_captured_pixels(image, CaptureMode::Region, &data_directory, &repository).unwrap();

    let persisted = image::open(&asset.original_path).unwrap().into_rgba8();
    assert_eq!(persisted.dimensions(), (2, 2));
    assert_eq!(persisted.get_pixel(0, 0), &image::Rgba([90, 80, 70, 255]));
    assert_eq!(asset.capture_mode, Some(CaptureMode::Region));

    fs::remove_dir_all(temporary_directory).unwrap();
}

fn temporary_directory() -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("magic-image-library-capture-test-{unique_suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}
