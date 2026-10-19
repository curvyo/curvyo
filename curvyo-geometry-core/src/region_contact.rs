//! How a cell of Fracture lies against a new operand's region, decided without an overlay when
//! their boundaries do not meet (`specs/0037-fracture-and-flatten/plan.md`, note on nesting).
//!
//! Two regions whose boundaries neither cross nor touch are apart, or one holds the other, and
//! one vertex per outline tells which. Everything is exact integer arithmetic on the grid; any
//! contact of the boundaries, a shared point included, answers [`Contact::Unknown`], and the
//! caller then runs the overlay.

use crate::boolean_grid::{Path64, Paths64, Point64};
use crate::stack_regions::GridBox;

/// How a cell lies against a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Contact {
    /// The two share no point.
    Apart,
    /// The cell lies inside the region.
    Inside,
    /// Anything else; only an overlay tells.
    Unknown,
}

/// The edges of a region, sorted into vertical strips of equal width, so the edges near a
/// segment are found without visiting them all.
pub(crate) struct EdgeIndex<'a> {
    region: &'a Paths64,
    bounds: GridBox,
    min_x: i64,
    width: i64,
    strips: Vec<Vec<(Point64, Point64)>>,
}

impl<'a> EdgeIndex<'a> {
    /// The index of `region`, whose box is `bounds`.
    pub(crate) fn new(region: &'a Paths64, bounds: GridBox) -> Self {
        let edges: Vec<(Point64, Point64)> = region.iter().flat_map(edges).collect();
        let (min_x, max_x) = bounds.x_range();
        let count = (edges.len() / 2).max(1);
        let span = (max_x - min_x).max(0) + 1;
        let width = span.div_euclid(i64::try_from(count).unwrap_or(i64::MAX)) + 1;
        let mut index = Self {
            region,
            bounds,
            min_x,
            width,
            strips: vec![Vec::new(); count],
        };
        for edge in edges {
            for strip in index.strips_of(edge.0.x.min(edge.1.x), edge.0.x.max(edge.1.x)) {
                index.strips[strip].push(edge);
            }
        }
        index
    }

    /// The strips the x range `low..=high` reaches, clamped to the index.
    fn strips_of(&self, low: i64, high: i64) -> std::ops::RangeInclusive<usize> {
        let last = self.strips.len() - 1;
        let strip = |x: i64| {
            usize::try_from((x - self.min_x).div_euclid(self.width))
                .unwrap_or(0)
                .min(last)
        };
        strip(low)..=strip(high)
    }

    /// Whether the segment `a b` meets an edge of the region, a shared point included.
    fn meets(&self, a: Point64, b: Point64) -> bool {
        if !self.bounds.holds_segment_box(a, b) {
            return false;
        }
        self.strips_of(a.x.min(b.x), a.x.max(b.x)).any(|strip| {
            self.strips[strip]
                .iter()
                .any(|&(c, d)| segments_meet(a, b, c, d))
        })
    }

    /// Whether an edge of the region reaches into `bounds` at all, a cheap first test: the
    /// boundary of a cell lies within its box.
    fn reaches(&self, bounds: GridBox) -> bool {
        let (low, high) = bounds.x_range();
        self.strips_of(low, high).any(|strip| {
            self.strips[strip]
                .iter()
                .any(|&(c, d)| bounds.holds_segment_box(c, d))
        })
    }

    /// How `cell`, whose box is `bounds`, lies against the region.
    pub(crate) fn contact(&self, cell: &Paths64, bounds: GridBox) -> Contact {
        let boundaries_meet = self.reaches(bounds)
            && cell
                .iter()
                .any(|path| edges(path).any(|(a, b)| self.meets(a, b)));
        if boundaries_meet {
            return Contact::Unknown;
        }
        let region_in_cell = self
            .region
            .iter()
            .any(|path| path.first().is_some_and(|&p| winding(cell, p) != 0));
        if region_in_cell {
            return Contact::Unknown;
        }
        let inside: Vec<bool> = cell
            .iter()
            .filter_map(|path| path.first())
            .map(|&p| winding(self.region, p) != 0)
            .collect();
        if inside.iter().all(|&i| i) {
            Contact::Inside
        } else if inside.iter().all(|&i| !i) {
            Contact::Apart
        } else {
            Contact::Unknown
        }
    }
}

/// The closed polygon's edges, the closing one included.
fn edges(path: &Path64) -> impl Iterator<Item = (Point64, Point64)> + '_ {
    let count = path.len();
    (0..count).map(move |i| (path[i], path[(i + 1) % count]))
}

/// The orientation of `c` against the line `a b`: positive, negative or zero. Exact.
fn orientation(a: Point64, b: Point64, c: Point64) -> i128 {
    i128::from(b.x - a.x) * i128::from(c.y - a.y) - i128::from(b.y - a.y) * i128::from(c.x - a.x)
}

/// Whether `c` lies within the box of the segment `a b` (for a point on its line).
fn within(a: Point64, b: Point64, c: Point64) -> bool {
    c.x >= a.x.min(b.x) && c.x <= a.x.max(b.x) && c.y >= a.y.min(b.y) && c.y <= a.y.max(b.y)
}

/// Whether the closed segments `a b` and `c d` share a point.
fn segments_meet(a: Point64, b: Point64, c: Point64, d: Point64) -> bool {
    let (o1, o2) = (orientation(a, b, c), orientation(a, b, d));
    let (o3, o4) = (orientation(c, d, a), orientation(c, d, b));
    if o1.signum() * o2.signum() < 0 && o3.signum() * o4.signum() < 0 {
        return true;
    }
    (o1 == 0 && within(a, b, c))
        || (o2 == 0 && within(a, b, d))
        || (o3 == 0 && within(c, d, a))
        || (o4 == 0 && within(c, d, b))
}

/// The winding number of `paths` about `p`, which lies on none of their edges.
fn winding(paths: &Paths64, p: Point64) -> i32 {
    let mut total = 0;
    for path in paths {
        for (a, b) in edges(path) {
            if a.y <= p.y {
                if b.y > p.y && orientation(a, b, p) > 0 {
                    total += 1;
                }
            } else if b.y <= p.y && orientation(a, b, p) < 0 {
                total -= 1;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::normalize;

    fn square(x: i64, y: i64, side: i64) -> Path64 {
        vec![
            Point64::new(x, y),
            Point64::new(x + side, y),
            Point64::new(x + side, y + side),
            Point64::new(x, y + side),
        ]
    }

    fn contact(cell: &Paths64, region: &Paths64) -> Contact {
        EdgeIndex::new(region, GridBox::of(region)).contact(cell, GridBox::of(cell))
    }

    #[test]
    fn nested_apart_and_touching_shapes_are_told_apart() {
        let ring = normalize(&vec![
            square(0, 0, 100),
            square(20, 20, 60).into_iter().rev().collect(),
        ]);
        let small = vec![square(40, 40, 10)];
        assert_eq!(contact(&ring, &small), Contact::Apart, "in the hole");
        assert_eq!(contact(&small, &ring), Contact::Apart);
        assert_eq!(contact(&small, &vec![square(0, 0, 100)]), Contact::Inside);
        assert_eq!(
            contact(&vec![square(0, 0, 100)], &small),
            Contact::Unknown,
            "the region lies in the cell"
        );
        assert_eq!(
            contact(&vec![square(100, 0, 10)], &vec![square(0, 0, 100)]),
            Contact::Unknown,
            "a shared edge is a contact"
        );
        assert_eq!(contact(&vec![square(200, 0, 10)], &small), Contact::Apart);
    }
}
