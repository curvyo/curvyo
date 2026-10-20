//! Weld's kernel (`specs/0068-laserable-lettering`, `adrs.md` decisions 1, 2 and 5): one set of
//! lines in which every boundary of a visible region exists once.
//!
//! Flatten's fold from the top (`specs/0037-fracture-and-flatten`) on the 0.001 mm grid, with
//! one change: each operand's **boundary** is clipped by the running union of the operands above
//! it as an open string (`i_overlay`'s string clip in the same integer engine), keeping what lies
//! strictly outside. A weld point is so emitted once, by the topmost operand whose boundary holds
//! it. One more string clip against the final union `U` keeps the pieces strictly inside `U`: the
//! inner lines. `U`'s own boundary gives the closed outlines, exactly as Union makes them.

use curvyo_document_core::{Point, Tolerance};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::string::clip::ClipRule;
use i_overlay::string::overlay::StringOverlay;

use crate::Outline;
use crate::boolean::{BooleanError, operand_regions};
use crate::boolean_cleanup::{cleanup, distance_squared_to_segment};
use crate::boolean_grid::{Path64, Paths64, Point64, canonical_millimetres, run};
use crate::stack_regions::{GridBox, has_area};
use crate::weld_lines::chain_lines;

/// Keeps the parts of a string strictly outside the shape: neither inside nor on its boundary.
/// (`i_overlay` 9.0.1 reads `boundary_included` together with `invert`: with `invert`, "included"
/// means the boundary counts as inside the clipped-away area.)
const STRICTLY_OUTSIDE: ClipRule = ClipRule {
    invert: true,
    boundary_included: true,
};

/// Keeps the parts of a string strictly inside the shape, its boundary excluded.
const STRICTLY_INSIDE: ClipRule = ClipRule {
    invert: false,
    boundary_included: false,
};

/// The result of [`weld`].
#[derive(Debug, Clone, PartialEq)]
pub struct Welded {
    /// The lines as `(points, closed)`, in millimetres on the 0.001 mm grid: the closed outlines
    /// of the union of all regions in the canonical form of [`crate::BooleanResult`], then the
    /// closed inner outlines (wound with a positive area, starting at their smallest point) by
    /// first node, then the open inner outlines from their smaller end by first and second node.
    pub outlines: Vec<(Vec<Point>, bool)>,
    /// The operands covered completely by the operands above them, by index, bottom first.
    pub hidden: Vec<usize>,
}

