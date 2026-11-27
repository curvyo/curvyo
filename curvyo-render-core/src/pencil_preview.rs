//! The draw list of a Pencil stroke in progress (`specs/0158-pen-display` slice 1 criterion 10,
//! ADR 0095 decision 7, design-system row "Live preview outline").
//!
//! With widths, the hollow outline of the raw samples: each side offset by half the sample's
//! width along the local normal, joined by round ends, drawn as the 1.5 px `--preview-new` line
//! over its white casing; without widths the centre line in the same look. Linear in the sample
//! count, no boolean union.

use curvyo_document_core::{AnchorSnapshot, NewAnchor, Point, Vec2, ViewTransform};

use crate::glyphs::DrawList;
use crate::shape_preview::UNUSED_ANCHOR_ID;
use crate::stroke;
use crate::theme;

/// Line pieces of each round end (a half circle).
const CAP_STEPS: u32 = 8;
/// Steps shorter than this (mm) give no direction of their own.
const COINCIDENT_MM: f64 = 1e-9;

/// The preview of a stroke through `points` (document space); `widths` (mm, one per point) draw
/// the hollow outline, `None` the centre line.
#[must_use]
pub fn build_pencil_preview(
    points: &[Point],
    widths: Option<&[f64]>,
    view: ViewTransform,
) -> DrawList {
    if points.is_empty() {
        return DrawList::default();
    }
    let (outline, closed) = match widths {
        Some(widths) if widths.len() == points.len() => (hollow_outline(points, widths), true),
        _ => (points.to_vec(), false),
    };
    let anchors: Vec<AnchorSnapshot> = outline
        .into_iter()
        .map(|p| NewAnchor::corner(UNUSED_ANCHOR_ID, p))
        .collect();
    let width_mm = theme::LIVE_PREVIEW_STROKE_PX / view.scale();
    let tolerance_mm = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let mut list = stroke::path_stroke(
        &anchors,
        closed,
        width_mm * theme::CASING_WIDTH_FACTOR,
        theme::SELECTION_CASING,
        tolerance_mm,
    );
    list.extend(stroke::path_stroke(
        &anchors,
        closed,
        width_mm,
        theme::PREVIEW_NEW,
        tolerance_mm,
    ));
    list
}

/// The unit normal (left of travel) at each point, from its neighbours; a point with none of its
/// own takes the last one found (x for a stroke that never moved).
fn normals(points: &[Point]) -> Vec<Vec2> {
    let n = points.len();
    let mut out = Vec::with_capacity(n);
    let mut last = Vec2::new(0.0, 1.0);
    for i in 0..n {
        let (a, b) = (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = dx.hypot(dy);
        if length > COINCIDENT_MM {
            last = Vec2::new(-dy / length, dx / length);
        }
        out.push(last);
    }
    // The first points before any movement take the first direction found.
    if let Some(first) = out.iter().position(|v| *v != Vec2::new(0.0, 1.0)) {
        let direction = out[first];
        for v in &mut out[..first] {
            *v = direction;
        }
    }
    out
}

/// The closed outline: the left side forward, the round end, the right side back, the round start.
fn hollow_outline(points: &[Point], widths: &[f64]) -> Vec<Point> {
    let normals = normals(points);
    let at = |i: usize, side: f64| {
        let r = widths[i].max(0.0) / 2.0 * side;
        Point::new(
            points[i].x + normals[i].x * r,
            points[i].y + normals[i].y * r,
        )
    };
    let mut out = Vec::with_capacity(2 * points.len() + 2 * CAP_STEPS as usize);
    out.extend((0..points.len()).map(|i| at(i, 1.0)));
    let last = points.len() - 1;
    cap(
        &mut out,
        points[last],
        normals[last],
        widths[last] / 2.0,
        1.0,
    );
    out.extend((0..points.len()).rev().map(|i| at(i, -1.0)));
    cap(&mut out, points[0], normals[0], widths[0] / 2.0, -1.0);
    out
}

/// The inner points of a half circle of `radius` around `centre`, from the left side (`normal`)
/// to the right one, bulging forward (`ahead` 1, the end) or back (−1, the start, drawn from the
/// right side to the left).
fn cap(out: &mut Vec<Point>, centre: Point, normal: Vec2, radius: f64, ahead: f64) {
    let forward = Vec2::new(normal.y, -normal.x);
    for k in 1..CAP_STEPS {
        let angle = std::f64::consts::PI * f64::from(k) / f64::from(CAP_STEPS);
        let (side, along) = (angle.cos() * ahead, angle.sin() * ahead);
        let r = radius.max(0.0);
        out.push(Point::new(
            centre.x + (normal.x * side + forward.x * along) * r,
            centre.y + (normal.y * side + forward.y * along) * r,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ViewTransform {
        ViewTransform::new(4.0, Point::new(0.0, 0.0))
    }

    fn line(n: usize) -> Vec<Point> {
        #[allow(clippy::cast_precision_loss)]
        (0..n).map(|i| Point::new(i as f64, 10.0)).collect()
    }

    #[test]
    fn the_outline_lies_half_a_width_beside_the_samples_and_closes_round() {
        let points = line(5);
        let widths = [1.0, 2.0, 2.0, 2.0, 4.0];
        let outline = hollow_outline(&points, &widths);
        assert_eq!(outline.len(), 10 + 2 * (CAP_STEPS as usize - 1));
        // Left of travel along +x in y-down document space is +y for this normal convention.
        assert!((outline[0].y - 10.5).abs() < 1e-9);
        assert!((outline[4].y - 12.0).abs() < 1e-9);
        // The end cap bulges past the last sample by its radius.
        let farthest = outline.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        assert!((farthest - (4.0 + 2.0)).abs() < 0.05, "{farthest}");
        let nearest = outline.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        assert!((nearest - (0.0 - 0.5)).abs() < 0.05, "{nearest}");
    }

    #[test]
    fn both_looks_draw_the_preview_colour_over_the_casing() {
        for widths in [Some(vec![1.0; 30]), None] {
            let list = build_pencil_preview(&line(30), widths.as_deref(), view());
            let colours: Vec<_> = list.triangles.iter().map(|v| v.color).collect();
            assert!(colours.contains(&theme::PREVIEW_NEW));
            assert!(colours.contains(&theme::SELECTION_CASING));
            let first_line = colours
                .iter()
                .position(|c| *c == theme::PREVIEW_NEW)
                .unwrap();
            assert!(
                colours[..first_line]
                    .iter()
                    .all(|c| *c == theme::SELECTION_CASING)
            );
        }
        assert_eq!(build_pencil_preview(&[], None, view()).triangle_count(), 0);
    }

    #[test]
    fn a_stroke_that_never_moved_still_draws() {
        let points = vec![Point::new(1.0, 1.0); 4];
        let list = build_pencil_preview(&points, Some(&[1.0; 4]), view());
        assert!(list.triangle_count() > 0);
    }
}
