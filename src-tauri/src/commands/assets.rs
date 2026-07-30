use std::path::{Path, PathBuf};

use tauri::{Emitter, Manager, State};

use crate::{
    domain::asset::Asset,
    repository::assets::AssetRepository,
    services::{
        capture::{self, CaptureMode, CropRegion},
        import,
    },
};

#[cfg(debug_assertions)]
#[tauri::command]
pub fn create_asset(asset: Asset, repository: State<'_, AssetRepository>) -> Result<(), String> {
    repository.create(&asset).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_files<R: tauri::Runtime>(
    paths: Vec<PathBuf>,
    app: tauri::AppHandle<R>,
    repository: State<'_, AssetRepository>,
) -> Result<Vec<Asset>, String> {
    let data_directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    let mut assets = Vec::with_capacity(paths.len());
    for path in paths {
        let asset = import::import_file(&path, &data_directory, &repository)
            .map_err(|error| error.to_string())?;
        app.emit("asset-created", &asset)
            .map_err(|error| error.to_string())?;
        assets.push(asset);
    }
    Ok(assets)
}

#[tauri::command]
pub fn capture<R: tauri::Runtime>(
    mode: CaptureMode,
    region: Option<CropRegion>,
    app: tauri::AppHandle<R>,
    repository: State<'_, AssetRepository>,
) -> Result<Asset, String> {
    capture_with(mode, region, app, repository, capture::capture)
}

pub(crate) fn capture_with<R, F>(
    mode: CaptureMode,
    region: Option<CropRegion>,
    app: tauri::AppHandle<R>,
    repository: State<'_, AssetRepository>,
    capture_image: F,
) -> Result<Asset, String>
where
    R: tauri::Runtime,
    F: FnOnce(
        CaptureMode,
        Option<CropRegion>,
        &Path,
        &AssetRepository,
    ) -> Result<Asset, capture::CaptureError>,
{
    let data_directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    let asset = capture_image(mode, region, &data_directory, &repository)
        .map_err(|error| error.to_string())?;
    app.emit("asset-created", &asset)
        .map_err(|error| error.to_string())?;
    Ok(asset)
}

#[tauri::command]
pub fn list_assets_by_month(
    year: i32,
    month: u32,
    repository: State<'_, AssetRepository>,
) -> Result<Vec<Asset>, String> {
    repository
        .list_by_month(year, month)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_assets_by_day(
    year: i32,
    month: u32,
    day: u32,
    repository: State<'_, AssetRepository>,
) -> Result<Vec<Asset>, String> {
    repository
        .list_by_day(year, month, day)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_asset_tags(
    asset_id: String,
    tags: Vec<String>,
    repository: State<'_, AssetRepository>,
) -> Result<(), String> {
    repository
        .set_tags(&asset_id, &tags)
        .map_err(|error| error.to_string())
}
