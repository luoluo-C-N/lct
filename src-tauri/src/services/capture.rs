use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::AssetRepository,
    services::import::{persist_image, ImportError},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Region,
    Window,
    Fullscreen,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("unable to capture the screen: {0}")]
    Screenshot(String),
    #[error(transparent)]
    Import(#[from] ImportError),
}

pub fn capture(
    _mode: CaptureMode,
    data_directory: &Path,
    repository: &AssetRepository,
) -> Result<Asset, CaptureError> {
    let screen = screenshots::Screen::all()
        .map_err(|error| CaptureError::Screenshot(error.to_string()))?
        .into_iter()
        .next()
        .ok_or_else(|| CaptureError::Screenshot("no display is available".to_owned()))?;
    let screenshot = screen
        .capture()
        .map_err(|error| CaptureError::Screenshot(error.to_string()))?;

    let captures_directory = data_directory.join("assets").join("captures");
    fs::create_dir_all(&captures_directory)?;
    let source_path = captures_directory.join(format!(
        "capture-{}.png",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    screenshot
        .save(&source_path)
        .map_err(|error| CaptureError::Screenshot(error.to_string()))?;

    persist_captured_image(&source_path, data_directory, repository)
}

pub(crate) fn persist_captured_image(
    source_path: &Path,
    data_directory: &Path,
    repository: &AssetRepository,
) -> Result<Asset, CaptureError> {
    let result = persist_image(
        source_path,
        data_directory,
        repository,
        AssetSource::Capture,
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
