//! Evaluates editable Spiro controls into cubic Bézier spans.

use crate::spiro_approx::approximate_spiro_segment;
use crate::spiro_solver::{Failure, Solution, solve_corners};
use curvyo_document_core::{Point, Tolerance};

/// One Spiro interpolation point, optionally breaking smoothness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiroControl {
    /// Control position in document coordinates.
    pub point: Point,
    /// Whether the curve has a corner at this point.
    pub corner: bool,
}

/// A cubic Bézier span approximating a Spiro segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiroCubic {
    /// Span start.
    pub start: Point,
    /// First control point.
    pub control1: Point,
    /// Second control point.
    pub control2: Point,
    /// Span end.
    pub end: Point,
}

/// A Spiro curve could not be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SpiroError {
    /// Input is invalid or cannot be represented.
    #[error("Spiro controls or tolerance are invalid")]
    Invalid,
    /// The solver or approximation could not converge.
    #[error("Spiro curve could not be evaluated at the requested tolerance")]
    NoConvergence,
    /// The solution found is far from the earlier one: another branch, or a loop.
    #[error("Spiro curve jumps away from its earlier shape")]
    Far,
}
impl From<Failure> for SpiroError {
    fn from(value: Failure) -> Self {
        match value {
            Failure::Invalid => Self::Invalid,
            Failure::NoConvergence => Self::NoConvergence,
        }
    }
}

/// The solved shape of a Spiro curve: the four curvature coefficients of each segment, in the
/// segment's own scale-free terms, so a shape stays a good start when its controls move.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiroShape {
    segments: Vec<[f64; 4]>,
}

impl SpiroShape {
    /// The coefficients of each segment, in order (for a closed curve the last one closes it).
    #[must_use]
    pub fn segments(&self) -> &[[f64; 4]] {
        &self.segments
    }
}

/// How far a segment's tangent may turn away from the same segment's earlier shape between two
/// solves of the same controls moved a little; a larger change is a jump to another solution
/// (customer 2026-11-11).
pub const SPIRO_MAX_TURN_CHANGE: f64 = std::f64::consts::FRAC_PI_2;
/// How far the tangent may sweep along one segment after a control was added or removed, unless
/// the segment swept more before: more is a loop.
pub const SPIRO_MAX_NEW_SWEEP: f64 = 1.5 * std::f64::consts::PI;

/// How much less a fresh solve must sweep than the earlier branch for a drag to leave a loop.
const SPIRO_UNLOOP: f64 = std::f64::consts::FRAC_PI_2;

/// The parameters at which a segment's tangent is compared.
const TURN_SAMPLES: [f64; 5] = [-0.5, -0.25, 0.0, 0.25, 0.5];

/// The tangent direction along a segment relative to its middle, at `s` in `[-1/2, 1/2]`.
fn turn(k: &[f64; 4], s: f64) -> f64 {
    k[0] * s + k[1] * s * s / 2.0 + k[2] * s.powi(3) / 6.0 + k[3] * s.powi(4) / 24.0
}

/// The largest sweep of any segment of a shape.
fn most_sweep(shape: &[[f64; 4]]) -> f64 {
    shape.iter().map(sweep).fold(0.0, f64::max)
}

/// How far the tangent sweeps along a segment.
fn sweep(k: &[f64; 4]) -> f64 {
    let turns = TURN_SAMPLES.map(|s| turn(k, s));
    let high = turns.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let low = turns.iter().copied().fold(f64::INFINITY, f64::min);
    high - low
}

/// Whether `shape` stays near `near`. With every segment known (the controls moved), each turns
/// at most [`SPIRO_MAX_TURN_CHANGE`] differently: more is a jump. With a control added or
/// removed the old segments may reshape, but none sweeps more than [`SPIRO_MAX_NEW_SWEEP`] or
/// its earlier sweep: more is a loop.
fn stays_near(shape: &[[f64; 4]], near: &[Option<[f64; 4]>]) -> bool {
    if near.iter().all(Option::is_some) {
        return shape.iter().zip(near.iter().flatten()).all(|(k, earlier)| {
            TURN_SAMPLES
                .iter()
                .all(|&s| (turn(k, s) - turn(earlier, s)).abs() <= SPIRO_MAX_TURN_CHANGE)
        });
    }
    shape.iter().zip(near).all(|(k, earlier)| {
        sweep(k) <= earlier.map_or(SPIRO_MAX_NEW_SWEEP, |e| sweep(&e).max(SPIRO_MAX_NEW_SWEEP))
    })
}

fn split(controls: &[SpiroControl]) -> (Vec<(f64, f64)>, Vec<bool>) {
    (
        controls.iter().map(|c| (c.point.x, c.point.y)).collect(),
        controls.iter().map(|c| c.corner).collect(),
    )
}

/// Solves an open or closed Spiro curve from scratch.
///
/// # Errors
/// Returns an error for invalid controls or when no solution is found.
pub fn solve_spiro(controls: &[SpiroControl], closed: bool) -> Result<SpiroShape, SpiroError> {
    let (points, corners) = split(controls);
    let solution = solve_corners(&points, &corners, closed, None)?;
    Ok(SpiroShape {
        segments: solution.coefficients,
    })
}

