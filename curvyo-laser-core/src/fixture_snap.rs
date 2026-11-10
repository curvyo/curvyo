//! The snap points of a board and a stop, placed by the fixture's matrix (`specs/0102-laser-fixtures`
//! criterion 27 b and c; ADR 0049 §2). A clamp has none (criterion 27 d).

use curvyo_document_core::{Length, Point};

/// A fixture's placement: the matrix `[a, b, c, d, e, f]` maps a local point `(x, y)` to
/// `(a·x + c·y + e, b·x + d·y + f)` on the page (the SVG convention). Identity by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    m: [f64; 6],
}

impl Placement {
    /// The identity placement.
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// A placement from its six entries; `None` when an entry is not finite (the document reads
    /// such a matrix as identity and counts it as damage, ADR 0049 §6).
    #[must_use]
    pub fn new(m: [f64; 6]) -> Option<Self> {
        m.iter().all(|v| v.is_finite()).then_some(Self { m })
    }

    /// The local point `p` on the page.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the six entries of the SVG matrix, named as in the spec
    pub fn apply(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.m;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }
}

/// The corners and the edge midpoints of a board of `width` by `height`, in its frame, placed.
#[must_use]
pub fn board_points(width: Length, height: Length, placement: Placement) -> Vec<Point> {
    let (w, h) = (width.as_mm(), height.as_mm());
    [
        (0.0, 0.0),
        (w, 0.0),
        (w, h),
        (0.0, h),
        (w / 2.0, 0.0),
        (w, h / 2.0),
        (w / 2.0, h),
        (0.0, h / 2.0),
    ]
    .into_iter()
    .map(|(x, y)| placement.apply(Point::new(x, y)))
    .collect()
}

/// The outer corner, the two outer ends and the midpoints of the two outer edges of an L-stop
/// whose outer corner is at the frame's origin and whose legs run along +x and +y, placed.
#[must_use]
pub fn stop_points(leg_a: Length, leg_b: Length, placement: Placement) -> Vec<Point> {
    let (a, b) = (leg_a.as_mm(), leg_b.as_mm());
    [
        (0.0, 0.0),
        (a, 0.0),
        (0.0, b),
        (a / 2.0, 0.0),
        (0.0, b / 2.0),
    ]
    .into_iter()
    .map(|(x, y)| placement.apply(Point::new(x, y)))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(value: f64) -> Length {
        Length::from_mm(value)
    }

    #[test]
    fn a_board_has_four_corners_and_four_midpoints_in_its_frame() {
        let points = board_points(mm(100.0), mm(60.0), Placement::IDENTITY);
        assert_eq!(points.len(), 8);
        assert!(points.contains(&Point::new(100.0, 60.0)));
        assert!(points.contains(&Point::new(50.0, 0.0)));
        assert!(points.contains(&Point::new(0.0, 30.0)));
    }

    #[test]
    fn a_quarter_turn_with_an_offset_moves_the_corner_by_the_matrix() {
        // Rotation by 90 degrees (x' = -y, y' = x) and a move of (40, 25).
        let turned = Placement::new([0.0, 1.0, -1.0, 0.0, 40.0, 25.0]).unwrap();
        let points = board_points(mm(100.0), mm(60.0), turned);
        assert!(points.contains(&Point::new(40.0 - 60.0, 25.0 + 100.0)));
        assert!(points.contains(&Point::new(40.0, 25.0)));
    }

    #[test]
    fn a_non_finite_matrix_is_not_a_placement() {
        assert_eq!(
            Placement::new([1.0, 0.0, 0.0, 1.0, f64::NAN, 0.0]),
            None,
            "a NaN entry"
        );
    }

    #[test]
    fn a_stop_has_its_corner_two_ends_and_two_outer_midpoints() {
        let points = stop_points(mm(60.0), mm(40.0), Placement::IDENTITY);
        assert_eq!(points.len(), 5);
        assert_eq!(points[0], Point::new(0.0, 0.0));
        assert!(points.contains(&Point::new(60.0, 0.0)));
        assert!(points.contains(&Point::new(0.0, 40.0)));
    }
}
