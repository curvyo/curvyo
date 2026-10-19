//! The Pen's Ctrl rule: the point that keeps the distance from the last node and takes the
//! nearest angle stop (`specs/0072-pen-angle-snap-and-typed-values/`, `adrs.md` decision 1).

use curvyo_document_core::{Angle, Point, Tolerance};

use crate::angle_snap::snap_angle;

/// A pointer this close to the last node has no direction to snap: it stays where it is.
const ON_LAST: Tolerance = Tolerance::from_mm(1e-9);

/// `raw` turned about `last` to the nearest stop of [`snap_angle`]'s table, measured from the x
/// axis (y down, so 90° points down), with its distance from `last` kept. The node of a Ctrl
/// click, the rubber band with Ctrl held and, with the new node as `last`, the end of a snapped
/// handle all come from here. A `raw` on `last` stays where it is.
#[must_use]
pub fn constrain_segment(last: Point, raw: Point) -> Point {
    let offset = last.vector_to(raw);
    let length = offset.length();
    if !length.is_finite() || length <= ON_LAST.as_mm() {
        return raw;
    }
    let stop = snap_angle(Angle::from_radians(offset.y.atan2(offset.x))).as_radians();
    Point::new(last.x + length * stop.cos(), last.y + length * stop.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 0.005;

    fn assert_near(actual: Point, x: f64, y: f64) {
        assert!(
            (actual.x - x).abs() < TOLERANCE && (actual.y - y).abs() < TOLERANCE,
            "expected ({x}, {y}), got {actual:?}"
        );
    }

    /// Criteria 1 and 2: the examples from L = (100, 50).
    #[test]
    fn the_examples_snap_to_the_nearest_stop_and_keep_the_length() {
        let last = Point::new(100.0, 50.0);
        assert_near(
            constrain_segment(last, Point::new(130.0, 52.0)),
            130.067,
            50.0,
        );
        assert_near(
            constrain_segment(last, Point::new(100.4, 80.0)),
            100.0,
            80.003,
        );
        let diagonal = constrain_segment(last, Point::new(112.0, 38.0));
        assert!(((diagonal.x - 100.0) + (diagonal.y - 50.0)).abs() < 1e-9);
        let fifteen = last.vector_to(constrain_segment(last, Point::new(140.0, 62.0)));
        assert!((fifteen.y.atan2(fifteen.x).to_degrees() - 15.0).abs() < 1e-9);
        assert!((fifteen.length() - 12.0f64.hypot(40.0)).abs() < 1e-9);
    }

    /// Criterion 3: the handle example, N = (130, 50), pointer (150, 52).
    #[test]
    fn a_handle_end_snaps_about_the_new_node() {
        assert_near(
            constrain_segment(Point::new(130.0, 50.0), Point::new(150.0, 52.0)),
            150.1,
            50.0,
        );
    }
}
