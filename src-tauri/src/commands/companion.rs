use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{
    Emitter, LogicalPosition, LogicalRect, LogicalSize, Manager, PhysicalPosition, PhysicalSize,
    Runtime, State,
};
use thiserror::Error;

use crate::{
    domain::asset::Asset,
    domain::companion::{CompanionSettings, CompanionSkin, MotionSettings, WindowPlacement},
    repository::assets::AssetRepository,
    repository::companion::{CompanionRepository, CompanionRepositoryError},
    services::capture::{self, CropRegion, ScreenCapturer, ScreenshotCapturer},
    services::skins::{self, SkinImportError},
};

const COLLAPSED_SIZE: LogicalSize<f64> = LogicalSize {
    width: 72.0,
    height: 72.0,
};
const EXPANDED_SIZE: LogicalSize<f64> = LogicalSize {
    width: 232.0,
    height: 320.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowBounds {
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

#[derive(Default)]
pub struct CompanionRegionSelectionState {
    session: Mutex<Option<RegionCaptureSession>>,
}

struct RegionCaptureSession {
    original_bounds: WindowBounds,
    image: image::RgbaImage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionSelectionSession {
    pub scale_factor: f64,
    pub preview_data_url: String,
}

pub(crate) trait RegionSelectionWindow {
    fn bounds(&self) -> Result<WindowBounds, CompanionCommandError>;
    fn monitor_bounds(&self) -> Result<WindowBounds, CompanionCommandError>;
    fn scale_factor(&self) -> Result<f64, CompanionCommandError>;
    fn set_bounds(&self, bounds: WindowBounds) -> Result<(), CompanionCommandError>;
    fn hide(&self) -> Result<(), CompanionCommandError>;
    fn show(&self) -> Result<(), CompanionCommandError>;
    fn focus(&self) -> Result<(), CompanionCommandError>;
}

impl<R: Runtime> RegionSelectionWindow for tauri::WebviewWindow<R> {
    fn bounds(&self) -> Result<WindowBounds, CompanionCommandError> {
        Ok(WindowBounds {
            position: self
                .outer_position()
                .map_err(|error| CompanionCommandError::Window(error.to_string()))?,
            size: self
                .outer_size()
                .map_err(|error| CompanionCommandError::Window(error.to_string()))?,
        })
    }

    fn monitor_bounds(&self) -> Result<WindowBounds, CompanionCommandError> {
        let monitor = self
            .primary_monitor()
            .map_err(|error| CompanionCommandError::Window(error.to_string()))?
            .ok_or_else(|| CompanionCommandError::WindowUnavailable("monitor".to_owned()))?;
        Ok(WindowBounds {
            position: *monitor.position(),
            size: *monitor.size(),
        })
    }

    fn scale_factor(&self) -> Result<f64, CompanionCommandError> {
        self.scale_factor()
            .map_err(|error| CompanionCommandError::Window(error.to_string()))
    }

    fn set_bounds(&self, bounds: WindowBounds) -> Result<(), CompanionCommandError> {
        self.set_position(bounds.position)
            .and_then(|_| self.set_size(bounds.size))
            .map_err(|error| CompanionCommandError::Window(error.to_string()))
    }

    fn hide(&self) -> Result<(), CompanionCommandError> {
        self.hide()
            .map_err(|error| CompanionCommandError::Window(error.to_string()))
    }

    fn show(&self) -> Result<(), CompanionCommandError> {
        self.show()
            .map_err(|error| CompanionCommandError::Window(error.to_string()))
    }

    fn focus(&self) -> Result<(), CompanionCommandError> {
        self.set_focus()
            .map_err(|error| CompanionCommandError::Window(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSkinState {
    pub active_skin_id: String,
    pub skins: Vec<CompanionSkin>,
}

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum CompanionCommandError {
    #[error("unknown application window")]
    UnknownWindow,
    #[error("application window is unavailable: {0}")]
    WindowUnavailable(String),
    #[error("window operation failed: {0}")]
    Window(String),
    #[error("companion repository operation failed: {0}")]
    Repository(String),
    #[error("skin import failed: {0}")]
    Import(String),
    #[error("unsupported companion skin file: {0}")]
    UnsupportedSkinFile(String),
    #[error("event emission failed: {0}")]
    Event(String),
    #[error("skin file operation failed: {0}")]
    Files(String),
    #[error("invalid skin update: {0}")]
    InvalidSkinUpdate(String),
    #[error("capture failed: {0}")]
    Capture(String),
}

impl From<CompanionRepositoryError> for CompanionCommandError {
    fn from(error: CompanionRepositoryError) -> Self {
        Self::Repository(error.to_string())
    }
}

impl From<SkinImportError> for CompanionCommandError {
    fn from(error: SkinImportError) -> Self {
        let category = error
            .kind()
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "ImportFailed".to_owned());
        Self::Import(format!("{category}: {error}"))
    }
}

#[tauri::command]
pub fn list_companion_skins(
    repository: State<'_, CompanionRepository>,
) -> Result<Vec<CompanionSkin>, CompanionCommandError> {
    Ok(repository.list_skins()?)
}

#[tauri::command]
pub fn get_companion_settings(
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSettings, CompanionCommandError> {
    Ok(repository.get_settings()?)
}

#[tauri::command]
pub fn import_companion_skin<R: Runtime>(
    path: PathBuf,
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSkin, CompanionCommandError> {
    let data_directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| CompanionCommandError::Window(error.to_string()))?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let skin = if extension.eq_ignore_ascii_case("zip") {
        skins::import_zip_skin(&path, &data_directory, &repository)?
    } else if extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("webp") {
        skins::import_image_skin(&path, None, &data_directory, &repository)?
    } else {
        return Err(CompanionCommandError::UnsupportedSkinFile(
            path.display().to_string(),
        ));
    };
    emit_skin_state(&app, &repository)?;
    Ok(skin)
}

#[tauri::command]
pub fn update_companion_skin<R: Runtime>(
    mut skin: CompanionSkin,
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSkinState, CompanionCommandError> {
    let existing = repository
        .list_skins()?
        .into_iter()
        .find(|stored| stored.id == skin.id)
        .ok_or_else(|| CompanionCommandError::Repository(format!("skin not found: {}", skin.id)))?;
    if skin.source != existing.source
        || skin.visual_preset != existing.visual_preset
        || skin.texture_path != existing.texture_path
        || skin.preview_path != existing.preview_path
        || skin.created_at != existing.created_at
    {
        return Err(CompanionCommandError::InvalidSkinUpdate(
            "managed skin fields cannot be changed".to_owned(),
        ));
    }
    skin.name = skin.name.trim().to_owned();
    if skin.name.is_empty() || skin.name.chars().count() > 48 {
        return Err(CompanionCommandError::InvalidSkinUpdate(
            "skin name must contain 1 to 48 characters".to_owned(),
        ));
    }
    if skin.flow_colors.len() != 2 || skin.flow_colors.iter().any(|color| !is_hex_color(color)) {
        return Err(CompanionCommandError::InvalidSkinUpdate(
            "exactly two hexadecimal flow colors are required".to_owned(),
        ));
    }
    if !skin.flow_speed.is_finite() || !skin.flow_intensity.is_finite() {
        return Err(CompanionCommandError::InvalidSkinUpdate(
            "motion values must be finite".to_owned(),
        ));
    }
    let motion = MotionSettings::new(skin.flow_speed, skin.flow_intensity);
    skin.flow_speed = motion.flow_speed;
    skin.flow_intensity = motion.flow_intensity;
    repository.update_skin(&skin)?;
    emit_skin_state(&app, &repository)
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

#[tauri::command]
pub fn set_active_companion_skin<R: Runtime>(
    skin_id: String,
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSkinState, CompanionCommandError> {
    repository.set_active_skin(&skin_id)?;
    emit_skin_state(&app, &repository)
}

#[tauri::command]
pub fn delete_companion_skin<R: Runtime>(
    skin_id: String,
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSkinState, CompanionCommandError> {
    let skin = repository
        .list_skins()?
        .into_iter()
        .find(|skin| skin.id == skin_id)
        .ok_or_else(|| CompanionCommandError::Repository(format!("skin not found: {skin_id}")))?;
    if skin.source == crate::domain::companion::SkinSource::Builtin {
        repository.delete_skin(&skin_id)?;
    } else {
        delete_local_skin_files_and_record(&app, &repository, &skin)?;
    }
    emit_skin_state(&app, &repository)
}

fn delete_local_skin_files_and_record<R: Runtime>(
    app: &tauri::AppHandle<R>,
    repository: &CompanionRepository,
    skin: &CompanionSkin,
) -> Result<(), CompanionCommandError> {
    let skins_root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| CompanionCommandError::Files(error.to_string()))?
        .join("skins");
    let expected_directory = skins_root.join(&skin.id);
    let stored_directory = skin
        .texture_path
        .as_ref()
        .and_then(|path| path.parent())
        .ok_or_else(|| CompanionCommandError::Files("skin texture path is missing".to_owned()))?;
    if stored_directory != expected_directory {
        return Err(CompanionCommandError::Files(
            "skin texture path is outside its managed directory".to_owned(),
        ));
    }

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| CompanionCommandError::Files(error.to_string()))?
        .as_nanos();
    let staged_directory = skins_root.join(format!(".{}.delete-{suffix}", skin.id));
    fs::rename(&expected_directory, &staged_directory)
        .map_err(|error| CompanionCommandError::Files(error.to_string()))?;
    if let Err(error) = repository.delete_skin(&skin.id) {
        fs::rename(&staged_directory, &expected_directory).map_err(|rollback_error| {
            CompanionCommandError::Files(format!(
                "{error}; restoring skin directory failed: {rollback_error}"
            ))
        })?;
        return Err(error.into());
    }
    let _ = fs::remove_dir_all(staged_directory);
    Ok(())
}

#[tauri::command]
pub fn show_companion<R: Runtime>(
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSettings, CompanionCommandError> {
    set_companion_visibility(true, &app, &repository)
}

#[tauri::command]
pub fn hide_companion<R: Runtime>(
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSettings, CompanionCommandError> {
    set_companion_visibility(false, &app, &repository)
}

#[tauri::command]
pub fn focus_main_window<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<(), CompanionCommandError> {
    let window = known_window("main", &app)?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|error| CompanionCommandError::Window(error.to_string()))
}

#[tauri::command]
pub fn set_companion_expanded<R: Runtime>(
    expanded: bool,
    app: tauri::AppHandle<R>,
) -> Result<(), CompanionCommandError> {
    let window = known_window("companion", &app)?;
    let scale_factor = window
        .scale_factor()
        .map_err(|error| CompanionCommandError::Window(error.to_string()))?;
    let current = window
        .outer_position()
        .map_err(|error| CompanionCommandError::Window(error.to_string()))?
        .to_logical(scale_factor);
    let monitor = window
        .current_monitor()
        .map_err(|error| CompanionCommandError::Window(error.to_string()))?
        .ok_or_else(|| CompanionCommandError::WindowUnavailable("monitor".to_owned()))?;
    let work_area = monitor.work_area();
    let logical_work_area = LogicalRect {
        position: work_area.position.to_logical(scale_factor),
        size: work_area.size.to_logical(scale_factor),
    };
    let (position, size) = anchored_companion_bounds(current, logical_work_area, expanded);
    window
        .set_position(position)
        .and_then(|_| window.set_size(size))
        .map_err(|error| CompanionCommandError::Window(error.to_string()))
}

#[tauri::command]
pub fn begin_companion_region_selection<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, CompanionRegionSelectionState>,
) -> Result<RegionSelectionSession, CompanionCommandError> {
    let window = known_window("companion", &app)?;
    begin_region_selection_with(&window, &state, &ScreenshotCapturer)
}

#[tauri::command]
pub fn complete_companion_region_selection<R: Runtime>(
    region: CropRegion,
    app: tauri::AppHandle<R>,
    state: State<'_, CompanionRegionSelectionState>,
    repository: State<'_, AssetRepository>,
) -> Result<Asset, CompanionCommandError> {
    let window = known_window("companion", &app)?;
    let data_directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| CompanionCommandError::Files(error.to_string()))?;
    complete_region_selection_with(
        &window,
        &state,
        region,
        &data_directory,
        &repository,
        |asset| {
            app.emit("asset-created", asset)
                .map_err(|error| CompanionCommandError::Event(error.to_string()))
        },
    )
}

#[tauri::command]
pub fn cancel_companion_region_selection<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, CompanionRegionSelectionState>,
) -> Result<(), CompanionCommandError> {
    let window = known_window("companion", &app)?;
    cancel_region_selection_with(&window, &state)
}

pub(crate) fn begin_region_selection_with(
    window: &dyn RegionSelectionWindow,
    state: &CompanionRegionSelectionState,
    capturer: &dyn ScreenCapturer,
) -> Result<RegionSelectionSession, CompanionCommandError> {
    let mut active = state.session.lock().map_err(|_| {
        CompanionCommandError::Window("region selection state is unavailable".to_owned())
    })?;
    if active.is_some() {
        return Err(CompanionCommandError::Window(
            "region selection is already active".to_owned(),
        ));
    }
    let original = window.bounds()?;
    let monitor = window.monitor_bounds()?;
    let scale_factor = window.scale_factor()?;
    window.hide()?;
    let setup = (|| {
        let image = capturer
            .capture_primary()
            .map_err(|error| CompanionCommandError::Capture(error.to_string()))?;
        let preview_data_url = capture::encode_png_data_url(&image)
            .map_err(|error| CompanionCommandError::Capture(error.to_string()))?;
        window.set_bounds(monitor)?;
        window.show()?;
        window.focus()?;
        Ok((image, preview_data_url))
    })();
    let (image, preview_data_url) = match setup {
        Ok(result) => result,
        Err(error) => {
            restore_region_window(window, original).map_err(|rollback_error| {
                CompanionCommandError::Window(format!(
                    "{error}; restoring companion window failed: {rollback_error}"
                ))
            })?;
            return Err(error);
        }
    };
    *active = Some(RegionCaptureSession {
        original_bounds: original,
        image,
    });
    Ok(RegionSelectionSession {
        scale_factor,
        preview_data_url,
    })
}

pub(crate) fn cancel_region_selection_with(
    window: &dyn RegionSelectionWindow,
    state: &CompanionRegionSelectionState,
) -> Result<(), CompanionCommandError> {
    let mut active = state.session.lock().map_err(|_| {
        CompanionCommandError::Window("region selection state is unavailable".to_owned())
    })?;
    let original = active
        .as_ref()
        .map(|session| session.original_bounds)
        .ok_or_else(|| {
            CompanionCommandError::Window("region selection is not active".to_owned())
        })?;
    restore_region_window(window, original)?;
    *active = None;
    Ok(())
}

pub(crate) fn complete_region_selection_with<F>(
    window: &dyn RegionSelectionWindow,
    state: &CompanionRegionSelectionState,
    region: CropRegion,
    data_directory: &std::path::Path,
    repository: &AssetRepository,
    emit_asset: F,
) -> Result<Asset, CompanionCommandError>
where
    F: FnOnce(&Asset) -> Result<(), CompanionCommandError>,
{
    let session = {
        let mut active = state.session.lock().map_err(|_| {
            CompanionCommandError::Window("region selection state is unavailable".to_owned())
        })?;
        let original = active
            .as_ref()
            .map(|session| session.original_bounds)
            .ok_or_else(|| {
                CompanionCommandError::Window("region selection is not active".to_owned())
            })?;
        restore_region_window(window, original)?;
        active.take().expect("active region session checked above")
    };
    let cropped = capture::crop_image(session.image, region)
        .map_err(|error| CompanionCommandError::Capture(error.to_string()))?;
    let asset = capture::persist_captured_pixels(
        cropped,
        capture::CaptureMode::Region,
        data_directory,
        repository,
    )
    .map_err(|error| CompanionCommandError::Capture(error.to_string()))?;
    emit_asset(&asset)?;
    Ok(asset)
}

fn restore_region_window(
    window: &dyn RegionSelectionWindow,
    original: WindowBounds,
) -> Result<(), CompanionCommandError> {
    window.set_bounds(original)?;
    window.show()?;
    window.focus()
}

#[tauri::command]
pub fn save_companion_placement<R: Runtime>(
    placement: WindowPlacement,
    app: tauri::AppHandle<R>,
    repository: State<'_, CompanionRepository>,
) -> Result<CompanionSettings, CompanionCommandError> {
    let mut settings = repository.get_settings()?;
    settings.placement = Some(placement);
    repository.save_settings(&settings)?;
    app.emit("companion-settings-changed", &settings)
        .map_err(|error| CompanionCommandError::Event(error.to_string()))?;
    Ok(settings)
}

pub(crate) fn set_known_window_visible<R: Runtime>(
    label: &str,
    visible: bool,
    app: &tauri::AppHandle<R>,
) -> Result<(), CompanionCommandError> {
    let window = known_window(label, app)?;
    if visible {
        window.show()
    } else {
        window.hide()
    }
    .map_err(|error| CompanionCommandError::Window(error.to_string()))
}

pub fn handle_window_event<R: Runtime>(window: &tauri::Window<R>, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        match window.label() {
            "companion" => {
                api.prevent_close();
                let _ = handle_companion_close(window.app_handle());
            }
            "main" => window.app_handle().exit(0),
            _ => {}
        }
    }
}

pub(crate) fn handle_companion_close<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<CompanionSettings, CompanionCommandError> {
    let repository = app.state::<CompanionRepository>();
    set_companion_visibility(false, app, &repository)
}

fn set_companion_visibility<R: Runtime>(
    visible: bool,
    app: &tauri::AppHandle<R>,
    repository: &CompanionRepository,
) -> Result<CompanionSettings, CompanionCommandError> {
    set_known_window_visible("companion", visible, app)?;
    let mut settings = repository.get_settings()?;
    settings.visible = visible;
    repository.save_settings(&settings)?;
    app.emit("companion-visibility-changed", &settings)
        .map_err(|error| CompanionCommandError::Event(error.to_string()))?;
    Ok(settings)
}

fn known_window<R: Runtime>(
    label: &str,
    app: &tauri::AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, CompanionCommandError> {
    if !matches!(label, "main" | "companion") {
        return Err(CompanionCommandError::UnknownWindow);
    }
    app.get_webview_window(label)
        .ok_or_else(|| CompanionCommandError::WindowUnavailable(label.to_owned()))
}

fn emit_skin_state<R: Runtime>(
    app: &tauri::AppHandle<R>,
    repository: &CompanionRepository,
) -> Result<CompanionSkinState, CompanionCommandError> {
    let settings = repository.get_settings()?;
    let state = CompanionSkinState {
        active_skin_id: settings.active_skin_id,
        skins: repository.list_skins()?,
    };
    app.emit("companion-skin-changed", &state)
        .map_err(|error| CompanionCommandError::Event(error.to_string()))?;
    Ok(state)
}

pub fn anchored_companion_bounds(
    current: LogicalPosition<f64>,
    monitor: LogicalRect<f64, f64>,
    expanded: bool,
) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let target = if expanded {
        EXPANDED_SIZE
    } else {
        COLLAPSED_SIZE
    };
    let current_size = if expanded {
        COLLAPSED_SIZE
    } else {
        EXPANDED_SIZE
    };
    let monitor_right = monitor.position.x + monitor.size.width;
    let monitor_bottom = monitor.position.y + monitor.size.height;
    let anchor_right =
        current.x + current_size.width / 2.0 >= monitor.position.x + monitor.size.width / 2.0;
    let anchor_bottom =
        current.y + current_size.height / 2.0 >= monitor.position.y + monitor.size.height / 2.0;
    let x = if anchor_right {
        current.x + current_size.width - target.width
    } else {
        current.x
    };
    let y = if anchor_bottom {
        current.y + current_size.height - target.height
    } else {
        current.y
    };
    let maximum_x = (monitor_right - target.width).max(monitor.position.x);
    let maximum_y = (monitor_bottom - target.height).max(monitor.position.y);

    (
        LogicalPosition::new(
            x.clamp(monitor.position.x, maximum_x),
            y.clamp(monitor.position.y, maximum_y),
        ),
        target,
    )
}
