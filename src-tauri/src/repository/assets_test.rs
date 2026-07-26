use chrono::{DateTime, Utc};
use rusqlite::{Connection, ErrorCode};

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::{migrate_schema, AssetRepository, AssetRepositoryError},
};

fn test_repository() -> AssetRepository {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .unwrap();
    AssetRepository::from_connection(connection).unwrap()
}

fn new_asset(created_at: &str) -> Asset {
    Asset {
        id: format!("asset-{created_at}"),
        created_at: DateTime::parse_from_rfc3339(created_at)
            .unwrap()
            .with_timezone(&Utc),
        imported_at: DateTime::parse_from_rfc3339("2026-07-25T12:01:00Z")
            .unwrap()
            .with_timezone(&Utc),
        source: AssetSource::Import,
        original_path: "C:/assets/original.png".into(),
        preview_path: "C:/assets/preview.png".into(),
        album_id: None,
        tags: Vec::new(),
        favorite: false,
        deleted_at: Some(DateTime::parse_from_rfc3339("2026-07-26T00:00:00Z").unwrap().with_timezone(&Utc)),
        capture_mode: Some(crate::domain::asset::CaptureMode::Fullscreen),
        annotation_data: Some("{\"shapes\":[]}".to_owned()),
        sync_version: 1,
        cloud_id: Some("cloud-1".to_owned()),
    }
}

#[test]
fn lists_only_assets_from_requested_month() {
    let repo = test_repository();
    repo.create(&new_asset("2026-07-25T12:00:00Z")).unwrap();
    repo.create(&new_asset("2026-08-01T12:00:00Z")).unwrap();

    let assets = repo.list_by_month(2026, 7).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].capture_mode, Some(crate::domain::asset::CaptureMode::Fullscreen));
    assert_eq!(assets[0].cloud_id.as_deref(), Some("cloud-1"));
}

#[test]
fn rejects_tags_for_a_missing_asset() {
    let repo = test_repository();

    let error = repo
        .set_tags("missing-asset", &["reference".to_owned()])
        .unwrap_err();

    assert!(matches!(
        error,
        AssetRepositoryError::Database(rusqlite::Error::SqliteFailure(sqlite_error, _))
            if sqlite_error.code == ErrorCode::ConstraintViolation
    ));
}

#[test]
fn loads_tags_from_the_relationship_tables() {
    let repo = test_repository();
    let asset = new_asset("2026-07-25T12:00:00Z");
    repo.create(&asset).unwrap();
    repo.set_tags(&asset.id, &["reference".to_owned(), "magic".to_owned()])
        .unwrap();

    let assets = repo.list_by_month(2026, 7).unwrap();

    assert_eq!(assets[0].tags, vec!["magic", "reference"]);
}

#[test]
fn migrates_a_v1_database_to_v2_without_rebuilding_assets() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE assets (
                id TEXT PRIMARY KEY NOT NULL,
                created_at TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                source TEXT NOT NULL,
                original_path TEXT NOT NULL,
                preview_path TEXT NOT NULL,
                album_id TEXT,
                favorite INTEGER NOT NULL DEFAULT 0,
                sync_version INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO assets (
                id, created_at, imported_at, source, original_path, preview_path, favorite, sync_version
            ) VALUES ('existing', '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z', 'import', 'original', 'preview', 0, 0);",
        )
        .unwrap();

    migrate_schema(&connection).unwrap();

    let columns = connection
        .prepare("SELECT name FROM pragma_table_info('assets') ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let version: String = connection
        .query_row("SELECT value FROM app_meta WHERE key = 'schema_version'", [], |row| row.get(0))
        .unwrap();

    assert!(columns.contains(&"deleted_at".to_owned()));
    assert!(columns.contains(&"capture_mode".to_owned()));
    assert!(columns.contains(&"annotation_data".to_owned()));
    assert!(columns.contains(&"cloud_id".to_owned()));
    assert_eq!(version, "2");
    assert_eq!(
        connection
            .query_row("SELECT id FROM assets WHERE id = 'existing'", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "existing"
    );
}
