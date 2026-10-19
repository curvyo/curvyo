//! Fracture's kernel: the cells of a stack of operands, each connected area covered by the same
//! set of operands, with the topmost of that set as its owner
//! (`specs/0037-fracture-and-flatten` criteria 6 to 8).
//!
//! An incremental fold on the grid, not a planar arrangement (`adrs.md`): every new operand
//! splits each cell it overlaps in area into the part inside and the part outside it, and what
//! is left of it outside the union of the operands before it becomes a cell of its own.
//! Cleanup and the canonical form run once per cell at the end.

use curvyo_document_core::{Point, Tolerance};
use i_overlay::core::overlay_rule::OverlayRule;

use crate::Outline;
use crate::boolean::operand_regions;
use crate::boolean_grid::{Paths64, run};
use crate::region_contact::{Contact, EdgeIndex};
use crate::stack_regions::{GridBox, StackError, connected_pieces, final_outlines, has_area};

/// The result of [`fracture_cells`].
#[derive(Debug, Clone, PartialEq)]
pub struct Fracture {
    /// One entry per piece: the index of its owner and its outlines in the canonical form of the
    /// boolean kernel, outer outline first. Sorted by owner, then by the first node of the outer
    /// outline (x, then y).
    pub pieces: Vec<(usize, Vec<Vec<Point>>)>,
    /// The operands that overlap no other operand in area, in order; they have no piece.
    pub untouched: Vec<usize>,
}

/// A part of the plane covered by exactly the operands of `covering` (ascending).
struct Cell {
    covering: Vec<usize>,
    region: Paths64,
    bounds: GridBox,
}

impl Cell {
    fn new(covering: Vec<usize>, region: Paths64) -> Self {
        let bounds = GridBox::of(&region);
        Self {
            covering,
            region,
            bounds,
        }
    }
}

