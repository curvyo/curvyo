//! The contours one Cut-out item runs: the outer contour of the union of its leaves, grown by the
//! margin (criterion 23, Frage 168 A). The geometry is the kernel's (`curvyo-geometry-core`); the
//! tolerances come from the caller.

use curvyo_document_core::{Length, Point, Tolerance, Vec2};
use curvyo_geometry_core::{
    BooleanError, BooleanOp, OffsetError, OffsetJoin, Outline, OutlineTriple, boolean, offset,
    offset_distance_is_zero,
};

/// Why a Cut-out contour could not be made.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CutOutError {
    /// The union of the leaves failed in the kernel.
    #[error("the union of the leaves failed: {0}")]
    Union(#[from] BooleanError),
    /// The margin offset failed in the kernel.
    #[error("the margin offset failed: {0}")]
    Offset(#[from] OffsetError),
}

/// The contours one Cut-out item runs. `leaves` holds one entry per drawn leaf of the item, each
/// the leaf's outlines (an open one is closed by a straight line, `0054`). The result is the
/// outer boundary of the union: holes and inner detail do not run. A `margin` of 0 skips the
/// offset; otherwise the contour grows by the Round outset (`0038`).
///
/// A union whose parts do not touch has one outer boundary per part; each runs as its own
/// contour.
///
/// # Errors
/// [`CutOutError::Union`] and [`CutOutError::Offset`] when the kernel refuses the operands or the
/// margin.
pub fn cut_out_contours(
    leaves: &[&[Outline<'_>]],
    margin: Length,
    tolerance: Tolerance,
) -> Result<Vec<Vec<Point>>, CutOutError> {
    let union = boolean(BooleanOp::Union, leaves, tolerance)?;
    let outer = outer_boundaries(&union.into_outlines());
    if offset_distance_is_zero(margin) {
        return Ok(outer);
    }
    let mut grown = Vec::new();
    for contour in &outer {
        let triples: Vec<OutlineTriple> = contour
            .iter()
            .map(|&point| (point, Vec2::ZERO, Vec2::ZERO))
            .collect();
        let outset = offset(
            &[Outline::new(&triples, true)],
            margin,
            OffsetJoin::Round,
            tolerance,
        )?;
        grown.extend(
            outset
                .outlines()
                .iter()
                .map(|outline| outline.iter().map(|anchor| anchor.point).collect()),
        );
    }
    Ok(grown)
}

/// The outlines of a union that no other outline encloses: a hole lies inside the outer one.
fn outer_boundaries(outlines: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let enclosed = |index: usize| {
        let Some(&probe) = outlines[index].first() else {
            return false;
        };
        outlines
            .iter()
            .enumerate()
            .any(|(other, polygon)| other != index && contains(polygon, probe))
    };
    (0..outlines.len())
        .filter(|&index| !enclosed(index))
        .map(|index| outlines[index].clone())
        .collect()
}

/// Whether `probe` lies inside `polygon` (crossing number, the polygon closed by its last
/// segment).
fn contains(polygon: &[Point], probe: Point) -> bool {
    let mut inside = false;
    let Some(&(mut previous)) = polygon.last() else {
        return false;
    };
    for &current in polygon {
        let crosses = (current.y > probe.y) != (previous.y > probe.y);
        if crosses {
            let x_at = previous.x
                + (probe.y - previous.y) / (current.y - previous.y) * (current.x - previous.x);
            if probe.x < x_at {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 0.01;

    fn tolerance() -> Tolerance {
        Tolerance::from_mm(TOLERANCE)
    }

    fn corner(x: f64, y: f64) -> OutlineTriple {
        (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<OutlineTriple> {
        vec![
            corner(x0, y0),
            corner(x1, y0),
            corner(x1, y1),
            corner(x0, y1),
        ]
    }

    fn bounds(contour: &[Point]) -> (f64, f64, f64, f64) {
        contour.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(a, b, c, d), p| (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)),
        )
    }

    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= 0.05
    }

    #[test]
    fn two_clones_joined_by_a_bar_give_one_closed_contour_and_no_line_inside() {
        let left = square(0.0, 0.0, 10.0, 10.0);
        let right = square(20.0, 0.0, 30.0, 10.0);
        let bar = square(8.0, 4.0, 22.0, 6.0);
        let (l, r, b) = (
            Outline::new(&left, true),
            Outline::new(&right, true),
            Outline::new(&bar, true),
        );
        let contours =
            cut_out_contours(&[&[l], &[r], &[b]], Length::from_mm(0.0), tolerance()).unwrap();
        assert_eq!(contours.len(), 1, "one closed line for the item");
        let (x0, y0, x1, y1) = bounds(&contours[0]);
        assert!(close(x0, 0.0) && close(y0, 0.0) && close(x1, 30.0) && close(y1, 10.0));
    }

    #[test]
    fn disjoint_parts_of_one_item_give_one_contour_each() {
        let left = square(0.0, 0.0, 10.0, 10.0);
        let right = square(20.0, 0.0, 30.0, 10.0);
        let (l, r) = (Outline::new(&left, true), Outline::new(&right, true));
        let contours = cut_out_contours(&[&[l], &[r]], Length::from_mm(0.0), tolerance()).unwrap();
        assert_eq!(contours.len(), 2, "one contour per disjoint part");
    }

    #[test]
    fn overlapping_items_stay_separate_contours() {
        // Frage 168 A: each item is cut on its own; two items that overlap are not merged.
        let first = square(0.0, 0.0, 10.0, 10.0);
        let second = square(5.0, 5.0, 15.0, 15.0);
        let item_a = [Outline::new(&first, true)];
        let item_b = [Outline::new(&second, true)];
        let mut contours =
            cut_out_contours(&[&item_a[..]], Length::from_mm(0.0), tolerance()).unwrap();
        contours
            .extend(cut_out_contours(&[&item_b[..]], Length::from_mm(0.0), tolerance()).unwrap());
        assert_eq!(contours.len(), 2, "one contour per item");
        let (x0, _, x1, _) = bounds(&contours[1]);
        assert!(
            close(x0, 5.0) && close(x1, 15.0),
            "the second item keeps its own outline"
        );
    }

    #[test]
    fn a_hole_of_a_leaf_does_not_run() {
        let outer = square(0.0, 0.0, 10.0, 10.0);
        let hole = square(3.0, 3.0, 7.0, 7.0);
        let ring = [Outline::new(&outer, true), Outline::new(&hole, true)];
        let contours = cut_out_contours(&[&ring], Length::from_mm(0.0), tolerance()).unwrap();
        assert_eq!(contours.len(), 1, "the hole is inside the outer contour");
        let (x0, _, x1, _) = bounds(&contours[0]);
        assert!(close(x0, 0.0) && close(x1, 10.0));
    }

    #[test]
    fn an_open_leaf_is_closed_by_a_straight_line() {
        let three_sides = [
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(10.0, 10.0),
            corner(0.0, 10.0),
        ];
        let open = [Outline::new(&three_sides, false)];
        let contours = cut_out_contours(&[&open], Length::from_mm(0.0), tolerance()).unwrap();
        assert_eq!(contours.len(), 1);
    }

    #[test]
    fn a_margin_grows_the_contour_by_the_round_outset() {
        let box_ = square(0.0, 0.0, 10.0, 10.0);
        let leaf = [Outline::new(&box_, true)];
        let contours = cut_out_contours(&[&leaf], Length::from_mm(1.0), tolerance()).unwrap();
        assert_eq!(contours.len(), 1);
        let (x0, y0, x1, y1) = bounds(&contours[0]);
        assert!(close(x0, -1.0) && close(y0, -1.0) && close(x1, 11.0) && close(y1, 11.0));
    }

    #[test]
    fn a_zero_margin_skips_the_offset_and_keeps_the_union_exact() {
        let box_ = square(0.0, 0.0, 10.0, 10.0);
        let leaf = [Outline::new(&box_, true)];
        let contours = cut_out_contours(&[&leaf], Length::from_mm(0.0), tolerance()).unwrap();
        let (x0, _, x1, _) = bounds(&contours[0]);
        assert!(close(x0, 0.0) && close(x1, 10.0));
    }

    #[test]
    fn a_point_is_inside_a_square_and_not_outside() {
        let polygon = [
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 4.0),
            Point::new(0.0, 4.0),
        ];
        assert!(contains(&polygon, Point::new(2.0, 2.0)));
        assert!(!contains(&polygon, Point::new(5.0, 2.0)));
    }
}
