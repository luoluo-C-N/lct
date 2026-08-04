use chrono::{DateTime, Utc};
use serde_json::json;
use tauri::State;

use super::asset::{Asset, AssetSource};
use crate::{
    commands::assets::{list_assets_by_day, list_assets_by_month},
    repository::assets::AssetRepository,
};

#[test]
fn serializes_asset_with_the_frontend_ipc_field_contract() {
    assert_month_query_signature(list_assets_by_month);
    assert_day_query_signature(list_assets_by_day);

    let asset = Asset {
        id: "asset-1".to_owned(),
        created_at: timestamp("2026-07-25T12:00:00Z"),
        imported_at: timestamp("2026-07-25T12:01:00Z"),
        source: AssetSource::Import,
        original_path: "C:/assets/original.png".into(),
        preview_path: "C:/assets/previews/preview.png".into(),
        display_name: "sunset.png".to_owned(),
        album_id: Some("album-1".to_owned()),
        tags: vec!["magic".to_owned()],
        favorite: true,
        deleted_at: None,
        capture_mode: None,
        annotation_data: None,
        sync_version: 7,
        cloud_id: None,
    };

    assert_eq!(
        serde_json::to_value(vec![asset]).unwrap(),
        json!([{
            "id": "asset-1",
            "createdAt": "2026-07-25T12:00:00Z",
            "importedAt": "2026-07-25T12:01:00Z",
            "source": "import",
            "originalPath": "C:/assets/original.png",
            "previewPath": "C:/assets/previews/preview.png",
            "displayName": "sunset.png",
            "albumId": "album-1",
            "tags": ["magic"],
            "favorite": true,
            "deletedAt": null,
            "captureMode": null,
            "annotationData": null,
            "syncVersion": 7
            ,"cloudId": null
        }])
    );
}

fn assert_month_query_signature(
    _query: for<'a> fn(i32, u32, State<'a, AssetRepository>) -> Result<Vec<Asset>, String>,
) {
}

fn assert_day_query_signature(
    _query: for<'a> fn(i32, u32, u32, State<'a, AssetRepository>) -> Result<Vec<Asset>, String>,
) {
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
