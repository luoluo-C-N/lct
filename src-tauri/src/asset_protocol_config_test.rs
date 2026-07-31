use std::{
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Utc};
use rusqlite::Connection;
use tauri::{utils::config::Config, Listener, Manager};

use crate::{
    commands,
    domain::asset::{Asset, CaptureMode},
    repository::assets::AssetRepository,
};

#[test]
fn setup_registers_both_repository_states_on_the_app_handle() {
    let app = tauri::test::mock_builder()
        .build(tauri::generate_context!())
        .unwrap();
    let asset_repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let companion_repository = crate::repository::companion::CompanionRepository::from_connection(
        Connection::open_in_memory().unwrap(),
    )
    .unwrap();

    crate::manage_repository_states(app.handle(), asset_repository, companion_repository).unwrap();

    assert!(app.try_state::<AssetRepository>().is_some());
    assert!(app
        .try_state::<crate::repository::companion::CompanionRepository>()
        .is_some());
}

#[test]
fn asset_protocol_only_exposes_the_generated_preview_tree() {
    let config: Config = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let asset_protocol = config.app.security.asset_protocol;

    assert!(asset_protocol.enable);
    assert_eq!(
        asset_protocol.scope.allowed_paths(),
        &[
            PathBuf::from("$APPLOCALDATA/assets/previews/**/*"),
            PathBuf::from("$APPLOCALDATA/skins/**/*"),
        ]
    );
}

#[test]
fn config_declares_a_safe_companion_window() {
    let config: Config = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let companion = config
        .app
        .windows
        .iter()
        .find(|window| window.label == "companion")
        .expect("companion window configuration");

    assert_eq!(companion.width, 72.0);
    assert_eq!(companion.height, 72.0);
    assert!(!companion.decorations);
    assert!(companion.always_on_top);
    assert!(companion.skip_taskbar);
    assert!(!companion.resizable);
    assert!(companion.transparent);
    assert!(companion.visible);
}

#[test]
fn companion_capability_only_grants_default_core_and_window_dragging() {
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/companion.json")).unwrap();

    assert_eq!(capability["windows"], serde_json::json!(["companion"]));
    assert_eq!(
        capability["permissions"],
        serde_json::json!(["core:default", "core:window:allow-start-dragging"])
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
fn import_command_emits_a_persisted_asset_before_a_later_file_fails() {
    let temporary_directory = temporary_directory("partial-import-events");
    let valid_source_path = temporary_directory.join("valid.png");
    let invalid_source_path = temporary_directory.join("invalid.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(&valid_source_path)
        .unwrap();
    fs::write(&invalid_source_path, b"not a PNG").unwrap();
    let valid_created_at =
        DateTime::<Utc>::from(fs::metadata(&valid_source_path).unwrap().created().unwrap());
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

    let result = commands::assets::import_files(
        vec![valid_source_path, invalid_source_path],
        app_handle,
        app.state(),
    );
    let received_payload = receiver.recv_timeout(Duration::from_secs(1));
    let persisted_assets = app
        .state::<AssetRepository>()
        .list_by_month(valid_created_at.year(), valid_created_at.month())
        .unwrap();
    let persisted_paths = persisted_assets
        .iter()
        .flat_map(|asset| [&asset.original_path, &asset.preview_path])
        .cloned()
        .collect::<Vec<_>>();
    let persisted_files_exist = persisted_paths.iter().all(|path| path.exists());

    for path in persisted_paths {
        fs::remove_file(path).unwrap();
    }
    fs::remove_dir_all(temporary_directory).unwrap();

    assert!(result
        .expect_err("mixed import batch must report the invalid file")
        .contains("failed to decode or encode an image"));
    assert_eq!(persisted_assets.len(), 1);
    assert!(persisted_files_exist);
    let emitted_asset =
        serde_json::from_str::<Asset>(&received_payload.expect("persisted asset event")).unwrap();
    assert_eq!(emitted_asset, persisted_assets[0]);
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
