use tauri::State;

use crate::{domain::asset::Asset, repository::assets::AssetRepository};

#[tauri::command]
pub fn create_asset(asset: Asset, repository: State<'_, AssetRepository>) -> Result<(), String> {
    repository.create(&asset).map_err(|error| error.to_string())
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
