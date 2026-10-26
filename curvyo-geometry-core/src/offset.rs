//! Offsetting one operand's region outward or inward by a signed distance
//! (`specs/0038-path-offset`; ADR 0003 §4 as amended on 2026-10-10 and 2026-10-12).
//!
//! # Pipeline
//!
//! 1. `offset_pieces` builds the exact raw offset of every outline: parallel lines, offset
//!    cubics, true arcs for Round joins, miter and bevel lines.
//! 2. `offset_rebuild` flattens it onto the 0.001 mm grid of the Boolean operations, lets
//!    `i_overlay` (integer engine, Positive fill rule) decide which parts survive, and maps every
//!    surviving ring back onto the exact pieces.
//! 3. The polyline route of the first build (`i_overlay`'s integer offsetter on the flattened
//!    operand) is the fallback: for operands whose outlines cross or touch, and for every ring
//!    that cannot be mapped (criterion 39).
//!
//! The angles handed to the library and the arcs are computed with `+ - * /` and `sqrt` only, and
//! the library runs in its integer mode, so a result is the same on every target (criterion 44).

use curvyo_document_core::{Length, Point, Tolerance, Vec2};
use kurbo::{PathSeg as KSeg, Point as KPoint, Vec2 as KVec};

use crate::boolean::{Outline, grid_paths, outline_in_range};
use crate::boolean_grid::{
    GRID_MM, MAX_COORDINATE_MM, Paths64, Point64, double_area, normalize, snap, to_millimetres,
};
use crate::flatten::flatten;
use crate::offset_finish::{canonical, lines_of_mm};
use crate::offset_pieces::{RawStyle, closed_loop, segments_of};
use crate::offset_polyline::{polyline_offset, without_flattening_noise};
use crate::offset_rebuild::{FillRule, rebuild};
use crate::outline_touch::point_segment_distance;

/// How a corner is treated where the offset outline opens a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetJoin {
    /// An arc of radius |d| around the corner.
    Round,
    /// The two edges extended to their meeting point, clipped at 4 x |d| from the corner.
    Miter,
    /// A straight cut across the gap.
    Bevel,
}

/// Why an offset was refused. Nothing is computed in any of these cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OffsetError {
    /// The tolerance leaves no room for the grid: it must exceed two grid pitches (0.002 mm).
    #[error("tolerance too small for the 0.001 mm grid")]
    ToleranceTooSmall,
    /// The distance rounds to zero on the 0.001 mm grid.
    #[error("the distance is zero")]
    ZeroDistance,
    /// The operand or the result has a coordinate that is not finite or lies beyond 10⁷ mm, or
    /// the distance is not finite or beyond that range.
    #[error("a coordinate is not finite or out of range")]
    OutOfRange,
    /// A closed operand encloses no area.
    #[error("the operand encloses no area")]
    NoArea,
    /// An outline of the operand is open: only closed paths are offset (criterion 55).
    #[error("an open path is not offset")]
    OpenPath,
    /// Nothing is left: the operand vanished under the inset.
    #[error("the result is empty")]
    Empty,
}

/// One node of an offset result: its point, its handles relative to it (zero on a straight
/// side) and whether the pieces meet there with the same tangent (criterion 33).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetAnchor {
    /// The node.
    pub point: Point,
    /// The handle towards the previous node, relative to `point`.
    pub handle_in: Vec2,
    /// The handle towards the next node, relative to `point`.
    pub handle_out: Vec2,
    /// The neighbouring pieces have the same tangent here (within 0.1°).
    pub smooth: bool,
}

/// The result of [`offset`]: closed outlines of lines and cubics.
///
/// - **Winding:** an outer outline has a positive area (as [`signed_area_mm2`] counts it), a hole
///   a negative one; filled with the nonzero rule the outlines paint the result region.
/// - Every outline starts at its smallest node (x, then y); outlines are sorted by that node,
///   ties by area (larger first).
/// - Lines and arcs are exact; cubics follow the exact offset within the tolerance. An outline
///   that could not be rebuilt from exact pieces is a polyline of the 0.001 mm grid (criterion 39).
///
/// [`signed_area_mm2`]: crate::signed_area_mm2
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetOutlines {
    outlines: Vec<Vec<OffsetAnchor>>,
    fallback: usize,
}

impl OffsetOutlines {
    /// Outlines already in canonical order, `fallback` of them polylines.
    pub(crate) const fn from_parts(outlines: Vec<Vec<OffsetAnchor>>, fallback: usize) -> Self {
        Self { outlines, fallback }
    }

    /// The outlines in canonical order. Never empty.
    #[must_use]
    pub fn outlines(&self) -> &[Vec<OffsetAnchor>] {
        &self.outlines
    }

    /// The outlines, by value.
    #[must_use]
    pub fn into_outlines(self) -> Vec<Vec<OffsetAnchor>> {
        self.outlines
    }

    /// How many outlines are polylines because their pieces could not be matched (criterion 39).
    #[must_use]
    pub const fn fallback_outlines(&self) -> usize {
        self.fallback
    }

