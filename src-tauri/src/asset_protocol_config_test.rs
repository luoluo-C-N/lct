use std::{
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use tauri::{utils::config::Config, Listener, Manager};

use crate::{
    commands,
    domain::asset::{Asset, CaptureMode},
    repository::assets::AssetRepository,
};

#[test]
fn asset_protocol_only_exposes_the_generated_preview_tree() {
    let config: Config = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let asset_protocol = config.app.security.asset_protocol;

    assert!(asset_protocol.enable);
    assert_eq!(
        asset_protocol.scope.allowed_paths(),
        &[PathBuf::from("$APPLOCALDATA/assets/previews/**/*")]
    );
}

#[test]
fn import_command_emits_each_created_asset_as_its_event_payload() {
    let temporary_directory = temporary_directory("import-events");
    let source_paths = [
        temporary_directory.join("red.png"),
        temporary_directory.join("blue.png"),
    ];
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(&source_paths[0])
        .unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 255, 255]))
        .save(&source_paths[1])
        .unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let app = tauri::test::mock_builder()
        .manage(repository)
        .build(tauri::generate_context!())
        .unwrap();
    let app_handle = app.handle().clone();
    let (sender, receiver) = mpsc::channel();
    app_handle.listen("asset-created", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });

    let assets =
        commands::assets::import_files(source_paths.to_vec(), app_handle, app.state()).unwrap();

    let received_payloads = (0..assets.len())
        .map(|_| receiver.recv_timeout(Duration::from_secs(1)))
        .collect::<Result<Vec<_>, _>>();
    for asset in &assets {
        fs::remove_file(&asset.original_path).unwrap();
        fs::remove_file(&asset.preview_path).unwrap();
    }
    fs::remove_dir_all(temporary_directory).unwrap();

    let payloads = received_payloads
        .unwrap()
        .into_iter()
        .map(|payload| serde_json::from_str::<Asset>(&payload).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(payloads, assets);
    assert!(receiver.try_recv().is_err());
}

#[test]
fn capture_command_emits_the_created_asset_as_its_event_payload() {
    let temporary_directory = temporary_directory("capture-events");
    let captured_image_path = temporary_directory.join("captured.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 255, 0, 255]))
        .save(&captured_image_path)
        .unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let app = tauri::test::mock_builder()
        .manage(repository)
        .build(tauri::generate_context!())
        .unwrap();
    let app_handle = app.handle().clone();
    let (sender, receiver) = mpsc::channel();
    app_handle.listen("asset-created", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });

    let asset = commands::assets::capture_with(
        CaptureMode::Fullscreen,
        None,
        app_handle,
        app.state(),
        move |mode, _region, data_directory, repository| {
            crate::services::capture::persist_captured_image(
                &captured_image_path,
                mode,
                data_directory,
                repository,
            )
        },
    )
    .unwrap();

    let payload = receiver.recv_timeout(Duration::from_secs(1));
    fs::remove_file(&asset.original_path).unwrap();
    fs::remove_file(&asset.preview_path).unwrap();
    fs::remove_dir_all(temporary_directory).unwrap();

    assert_eq!(
        serde_json::from_str::<Asset>(&payload.unwrap()).unwrap(),
        asset
    );
    assert!(receiver.try_recv().is_err());
}

fn temporary_directory(label: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("magic-image-library-{label}-test-{unique_suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}
