//! Crosshair library: built-in and user presets, favourites, recent history.

use serde::{Deserialize, Serialize};

use crate::crosshair::presets::PresetCategory;
use crate::crosshair::CrosshairSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub category: PresetCategory,
    pub spec: CrosshairSpec,
    pub favorite: bool,
    /// Shipped with the app (read-only: editing creates a copy).
    pub builtin: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInput {
    pub name: String,
    pub category: PresetCategory,
    pub spec: CrosshairSpec,
}

/// A crosshair recently applied to the overlay.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentEntry {
    pub id: i64,
    pub name: String,
    /// "preset" or "profile".
    pub source: String,
    pub source_id: Option<String>,
    pub spec: CrosshairSpec,
    pub used_at: i64,
}

/// Number of history entries kept.
pub const RECENTS_KEPT: i64 = 24;
