//! Draws the Measure tool's measurement at constant screen size (`specs/0070-measure-and-dimension`
//! UX notes §2 and §3): the blue line with end ticks, the anchor dots and a radius line.

use crate::{DrawList, glyphs, theme};
use curvyo_document_core::{Point, Vec2, ViewTransform};

/// Line width of the measure line.
const LINE_PX: f64 = 1.0;
/// Length of an end tick, across the line.
const TICK_PX: f64 = 8.0;
/// Diameter of an anchor dot and of its white casing.
const DOT_PX: f64 = 6.0;
const DOT_CASING_PX: f64 = 8.0;

fn ticked_line(list: &mut DrawList, a: Point, b: Point, ends: &[Point], px: f64) {
    list.extend(glyphs::thick_line(a, b, LINE_PX * px, theme::PREVIEW_NEW));
    let d = a.vector_to(b);
    if d.length() <= f64::EPSILON {
        return;
    }
    let across = Vec2::new(-d.y, d.x).normalized_to(TICK_PX / 2.0 * px);
    for &end in ends {
        list.extend(glyphs::thick_line(
            end.translated(across),
            end.translated(across.negated()),
            LINE_PX * px,
            theme::PREVIEW_NEW,
        ));
    }
}

/// The anchor dots of a measurement: 6px `--preview-new` with a 1px white casing.
#[must_use]
pub fn build_measure_dots(points: &[Point], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    for &point in points {
        list.extend(glyphs::circle(point, DOT_CASING_PX * px, theme::CHECKER_A));
        list.extend(glyphs::circle(point, DOT_PX * px, theme::PREVIEW_NEW));
    }
    list
}

/// A measurement from `a` to `b`: the line with ticks at both ends.
#[must_use]
pub fn build_measure_line(a: Point, b: Point, view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    ticked_line(&mut list, a, b, &[a, b], 1.0 / view.scale());
    list
}

/// A radius measurement: the line from the centre through `rim`, a tick at the outline.
#[must_use]
pub fn build_measure_radius(center: Point, rim: Point, view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    ticked_line(&mut list, center, rim, &[rim], 1.0 / view.scale());
    list
}

/// The hover band along a pick source (UX notes §6): a 4px `--segment-hover` band along the
/// polyline `points` (a segment, or an arc sampled by the caller).
#[must_use]
pub fn build_pick_band(points: &[Point], view: ViewTransform) -> DrawList {
    let width = theme::SEGMENT_HOVER_WIDTH_PX / view.scale();
    let mut list = DrawList::default();
    for pair in points.windows(2) {
        list.extend(glyphs::thick_line(
            pair[0],
            pair[1],
            width,
            theme::SEGMENT_HOVER,
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_band_is_one_quad_per_piece_in_the_hover_colour() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        let list = build_pick_band(&points, view);
        assert_eq!(list.triangles.len(), 2 * 6);
        assert!(
            list.triangles
                .iter()
                .all(|v| v.color == theme::SEGMENT_HOVER)
        );
    }

    #[test]
    fn a_measure_line_is_blue_with_two_ticks() {
        let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let list = build_measure_line(Point::new(0.0, 0.0), Point::new(50.0, 0.0), view);
        assert_eq!(list.triangles.len(), 3 * 6, "the line and two ticks");
        assert!(list.triangles.iter().all(|v| v.color == theme::PREVIEW_NEW));
    }
}
