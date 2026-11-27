//! Software renderer (tiny-skia). The single source of pixels for both the
//! overlay window and the previews shown in the interface.
//!
//! Layers, bottom to top: halo → shadow → body (outlines then fills). The body
//! is drawn opaque in its own layer and composited with the global opacity, so
//! overlapping elements never look darker where they cross.

use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, GradientStop, Paint, PathBuilder, Pixmap,
    PixmapPaint, Point, RadialGradient, Rect, SpreadMode, Stroke, Transform,
};

use super::geometry::{self, Geometry, Prim, Rgb};
use super::model::CrosshairSpec;

/// Renders a spec. `scale` multiplies every dimension (DPI scaling).
pub fn render(spec: &CrosshairSpec, scale: f32) -> Pixmap {
    let spec = spec.scaled(scale.clamp(0.5, 4.0));
    render_geometry(&geometry::build(&spec))
}

pub fn render_geometry(g: &Geometry) -> Pixmap {
    let mut out = Pixmap::new(g.size, g.size).expect("canvas size is never zero");
    let transform = if g.rotation.abs() > f32::EPSILON {
        Transform::from_rotate_at(g.rotation, g.pivot.0, g.pivot.1)
    } else {
        Transform::identity()
    };

    if let Some(h) = g.halo {
        draw_halo(&mut out, h, g.opacity);
    }

    if let Some(s) = g.shadow {
        let mut sil = Pixmap::new(g.size, g.size).expect("non-zero");
        let expand = g.outline.map_or(0.0, |o| o.thickness);
        for e in &g.elements {
            fill_prim(&mut sil, &e.prim.expanded(expand), &solid(s.color, e.alpha), transform);
        }
        box_blur(&mut sil, s.blur);
        out.draw_pixmap(
            s.offset_x,
            s.offset_y,
            sil.as_ref(),
            &pixmap_paint(s.opacity * g.opacity),
            Transform::identity(),
            None,
        );
    }

    let mut body = Pixmap::new(g.size, g.size).expect("non-zero");
    if let Some(o) = g.outline {
        for e in &g.elements {
            fill_prim(&mut body, &e.prim.expanded(o.thickness), &solid(o.color, e.alpha), transform);
        }
    }
    for e in &g.elements {
        fill_prim(&mut body, &e.prim, &solid(g.color, e.alpha), transform);
    }
    out.draw_pixmap(0, 0, body.as_ref(), &pixmap_paint(g.opacity), Transform::identity(), None);
    out
}

fn solid(c: Rgb, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c.0, c.1, c.2, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8));
    p.anti_alias = true;
    p
}

fn pixmap_paint(opacity: f32) -> PixmapPaint {
    PixmapPaint {
        opacity: opacity.clamp(0.0, 1.0),
        blend_mode: BlendMode::SourceOver,
        quality: FilterQuality::Nearest,
    }
}

fn fill_prim(pm: &mut Pixmap, prim: &Prim, paint: &Paint, transform: Transform) {
    match *prim {
        Prim::Rect { x, y, w, h } => {
            if let Some(rect) = Rect::from_xywh(x, y, w, h) {
                pm.fill_rect(rect, paint, transform, None);
            }
        }
        Prim::Disc { cx, cy, r } => {
            if let Some(path) = PathBuilder::from_circle(cx, cy, r.max(0.25)) {
                pm.fill_path(&path, paint, FillRule::Winding, transform, None);
            }
        }
        Prim::Ring { cx, cy, r, width } => {
            if let Some(path) = PathBuilder::from_circle(cx, cy, r.max(0.25)) {
                let stroke = Stroke { width: width.max(0.25), ..Stroke::default() };
                pm.stroke_path(&path, paint, &stroke, transform, None);
            }
        }
    }
}

