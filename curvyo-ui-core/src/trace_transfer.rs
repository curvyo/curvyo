//! One trace job of the Trace tool's worker and its result as a flat list of numbers
//! (`specs/0025-raster-trace` criteria 11, 15, 16 and 17; ADR 0023 §5): the worker runs
//! [`trace_job`], posts the numbers, and the session reads them back with [`read_trace_wire`].

use std::ops::ControlFlow;

use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{NodeLimitOutcome, Raster, TraceError, TraceParams, trace};
use kurbo::{BezPath, PathEl, Point};

/// How a trace job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceStatus {
    /// Outlines were made (possibly none).
    Traced,
    /// A region has 65 000 holes or more: "Too much detail: raise Specks." (criterion 16).
    TooDetailed,
    /// The picture or the map cannot be traced.
    Invalid,
}

impl TraceStatus {
    fn code(self) -> f64 {
        match self {
            Self::Traced => 0.0,
            Self::TooDetailed => 1.0,
            Self::Invalid => 2.0,
        }
    }

    /// The host's code: `"traced"`, `"too_detailed"` or `"invalid"`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Traced => "traced",
            Self::TooDetailed => "too_detailed",
            Self::Invalid => "invalid",
        }
    }
}

/// A trace result as the session keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceWire {
    /// How the job ended.
    pub status: TraceStatus,
    /// Outlines.
    pub paths: usize,
    /// Anchor points of all outlines.
    pub nodes: usize,
    /// Regions and holes removed as specks.
    pub specks_removed: usize,
    /// Pixels below the Threshold before specks.
    pub foreground_pixels: u64,
    /// One outline covers the whole picture.
    pub covers_whole_image: bool,
    /// What the Node limit did (criterion 43).
    pub node_limit: NodeLimitOutcome,
    /// The outlines in document millimetres.
    pub outlines: Vec<BezPath>,
}

/// The numbers before the outlines: status, paths, nodes, specks, foreground pixels, whole
/// picture, Node limit outcome (0 untouched, 1 reached, 2 not met), outline count.
pub const TRACE_WIRE_HEADER: usize = 8;

const MOVE: f64 = 0.0;
const LINE: f64 = 1.0;
const CURVE: f64 = 3.0;
const CLOSE: f64 = 4.0;

/// One trace with slice 1's three bar values and every other setting at its first-use value;
/// see [`trace_job_with`].
#[must_use]
pub fn trace_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    threshold: u8,
    smooth: u8,
    specks_mm: f64,
) -> Vec<f64> {
    let params = TraceParams {
        threshold,
        smooth,
        specks: Length::from_mm(specks_mm),
        ..TraceParams::with_threshold(threshold)
    };
    trace_job_with(rgba, width, height, pixel_map, &params)
}

/// One trace for the worker: `rgba` (`width` x `height`, straight alpha) mapped by `pixel_map`
/// (`[a, b, c, d, e, f]`) with `params`; the result as numbers. Never panics on bad input: it
/// says `Invalid`.
#[must_use]
pub fn trace_job_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &TraceParams,
) -> Vec<f64> {
    let raster = Raster {
        width,
        height,
        rgba,
    };
    let mut go = |_| ControlFlow::Continue(());
    match trace(raster, Affine::from_array(pixel_map), params, &mut go) {
        Ok(result) => {
            #[allow(clippy::cast_precision_loss)] // counts far below 2^52
            let mut wire = vec![
                TraceStatus::Traced.code(),
                result.counts.paths as f64,
                result.counts.nodes as f64,
                result.counts.specks_removed as f64,
                result.foreground_pixels as f64,
                f64::from(u8::from(result.covers_whole_image)),
                node_limit_code(result.node_limit),
                result.outlines.len() as f64,
            ];
            for outline in &result.outlines {
                write_outline(&mut wire, outline);
            }
            wire
        }
        Err(TraceError::TooDetailed) => status_only(TraceStatus::TooDetailed),
        Err(TraceError::Invalid(_) | TraceError::Cancelled) => status_only(TraceStatus::Invalid),
    }
}

/// The automatic Threshold of `rgba` (`width` x `height`), for the worker (criterion 9).
#[must_use]
pub fn trace_auto_threshold(rgba: &[u8], width: u32, height: u32) -> u8 {
    curvyo_vectorize_core::auto_threshold(Raster {
        width,
        height,
        rgba,
    })
}

fn node_limit_code(outcome: NodeLimitOutcome) -> f64 {
    match outcome {
        NodeLimitOutcome::Untouched => 0.0,
        NodeLimitOutcome::Reached => 1.0,
        NodeLimitOutcome::NotMet => 2.0,
    }
}

fn status_only(status: TraceStatus) -> Vec<f64> {
    let mut wire = vec![0.0; TRACE_WIRE_HEADER];
    wire[0] = status.code();
    wire
}

fn write_outline(wire: &mut Vec<f64>, outline: &BezPath) {
    let elements = outline.elements();
    #[allow(clippy::cast_precision_loss)]
    wire.push(elements.len() as f64);
    let mut last = Point::ZERO;
    for element in elements {
        match *element {
            PathEl::MoveTo(p) => {
                wire.extend([MOVE, p.x, p.y]);
                last = p;
            }
            PathEl::LineTo(p) => {
                wire.extend([LINE, p.x, p.y]);
                last = p;
            }
            // The tracer makes no quadratic segments; one would be raised to a cubic.
            PathEl::QuadTo(c, p) => {
                let (c1, c2) = (last.lerp(c, 2.0 / 3.0), p.lerp(c, 2.0 / 3.0));
                wire.extend([CURVE, c1.x, c1.y, c2.x, c2.y, p.x, p.y]);
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                wire.extend([CURVE, c1.x, c1.y, c2.x, c2.y, p.x, p.y]);
                last = p;
            }
            PathEl::ClosePath => wire.push(CLOSE),
        }
    }
}

