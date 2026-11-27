//! Monitor detection and overlay placement.
//!
//! Coordinates are physical pixels (the calling thread is switched to
//! per-monitor DPI awareness v2 while enumerating), so a 4K monitor at 150 %
//! reports 3840×2160 and a scale of 1.5.

#[cfg(windows)]
mod win32;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    /// GDI device name, e.g. `\\.\DISPLAY1` (stable while the setup is unchanged).
    pub id: String,
    /// Friendly monitor name ("DELL U2720Q"), or a generic fallback.
    pub name: String,
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub dpi: u32,
    pub scale: f32,
    pub refresh_rate: u32,
    pub primary: bool,
}

/// Lists the connected monitors, primary first.
pub fn list() -> Vec<DisplayInfo> {
    #[cfg(windows)]
    {
        let mut v = win32::enumerate();
        sort_displays(&mut v);
        v
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

pub fn sort_displays(v: &mut [DisplayInfo]) {
    v.sort_by(|a, b| b.primary.cmp(&a.primary).then(a.x.cmp(&b.x)).then(a.y.cmp(&b.y)));
    for (i, d) in v.iter_mut().enumerate() {
        d.index = i as u32 + 1;
    }
}

/// Picks the wanted monitor. Falls back to the primary monitor (then the first
/// one) when it is not connected; the boolean is `true` on fallback.
pub fn resolve<'a>(displays: &'a [DisplayInfo], wanted: Option<&str>) -> Option<(&'a DisplayInfo, bool)> {
    if let Some(id) = wanted {
        if let Some(d) = displays.iter().find(|d| d.id.eq_ignore_ascii_case(id)) {
            return Some((d, false));
        }
    }
    let fallback = displays.iter().find(|d| d.primary).or_else(|| displays.first())?;
    Some((fallback, wanted.is_some()))
}

/// Top-left position of a `size × size` window whose centre is the centre of
/// the monitor (plus the user offset). The monitor centre of an even
/// resolution lies on a pixel boundary, like the canvas centre.
pub fn centered_origin(d: &DisplayInfo, size: i32, offset_x: i32, offset_y: i32) -> (i32, i32) {
    (
        d.x + d.width / 2 - size / 2 + offset_x,
        d.y + d.height / 2 - size / 2 + offset_y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disp(id: &str, x: i32, w: i32, h: i32, primary: bool) -> DisplayInfo {
        DisplayInfo {
            id: id.into(),
            name: id.into(),
            index: 0,
            x,
            y: 0,
            width: w,
            height: h,
            dpi: 96,
            scale: 1.0,
            refresh_rate: 60,
            primary,
        }
    }

    #[test]
    fn resolve_prefers_requested_then_primary() {
        let ds = vec![disp(r"\\.\DISPLAY2", 1920, 2560, 1440, false), disp(r"\\.\DISPLAY1", 0, 1920, 1080, true)];
        let (d, fb) = resolve(&ds, Some(r"\\.\DISPLAY2")).unwrap();
        assert_eq!(d.id, r"\\.\DISPLAY2");
        assert!(!fb);
        let (d, fb) = resolve(&ds, Some(r"\\.\DISPLAY9")).unwrap();
        assert!(d.primary);
        assert!(fb, "missing monitor must be reported as fallback");
        let (d, fb) = resolve(&ds, None).unwrap();
        assert!(d.primary && !fb);
        assert!(resolve(&[], None).is_none());
    }

    #[test]
    fn centre_is_exact() {
        let d = disp("a", 0, 1920, 1080, true);
        assert_eq!(centered_origin(&d, 32, 0, 0), (944, 524)); // 960-16, 540-16
        let second = disp("b", 1920, 2560, 1440, false);
        assert_eq!(centered_origin(&second, 40, 0, 0), (1920 + 1280 - 20, 700));
        assert_eq!(centered_origin(&d, 32, 5, -3), (949, 521));
        // Monitor left of the primary (negative coordinates).
        let left = disp("c", -1280, 1280, 1024, false);
        assert_eq!(centered_origin(&left, 20, 0, 0), (-650, 502));
    }

    #[test]
    fn sort_puts_primary_first_and_numbers() {
        let mut ds = vec![disp("b", 1920, 1, 1, false), disp("a", 0, 1, 1, true), disp("c", -1920, 1, 1, false)];
        sort_displays(&mut ds);
        assert_eq!(ds.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), ["a", "c", "b"]);
        assert_eq!(ds.iter().map(|d| d.index).collect::<Vec<_>>(), [1, 2, 3]);
    }

    #[cfg(windows)]
    #[test]
    fn real_monitors_are_detected() {
        let ds = list();
        assert!(!ds.is_empty(), "at least one monitor expected");
        assert_eq!(ds.iter().filter(|d| d.primary).count(), 1, "exactly one primary monitor");
        for d in &ds {
            assert!(d.width > 0 && d.height > 0);
            assert!(d.dpi >= 72 && d.scale >= 0.75);
            assert!(d.id.starts_with(r"\\.\"), "{}", d.id);
        }
    }
}
