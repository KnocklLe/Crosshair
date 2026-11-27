//! Versioned JSON import/export.
//!
//! ```json
//! { "format": "crosshair-studio", "version": 1, "exportedAt": 0,
//!   "appVersion": "1.0.0", "profiles": [], "presets": [] }
//! ```
//!
//! Import never trusts the file: size, format, version, item count and every
//! field are checked. Structural problems reject the whole file; an invalid
//! item is skipped and reported, the other items are imported.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::crosshair::presets::PresetCategory;
use crate::crosshair::validate::{clean_description, clean_name, validate_spec};
use crate::crosshair::CrosshairSpec;
use crate::error::{AppError, AppResult};
use crate::library::Preset;
use crate::settings::Settings;
use crate::shortcuts::ShortcutBinding;

use super::Profile;

pub const EXPORT_FORMAT: &str = "crosshair-studio";
pub const EXPORT_VERSION: u32 = 1;
pub const MAX_IMPORT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMPORT_ITEMS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedProfile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub spec: CrosshairSpec,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedPreset {
    pub name: String,
    pub category: PresetCategory,
    pub spec: CrosshairSpec,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub exported_at: i64,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub profiles: Vec<ExportedProfile>,
    #[serde(default)]
    pub presets: Vec<ExportedPreset>,
    /// Present only in full backups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<ShortcutBinding>>,
}

impl ExportFile {
    pub fn new(now: i64, app_version: &str) -> Self {
        Self {
            format: EXPORT_FORMAT.into(),
            version: EXPORT_VERSION,
            exported_at: now,
            app_version: app_version.into(),
            profiles: vec![],
            presets: vec![],
            settings: None,
            shortcuts: None,
        }
    }

    pub fn with_profiles(mut self, profiles: &[Profile]) -> Self {
        self.profiles = profiles
            .iter()
            .map(|p| ExportedProfile {
                name: p.name.clone(),
                description: p.description.clone(),
                spec: p.spec.clone(),
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect();
        self
    }

    pub fn with_presets(mut self, presets: &[Preset]) -> Self {
        self.presets = presets
            .iter()
            .map(|p| ExportedPreset {
                name: p.name.clone(),
                category: p.category,
                spec: p.spec.clone(),
                favorite: p.favorite,
                created_at: p.created_at,
            })
            .collect();
        self
    }

    pub fn to_json(&self) -> AppResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItem {
    pub kind: String,
    pub name: String,
    pub reason: String,
}

/// Result of a successful parse: only valid, cleaned items.
#[derive(Debug, Clone)]
pub struct ImportBundle {
    pub profiles: Vec<ExportedProfile>,
    pub presets: Vec<ExportedPreset>,
    pub settings: Option<Settings>,
    pub shortcuts: Option<Vec<ShortcutBinding>>,
    pub skipped: Vec<SkippedItem>,
}

/// Brings an older document up to [`EXPORT_VERSION`]. Each future version adds
/// one step here (v1 → v2, v2 → v3…).
fn migrate(doc: Value, version: u32) -> AppResult<Value> {
    match version {
        EXPORT_VERSION => Ok(doc),
        0 => Err(AppError::Import("Invalid file version (0).".into())),
        v => Err(AppError::Import(format!(
            "This file was created by a newer version of Crosshair Studio (format v{v}). Update the application to import it."
        ))),
    }
}

pub fn parse_import(bytes: &[u8]) -> AppResult<ImportBundle> {
    if bytes.is_empty() {
        return Err(AppError::Import("The file is empty.".into()));
    }
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Import(format!(
            "The file is too large ({} KB, maximum {} KB).",
            bytes.len() / 1024,
            MAX_IMPORT_BYTES / 1024
        )));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Import("The file is not valid UTF-8 text.".into()))?;
    let text = text.trim_start_matches('\u{feff}');
    let doc: Value = serde_json::from_str(text)
        .map_err(|e| AppError::Import(format!("The file is not valid JSON: {e}")))?;

    let obj = doc
        .as_object()
        .ok_or_else(|| AppError::Import("The JSON root must be an object.".into()))?;
    if obj.get("format").and_then(Value::as_str) != Some(EXPORT_FORMAT) {
        return Err(AppError::Import("This is not a Crosshair Studio file (missing \"format\").".into()));
    }
    let version = obj
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::Import("Missing or invalid \"version\".".into()))? as u32;

    let doc = migrate(doc, version)?;
    let file: ExportFile = serde_json::from_value(doc)
        .map_err(|e| AppError::Import(format!("Invalid file structure: {e}")))?;

    let total = file.profiles.len() + file.presets.len();
    if total == 0 && file.settings.is_none() {
        return Err(AppError::Import("The file contains no profile or preset.".into()));
    }
    if total > MAX_IMPORT_ITEMS {
        return Err(AppError::Import(format!(
            "Too many items ({total}, maximum {MAX_IMPORT_ITEMS})."
        )));
    }

    let mut skipped = Vec::new();
    let mut profiles = Vec::new();
    for p in file.profiles {
        match clean_profile(&p) {
            Ok(clean) => profiles.push(clean),
            Err(e) => skipped.push(SkippedItem { kind: "profile".into(), name: p.name, reason: e.to_string() }),
        }
    }
    let mut presets = Vec::new();
    for p in file.presets {
        match clean_preset(&p) {
            Ok(clean) => presets.push(clean),
            Err(e) => skipped.push(SkippedItem { kind: "preset".into(), name: p.name, reason: e.to_string() }),
        }
    }
    let settings = match file.settings {
        Some(s) => match s.validated() {
            Ok(s) => Some(s),
            Err(e) => {
                skipped.push(SkippedItem { kind: "settings".into(), name: "settings".into(), reason: e.to_string() });
                None
            }
        },
        None => None,
    };

