use std::path::Path;

use tauri::Manager;

mod commands;
mod domain;
mod repository;
mod services;

#[cfg(test)]
#[path = "asset_protocol_config_test.rs"]
mod asset_protocol_config_test;

fn manage_repositories<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    database_path: &Path,
) -> Result<domain::companion::CompanionSettings, String> {
    let asset_repository = repository::assets::AssetRepository::open(database_path)
        .map_err(|error| error.to_string())?;
    let companion_repository = repository::companion::CompanionRepository::open(database_path)
        .map_err(|error| error.to_string())?;
    let companion_settings = companion_repository
        .get_settings()
        .map_err(|error| error.to_string())?;
    manage_repository_states(app, asset_repository, companion_repository)?;
    Ok(companion_settings)
}

fn manage_repository_states<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    asset_repository: repository::assets::AssetRepository,
    companion_repository: repository::companion::CompanionRepository,
) -> Result<(), String> {
    if !app.manage(asset_repository) {
        return Err("asset repository state is already managed".to_owned());
    }
    if !app.manage(companion_repository) {
        return Err("companion repository state is already managed".to_owned());
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::companion::CompanionRegionSelectionState::default())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_directory)?;
            let database_path = data_directory.join("assets.sqlite3");
            let companion_settings =
                manage_repositories(app.handle(), &database_path).map_err(std::io::Error::other)?;
            if let Some(window) = app.get_webview_window("companion") {
                if let Some(placement) = companion_settings.placement {
                    window.set_position(tauri::LogicalPosition::new(placement.x, placement.y))?;
                    window.set_size(tauri::LogicalSize::new(placement.width, placement.height))?;
                }
                if companion_settings.visible {
                    window.show()?;
                } else {
                    window.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(commands::companion::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            #[cfg(debug_assertions)]
            commands::assets::create_asset,
            commands::assets::import_files,
            commands::assets::capture,
            commands::assets::list_assets_by_month,
            commands::assets::list_assets_by_day,
            commands::assets::set_asset_tags,
            commands::companion::list_companion_skins,
            commands::companion::get_companion_settings,
            commands::companion::import_companion_skin,
            commands::companion::update_companion_skin,
            commands::companion::set_active_companion_skin,
            commands::companion::delete_companion_skin,
            commands::companion::show_companion,
            commands::companion::hide_companion,
            commands::companion::focus_main_window,
            commands::companion::set_companion_expanded,
            commands::companion::begin_companion_region_selection,
            commands::companion::finish_companion_region_selection,
            commands::companion::save_companion_placement,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Magic Image Library");
}