/// The pieces of the stack `operands` (stacking order, bottom first; outlines painted with the
/// nonzero rule, as [`crate::boolean`] takes them, with the same `tolerance`). Every piece of an
/// operand that overlaps no other one is left out, and the operand is listed as untouched.
///
/// # Errors
/// [`StackError::Operands`] for the operand checks of [`crate::boolean`];
/// [`StackError::TooManyPieces`] as soon as more than `max_pieces` cells
/// would become pieces.
pub fn fracture_cells(
    operands: &[&[Outline<'_>]],
    tolerance: Tolerance,
    max_pieces: usize,
) -> Result<Fracture, StackError> {
    let regions = operand_regions(operands, tolerance)?;
    let mut fold = Fold::new(regions.len());
    for (index, region) in regions.into_iter().enumerate() {
        fold.add(index, region);
        if fold.counted() > max_pieces {
            return Err(StackError::TooManyPieces);
        }
    }
    fold.into_fracture(max_pieces)
}

/// The state of the fold: the cells so far, which operands overlap another, and the union of
/// the operands so far with its box.
struct Fold {
    cells: Vec<Cell>,
    touched: Vec<bool>,
    union: Paths64,
    union_box: GridBox,
}

impl Fold {
    fn new(operands: usize) -> Self {
        let union = Paths64::new();
        let union_box = GridBox::of(&union);
        Self {
            cells: Vec::new(),
            touched: vec![false; operands],
            union,
            union_box,
        }
    }

    /// Splits every cell the operand `index` overlaps in area and adds what is left of the
    /// operand outside the union as a cell of its own. A cell whose boundary does not meet the
    /// operand's is placed by [`EdgeIndex::contact`] without an overlay, so nested shapes,
    /// whose boxes all meet, stay cheap.
    fn add(&mut self, index: usize, region: Paths64) {
        let bounds = GridBox::of(&region);
        let edges = EdgeIndex::new(&region, bounds);
        let mut inside: Vec<Cell> = Vec::new();
        for cell in self
            .cells
            .iter_mut()
            .filter(|cell| cell.bounds.meets(bounds))
        {
            let (common, outside) = match edges.contact(&cell.region, cell.bounds) {
                Contact::Apart => continue,
                Contact::Inside => (std::mem::take(&mut cell.region), Paths64::new()),
                Contact::Unknown => {
                    let common = run(OverlayRule::Intersect, &cell.region, &region);
                    if !has_area(&common) {
                        continue;
                    }
                    let outside = run(OverlayRule::Difference, &cell.region, &region);
                    (common, outside)
                }
            };
            self.touched[index] = true;
            for &member in &cell.covering {
                self.touched[member] = true;
            }
            let mut covering = cell.covering.clone();
            covering.push(index);
            inside.push(Cell::new(covering, common));
            *cell = Cell::new(std::mem::take(&mut cell.covering), outside);
        }
        let overlaps = bounds.meets(self.union_box);
        let rest = if overlaps {
            run(OverlayRule::Difference, &region, &self.union)
        } else {
            region.clone()
        };
        self.union = if self.union.is_empty() {
            region
        } else {
            run(OverlayRule::Union, &self.union, &region)
        };
        self.union_box = self.union_box.union(bounds);
        self.cells.retain(|cell| !cell.region.is_empty());
        self.cells.extend(inside);
        if !rest.is_empty() {
            self.cells.push(Cell::new(vec![index], rest));
        }
    }

    /// How many cells would become pieces now: those of more than one operand or of an operand
    /// that overlaps another.
    fn counted(&self) -> usize {
        self.cells
            .iter()
            // invariant: a cell is made with at least one covering operand.
            .filter(|cell| cell.covering.len() > 1 || self.touched[cell.covering[0]])
            .count()
    }

    /// The pieces of the touched cells, each cell cut into connected parts, in the order of
    /// [`Fracture::pieces`].
    fn into_fracture(self, max_pieces: usize) -> Result<Fracture, StackError> {
        let mut pieces: Vec<(usize, Vec<Vec<Point>>)> = Vec::new();
        for cell in &self.cells {
            // invariant: a cell is made with at least one covering operand.
            let Some(&owner) = cell.covering.last() else {
                continue;
            };
            if !self.touched[owner] {
                continue;
            }
            for piece in connected_pieces(final_outlines(&cell.region)) {
                pieces.push((owner, piece));
            }
        }
        if pieces.len() > max_pieces {
            return Err(StackError::TooManyPieces);
        }
        pieces.sort_by(|a, b| {
            let start = |piece: &(usize, Vec<Vec<Point>>)| piece.1[0][0];
            a.0.cmp(&b.0)
                .then(start(a).x.total_cmp(&start(b).x))
                .then(start(a).y.total_cmp(&start(b).y))
        });
        let untouched = (0..self.touched.len())
            .filter(|&i| !self.touched[i])
            .collect();
        Ok(Fracture { pieces, untouched })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visible_region::tests::disc;
    use crate::{OutlineTriple, signed_area_mm2};
    use curvyo_document_core::Vec2;

    const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

    fn square(x: f64, y: f64, side: f64) -> Vec<OutlineTriple> {
        [(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
            .iter()
            .map(|&(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
            .collect()
    }

    fn fracture_stack(
        shapes: &[Vec<OutlineTriple>],
        max_pieces: usize,
    ) -> Result<Fracture, StackError> {
        let outlines: Vec<[Outline<'_>; 1]> = shapes
            .iter()
            .map(|anchors| [Outline::new(anchors, true)])
            .collect();
        let operands: Vec<&[Outline<'_>]> = outlines.iter().map(<[_; 1]>::as_slice).collect();
        fracture_cells(&operands, TOLERANCE, max_pieces)
    }

    fn area(outlines: &[Vec<Point>]) -> f64 {
        outlines
            .iter()
            .map(|outline| signed_area_mm2(outline))
            .sum()
    }

    /// Criterion 7: two offset squares give the two Ls and the overlap, each owned by the
    /// topmost square covering it.
    #[test]
    fn two_offset_squares_give_two_ls_and_the_overlap() {
        let result =
            fracture_stack(&[square(0.0, 0.0, 20.0), square(10.0, 10.0, 20.0)], 5_000).unwrap();
        assert_eq!(result.untouched, Vec::<usize>::new());
        let summary: Vec<(usize, usize, i64)> = result
            .pieces
            .iter()
            .map(|(owner, outlines)| {
                #[allow(clippy::cast_possible_truncation)]
                let rounded = area(outlines).round() as i64;
                (*owner, outlines[0].len(), rounded)
            })
            .collect();
        assert_eq!(summary, vec![(0, 6, 300), (1, 4, 100), (1, 6, 300)]);
    }

    /// Criterion 7: concentric discs give a ring (outer outline and hole) and the inner disc;
    /// with the small disc below, both pieces belong to the large one.
    #[test]
    fn concentric_discs_give_a_ring_and_a_disc() {
        let pi = std::f64::consts::PI;
        let on_top = fracture_stack(&[disc(20.0), disc(10.0)], 5_000).unwrap();
        let owners: Vec<usize> = on_top.pieces.iter().map(|piece| piece.0).collect();
        assert_eq!(owners, vec![0, 1]);
        let ring = &on_top.pieces[0].1;
        assert_eq!(ring.len(), 2);
        assert!((area(ring) - pi * 300.0).abs() < 0.2, "{}", area(ring));
        let below = fracture_stack(&[disc(10.0), disc(20.0)], 5_000).unwrap();
        assert!(below.pieces.iter().all(|piece| piece.0 == 1));
        assert_eq!(below.pieces.len(), 2);
    }

    /// Criterion 7: a bar across both arms of a U overlaps it twice: two overlap pieces.
    #[test]
    fn a_bar_across_a_u_gives_two_separate_overlap_pieces() {
        let u: Vec<OutlineTriple> = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 20.0),
            (20.0, 20.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (30.0, 30.0),
            (0.0, 30.0),
        ]
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect();
        let bar: Vec<OutlineTriple> = [(-5.0, 5.0), (35.0, 5.0), (35.0, 10.0), (-5.0, 10.0)]
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect();
        let result = fracture_stack(&[u, bar], 5_000).unwrap();
        let overlaps = result
            .pieces
            .iter()
            .filter(|(owner, outlines)| {
                // The overlaps start at the arms' left edges; the bar alone between them at 10.
                let x = outlines[0][0].x;
                *owner == 1
                    && (x.abs() < 1e-9 || (x - 20.0).abs() < 1e-9)
                    && (area(outlines) - 50.0).abs() < 1e-6
            })
            .count();
        assert_eq!(overlaps, 2);
    }

    /// Criteria 3, 4 and 6: an operand that overlaps nothing is untouched; the limit refuses.
    #[test]
    fn an_operand_apart_is_untouched_and_the_limit_refuses() {
        let shapes = [
            square(0.0, 0.0, 20.0),
            square(10.0, 10.0, 20.0),
            square(100.0, 0.0, 5.0),
        ];
        let result = fracture_stack(&shapes, 5_000).unwrap();
        assert_eq!(result.untouched, vec![2]);
        assert_eq!(result.pieces.len(), 3);
        assert_eq!(fracture_stack(&shapes, 2), Err(StackError::TooManyPieces));
    }
}
