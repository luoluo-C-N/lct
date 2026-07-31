use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::AssetRepository,
    services::import::{new_asset_id, persist_image, ImportError},
};

pub use crate::domain::asset::CaptureMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CropRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("a crop region is required for region capture")]
    MissingCropRegion,
    #[error("the crop region is outside the captured image")]
    InvalidCropRegion,
    #[error("no focused window is available to capture")]
    NoFocusedWindow,
    #[error("unable to read the focused window bounds: {0}")]
    WindowRect(String),
    #[error("unable to capture the screen: {0}")]
    Screenshot(String),
    #[error(transparent)]
    Import(#[from] ImportError),
}

pub trait WindowLocator: Send + Sync {
    fn foreground_window_rect(&self) -> Result<CropRegion, CaptureError>;
}

pub struct Win32WindowLocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureTarget {
    Fullscreen,
    PrimaryRegion(CropRegion),
    VirtualDesktopRegion(CropRegion),
}

#[cfg(target_os = "windows")]
impl WindowLocator for Win32WindowLocator {
    fn foreground_window_rect(&self) -> Result<CropRegion, CaptureError> {
        use windows_sys::Win32::{
            Foundation::RECT,
            UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect},
        };

        let window = unsafe { GetForegroundWindow() };
        if window.is_null() {
            return Err(CaptureError::NoFocusedWindow);
        }

        let mut rect = RECT::default();
        if unsafe { GetWindowRect(window, &mut rect) } == 0 {
            return Err(CaptureError::WindowRect(
                std::io::Error::last_os_error().to_string(),
            ));
        }

        let screens = screenshots::Screen::all()
            .map_err(|error| CaptureError::Screenshot(error.to_string()))?;
        let origin_x = screens
            .iter()
            .map(|screen| screen.display_info.x)
            .min()
            .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;
        let origin_y = screens
            .iter()
            .map(|screen| screen.display_info.y)
            .min()
            .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;

        normalize_window_region(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            origin_x,
            origin_y,
        )
    }
}

#[cfg(not(target_os = "windows"))]
impl WindowLocator for Win32WindowLocator {
    fn foreground_window_rect(&self) -> Result<CropRegion, CaptureError> {
        Err(CaptureError::Screenshot(
            "window capture is only supported on Windows".to_owned(),
        ))
    }
}

pub(crate) fn normalize_window_region(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    origin_x: i32,
    origin_y: i32,
) -> Result<CropRegion, CaptureError> {
    let x = left
        .checked_sub(origin_x)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(CaptureError::InvalidCropRegion)?;
    let y = top
        .checked_sub(origin_y)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(CaptureError::InvalidCropRegion)?;
    let width = right
        .checked_sub(left)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or(CaptureError::InvalidCropRegion)?;
    let height = bottom
        .checked_sub(top)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or(CaptureError::InvalidCropRegion)?;

    Ok(CropRegion {
        x,
        y,
        width,
        height,
    })
}

pub(crate) fn resolve_capture_target(
    mode: CaptureMode,
    region: Option<CropRegion>,
    window_locator: &dyn WindowLocator,
) -> Result<CaptureTarget, CaptureError> {
    match mode {
        CaptureMode::Fullscreen => Ok(CaptureTarget::Fullscreen),
        CaptureMode::Region => validate_region(mode, region)?
            .map_or(Err(CaptureError::MissingCropRegion), |region| {
                Ok(CaptureTarget::PrimaryRegion(region))
            }),
        CaptureMode::Window => window_locator
            .foreground_window_rect()
            .map(CaptureTarget::VirtualDesktopRegion),
    }
}

pub(crate) fn crop_image(
    image: image::RgbaImage,
    region: CropRegion,
) -> Result<image::RgbaImage, CaptureError> {
    let right = region
        .x
        .checked_add(region.width)
        .ok_or(CaptureError::InvalidCropRegion)?;
    let bottom = region
        .y
        .checked_add(region.height)
        .ok_or(CaptureError::InvalidCropRegion)?;
    if region.width == 0 || region.height == 0 || right > image.width() || bottom > image.height() {
        return Err(CaptureError::InvalidCropRegion);
    }
    Ok(
        image::imageops::crop_imm(&image, region.x, region.y, region.width, region.height)
            .to_image(),
    )
}

