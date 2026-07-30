use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::{
    domain::companion::{
        CompanionSettings, CompanionSkin, SkinSource, VisualPreset, WindowPlacement,
    },
    repository::{
        assets::AssetRepository,
        companion::{CompanionRepository, CompanionRepositoryError},
    },
};

fn v2_database_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "magic-image-library-companion-test-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn v2_connection_with_one_asset(path: &PathBuf) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO app_meta (key, value) VALUES ('schema_version', '2');
             CREATE TABLE assets (
                id TEXT PRIMARY KEY NOT NULL,
                created_at TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                source TEXT NOT NULL,
                original_path TEXT NOT NULL,
                preview_path TEXT NOT NULL,
                album_id TEXT,
                favorite INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT,
                capture_mode TEXT,
                annotation_data TEXT,
                sync_version INTEGER NOT NULL DEFAULT 0,
                cloud_id TEXT
             );
             INSERT INTO assets (
                id, created_at, imported_at, source, original_path, preview_path, favorite, sync_version
             ) VALUES ('existing', '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z', 'import', 'original', 'preview', 0, 0);",
        )
        .unwrap();
    connection
}

fn in_memory_repository() -> CompanionRepository {
    CompanionRepository::from_connection(Connection::open_in_memory().unwrap()).unwrap()
}

#[test]
fn migrates_v2_and_seeds_builtin_skins_without_losing_assets() {
    let path = v2_database_path();
    let connection = v2_connection_with_one_asset(&path);
    drop(connection);
    let repository = CompanionRepository::open(&path).unwrap();

    assert_eq!(repository.list_skins().unwrap().len(), 3);
    assert_eq!(
        repository.get_settings().unwrap().active_skin_id,
        "quiet-aurora"
    );
    drop(repository);

    let assets = AssetRepository::open(&path)
        .unwrap()
        .list_by_month(2026, 7)
        .unwrap();
    assert_eq!(
        assets
            .iter()
            .map(|asset| asset.id.as_str())
            .collect::<Vec<_>>(),
        vec!["existing"]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn protects_builtin_and_active_skins() {
    let repository = in_memory_repository();

    assert!(matches!(
        repository.delete_skin("quiet-aurora"),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));

    let skin = imported_skin();
    repository.create_skin(&skin).unwrap();
    repository.set_active_skin(&skin.id).unwrap();
    assert!(matches!(
        repository.delete_skin(&skin.id),
        Err(CompanionRepositoryError::ActiveSkin)
    ));
}

#[test]
fn rejects_external_builtin_skins_and_reserved_skin_mutations() {
    let repository = in_memory_repository();
    let mut forged_builtin = imported_skin();
    forged_builtin.id = "forged-builtin".to_owned();
    forged_builtin.source = SkinSource::Builtin;

    assert!(matches!(
        repository.create_skin(&forged_builtin),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));
    assert!(!repository
        .list_skins()
        .unwrap()
        .iter()
        .any(|skin| skin.id == "forged-builtin"));

    let mut forged_reserved_id = imported_skin();
    forged_reserved_id.id = "deep-ink".to_owned();
    assert!(matches!(
        repository.create_skin(&forged_reserved_id),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));

    let mut porcelain = CompanionSkin::builtin(VisualPreset::PorcelainPearl);
    porcelain.source = SkinSource::Image;
    porcelain.flow_colors = vec!["#000000".to_owned()];

    assert!(matches!(
        repository.update_skin(&porcelain),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));
    assert!(matches!(
        repository.delete_skin("porcelain-pearl"),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));
    let stored_porcelain = repository
        .list_skins()
        .unwrap()
        .into_iter()
        .find(|skin| skin.id == "porcelain-pearl")
        .unwrap();
    assert_eq!(stored_porcelain.source, SkinSource::Builtin);
    assert_eq!(stored_porcelain.flow_colors, vec!["#E3BD7E", "#FFF8EA"]);
}

#[test]
fn rejects_updates_for_unknown_skins() {
    let repository = in_memory_repository();
    let mut missing = imported_skin();
    missing.id = "missing-skin".to_owned();

    assert!(matches!(
        repository.update_skin(&missing),
        Err(CompanionRepositoryError::SkinNotFound(id)) if id == "missing-skin"
    ));
}

#[test]
fn rollback_removes_an_imported_skin_even_when_it_is_active() {
    let repository = in_memory_repository();
    let skin = imported_skin();
    repository.create_skin(&skin).unwrap();
    repository.set_active_skin(&skin.id).unwrap();

    repository.rollback_imported_skin(&skin.id).unwrap();

    assert!(!repository
        .list_skins()
        .unwrap()
        .iter()
        .any(|stored| stored.id == skin.id));
    assert_eq!(
        repository.get_settings().unwrap().active_skin_id,
        "quiet-aurora"
    );
    assert_eq!(repository.list_skins().unwrap().len(), 3);
}

#[test]
fn persists_skin_changes_and_window_settings() {
    let repository = in_memory_repository();
    let mut skin = imported_skin();
    repository.create_skin(&skin).unwrap();

    skin.name = "Updated imported skin".to_owned();
    skin.flow_colors = vec!["#111111".to_owned(), "#EEEEEE".to_owned()];
    repository.update_skin(&skin).unwrap();
    repository.set_active_skin(&skin.id).unwrap();
    repository
        .save_settings(&CompanionSettings {
            active_skin_id: skin.id.clone(),
            motion_enabled: false,
            visible: false,
            placement: Some(WindowPlacement {
                x: 24.0,
                y: 48.0,
                width: 360.0,
                height: 480.0,
            }),
        })
        .unwrap();

    let stored_skin = repository
        .list_skins()
        .unwrap()
        .into_iter()
        .find(|stored| stored.id == skin.id)
        .unwrap();
    assert_eq!(stored_skin.name, "Updated imported skin");
    assert_eq!(stored_skin.flow_colors, vec!["#111111", "#EEEEEE"]);
    assert_eq!(
        repository.get_settings().unwrap(),
        CompanionSettings {
            active_skin_id: skin.id,
            motion_enabled: false,
            visible: false,
            placement: Some(WindowPlacement {
                x: 24.0,
                y: 48.0,
                width: 360.0,
                height: 480.0,
            }),
        }
    );
}

#[test]
fn round_trips_image_and_package_skin_sources() {
    let repository = in_memory_repository();
    let image = imported_skin();
    let mut package = imported_skin();
    package.id = "package-skin".to_owned();
    package.source = SkinSource::Package;

    repository.create_skin(&image).unwrap();
    repository.create_skin(&package).unwrap();

    let stored = repository.list_skins().unwrap();
    assert_eq!(
        stored
            .iter()
            .find(|skin| skin.id == image.id)
            .unwrap()
            .source,
        SkinSource::Image
    );
    assert_eq!(
        stored
            .iter()
            .find(|skin| skin.id == package.id)
            .unwrap()
            .source,
        SkinSource::Package
    );
}

fn imported_skin() -> CompanionSkin {
    CompanionSkin {
        id: "imported-skin".to_owned(),
        name: "Imported skin".to_owned(),
        source: SkinSource::Image,
        visual_preset: VisualPreset::DeepInk,
        texture_path: Some("C:/skins/texture.png".into()),
        preview_path: Some("C:/skins/preview.png".into()),
        flow_colors: vec!["#123456".to_owned()],
        flow_speed: 1.2,
        flow_intensity: 0.8,
        created_at: DateTime::parse_from_rfc3339("2026-07-30T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    }
}
