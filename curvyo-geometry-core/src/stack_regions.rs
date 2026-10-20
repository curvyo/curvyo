//! What Fracture and Flatten share on the grid (`specs/0037-fracture-and-flatten/adrs.md`): the
//! refusal type, boxes, the area test, and the final form of a region as connected pieces.

use curvyo_document_core::{Point, Vec2};

use crate::boolean::BooleanError;
use crate::boolean_cleanup::{cleanup, is_sliver};
use crate::boolean_grid::{Paths64, Point64, canonical_millimetres, double_area};
use crate::{Outline, OutlineTriple, outline_nesting};

/// Why Fracture or Flatten was refused. Nothing is computed or changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StackError {
    /// The operand checks the boolean operations make refused, with the same offenders.
    #[error(transparent)]
    Operands(#[from] BooleanError),
    /// Fracture would make more pieces than the caller's limit.
    #[error("more pieces than the limit")]
    TooManyPieces,
}

/// The bounding box of a grid region, in grid units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GridBox {
    min: (i64, i64),
    max: (i64, i64),
}

impl GridBox {
    /// The box of `paths`; an empty region gets a box that meets nothing.
    pub(crate) fn of(paths: &Paths64) -> Self {
        let mut grid_box = Self {
            min: (i64::MAX, i64::MAX),
            max: (i64::MIN, i64::MIN),
        };
        for point in paths.iter().flatten() {
            grid_box.min = (grid_box.min.0.min(point.x), grid_box.min.1.min(point.y));
            grid_box.max = (grid_box.max.0.max(point.x), grid_box.max.1.max(point.y));
        }
        grid_box
    }

    /// The box's x range, `min..=max`.
    pub(crate) fn x_range(self) -> (i64, i64) {
        (self.min.0, self.max.0)
    }

    /// Whether the box of the segment `a b` meets this box, a shared edge or point included.
    pub(crate) fn holds_segment_box(self, a: Point64, b: Point64) -> bool {
        a.x.max(b.x) >= self.min.0
            && a.x.min(b.x) <= self.max.0
            && a.y.max(b.y) >= self.min.1
            && a.y.min(b.y) <= self.max.1
    }

    /// The smallest box holding both.
    pub(crate) fn union(self, other: Self) -> Self {
        Self {
            min: (self.min.0.min(other.min.0), self.min.1.min(other.min.1)),
            max: (self.max.0.max(other.max.0), self.max.1.max(other.max.1)),
        }
    }

    /// Whether the two boxes share at least a point, a shared edge or corner included, so the
    /// boundary of one may run along the other's (Weld, `specs/0068-laserable-lettering`).
    pub(crate) fn touches(self, other: Self) -> bool {
        self.min.0 <= other.max.0
            && other.min.0 <= self.max.0
            && self.min.1 <= other.max.1
            && other.min.1 <= self.max.1
    }

    /// Whether the two boxes share more than an edge, so the regions may overlap in area.
    pub(crate) fn meets(self, other: Self) -> bool {
        self.min.0 < other.max.0
            && other.min.0 < self.max.0
            && self.min.1 < other.max.1
            && other.min.1 < self.max.1
    }
}

/// Whether an overlay result has area: an outer outline that is not rounding noise. Shapes that
/// only touch along an edge or at a point give none (criterion 3).
pub(crate) fn has_area(paths: &Paths64) -> bool {
    paths
        .iter()
        .any(|path| path.len() >= 3 && double_area(path) > 0 && !is_sliver(path))
}

/// The final form of a region in millimetres, cleaned once (never between the steps of a fold).
pub(crate) fn final_outlines(region: &Paths64) -> Vec<Vec<Point>> {
    canonical_millimetres(&cleanup(region))
}

/// `outlines` (canonical form) cut into connected pieces: each outer outline with the holes
/// directly inside it, in the canonical order of the outer outlines, outer outline first.
pub(crate) fn connected_pieces(outlines: Vec<Vec<Point>>) -> Vec<Vec<Vec<Point>>> {
    let triples: Vec<Vec<OutlineTriple>> = outlines
        .iter()
        .map(|outline| {
            outline
                .iter()
                .map(|&point| (point, Vec2::ZERO, Vec2::ZERO))
                .collect()
        })
        .collect();
    let views: Vec<Outline<'_>> = triples
        .iter()
        .map(|anchors| Outline::new(anchors, true))
        .collect();
    let nesting = outline_nesting(&views);
    let mut piece_of: Vec<Option<usize>> = vec![None; outlines.len()];
    let mut pieces: Vec<Vec<usize>> = Vec::new();
    for (index, place) in nesting.iter().enumerate() {
        if place.depth % 2 == 0 {
            piece_of[index] = Some(pieces.len());
            pieces.push(vec![index]);
        }
    }
    for (index, place) in nesting.iter().enumerate() {
        if place.depth % 2 == 1
            && let Some(piece) = place.parent.and_then(|parent| piece_of[parent])
        {
            pieces[piece].push(index);
        }
    }
    let mut outlines: Vec<Option<Vec<Point>>> = outlines.into_iter().map(Some).collect();
    pieces
        .into_iter()
        .map(|members| {
            members
                .into_iter()
                .filter_map(|index| outlines[index].take())
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::normalize;

    fn square(x: i64, y: i64, side: i64) -> Vec<Point64> {
        vec![
            Point64::new(x, y),
            Point64::new(x + side, y),
            Point64::new(x + side, y + side),
            Point64::new(x, y + side),
        ]
    }

    #[test]
    fn boxes_that_share_only_an_edge_do_not_meet() {
        let a = GridBox::of(&vec![square(0, 0, 10)]);
        assert!(!a.meets(GridBox::of(&vec![square(10, 0, 10)])));
        assert!(a.meets(GridBox::of(&vec![square(9, 9, 10)])));
        assert!(!a.meets(GridBox::of(&Paths64::new())));
    }

    #[test]
    fn two_separate_rings_are_two_pieces_with_their_holes() {
        let region = normalize(&vec![
            square(0, 0, 10_000),
            square(2_000, 2_000, 6_000).into_iter().rev().collect(),
            square(20_000, 0, 10_000),
            square(22_000, 2_000, 6_000).into_iter().rev().collect(),
        ]);
        let pieces = connected_pieces(final_outlines(&region));
        assert_eq!(pieces.len(), 2);
        assert!(pieces.iter().all(|piece| piece.len() == 2));
        assert!(pieces[0][0][0].x < pieces[1][0][0].x);
    }
}
