use tauri::Manager;

mod commands;
mod domain;
mod repository;
mod services;

#[cfg(test)]
#[path = "asset_protocol_config_test.rs"]
mod asset_protocol_config_test;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_directory)?;
            let database_path = data_directory.join("assets.sqlite3");
            app.manage(repository::assets::AssetRepository::open(&database_path)?);
            let companion_repository =
                repository::companion::CompanionRepository::open(&database_path)?;
            let companion_settings = companion_repository.get_settings()?;
            app.manage(companion_repository);
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
            commands::companion::save_companion_placement,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Magic Image Library");
}
