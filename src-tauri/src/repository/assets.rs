use std::{path::Path, sync::Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, Row};
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
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS assets (
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
            CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS asset_tags (
                asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
                tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (asset_id, tag_id)
            );",
        )?;
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
            row.map_err(AssetRepositoryError::from)
                .and_then(Asset::try_from)
        })
        .collect()
    }
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
            favorite: stored.favorite != 0,
            sync_version: stored.sync_version,
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
