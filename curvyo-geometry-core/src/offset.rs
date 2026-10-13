//! Offsetting one operand's region outward or inward by a signed distance
//! (`specs/0038-path-offset`; ADR 0003 §4 as amended on 2026-10-10 and 2026-10-12).
//!
//! # Pipeline
//!
//! 1. `offset_pieces` builds the exact raw offset of every outline: parallel lines, offset
//!    cubics, true arcs for Round joins and caps, miter and bevel lines.
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
use i_overlay::i_float::int::angle::Angle;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::outline::offset::IntOutlineOffset;
use i_overlay::mesh::int::stroke::offset::IntStrokeOffset;
use i_overlay::mesh::int::style::{IntLineCap, IntLineJoin, IntOutlineStyle, IntStrokeStyle};
use i_overlay::mesh::math::MathMode;
use kurbo::{ParamCurve, PathSeg as KSeg, Point as KPoint, Vec2 as KVec};

use crate::boolean::{Outline, grid_paths, outline_in_range};
use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{
    GRID_MM, MAX_COORDINATE_MM, Path64, Paths64, Point64, double_area, normalize, snap,
};
use crate::flatten::{flatten_closed, flatten_open};
use crate::offset_pieces::{RawStyle, closed_loop, open_loop, segments_of};
use crate::offset_rebuild::{double_area_mm2, is_smooth, rebuild};

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

/// How the ends of an open path are finished when it is grown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetCap {
    /// A half disc of radius |d|.
    Round,
    /// Cut square at the end point.
    Flat,
    /// Cut square |d| beyond the end point.
    Square,
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
    /// An open path cannot be shrunk.
    #[error("an open path cannot be inset")]
    InsetOfOpenPath,
    /// Nothing is left: the operand vanished under the inset.
    #[error("the result is empty")]
    Empty,
}

/// One turn in the library's binary angle units.
const UNITS_PER_TURN: f64 = 4_294_967_296.0;

/// The miter limit of 4 (SVG's) as the smallest interior angle that keeps a sharp miter:
/// `2 x asin(1/4)` = 28.955°, in binary angle units (`round(0.505360510284157 / 2π x 2³²)`).
/// A constant, so no platform's `asin` takes part.
const MITER_LIMIT_ANGLE: Angle = Angle::from_bits(345_446_896);

/// The library's default near-straight cutoff for miters, 5°.
const MITER_MIN_TURN_DEFAULT_RAD: f64 = 5.0 * std::f64::consts::PI / 180.0;

/// Largest miter error at a near-straight turn, squared over the distance (see `miter_min_turn`).
const MITER_STRAIGHT_ERROR_MM: f64 = 0.005;

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

    /// Every outline flattened to a polygon whose chords stay within `tolerance_mm` of it.
    #[must_use]
    pub fn flattened(&self, tolerance_mm: f64) -> Vec<Vec<Point>> {
        self.outlines
            .iter()
            .map(|outline| {
                let triples: Vec<crate::OutlineTriple> = outline
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                flatten_closed(&triples, tolerance_mm)
            })
            .collect()
    }
}

