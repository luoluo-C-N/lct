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

fn imported_skin() -> CompanionSkin {
    CompanionSkin {
        id: "imported-skin".to_owned(),
        name: "Imported skin".to_owned(),
        source: SkinSource::Imported,
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
