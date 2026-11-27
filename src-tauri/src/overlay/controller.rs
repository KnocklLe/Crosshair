//! Platform-independent overlay logic: options, status and the visibility
//! rules. Kept separate from Win32 so it can be unit-tested.

use serde::Serialize;

use crate::settings::{OverlaySettings, VisibilityMode};

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayOptions {
    pub monitor: Option<String>,
    pub visibility: VisibilityMode,
    pub keep_on_top: bool,
    pub topmost_interval_ms: u32,
    pub dpi_scaling: bool,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl OverlayOptions {
    pub fn from_settings(s: &OverlaySettings, lite_mode: bool) -> Self {
        Self {
            monitor: s.monitor.clone(),
            visibility: s.visibility,
            keep_on_top: s.keep_on_top,
            // Lite mode: re-assert the z-order less often.
            topmost_interval_ms: if lite_mode { s.topmost_interval_ms.max(5000) } else { s.topmost_interval_ms },
            dpi_scaling: s.dpi_scaling,
            offset_x: s.offset_x,
            offset_y: s.offset_y,
        }
    }
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self::from_settings(&OverlaySettings::default(), false)
    }
}

/// State of the overlay as confirmed by the overlay thread (never assumed).
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    /// The overlay thread and its window exist.
    pub running: bool,
    /// `IsWindowVisible` on the overlay window.
    pub visible: bool,
    /// The user wants the overlay on.
    pub requested: bool,
    /// Hidden by the "fullscreen only" rule.
    pub auto_hidden: bool,
    pub monitor_id: Option<String>,
    pub monitor_name: Option<String>,
    /// The selected monitor is not connected; the primary one is used.
    pub monitor_fallback: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f32,
    pub render_count: u64,
    pub last_render_micros: u64,
    /// A program in exclusive fullscreen is in the foreground: the overlay
    /// cannot be displayed above it.
    pub exclusive_fullscreen: bool,
    pub last_error: Option<String>,
}

/// Should the window be shown?
pub fn should_show(requested: bool, mode: VisibilityMode, foreground_is_fullscreen: bool) -> bool {
    requested
        && match mode {
            VisibilityMode::Always => true,
            VisibilityMode::FullscreenOnly => foreground_is_fullscreen,
        }
}

/// Does the window rectangle cover the whole monitor (borderless/fullscreen)?
pub fn covers_monitor(win: (i32, i32, i32, i32), mon: (i32, i32, i32, i32)) -> bool {
    let (wl, wt, wr, wb) = win;
    let (ml, mt, mr, mb) = mon;
    wl <= ml && wt <= mt && wr >= mr && wb >= mb
}

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    StatusChanged(OverlayStatus),
    DisplaysChanged,
    Warning { code: &'static str, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_rules() {
        use VisibilityMode::*;
        assert!(should_show(true, Always, false));
        assert!(!should_show(false, Always, true));
        assert!(should_show(true, FullscreenOnly, true));
        assert!(!should_show(true, FullscreenOnly, false));
        assert!(!should_show(false, FullscreenOnly, true));
    }

    #[test]
    fn fullscreen_detection() {
        let mon = (0, 0, 1920, 1080);
        assert!(covers_monitor((0, 0, 1920, 1080), mon));
        assert!(covers_monitor((-8, -8, 1928, 1088), mon)); // maximized-like overshoot
        assert!(!covers_monitor((0, 0, 1920, 1040), mon)); // taskbar visible
        assert!(!covers_monitor((100, 100, 800, 600), mon));
        let second = (1920, 0, 4480, 1440);
        assert!(covers_monitor((1920, 0, 4480, 1440), second));
        assert!(!covers_monitor((0, 0, 1920, 1080), second));
    }

    #[test]
    fn lite_mode_slows_down_the_watchdog() {
        let s = OverlaySettings::default();
        assert_eq!(OverlayOptions::from_settings(&s, false).topmost_interval_ms, 2000);
        assert_eq!(OverlayOptions::from_settings(&s, true).topmost_interval_ms, 5000);
    }
}
