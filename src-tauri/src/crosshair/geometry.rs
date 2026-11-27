//! Converts a [`CrosshairSpec`] into simple primitives (rectangles, discs,
//! rings) and computes the canvas size.
//!
//! Pixel alignment: the canvas side is always even, so its centre `C` lies on
//! a pixel boundary — exactly like the centre of a 1920×1080 screen. An
//! element whose size is odd is centred on the pixel at `C + 0.5`; an element
//! whose size is even is centred on `C`. With whole-pixel parameters every
//! rectangle edge therefore falls on a pixel boundary: lines are perfectly
//! sharp and symmetric.

use super::model::{CrosshairConfig, CrosshairShape, CrosshairSpec, DotConfig, DotShape, SpecKind};
use super::validate::parse_hex;

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Prim {
    /// Axis-aligned rectangle (before rotation), canvas coordinates.
    Rect { x: f32, y: f32, w: f32, h: f32 },
    /// Filled disc.
    Disc { cx: f32, cy: f32, r: f32 },
    /// Stroked circle: `r` is the radius of the stroke centre line.
    Ring { cx: f32, cy: f32, r: f32, width: f32 },
}

impl Prim {
    /// Grows the primitive by `e` pixels on every side (used for outlines).
    pub fn expanded(&self, e: f32) -> Prim {
        match *self {
            Prim::Rect { x, y, w, h } => Prim::Rect {
                x: x - e,
                y: y - e,
                w: w + 2.0 * e,
                h: h + 2.0 * e,
            },
            Prim::Disc { cx, cy, r } => Prim::Disc { cx, cy, r: r + e },
            Prim::Ring { cx, cy, r, width } => Prim::Ring {
                cx,
                cy,
                r,
                width: width + 2.0 * e,
            },
        }
    }

    fn translated(&self, d: f32) -> Prim {
        match *self {
            Prim::Rect { x, y, w, h } => Prim::Rect { x: x + d, y: y + d, w, h },
            Prim::Disc { cx, cy, r } => Prim::Disc { cx: cx + d, cy: cy + d, r },
            Prim::Ring { cx, cy, r, width } => Prim::Ring {
                cx: cx + d,
                cy: cy + d,
                r,
                width,
            },
        }
    }

