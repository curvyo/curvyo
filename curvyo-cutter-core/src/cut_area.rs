//! The cut-area check of a plotter page (`specs/0159-cutting-plotter` criterion 6, `adrs.md`
//! PL9): which items lie outside the active mat's cut area or the matless work area.

use curvyo_document_core::Point;

use crate::mat::Rect;

/// The tolerance of the check: an item may touch the edge (µm rounding of the job).
pub const CUT_AREA_TOLERANCE_MM: f64 = 0.001;

/// The indices of the items whose bounds (`min`, `max` in page mm) do not lie wholly inside
/// `area`; an item partly outside counts as outside.
#[must_use]
pub fn outside_cut_area(bounds: &[(Point, Point)], area: Rect) -> Vec<usize> {
    bounds
        .iter()
        .enumerate()
        .filter(|(_, (min, max))| !area.holds(*min, *max, CUT_AREA_TOLERANCE_MM))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat::MatSize;

    #[test]
    fn a_square_one_mm_past_the_edge_is_outside() {
        let area = MatSize::In12x12.cut_area();
        let inside = (Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let edge = 6.35 + 292.1;
        let past = (Point::new(edge - 9.0, 10.0), Point::new(edge + 1.0, 20.0));
        let touching = (Point::new(edge - 10.0, 10.0), Point::new(edge, 20.0));
        assert_eq!(outside_cut_area(&[inside, past, touching], area), [1]);
    }
}
