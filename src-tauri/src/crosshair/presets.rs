//! Built-in library presets (seeded on first launch, restorable from Settings).

use serde::{Deserialize, Serialize};

use super::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PresetCategory {
    Minimal,
    Competitive,
    Classic,
    Dot,
    Circle,
    Custom,
}

impl PresetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PresetCategory::Minimal => "minimal",
            PresetCategory::Competitive => "competitive",
            PresetCategory::Classic => "classic",
            PresetCategory::Dot => "dot",
            PresetCategory::Circle => "circle",
            PresetCategory::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "minimal" => PresetCategory::Minimal,
            "competitive" => PresetCategory::Competitive,
            "classic" => PresetCategory::Classic,
            "dot" => PresetCategory::Dot,
            "circle" => PresetCategory::Circle,
            "custom" => PresetCategory::Custom,
            _ => return None,
        })
    }
}

pub struct BuiltinPreset {
    /// Stable identifier, also used as the database id.
    pub id: &'static str,
    pub name: &'static str,
    pub category: PresetCategory,
    pub spec: CrosshairSpec,
}

fn outline(color: &str, thickness: f32) -> Outline {
    Outline { enabled: true, color: color.into(), thickness }
}

fn no_outline() -> Outline {
    Outline { enabled: false, ..Outline::default() }
}

#[allow(clippy::too_many_arguments)]
fn cross(shape: CrosshairShape, color: &str, len: f32, thickness: f32, gap: f32, o: Outline) -> CrosshairConfig {
    CrosshairConfig {
        shape,
        length_h: len,
        length_v: len,
        thickness,
        gap,
        color: color.into(),
        outline: o,
        ..CrosshairConfig::default()
    }
}

fn dot(size: f32, color: &str, shape: DotShape, o: Outline) -> DotConfig {
    DotConfig { size, color: color.into(), shape, outline: o, ..DotConfig::default() }
}

fn ch(c: CrosshairConfig) -> CrosshairSpec {
    CrosshairSpec::crosshair(c)
}

fn dt(d: DotConfig) -> CrosshairSpec {
    CrosshairSpec::dot(d)
}

