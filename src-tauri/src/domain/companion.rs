use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSkin {
    pub id: String,
    pub name: String,
    pub source: SkinSource,
    pub visual_preset: VisualPreset,
    pub texture_path: Option<PathBuf>,
    pub preview_path: Option<PathBuf>,
    pub flow_colors: Vec<String>,
    pub flow_speed: f32,
    pub flow_intensity: f32,
    pub created_at: DateTime<Utc>,
}

impl CompanionSkin {
    pub fn builtin(visual_preset: VisualPreset) -> Self {
        let (id, name, flow_colors, flow_speed, flow_intensity) = match visual_preset {
            VisualPreset::QuietAurora => (
                "quiet-aurora",
                "Quiet Aurora",
                vec!["#BD9FFF", "#FFF4DC"],
                1.0,
                0.7,
            ),
            VisualPreset::PorcelainPearl => (
                "porcelain-pearl",
                "Porcelain Pearl",
                vec!["#E3BD7E", "#FFF8EA"],
                1.0,
                0.7,
            ),
            VisualPreset::DeepInk => ("deep-ink", "Deep Ink", vec!["#49D9CF", "#D4FFF8"], 1.0, 0.7),
            VisualPreset::Custom => panic!("custom skins cannot be constructed as built-ins"),
        };

        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            source: SkinSource::Builtin,
            visual_preset,
            texture_path: None,
            preview_path: None,
            flow_colors: flow_colors.into_iter().map(str::to_owned).collect(),
            flow_speed,
            flow_intensity,
            created_at: Utc::now(),
        }
    }

    pub fn builtins() -> [Self; 3] {
        [
            Self::builtin(VisualPreset::QuietAurora),
            Self::builtin(VisualPreset::PorcelainPearl),
            Self::builtin(VisualPreset::DeepInk),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinSource {
    Builtin,
    Image,
    Package,
}

impl SkinSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Image => "image",
            Self::Package => "package",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "builtin" => Some(Self::Builtin),
            "image" | "imported" => Some(Self::Image),
            "package" => Some(Self::Package),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualPreset {
    QuietAurora,
    PorcelainPearl,
    DeepInk,
    Custom,
}

impl VisualPreset {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::QuietAurora => "quiet_aurora",
            Self::PorcelainPearl => "porcelain_pearl",
            Self::DeepInk => "deep_ink",
            Self::Custom => "custom",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "quiet_aurora" => Some(Self::QuietAurora),
            "porcelain_pearl" => Some(Self::PorcelainPearl),
            "deep_ink" => Some(Self::DeepInk),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSettings {
    pub active_skin_id: String,
    pub motion_enabled: bool,
    pub visible: bool,
    pub placement: Option<WindowPlacement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPlacement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSettings {
    pub flow_speed: f32,
    pub flow_intensity: f32,
}

impl MotionSettings {
    pub fn new(flow_speed: f32, flow_intensity: f32) -> Self {
        Self {
            flow_speed: flow_speed.clamp(0.0, 2.0),
            flow_intensity: flow_intensity.clamp(0.0, 1.0),
        }
    }
}
