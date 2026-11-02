//! Cutting closed pieces and strips at the region (`specs/0071-pattern-fill` criteria 13 to 16):
//! overlays on the 0.001 mm grid with `i_overlay`'s integer engine, so the result is the same on
//! every target.

use curvyo_document_core::Point;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{IntOverlayOptions, Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::core::solver::Solver;
use i_overlay::mesh::int::stroke::offset::IntStrokeOffset;
use i_overlay::mesh::int::style::{IntLineCap, IntLineJoin, IntStrokeStyle};
use i_overlay::mesh::math::MathMode;

use crate::OutlineTriple;
use crate::boolean_grid::{Path64, Paths64, snap};
use crate::flatten::flatten;

/// An outline flattened and snapped to the grid, consecutive duplicates dropped; a closed one does
/// not repeat its first point.
pub(crate) fn grid_points(anchors: &[OutlineTriple], closed: bool, tolerance_mm: f64) -> Path64 {
    let mut path: Path64 = Vec::new();
    for point in flatten(anchors, closed, tolerance_mm) {
        let snapped = snap(point);
        if path.last() != Some(&snapped) {
            path.push(snapped);
        }
    }
    if closed {
        while path.len() > 1 && path.first() == path.last() {
            path.pop();
        }
    }
    path
}

/// An open line flattened, on the grid and exact: the points whose grid point repeats the one
/// before are dropped from both.
pub(crate) fn grid_line(anchors: &[OutlineTriple], tolerance_mm: f64) -> (Path64, Vec<Point>) {
    let mut grid: Path64 = Vec::new();
    let mut exact = Vec::new();
    for point in flatten(anchors, false, tolerance_mm) {
        let snapped = snap(point);
        if grid.last() != Some(&snapped) {
            grid.push(snapped);
            exact.push(point);
        }
    }
    (grid, exact)
}

/// The shapes (an outer contour with its holes) of `piece` (nonzero) within `region`.
pub(crate) fn intersect(piece: &Paths64, region: &Paths64) -> Vec<Paths64> {
    let (piece_edges, region_edges) = (edges(piece), edges(region));
    let mut overlay = Overlay::<i64>::new_custom(
        piece_edges + region_edges,
        IntOverlayOptions::default(),
        solver(piece_edges, region_edges),
    );
    for contour in piece {
        overlay.add_contour(contour, ShapeType::Subject);
    }
    for contour in region {
        overlay.add_contour(contour, ShapeType::Clip);
    }
    overlay.overlay(OverlayRule::Intersect, FillRule::NonZero)
}

fn edges(paths: &Paths64) -> usize {
    paths.iter().map(Vec::len).sum()
}

/// How an overlay of `piece_edges` against `region_edges` finds its crossings. `i_overlay`'s
/// default compares every pair of edges below 4,000 edges and keeps a tree above; both are slow
/// when many long edges overlap, as along a needle outline. Splitting the edges into fragments
/// first is faster above 4,000 edges and for a small piece against a region of many edges, slower
/// for a few long strips against a smooth region (`plan.md`, "Measurements").
fn solver(piece_edges: usize, region_edges: usize) -> Solver {
    if piece_edges + region_edges >= 4_000 || (piece_edges <= 64 && region_edges >= 128) {
        Solver::FRAG
    } else {
        Solver::AUTO
    }
}

/// The union of `contours` (nonzero), as shapes.
pub(crate) fn union(contours: &Paths64) -> Vec<Paths64> {
    let mut overlay = Overlay::<i64>::new(contours.iter().map(Vec::len).sum());
    for contour in contours {
        overlay.add_contour(contour, ShapeType::Subject);
    }
    overlay.overlay(OverlayRule::Subject, FillRule::NonZero)
}

/// The union of `strips` (nonzero) within `region`, as shapes. Strips that cross on the grid
/// (`rounded` false: lines along the axes) are first united, then intersected with the region,
/// which is several times faster for them than one overlay; strips whose crossings are rounded
/// are cut in one overlay that splits long edges into fragments first, about twice as fast as
/// the two (`plan.md`, "Measurements").
pub(crate) fn merge(strips: &Paths64, region: &Paths64, rounded: bool) -> Vec<Paths64> {
    if rounded {
        let capacity = strips.iter().chain(region).map(Vec::len).sum();
        let mut overlay =
            Overlay::<i64>::new_custom(capacity, IntOverlayOptions::default(), Solver::FRAG);
        for contour in strips {
            overlay.add_contour(contour, ShapeType::Subject);
        }
        for contour in region {
            overlay.add_contour(contour, ShapeType::Clip);
        }
        return overlay.overlay(OverlayRule::Intersect, FillRule::NonZero);
    }
    let merged: Paths64 = union(strips).into_iter().flatten().collect();
    intersect(&merged, region)
}

/// The closed strip `width_units` wide along `string`, with flat ends and bevelled joins, as
/// contours: a line of Shapes mode (`plan.md`, "Decisions").
pub(crate) fn strip(string: &Path64, width_units: i64) -> Paths64 {
    let style = IntStrokeStyle::new(width_units)
        .start_cap(IntLineCap::Butt)
        .end_cap(IntLineCap::Butt)
        .line_join(IntLineJoin::Bevel)
        .math(MathMode::Integer);
    if string.validate_stroke(&style).is_err() {
        return Vec::new();
    }
    string
        .stroke(&style, false)
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect()
}

/// How much folding the strip of `string` has to resolve: per segment shorter than the strip is
/// wide, the width over its length (at most 16), and per turn sharper than a right angle, 128
/// (measured: a turn costs the stroke about a hundred times a short segment).
/// Tight bends and sharp turns fold the strip's sides over each other; this is what the stroke's
/// work grows with besides the points.
pub(crate) fn folds(string: &Path64, width_units: i64) -> u64 {
    let width2 = i128::from(width_units) * i128::from(width_units);
    let short: u64 = string
        .windows(2)
        .map(|w| {
            let (dx, dy) = (i128::from(w[1].x - w[0].x), i128::from(w[1].y - w[0].y));
            let ratio2 = (width2 / (dx * dx + dy * dy).max(1)).min(256);
            // The whole square root of `ratio2`, at most 16.
            (1..=16_u64)
                .take_while(|k| i128::from(k * k) <= ratio2)
                .count() as u64
        })
        .sum();
    let sharp = string
        .windows(3)
        .filter(|w| {
            let (ax, ay) = (i128::from(w[1].x - w[0].x), i128::from(w[1].y - w[0].y));
            let (bx, by) = (i128::from(w[2].x - w[1].x), i128::from(w[2].y - w[1].y));
            ax * bx + ay * by < 0
        })
        .count() as u64;
    short + 128 * sharp
}

/// A grid point back in millimetres, as a corner anchor.
pub(crate) fn corner(point: Point) -> OutlineTriple {
    (
        point,
        curvyo_document_core::Vec2::ZERO,
        curvyo_document_core::Vec2::ZERO,
    )
}
