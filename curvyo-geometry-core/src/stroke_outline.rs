//! The stroke outline (`specs/0085-stroke-to-path`, `adrs.md` decisions 1 and 2): the region a
//! stroke paints (width, join with the miter limit 4, caps, dashes, markers), as closed outlines
//! of exact pieces.
//!
//! The raw loops of every outline, dash, dot and marker (`stroke_loops`, `stroke_dash`,
//! `stroke_markers`) go into one `i_overlay` overlay with the `NonZero` fill rule through
//! `offset_rebuild`, which maps every surviving ring back onto the exact pieces; a ring it cannot
//! map is a polyline of the 0.001 mm grid. The output form is the offset kernel's.

use curvyo_document_core::{DashPattern, Length, LineCap, LineJoin, Markers, Tolerance};
use kurbo::{PathSeg, Point as KPoint, Vec2 as KVec};

use crate::boolean::Outline;
use crate::boolean_grid::{GRID_MM, MAX_COORDINATE_MM};
use crate::offset::{OffsetAnchor, OffsetJoin, OffsetOutlines};
use crate::offset_finish::canonical;
use crate::offset_pieces::{RawStyle, segments_between};
use crate::offset_polyline::without_flattening_noise;
use crate::offset_rebuild::{FillRule, rebuild};
use crate::stroke_dash::{DashPiece, Measured, dash_pieces, object_is_dashed};
use crate::stroke_loops::{Cap, closed_loops, dot_loop, open_loop};
use crate::stroke_markers::marker_loops;

/// Everything about a stroke that shapes its area. Alignment is always centre.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeShape {
    /// The stroke width.
    pub width: Length,
    /// The join at corners (a miter beyond the limit of 4 is a bevel).
    pub join: LineJoin,
    /// The cap at open ends and at every dash end.
    pub cap: LineCap,
    /// The dash pattern in multiples of the width; solid when empty.
    pub dash: DashPattern,
    /// The markers, drawn on every outline as the canvas draws them; the caller passes none
    /// where the canvas draws none (primitives, compound paths).
    pub markers: Markers,
}

/// Why no stroke outline was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StrokeOutlineError {
    /// The tolerance leaves no room for the grid: it must exceed two grid pitches (0.002 mm).
    #[error("tolerance too small for the 0.001 mm grid")]
    ToleranceTooSmall,
    /// A coordinate, handle or the width is not finite, or the stroke reaches beyond 10⁷ mm.
    #[error("a coordinate is not finite or out of range")]
    OutOfRange,
    /// The stroke paints nothing on the 0.001 mm grid (criterion 27): not a refusal.
    #[error("the stroke has no area")]
    Empty,
}

