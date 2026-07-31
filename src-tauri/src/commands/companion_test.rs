use std::{
    fs,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use tauri::{
    Listener, LogicalPosition, LogicalRect, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::{
    commands::companion::{
        anchored_companion_bounds, delete_companion_skin, handle_companion_close,
        handle_window_event, import_companion_skin, set_active_companion_skin,
        set_known_window_visible, CompanionCommandError, CompanionSkinState,
    },
    repository::companion::CompanionRepository,
};

#[test]
fn setting_active_skin_emits_exactly_once_after_commit() {
    let app = mock_app();
    let (sender, receiver) = mpsc::channel();
    app.listen("companion-skin-changed", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });

    let settings =
        set_active_companion_skin("deep-ink".to_owned(), app.handle().clone(), app.state())
            .unwrap();

    let payload = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        serde_json::from_str::<CompanionSkinState>(&payload).unwrap(),
        settings
    );
    assert_eq!(settings.active_skin_id, "deep-ink");
    assert_eq!(settings.skins.len(), 3);
    assert!(receiver.try_recv().is_err());
}

#[test]
fn failed_set_delete_and_import_emit_no_skin_events() {
    let app = mock_app();
    let (sender, receiver) = mpsc::channel();
    app.listen("companion-skin-changed", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });
    let invalid_file = std::env::temp_dir().join(format!(
        "magic-image-library-invalid-skin-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(&invalid_file, b"not an image").unwrap();

    assert!(
        set_active_companion_skin("missing".to_owned(), app.handle().clone(), app.state()).is_err()
    );
    assert!(
        delete_companion_skin("quiet-aurora".to_owned(), app.handle().clone(), app.state())
            .is_err()
    );
    assert!(
        import_companion_skin(invalid_file.clone(), app.handle().clone(), app.state()).is_err()
    );

    fs::remove_file(invalid_file).unwrap();
    assert!(receiver.try_recv().is_err());
    assert_eq!(
        app.state::<CompanionRepository>()
            .get_settings()
            .unwrap()
            .active_skin_id,
        "quiet-aurora"
    );
}

#[test]
fn invalid_window_operations_are_rejected_before_mutation() {
    let app = mock_app();

    assert!(matches!(
        set_known_window_visible("other", true, app.handle()),
        Err(CompanionCommandError::UnknownWindow)
    ));
}

#[test]
fn expansion_anchors_to_the_nearest_monitor_edges() {
    let monitor = LogicalRect {
        position: LogicalPosition::new(0.0, 0.0),
        size: LogicalSize::new(1000.0, 800.0),
    };

    let (position, size) =
        anchored_companion_bounds(LogicalPosition::new(900.0, 700.0), monitor, true);

    assert_eq!(size, LogicalSize::new(232.0, 320.0));
    assert_eq!(position, LogicalPosition::new(740.0, 452.0));
}

#[test]
fn collapsing_restores_the_orb_position_after_right_bottom_expansion() {
    let monitor = LogicalRect {
        position: LogicalPosition::new(0.0, 0.0),
        size: LogicalSize::new(1000.0, 800.0),
    };
    let collapsed_position = LogicalPosition::new(900.0, 700.0);
    let (expanded_position, _) = anchored_companion_bounds(collapsed_position, monitor, true);

    let (restored_position, size) = anchored_companion_bounds(expanded_position, monitor, false);

    assert_eq!(size, LogicalSize::new(72.0, 72.0));
    assert_eq!(restored_position, collapsed_position);
}

#[test]
fn expansion_clamps_a_partially_offscreen_companion_to_the_work_area() {
    let monitor = LogicalRect {
        position: LogicalPosition::new(-1280.0, 0.0),
        size: LogicalSize::new(1280.0, 720.0),
    };

    let (position, size) =
        anchored_companion_bounds(LogicalPosition::new(-1400.0, -50.0), monitor, true);

    assert_eq!(size, LogicalSize::new(232.0, 320.0));
    assert_eq!(position, LogicalPosition::new(-1280.0, 0.0));
}

#[test]
fn companion_close_handling_records_hidden_state_without_destroying_the_window() {
    let app = mock_app();
    assert!(app.get_webview_window("companion").is_some());

    handle_companion_close(app.handle()).unwrap();

    assert!(app.get_webview_window("companion").is_some());
    assert!(
        !app.state::<CompanionRepository>()
            .get_settings()
            .unwrap()
            .visible
    );
}

fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
    let app = tauri::test::mock_builder()
        .on_window_event(handle_window_event)
        .manage(
            CompanionRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap(),
        )
        .build(tauri::generate_context!())
        .unwrap();
    WebviewWindowBuilder::new(
        &app,
        "companion",
        WebviewUrl::App("index.html?window=companion".into()),
    )
    .visible(true)
    .build()
    .unwrap();
    app
}
