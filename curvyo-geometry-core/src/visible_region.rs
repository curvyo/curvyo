//! Flatten's kernel: the visible region of every operand of a stack, its region minus the regions
//! of all operands above it (`specs/0037-fracture-and-flatten` criteria 10 and 11; not the
//! curve-to-polyline `flatten` of this crate).
//!
//! A fold from the top on the grid (`adrs.md`): each operand is cut by the running union of the
//! operands above it, then joined to it; a box test spares the overlay of an operand that lies
//! apart. Cleanup and the canonical form run once per result.

use curvyo_document_core::{Point, Tolerance};
use i_overlay::core::overlay_rule::OverlayRule;

use crate::Outline;
use crate::boolean::operand_regions;
use crate::boolean_grid::{Paths64, run};
use crate::stack_regions::{GridBox, StackError, final_outlines, has_area};

/// What Flatten makes of one operand.
#[derive(Debug, Clone, PartialEq)]
pub enum Visible {
    /// Nothing above overlaps it in area: it stays exactly as it is.
    Untouched,
    /// Part of it is covered: its visible region, outlines in the canonical form of the boolean
    /// kernel (one compound path when there are several).
    Trimmed(Vec<Vec<Point>>),
    /// It is covered completely.
    Hidden,
}

/// The visible region of each operand, one entry per operand in order. `operands` are in
/// stacking order, bottom first; each is a list of outlines painted with the nonzero rule, as
/// [`crate::boolean`] takes them, with the same `tolerance`.
///
/// # Errors
/// [`StackError::Operands`] for the operand checks of [`crate::boolean`]; nothing is computed
/// then.
pub fn visible_regions(
    operands: &[&[Outline<'_>]],
    tolerance: Tolerance,
) -> Result<Vec<Visible>, StackError> {
    let regions = operand_regions(operands, tolerance)?;
    let mut visible = vec![Visible::Untouched; regions.len()];
    // The union of the operands above the current one, and its box.
    let mut above = Paths64::new();
    let mut above_box = GridBox::of(&above);
    for (index, region) in regions.iter().enumerate().rev() {
        let region_box = GridBox::of(region);
        let overlaps =
            region_box.meets(above_box) && has_area(&run(OverlayRule::Intersect, region, &above));
        if overlaps {
            let outlines = final_outlines(&run(OverlayRule::Difference, region, &above));
            visible[index] = if outlines.is_empty() {
                Visible::Hidden
            } else {
                Visible::Trimmed(outlines)
            };
        }
        above = if above.is_empty() {
            region.clone()
        } else {
            run(OverlayRule::Union, &above, region)
        };
        above_box = above_box.union(region_box);
    }
    Ok(visible)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{OutlineTriple, signed_area_mm2};
    use curvyo_document_core::Vec2;

    const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

    fn square(x: f64, y: f64, side: f64) -> Vec<OutlineTriple> {
        [(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
            .iter()
            .map(|&(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
            .collect()
    }

    pub(crate) fn disc(r: f64) -> Vec<OutlineTriple> {
        let k = 0.552_284_749_830_793_4 * r;
        let a = |x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
            (
                Point::new(x, y),
                Vec2::new(hin.0, hin.1),
                Vec2::new(hout.0, hout.1),
            )
        };
        vec![
            a(r, 0.0, (0.0, -k), (0.0, k)),
            a(0.0, r, (k, 0.0), (-k, 0.0)),
            a(-r, 0.0, (0.0, k), (0.0, -k)),
            a(0.0, -r, (-k, 0.0), (k, 0.0)),
        ]
    }

    fn flatten_stack(shapes: &[Vec<OutlineTriple>]) -> Vec<Visible> {
        let outlines: Vec<[Outline<'_>; 1]> = shapes
            .iter()
            .map(|anchors| [Outline::new(anchors, true)])
            .collect();
        let operands: Vec<&[Outline<'_>]> = outlines.iter().map(<[_; 1]>::as_slice).collect();
        visible_regions(&operands, TOLERANCE).unwrap()
    }

    /// Criterion 10: the lower square becomes the L of 300 mm² with 6 nodes; the upper one is
    /// untouched.
    #[test]
    fn the_lower_square_is_trimmed_to_an_l_and_the_upper_is_untouched() {
        let result = flatten_stack(&[square(0.0, 0.0, 20.0), square(10.0, 10.0, 20.0)]);
        let Visible::Trimmed(outlines) = &result[0] else {
            panic!("the lower square is trimmed: {result:?}");
        };
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].len(), 6);
        assert!((signed_area_mm2(&outlines[0]) - 300.0).abs() < 1e-6);
        assert_eq!(result[1], Visible::Untouched);
    }

    /// Criterion 11: a small disc under a large one is hidden; the large one is untouched.
    #[test]
    fn a_disc_under_a_larger_disc_is_hidden() {
        assert_eq!(
            flatten_stack(&[disc(10.0), disc(20.0)]),
            vec![Visible::Hidden, Visible::Untouched]
        );
    }

    /// Criterion 3: shapes that only share an edge do not overlap.
    #[test]
    fn squares_sharing_an_edge_stay_untouched() {
        assert_eq!(
            flatten_stack(&[square(0.0, 0.0, 10.0), square(10.0, 0.0, 10.0)]),
            vec![Visible::Untouched, Visible::Untouched]
        );
    }
}
