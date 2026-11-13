//! Draws the width points of a path as screen-sized diamonds on its centre line.

use crate::{DrawList, glyphs, theme};
use curvyo_document_core::{Point, ViewTransform};

/// The size of a width diamond, screen pixels (`specs/0115` criterion 5).
pub const WIDTH_DIAMOND_PX: f64 = 16.0;
/// The dark ring around a diamond and the light casing around that, screen pixels each side.
const RING_PX: f64 = 1.5;
const CASING_PX: f64 = 1.5;

/// One diamond on the centre line.
#[derive(Debug, Clone, Copy)]
pub struct WidthDiamond {
    /// The point on the centre line.
    pub at: Point,
    /// The width point is selected.
    pub selected: bool,
    /// The ghost under the pointer where a click adds a point.
    pub ghost: bool,
}

/// Draws the diamonds at a constant screen size: a white fill (the accent when selected) in a
/// dark node-stroke ring and a light casing, so a diamond reads on dark and light strokes and on
/// the canvas; the ghost is the same shape and ring with a faint fill, so it reads on a light page
/// too (UX review 2026-11-12).
#[must_use]
pub fn build_width_diamonds(diamonds: &[WidthDiamond], view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    let scale = view.scale();
    for diamond in diamonds {
        let fill = if diamond.selected {
            theme::ACCENT
        } else {
            theme::CHECKER_A
        };
        let fill = if diamond.ghost {
            fill.with_alpha(fill.a / 2)
        } else {
            fill
        };
        let size = WIDTH_DIAMOND_PX;
        for (px, color) in [
            (size + 2.0 * (RING_PX + CASING_PX), theme::SELECTION_CASING),
            (size + 2.0 * RING_PX, theme::NODE_STROKE),
            (size, fill),
        ] {
            list.extend(glyphs::diamond(diamond.at, px / scale, color));
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_diamond_draws_and_the_ghost_is_faint() {
        let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let one = build_width_diamonds(
            &[WidthDiamond {
                at: Point::new(1.0, 1.0),
                selected: false,
                ghost: false,
            }],
            view,
        );
        let ghost = build_width_diamonds(
            &[WidthDiamond {
                at: Point::new(1.0, 1.0),
                selected: false,
                ghost: true,
            }],
            view,
        );
        assert!(one.triangle_count() > 0);
        assert_eq!(one.triangle_count(), ghost.triangle_count());
        // UX review 2026-11-12: on a light page the ghost reads by its full dark ring; only
        // its fill is faint.
        assert!(
            ghost
                .triangles
                .iter()
                .any(|vertex| vertex.color == theme::NODE_STROKE)
        );
        assert!(
            ghost
                .triangles
                .iter()
                .any(|vertex| vertex.color.a < theme::CHECKER_A.a)
        );
    }
}
