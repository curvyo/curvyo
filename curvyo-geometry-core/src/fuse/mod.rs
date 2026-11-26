//! Bridge's kernel (`specs/0156-fused-lettering`, `adrs.md` decisions 1 to 4): the entry points
//! that add bridges to lettering on the 0.001 mm grid.
//!
//! **Lettering** (`fuse_cut_out`): the union of the operands falls into pieces; dots go to their
//! nearest non-dot, the rest are joined by a minimum spanning forest of their narrowest gaps
//! within Reach; each bridge is a band of the width, blended into both pieces. The Outline is the
//! Union of the pieces and the bridges.
//!
//! **Stencil** (`fuse_stencil`): the union is the area cut away; every hole of it is an island,
//! held to the sheet by bridges across the stroke. The Outline is the union minus the bridges.

mod axis;
mod blend;
mod forest;
mod gap;
mod islands;
mod measure;
mod pieces;

use curvyo_document_core::{Length, Point, Tolerance};
use i_overlay::core::overlay_rule::OverlayRule;

use crate::Outline;
use crate::boolean::{BooleanError, operand_regions};
use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{GRID_MM, Paths64, canonical_millimetres, normalize, run};
use crate::stack_regions::{GridBox, has_area};
use forest::Edge;
use pieces::{Piece, dots, pieces_of};

/// What Bridge made, in millimetres on the 0.001 mm grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Fused {
    /// The line the laser cuts: closed outlines in the canonical form of
    /// [`crate::BooleanResult`] (outer outlines positive, holes negative).
    pub outline: Vec<Vec<Point>>,
    /// One entry per bridge, ordered by its first point (x, then y); an entry is usually one
    /// closed outline and may be several.
    pub connectors: Vec<Vec<Vec<Point>>>,
}

/// Why Bridge was refused. Nothing is computed or changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FuseError {
    /// The operand checks of the boolean operations refused, with the same offenders.
    #[error(transparent)]
    Operands(#[from] BooleanError),
    /// No two pieces lie within Reach of each other, or there is one piece.
    #[error("nothing to join")]
    NothingToJoin,
    /// More bridges than the caller's limit.
    #[error("more bridges than the limit")]
    TooManyBridges,
    /// Stencil: the lettering encloses no island.
    #[error("no islands")]
    NoIslands,
}

/// Millimetres as grid units, as floating point.
fn units(length: Length) -> f64 {
    length.as_mm() / GRID_MM
}

/// Share of the gap a fillet's radius may take: a closing of radius `r` fills every gap narrower
/// than `2 r`, so the radius stays below half the gap and only the bridge's junctions are filled.
const BLEND_GAP_SHARE: f64 = 0.45;

/// The fillet radius of a bridge across `gap` grid units: the width (spec Words, "Blend"), at
/// most [`BLEND_GAP_SHARE`] of the gap, in whole grid units.
fn blend_radius(width: f64, gap: f64) -> i64 {
    // At most 10 mm in grid units, so the cast is exact.
    #[allow(clippy::cast_possible_truncation)]
    let radius = width.min(BLEND_GAP_SHARE * gap).round() as i64;
    radius.max(1)
}