/// The outline of the region `stroke` paints along `outlines` (open ones stay open), as closed
/// outlines in the form of [`OffsetOutlines`]: outer outlines positive, holes negative, lines and
/// arcs exact, curves within `tolerance` of the exact stroke edge.
///
/// # Errors
/// [`StrokeOutlineError`]: checked in the order tolerance, range (coordinates and width), then
/// an empty result.
pub fn stroke_outline(
    outlines: &[Outline<'_>],
    stroke: &StrokeShape,
    tolerance: Tolerance,
) -> Result<OffsetOutlines, StrokeOutlineError> {
    let flatten_mm = tolerance.as_mm() - 2.0 * GRID_MM;
    if !(flatten_mm > 0.0 && flatten_mm.is_finite()) {
        return Err(StrokeOutlineError::ToleranceTooSmall);
    }
    let width = stroke.width.as_mm();
    let width_ok = width.is_finite() && width > 0.0 && width <= MAX_COORDINATE_MM;
    if !width_ok || !outlines.iter().all(|outline| in_range(outline, width)) {
        return Err(StrokeOutlineError::OutOfRange);
    }
    let style = RawStyle {
        join: match stroke.join {
            LineJoin::Miter => OffsetJoin::Miter,
            LineJoin::Round => OffsetJoin::Round,
            LineJoin::Bevel => OffsetJoin::Bevel,
        },
        curve_tolerance_mm: flatten_mm / 2.0,
        clip_long_miters: false,
    };
    let cap = match stroke.cap {
        LineCap::Butt => Cap::Butt,
        LineCap::Round => Cap::Round,
        LineCap::Square => Cap::Square,
    };
    let prepared: Vec<Prepared> = outlines.iter().filter_map(Prepared::new).collect();
    let measured: Vec<Measured<'_>> = prepared
        .iter()
        .map(|outline| Measured::new(&outline.segments))
        .collect();
    let dashed = object_is_dashed(&measured, &stroke.dash, width, cap);
    let mut loops: Vec<Vec<PathSeg>> = Vec::new();
    for (outline, measured) in prepared.iter().zip(&measured) {
        outline_loops(outline, measured, stroke, style, cap, dashed, &mut loops);
    }
    if loops.is_empty() {
        return Err(StrokeOutlineError::Empty);
    }
    let rebuilt = rebuild(
        &loops,
        flatten_mm,
        tolerance.as_mm(),
        FillRule::NonZero,
        |rings| without_flattening_noise(rings, flatten_mm),
    );
    if rebuilt.outlines.is_empty() {
        return Err(StrokeOutlineError::Empty);
    }
    let outlines = canonical(&rebuilt.outlines);
    let in_range = |p: &OffsetAnchor| {
        p.point.x.abs() <= MAX_COORDINATE_MM && p.point.y.abs() <= MAX_COORDINATE_MM
    };
    if !outlines.iter().flatten().all(in_range) {
        return Err(StrokeOutlineError::OutOfRange);
    }
    Ok(OffsetOutlines::from_parts(outlines, rebuilt.fallback))
}

/// One outline ready to stroke: its anchors, segments and whether it is closed.
struct Prepared {
    anchors: Vec<(KPoint, KVec, KVec)>,
    segments: Vec<PathSeg>,
    closed: bool,
}

impl Prepared {
    /// The anchors and segments of `outline`, or `None` when it has no segment.
    fn new(outline: &Outline<'_>) -> Option<Self> {
        let anchors: Vec<(KPoint, KVec, KVec)> = outline
            .anchors
            .iter()
            .map(|(p, hin, hout)| {
                (
                    KPoint::new(p.x, p.y),
                    KVec::new(hin.x, hin.y),
                    KVec::new(hout.x, hout.y),
                )
            })
            .collect();
        if anchors.len() < 2 {
            return None;
        }
        let segments = segments_between(&anchors, outline.closed);
        (!segments.is_empty()).then_some(Self {
            anchors,
            segments,
            closed: outline.closed,
        })
    }
}

/// The raw loops of one outline (`measured` its lengths): the stroke (solid, or dashed when
/// `dashed`) and the markers.
fn outline_loops(
    outline: &Prepared,
    measured: &Measured<'_>,
    stroke: &StrokeShape,
    style: RawStyle,
    cap: Cap,
    dashed: bool,
    loops: &mut Vec<Vec<PathSeg>>,
) {
    let Prepared {
        anchors,
        segments,
        closed,
    } = outline;
    let width = stroke.width.as_mm();
    let half = width / 2.0;
    let pieces = if dashed {
        dash_pieces(measured, &stroke.dash, width, cap)
    } else {
        None
    };
    match pieces {
        None if *closed => loops.extend(closed_loops(segments, half, style)),
        None => loops.push(open_loop(segments, half, style, cap)),
        Some(pieces) => {
            for piece in pieces {
                match piece {
                    DashPiece::Dash(chain) => loops.push(open_loop(&chain, half, style, cap)),
                    DashPiece::Dot { at, ahead } => loops.extend(dot_loop(at, ahead, half, cap)),
                }
            }
        }
    }
    loops.extend(marker_loops(
        anchors,
        *closed,
        segments,
        stroke.markers,
        width,
    ));
}

/// Whether every coordinate and handle of `outline`, grown by `width` (a marker reaches two
/// widths, a cap or miter four half widths), stays within range.
fn in_range(outline: &Outline<'_>, width: f64) -> bool {
    let reach = 4.0 * width;
    let ok = |value: f64| value.is_finite() && value.abs() + reach <= MAX_COORDINATE_MM;
    outline
        .anchors
        .iter()
        .all(|(point, handle_in, handle_out)| {
            [
                point.x,
                point.y,
                point.x + handle_in.x,
                point.y + handle_in.y,
                point.x + handle_out.x,
                point.y + handle_out.y,
            ]
            .into_iter()
            .all(ok)
        })
}
