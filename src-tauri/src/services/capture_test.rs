use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use crate::{
    domain::asset::AssetSource, repository::assets::AssetRepository,
    services::capture::{validate_region, CaptureMode},
};

#[test]
fn region_capture_requires_a_crop_region() {
    let error = validate_region(CaptureMode::Region, None).unwrap_err();

    assert_eq!(error.to_string(), "a crop region is required for region capture");
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

    let asset = crate::services::capture::persist_captured_image(&source_path, &data_directory, &repository).unwrap();

    assert!(!source_path.exists());
    assert!(asset.original_path.exists());
    assert_eq!(asset.source, AssetSource::Capture);

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
