use std::{path::Path, sync::Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, params_from_iter, types::ToSql, Connection, OptionalExtension, Row};
use thiserror::Error;

use crate::domain::asset::{Asset, AssetCursor, AssetPage, AssetQuery, AssetSource};
use crate::domain::companion::CompanionSkin;

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
    display_name: String,
    album_id: Option<String>,
    favorite: i64,
    deleted_at: Option<String>,
    capture_mode: Option<String>,
    annotation_data: Option<String>,
    sync_version: i64,
    cloud_id: Option<String>,
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
                id, created_at, imported_at, source, original_path, preview_path, display_name, album_id, favorite, deleted_at, capture_mode, annotation_data, sync_version, cloud_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                asset.id,
                asset.created_at.to_rfc3339(),
                asset.imported_at.to_rfc3339(),
                asset.source.as_str(),
                asset.original_path.to_string_lossy(),
                asset.preview_path.to_string_lossy(),
                asset.display_name,
                asset.album_id,
                i64::from(asset.favorite),
                asset.deleted_at.as_ref().map(|value| value.to_rfc3339()),
                asset.capture_mode.map(|value| value.as_str()),
                asset.annotation_data.as_deref(),
                asset.sync_version,
                asset.cloud_id.as_deref(),
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

    pub fn get_by_id(&self, id: &str) -> Result<Option<Asset>, AssetRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("asset repository lock poisoned");
        let stored = connection
            .query_row(
                "SELECT id, created_at, imported_at, source, original_path, preview_path, display_name, album_id, favorite, deleted_at, capture_mode, annotation_data, sync_version, cloud_id
                 FROM assets WHERE id = ?1",
                params![id],
                stored_asset_from_row,
            )
            .optional()?;
        stored
            .map(|stored| asset_with_tags(&connection, stored))
            .transpose()
    }

    pub fn query(&self, query: &AssetQuery) -> Result<AssetPage, AssetRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("asset repository lock poisoned");
        let limit = if query.limit == 0 {
            60
        } else {
            query.limit.min(120)
        };
        let sort_column = if query.deleted {
            "deleted_at"
        } else {
            "created_at"
        };
        let mut sql = format!(
            "SELECT id, created_at, imported_at, source, original_path, preview_path, display_name, album_id, favorite, deleted_at, capture_mode, annotation_data, sync_version, cloud_id
             FROM assets WHERE {} IS {}NULL",
            if query.deleted { "deleted_at" } else { "deleted_at" },
            if query.deleted { "NOT " } else { "" },
        );
        let mut values: Vec<Box<dyn ToSql>> = Vec::new();

        if let (Some(year), Some(month)) = (query.year, query.month) {
            let start = date_start(year, month, 1)?;
            let end = if month == 12 {
                date_start(year + 1, 1, 1)?
            } else {
                date_start(year, month + 1, 1)?
            };
            sql.push_str(" AND created_at >= ? AND created_at < ?");
            values.push(Box::new(start.to_rfc3339()));
            values.push(Box::new(end.to_rfc3339()));
        }
        if let Some(text) = query
            .text
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            sql.push_str(" AND display_name LIKE ? ESCAPE '\\' COLLATE NOCASE");
            values.push(Box::new(format!("%{}%", escape_like(text))));
        }
        if let Some(source) = query.source {
            sql.push_str(" AND source = ?");
            values.push(Box::new(source.as_str()));
        }
        if query.favorite_only {
            sql.push_str(" AND favorite != 0");
        }
        for tag in query
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
        {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1 FROM asset_tags
                    INNER JOIN tags ON tags.id = asset_tags.tag_id
                    WHERE asset_tags.asset_id = assets.id AND lower(tags.name) = lower(?)
                )",
            );
            values.push(Box::new(tag.to_owned()));
        }
        if let Some(cursor) = &query.cursor {
            sql.push_str(&format!(
                " AND (({sort_column} < ?) OR ({sort_column} = ? AND id < ?))"
            ));
            values.push(Box::new(cursor.sort_timestamp.to_rfc3339()));
            values.push(Box::new(cursor.sort_timestamp.to_rfc3339()));
            values.push(Box::new(cursor.id.clone()));
        }
        sql.push_str(&format!(
            " ORDER BY {sort_column} DESC, id DESC LIMIT {}",
            limit + 1
        ));

        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map(
            params_from_iter(values.iter().map(|value| value.as_ref())),
            stored_asset_from_row,
        )?;
        let mut assets = rows
            .map(|row| {
                row.map_err(AssetRepositoryError::from)
                    .and_then(|stored| asset_with_tags(&connection, stored))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = assets.len() > limit as usize;
        if has_more {
            assets.pop();
        }
        let next_cursor = has_more.then(|| {
            let asset = assets.last().expect("a page with more rows has an item");
            AssetCursor {
                sort_timestamp: if query.deleted {
                    asset
                        .deleted_at
                        .expect("deleted query rows have deleted_at")
                } else {
                    asset.created_at
                },
                id: asset.id.clone(),
            }
        });
        Ok(AssetPage {
            items: assets,
            next_cursor,
        })
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
            "SELECT id, created_at, imported_at, source, original_path, preview_path, display_name, album_id, favorite, deleted_at, capture_mode, annotation_data, sync_version, cloud_id
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
            asset_with_tags(&connection, stored)
        })
        .collect()
    }
}

