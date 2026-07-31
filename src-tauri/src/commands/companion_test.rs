use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use tauri::{
    Listener, LogicalPosition, LogicalRect, LogicalSize, Manager, PhysicalPosition, PhysicalSize,
    WebviewUrl, WebviewWindowBuilder,
};

use crate::{
    commands::companion::{
        anchored_companion_bounds, begin_region_selection_with, delete_companion_skin,
        finish_region_selection_with, handle_companion_close, handle_window_event,
        import_companion_skin, set_active_companion_skin, set_known_window_visible,
        update_companion_skin, CompanionCommandError, CompanionRegionSelectionState,
        CompanionSkinState, RegionSelectionWindow, WindowBounds,
    },
    domain::companion::{CompanionSkin, SkinSource, VisualPreset},
    repository::companion::CompanionRepository,
};

struct FakeRegionWindow {
    bounds: Mutex<WindowBounds>,
    monitor: WindowBounds,
    scale_factor: f64,
    fail_next_set: AtomicBool,
}

impl RegionSelectionWindow for FakeRegionWindow {
    fn bounds(&self) -> Result<WindowBounds, CompanionCommandError> {
        Ok(*self.bounds.lock().unwrap())
    }

    fn monitor_bounds(&self) -> Result<WindowBounds, CompanionCommandError> {
        Ok(self.monitor)
    }

    fn scale_factor(&self) -> Result<f64, CompanionCommandError> {
        Ok(self.scale_factor)
    }

    fn set_bounds(&self, bounds: WindowBounds) -> Result<(), CompanionCommandError> {
        if self.fail_next_set.swap(false, Ordering::SeqCst) {
            return Err(CompanionCommandError::Window(
                "injected resize failure".to_owned(),
            ));
        }
        *self.bounds.lock().unwrap() = bounds;
        Ok(())
    }
}

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
fn deleting_a_local_skin_removes_its_persisted_directory_and_record() {
    let app = mock_app();
    let skin_id = format!(
        "delete-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let skin_directory = app
        .path()
        .app_local_data_dir()
        .unwrap()
        .join("skins")
        .join(&skin_id);
    fs::create_dir_all(&skin_directory).unwrap();
    fs::write(skin_directory.join("texture.png"), b"texture").unwrap();
    fs::write(skin_directory.join("preview.png"), b"preview").unwrap();
    app.state::<CompanionRepository>()
        .create_skin(&CompanionSkin {
            id: skin_id.clone(),
            name: "Delete me".to_owned(),
            source: SkinSource::Image,
            visual_preset: VisualPreset::Custom,
            texture_path: Some(skin_directory.join("texture.png")),
            preview_path: Some(skin_directory.join("preview.png")),
            flow_colors: vec!["#BDA7FF".to_owned(), "#55D8CF".to_owned()],
            flow_speed: 1.0,
            flow_intensity: 0.7,
            created_at: chrono::Utc::now(),
        })
        .unwrap();

    delete_companion_skin(skin_id.clone(), app.handle().clone(), app.state()).unwrap();

    assert!(!skin_directory.exists());
    assert!(app
        .state::<CompanionRepository>()
        .list_skins()
        .unwrap()
        .iter()
        .all(|skin| skin.id != skin_id));
}

#[test]
fn updating_a_local_skin_preserves_managed_fields_and_clamps_motion() {
    let app = mock_app();
    let original = local_skin_fixture(&app, "update-test");
    let mut unsafe_update = original.clone();
    unsafe_update.texture_path = Some(std::env::temp_dir().join("outside.png"));
    assert!(update_companion_skin(unsafe_update, app.handle().clone(), app.state()).is_err());
    let stored = app
        .state::<CompanionRepository>()
        .list_skins()
        .unwrap()
        .into_iter()
        .find(|skin| skin.id == original.id)
        .unwrap();
    assert_eq!(stored.texture_path, original.texture_path);

    let mut safe_update = original.clone();
    safe_update.name = "Updated".to_owned();
    safe_update.flow_speed = 9.0;
    safe_update.flow_intensity = -1.0;
    let state = update_companion_skin(safe_update, app.handle().clone(), app.state()).unwrap();
    let updated = state
        .skins
        .into_iter()
        .find(|skin| skin.id == original.id)
        .unwrap();
    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.flow_speed, 2.0);
    assert_eq!(updated.flow_intensity, 0.0);

    fs::remove_dir_all(original.texture_path.unwrap().parent().unwrap()).unwrap();
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
fn region_selection_covers_the_monitor_and_restores_the_original_bounds() {
    let original = physical_bounds(120, 80, 72, 72);
    let monitor = physical_bounds(0, 0, 1920, 1080);
    let window = fake_region_window(original, monitor);
    let state = CompanionRegionSelectionState::default();

    let session = begin_region_selection_with(&window, &state).unwrap();

    assert_eq!(session.scale_factor, 1.5);
    assert_eq!(window.bounds().unwrap(), monitor);
    assert!(begin_region_selection_with(&window, &state).is_err());

    finish_region_selection_with(&window, &state).unwrap();
    assert_eq!(window.bounds().unwrap(), original);
    assert!(finish_region_selection_with(&window, &state).is_err());
}

#[test]
fn failed_region_selection_setup_restores_the_original_bounds() {
    let original = physical_bounds(120, 80, 72, 72);
    let window = FakeRegionWindow {
        bounds: Mutex::new(original),
        monitor: physical_bounds(0, 0, 1920, 1080),
        scale_factor: 1.5,
        fail_next_set: AtomicBool::new(true),
    };
    let state = CompanionRegionSelectionState::default();

    assert!(begin_region_selection_with(&window, &state).is_err());
    assert_eq!(window.bounds().unwrap(), original);
    assert!(finish_region_selection_with(&window, &state).is_err());
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

fn local_skin_fixture(app: &tauri::App<tauri::test::MockRuntime>, label: &str) -> CompanionSkin {
    let skin_id = format!(
        "{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let skin_directory = app
        .path()
        .app_local_data_dir()
        .unwrap()
        .join("skins")
        .join(&skin_id);
    fs::create_dir_all(&skin_directory).unwrap();
    fs::write(skin_directory.join("texture.png"), b"texture").unwrap();
    fs::write(skin_directory.join("preview.png"), b"preview").unwrap();
    let skin = CompanionSkin {
        id: skin_id,
        name: label.to_owned(),
        source: SkinSource::Image,
        visual_preset: VisualPreset::Custom,
        texture_path: Some(skin_directory.join("texture.png")),
        preview_path: Some(skin_directory.join("preview.png")),
        flow_colors: vec!["#BDA7FF".to_owned(), "#55D8CF".to_owned()],
        flow_speed: 1.0,
        flow_intensity: 0.7,
        created_at: chrono::Utc::now(),
    };
    app.state::<CompanionRepository>()
        .create_skin(&skin)
        .unwrap();
    skin
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

fn physical_bounds(x: i32, y: i32, width: u32, height: u32) -> WindowBounds {
    WindowBounds {
        position: PhysicalPosition::new(x, y),
        size: PhysicalSize::new(width, height),
    }
}

fn fake_region_window(original: WindowBounds, monitor: WindowBounds) -> FakeRegionWindow {
    FakeRegionWindow {
        bounds: Mutex::new(original),
        monitor,
        scale_factor: 1.5,
        fail_next_set: AtomicBool::new(false),
    }
}
