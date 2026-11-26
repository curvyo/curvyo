//! The pieces of the union of the operands (`specs/0156-fused-lettering`, Words "Piece" and "Dot
//! piece"; `adrs.md` decision 3 steps 1 to 3): each an outer contour with its holes, indexed by
//! the smallest point of its outer contour, and which of them are dots.

use crate::boolean_grid::{Path64, Paths64, double_area, normalize_shapes};
use crate::fuse::measure::{P, winding};
use crate::stack_regions::GridBox;

/// One connected area of the union.
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    /// The outer contour (positive area) first, then the holes (negative area), each hole
    /// rotated to no particular start but sorted by its smallest point.
    pub(crate) contours: Paths64,
    /// Twice the piece's area, exact.
    pub(crate) double_area: i128,
    /// The box of the outer contour.
    pub(crate) bounds: GridBox,
    /// The piece lies in a hole of another piece (not bridged in Lettering mode, spec issue 6).
    pub(crate) in_hole: bool,
}

impl Piece {
    /// The outer contour.
    pub(crate) fn outer(&self) -> &Path64 {
        &self.contours[0]
    }

    /// The holes.
    pub(crate) fn holes(&self) -> &[Path64] {
        &self.contours[1..]
    }
}

/// The smallest vertex (x, then y) of a contour.
fn smallest(path: &Path64) -> (i64, i64) {
    path.iter().map(|p| (p.x, p.y)).min().unwrap_or((0, 0))
}

/// The pieces of the normalized `union`, sorted by the smallest point of their outer contour.
pub(crate) fn pieces_of(union: &Paths64) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = normalize_shapes(union)
        .into_iter()
        .filter(|shape| !shape.is_empty())
        .map(|mut contours| {
            contours[1..].sort_by_key(smallest);
            let outer = vec![contours[0].clone()];
            Piece {
                double_area: contours.iter().map(|c| double_area(c)).sum(),
                bounds: GridBox::of(&outer),
                contours,
                in_hole: false,
            }
        })
        .collect();
    pieces.sort_by_key(|piece| smallest(piece.outer()));
    let probes: Vec<P> = pieces.iter().map(|p| P::of(p.outer()[0])).collect();
    for (index, probe) in probes.iter().enumerate() {
        pieces[index].in_hole = pieces.iter().enumerate().any(|(other, piece)| {
            other != index
                && winding(piece.outer(), *probe) != 0
                && piece.holes().iter().any(|hole| winding(hole, *probe) != 0)
        });
    }
    pieces
}

/// Which pieces are dots: a piece whose area is below a quarter of the area of the largest piece
/// within Reach of it (spec issue 2). `near` lists the pairs within Reach.
pub(crate) fn dots(pieces: &[Piece], near: &[(usize, usize)]) -> Vec<bool> {
    let mut largest = vec![0_i128; pieces.len()];
    for &(a, b) in near {
        largest[a] = largest[a].max(pieces[b].double_area);
        largest[b] = largest[b].max(pieces[a].double_area);
    }
    pieces
        .iter()
        .zip(largest)
        .map(|(piece, largest)| piece.double_area.saturating_mul(4) < largest)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::{Point64, normalize};

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Path64 {
        vec![
            Point64::new(x0, y0),
            Point64::new(x1, y0),
            Point64::new(x1, y1),
            Point64::new(x0, y1),
        ]
    }

    #[test]
    fn pieces_are_indexed_by_their_smallest_point_and_a_piece_in_a_hole_is_marked() {
        let mut hole = rect(10_000, 10_000, 30_000, 30_000);
        hole.reverse();
        let union = normalize(&vec![
            rect(50_000, 0, 51_000, 1_000),
            rect(0, 0, 40_000, 40_000),
            hole,
            rect(15_000, 15_000, 20_000, 20_000),
        ]);
        let pieces = pieces_of(&union);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].holes().len(), 1);
        assert!(pieces[1].in_hole);
        assert!(!pieces[0].in_hole && !pieces[2].in_hole);
        assert_eq!(pieces[2].outer().iter().map(|p| p.x).min(), Some(50_000));
    }

    /// Spec issue 2: a lone ü has dots, measured against the largest piece within Reach.
    #[test]
    fn a_dot_is_small_against_the_largest_piece_within_reach() {
        let union = normalize(&vec![
            rect(0, 0, 1_000, 1_000),
            rect(3_000, 0, 4_000, 1_000),
            rect(0, 3_000, 4_000, 8_000),
        ]);
        let pieces = pieces_of(&union);
        let near = [(0, 1), (0, 2), (1, 2)];
        assert_eq!(dots(&pieces, &near), vec![true, false, true]);
        assert_eq!(dots(&pieces, &[(0, 1)]), vec![true, false, false]);
        assert_eq!(dots(&pieces, &[(0, 2)]), vec![false, false, false]);
    }
}
