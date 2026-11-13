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
    /// Too few or non-finite controls, a shape of another curve, or a tolerance too fine for the
    /// coordinates.
    #[error("Spiro controls or tolerance are invalid")]
    Invalid,
}

impl From<Failure> for SpiroError {
    fn from(_: Failure) -> Self {
        Self::Invalid
    }
}

/// The solved shape of a Spiro curve: the four curvature coefficients of each segment, in the
/// segment's own scale-free terms.
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

/// Solves an open or closed Spiro curve: a function of the controls alone, so what is shown
/// while they move is what is stored and what a reopen draws (`specs/0115`). A run whose
/// iteration does not converge is drawn as its last iterate, as Inkscape does.
///
/// # Errors
/// [`SpiroError::Invalid`] for too few or non-finite controls.
pub fn solve_spiro(controls: &[SpiroControl], closed: bool) -> Result<SpiroShape, SpiroError> {
    let points: Vec<_> = controls.iter().map(|c| (c.point.x, c.point.y)).collect();
    let corners: Vec<_> = controls.iter().map(|c| c.corner).collect();
    Ok(SpiroShape {
        segments: solve_corners(&points, &corners, closed)?,
    })
}

/// The cubic Bézier spans of a solved Spiro curve, within `tolerance` of it. A straight segment
/// is one straight span; a shape with a segment that cannot be approximated (a wild last iterate)
/// is drawn as its control polygon, so a Spiro of finite controls is always drawn.
///
/// # Errors
/// [`SpiroError::Invalid`] when `shape` was not solved for this many controls or `tolerance` is
/// not finite, not positive or too fine for the controls' coordinates.
pub fn spiro_cubics(
    controls: &[SpiroControl],
    closed: bool,
    shape: &SpiroShape,
    tolerance: Tolerance,
) -> Result<Vec<SpiroCubic>, SpiroError> {
    let points: Vec<_> = controls.iter().map(|c| (c.point.x, c.point.y)).collect();
    let segments = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    let coordinates = points
        .iter()
        .flat_map(|&(x, y)| [x.abs(), y.abs()])
        .fold(1.0, f64::max);
    let tolerance = tolerance.as_mm();
    if shape.segments.len() != segments
        || segments == 0
        || !coordinates.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 16.0 * f64::EPSILON * coordinates
    {
        return Err(SpiroError::Invalid);
    }
    let ends = |i: usize| (points[i], points[(i + 1) % points.len()]);
    // A straight span with its handles at the thirds, as the approximation draws a straight
    // segment.
    let straight = |(a, b): ((f64, f64), (f64, f64))| SpiroCubic {
        start: Point::new(a.0, a.1),
        control1: Point::new(a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0),
        control2: Point::new(b.0 - (b.0 - a.0) / 3.0, b.1 - (b.1 - a.1) / 3.0),
        end: Point::new(b.0, b.1),
    };
    let mut result = Vec::new();
    for (i, &ks) in shape.segments.iter().enumerate() {
        // A segment the solver left straight has exactly zero coefficients.
        #[allow(clippy::float_cmp)]
        let straight_segment = ks == [0.0; 4];
        if straight_segment {
            result.push(straight(ends(i)));
            continue;
        }
        let (start, end) = ends(i);
        let spans = approximate_spiro_segment(start, end, ks, tolerance)
            .ok()
            .filter(|spans| {
                spans
                    .iter()
                    .flat_map(|(span, _)| span)
                    .all(|point| point.0.is_finite() && point.1.is_finite())
            });
        let Some(spans) = spans else {
            return Ok((0..segments).map(|i| straight(ends(i))).collect());
        };
        result.extend(spans.into_iter().map(|(span, _)| SpiroCubic {
            start: Point::new(span[0].0, span[0].1),
            control1: Point::new(span[1].0, span[1].1),
            control2: Point::new(span[2].0, span[2].1),
            end: Point::new(span[3].0, span[3].1),
        }));
    }
    Ok(result)
}

/// Evaluates an open or closed Spiro curve without changing its controls.
///
/// # Errors
/// [`SpiroError::Invalid`] as for [`solve_spiro`] and [`spiro_cubics`].
pub fn evaluate_spiro(
    controls: &[SpiroControl],
    closed: bool,
    tolerance: Tolerance,
) -> Result<Vec<SpiroCubic>, SpiroError> {
    let shape = solve_spiro(controls, closed)?;
    spiro_cubics(controls, closed, &shape, tolerance)
}
