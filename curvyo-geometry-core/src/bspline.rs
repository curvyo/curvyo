//! Evaluates clamped or periodic B-spline controls into cubic Bézier spans.

use curvyo_document_core::Point;

/// A B-spline control position and whether it breaks tangent continuity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineControl {
    /// The stored control position.
    pub point: Point,
    /// Whether the curve passes through this control as a corner.
    pub corner: bool,
}

/// One evaluated cubic span in traversal order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineCubic {
    /// The first endpoint.
    pub start: Point,
    /// The first Bézier control.
    pub control1: Point,
    /// The second Bézier control.
    pub control2: Point,
    /// The final endpoint.
    pub end: Point,
}

/// A B-spline cannot be evaluated from these controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SplineError {
    /// A coordinate is not finite.
    #[error("B-spline controls must be finite")]
    NonFinite,
    /// A closed B-spline needs at least three controls.
    #[error("a closed B-spline needs at least three controls")]
    TooFewClosed,
    /// The evaluated geometry exceeds the representable coordinate range.
    #[error("B-spline geometry is outside the numeric range")]
    OutOfRange,
}

/// Evaluates open clamped or closed periodic B-splines, splitting at corners.
///
/// # Errors
/// Returns an error for nonfinite controls or too few controls in a closed curve.
pub fn evaluate_bspline(
    controls: &[SplineControl],
    closed: bool,
) -> Result<Vec<SplineCubic>, SplineError> {
    let spans = evaluate(controls, closed)?;
    if spans.iter().any(|span| {
        [span.start, span.control1, span.control2, span.end]
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
    }) {
        return Err(SplineError::OutOfRange);
    }
    Ok(spans)
}

fn evaluate(controls: &[SplineControl], closed: bool) -> Result<Vec<SplineCubic>, SplineError> {
    if controls
        .iter()
        .any(|c| !c.point.x.is_finite() || !c.point.y.is_finite())
    {
        return Err(SplineError::NonFinite);
    }
    if closed && controls.len() < 3 {
        return Err(SplineError::TooFewClosed);
    }
    if closed {
        let corners: Vec<usize> = controls
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.corner.then_some(i))
            .collect();
        if corners.is_empty() {
            return Ok(periodic(controls));
        }
        let mut result = Vec::new();
        for (i, &start) in corners.iter().enumerate() {
            let end = corners[(i + 1) % corners.len()];
            let mut run = vec![controls[start].point];
            let mut current = (start + 1) % controls.len();
            while current != end {
                run.push(controls[current].point);
                current = (current + 1) % controls.len();
            }
            run.push(controls[end].point);
            result.extend(open(&run));
        }
        return Ok(result);
    }
    let mut result = Vec::new();
    let mut start = 0;
    for i in 1..controls.len().saturating_sub(1) {
        if controls[i].corner {
            result.extend(open(
                &controls[start..=i]
                    .iter()
                    .map(|c| c.point)
                    .collect::<Vec<_>>(),
            ));
            start = i;
        }
    }
    result.extend(open(
        &controls[start..]
            .iter()
            .map(|c| c.point)
            .collect::<Vec<_>>(),
    ));
    Ok(result)
}

fn weighted(a: Point, wa: f64, b: Point, wb: f64, c: Point, wc: f64) -> Point {
    Point::new(
        a.x * wa + b.x * wb + c.x * wc,
        a.y * wa + b.y * wb + c.y * wc,
    )
}

fn periodic(controls: &[SplineControl]) -> Vec<SplineCubic> {
    let count = controls.len();
    (0..count)
        .map(|i| {
            let previous = controls[(i + count - 1) % count].point;
            let current = controls[i].point;
            let next = controls[(i + 1) % count].point;
            let following = controls[(i + 2) % count].point;
            SplineCubic {
                start: weighted(previous, 1.0 / 6.0, current, 4.0 / 6.0, next, 1.0 / 6.0),
                control1: weighted(current, 2.0 / 3.0, next, 1.0 / 3.0, following, 0.0),
                control2: weighted(current, 1.0 / 3.0, next, 2.0 / 3.0, following, 0.0),
                end: weighted(current, 1.0 / 6.0, next, 4.0 / 6.0, following, 1.0 / 6.0),
            }
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)] // Knot indices are bounded by the in-memory control count.
fn open(points: &[Point]) -> Vec<SplineCubic> {
    if points.len() < 2 {
        return Vec::new();
    }
    let degree = (points.len() - 1).min(3);
    let count = points.len();
    let spans = count - degree;
    let mut knots = vec![0.0; count + degree + 1];
    for (i, knot) in knots.iter_mut().enumerate().skip(degree + 1) {
        *knot = (i - degree).min(spans) as f64;
    }
    (degree..count)
        .map(|span| {
            let a = knots[span];
            let b = knots[span + 1];
            let p0 = de_boor(points, &knots, degree, span, a);
            let p1 = de_boor(points, &knots, degree, span, a + (b - a) / 3.0);
            let p2 = de_boor(points, &knots, degree, span, a + 2.0 * (b - a) / 3.0);
            let p3 = de_boor(points, &knots, degree, span, b);
            let q1 = weighted(p1, 27.0, p0, -8.0, p3, -1.0);
            let q2 = weighted(p2, 27.0, p0, -1.0, p3, -8.0);
            SplineCubic {
                start: p0,
                control1: weighted(q1, 1.0 / 9.0, q2, -1.0 / 18.0, p0, 0.0),
                control2: weighted(q2, 1.0 / 9.0, q1, -1.0 / 18.0, p0, 0.0),
                end: p3,
            }
        })
        .collect()
}

fn de_boor(points: &[Point], knots: &[f64], degree: usize, span: usize, t: f64) -> Point {
    let mut values: Vec<Point> = points[span - degree..=span].to_vec();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = knots[i + degree - r + 1] - knots[i];
            let alpha = if denominator > 0.0 {
                (t - knots[i]) / denominator
            } else {
                0.0
            };
            values[j] = weighted(values[j - 1], 1.0 - alpha, values[j], alpha, values[j], 0.0);
        }
    }
    values[degree]
}