/// A count read from the wire: a whole, non-negative, finite number.
fn count(value: f64) -> Option<usize> {
    // invariant: checked whole and in range before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value < 9.0e15)
        .then_some(value as usize)
}

/// Reads the numbers of [`trace_job`] back; `None` when they are not a well-formed result.
#[must_use]
pub fn read_trace_wire(wire: &[f64]) -> Option<TraceWire> {
    let header = wire.get(..TRACE_WIRE_HEADER)?;
    let status = match count(header[0])? {
        0 => TraceStatus::Traced,
        1 => TraceStatus::TooDetailed,
        2 => TraceStatus::Invalid,
        _ => return None,
    };
    let mut at = TRACE_WIRE_HEADER;
    let mut next = || -> Option<f64> {
        let value = *wire.get(at)?;
        at += 1;
        value.is_finite().then_some(value)
    };
    let point = |next: &mut dyn FnMut() -> Option<f64>| Some(Point::new(next()?, next()?));
    let node_limit = match count(header[6])? {
        0 => NodeLimitOutcome::Untouched,
        1 => NodeLimitOutcome::Reached,
        2 => NodeLimitOutcome::NotMet,
        _ => return None,
    };
    let outline_count = count(header[7])?;
    let mut outlines = Vec::with_capacity(outline_count.min(wire.len()));
    for _ in 0..outline_count {
        let elements = count(next()?)?;
        let mut path = BezPath::new();
        for _ in 0..elements {
            let tag = next()?;
            let element = if tag == MOVE {
                PathEl::MoveTo(point(&mut next)?)
            } else if tag == LINE {
                PathEl::LineTo(point(&mut next)?)
            } else if tag == CURVE {
                PathEl::CurveTo(point(&mut next)?, point(&mut next)?, point(&mut next)?)
            } else if tag == CLOSE {
                PathEl::ClosePath
            } else {
                return None;
            };
            path.push(element);
        }
        outlines.push(path);
    }
    if at != wire.len() {
        return None;
    }
    Some(TraceWire {
        status,
        paths: count(header[1])?,
        nodes: count(header[2])?,
        specks_removed: count(header[3])?,
        foreground_pixels: u64::try_from(count(header[4])?).ok()?,
        covers_whole_image: header[5] != 0.0,
        node_limit,
        outlines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_picture() -> Vec<u8> {
        (0..16 * 16)
            .flat_map(|i| {
                let (x, y) = (i % 16, i / 16);
                let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                    0
                } else {
                    255
                };
                [v, v, v, 255]
            })
            .collect()
    }

    #[test]
    fn a_job_round_trips_through_the_wire() {
        let wire = trace_job(
            &square_picture(),
            16,
            16,
            [0.5, 0.0, 0.0, 0.5, 1.0, 2.0],
            128,
            80,
            0.0,
        );
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(read.status, TraceStatus::Traced);
        assert_eq!((read.paths, read.nodes, read.specks_removed), (1, 4, 0));
        assert_eq!(read.foreground_pixels, 64);
        assert!(!read.covers_whole_image);
        assert_eq!(read.node_limit, NodeLimitOutcome::Untouched);
        let b = kurbo::Shape::bounding_box(&read.outlines[0]);
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (3.0, 4.0, 7.0, 8.0));
    }

    #[test]
    fn bad_input_and_bad_wires_are_refused() {
        let wire = trace_job(
            &[0; 8],
            16,
            16,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            128,
            80,
            0.0,
        );
        assert_eq!(read_trace_wire(&wire).unwrap().status, TraceStatus::Invalid);
        assert_eq!(read_trace_wire(&[0.0; 3]), None);
        let mut good = trace_job(
            &square_picture(),
            16,
            16,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            128,
            80,
            0.0,
        );
        good.push(1.0);
        assert_eq!(read_trace_wire(&good), None);
        good.pop();
        good[TRACE_WIRE_HEADER + 1] = 9.0;
        assert_eq!(read_trace_wire(&good), None);
        good[TRACE_WIRE_HEADER + 1] = MOVE;
        good[TRACE_WIRE_HEADER + 2] = f64::NAN;
        assert_eq!(read_trace_wire(&good), None);
    }

    /// Criterion 43: the Node limit's outcome crosses the wire with the full settings.
    #[test]
    fn the_node_limit_outcome_crosses_the_wire() {
        let disc: Vec<u8> = (0..64 * 64)
            .flat_map(|i| {
                let (x, y) = (f64::from(i % 64) - 31.5, f64::from(i / 64) - 31.5);
                let v = if x * x + y * y < 600.0 { 0 } else { 255 };
                [v, v, v, 255]
            })
            .collect();
        let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let params = |limit| TraceParams {
            node_limit: Some(limit),
            specks: Length::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        };
        let free = read_trace_wire(&trace_job_with(&disc, 64, 64, map, &params(1000))).unwrap();
        assert_eq!(free.node_limit, NodeLimitOutcome::Untouched);
        let tight = read_trace_wire(&trace_job_with(&disc, 64, 64, map, &params(1))).unwrap();
        assert_eq!(tight.node_limit, NodeLimitOutcome::NotMet);
        assert!(tight.nodes > 1);
    }
}