    /// Farthest distance from `(px, py)` covered by the primitive.
    fn reach(&self, px: f32, py: f32) -> f32 {
        match *self {
            Prim::Rect { x, y, w, h } => {
                let dx = (x - px).abs().max((x + w - px).abs());
                let dy = (y - py).abs().max((y + h - py).abs());
                (dx * dx + dy * dy).sqrt()
            }
            Prim::Disc { cx, cy, r } => ((cx - px).powi(2) + (cy - py).powi(2)).sqrt() + r,
            Prim::Ring { cx, cy, r, width } => {
                ((cx - px).powi(2) + (cy - py).powi(2)).sqrt() + r + width / 2.0
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    pub prim: Prim,
    /// Relative opacity of this element (1.0 except dynamic outer segments).
    pub alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlineParams {
    pub color: Rgb,
    pub thickness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowParams {
    pub color: Rgb,
    pub opacity: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HaloParams {
    pub color: Rgb,
    pub opacity: f32,
    pub cx: f32,
    pub cy: f32,
    pub inner: f32,
    pub outer: f32,
}

/// Everything the renderer needs, already in canvas coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// Side of the square canvas (always even).
    pub size: u32,
    /// Rotation pivot.
    pub pivot: (f32, f32),
    pub rotation: f32,
    pub elements: Vec<Element>,
    pub color: Rgb,
    pub opacity: f32,
    pub outline: Option<OutlineParams>,
    pub shadow: Option<ShadowParams>,
    pub halo: Option<HaloParams>,
}

/// Maximum canvas side, a hard safety limit.
pub const MAX_CANVAS: u32 = 1024;
const MARGIN: f32 = 2.0;

/// 0.5 when `v` is an odd whole number, 0 otherwise.
pub fn parity_offset(v: f32) -> f32 {
    let r = v.round();
    if (v - r).abs() < 1e-3 && (r as i64).rem_euclid(2) == 1 {
        0.5
    } else {
        0.0
    }
}

fn rgb(hex: &str) -> Rgb {
    parse_hex(hex).unwrap_or((255, 255, 255))
}

pub fn build(spec: &CrosshairSpec) -> Geometry {
    match spec.kind {
        SpecKind::Dot => build_dot(&spec.dot),
        SpecKind::Crosshair => build_crosshair(&spec.crosshair),
    }
}

fn dot_prim(size: f32, shape: DotShape) -> Prim {
    let o = parity_offset(size);
    match shape {
        DotShape::Circle => Prim::Disc { cx: o, cy: o, r: size / 2.0 },
        DotShape::Square => Prim::Rect {
            x: o - size / 2.0,
            y: o - size / 2.0,
            w: size,
            h: size,
        },
    }
}

fn build_dot(d: &DotConfig) -> Geometry {
    let size = d.size.max(1.0);
    let elements = vec![Element { prim: dot_prim(size, d.shape), alpha: 1.0 }];
    let outline = d.outline.enabled.then(|| OutlineParams {
        color: rgb(&d.outline.color),
        thickness: d.outline.thickness,
    });
    let o = parity_offset(size);
    let outline_w = outline.map_or(0.0, |o| o.thickness);
    let halo = d.halo.enabled.then(|| HaloParams {
        color: rgb(&d.halo.color),
        opacity: d.halo.opacity,
        cx: o,
        cy: o,
        inner: size / 2.0 + outline_w,
        outer: size / 2.0 + outline_w + d.halo.size,
    });
    finish(elements, (o, o), 0.0, rgb(&d.color), d.opacity, outline, None, halo)
}

fn build_crosshair(c: &CrosshairConfig) -> Geometry {
    let t = c.thickness.max(0.5);
    let o = parity_offset(t);
    let half = t / 2.0;
    let inner = half + c.gap;
    let mut el: Vec<Element> = Vec::new();
    let mut push = |prim: Prim, alpha: f32| el.push(Element { prim, alpha });

    // Individual arms; each flag enables one arm.
    let arms = |push: &mut dyn FnMut(Prim, f32), top: bool, right: bool, bottom: bool, left: bool, start: f32, lh: f32, lv: f32, alpha: f32| {
        if lh > 0.0 {
            if left {
                push(Prim::Rect { x: o - start - lh, y: o - half, w: lh, h: t }, alpha);
            }
            if right {
                push(Prim::Rect { x: o + start, y: o - half, w: lh, h: t }, alpha);
            }
        }
        if lv > 0.0 {
            if top {
                push(Prim::Rect { x: o - half, y: o - start - lv, w: t, h: lv }, alpha);
            }
            if bottom {
                push(Prim::Rect { x: o - half, y: o + start, w: t, h: lv }, alpha);
            }
        }
    };

    let ring = Prim::Ring {
        cx: o,
        cy: o,
        r: c.circle_radius,
        width: c.circle_thickness,
    };
    let dot_size = c.dot_size;

    match c.shape {
        CrosshairShape::Cross => {
            let (lh, lv) = (c.length_h, c.length_v);
            if lh > 0.0 || lv <= 0.0 {
                push(Prim::Rect { x: o - half - lh, y: o - half, w: 2.0 * lh + t, h: t }, 1.0);
            }
            if lv > 0.0 {
                push(Prim::Rect { x: o - half, y: o - half - lv, w: t, h: 2.0 * lv + t }, 1.0);
            }
        }
        CrosshairShape::CrossGap => {
            arms(&mut push, true, true, true, true, inner, c.length_h, c.length_v, 1.0);
        }
        CrosshairShape::CrossDynamic => {
            arms(&mut push, true, true, true, true, inner, c.length_h, c.length_v, 1.0);
            let outer_h = (c.length_h / 2.0).round().max(1.0);
            let outer_v = (c.length_v / 2.0).round().max(1.0);
            let start_h = inner + c.length_h + c.split_gap;
            let start_v = inner + c.length_v + c.split_gap;
            if c.length_h > 0.0 {
                push(Prim::Rect { x: o - start_h - outer_h, y: o - half, w: outer_h, h: t }, 0.55);
                push(Prim::Rect { x: o + start_h, y: o - half, w: outer_h, h: t }, 0.55);
            }
            if c.length_v > 0.0 {
                push(Prim::Rect { x: o - half, y: o - start_v - outer_v, w: t, h: outer_v }, 0.55);
                push(Prim::Rect { x: o - half, y: o + start_v, w: t, h: outer_v }, 0.55);
            }
        }
        CrosshairShape::Circle => push(ring, 1.0),
        CrosshairShape::CircleDot => {
            push(ring, 1.0);
            if dot_size > 0.0 {
                push(dot_prim(dot_size, c.dot_shape), 1.0);
            }
        }
        CrosshairShape::Dot => push(dot_prim(dot_size.max(1.0), c.dot_shape), 1.0),
        CrosshairShape::CrossDot => {
            arms(&mut push, true, true, true, true, inner, c.length_h, c.length_v, 1.0);
            if dot_size > 0.0 {
                push(dot_prim(dot_size, c.dot_shape), 1.0);
            }
        }
        CrosshairShape::Custom => {
            let k = &c.custom;
            arms(&mut push, k.top, k.right, k.bottom, k.left, inner, c.length_h, c.length_v, 1.0);
            if k.circle {
                push(ring, 1.0);
            }
            if k.dot && dot_size > 0.0 {
                push(dot_prim(dot_size, c.dot_shape), 1.0);
            }
        }
    }

    let outline = c.outline.enabled.then(|| OutlineParams {
        color: rgb(&c.outline.color),
        thickness: c.outline.thickness,
    });
    let shadow = c.shadow.enabled.then(|| ShadowParams {
        color: rgb(&c.shadow.color),
        opacity: c.shadow.opacity,
        offset_x: c.shadow.offset_x.round() as i32,
        offset_y: c.shadow.offset_y.round() as i32,
        blur: c.shadow.blur.round().max(0.0) as u32,
    });
    finish(el, (o, o), c.rotation, rgb(&c.color), c.opacity, outline, shadow, None)
}

#[allow(clippy::too_many_arguments)]
fn finish(
    elements: Vec<Element>,
    pivot: (f32, f32),
    rotation: f32,
    color: Rgb,
    opacity: f32,
    outline: Option<OutlineParams>,
    shadow: Option<ShadowParams>,
    halo: Option<HaloParams>,
) -> Geometry {
    let size = canvas_size(&elements, pivot, outline, shadow, halo);
    let c = (size / 2) as f32;
    let elements = elements
        .into_iter()
        .map(|e| Element { prim: e.prim.translated(c), alpha: e.alpha })
        .collect();
    Geometry {
        size,
        pivot: (pivot.0 + c, pivot.1 + c),
        rotation,
        elements,
        color,
        opacity,
        outline,
        shadow,
        halo: halo.map(|h| HaloParams { cx: h.cx + c, cy: h.cy + c, ..h }),
    }
}

/// Smallest even square that contains every element, its outline, its
/// shadow and its halo whatever the rotation.
pub fn canvas_size(
    elements: &[Element],
    pivot: (f32, f32),
    outline: Option<OutlineParams>,
    shadow: Option<ShadowParams>,
    halo: Option<HaloParams>,
) -> u32 {
    let outline_w = outline.map_or(0.0, |o| o.thickness);
    let mut reach = elements
        .iter()
        .map(|e| e.prim.expanded(outline_w).reach(pivot.0, pivot.1))
        .fold(1.0_f32, f32::max);
    if let Some(s) = shadow {
        let off = (s.offset_x.unsigned_abs().max(s.offset_y.unsigned_abs())) as f32;
        reach += off * std::f32::consts::SQRT_2 + 3.0 * s.blur as f32;
    }
    if let Some(h) = halo {
        reach = reach.max(h.outer);
    }
    let half = (reach + MARGIN).ceil() as u32 + 1;
    (half * 2).clamp(8, MAX_CANVAS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::model::*;

    fn cross(shape: CrosshairShape) -> CrosshairConfig {
        CrosshairConfig {
            shape,
            outline: Outline { enabled: false, ..Outline::default() },
            ..CrosshairConfig::default()
        }
    }

    fn rects(g: &Geometry) -> Vec<(f32, f32, f32, f32)> {
        g.elements
            .iter()
            .filter_map(|e| match e.prim {
                Prim::Rect { x, y, w, h } => Some((x, y, w, h)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn parity() {
        assert_eq!(parity_offset(1.0), 0.5);
        assert_eq!(parity_offset(3.0), 0.5);
        assert_eq!(parity_offset(2.0), 0.0);
        assert_eq!(parity_offset(1.5), 0.0);
    }

    #[test]
    fn canvas_is_even_and_bounded() {
        for shape in [
            CrosshairShape::Cross,
            CrosshairShape::CrossGap,
            CrosshairShape::CrossDynamic,
            CrosshairShape::Circle,
            CrosshairShape::CircleDot,
            CrosshairShape::Dot,
            CrosshairShape::CrossDot,
            CrosshairShape::Custom,
        ] {
            let mut c = cross(shape);
            c.rotation = 45.0;
            let g = build_crosshair(&c);
            assert_eq!(g.size % 2, 0, "{shape:?}");
            assert!(g.size >= 8 && g.size <= MAX_CANVAS);
            let s = g.size as f32;
            for e in &g.elements {
                assert!(e.prim.reach(g.pivot.0, g.pivot.1) <= s / 2.0, "{shape:?} overflows");
            }
        }
    }

    #[test]
    fn extreme_values_fit_in_max_canvas() {
        let mut c = cross(CrosshairShape::CrossDynamic);
        c.length_h = 200.0;
        c.length_v = 200.0;
        c.gap = 120.0;
        c.split_gap = 40.0;
        c.thickness = 24.0;
        c.rotation = 45.0;
        c.outline.enabled = true;
        c.outline.thickness = 8.0;
        c.shadow.enabled = true;
        c.shadow.offset_x = 20.0;
        c.shadow.blur = 16.0;
        let g = build_crosshair(&c);
        assert!(g.size <= MAX_CANVAS);
        let half = (g.size / 2) as f32;
        for e in &g.elements {
            assert!(e.prim.expanded(8.0).reach(g.pivot.0, g.pivot.1) <= half);
        }
    }

    #[test]
    fn gap_cross_arms_are_symmetric_and_pixel_aligned() {
        // thickness 2 (even): centred on the canvas boundary.
        let mut c = cross(CrosshairShape::CrossGap);
        c.thickness = 2.0;
        c.gap = 3.0;
        c.length_h = 6.0;
        c.length_v = 5.0;
        let g = build_crosshair(&c);
        let mid = (g.size / 2) as f32;
        let r = rects(&g);
        assert_eq!(r.len(), 4);
        // left arm: from mid-1-3-6 to mid-1-3
        assert!(r.contains(&(mid - 10.0, mid - 1.0, 6.0, 2.0)));
        // right arm mirrors the left one
        assert!(r.contains(&(mid + 4.0, mid - 1.0, 6.0, 2.0)));
        // vertical arms use lengthV
        assert!(r.contains(&(mid - 1.0, mid - 9.0, 2.0, 5.0)));
        assert!(r.contains(&(mid - 1.0, mid + 4.0, 2.0, 5.0)));
        for (x, y, w, h) in r {
            for v in [x, y, w, h] {
                assert_eq!(v, v.round(), "edge not on a pixel boundary");
            }
        }
    }

    #[test]
    fn odd_thickness_is_centred_on_a_pixel() {
        let mut c = cross(CrosshairShape::Cross);
        c.thickness = 1.0;
        c.length_h = 4.0;
        c.length_v = 4.0;
        let g = build_crosshair(&c);
        let mid = (g.size / 2) as f32;
        assert_eq!(g.pivot, (mid + 0.5, mid + 0.5));
        let r = rects(&g);
        // horizontal bar: 4 + 1 + 4 = 9 px, column `mid` is the centre pixel.
        assert!(r.contains(&(mid - 4.0, mid, 9.0, 1.0)));
        assert!(r.contains(&(mid, mid - 4.0, 1.0, 9.0)));
    }

    #[test]
    fn shapes_produce_expected_elements() {
        let count = |s| build_crosshair(&cross(s)).elements.len();
        assert_eq!(count(CrosshairShape::Cross), 2);
        assert_eq!(count(CrosshairShape::CrossGap), 4);
        assert_eq!(count(CrosshairShape::CrossDynamic), 8);
        assert_eq!(count(CrosshairShape::Circle), 1);
        assert_eq!(count(CrosshairShape::CircleDot), 2);
        assert_eq!(count(CrosshairShape::Dot), 1);
        assert_eq!(count(CrosshairShape::CrossDot), 5);

        let mut custom = cross(CrosshairShape::Custom);
        custom.custom = CustomElements {
            top: false,
            right: true,
            bottom: true,
            left: true,
            circle: true,
            dot: true,
        };
        assert_eq!(build_crosshair(&custom).elements.len(), 5);
    }

    #[test]
    fn dynamic_outer_segments_are_faded() {
        let g = build_crosshair(&cross(CrosshairShape::CrossDynamic));
        assert_eq!(g.elements.iter().filter(|e| e.alpha < 1.0).count(), 4);
    }

    #[test]
    fn center_dot_dimensions() {
        let dot = DotConfig {
            size: 6.0,
            shape: DotShape::Square,
            outline: Outline { enabled: true, thickness: 1.0, ..Outline::default() },
            ..DotConfig::default()
        };
        let g = build_dot(&dot);
        let mid = (g.size / 2) as f32;
        assert_eq!(g.elements[0].prim, Prim::Rect { x: mid - 3.0, y: mid - 3.0, w: 6.0, h: 6.0 });
        assert_eq!(g.outline.unwrap().thickness, 1.0);

        let odd = build_dot(&DotConfig { size: 3.0, ..DotConfig::default() });
        let mid = (odd.size / 2) as f32;
        assert_eq!(odd.elements[0].prim, Prim::Disc { cx: mid + 0.5, cy: mid + 0.5, r: 1.5 });
    }

    #[test]
    fn halo_enlarges_canvas() {
        let plain = build_dot(&DotConfig::default());
        let mut with_halo = DotConfig::default();
        with_halo.halo.enabled = true;
        with_halo.halo.size = 20.0;
        let g = build_dot(&with_halo);
        assert!(g.size > plain.size);
        assert!(g.halo.unwrap().outer <= (g.size / 2) as f32);
    }

    #[test]
    fn scaled_spec_keeps_whole_pixels() {
        let spec = CrosshairSpec::default().scaled(1.5);
        assert_eq!(spec.crosshair.thickness, 3.0);
        assert_eq!(spec.crosshair.gap, 5.0); // 4.5 rounded
        assert_eq!(spec.crosshair.length_h, 9.0);
        assert_eq!(spec.dot.size, 6.0);
    }
}
