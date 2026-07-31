use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::Datelike;
use rusqlite::Connection;
use tauri::{
    Listener, LogicalPosition, LogicalRect, LogicalSize, Manager, PhysicalPosition, PhysicalSize,
    WebviewUrl, WebviewWindowBuilder,
};

use crate::{
    commands::companion::{
        anchored_companion_bounds, begin_region_selection_with, cancel_region_selection_with,
        complete_companion_region_selection, complete_region_selection_with, delete_companion_skin,
        handle_companion_close, handle_window_event, import_companion_skin,
        set_active_companion_skin, set_known_window_visible, update_companion_skin,
        CompanionCommandError, CompanionRegionSelectionState, CompanionSkinState, MonitorSnapshot,
        RegionSelectionWindow, WindowBounds,
    },
    domain::asset::CaptureMode,
    domain::companion::{CompanionSkin, SkinSource, VisualPreset},
    repository::assets::AssetRepository,
    repository::companion::CompanionRepository,
    services::capture::{CaptureError, CropRegion, ScreenCapturer},
};

struct FakeRegionWindow {
    bounds: Mutex<WindowBounds>,
    monitor: WindowBounds,
    primary_scale_factor: f64,
    visible: AtomicBool,
    fail_operations: Mutex<Vec<&'static str>>,
    operations: Arc<Mutex<Vec<&'static str>>>,
}

impl FakeRegionWindow {
    fn operation(&self, name: &'static str) -> Result<(), CompanionCommandError> {
        self.operations.lock().unwrap().push(name);
        let mut failures = self.fail_operations.lock().unwrap();
        if failures.first() == Some(&name) {
            failures.remove(0);
            return Err(CompanionCommandError::Window(format!(
                "injected {name} failure"
            )));
        }
        Ok(())
    }
}

impl RegionSelectionWindow for FakeRegionWindow {
    fn bounds(&self) -> Result<WindowBounds, CompanionCommandError> {
        self.operation("bounds")?;
        Ok(*self.bounds.lock().unwrap())
    }

    fn primary_monitor(&self) -> Result<MonitorSnapshot, CompanionCommandError> {
        self.operation("primary_monitor")?;
        Ok(MonitorSnapshot {
            bounds: self.monitor,
            scale_factor: self.primary_scale_factor,
        })
    }

    fn set_bounds(&self, bounds: WindowBounds) -> Result<(), CompanionCommandError> {
        self.operation("set_bounds")?;
        *self.bounds.lock().unwrap() = bounds;
        Ok(())
    }

    fn hide(&self) -> Result<(), CompanionCommandError> {
        self.operation("hide")?;
        self.visible.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn show(&self) -> Result<(), CompanionCommandError> {
        self.operation("show")?;
        self.visible.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn focus(&self) -> Result<(), CompanionCommandError> {
        self.operation("focus")
    }
}

struct FakeScreenCapturer {
    image: image::RgbaImage,
    fail: AtomicBool,
    calls: AtomicUsize,
    operations: Arc<Mutex<Vec<&'static str>>>,
}

impl ScreenCapturer for FakeScreenCapturer {
    fn capture_primary(&self) -> Result<image::RgbaImage, CaptureError> {
        self.operations.lock().unwrap().push("capture");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(CaptureError::Screenshot(
                "injected capture failure".to_owned(),
            ));
        }
        Ok(self.image.clone())
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
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, monitor, operations.clone());
    let capturer = fake_screen_capturer(operations.clone());
    let state = CompanionRegionSelectionState::default();

    let session = begin_region_selection_with(&window, &state, &capturer).unwrap();

    assert_eq!(session.scale_factor, 1.0);
    assert!(session
        .preview_data_url
        .starts_with("data:image/png;base64,"));
    assert_eq!(window.bounds().unwrap(), monitor);
    assert_eq!(capturer.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        &operations.lock().unwrap()[..7],
        &[
            "bounds",
            "primary_monitor",
            "hide",
            "capture",
            "set_bounds",
            "show",
            "focus",
        ]
    );
    assert!(begin_region_selection_with(&window, &state, &capturer).is_err());
    assert_eq!(capturer.calls.load(Ordering::SeqCst), 1);

    cancel_region_selection_with(&window, &state).unwrap();
    assert_eq!(window.bounds().unwrap(), original);
    assert!(window.visible.load(Ordering::SeqCst));
    assert!(cancel_region_selection_with(&window, &state).is_err());
}

#[test]
fn failed_region_selection_setup_restores_the_original_bounds() {
    for failed_operation in ["set_bounds", "show", "focus"] {
        let original = physical_bounds(120, 80, 72, 72);
        let operations = Arc::new(Mutex::new(Vec::new()));
        let window = fake_region_window(
            original,
            physical_bounds(0, 0, 1920, 1080),
            operations.clone(),
        );
        window
            .fail_operations
            .lock()
            .unwrap()
            .push(failed_operation);
        let capturer = fake_screen_capturer(operations);
        let state = CompanionRegionSelectionState::default();

        assert!(begin_region_selection_with(&window, &state, &capturer).is_err());
        assert_eq!(window.bounds().unwrap(), original);
        assert!(window.visible.load(Ordering::SeqCst));
        assert!(cancel_region_selection_with(&window, &state).is_err());
    }
}

#[test]
fn failed_desktop_capture_restores_the_hidden_companion() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(
        original,
        physical_bounds(0, 0, 1920, 1080),
        operations.clone(),
    );
    let capturer = fake_screen_capturer(operations);
    capturer.fail.store(true, Ordering::SeqCst);
    let state = CompanionRegionSelectionState::default();

