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
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_directory)?;
            app.manage(repository::assets::AssetRepository::open(
                data_directory.join("assets.sqlite3"),
            )?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::assets::create_asset,
            commands::assets::import_files,
            commands::assets::capture,
            commands::assets::list_assets_by_month,
            commands::assets::list_assets_by_day,
            commands::assets::set_asset_tags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Magic Image Library");
}