/// The normalized union of the operands' regions.
fn union_of(operands: &[&[Outline<'_>]], tolerance: Tolerance) -> Result<Paths64, BooleanError> {
    let regions = operand_regions(operands, tolerance)?;
    let all: Paths64 = regions.into_iter().flatten().collect();
    Ok(normalize(&all))
}

/// Every piece's contours whose box meets `bounds`, as one set.
fn near_pieces(pieces: &[Piece], bounds: GridBox) -> Paths64 {
    pieces
        .iter()
        .filter(|piece| piece.bounds.touches(bounds))
        .flat_map(|piece| piece.contours.iter().cloned())
        .collect()
}

/// The canonical form of each bridge, ordered by its first point.
fn canonical_bridges(bridges: &[Paths64]) -> Vec<Vec<Vec<Point>>> {
    let mut out: Vec<Vec<Vec<Point>>> = bridges
        .iter()
        .map(|bridge| canonical_millimetres(&cleanup(bridge)))
        .filter(|outlines| !outlines.is_empty())
        .collect();
    out.sort_by(|a, b| {
        let first = |o: &Vec<Vec<Point>>| (o[0][0].x, o[0][0].y);
        let (fa, fb) = (first(a), first(b));
        fa.0.total_cmp(&fb.0).then_with(|| fa.1.total_cmp(&fb.1))
    });
    out
}

/// The pairs of pieces within `reach` grid units of each other (pieces in a hole take no part),
/// with their narrowest gaps.
fn candidate_edges(pieces: &[Piece], reach: f64, tie: f64) -> Vec<Edge> {
    // Saturating: an unbounded Reach (Join all) grows every box to the whole grid.
    #[allow(clippy::cast_possible_truncation)]
    let grow = reach.min(1.0e12).ceil() as i64;
    let mut edges = Vec::new();
    for a in 0..pieces.len() {
        if pieces[a].in_hole {
            continue;
        }
        let reach_box = pieces[a].bounds.grown(grow);
        for b in a + 1..pieces.len() {
            if pieces[b].in_hole || !reach_box.touches(pieces[b].bounds) {
                continue;
            }
            let gap = gap::narrowest_gap(pieces[a].outer(), pieces[b].outer(), tie);
            if gap.distance <= reach {
                edges.push(Edge { a, b, gap });
            }
        }
    }
    edges
}

/// Adds bridges to the cut-out lettering `operands` (stacking order, bottom first; each a list
/// of outlines painted with the nonzero rule, as [`crate::boolean`] takes them).
///
/// `width` is each bridge's neck; `reach` the largest gap that is bridged, `None` joins every
/// piece (Join all); `max_bridges` the caller's limit. Pieces inside a hole of another piece stay
/// apart and in the result.
///
/// # Errors
/// [`FuseError::Operands`] with the checks of [`crate::boolean`];
/// [`FuseError::NothingToJoin`] when no bridge is made; [`FuseError::TooManyBridges`] beyond
/// `max_bridges`.
pub fn fuse_cut_out(
    operands: &[&[Outline<'_>]],
    width: Length,
    reach: Option<Length>,
    max_bridges: usize,
    tolerance: Tolerance,
) -> Result<Fused, FuseError> {
    let union = union_of(operands, tolerance)?;
    let pieces = pieces_of(&union);
    let tie = tolerance.as_mm() / GRID_MM;
    let reach = reach.map_or(f64::INFINITY, units);
    let width_units = units(width);
    let edges = candidate_edges(&pieces, reach, tie);
    let near: Vec<(usize, usize)> = edges.iter().map(|e| (e.a, e.b)).collect();
    let dot = dots(&pieces, &near);
    let band_of = |edge: &Edge| {
        axis::band(
            &edge.gap,
            width_units,
            &pieces[edge.a].contours,
            &pieces[edge.b].contours,
        )
    };
    let taken = forest::choose(pieces.len(), &dot, edges, |edge| {
        let band = band_of(edge);
        let others: Paths64 = pieces
            .iter()
            .enumerate()
            .filter(|&(i, p)| i != edge.a && i != edge.b && p.bounds.meets(GridBox::of(&band)))
            .flat_map(|(_, p)| p.contours.iter().cloned())
            .collect();
        !others.is_empty() && has_area(&run(OverlayRule::Intersect, &band, &others))
    });
    if taken.is_empty() {
        return Err(FuseError::NothingToJoin);
    }
    if taken.len() > max_bridges {
        return Err(FuseError::TooManyBridges);
    }
    let bridges: Vec<Paths64> = taken
        .iter()
        .map(|edge| {
            let band = band_of(edge);
            let radius = blend_radius(width_units, edge.gap.distance);
            let mut joined = pieces[edge.a].contours.clone();
            joined.extend(pieces[edge.b].contours.iter().cloned());
            let reach_box = GridBox::of(&band).grown(radius.saturating_mul(4));
            blend::added(&band, &joined, &near_pieces(&pieces, reach_box), radius)
        })
        .collect();
    let mut all = union;
    all.extend(bridges.iter().flatten().cloned());
    Ok(Fused {
        outline: canonical_millimetres(&cleanup(&normalize(&all))),
        connectors: canonical_bridges(&bridges),
    })
}

/// Adds bridges to the stencil lettering `operands` (read as [`fuse_cut_out`] reads them): the
/// union is the area cut away and each of its holes an island held by `per_island` bridges of
/// `width` (at most a quarter of the island's perimeter).
///
/// # Errors
/// [`FuseError::Operands`] with the checks of [`crate::boolean`]; [`FuseError::NoIslands`] when
/// the union has no hole; [`FuseError::TooManyBridges`] beyond `max_bridges`.
pub fn fuse_stencil(
    operands: &[&[Outline<'_>]],
    width: Length,
    per_island: u8,
    max_bridges: usize,
    tolerance: Tolerance,
) -> Result<Fused, FuseError> {
    let union = union_of(operands, tolerance)?;
    let pieces = pieces_of(&union);
    let tie = tolerance.as_mm() / GRID_MM;
    let width_units = units(width);
    let mut planned: Vec<(islands::Crossing, f64)> = Vec::new();
    let mut any_island = false;
    for piece in &pieces {
        for hole in 0..piece.holes().len() {
            any_island = true;
            let perimeter = islands::perimeter(piece, hole);
            let width = width_units.min(perimeter / 4.0);
            let all = islands::crossings(piece, hole);
            for crossing in islands::choose(&all, usize::from(per_island), perimeter, tie) {
                planned.push((crossing, width));
            }
        }
    }
    if !any_island {
        return Err(FuseError::NoIslands);
    }
    if planned.len() > max_bridges {
        return Err(FuseError::TooManyBridges);
    }
    let bridges: Vec<Paths64> = planned
        .iter()
        .map(|(crossing, width)| {
            let band = vec![islands::band(crossing, *width)];
            let radius = blend_radius(*width, crossing.length);
            let reach_box = GridBox::of(&band).grown(radius.saturating_mul(4));
            blend::standing(&band, &near_pieces(&pieces, reach_box), radius)
        })
        .collect();
    let standing: Paths64 = bridges.iter().flatten().cloned().collect();
    let outline = if standing.is_empty() {
        union
    } else {
        run(OverlayRule::Difference, &union, &standing)
    };
    Ok(Fused {
        outline: canonical_millimetres(&cleanup(&outline)),
        connectors: canonical_bridges(&bridges),
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
