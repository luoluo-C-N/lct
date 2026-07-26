use std::{path::Path, sync::Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use thiserror::Error;

use crate::domain::asset::{Asset, AssetSource};

pub struct AssetRepository {
    connection: Mutex<Connection>,
}

#[derive(Debug, Error)]
pub enum AssetRepositoryError {
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error("invalid date: {year:04}-{month:02}-{day:02}")]
    InvalidDate { year: i32, month: u32, day: u32 },
    #[error("asset contains an unsupported source: {0}")]
    InvalidSource(String),
    #[error("asset has an invalid {field} timestamp: {value}")]
    InvalidTimestamp { field: &'static str, value: String },
    #[error("unsupported asset schema version: {0}")]
    UnsupportedSchemaVersion(String),
}

#[derive(Debug)]
struct StoredAsset {
    id: String,
    created_at: String,
    imported_at: String,
    source: String,
    original_path: String,
    preview_path: String,
    album_id: Option<String>,
    favorite: i64,
    sync_version: i64,
}

impl AssetRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AssetRepositoryError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn create(&self, asset: &Asset) -> Result<(), AssetRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("asset repository lock poisoned");
        connection.execute(
            "INSERT INTO assets (
                id, created_at, imported_at, source, original_path, preview_path, album_id, favorite, sync_version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                asset.id,
                asset.created_at.to_rfc3339(),
                asset.imported_at.to_rfc3339(),
                asset.source.as_str(),
                asset.original_path.to_string_lossy(),
                asset.preview_path.to_string_lossy(),
                asset.album_id,
                i64::from(asset.favorite),
                asset.sync_version,
            ],
        )?;
        Ok(())
    }

    pub fn list_by_month(&self, year: i32, month: u32) -> Result<Vec<Asset>, AssetRepositoryError> {
        let start = date_start(year, month, 1)?;
        let end = if month == 12 {
            date_start(year + 1, 1, 1)?
        } else {
            date_start(year, month + 1, 1)?
        };
        self.list_between(start, end)
    }

    pub fn list_by_day(
        &self,
        year: i32,
        month: u32,
        day: u32,
    ) -> Result<Vec<Asset>, AssetRepositoryError> {
        let start = date_start(year, month, day)?;
        let end = start + chrono::Duration::days(1);
        self.list_between(start, end)
    }

    pub fn set_tags(&self, asset_id: &str, tags: &[String]) -> Result<(), AssetRepositoryError> {
        let mut connection = self
            .connection
            .lock()
            .expect("asset repository lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM asset_tags WHERE asset_id = ?1",
            params![asset_id],
        )?;

        for tag in tags {
            transaction.execute(
                "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                params![tag],
            )?;
            transaction.execute(
                "INSERT INTO asset_tags (asset_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2",
                params![asset_id, tag],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn from_connection(connection: Connection) -> Result<Self, AssetRepositoryError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn list_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Asset>, AssetRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("asset repository lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, created_at, imported_at, source, original_path, preview_path, album_id, favorite, sync_version
             FROM assets
             WHERE created_at >= ?1 AND created_at < ?2
             ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339()],
            stored_asset_from_row,
        )?;

        rows.map(|row| {
            let stored = row.map_err(AssetRepositoryError::from)?;
            let tags = load_tags(&connection, &stored.id)?;
            let mut asset = Asset::try_from(stored)?;
            asset.tags = tags;
            Ok(asset)
        }).collect()
    }
}

fn load_tags(connection: &Connection, asset_id: &str) -> Result<Vec<String>, AssetRepositoryError> {
    let mut statement = connection.prepare(
        "SELECT tags.name FROM tags
         INNER JOIN asset_tags ON asset_tags.tag_id = tags.id
         WHERE asset_tags.asset_id = ?1
         ORDER BY tags.name",
    )?;
    let tags = statement
        .query_map(params![asset_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(AssetRepositoryError::from)?;
    Ok(tags)
}

pub(crate) fn migrate_schema(connection: &Connection) -> Result<(), AssetRepositoryError> {
    let assets_exist = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'assets')",
        [],
        |row| row.get::<_, bool>(0),
    )?;

    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    let version = connection
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    match version.as_deref() {
        Some("2") => {}
        Some("1") | None if assets_exist => {
            connection.execute_batch(
                "ALTER TABLE assets ADD COLUMN deleted_at TEXT;
                 ALTER TABLE assets ADD COLUMN capture_mode TEXT;
                 ALTER TABLE assets ADD COLUMN annotation_data TEXT;
                 ALTER TABLE assets ADD COLUMN cloud_id TEXT;",
            )?;
        }
        None => {
            connection.execute_batch(
                "CREATE TABLE assets (
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
                );",
            )?;
        }
        Some(version) => {
            return Err(AssetRepositoryError::UnsupportedSchemaVersion(version.to_owned()));
        }
    }

    connection.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
         CREATE INDEX IF NOT EXISTS idx_assets_deleted_at ON assets(deleted_at);
         CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
         );
         CREATE TABLE IF NOT EXISTS asset_tags (
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (asset_id, tag_id)
         );
         INSERT INTO app_meta(key, value) VALUES ('schema_version', '2')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
    )?;
    Ok(())
}

fn date_start(year: i32, month: u32, day: u32) -> Result<DateTime<Utc>, AssetRepositoryError> {
    NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| date.and_utc())
        .ok_or(AssetRepositoryError::InvalidDate { year, month, day })
}

fn stored_asset_from_row(row: &Row<'_>) -> rusqlite::Result<StoredAsset> {
    Ok(StoredAsset {
        id: row.get(0)?,
        created_at: row.get(1)?,
        imported_at: row.get(2)?,
        source: row.get(3)?,
        original_path: row.get(4)?,
        preview_path: row.get(5)?,
        album_id: row.get(6)?,
        favorite: row.get(7)?,
        sync_version: row.get(8)?,
    })
}

impl TryFrom<StoredAsset> for Asset {
    type Error = AssetRepositoryError;

    fn try_from(stored: StoredAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            id: stored.id,
            created_at: parse_timestamp("created_at", stored.created_at)?,
            imported_at: parse_timestamp("imported_at", stored.imported_at)?,
            source: AssetSource::parse(&stored.source)
                .ok_or(AssetRepositoryError::InvalidSource(stored.source))?,
            original_path: stored.original_path.into(),
            preview_path: stored.preview_path.into(),
            album_id: stored.album_id,
            tags: Vec::new(),
            favorite: stored.favorite != 0,
            deleted_at: None,
            capture_mode: None,
            annotation_data: None,
            sync_version: stored.sync_version,
            cloud_id: None,
        })
    }
}

fn parse_timestamp(
    field: &'static str,
    value: String,
) -> Result<DateTime<Utc>, AssetRepositoryError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|_| AssetRepositoryError::InvalidTimestamp { field, value })
}
