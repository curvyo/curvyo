//! The hatch of Engrave and Engrave + Outline (`specs/0101-laser-operations` ux-notes section 13.2):
//! the closed areas of a leaf are hatched at 45 degrees, lines 1.5 px wide every 6 px in screen
//! space, in `--engrave-hatch`. Holes are hatched as well as the even-odd rule says (a hole inside
//! an outline is not hatched). The hatch lines sit on one grid in document space, so they do not
//! crawl while the view pans.

use curvyo_document_core::{AnchorSnapshot, ObjectSnapshot, Point, ViewTransform};

use crate::glyphs::{DrawList, thick_line};
use crate::theme::OP_ENGRAVE_HATCH;

/// The distance between two hatch lines, screen pixels.
pub const HATCH_PERIOD_PX: f64 = 6.0;
/// The width of a hatch line, screen pixels.
pub const HATCH_WIDTH_PX: f64 = 1.5;
/// The most hatch lines of one object in a frame: a larger area is hatched in part (criterion 29;
/// the first number to measure, ux-notes section 13.4).
pub const HATCH_LINES_PER_OBJECT: usize = 4000;

/// Samples of one cubic segment.
const CUBIC_STEPS: usize = 8;

/// The polyline of an outline through `anchors`: straight where a segment has no handles, sampled
/// where it has. A closed outline returns to its first point implicitly (no repeated point).
#[must_use]
pub fn flatten(anchors: &[AnchorSnapshot], closed: bool) -> Vec<Point> {
    let mut out = Vec::with_capacity(anchors.len() * CUBIC_STEPS);
    let count = anchors.len();
    if count == 0 {
        return out;
    }
    out.push(anchors[0].point);
    let segments = if closed { count } else { count - 1 };
    for i in 0..segments {
        let from = &anchors[i];
        let to = &anchors[(i + 1) % count];
        if from.handle_out == curvyo_document_core::Vec2::ZERO
            && to.handle_in == curvyo_document_core::Vec2::ZERO
        {
            out.push(to.point);
            continue;
        }
        let c1 = from.point.translated(from.handle_out);
        let c2 = to.point.translated(to.handle_in);
        for step in 1..=CUBIC_STEPS {
            // The step count is a small constant: the cast loses nothing.
            #[allow(clippy::cast_precision_loss)]
            let t = step as f64 / CUBIC_STEPS as f64;
            out.push(cubic_at(from.point, c1, c2, to.point, t));
        }
    }
    // A closed outline's last segment ends on its first point: that point is not repeated.
    if closed && out.len() > 1 && out.last() == out.first() {
        out.pop();
    }
    out
}

fn cubic_at(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(
        w0 * p0.x + w1 * p1.x + w2 * p2.x + w3 * p3.x,
        w0 * p0.y + w1 * p1.y + w2 * p2.y + w3 * p3.y,
    )
}

/// The outlines of `object` as polylines, the area the hatch covers. `None` for an object with no
/// area of its own (a sketch, a dimension, a clone of a group, text or image of a picture clone).
#[must_use]
pub fn object_polygons(object: &ObjectSnapshot) -> Option<Vec<Vec<Point>>> {
    let paths = |path: &curvyo_document_core::PathSnapshot| {
        path.subpaths()
            .map(|sub| flatten(sub.anchors, sub.closed))
            .collect::<Vec<_>>()
    };
    match object {
        ObjectSnapshot::Path(path) => Some(paths(path)),
        ObjectSnapshot::Clone(clone) if !clone.is_picture() => Some(paths(&clone.resolved)),
        ObjectSnapshot::Primitive(primitive) => {
            let drawn = curvyo_document_core::drawing_outline_of_rotated(
                &primitive.shape,
                primitive.rotation,
            );
            let anchors = crate::shape_preview::outline_to_anchors(&drawn.anchors);
            Some(vec![flatten(&anchors, drawn.closed)])
        }
        ObjectSnapshot::Text(text) => text.outline.as_ref().map(|outline| paths(&outline.glyphs)),
        _ => None,
    }
}

