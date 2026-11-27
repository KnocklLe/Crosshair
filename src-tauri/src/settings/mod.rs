//! User settings, stored as one versioned JSON document.

pub mod autostart;

use serde::{Deserialize, Serialize};

use crate::crosshair::validate::{clean_name, LIMITS};
use crate::error::{AppError, AppResult};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Auto,
    En,
    Fr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityMode {
    /// The overlay is shown whenever it is switched on.
    #[default]
    Always,
    /// Shown only while a fullscreen or borderless window is in the
    /// foreground on the overlay's monitor.
    FullscreenOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlaySettings {
    /// Switch the overlay on when the app starts.
    pub auto_activate: bool,
    /// Monitor device name (`\\.\DISPLAY1`); `None` = primary monitor.
    pub monitor: Option<String>,
    pub visibility: VisibilityMode,
    /// Periodically re-assert the topmost z-order (some borderless games
    /// steal it).
    pub keep_on_top: bool,
    pub topmost_interval_ms: u32,
    /// Multiply crosshair dimensions by the monitor scale factor.
    pub dpi_scaling: bool,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            auto_activate: true,
            monitor: None,
            visibility: VisibilityMode::Always,
            keep_on_top: true,
            topmost_interval_ms: 2000,
            dpi_scaling: false,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PerformanceSettings {
    /// Disables interface animations and lowers the topmost watchdog rate.
    pub lite_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotSizePreset {
    pub id: String,
    pub name: String,
    pub size: f32,
}

pub fn default_dot_presets() -> Vec<DotSizePreset> {
    [("micro", "Micro", 2.0), ("small", "Small", 4.0), ("medium", "Medium", 6.0), ("large", "Large", 9.0)]
        .into_iter()
        .map(|(id, name, size)| DotSizePreset { id: id.into(), name: name.into(), size })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub language: Language,
    pub theme: Theme,
    pub launch_at_startup: bool,
    /// When launched by Windows at startup, stay in the tray.
    pub start_minimized: bool,
    /// The close button hides the window to the notification area.
    pub close_to_tray: bool,
    pub overlay: OverlaySettings,
    pub performance: PerformanceSettings,
    pub dot_presets: Vec<DotSizePreset>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            language: Language::Auto,
            theme: Theme::Dark,
            launch_at_startup: false,
            start_minimized: true,
            close_to_tray: true,
            overlay: OverlaySettings::default(),
            performance: PerformanceSettings::default(),
            dot_presets: default_dot_presets(),
        }
    }
}

impl Settings {
    /// Loads a stored document, migrating older versions. Unknown or broken
    /// data falls back to defaults rather than preventing the app to start.
    pub fn from_stored(json: Option<&str>) -> Settings {
        let Some(json) = json else { return Settings::default() };
        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => Self::migrate(value).and_then(|s| s.validated()).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    fn migrate(value: serde_json::Value) -> AppResult<Settings> {
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        // Future versions: transform `value` step by step here.
        if version > SETTINGS_VERSION {
            return Err(AppError::validation("settings from a newer version"));
        }
        let mut s: Settings = serde_json::from_value(value)?;
        s.version = SETTINGS_VERSION;
        Ok(s)
    }

    /// Validates and normalizes; returns the cleaned settings.
    pub fn validated(mut self) -> AppResult<Settings> {
        let o = &mut self.overlay;
        if !(250..=60_000).contains(&o.topmost_interval_ms) {
            return Err(AppError::validation("overlay.topmostIntervalMs must be between 250 and 60000"));
        }
        if o.offset_x.abs() > 4000 || o.offset_y.abs() > 4000 {
            return Err(AppError::validation("overlay offsets must be between -4000 and 4000"));
        }
        if let Some(m) = &o.monitor {
            if m.is_empty() || m.len() > 64 || m.chars().any(|c| c.is_control()) {
                return Err(AppError::validation("overlay.monitor is invalid"));
            }
        }
        if self.dot_presets.is_empty() || self.dot_presets.len() > 8 {
            return Err(AppError::validation("dot size presets: between 1 and 8 entries"));
        }
        for p in &mut self.dot_presets {
            p.name = clean_name(&p.name)?;
            if p.id.is_empty() || p.id.len() > 32 {
                return Err(AppError::validation("dot size preset id is invalid"));
            }
            let r = LIMITS.dot_size;
            if !p.size.is_finite() || p.size < r.min || p.size > r.max {
                return Err(AppError::validation(format!(
                    "dot size preset \"{}\" must be between {} and {} px",
                    p.name, r.min, r.max
                )));
            }
        }
        self.version = SETTINGS_VERSION;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_dark() {
        let s = Settings::default().validated().unwrap();
        assert_eq!(s.theme, Theme::Dark);
        let sizes: Vec<f32> = s.dot_presets.iter().map(|p| p.size).collect();
        assert_eq!(sizes, [2.0, 4.0, 6.0, 9.0]);
    }

    #[test]
    fn stored_round_trip_and_fallbacks() {
        let mut s = Settings::default();
        s.overlay.offset_x = 3;
        s.language = Language::Fr;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(Settings::from_stored(Some(&json)), s);
        assert_eq!(Settings::from_stored(None), Settings::default());
        assert_eq!(Settings::from_stored(Some("garbage")), Settings::default());
        // Partial document: missing fields take defaults.
        let partial = Settings::from_stored(Some(r#"{"version":1,"theme":"light"}"#));
        assert_eq!(partial.theme, Theme::Light);
        assert!(partial.overlay.keep_on_top);
    }

    #[test]
    fn invalid_values_are_rejected() {
        let mut s = Settings::default();
        s.overlay.topmost_interval_ms = 10;
        assert!(s.validated().is_err());

        let mut s = Settings::default();
        s.dot_presets[0].size = 0.0;
        assert!(s.validated().is_err());

        let mut s = Settings::default();
        s.dot_presets.clear();
        assert!(s.validated().is_err());
    }

    #[test]
    fn dot_presets_are_editable() {
        let mut s = Settings::default();
        s.dot_presets[3].size = 12.0;
        s.dot_presets[3].name = "  XL ".into();
        let s = s.validated().unwrap();
        assert_eq!(s.dot_presets[3].size, 12.0);
        assert_eq!(s.dot_presets[3].name, "XL");
    }
}