    assert!(begin_region_selection_with(&window, &state, &capturer).is_err());
    assert_eq!(window.bounds().unwrap(), original);
    assert!(window.visible.load(Ordering::SeqCst));
    assert!(cancel_region_selection_with(&window, &state).is_err());
}

#[test]
fn completing_region_selection_crops_the_stored_frame_without_recapturing() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    let capturer = FakeScreenCapturer {
        image: image::RgbaImage::from_fn(4, 4, |x, y| image::Rgba([x as u8, y as u8, 42, 255])),
        fail: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        operations,
    };
    let state = CompanionRegionSelectionState::default();
    begin_region_selection_with(&window, &state, &capturer).unwrap();
    let temporary_directory = temporary_region_directory("complete");
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let emitted = Mutex::new(Vec::new());

    let asset = complete_region_selection_with(
        &window,
        &state,
        CropRegion {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
        &temporary_directory,
        &repository,
        |asset| {
            emitted.lock().unwrap().push(asset.clone());
            Ok(())
        },
    )
    .unwrap();

    let persisted = image::open(&asset.original_path).unwrap().into_rgba8();
    assert_eq!(persisted.dimensions(), (2, 2));
    assert_eq!(persisted.get_pixel(0, 0), &image::Rgba([1, 1, 42, 255]));
    assert_eq!(asset.capture_mode, Some(CaptureMode::Region));
    assert_eq!(*emitted.lock().unwrap(), vec![asset]);
    assert_eq!(capturer.calls.load(Ordering::SeqCst), 1);
    assert_eq!(window.bounds().unwrap(), original);
    assert!(cancel_region_selection_with(&window, &state).is_err());

    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn restore_failure_keeps_the_region_session_available_for_cancel_retry() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    let capturer = fake_screen_capturer(operations);
    let state = CompanionRegionSelectionState::default();
    begin_region_selection_with(&window, &state, &capturer).unwrap();
    window.fail_operations.lock().unwrap().push("set_bounds");
    let temporary_directory = temporary_region_directory("restore-retry");
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();

    assert!(complete_region_selection_with(
        &window,
        &state,
        CropRegion {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        &temporary_directory,
        &repository,
        |_| Ok(()),
    )
    .is_err());

    cancel_region_selection_with(&window, &state).unwrap();
    assert_eq!(window.bounds().unwrap(), original);
    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn failed_region_setup_and_rollback_retains_recovery_for_cancel_retry() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    window
        .fail_operations
        .lock()
        .unwrap()
        .extend(["set_bounds", "set_bounds"]);
    let capturer = fake_screen_capturer(operations);
    let state = CompanionRegionSelectionState::default();

    assert!(begin_region_selection_with(&window, &state, &capturer).is_err());

    cancel_region_selection_with(&window, &state).unwrap();
    assert_eq!(window.bounds().unwrap(), original);
    assert!(window.visible.load(Ordering::SeqCst));
}

#[test]
fn invalid_region_restores_the_companion_without_emitting_or_persisting() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    let capturer = fake_screen_capturer(operations);
    let state = CompanionRegionSelectionState::default();
    begin_region_selection_with(&window, &state, &capturer).unwrap();
    let temporary_directory = temporary_region_directory("invalid");
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let emitted = AtomicUsize::new(0);

    assert!(complete_region_selection_with(
        &window,
        &state,
        CropRegion {
            x: 3,
            y: 3,
            width: 2,
            height: 2,
        },
        &temporary_directory,
        &repository,
        |_| {
            emitted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )
    .is_err());

    assert_eq!(emitted.load(Ordering::SeqCst), 0);
    assert_eq!(window.bounds().unwrap(), original);
    assert!(cancel_region_selection_with(&window, &state).is_err());
    assert!(repository
        .list_by_month(chrono::Utc::now().year(), chrono::Utc::now().month())
        .unwrap()
        .is_empty());
    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn persistence_failure_restores_the_companion_without_emitting() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    let capturer = fake_screen_capturer(operations);
    let state = CompanionRegionSelectionState::default();
    begin_region_selection_with(&window, &state, &capturer).unwrap();
    let temporary_directory = temporary_region_directory("persist-failure");
    let blocked_data_path = temporary_directory.join("not-a-directory");
    fs::write(&blocked_data_path, b"blocked").unwrap();
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();
    let emitted = AtomicUsize::new(0);

    assert!(complete_region_selection_with(
        &window,
        &state,
        CropRegion {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        &blocked_data_path,
        &repository,
        |_| {
            emitted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )
    .is_err());

    assert_eq!(emitted.load(Ordering::SeqCst), 0);
    assert_eq!(window.bounds().unwrap(), original);
    assert!(cancel_region_selection_with(&window, &state).is_err());
    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn event_failure_clears_the_region_session_after_persistence() {
    let original = physical_bounds(120, 80, 72, 72);
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(original, physical_bounds(0, 0, 4, 4), operations.clone());
    let capturer = fake_screen_capturer(operations);
    let state = CompanionRegionSelectionState::default();
    begin_region_selection_with(&window, &state, &capturer).unwrap();
    let temporary_directory = temporary_region_directory("event-failure");
    let repository =
        AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap();

    assert!(complete_region_selection_with(
        &window,
        &state,
        CropRegion {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        &temporary_directory,
        &repository,
        |_| Err(CompanionCommandError::Event(
            "listener unavailable".to_owned()
        )),
    )
    .is_err());

    assert_eq!(window.bounds().unwrap(), original);
    assert!(cancel_region_selection_with(&window, &state).is_err());
    assert_eq!(
        repository
            .list_by_month(chrono::Utc::now().year(), chrono::Utc::now().month())
            .unwrap()
            .len(),
        1
    );
    fs::remove_dir_all(temporary_directory).unwrap();
}

#[test]
fn frozen_region_command_emits_the_persisted_asset_exactly_once() {
    let app = mock_app();
    let operations = Arc::new(Mutex::new(Vec::new()));
    let window = fake_region_window(
        physical_bounds(120, 80, 72, 72),
        physical_bounds(0, 0, 4, 4),
        operations.clone(),
    );
    let capturer = fake_screen_capturer(operations);
    begin_region_selection_with(
        &window,
        &app.state::<CompanionRegionSelectionState>(),
        &capturer,
    )
    .unwrap();
    let (sender, receiver) = mpsc::channel();
    app.listen("asset-created", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });

    let asset = complete_companion_region_selection(
        CropRegion {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        app.handle().clone(),
        app.state(),
        app.state(),
    )
    .unwrap();

    let emitted =
        serde_json::from_str(&receiver.recv_timeout(Duration::from_secs(1)).unwrap()).unwrap();
    assert_eq!(asset, emitted);
    assert!(receiver.try_recv().is_err());
    fs::remove_file(asset.original_path).unwrap();
    fs::remove_file(asset.preview_path).unwrap();
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
        .manage(AssetRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap())
        .manage(
            CompanionRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap(),
        )
        .manage(CompanionRegionSelectionState::default())
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

fn fake_region_window(
    original: WindowBounds,
    monitor: WindowBounds,
    operations: Arc<Mutex<Vec<&'static str>>>,
) -> FakeRegionWindow {
    FakeRegionWindow {
        bounds: Mutex::new(original),
        monitor,
        primary_scale_factor: 1.0,
        visible: AtomicBool::new(true),
        fail_operations: Mutex::new(Vec::new()),
        operations,
    }
}

fn fake_screen_capturer(operations: Arc<Mutex<Vec<&'static str>>>) -> FakeScreenCapturer {
    FakeScreenCapturer {
        image: image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255])),
        fail: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        operations,
    }
}

fn temporary_region_directory(label: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "magic-image-library-region-{label}-test-{unique_suffix}"
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