/// Solves a Spiro curve whose controls moved a little from a solved one: from the earlier
/// coefficients of each segment that has them (`near`, one entry per segment of `controls`) and a
/// straight start for the others, falling back to a fresh solve. The answer must stay near the
/// earlier shape (see [`SPIRO_MAX_TURN_CHANGE`]), so a drag never jumps to another solution or
/// grows a loop. With `fresh` false only the earlier shape is followed (cheap: a held point
/// tries several places per pointer event and needs the fresh solve only at the pointer itself).
///
/// # Errors
/// [`SpiroError::Far`] when the only solution found is far from `near`; the errors of
/// [`solve_spiro`] otherwise, and [`SpiroError::Invalid`] when `near` has the wrong length.
pub fn solve_spiro_near(
    controls: &[SpiroControl],
    closed: bool,
    near: &[Option<[f64; 4]>],
    fresh: bool,
) -> Result<SpiroShape, SpiroError> {
    let (points, corners) = split(controls);
    let segments = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    if near.len() != segments {
        return Err(SpiroError::Invalid);
    }
    let earlier: Vec<_> = near.iter().flatten().copied().collect();
    let moved = near.iter().all(Option::is_some);
    // A curve without a loop never gets one while its controls only move (lead decision
    // 2026-11-11, Proposal): a looping answer is no answer, so the point stays where the curve had
    // none. An added or removed control keeps the sweep limit of `stays_near`.
    let loopy = |solution: &Solution| most_sweep(&solution.coefficients) > std::f64::consts::PI;
    let loop_free = most_sweep(&earlier) <= std::f64::consts::PI;
    let seed: Vec<_> = near.iter().map(|k| k.unwrap_or([0.0; 4])).collect();
    let warm = solve_corners(&points, &corners, closed, Some(&seed))
        .ok()
        .filter(|w| !(moved && loop_free && loopy(w)) && stays_near(&w.coefficients, near));
    // A curve without a loop from the earlier shape is the answer; a fresh solve (the costly
    // part) competes only with a looping one.
    if let Some(w) = warm.as_ref().filter(|w| !fresh || !loopy(w)) {
        return Ok(SpiroShape {
            segments: w.coefficients.clone(),
        });
    }
    if !fresh {
        return Err(SpiroError::Far);
    }
    if moved && loop_free {
        // The controls only moved and the earlier branch, without a loop, has no solution near:
        // any other would be a jump.
        return Err(SpiroError::Far);
    }
    // Leaving a loop takes the plain curve Newton finds from straight segments (quick); a
    // control added or removed gets the whole search.
    let fresh = if moved {
        solve_corners(&points, &corners, closed, Some(&vec![[0.0; 4]; segments]))
    } else {
        solve_corners(&points, &corners, closed, None)
    };
    let best = match (warm, fresh) {
        (Some(w), Ok(f)) => {
            // Moved controls leave a loop only for a curve that sweeps clearly less; with a
            // control added or removed the curve that sweeps least is taken.
            let margin = if moved { SPIRO_UNLOOP } else { 0.0 };
            let fresh_fits = moved || stays_near(&f.coefficients, near);
            if fresh_fits && most_sweep(&f.coefficients) + margin < most_sweep(&w.coefficients) {
                f
            } else {
                w
            }
        }
        (Some(w), Err(_)) => w,
        (None, Ok(f))
            if stays_near(&f.coefficients, near)
                || (moved && most_sweep(&f.coefficients) + SPIRO_UNLOOP < most_sweep(&earlier)) =>
        {
            f
        }
        (None, Ok(_)) => return Err(SpiroError::Far),
        // Moved controls had a curve: none near it is a jump, whatever the quick solve said.
        (None, Err(_)) if moved => return Err(SpiroError::Far),
        (None, Err(failure)) => return Err(failure.into()),
    };
    Ok(SpiroShape {
        segments: best.coefficients,
    })
}

/// The cubic Bézier spans of a solved Spiro curve, within `tolerance` of it.
///
/// # Errors
/// [`SpiroError::Invalid`] when `shape` was not solved for this many segments; an approximation
/// error otherwise.
pub fn spiro_cubics(
    controls: &[SpiroControl],
    closed: bool,
    shape: &SpiroShape,
    tolerance: Tolerance,
) -> Result<Vec<SpiroCubic>, SpiroError> {
    let (points, _) = split(controls);
    let segments = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    if shape.segments.len() != segments || segments == 0 {
        return Err(SpiroError::Invalid);
    }
    let mut result = Vec::new();
    for (i, &ks) in shape.segments.iter().enumerate() {
        let spans = approximate_spiro_segment(
            points[i],
            points[(i + 1) % points.len()],
            ks,
            tolerance.as_mm(),
        )?;
        for (span, _) in spans {
            if span
                .iter()
                .any(|point| !point.0.is_finite() || !point.1.is_finite())
            {
                return Err(SpiroError::NoConvergence);
            }
            result.push(SpiroCubic {
                start: Point::new(span[0].0, span[0].1),
                control1: Point::new(span[1].0, span[1].1),
                control2: Point::new(span[2].0, span[2].1),
                end: Point::new(span[3].0, span[3].1),
            });
        }
    }
    Ok(result)
}

/// Evaluates an open or closed Spiro curve without changing its controls.
///
/// # Errors
/// Returns an error for invalid controls or when the curve cannot be evaluated.
pub fn evaluate_spiro(
    controls: &[SpiroControl],
    closed: bool,
    tolerance: Tolerance,
) -> Result<Vec<SpiroCubic>, SpiroError> {
    let shape = solve_spiro(controls, closed)?;
    spiro_cubics(controls, closed, &shape, tolerance)
}
