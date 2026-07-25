use chrono::{DateTime, Utc};
use rusqlite::{Connection, ErrorCode};

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::{AssetRepository, AssetRepositoryError},
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
        favorite: false,
        sync_version: 1,
    }
}

#[test]
fn lists_only_assets_from_requested_month() {
    let repo = test_repository();
    repo.create(&new_asset("2026-07-25T12:00:00Z")).unwrap();
    repo.create(&new_asset("2026-08-01T12:00:00Z")).unwrap();

    assert_eq!(repo.list_by_month(2026, 7).unwrap().len(), 1);
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