/// Welds `operands` (stacking order, bottom first; each a list of outlines painted with the
/// nonzero rule, as [`crate::boolean`] takes them, with the same `tolerance`).
///
/// # Errors
/// The operand checks of [`crate::boolean`] with the same offenders, or
/// [`BooleanError::EmptyResult`] when nothing is left after cleanup.
pub fn weld(operands: &[&[Outline<'_>]], tolerance: Tolerance) -> Result<Welded, BooleanError> {
    let regions = operand_regions(operands, tolerance)?;
    // The union of the operands above the current one, as its connected shapes (an outer
    // outline with its holes) and their boxes: a step overlays only the shapes it touches.
    let mut above: Vec<Shape> = Vec::new();
    let mut kept: Vec<Path64> = Vec::new();
    let mut hidden = Vec::new();
    for (index, region) in regions.iter().enumerate().rev() {
        let region_box = GridBox::of(region);
        let (near, apart): (Vec<Shape>, Vec<Shape>) = above
            .into_iter()
            .partition(|shape| shape.bounds.touches(region_box));
        let near_contours: Paths64 = near
            .iter()
            .flat_map(|s| s.contours.iter().cloned())
            .collect();
        let pieces = if near.is_empty() {
            region.iter().map(closed_string).collect()
        } else {
            outside(region, &near_contours)
        };
        if pieces.is_empty() && !has_area(&run(OverlayRule::Difference, region, &near_contours)) {
            hidden.push(index);
        }
        kept.extend(pieces);
        above = apart;
        above.extend(union_shapes(&near_contours, region));
    }
    let above: Paths64 = above.into_iter().flat_map(|shape| shape.contours).collect();
    hidden.reverse();
    let boundary = cleanup(&above);
    let union = canonical_millimetres(&boundary);
    if union.is_empty() {
        return Err(BooleanError::EmptyResult);
    }
    let mut outlines: Vec<(Vec<Point>, bool)> =
        union.into_iter().map(|outline| (outline, true)).collect();
    let kept = noded(&kept, &above);
    outlines.extend(chain_lines(
        &noded(&clip(&kept, &above, STRICTLY_INSIDE), &boundary),
        &boundary,
    ));
    Ok(Welded { outlines, hidden })
}

/// A closed contour as an open string that returns to its start.
fn closed_string(contour: &Path64) -> Path64 {
    let mut string = contour.clone();
    if let Some(&first) = contour.first() {
        string.push(first);
    }
    string
}

/// One connected shape of the running union: an outer outline with its holes, and its box.
struct Shape {
    contours: Paths64,
    bounds: GridBox,
}

/// The union of `near` (the running union's shapes that touch the region) and `region`, as
/// shapes. Shapes whose box misses the region's do not take part: they cannot meet it.
fn union_shapes(near: &Paths64, region: &Paths64) -> Vec<Shape> {
    let capacity = near.iter().chain(region).map(Vec::len).sum();
    let mut overlay = Overlay::<i64>::new(capacity);
    for contour in near {
        overlay.add_contour(contour, ShapeType::Subject);
    }
    for contour in region {
        overlay.add_contour(contour, ShapeType::Clip);
    }
    overlay
        .overlay(OverlayRule::Union, FillRule::NonZero)
        .into_iter()
        .map(|contours| Shape {
            bounds: GridBox::of(&contours),
            contours,
        })
        .collect()
}

/// The parts of `region`'s boundary strictly outside `near` (the running union's shapes whose
/// box touches the region's; a shape that misses it has winding 0 there).
fn outside(region: &Paths64, near: &Paths64) -> Vec<Path64> {
    let strings: Vec<Path64> = region.iter().map(closed_string).collect();
    clip(&noded(&strings, near), near, STRICTLY_OUTSIDE)
}

/// `strings` with every vertex of `shape` that lies within one grid unit of a segment's interior
/// inserted into that segment, in order along it (`adrs.md` decision 3, "Known limit", dated note
/// of 2026-10-20). A crossing that an overlay rounded to the grid bends the shape's edge by up to
/// a unit off the line it came from; the string that runs along that line then lies a fraction of
/// a unit inside or outside the shape and would survive as a near-parallel double. With the
/// vertex inserted it runs exactly on the shape's edge. A string moves by at most one unit.
fn noded(strings: &[Path64], shape: &Paths64) -> Vec<Path64> {
    let mut vertices: Vec<Point64> = shape.iter().flatten().copied().collect();
    vertices.sort_unstable_by_key(|p| (p.x, p.y));
    vertices.dedup();
    strings
        .iter()
        .map(|string| {
            let mut out = Path64::with_capacity(string.len());
            for pair in string.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                out.push(a);
                out.extend(near_vertices(&vertices, a, b));
            }
            out.extend(string.last());
            out
        })
        .collect()
}

/// How far, in grid units, a vertex of the shape may lie from the original segment and still be
/// considered for insertion. Overlay steps round each crossing anew, so a vertex can drift a few
/// units from the line its edge came from; each inserted vertex is checked against the rest of
/// the segment from the vertex inserted before it, which follows that drift one unit at a time.
const DRIFT_UNITS_SQUARED: f64 = 16.0;

/// The vertices of `sorted` (by x, then y) along the interior of `a b`, in order from `a` to `b`:
/// each within one grid unit of the segment from the vertex inserted before it (from `a` for the
/// first) to `b`.
fn near_vertices(sorted: &[Point64], a: Point64, b: Point64) -> Vec<Point64> {
    let (abx, aby) = (i128::from(b.x - a.x), i128::from(b.y - a.y));
    let length_squared = abx * abx + aby * aby;
    let from = sorted.partition_point(|p| p.x < a.x.min(b.x) - 4);
    let mut candidates: Vec<(i128, Point64)> = sorted[from..]
        .iter()
        .take_while(|p| p.x <= a.x.max(b.x) + 4)
        .filter(|p| p.y >= a.y.min(b.y) - 4 && p.y <= a.y.max(b.y) + 4)
        .map(|&p| (i128::from(p.x - a.x) * abx + i128::from(p.y - a.y) * aby, p))
        .filter(|&(along, p)| {
            along > 0
                && along < length_squared
                && distance_squared_to_segment(p, a, b) <= DRIFT_UNITS_SQUARED
        })
        .collect();
    candidates.sort_unstable_by_key(|&(along, p)| (along, p.x, p.y));
    let mut inserted = Vec::new();
    let mut start = a;
    for (_, p) in candidates {
        if p != start && p != b && distance_squared_to_segment(p, start, b) <= 1.0 {
            inserted.push(p);
            start = p;
        }
    }
    inserted
}

/// The parts of `strings` that `rule` keeps against the nonzero region of `shape`.
fn clip(strings: &[Path64], shape: &Paths64, rule: ClipRule) -> Vec<Path64> {
    if strings.is_empty() {
        return Vec::new();
    }
    let capacity = strings.iter().chain(shape).map(Vec::len).sum();
    let mut overlay = StringOverlay::<i64>::new(capacity);
    for contour in shape {
        overlay.add_shape_contour(contour);
    }
    for string in strings {
        overlay.add_string_path(string);
    }
    overlay.clip_string_lines(FillRule::NonZero, rule)
}

#[cfg(test)]
#[path = "weld_tests.rs"]
mod tests;
