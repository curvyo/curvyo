//! The machine origin marker of a manufacturing's output page (`specs/0026-machine-profile`
//! criterion 11, UX notes §6): a view item of constant screen size, never output.

use curvyo_document_core::{Point, ViewTransform};

use crate::{DrawList, RgbaColor, theme};

/// The ink of the marker: the toolbar icon colour.
const MARKER_INK: RgbaColor = theme::NODE_STROKE;
/// The side of the filled square on the corner point, screen pixels.
const SQUARE_PX: f64 = 6.0;
/// The length of each arm along an edge, screen pixels.
const ARM_PX: f64 = 14.0;
/// The width of each arm, screen pixels.
const LINE_PX: f64 = 2.0;
/// The white casing around every part, screen pixels.
const CASING_PX: f64 = 1.0;
/// Below this shorter page side in screen pixels the marker is not drawn.
pub const ORIGIN_MARKER_MIN_PAGE_PX: f64 = 48.0;

/// The marker at `corner` of a page whose inside lies towards `inward` (each component `1.0` or
/// `-1.0`): a 6px square on the corner point and two 14px arms along the edges, inside the page,
/// cased in white. Empty while the page's shorter side is under 48 screen pixels.
#[must_use]
pub fn build_origin_marker(
    corner: Point,
    inward: (f64, f64),
    shorter_side_mm: f64,
    view: ViewTransform,
) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    if shorter_side_mm / px < ORIGIN_MARKER_MIN_PAGE_PX {
        return list;
    }
    let rect = |w: f64, h: f64, grow: f64, color: RgbaColor| {
        let (sx, sy) = inward;
        let a = Point::new(corner.x - sx * grow * px, corner.y - sy * grow * px);
        let b = Point::new(
            corner.x + sx * (w + grow) * px,
            corner.y + sy * (h + grow) * px,
        );
        crate::glyphs::filled_rect(a, b, color)
    };
    for (grow, color) in [(CASING_PX, theme::SELECTION_CASING), (0.0, MARKER_INK)] {
        list.extend(rect(ARM_PX, LINE_PX, grow, color));
        list.extend(rect(LINE_PX, ARM_PX, grow, color));
        list.extend(rect(SQUARE_PX, SQUARE_PX, grow, color));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extent(list: &DrawList, color: RgbaColor) -> (f64, f64, f64, f64) {
        list.triangles.iter().filter(|v| v.color == color).fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), v| {
                (
                    x0.min(v.position.x),
                    y0.min(v.position.y),
                    x1.max(v.position.x),
                    y1.max(v.position.y),
                )
            },
        )
    }

    #[test]
    fn the_marker_keeps_its_screen_size_and_lies_inside_the_page() {
        for scale in [0.5, 2.0, 10.0] {
            let view = ViewTransform::new(scale, Point::new(0.0, 0.0));
            let marker = build_origin_marker(Point::new(400.0, 300.0), (-1.0, -1.0), 300.0, view);
            let (x0, y0, x1, y1) = extent(&marker, MARKER_INK);
            assert!(((x1 - x0) * scale - ARM_PX).abs() < 1e-9);
            assert!(((y1 - y0) * scale - ARM_PX).abs() < 1e-9);
            assert!(x1 <= 400.0 && y1 <= 300.0, "drawn inside the page");
        }
    }

    #[test]
    fn a_small_page_on_screen_shows_no_marker() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let marker = build_origin_marker(Point::new(0.0, 0.0), (1.0, 1.0), 40.0, view);
        assert_eq!(marker.triangles.len(), 0);
    }
}