fn draw_halo(pm: &mut Pixmap, h: geometry::HaloParams, opacity: f32) {
    let a = (h.opacity * opacity).clamp(0.0, 1.0);
    let (r, g, b) = h.color;
    let start = Color::from_rgba8(r, g, b, (a * 255.0).round() as u8);
    let end = Color::from_rgba8(r, g, b, 0);
    let center = Point::from_xy(h.cx, h.cy);
    let Some(shader) = RadialGradient::new(
        center,
        h.inner.max(0.0),
        center,
        h.outer.max(h.inner + 0.5),
        vec![GradientStop::new(0.0, start), GradientStop::new(1.0, end)],
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = true;
    if let Some(path) = PathBuilder::from_circle(h.cx, h.cy, h.outer) {
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// Three-pass box blur on premultiplied RGBA (approximates a gaussian of
/// standard deviation ≈ `radius`). Pixels outside the canvas count as
/// transparent.
pub fn box_blur(pm: &mut Pixmap, radius: u32) {
    if radius == 0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let r = radius as usize;
    let mut tmp = vec![0u8; w * h * 4];
    for _ in 0..3 {
        blur_line(pm.data(), &mut tmp, w, h, r, true);
        blur_line(&tmp, pm.data_mut(), w, h, r, false);
    }
}

fn blur_line(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len) = if horizontal { (h, w) } else { (w, h) };
    let div = (2 * r + 1) as u32;
    let index = |line: usize, i: usize| -> usize {
        if horizontal {
            (line * w + i) * 4
        } else {
            (i * w + line) * 4
        }
    };
    for line in 0..lines {
        for ch in 0..4 {
            let mut sum: u32 = 0;
            for i in 0..=r.min(len - 1) {
                sum += src[index(line, i) + ch] as u32;
            }
            for i in 0..len {
                dst[index(line, i) + ch] = (sum / div) as u8;
                let add = i + r + 1;
                if add < len {
                    sum += src[index(line, add) + ch] as u32;
                }
                if i >= r {
                    sum -= src[index(line, i - r) + ch] as u32;
                }
            }
        }
    }
}

/// Premultiplied BGRA, the layout expected by `UpdateLayeredWindow`.
pub fn to_bgra_premultiplied(pm: &Pixmap) -> Vec<u8> {
    let mut v = pm.data().to_vec();
    for px in v.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    v
}

/// Straight (non-premultiplied) RGBA, the layout expected by `ImageData`.
pub fn to_rgba_straight(pm: Pixmap) -> Vec<u8> {
    pm.take_demultiplied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::model::*;

    fn alpha_at(pm: &Pixmap, x: u32, y: u32) -> u8 {
        pm.pixel(x, y).unwrap().alpha()
    }

    fn no_outline(shape: CrosshairShape) -> CrosshairSpec {
        CrosshairSpec::crosshair(CrosshairConfig {
            shape,
            outline: Outline { enabled: false, ..Outline::default() },
            ..CrosshairConfig::default()
        })
    }

    #[test]
    fn gap_cross_pixels_are_crisp() {
        let mut spec = no_outline(CrosshairShape::CrossGap);
        spec.crosshair.thickness = 2.0;
        spec.crosshair.gap = 3.0;
        spec.crosshair.length_h = 6.0;
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        // Inside the right arm: fully opaque.
        assert_eq!(alpha_at(&pm, mid + 5, mid), 255);
        assert_eq!(alpha_at(&pm, mid + 5, mid - 1), 255);
        // Just outside the arm thickness: fully transparent (no blur).
        assert_eq!(alpha_at(&pm, mid + 5, mid + 1), 0);
        // In the gap: transparent.
        assert_eq!(alpha_at(&pm, mid + 1, mid), 0);
        assert_eq!(alpha_at(&pm, mid, mid), 0);
    }

    #[test]
    fn color_is_respected() {
        let mut spec = no_outline(CrosshairShape::Cross);
        spec.crosshair.color = "#FF0000".into();
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        let p = pm.pixel(mid + 2, mid).unwrap();
        assert_eq!((p.red(), p.green(), p.blue(), p.alpha()), (255, 0, 0, 255));
    }

    #[test]
    fn opacity_is_applied_once_even_where_elements_overlap() {
        let mut spec = no_outline(CrosshairShape::Cross);
        spec.crosshair.opacity = 0.5;
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        // Centre (both bars overlap) and arm have the same alpha.
        let centre = alpha_at(&pm, mid, mid);
        let arm = alpha_at(&pm, mid + 3, mid);
        assert_eq!(centre, arm);
        assert!((120..=135).contains(&arm), "alpha {arm}");
    }

    #[test]
    fn outline_surrounds_the_body() {
        let mut spec = CrosshairSpec::crosshair(CrosshairConfig {
            shape: CrosshairShape::CrossGap,
            color: "#FFFFFF".into(),
            ..CrosshairConfig::default()
        });
        spec.crosshair.outline = Outline { enabled: true, color: "#000000".into(), thickness: 1.0 };
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        let edge = pm.pixel(mid + 6, mid + 1).unwrap(); // just below a 2 px arm
        assert_eq!(edge.alpha(), 255);
        assert_eq!(edge.red(), 0);
    }

    #[test]
    fn center_dot_renders_in_the_middle() {
        let spec = CrosshairSpec::dot(DotConfig {
            size: 4.0,
            shape: DotShape::Square,
            outline: Outline { enabled: false, ..Outline::default() },
            ..DotConfig::default()
        });
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        assert_eq!(alpha_at(&pm, mid, mid), 255);
        assert_eq!(alpha_at(&pm, mid - 2, mid - 2), 255);
        assert_eq!(alpha_at(&pm, mid + 2, mid), 0);
    }

    #[test]
    fn halo_and_shadow_produce_soft_pixels() {
        let mut dot = DotConfig::default();
        dot.halo.enabled = true;
        dot.halo.size = 10.0;
        let pm = render(&CrosshairSpec::dot(dot), 1.0);
        let mid = pm.width() / 2;
        let a = alpha_at(&pm, mid + 6, mid);
        assert!(a > 0 && a < 255, "halo alpha {a}");

        let mut spec = no_outline(CrosshairShape::Cross);
        spec.crosshair.shadow = Shadow { enabled: true, blur: 2.0, ..Shadow::default() };
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        assert!(alpha_at(&pm, mid + 3, mid + 3) > 0);
    }

    #[test]
    fn bgra_conversion_swaps_channels() {
        let mut spec = no_outline(CrosshairShape::Dot);
        spec.crosshair.dot_size = 4.0;
        spec.crosshair.color = "#FF0000".into();
        let pm = render(&spec, 1.0);
        let mid = pm.width() / 2;
        let i = ((mid * pm.width() + mid) * 4) as usize;
        let bgra = to_bgra_premultiplied(&pm);
        assert_eq!(&bgra[i..i + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn every_shape_renders_visible_pixels() {
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
            let pm = render(&no_outline(shape), 1.0);
            assert!(pm.pixels().iter().any(|p| p.alpha() > 0), "{shape:?} is empty");
        }
    }
}