pub fn builtin_presets() -> Vec<BuiltinPreset> {
    use CrosshairShape as S;
    use PresetCategory as C;
    let p = |id, name, category, spec| BuiltinPreset { id, name, category, spec };

    vec![
        // ── Minimal ─────────────────────────────────────────────
        p("builtin-pinpoint", "Pinpoint", C::Minimal, dt(dot(2.0, "#FFFFFF", DotShape::Square, outline("#000000", 1.0)))),
        p("builtin-hairline", "Hairline", C::Minimal, ch(cross(S::CrossGap, "#FFFFFF", 5.0, 1.0, 2.0, outline("#000000", 1.0)))),
        p("builtin-whisper", "Whisper", C::Minimal, ch(CrosshairConfig {
            opacity: 0.75,
            ..cross(S::CrossGap, "#E5E7EB", 3.0, 1.0, 2.0, no_outline())
        })),
        p("builtin-tiny-plus", "Tiny Plus", C::Minimal, ch(cross(S::Cross, "#00FF66", 3.0, 1.0, 0.0, outline("#000000", 1.0)))),

        // ── Competitive ─────────────────────────────────────────
        p("builtin-tactical-green", "Tactical Green", C::Competitive, ch(cross(S::CrossGap, "#00FF66", 5.0, 2.0, 3.0, outline("#000000", 1.0)))),
        p("builtin-cyan-precision", "Cyan Precision", C::Competitive, ch(cross(S::CrossGap, "#00FFFF", 4.0, 1.0, 2.0, outline("#000000", 1.0)))),
        p("builtin-yellow-pro", "Yellow Pro", C::Competitive, ch(cross(S::CrossDot, "#FFFF00", 4.0, 2.0, 2.0, outline("#000000", 1.0)))),
        p("builtin-magenta-edge", "Magenta Edge", C::Competitive, ch(cross(S::CrossGap, "#FF00FF", 6.0, 2.0, 4.0, outline("#000000", 1.0)))),
        p("builtin-violet-strike", "Violet Strike", C::Competitive, ch(CrosshairConfig {
            dot_size: 2.0,
            ..cross(S::CrossDot, "#A78BFA", 5.0, 2.0, 3.0, outline("#000000", 1.0))
        })),

        // ── Classic ─────────────────────────────────────────────
        p("builtin-classic-cross", "Classic Cross", C::Classic, ch(cross(S::Cross, "#00FF00", 8.0, 2.0, 0.0, outline("#000000", 1.0)))),
        p("builtin-dynamic-split", "Dynamic Split", C::Classic, ch(CrosshairConfig {
            split_gap: 3.0,
            ..cross(S::CrossDynamic, "#00FF00", 6.0, 2.0, 4.0, outline("#000000", 1.0))
        })),
        p("builtin-retro-red", "Retro Red", C::Classic, ch(cross(S::CrossGap, "#FF3B30", 10.0, 3.0, 5.0, outline("#000000", 1.0)))),
        p("builtin-military", "Military", C::Classic, ch(CrosshairConfig {
            dot_size: 3.0,
            ..cross(S::CrossDot, "#F5F5F7", 12.0, 1.0, 6.0, no_outline())
        })),
        p("builtin-long-range", "Long Range", C::Classic, ch(CrosshairConfig {
            length_h: 24.0,
            length_v: 24.0,
            ..cross(S::CrossGap, "#00FF66", 24.0, 1.0, 8.0, outline("#000000", 1.0))
        })),

        // ── Dot ─────────────────────────────────────────────────
        p("builtin-dot-micro", "Micro Dot", C::Dot, dt(dot(2.0, "#00FF66", DotShape::Circle, outline("#000000", 1.0)))),
        p("builtin-dot-small", "Small Dot", C::Dot, dt(dot(4.0, "#00FF66", DotShape::Circle, outline("#000000", 1.0)))),
        p("builtin-dot-medium", "Medium Dot", C::Dot, dt(dot(6.0, "#FF3B30", DotShape::Circle, outline("#000000", 1.0)))),
        p("builtin-dot-large", "Large Dot", C::Dot, dt(dot(9.0, "#FFFF00", DotShape::Circle, outline("#000000", 1.5)))),
        p("builtin-dot-halo", "Halo Dot", C::Dot, dt(DotConfig {
            halo: Halo { enabled: true, color: "#8B5CF6".into(), size: 8.0, opacity: 0.5 },
            ..dot(4.0, "#FFFFFF", DotShape::Circle, outline("#000000", 1.0))
        })),
        p("builtin-dot-square", "Square Dot", C::Dot, dt(dot(4.0, "#00FFFF", DotShape::Square, outline("#000000", 1.0)))),

        // ── Circle ──────────────────────────────────────────────
        p("builtin-ring", "Ring", C::Circle, ch(CrosshairConfig {
            circle_radius: 8.0,
            circle_thickness: 1.5,
            ..cross(S::Circle, "#00FF66", 0.0, 1.0, 0.0, outline("#000000", 1.0))
        })),
        p("builtin-ring-dot", "Ring + Dot", C::Circle, ch(CrosshairConfig {
            circle_radius: 10.0,
            circle_thickness: 1.5,
            dot_size: 2.0,
            dot_shape: DotShape::Circle,
            ..cross(S::CircleDot, "#FFFFFF", 0.0, 1.0, 0.0, outline("#000000", 1.0))
        })),
        p("builtin-wide-ring", "Wide Ring", C::Circle, ch(CrosshairConfig {
            circle_radius: 24.0,
            circle_thickness: 2.0,
            dot_size: 3.0,
            dot_shape: DotShape::Circle,
            ..cross(S::CircleDot, "#FF00FF", 0.0, 1.0, 0.0, outline("#000000", 1.0))
        })),
        p("builtin-shotgun", "Shotgun Spread", C::Circle, ch(CrosshairConfig {
            circle_radius: 40.0,
            circle_thickness: 1.0,
            dot_size: 2.0,
            dot_shape: DotShape::Circle,
            opacity: 0.85,
            ..cross(S::CircleDot, "#F5F5F7", 0.0, 1.0, 0.0, outline("#000000", 1.0))
        })),

        // ── Custom ──────────────────────────────────────────────
        p("builtin-t-shape", "T-Shape", C::Custom, ch(CrosshairConfig {
            custom: CustomElements { top: false, right: true, bottom: true, left: true, circle: false, dot: true },
            dot_size: 2.0,
            ..cross(S::Custom, "#00FF66", 6.0, 2.0, 3.0, outline("#000000", 1.0))
        })),
        p("builtin-x-cross", "X Cross", C::Custom, ch(CrosshairConfig {
            rotation: 45.0,
            ..cross(S::CrossGap, "#FFFFFF", 6.0, 2.0, 3.0, outline("#000000", 1.0))
        })),
        p("builtin-scope", "Scope", C::Custom, ch(CrosshairConfig {
            custom: CustomElements { top: true, right: true, bottom: true, left: true, circle: true, dot: true },
            circle_radius: 18.0,
            circle_thickness: 1.0,
            dot_size: 2.0,
            dot_shape: DotShape::Circle,
            ..cross(S::Custom, "#FF3B30", 10.0, 1.0, 6.0, outline("#000000", 1.0))
        })),
        p("builtin-shadow-cross", "Soft Shadow", C::Custom, ch(CrosshairConfig {
            shadow: Shadow { enabled: true, color: "#000000".into(), opacity: 0.8, offset_x: 1.0, offset_y: 1.0, blur: 2.0 },
            ..cross(S::CrossGap, "#FFFFFF", 6.0, 2.0, 3.0, no_outline())
        })),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::{render::render, validate::validate_spec};
    use std::collections::HashSet;

    #[test]
    fn at_least_twenty_valid_presets() {
        let presets = builtin_presets();
        assert!(presets.len() >= 20);
        let mut ids = HashSet::new();
        for p in &presets {
            assert!(ids.insert(p.id), "duplicate id {}", p.id);
            validate_spec(&p.spec).unwrap_or_else(|e| panic!("{}: {e}", p.name));
            let pm = render(&p.spec, 1.0);
            assert!(pm.pixels().iter().any(|px| px.alpha() > 0), "{} renders nothing", p.name);
        }
    }

    #[test]
    fn every_category_is_represented() {
        let cats: HashSet<_> = builtin_presets().iter().map(|p| p.category).collect();
        assert_eq!(cats.len(), 6);
    }

    #[test]
    fn category_round_trip() {
        for c in [
            PresetCategory::Minimal,
            PresetCategory::Competitive,
            PresetCategory::Classic,
            PresetCategory::Dot,
            PresetCategory::Circle,
            PresetCategory::Custom,
        ] {
            assert_eq!(PresetCategory::parse(c.as_str()), Some(c));
        }
        assert_eq!(PresetCategory::parse("rgb"), None);
    }
}
