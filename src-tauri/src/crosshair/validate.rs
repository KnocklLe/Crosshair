//! Validation of every value that reaches the backend.
//!
//! The limits below are the single source of truth: they are sent to the
//! frontend (`get_bootstrap`) to configure the sliders.

use serde::Serialize;

use super::model::{CrosshairSpec, DotConfig, Halo, Outline, Shadow};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

const fn r(min: f32, max: f32, step: f32) -> Range {
    Range { min, max, step }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub dot_size: Range,
    pub opacity: Range,
    pub outline_thickness: Range,
    pub halo_size: Range,
    pub length: Range,
    pub thickness: Range,
    pub gap: Range,
    pub rotation: Range,
    pub shadow_offset: Range,
    pub shadow_blur: Range,
    pub center_dot_size: Range,
    pub circle_radius: Range,
    pub circle_thickness: Range,
    pub split_gap: Range,
    pub name_max_chars: usize,
    pub description_max_chars: usize,
}

pub const LIMITS: Limits = Limits {
    dot_size: r(1.0, 64.0, 1.0),
    opacity: r(0.0, 1.0, 0.01),
    outline_thickness: r(0.5, 8.0, 0.5),
    halo_size: r(1.0, 48.0, 1.0),
    length: r(0.0, 200.0, 1.0),
    thickness: r(0.5, 24.0, 0.5),
    gap: r(0.0, 120.0, 1.0),
    rotation: r(-180.0, 180.0, 1.0),
    shadow_offset: r(-20.0, 20.0, 1.0),
    shadow_blur: r(0.0, 16.0, 1.0),
    center_dot_size: r(0.0, 32.0, 1.0),
    circle_radius: r(1.0, 150.0, 1.0),
    circle_thickness: r(0.5, 16.0, 0.5),
    split_gap: r(0.0, 40.0, 1.0),
    name_max_chars: 64,
    description_max_chars: 280,
};

/// Collects every problem instead of stopping at the first one.
#[derive(Default)]
struct Report(Vec<String>);

impl Report {
    fn range(&mut self, field: &str, value: f32, range: Range) {
        if !value.is_finite() {
            self.0.push(format!("{field} must be a finite number"));
        } else if value < range.min || value > range.max {
            self.0.push(format!(
                "{field} must be between {} and {} (got {value})",
                range.min, range.max
            ));
        }
    }

    fn color(&mut self, field: &str, value: &str) {
        if !is_hex_color(value) {
            self.0.push(format!("{field} must be a #RRGGBB color (got \"{value}\")"));
        }
    }

    fn outline(&mut self, prefix: &str, o: &Outline) {
        self.color(&format!("{prefix}.color"), &o.color);
        self.range(&format!("{prefix}.thickness"), o.thickness, LIMITS.outline_thickness);
    }

    fn halo(&mut self, prefix: &str, h: &Halo) {
        self.color(&format!("{prefix}.color"), &h.color);
        self.range(&format!("{prefix}.size"), h.size, LIMITS.halo_size);
        self.range(&format!("{prefix}.opacity"), h.opacity, LIMITS.opacity);
    }

    fn shadow(&mut self, prefix: &str, s: &Shadow) {
        self.color(&format!("{prefix}.color"), &s.color);
        self.range(&format!("{prefix}.opacity"), s.opacity, LIMITS.opacity);
        self.range(&format!("{prefix}.offsetX"), s.offset_x, LIMITS.shadow_offset);
        self.range(&format!("{prefix}.offsetY"), s.offset_y, LIMITS.shadow_offset);
        self.range(&format!("{prefix}.blur"), s.blur, LIMITS.shadow_blur);
    }

    fn finish(self) -> AppResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(self.0.join("; ")))
        }
    }
}

pub fn validate_dot(d: &DotConfig) -> AppResult<()> {
    let mut rep = Report::default();
    check_dot(&mut rep, d);
    rep.finish()
}

fn check_dot(rep: &mut Report, d: &DotConfig) {
    rep.range("dot.size", d.size, LIMITS.dot_size);
    rep.color("dot.color", &d.color);
    rep.range("dot.opacity", d.opacity, LIMITS.opacity);
    rep.outline("dot.outline", &d.outline);
    rep.halo("dot.halo", &d.halo);
}

