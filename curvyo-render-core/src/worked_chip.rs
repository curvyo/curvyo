//! The Worked chip of a test grid (`specs/0028-material-test-library` ux-notes §7.3): a 20 px
//! circle with a tick, centred on the marked cell's square at any zoom; drawn in both views,
//! never output or exported.

use curvyo_document_core::{Point, ViewTransform};

use crate::glyphs::{DrawList, circle, thick_line};
use crate::theme::{OP_CHIP_GROUND, OP_INK, OP_LANE_LINE};

/// The chip's diameter, screen pixels.
pub const WORKED_CHIP_PX: f64 = 20.0;

/// The Worked chip centred on `centre` (document millimetres): `--toolbar-bg` ground, 1 px
/// `--lane-line` border, a 14 px ink tick. Nothing at a degenerate zoom.
#[must_use]
pub fn worked_chip_list(centre: Point, view: ViewTransform) -> DrawList {
    let scale = view.scale();
    let mut list = DrawList::default();
    if scale <= 0.0 || !scale.is_finite() {
        return list;
    }
    let px = 1.0 / scale;
    list.extend(circle(centre, WORKED_CHIP_PX * px, OP_LANE_LINE));
    list.extend(circle(centre, (WORKED_CHIP_PX - 2.0) * px, OP_CHIP_GROUND));
    // The tick inside a 14 px box: a short leg down-right, a long leg up-right.
    let at = |x: f64, y: f64| Point::new(centre.x + (x - 7.0) * px, centre.y + (y - 7.0) * px);
    let width = 1.75 * px;
    list.extend(thick_line(at(2.0, 7.5), at(5.5, 11.0), width, OP_INK));
    list.extend(thick_line(at(5.5, 11.0), at(12.0, 3.5), width, OP_INK));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chip_is_drawn_at_any_zoom_and_never_at_a_degenerate_one() {
        let near = worked_chip_list(Point::new(5.0, 5.0), ViewTransform::identity());
        assert!(near.triangles.len() > 3);
        let far = worked_chip_list(
            Point::new(5.0, 5.0),
            ViewTransform::new(0.01, Point::new(0.0, 0.0)),
        );
        assert_eq!(
            far.triangles.len(),
            near.triangles.len(),
            "the same parts at every zoom"
        );
        let flat = worked_chip_list(
            Point::new(5.0, 5.0),
            ViewTransform::new(0.0, Point::new(0.0, 0.0)),
        );
        assert_eq!(flat.triangles.len(), 0);
    }
}