/// Offsets one operand by `distance`: positive grows its region (Outset), negative shrinks it
/// (Inset).
///
/// `operand` is the operand's outlines. If all are closed, the region they paint (nonzero) is
/// offset with `join`; if any is open, the operand is a line and grows into the closed outline of
/// every point within `|distance|` of it, with `join` at its corners and `cap` at its ends.
///
/// The result is built from the exact offset of every piece of the operand (parallel lines,
/// offset cubics, arcs, miter and bevel lines, caps); `i_overlay` decides only which parts survive
/// (ADR 0003 §4 as amended on 2026-10-12). `tolerance` bounds how far the result may lie from the
/// exact offset of the operand's curves. An operand whose outlines cross or touch takes the
/// polyline route of the first build, as does any outline that cannot be matched.
///
/// # Errors
///
/// [`OffsetError`]: checked in the order tolerance, distance (not finite or out of range, then
/// zero), inset of an open operand, operand out of range, closed operand without area; after the
/// offset, an empty result, then a result out of range.
pub fn offset(
    operand: &[Outline<'_>],
    distance: Length,
    join: OffsetJoin,
    cap: OffsetCap,
    tolerance: Tolerance,
) -> Result<OffsetOutlines, OffsetError> {
    let flatten_tolerance_mm = tolerance.as_mm() - 2.0 * GRID_MM;
    if !(flatten_tolerance_mm > 0.0 && flatten_tolerance_mm.is_finite()) {
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
    let is_open = operand.iter().any(|outline| !outline.closed);
    if is_open && units < 0 {
        return Err(OffsetError::InsetOfOpenPath);
    }
    if !operand.iter().all(outline_in_range) {
        return Err(OffsetError::OutOfRange);
    }
    // The distance on the grid, as the polyline route uses it.
    #[allow(clippy::cast_precision_loss)] // |units| <= 10^10
    let d = units as f64 * GRID_MM;
    let style = RawStyle {
        join,
        cap,
        curve_tolerance_mm: flatten_tolerance_mm / 2.0,
    };
    let loops = if is_open {
        open_loops(operand, d.abs(), style)
    } else {
        let region = normalize(&grid_paths(operand, flatten_tolerance_mm));
        if region.is_empty() {
            return Err(OffsetError::NoArea);
        }
        closed_loops(operand, &region, d, style, flatten_tolerance_mm)
    };
    let (outlines, fallback) = if let Some(loops) = loops {
        let rebuilt = rebuild(&loops, flatten_tolerance_mm, tolerance.as_mm(), |rings| {
            without_flattening_noise(rings, flatten_tolerance_mm)
        });
        (rebuilt.outlines, rebuilt.fallback)
    } else {
        let polylines = polyline_offset(operand, units, join, cap, flatten_tolerance_mm, is_open)?;
        let count = polylines.len();
        (
            polylines.iter().map(|path| lines_of_mm(path)).collect(),
            count,
        )
    };
    if outlines.is_empty() {
        return Err(OffsetError::Empty);
    }
    let in_range = |p: &OffsetAnchor| {
        p.point.x.abs() <= MAX_COORDINATE_MM && p.point.y.abs() <= MAX_COORDINATE_MM
    };
    let outlines = canonical(&outlines);
    if !outlines.iter().flatten().all(in_range) {
        return Err(OffsetError::OutOfRange);
    }
    Ok(OffsetOutlines { outlines, fallback })
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

/// The raw loops of an open operand grown by `radius`: each open outline stroked with its caps,
/// each closed one on both sides. `None` (the polyline route) if an outline has no segment.
fn open_loops(operand: &[Outline<'_>], radius: f64, style: RawStyle) -> Option<Vec<Vec<KSeg>>> {
    let mut loops = Vec::new();
    for outline in operand {
        let segments = segments_of(&kurbo_anchors(outline), outline.closed);
        if segments.is_empty() {
            return None;
        }
        if outline.closed {
            let reversed: Vec<KSeg> = segments.iter().rev().map(KSeg::reverse).collect();
            loops.push(closed_loop(&segments, radius, style));
            loops.push(closed_loop(&reversed, radius, style));
        } else {
            loops.push(open_loop(&segments, radius, style));
        }
    }
    Some(loops)
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
        let mut segments = segments_of(&kurbo_anchors(outline), true);
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

/// A grid polygon as closed lines in millimetres.
fn lines_of_mm(path: &[Point64]) -> Vec<KSeg> {
    let mm = |p: Point64| {
        #[allow(clippy::cast_precision_loss)] // |units| <= 10^10
        KPoint::new(p.x as f64 * GRID_MM, p.y as f64 * GRID_MM)
    };
    (0..path.len())
        .map(|i| {
            KSeg::Line(kurbo::Line::new(
                mm(path[i]),
                mm(path[(i + 1) % path.len()]),
            ))
        })
        .collect()
}

/// The outlines as nodes, each started at its smallest node, sorted by that node and then by
/// area (larger first).
fn canonical(outlines: &[Vec<KSeg>]) -> Vec<Vec<OffsetAnchor>> {
    let mut keyed: Vec<(f64, Vec<OffsetAnchor>)> = outlines
        .iter()
        .filter(|pieces| !pieces.is_empty())
        .map(|pieces| {
            let n = pieces.len();
            let start = (0..n)
                .min_by(|&a, &b| {
                    let (pa, pb) = (pieces[a].eval(0.0), pieces[b].eval(0.0));
                    pa.x.total_cmp(&pb.x).then(pa.y.total_cmp(&pb.y))
                })
                .unwrap_or(0);
            let anchors = (0..n)
                .map(|k| {
                    let index = (start + k) % n;
                    let (before, piece) = (&pieces[(index + n - 1) % n], &pieces[index]);
                    let point = piece.eval(0.0);
                    let handle_in = match *before {
                        KSeg::Cubic(c) => c.p2 - c.p3,
                        _ => KVec::ZERO,
                    };
                    let handle_out = match *piece {
                        KSeg::Cubic(c) => c.p1 - c.p0,
                        _ => KVec::ZERO,
                    };
                    OffsetAnchor {
                        point: Point::new(point.x, point.y),
                        handle_in: Vec2::new(handle_in.x, handle_in.y),
                        handle_out: Vec2::new(handle_out.x, handle_out.y),
                        smooth: is_smooth(before, piece),
                    }
                })
                .collect();
            (double_area_mm2(pieces), anchors)
        })
        .collect();
    keyed.sort_by(|(area_a, a), (area_b, b)| {
        a[0].point
            .x
            .total_cmp(&b[0].point.x)
            .then(a[0].point.y.total_cmp(&b[0].point.y))
            .then(area_b.total_cmp(area_a))
    });
    keyed.into_iter().map(|(_, anchors)| anchors).collect()
}

/// The polyline route of the first build: flatten, offset with `i_overlay`'s own offsetter, clean
/// up. Used for operands whose outlines cross or touch and for degenerate open lines.
fn polyline_offset(
    operand: &[Outline<'_>],
    units: i64,
    join: OffsetJoin,
    cap: OffsetCap,
    flatten_tolerance_mm: f64,
    is_open: bool,
) -> Result<Paths64, OffsetError> {
    #[allow(clippy::cast_precision_loss)] // |units| <= 10^10
    let radius_mm = (units as f64 * GRID_MM).abs();
    let raw = if is_open {
        stroke_line(operand, units, join, cap, radius_mm, flatten_tolerance_mm)?
    } else {
        let region = normalize(&grid_paths(operand, flatten_tolerance_mm));
        let style = IntOutlineStyle::new(units)
            .line_join(int_join(join, radius_mm, flatten_tolerance_mm))
            .miter_min_turn(miter_min_turn(radius_mm))
            .math(MathMode::Integer);
        region
            .validate_outline(&style)
            .map_err(|_| OffsetError::OutOfRange)?;
        let shapes = region
            .outline(&style)
            .map_err(|_| OffsetError::OutOfRange)?;
        shapes.into_iter().flatten().collect()
    };
    Ok(without_flattening_noise(
        cleanup(&raw),
        flatten_tolerance_mm,
    ))
}

/// `paths` without the outlines that are on average narrower than the flattening tolerance.
///
/// Where an inset makes two curved outlines meet (a ring shrunk by half its width), the chords of
/// the two flattened curves do not coincide and leave slivers a few thousandths of a millimetre
/// wide (up to twice the tolerance at their thickest, as both chords may deviate by it). The
/// flattening is only exact to its tolerance, so a part that thin is not a part of the exact result: the spec counts that ring as vanished (criterion 16). The width estimate is twice
/// the area over the perimeter, which is half the side for a square: a square inset to a side of
/// 0.02 mm (criterion 15) is 0.01 mm wide by it and stays (the tolerance is 0.008 mm).
fn without_flattening_noise(mut paths: Paths64, flatten_tolerance_mm: f64) -> Paths64 {
    let threshold_units = flatten_tolerance_mm / GRID_MM;
    paths.retain(|path| {
        let mut perimeter = 0.0_f64;
        let Some(&last) = path.last() else {
            return false;
        };
        let mut previous = last;
        for point in path {
            // IEEE `sqrt` of a sum of squares, not `hypot` (criterion 12). Exact below 2^53.
            #[allow(clippy::cast_precision_loss)]
            let (dx, dy) = ((point.x - previous.x) as f64, (point.y - previous.y) as f64);
            perimeter += (dx * dx + dy * dy).sqrt();
            previous = *point;
        }
        #[allow(clippy::cast_precision_loss)]
        let double_area = double_area(path).unsigned_abs() as f64;
        double_area >= threshold_units * perimeter
    });
    paths
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

/// The open operand grown into a closed region: every outline stroked as a line (an open one
/// with caps, a closed one as a loop), the pieces united.
fn stroke_line(
    operand: &[Outline<'_>],
    units: i64,
    join: OffsetJoin,
    cap: OffsetCap,
    radius_mm: f64,
    flatten_tolerance_mm: f64,
) -> Result<Paths64, OffsetError> {
    let int_cap = match cap {
        OffsetCap::Round => IntLineCap::Round(arc_options(radius_mm, flatten_tolerance_mm)),
        OffsetCap::Flat => IntLineCap::Butt,
        OffsetCap::Square => IntLineCap::Square,
    };
    let style = IntStrokeStyle::new(2 * units)
        .start_cap(int_cap.clone())
        .end_cap(int_cap)
        .line_join(int_join(join, radius_mm, flatten_tolerance_mm))
        .miter_min_turn(miter_min_turn(radius_mm))
        .math(MathMode::Integer);
    let mut pieces = Paths64::new();
    for outline in operand {
        let points = if outline.closed {
            flatten_closed(outline.anchors, flatten_tolerance_mm)
        } else {
            flatten_open(outline.anchors, flatten_tolerance_mm)
        };
        let line = snap_line(&points);
        line.validate_stroke(&style)
            .map_err(|_| OffsetError::OutOfRange)?;
        let shapes = line
            .stroke(&style, outline.closed)
            .map_err(|_| OffsetError::OutOfRange)?;
        pieces.extend(shapes.into_iter().flatten());
    }
    Ok(normalize(&pieces))
}

/// Snaps a line's vertices to the grid and drops consecutive duplicates (not the wrap-around pair:
/// an open line may end where it starts).
fn snap_line(points: &[Point]) -> Path64 {
    let mut line: Path64 = Vec::with_capacity(points.len());
    for point in points {
        let snapped = snap(*point);
        if line.last() != Some(&snapped) {
            line.push(snapped);
        }
    }
    line
}

/// The library's join for `join` at radius `radius_mm`.
fn int_join(join: OffsetJoin, radius_mm: f64, flatten_tolerance_mm: f64) -> IntLineJoin {
    match join {
        OffsetJoin::Round => IntLineJoin::Round(arc_options(radius_mm, flatten_tolerance_mm)),
        OffsetJoin::Miter => IntLineJoin::Miter(MITER_LIMIT_ANGLE),
        OffsetJoin::Bevel => IntLineJoin::Bevel,
    }
}

/// Arc steps that keep every chord within `tolerance_mm` of an arc of radius `radius_mm`.
///
/// A chord over the angle θ deviates from its arc by `r (1 - cos(θ/2)) ≤ r θ² / 8`, so
/// `θ = 2 sqrt(2 t / r)` keeps it within `t`. That is within 12 % of the exact largest step for
/// the tolerances used, so the node count stays inside criterion 11's budget, and it needs only
/// `sqrt`. The library clamps the step to 0.35°..45°.
fn arc_options(radius_mm: f64, tolerance_mm: f64) -> ArcOptions {
    let step_rad = 2.0 * (2.0 * tolerance_mm / radius_mm).sqrt();
    ArcOptions {
        max_step: angle_of(step_rad, ArcOptions::MAX_STEP),
        ..ArcOptions::default()
    }
}

/// The near-straight cutoff of miters: below it the library closes a turn with a straight edge,
/// which misses the miter by up to `r θ² / 8`. `min(5°, sqrt(8 x 0.005 mm / r))` keeps that within
/// 0.005 mm at any distance (`adrs.md`, "near-straight miter cutoff").
fn miter_min_turn(radius_mm: f64) -> Angle {
    let limit_rad = (8.0 * MITER_STRAIGHT_ERROR_MM / radius_mm).sqrt();
    angle_of(
        limit_rad.min(MITER_MIN_TURN_DEFAULT_RAD),
        Angle::QUARTER_TURN,
    )
}

/// `radians` as a binary angle, at most `cap` (for an angle that is huge or not a number).
fn angle_of(radians: f64, cap: Angle) -> Angle {
    let units = radians / std::f64::consts::TAU * UNITS_PER_TURN;
    // Below `cap` (≤ 2³⁰) after the comparison, and not negative, so the cast is exact up to
    // rounding; NaN fails the comparison and takes the cap.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if units >= 0.0 && units < f64::from(cap.bits()) {
        Angle::from_bits(units.round() as u32)
    } else {
        cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_miter_limit_constant_is_two_arcsines_of_a_quarter() {
        let expected = 2.0 * 0.25_f64.asin() / std::f64::consts::TAU * UNITS_PER_TURN;
        assert!((f64::from(MITER_LIMIT_ANGLE.bits()) - expected).abs() <= 0.5);
    }

    #[test]
    fn arc_steps_keep_the_chord_within_the_tolerance() {
        for radius in [0.5, 2.0, 5.0, 50.0, 1000.0] {
            let step = f64::from(arc_options(radius, 0.008).max_step.bits()) / UNITS_PER_TURN
                * std::f64::consts::TAU;
            let sagitta = radius * (1.0 - (step / 2.0).cos());
            assert!(sagitta <= 0.008 + 1e-9, "radius {radius}: {sagitta}");
        }
    }

    #[test]
    fn the_miter_cutoff_shrinks_with_the_distance() {
        assert_eq!(
            miter_min_turn(1.0),
            angle_of(MITER_MIN_TURN_DEFAULT_RAD, Angle::QUARTER_TURN)
        );
        let at_fifty =
            f64::from(miter_min_turn(50.0).bits()) / UNITS_PER_TURN * std::f64::consts::TAU;
        assert!(50.0 * at_fifty * at_fifty / 8.0 <= 0.005 + 1e-9);
    }

    #[test]
    fn the_zero_rule_is_the_grid_rounding() {
        assert!(offset_distance_is_zero(Length::from_mm(0.0)));
        assert!(offset_distance_is_zero(Length::from_mm(-0.0004)));
        assert!(!offset_distance_is_zero(Length::from_mm(0.0005)));
        assert!(!offset_distance_is_zero(Length::from_mm(f64::NAN)));
        assert_eq!(
            without_flattening_noise(vec![Vec::new()], 0.008),
            Paths64::new()
        );
    }

    #[test]
    fn a_nan_angle_takes_the_cap() {
        assert_eq!(angle_of(f64::NAN, Angle::QUARTER_TURN), Angle::QUARTER_TURN);
    }
}
