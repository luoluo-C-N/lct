use chrono::{DateTime, Utc};

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::AssetRepository,
};

fn test_repository() -> AssetRepository {
    AssetRepository::in_memory().unwrap()
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
