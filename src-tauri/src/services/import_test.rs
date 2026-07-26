use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use crate::{
    domain::asset::AssetSource, repository::assets::AssetRepository, services::import::import_files,
};

#[test]
fn import_copies_png_and_creates_import_asset() {
    let temporary_directory = temporary_directory();
    let source_path = temporary_directory.join("source.png");
    let data_directory = temporary_directory.join("app-data");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(&source_path)
        .unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();

    let assets = import_files(&[source_path], &data_directory, &repository).unwrap();

    assert_eq!(assets.len(), 1);
    assert!(assets[0].original_path.exists());
    assert!(assets[0].preview_path.exists());
    assert_eq!(
        assets[0].preview_path.parent(),
        Some(data_directory.join("assets").join("previews").as_path())
    );
    assert_eq!(assets[0].source, AssetSource::Import);

    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn import_removes_copied_file_when_image_is_invalid() {
    let temporary_directory = temporary_directory();
    let source_path = temporary_directory.join("invalid.png");
    let data_directory = temporary_directory.join("app-data");
    fs::write(&source_path, b"not a PNG").unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();

    assert!(import_files(&[source_path], &data_directory, &repository).is_err());
    assert_eq!(
        fs::read_dir(data_directory.join("assets").join("originals"))
            .unwrap()
            .count(),
        0
    );

    fs::remove_dir_all(temporary_directory).unwrap();
}

fn temporary_directory() -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("magic-image-library-import-test-{unique_suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}
