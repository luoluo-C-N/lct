use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub imported_at: DateTime<Utc>,
    pub source: AssetSource,
    pub original_path: PathBuf,
    pub preview_path: PathBuf,
    pub album_id: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub capture_mode: Option<CaptureMode>,
    pub annotation_data: Option<String>,
    pub sync_version: i64,
    pub cloud_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode { Region, Window, Fullscreen }
impl CaptureMode { pub(crate) fn as_str(self) -> &'static str { match self { Self::Region => "region", Self::Window => "window", Self::Fullscreen => "fullscreen" } } pub(crate) fn parse(value: &str) -> Option<Self> { match value { "region" => Some(Self::Region), "window" => Some(Self::Window), "fullscreen" => Some(Self::Fullscreen), _ => None } } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    Import,
    Capture,
}

impl AssetSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Capture => "capture",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "import" => Some(Self::Import),
            "capture" => Some(Self::Capture),
            _ => None,
        }
    }
}