    Ok(ImportBundle { profiles, presets, settings, shortcuts: file.shortcuts, skipped })
}

fn clean_profile(p: &ExportedProfile) -> AppResult<ExportedProfile> {
    validate_spec(&p.spec)?;
    Ok(ExportedProfile {
        name: clean_name(&p.name)?,
        description: clean_description(&p.description)?,
        ..p.clone()
    })
}

fn clean_preset(p: &ExportedPreset) -> AppResult<ExportedPreset> {
    validate_spec(&p.spec)?;
    Ok(ExportedPreset { name: clean_name(&p.name)?, ..p.clone() })
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub profiles_imported: usize,
    pub presets_imported: usize,
    pub settings_restored: bool,
    pub skipped: Vec<SkippedItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::model::*;

    fn sample_profile(name: &str) -> Profile {
        Profile {
            id: "id".into(),
            name: name.into(),
            description: "desc".into(),
            spec: CrosshairSpec::dot(DotConfig { size: 6.0, ..DotConfig::default() }),
            sort_index: 0,
            created_at: 10,
            updated_at: 20,
        }
    }

    #[test]
    fn export_then_import_round_trip() {
        let json = ExportFile::new(123, "1.0.0")
            .with_profiles(&[sample_profile("A"), sample_profile("B")])
            .to_json()
            .unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["format"], EXPORT_FORMAT);
        assert_eq!(v["version"], EXPORT_VERSION);
        assert_eq!(v["profiles"][0]["spec"]["kind"], "dot");
        assert_eq!(v["profiles"][0]["spec"]["dot"]["size"], 6.0);

        let bundle = parse_import(json.as_bytes()).unwrap();
        assert_eq!(bundle.profiles.len(), 2);
        assert_eq!(bundle.profiles[0].name, "A");
        assert_eq!(bundle.profiles[0].spec.dot.size, 6.0);
        assert!(bundle.skipped.is_empty());
    }

    #[test]
    fn rejects_foreign_or_broken_files() {
        assert!(parse_import(b"").is_err());
        assert!(parse_import(b"not json").is_err());
        assert!(parse_import(b"[1,2]").is_err());
        assert!(parse_import(br#"{"version":1,"profiles":[]}"#).is_err());
        assert!(parse_import(br#"{"format":"other","version":1}"#).is_err());
        assert!(parse_import(br#"{"format":"crosshair-studio"}"#).is_err());
        assert!(parse_import(br#"{"format":"crosshair-studio","version":1,"profiles":[]}"#).is_err());
        assert!(parse_import(br#"{"format":"crosshair-studio","version":1,"profiles":"x"}"#).is_err());
    }

    #[test]
    fn rejects_newer_versions() {
        let err = parse_import(br#"{"format":"crosshair-studio","version":99,"profiles":[]}"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("newer version"), "{err}");
    }

    #[test]
    fn rejects_oversized_files() {
        let big = vec![b' '; MAX_IMPORT_BYTES + 1];
        assert!(parse_import(&big).unwrap_err().to_string().contains("too large"));
    }

    #[test]
    fn invalid_items_are_skipped_and_reported() {
        let json = r##"{
          "format": "crosshair-studio", "version": 1,
          "profiles": [
            { "name": "Good", "spec": { "kind": "crosshair" } },
            { "name": "BadColor", "spec": { "crosshair": { "color": "green" } } },
            { "name": "   ", "spec": {} }
          ],
          "presets": [
            { "name": "Huge", "category": "dot", "spec": { "dot": { "size": 500 } } },
            { "name": "Fine", "category": "dot", "spec": { "kind": "dot" } }
          ]
        }"##;
        let b = parse_import(json.as_bytes()).unwrap();
        assert_eq!(b.profiles.len(), 1);
        assert_eq!(b.presets.len(), 1);
        assert_eq!(b.skipped.len(), 3);
        assert!(b.skipped.iter().any(|s| s.name == "BadColor" && s.reason.contains("color")));
    }

    #[test]
    fn missing_fields_take_defaults() {
        let json = r#"{"format":"crosshair-studio","version":1,
            "profiles":[{"name":"Old","spec":{"kind":"crosshair","crosshair":{"gap":7}}}]}"#;
        let b = parse_import(json.as_bytes()).unwrap();
        let c = &b.profiles[0].spec.crosshair;
        assert_eq!(c.gap, 7.0);
        assert_eq!(c.thickness, CrosshairConfig::default().thickness);
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let json = ExportFile::new(0, "1").with_profiles(&[sample_profile("Bom")]).to_json().unwrap();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(json.as_bytes());
        assert_eq!(parse_import(&bytes).unwrap().profiles[0].name, "Bom");
    }

    #[test]
    fn too_many_items_is_rejected() {
        let profiles: Vec<Profile> = (0..=MAX_IMPORT_ITEMS).map(|i| sample_profile(&format!("p{i}"))).collect();
        let json = ExportFile::new(0, "1").with_profiles(&profiles).to_json().unwrap();
        // Keep the test independent of the size limit.
        if json.len() <= MAX_IMPORT_BYTES {
            assert!(parse_import(json.as_bytes()).unwrap_err().to_string().contains("Too many"));
        }
    }
}
