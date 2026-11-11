//! Evaluates editable Spiro controls into cubic Bézier spans.

use crate::spiro_approx::approximate_spiro_segment;
use crate::spiro_solver::{Failure, solve_corners};
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
}
impl From<Failure> for SpiroError {
    fn from(value: Failure) -> Self {
        match value {
            Failure::Invalid => Self::Invalid,
            Failure::NoConvergence => Self::NoConvergence,
        }
    }
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
    let points: Vec<_> = controls.iter().map(|c| (c.point.x, c.point.y)).collect();
    let corners: Vec<_> = controls.iter().map(|c| c.corner).collect();
    let solution = solve_corners(&points, &corners, closed)?;
    let mut result = Vec::new();
    for (i, ks) in solution.coefficients.into_iter().enumerate() {
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
