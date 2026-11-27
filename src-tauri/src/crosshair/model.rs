//! Crosshair data model shared by the overlay, the editor and the storage.
//!
//! All structs use `#[serde(default)]`: a file or database row written by an
//! older version that lacks a field is completed with the default value
//! (forward-compatible deserialization).

use serde::{Deserialize, Serialize};

/// What the overlay displays for a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecKind {
    /// The dedicated "Center Dot" mode.
    Dot,
    /// A crosshair built in the editor.
    #[default]
    Crosshair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DotShape {
    #[default]
    Circle,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CrosshairShape {
    /// Simple cross, arms joined in the middle.
    Cross,
    /// Cross with a central gap.
    #[default]
    CrossGap,
    /// "Dynamic" look (split outer segments) but visually fixed.
    CrossDynamic,
    /// Ring.
    Circle,
    /// Ring with a centre dot.
    CircleDot,
    /// Centre dot only.
    Dot,
    /// Cross with a gap and a centre dot.
    CrossDot,
    /// Free composition of arms, ring and dot.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Outline {
    pub enabled: bool,
    pub color: String,
    pub thickness: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#000000".into(),
            thickness: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Halo {
    pub enabled: bool,
    pub color: String,
    /// Width of the glow beyond the dot edge, in pixels.
    pub size: f32,
    pub opacity: f32,
}

impl Default for Halo {
    fn default() -> Self {
        Self {
            enabled: false,
            color: "#FFFFFF".into(),
            size: 6.0,
            opacity: 0.35,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Shadow {
    pub enabled: bool,
    pub color: String,
    pub opacity: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            enabled: false,
            color: "#000000".into(),
            opacity: 0.6,
            offset_x: 1.0,
            offset_y: 1.0,
            blur: 2.0,
        }
    }
}

/// Center Dot mode configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DotConfig {
    pub size: f32,
    pub color: String,
    pub opacity: f32,
    pub shape: DotShape,
    pub outline: Outline,
    pub halo: Halo,
}

impl Default for DotConfig {
    fn default() -> Self {
        Self {
            size: 4.0,
            color: "#00FF66".into(),
            opacity: 1.0,
            shape: DotShape::Circle,
            outline: Outline::default(),
            halo: Halo::default(),
        }
    }
}

/// Elements toggled individually by the `Custom` shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomElements {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
    pub circle: bool,
    pub dot: bool,
}

impl Default for CustomElements {
    fn default() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
            circle: false,
            dot: false,
        }
    }
}

/// Custom crosshair configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CrosshairConfig {
    pub shape: CrosshairShape,
    /// Length of the left/right arms.
    pub length_h: f32,
    /// Length of the top/bottom arms.
    pub length_v: f32,
    pub thickness: f32,
    /// Distance between the central square and the arms.
    pub gap: f32,
    pub color: String,
    pub opacity: f32,
    /// Rotation in degrees, clockwise.
    pub rotation: f32,
    pub outline: Outline,
    pub shadow: Shadow,
    pub dot_size: f32,
    pub dot_shape: DotShape,
    pub circle_radius: f32,
    pub circle_thickness: f32,
    /// Distance between inner and outer segments (dynamic shape).
    pub split_gap: f32,
    pub custom: CustomElements,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            shape: CrosshairShape::CrossGap,
            length_h: 6.0,
            length_v: 6.0,
            thickness: 2.0,
            gap: 3.0,
            color: "#00FF66".into(),
            opacity: 1.0,
            rotation: 0.0,
            outline: Outline::default(),
            shadow: Shadow::default(),
            dot_size: 2.0,
            dot_shape: DotShape::Square,
            circle_radius: 10.0,
            circle_thickness: 1.5,
            split_gap: 3.0,
            custom: CustomElements::default(),
        }
    }
}

/// Complete description of what the overlay draws.
///
/// A spec keeps both configurations so that switching between the Center Dot
/// mode and the crosshair never loses settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CrosshairSpec {
    pub kind: SpecKind,
    pub dot: DotConfig,
    pub crosshair: CrosshairConfig,
}

impl CrosshairSpec {
    pub fn dot(dot: DotConfig) -> Self {
        Self {
            kind: SpecKind::Dot,
            dot,
            crosshair: CrosshairConfig::default(),
        }
    }

    pub fn crosshair(crosshair: CrosshairConfig) -> Self {
        Self {
            kind: SpecKind::Crosshair,
            dot: DotConfig::default(),
            crosshair,
        }
    }

    /// Returns a copy with every length multiplied by `factor` (DPI scaling).
    /// Whole-pixel values stay whole so the result remains crisp.
    pub fn scaled(&self, factor: f32) -> Self {
        if (factor - 1.0).abs() < f32::EPSILON {
            return self.clone();
        }
        let s = |v: f32| scale_len(v, factor);
        let mut out = self.clone();
        out.dot.size = s(self.dot.size).max(1.0);
        out.dot.outline.thickness = s(self.dot.outline.thickness);
        out.dot.halo.size = s(self.dot.halo.size);
        let c = &mut out.crosshair;
        let src = &self.crosshair;
        c.length_h = s(src.length_h);
        c.length_v = s(src.length_v);
        c.thickness = s(src.thickness).max(0.5);
        c.gap = s(src.gap);
        c.outline.thickness = s(src.outline.thickness);
        c.shadow.offset_x = s(src.shadow.offset_x);
        c.shadow.offset_y = s(src.shadow.offset_y);
        c.shadow.blur = s(src.shadow.blur);
        c.dot_size = s(src.dot_size);
        c.circle_radius = s(src.circle_radius);
        c.circle_thickness = s(src.circle_thickness).max(0.5);
        c.split_gap = s(src.split_gap);
        out
    }
}

fn scale_len(v: f32, factor: f32) -> f32 {
    let scaled = v * factor;
    if (v - v.round()).abs() < 1e-3 {
        scaled.round()
    } else {
        scaled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same fixture is checked by `src/services/spec.test.ts`, so Rust and
    /// TypeScript defaults cannot drift apart.
    #[test]
    fn defaults_match_the_shared_fixture() {
        let fixture: CrosshairSpec =
            serde_json::from_str(include_str!("../../../src/services/default-spec.json")).unwrap();
        assert_eq!(fixture, CrosshairSpec::default());
    }

    #[test]
    fn serialization_uses_frontend_field_names() {
        let v = serde_json::to_value(CrosshairSpec::default()).unwrap();
        assert_eq!(v["kind"], "crosshair");
        assert_eq!(v["crosshair"]["shape"], "cross_gap");
        assert!(v["crosshair"].get("lengthH").is_some());
        assert!(v["crosshair"]["shadow"].get("offsetX").is_some());
        assert_eq!(v["dot"]["shape"], "circle");
    }
}
