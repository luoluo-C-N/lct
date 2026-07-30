use std::{path::Path, sync::Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use thiserror::Error;

use crate::{
    domain::companion::{CompanionSettings, CompanionSkin, SkinSource, VisualPreset},
    repository::assets::{migrate_schema, AssetRepositoryError},
};

pub struct CompanionRepository {
    connection: Mutex<Connection>,
}

const BUILTIN_SKIN_IDS: [&str; 3] = ["quiet-aurora", "porcelain-pearl", "deep-ink"];

#[derive(Debug, Error)]
pub enum CompanionRepositoryError {
    #[error(transparent)]
    AssetMigration(#[from] AssetRepositoryError),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("skin not found: {0}")]
    SkinNotFound(String),
    #[error("built-in skins are immutable")]
    BuiltinSkin,
    #[error("the active skin cannot be deleted")]
    ActiveSkin,
    #[error("skin has an unsupported source: {0}")]
    InvalidSource(String),
    #[error("skin has an unsupported visual preset: {0}")]
    InvalidVisualPreset(String),
    #[error("skin has an invalid created_at timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("companion settings are missing")]
    MissingSettings,
}

#[derive(Debug)]
struct StoredSkin {
    id: String,
    name: String,
    source: String,
    visual_preset: String,
    texture_path: Option<String>,
    preview_path: Option<String>,
    flow_colors: String,
    flow_speed: f32,
    flow_intensity: f32,
    created_at: String,
}

impl CompanionRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CompanionRepositoryError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub(crate) fn from_connection(
        connection: Connection,
    ) -> Result<Self, CompanionRepositoryError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn list_skins(&self) -> Result<Vec<CompanionSkin>, CompanionRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, name, source, visual_preset, texture_path, preview_path, flow_colors, flow_speed, flow_intensity, created_at
             FROM companion_skins ORDER BY id",
        )?;
        let skins = statement
            .query_map([], stored_skin_from_row)?
            .map(|skin| skin.map_err(CompanionRepositoryError::from)?.try_into())
            .collect();
        skins
    }

    pub fn get_settings(&self) -> Result<CompanionSettings, CompanionRepositoryError> {
        let connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let stored = connection
            .query_row(
                "SELECT active_skin_id, motion_enabled, visible, placement_json
                 FROM companion_settings WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(CompanionRepositoryError::MissingSettings)?;

        Ok(CompanionSettings {
            active_skin_id: stored.0,
            motion_enabled: stored.1 != 0,
            visible: stored.2 != 0,
            placement: stored
                .3
                .map(|json| serde_json::from_str(&json))
                .transpose()?,
        })
    }

    pub fn create_skin(&self, skin: &CompanionSkin) -> Result<(), CompanionRepositoryError> {
        if skin.source == SkinSource::Builtin || is_builtin_skin_id(&skin.id) {
            return Err(CompanionRepositoryError::BuiltinSkin);
        }
        let connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        insert_skin(&connection, skin)
    }

    pub fn update_skin(&self, skin: &CompanionSkin) -> Result<(), CompanionRepositoryError> {
        if skin.source == SkinSource::Builtin || is_builtin_skin_id(&skin.id) {
            return Err(CompanionRepositoryError::BuiltinSkin);
        }
        let connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let values = SkinDatabaseValues::from_skin(skin)?;
        let updated = connection.execute(
            "UPDATE companion_skins SET
                name = ?2, source = ?3, visual_preset = ?4, texture_path = ?5, preview_path = ?6,
                flow_colors = ?7, flow_speed = ?8, flow_intensity = ?9, created_at = ?10
             WHERE id = ?1",
            params![
                skin.id,
                skin.name,
                values.source,
                values.visual_preset,
                values.texture_path,
                values.preview_path,
                values.flow_colors,
                skin.flow_speed,
                skin.flow_intensity,
                values.created_at,
            ],
        )?;
        if updated == 0 {
            return Err(CompanionRepositoryError::SkinNotFound(skin.id.clone()));
        }
        Ok(())
    }

    pub fn delete_skin(&self, skin_id: &str) -> Result<(), CompanionRepositoryError> {
        let mut connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let transaction = connection.transaction()?;
        let source = transaction
            .query_row(
                "SELECT source FROM companion_skins WHERE id = ?1",
                params![skin_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| CompanionRepositoryError::SkinNotFound(skin_id.to_owned()))?;
        if is_builtin_skin_id(skin_id) || SkinSource::parse(&source) == Some(SkinSource::Builtin) {
            return Err(CompanionRepositoryError::BuiltinSkin);
        }
        let active_skin_id: String = transaction.query_row(
            "SELECT active_skin_id FROM companion_settings WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        if active_skin_id == skin_id {
            return Err(CompanionRepositoryError::ActiveSkin);
        }
        transaction.execute(
            "DELETE FROM companion_skins WHERE id = ?1",
            params![skin_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_active_skin(&self, skin_id: &str) -> Result<(), CompanionRepositoryError> {
        let mut connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE companion_settings SET active_skin_id = ?1 WHERE singleton = 1",
            params![skin_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_settings(
        &self,
        settings: &CompanionSettings,
    ) -> Result<(), CompanionRepositoryError> {
        let mut connection = self
            .connection
            .lock()
            .expect("companion repository lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO companion_settings (
                singleton, active_skin_id, motion_enabled, visible, placement_json
             ) VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(singleton) DO UPDATE SET
                active_skin_id = excluded.active_skin_id,
                motion_enabled = excluded.motion_enabled,
                visible = excluded.visible,
                placement_json = excluded.placement_json",
            params![
                settings.active_skin_id,
                i64::from(settings.motion_enabled),
                i64::from(settings.visible),
                settings
                    .placement
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn is_builtin_skin_id(skin_id: &str) -> bool {
    BUILTIN_SKIN_IDS.contains(&skin_id)
}

fn insert_skin(
    connection: &Connection,
    skin: &CompanionSkin,
) -> Result<(), CompanionRepositoryError> {
    let values = SkinDatabaseValues::from_skin(skin)?;
    connection.execute(
        "INSERT INTO companion_skins (
            id, name, source, visual_preset, texture_path, preview_path, flow_colors, flow_speed, flow_intensity, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            skin.id,
            skin.name,
            values.source,
            values.visual_preset,
            values.texture_path,
            values.preview_path,
            values.flow_colors,
            skin.flow_speed,
            skin.flow_intensity,
            values.created_at,
        ],
    )?;
    Ok(())
}

struct SkinDatabaseValues {
    source: String,
    visual_preset: String,
    texture_path: Option<String>,
    preview_path: Option<String>,
    flow_colors: String,
    created_at: String,
}

impl SkinDatabaseValues {
    fn from_skin(skin: &CompanionSkin) -> Result<Self, CompanionRepositoryError> {
        Ok(Self {
            source: skin.source.as_str().to_owned(),
            visual_preset: skin.visual_preset.as_str().to_owned(),
            texture_path: skin
                .texture_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            preview_path: skin
                .preview_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            flow_colors: serde_json::to_string(&skin.flow_colors)?,
            created_at: skin.created_at.to_rfc3339(),
        })
    }
}

fn stored_skin_from_row(row: &Row<'_>) -> rusqlite::Result<StoredSkin> {
    Ok(StoredSkin {
        id: row.get(0)?,
        name: row.get(1)?,
        source: row.get(2)?,
        visual_preset: row.get(3)?,
        texture_path: row.get(4)?,
        preview_path: row.get(5)?,
        flow_colors: row.get(6)?,
        flow_speed: row.get(7)?,
        flow_intensity: row.get(8)?,
        created_at: row.get(9)?,
    })
}

impl TryFrom<StoredSkin> for CompanionSkin {
    type Error = CompanionRepositoryError;

    fn try_from(stored: StoredSkin) -> Result<Self, Self::Error> {
        Ok(Self {
            id: stored.id,
            name: stored.name,
            source: SkinSource::parse(&stored.source)
                .ok_or(CompanionRepositoryError::InvalidSource(stored.source))?,
            visual_preset: VisualPreset::parse(&stored.visual_preset).ok_or(
                CompanionRepositoryError::InvalidVisualPreset(stored.visual_preset),
            )?,
            texture_path: stored.texture_path.map(Into::into),
            preview_path: stored.preview_path.map(Into::into),
            flow_colors: serde_json::from_str(&stored.flow_colors)?,
            flow_speed: stored.flow_speed,
            flow_intensity: stored.flow_intensity,
            created_at: DateTime::parse_from_rfc3339(&stored.created_at)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .map_err(|_| CompanionRepositoryError::InvalidTimestamp(stored.created_at))?,
        })
    }
}