fn asset_with_tags(
    connection: &Connection,
    stored: StoredAsset,
) -> Result<Asset, AssetRepositoryError> {
    let tags = load_tags(connection, &stored.id)?;
    let mut asset = Asset::try_from(stored)?;
    asset.tags = tags;
    Ok(asset)
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

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
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

    let transaction = connection.unchecked_transaction()?;

    match version.as_deref() {
        Some("4") => {}
        Some("2") | Some("3") => {
            transaction.execute_batch(
                "ALTER TABLE assets ADD COLUMN display_name TEXT NOT NULL DEFAULT '';",
            )?;
        }
        Some("1") | None if assets_exist => {
            transaction.execute_batch(
                "ALTER TABLE assets ADD COLUMN deleted_at TEXT;
                 ALTER TABLE assets ADD COLUMN capture_mode TEXT;
                 ALTER TABLE assets ADD COLUMN annotation_data TEXT;
                 ALTER TABLE assets ADD COLUMN cloud_id TEXT;
                 ALTER TABLE assets ADD COLUMN display_name TEXT NOT NULL DEFAULT '';",
            )?;
        }
        None => {
            transaction.execute_batch(
                "CREATE TABLE assets (
                    id TEXT PRIMARY KEY NOT NULL,
                    created_at TEXT NOT NULL,
                    imported_at TEXT NOT NULL,
                    source TEXT NOT NULL,
                    original_path TEXT NOT NULL,
                    preview_path TEXT NOT NULL,
                    display_name TEXT NOT NULL DEFAULT '',
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
            return Err(AssetRepositoryError::UnsupportedSchemaVersion(
                version.to_owned(),
            ));
        }
    }

    transaction.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
         CREATE INDEX IF NOT EXISTS idx_assets_deleted_at ON assets(deleted_at);
         CREATE INDEX IF NOT EXISTS idx_assets_active_created ON assets(deleted_at, created_at DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_assets_source_created ON assets(source, created_at DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_assets_favorite_created ON assets(favorite, created_at DESC, id DESC);
         CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
         );
         CREATE TABLE IF NOT EXISTS asset_tags (
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (asset_id, tag_id)
         );
         CREATE INDEX IF NOT EXISTS idx_asset_tags_tag_asset ON asset_tags(tag_id, asset_id);
         CREATE TABLE IF NOT EXISTS companion_skins (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source TEXT NOT NULL,
            visual_preset TEXT NOT NULL,
            texture_path TEXT,
            preview_path TEXT,
            flow_colors TEXT NOT NULL,
            flow_speed REAL NOT NULL,
            flow_intensity REAL NOT NULL,
            created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS companion_settings (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            active_skin_id TEXT NOT NULL REFERENCES companion_skins(id),
            motion_enabled INTEGER NOT NULL,
            visible INTEGER NOT NULL,
            placement_json TEXT
         );
         INSERT INTO app_meta(key, value) VALUES ('schema_version', '4')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
    )?;
    backfill_display_names(&transaction)?;
    seed_builtin_skins(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn backfill_display_names(connection: &Connection) -> Result<(), AssetRepositoryError> {
    let rows = {
        let mut statement =
            connection.prepare("SELECT id, original_path FROM assets WHERE display_name = ''")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (id, original_path) in rows {
        let display_name = Path::new(&original_path)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(&id);
        connection.execute(
            "UPDATE assets SET display_name = ?1 WHERE id = ?2",
            params![display_name, id],
        )?;
    }
    Ok(())
}

fn seed_builtin_skins(connection: &Connection) -> Result<(), AssetRepositoryError> {
    for skin in CompanionSkin::builtins() {
        connection.execute(
            "INSERT INTO companion_skins (
                id, name, source, visual_preset, texture_path, preview_path, flow_colors, flow_speed, flow_intensity, created_at
            ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO NOTHING",
            params![
                skin.id,
                skin.name,
                skin.source.as_str(),
                skin.visual_preset.as_str(),
                serde_json::to_string(&skin.flow_colors).expect("builtin flow colors serialize"),
                skin.flow_speed,
                skin.flow_intensity,
                skin.created_at.to_rfc3339(),
            ],
        )?;
    }
    connection.execute(
        "INSERT INTO companion_settings (
            singleton, active_skin_id, motion_enabled, visible, placement_json
        ) VALUES (1, 'quiet-aurora', 1, 1, NULL)
        ON CONFLICT(singleton) DO NOTHING",
        [],
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
        display_name: row.get(6)?,
        album_id: row.get(7)?,
        favorite: row.get(8)?,
        deleted_at: row.get(9)?,
        capture_mode: row.get(10)?,
        annotation_data: row.get(11)?,
        sync_version: row.get(12)?,
        cloud_id: row.get(13)?,
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
            display_name: stored.display_name,
            album_id: stored.album_id,
            tags: Vec::new(),
            favorite: stored.favorite != 0,
            deleted_at: stored
                .deleted_at
                .map(|value| parse_timestamp("deleted_at", value))
                .transpose()?,
            capture_mode: stored
                .capture_mode
                .as_deref()
                .map(crate::domain::asset::CaptureMode::parse)
                .flatten(),
            annotation_data: stored.annotation_data,
            sync_version: stored.sync_version,
            cloud_id: stored.cloud_id,
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