pub(crate) fn validate_region(
    mode: CaptureMode,
    region: Option<CropRegion>,
) -> Result<Option<CropRegion>, CaptureError> {
    if matches!(mode, CaptureMode::Region) && region.is_none() {
        return Err(CaptureError::MissingCropRegion);
    }
    Ok(region)
}

pub fn capture(
    mode: CaptureMode,
    region: Option<CropRegion>,
    window_locator: &dyn WindowLocator,
    data_directory: &Path,
    repository: &AssetRepository,
) -> Result<Asset, CaptureError> {
    let target = resolve_capture_target(mode, region, window_locator)?;
    let screenshot = capture_target(target)?;

    let captures_directory = data_directory.join("assets").join("captures");
    fs::create_dir_all(&captures_directory)?;
    let source_path = captures_directory.join(format!("capture-{}.png", new_asset_id()));
    screenshot
        .save(&source_path)
        .map_err(|error| CaptureError::Screenshot(error.to_string()))?;

    persist_captured_image(&source_path, mode, data_directory, repository)
}

fn capture_target(target: CaptureTarget) -> Result<screenshots::image::RgbaImage, CaptureError> {
    let screens =
        screenshots::Screen::all().map_err(|error| CaptureError::Screenshot(error.to_string()))?;
    let primary = screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .or_else(|| screens.first())
        .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;

    match target {
        CaptureTarget::Fullscreen => primary
            .capture()
            .map_err(|error| CaptureError::Screenshot(error.to_string())),
        CaptureTarget::PrimaryRegion(region) => {
            let screenshot = primary
                .capture()
                .map_err(|error| CaptureError::Screenshot(error.to_string()))?;
            let right = region
                .x
                .checked_add(region.width)
                .ok_or(CaptureError::InvalidCropRegion)?;
            let bottom = region
                .y
                .checked_add(region.height)
                .ok_or(CaptureError::InvalidCropRegion)?;
            if region.width == 0
                || region.height == 0
                || right > screenshot.width()
                || bottom > screenshot.height()
            {
                return Err(CaptureError::InvalidCropRegion);
            }
            Ok(screenshots::image::imageops::crop_imm(
                &screenshot,
                region.x,
                region.y,
                region.width,
                region.height,
            )
            .to_image())
        }
        CaptureTarget::VirtualDesktopRegion(region) => {
            let origin_x = screens
                .iter()
                .map(|screen| screen.display_info.x)
                .min()
                .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;
            let origin_y = screens
                .iter()
                .map(|screen| screen.display_info.y)
                .min()
                .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;
            let virtual_x = i64::from(origin_x) + i64::from(region.x);
            let virtual_y = i64::from(origin_y) + i64::from(region.y);
            let virtual_x =
                i32::try_from(virtual_x).map_err(|_| CaptureError::InvalidCropRegion)?;
            let virtual_y =
                i32::try_from(virtual_y).map_err(|_| CaptureError::InvalidCropRegion)?;
            let screen = screens
                .iter()
                .find(|screen| {
                    let info = screen.display_info;
                    virtual_x >= info.x
                        && virtual_y >= info.y
                        && i64::from(virtual_x) < i64::from(info.x) + i64::from(info.width)
                        && i64::from(virtual_y) < i64::from(info.y) + i64::from(info.height)
                })
                .ok_or(CaptureError::InvalidCropRegion)?;
            screen
                .capture_area(
                    virtual_x - screen.display_info.x,
                    virtual_y - screen.display_info.y,
                    region.width,
                    region.height,
                )
                .map_err(|error| CaptureError::Screenshot(error.to_string()))
        }
    }
}

pub(crate) fn persist_captured_image(
    source_path: &Path,
    mode: CaptureMode,
    data_directory: &Path,
    repository: &AssetRepository,
) -> Result<Asset, CaptureError> {
    let result = persist_image(
        source_path,
        data_directory,
        repository,
        AssetSource::Capture,
        Some(mode),
    )
    .map_err(Into::into);
    let _ = fs::remove_file(source_path);
    result
}

impl From<std::io::Error> for CaptureError {
    fn from(error: std::io::Error) -> Self {
        Self::Screenshot(error.to_string())
    }
}
