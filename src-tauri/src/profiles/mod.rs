//! Profiles: named, persistent crosshair configurations. Exactly one profile
//! is active at a time; the overlay always displays the active profile.

pub mod transfer;

use serde::{Deserialize, Serialize};

use crate::crosshair::CrosshairSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub spec: CrosshairSpec,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Data accepted from the UI to create or edit a profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// When absent on creation, the active configuration is copied.
    #[serde(default)]
    pub spec: Option<CrosshairSpec>,
}