    /// The number of segments (lines and cubics) over all outlines.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.outlines.iter().map(Vec::len).sum()
    }
}

/// Offsets one operand by `distance`: positive grows its region (Outset), negative shrinks it
/// (Inset).
///
/// `operand` is the operand's outlines, all closed: the region they paint (nonzero) is offset with
/// `join`. An open outline is refused (criterion 55: the stroke does that job).
///
/// The result is built from the exact offset of every piece of the operand (parallel lines,
/// offset cubics, arcs, miter and bevel lines); `i_overlay` decides only which parts survive
/// (ADR 0003 §4 as amended on 2026-10-12). `tolerance` bounds how far the result may lie from the
/// exact offset of the operand's curves. An operand whose outlines cross or touch takes the
/// polyline route of the first build, as does any outline that cannot be matched.
///
/// # Errors
///
/// [`OffsetError`]: checked in the order tolerance, distance (not finite or out of range, then
/// zero), an open outline, operand out of range, closed operand without area; after the
/// offset, an empty result, then a result out of range.
pub fn offset(
    operand: &[Outline<'_>],
    distance: Length,
    join: OffsetJoin,
    tolerance: Tolerance,
) -> Result<OffsetOutlines, OffsetError> {
    let checked = checked_request(operand, distance, tolerance)?;
    let style = RawStyle {
        join,
        curve_tolerance_mm: checked.flatten_mm / 2.0,
        clip_long_miters: true,
    };
    let region = normalize(&grid_paths(operand, checked.flatten_mm));
    if region.is_empty() {
        return Err(OffsetError::NoArea);
    }
    let loops = closed_loops(operand, &region, checked.d, style, checked.flatten_mm);
    let (outlines, fallback) = if let Some(loops) = loops {
        let rebuilt = rebuild(&loops, checked.flatten_mm, tolerance.as_mm(), FillRule::Positive, |rings| {
            let rings = without_flattening_noise(rings, checked.flatten_mm);
            without_inverted_rings(rings, operand, checked.d.abs(), checked.flatten_mm)
        });
        (rebuilt.outlines, rebuilt.fallback)
    } else {
        let polylines = polyline_offset(operand, checked.units, join, checked.flatten_mm)?;
        let count = polylines.len();
        (
            polylines.iter().map(|path| lines_of_mm(path)).collect(),
            count,
        )
    };
    if outlines.is_empty() {
        return Err(OffsetError::Empty);
    }
    let outlines = canonical(&outlines);
    let in_range = |p: &OffsetAnchor| {
        p.point.x.abs() <= MAX_COORDINATE_MM && p.point.y.abs() <= MAX_COORDINATE_MM
    };
    if !outlines.iter().flatten().all(in_range) {
        return Err(OffsetError::OutOfRange);
    }
    Ok(OffsetOutlines { outlines, fallback })
}

/// `rings` without those that lie closer to the operand than the distance.
///
/// Every point of a true offset lies at least `|d|` from the operand's outlines. An outline that
/// shrinks by more than its own size turns its parallel curve inside out without crossing it (a
/// hole of radius 10 grown by 10.5 gives a circle of radius 0.5 wound like the hole), so the
/// overlay keeps it as a spurious hole or island (criterion 28). Such a ring lies nearer than
/// `|d|`: it is dropped when two of three of its vertices, spread over it, do.
fn without_inverted_rings(
    mut rings: Paths64,
    operand: &[Outline<'_>],
    distance_mm: f64,
    flatten_mm: f64,
) -> Paths64 {
    // The ring and the operand are each flattened within `flatten_mm`, the ring also snapped.
    let threshold = distance_mm - 3.0 * flatten_mm - 2.0 * GRID_MM;
    if threshold <= 0.0 {
        return rings;
    }
    let polylines: Vec<Vec<Point>> = operand
        .iter()
        .map(|outline| flatten(outline.anchors, outline.closed, flatten_mm))
        .collect();
    let near = |point: Point| {
        polylines.iter().any(|points| {
            let n = points.len();
            (0..n)
                .any(|i| point_segment_distance(point, points[i], points[(i + 1) % n]) < threshold)
        })
    };
    rings.retain(|ring| {
        let n = ring.len();
        let close = [0, n / 3, 2 * n / 3]
            .into_iter()
            .filter(|&i| near(to_millimetres(&ring[i])))
            .count();
        close < 2
    });
    rings
}

/// What [`offset`] works with once its input passed the checks.
struct Checked {
    /// The flattening tolerance: the tolerance less two grid pitches.
    flatten_mm: f64,
    /// The distance in grid units.
    units: i64,
    /// The distance on the grid, in millimetres.
    d: f64,
}

/// The refusals [`offset`] makes before any work, in its documented order.
fn checked_request(
    operand: &[Outline<'_>],
    distance: Length,
    tolerance: Tolerance,
) -> Result<Checked, OffsetError> {
    let flatten_mm = tolerance.as_mm() - 2.0 * GRID_MM;
    if !(flatten_mm > 0.0 && flatten_mm.is_finite()) {
        return Err(OffsetError::ToleranceTooSmall);
    }
    let distance_mm = distance.as_mm();
    if !distance_mm.is_finite() || distance_mm.abs() > MAX_COORDINATE_MM {
        return Err(OffsetError::OutOfRange);
    }
    if offset_distance_is_zero(distance) {
        return Err(OffsetError::ZeroDistance);
    }
    let units = snap(Point::new(distance_mm, 0.0)).x;
    if operand.iter().any(|outline| !outline.closed) {
        return Err(OffsetError::OpenPath);
    }
    if !operand.iter().all(outline_in_range) {
        return Err(OffsetError::OutOfRange);
    }
    // Exact: |units| <= 10^10.
    #[allow(clippy::cast_precision_loss)]
    let d = units as f64 * GRID_MM;
    Ok(Checked {
        flatten_mm,
        units,
        d,
    })
}

/// An operand outline as `kurbo` anchors.
fn kurbo_anchors(outline: &Outline<'_>) -> Vec<(KPoint, KVec, KVec)> {
    outline
        .anchors
        .iter()
        .map(|(p, hin, hout)| {
            (
                KPoint::new(p.x, p.y),
                KVec::new(hin.x, hin.y),
                KVec::new(hout.x, hout.y),
            )
        })
        .collect()
}

/// The raw loops of a closed operand: the outlines that bound its nonzero region, each wound
/// with the region on its left, offset by `d`. `None` (the polyline route) when the outlines
/// cross or touch, which shows as a region that is not exactly theirs.
fn closed_loops(
    operand: &[Outline<'_>],
    region: &Paths64,
    d: f64,
    style: RawStyle,
    flatten_tolerance_mm: f64,
) -> Option<Vec<Vec<KSeg>>> {
    let polygons = grid_paths(operand, flatten_tolerance_mm);
    let signs: Vec<i32> = polygons
        .iter()
        .map(|polygon| match double_area(polygon).signum() {
            1 => 1,
            -1 => -1,
            _ => 0,
        })
        .collect();
    let mut loops = Vec::new();
    let mut boundary_area: i128 = 0;
    let mut boundary_count = 0;
    for (index, outline) in operand.iter().enumerate() {
        if signs[index] == 0 {
            continue;
        }
        let probe = polygons[index][0];
        let outside: i32 = (0..operand.len())
            .filter(|&other| other != index)
            .map(|other| winding_number(&polygons[other], probe))
            .sum();
        let inside = outside + signs[index];
        if (outside == 0) == (inside == 0) {
            continue;
        }
        let wanted = if outside == 0 { 1 } else { -1 };
        let mut segments = segments_of(&kurbo_anchors(outline));
        if segments.is_empty() {
            return None;
        }
        if wanted != signs[index] {
            segments = segments.iter().rev().map(KSeg::reverse).collect();
        }
        boundary_area += double_area(&polygons[index]).abs() * i128::from(wanted);
        boundary_count += 1;
        loops.push(closed_loop(&segments, d, style));
    }
    let region_area: i128 = region.iter().map(|path| double_area(path)).sum();
    let slack: i128 = polygons.iter().map(|p| p.len() as i128).sum();
    if boundary_count != region.len() || (region_area - boundary_area).abs() > slack {
        return None;
    }
    Some(loops)
}

/// The winding number of the grid polygon `polygon` around `point` (0 on its boundary counts as
/// outside for the vertex it is probed with; the caller's outlines do not touch).
fn winding_number(polygon: &[Point64], point: Point64) -> i32 {
    let mut winding = 0;
    let n = polygon.len();
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        let side = i128::from(b.x - a.x) * i128::from(point.y - a.y)
            - i128::from(point.x - a.x) * i128::from(b.y - a.y);
        if a.y <= point.y {
            if b.y > point.y && side > 0 {
                winding += 1;
            }
        } else if b.y <= point.y && side < 0 {
            winding -= 1;
        }
    }
    winding
}

/// Whether `distance` rounds to zero on the kernel's 0.001 mm grid, so [`offset`] would refuse it
/// with [`OffsetError::ZeroDistance`]. Callers that check the distance before other refusals use
/// this, so their rule and the kernel's cannot drift apart. A distance that is not finite or out
/// of range is not zero.
#[must_use]
pub fn offset_distance_is_zero(distance: Length) -> bool {
    let distance_mm = distance.as_mm();
    distance_mm.is_finite()
        && distance_mm.abs() <= MAX_COORDINATE_MM
        && snap(Point::new(distance_mm, 0.0)).x == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zero_rule_is_the_grid_rounding() {
        assert!(offset_distance_is_zero(Length::from_mm(0.0)));
        assert!(offset_distance_is_zero(Length::from_mm(-0.0004)));
        assert!(!offset_distance_is_zero(Length::from_mm(0.0005)));
        assert!(!offset_distance_is_zero(Length::from_mm(f64::NAN)));
    }
}