/// The hatch of `polygons` (their outlines, even-odd over all of them), at the view's scale.
#[must_use]
pub fn hatch_list(polygons: &[Vec<Point>], view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    let scale = view.scale();
    if polygons.iter().all(Vec::is_empty) || scale <= 0.0 {
        return list;
    }
    // Lines x - y = c. The perpendicular distance of two of them is (c2 - c1) / sqrt 2, so the grid
    // step in c is the period times sqrt 2.
    let step = HATCH_PERIOD_PX / scale * std::f64::consts::SQRT_2;
    let width_mm = HATCH_WIDTH_PX / scale;
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for point in polygons.iter().flatten() {
        low = low.min(point.x - point.y);
        high = high.max(point.x - point.y);
    }
    let mut lines = 0;
    // Grid indices of the lines that can cross the bounds: far inside the i64 range for any
    // document (a millimetre grid of 1e-6 mm steps over 1e6 mm is 1e12 indices).
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let (from_index, to_index) = ((low / step).floor() as i64, (high / step).ceil() as i64);
    for k in from_index..=to_index {
        #[allow(clippy::cast_precision_loss)]
        let c = k as f64 * step;
        let mut crossings = crossings_on(polygons, c);
        crossings.sort_by(f64::total_cmp);
        for pair in crossings.as_chunks::<2>().0 {
            if lines >= HATCH_LINES_PER_OBJECT {
                return list;
            }
            lines += 1;
            let a = Point::new(pair[0], pair[0] - c);
            let b = Point::new(pair[1], pair[1] - c);
            list.extend(thick_line(a, b, width_mm, OP_ENGRAVE_HATCH));
        }
    }
    list
}

/// The x of every crossing of the line x - y = `c` with an edge of `polygons`.
fn crossings_on(polygons: &[Vec<Point>], c: f64) -> Vec<f64> {
    let mut xs = Vec::new();
    for polygon in polygons {
        let count = polygon.len();
        for i in 0..count {
            let start = polygon[i];
            let end = polygon[(i + 1) % count];
            let f_start = start.x - start.y - c;
            let f_end = end.x - end.y - c;
            if (f_start < 0.0) != (f_end < 0.0) {
                let t = f_start / (f_start - f_end);
                xs.push(start.x + t * (end.x - start.x));
            }
        }
    }
    xs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(from: f64, to: f64) -> Vec<Point> {
        vec![
            Point::new(from, from),
            Point::new(to, from),
            Point::new(to, to),
            Point::new(from, to),
        ]
    }

    #[test]
    fn a_square_is_hatched_and_a_hole_is_not() {
        let view = ViewTransform::identity();
        let solid = hatch_list(&[square(0.0, 20.0)], view);
        let ring = hatch_list(&[square(0.0, 20.0), square(5.0, 15.0)], view);
        assert!(solid.triangle_count() > 0);
        // No hatch vertex lies strictly inside the hole.
        let inside_hole = ring.triangles.iter().any(|v| {
            v.position.x > 6.0 && v.position.x < 14.0 && v.position.y > 6.0 && v.position.y < 14.0
        });
        assert!(!inside_hole, "the hole is not hatched");
        let hatch_inside_the_square = ring
            .triangles
            .iter()
            .any(|v| v.position.x > 1.0 && v.position.x < 19.0);
        assert!(hatch_inside_the_square);
    }

    #[test]
    fn the_hatch_lines_are_six_pixels_apart_in_screen_space() {
        // At scale 2 px per mm the period is 3 mm; a 20 mm square has about 20 / 3 * sqrt 2 lines
        // along each diagonal direction, so the count scales with the zoom.
        let zoomed_out = hatch_list(
            &[square(0.0, 20.0)],
            ViewTransform::new(1.0, Point::new(0.0, 0.0)),
        );
        let zoomed_in = hatch_list(
            &[square(0.0, 20.0)],
            ViewTransform::new(4.0, Point::new(0.0, 0.0)),
        );
        assert!(zoomed_in.triangle_count() > zoomed_out.triangle_count() * 3);
    }

    #[test]
    fn the_per_object_cap_stops_a_huge_area() {
        let big = square(0.0, 1.0e6);
        let list = hatch_list(&[big], ViewTransform::new(1.0, Point::new(0.0, 0.0)));
        assert!(list.triangle_count() <= HATCH_LINES_PER_OBJECT * 2);
    }

    #[test]
    fn flatten_keeps_a_straight_outline_as_its_corners() {
        let anchors: Vec<AnchorSnapshot> = square(0.0, 10.0)
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(1, i as u64),
                    p,
                )
            })
            .collect();
        assert_eq!(flatten(&anchors, true).len(), 4);
    }
}
