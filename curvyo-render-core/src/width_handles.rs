//! Draws the width points of a path as screen-sized diamonds on its centre line.

use crate::{DrawList, glyphs, theme};
use curvyo_document_core::{Point, ViewTransform};

/// The size of a width diamond, screen pixels.
const DIAMOND_PX: f64 = 11.0;

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

/// Draws the diamonds at a constant screen size: outlined, filled in the accent when selected,
/// faint for the ghost.
#[must_use]
pub fn build_width_diamonds(diamonds: &[WidthDiamond], view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    let scale = view.scale();
    for diamond in diamonds {
        let (outline, fill) = if diamond.ghost {
            (theme::ACCENT_HOVER, theme::ACCENT_HOVER)
        } else if diamond.selected {
            (theme::NODE_STROKE, theme::ACCENT)
        } else {
            (theme::NODE_STROKE, theme::CHECKER_A)
        };
        list.extend(glyphs::diamond(
            diamond.at,
            (DIAMOND_PX + 2.0) / scale,
            outline,
        ));
        list.extend(glyphs::diamond(diamond.at, DIAMOND_PX / scale, fill));
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
    }
}