/// Validates a complete spec (both configurations are always checked).
pub fn validate_spec(spec: &CrosshairSpec) -> AppResult<()> {
    let mut rep = Report::default();
    check_dot(&mut rep, &spec.dot);

    let c = &spec.crosshair;
    rep.range("crosshair.lengthH", c.length_h, LIMITS.length);
    rep.range("crosshair.lengthV", c.length_v, LIMITS.length);
    rep.range("crosshair.thickness", c.thickness, LIMITS.thickness);
    rep.range("crosshair.gap", c.gap, LIMITS.gap);
    rep.color("crosshair.color", &c.color);
    rep.range("crosshair.opacity", c.opacity, LIMITS.opacity);
    rep.range("crosshair.rotation", c.rotation, LIMITS.rotation);
    rep.outline("crosshair.outline", &c.outline);
    rep.shadow("crosshair.shadow", &c.shadow);
    rep.range("crosshair.dotSize", c.dot_size, LIMITS.center_dot_size);
    rep.range("crosshair.circleRadius", c.circle_radius, LIMITS.circle_radius);
    rep.range("crosshair.circleThickness", c.circle_thickness, LIMITS.circle_thickness);
    rep.range("crosshair.splitGap", c.split_gap, LIMITS.split_gap);
    rep.finish()
}

pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses `#RRGGBB`. Returns `None` for anything else.
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    if !is_hex_color(s) {
        return None;
    }
    let v = u32::from_str_radix(&s[1..], 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Trims and checks a user-provided name.
pub fn clean_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("The name cannot be empty"));
    }
    if trimmed.chars().count() > LIMITS.name_max_chars {
        return Err(AppError::validation(format!(
            "The name cannot exceed {} characters",
            LIMITS.name_max_chars
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(AppError::validation("The name contains invalid characters"));
    }
    Ok(trimmed.to_string())
}

pub fn clean_description(desc: &str) -> AppResult<String> {
    let trimmed = desc.trim();
    if trimmed.chars().count() > LIMITS.description_max_chars {
        return Err(AppError::validation(format!(
            "The description cannot exceed {} characters",
            LIMITS.description_max_chars
        )));
    }
    if trimmed.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::validation("The description contains invalid characters"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::model::*;

    #[test]
    fn default_spec_is_valid() {
        assert!(validate_spec(&CrosshairSpec::default()).is_ok());
    }

    #[test]
    fn hex_colors() {
        assert!(is_hex_color("#00ff66"));
        assert!(is_hex_color("#ABCDEF"));
        assert!(!is_hex_color("00FF66"));
        assert!(!is_hex_color("#00FF6"));
        assert!(!is_hex_color("#GG0000"));
        assert!(!is_hex_color("#00FF66AA"));
        assert_eq!(parse_hex("#8B5CF6"), Some((0x8B, 0x5C, 0xF6)));
        assert_eq!(parse_hex("red"), None);
    }

    #[test]
    fn center_dot_size_out_of_range_is_rejected() {
        let mut spec = CrosshairSpec::default();
        spec.dot.size = 0.0;
        let err = validate_spec(&spec).unwrap_err().to_string();
        assert!(err.contains("dot.size"), "{err}");
        spec.dot.size = 65.0;
        assert!(validate_spec(&spec).is_err());
        spec.dot.size = 64.0;
        assert!(validate_spec(&spec).is_ok());
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let mut spec = CrosshairSpec::default();
        spec.crosshair.thickness = f32::NAN;
        assert!(validate_spec(&spec).is_err());
        spec.crosshair.thickness = f32::INFINITY;
        assert!(validate_spec(&spec).is_err());
    }

    #[test]
    fn all_errors_are_reported_together() {
        let mut spec = CrosshairSpec::default();
        spec.crosshair.color = "blue".into();
        spec.crosshair.gap = -5.0;
        spec.dot.opacity = 2.0;
        let err = validate_spec(&spec).unwrap_err().to_string();
        assert!(err.contains("crosshair.color"));
        assert!(err.contains("crosshair.gap"));
        assert!(err.contains("dot.opacity"));
    }

    #[test]
    fn names_are_trimmed_and_checked() {
        assert_eq!(clean_name("  Pro  ").unwrap(), "Pro");
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(65)).is_err());
        assert!(clean_name("bad\u{0007}name").is_err());
        assert!(clean_description(&"y".repeat(281)).is_err());
        assert_eq!(clean_description(" ok ").unwrap(), "ok");
    }

    #[test]
    fn center_dot_presets_are_valid() {
        for size in [2.0, 4.0, 6.0, 9.0] {
            let dot = DotConfig { size, ..DotConfig::default() };
            assert!(validate_dot(&dot).is_ok());
        }
    }
}
