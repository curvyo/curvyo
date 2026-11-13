//! Solves the Spiro interpolation constraints of a curve's smooth runs.

mod clothoid;
mod constraints;
mod linear;

use clothoid::solve_g2;

#[derive(Debug)]
pub(super) enum Failure {
    Invalid,
    NoConvergence,
}

/// Two controls closer than this (in units of the run's size) are one point: the segment between
/// them has no direction and is drawn as that point.
const COINCIDENT: f64 = 1e-9;

/// The coefficients of one smooth run (one set per segment). Coincident neighbours are one
/// control to the solve and their segment is a point (straight, zero long), so a point placed on
/// another never leaves the curve without a direction (Inkscape offsets such points by a hair
/// instead). Always an answer for finite controls: see [`solve_g2`].
fn solve(controls: &[(f64, f64)], closed: bool) -> Result<Vec<[f64; 4]>, Failure> {
    if controls.len() < if closed { 3 } else { 2 }
        || controls
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return Err(Failure::Invalid);
    }
    let n = controls.len();
    let segments = if closed { n } else { n - 1 };
    let mut coefficients = vec![[0.0; 4]; segments];
    let Some(points) = unit_scale(controls) else {
        // Every control on one spot: the curve is that point.
        return Ok(coefficients);
    };
    let apart = |i: usize| {
        let (a, b) = (points[i], points[(i + 1) % n]);
        (b.0 - a.0).hypot(b.1 - a.1) > COINCIDENT
    };
    let kept: Vec<usize> = (0..segments).filter(|&i| apart(i)).collect();
    let mut run: Vec<_> = kept.iter().map(|&i| points[i]).collect();
    if !closed && let Some(&last) = kept.last() {
        run.push(points[last + 1]);
    }
    if run.len() < if closed { 3 } else { 2 } {
        return Ok(coefficients);
    }
    for (&segment, k) in kept.iter().zip(solve_g2(&run, closed)) {
        coefficients[segment] = k;
    }
    Ok(coefficients)
}

/// The controls about their centroid in units of their root mean square distance from it, `None`
/// when they all lie on one spot. The unit does not change when the controls move, turn or scale,
/// so neither do the solver's absolute thresholds (its curvature errors scale with the unit).
fn unit_scale(controls: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let n = count(controls.len());
    let (cx, cy) = controls
        .iter()
        .fold((0.0, 0.0), |(sx, sy), &(x, y)| (sx + x / n, sy + y / n));
    let scale = (controls
        .iter()
        .map(|&(x, y)| (x - cx).powi(2) + (y - cy).powi(2))
        .sum::<f64>()
        / n)
        .sqrt();
    (scale.is_finite() && scale > 0.0).then(|| {
        controls
            .iter()
            .map(|&(x, y)| ((x - cx) / scale, (y - cy) / scale))
            .collect()
    })
}

/// The indices of the controls of each smooth run between corner controls, in order (a corner
/// is an open end to the segments beside it, as libspiro's `v` point); one closed run of all
/// controls when there is no corner.
fn runs(corners: &[bool], closed: bool) -> Vec<Vec<usize>> {
    let n = corners.len();
    let Some(first) = corners.iter().position(|corner| *corner) else {
        return vec![(0..n).collect()];
    };
    let (start, end) = if closed {
        (first, first + n)
    } else {
        (0, n - 1)
    };
    let mut out = Vec::new();
    let mut run = vec![start % n];
    for i in start + 1..=end {
        run.push(i % n);
        if corners[i % n] || i == end {
            out.push(std::mem::replace(&mut run, vec![i % n]));
        }
    }
    out
}

/// The coefficients of every segment of a curve (one set per segment; for a closed curve the
/// last one closes it): its smooth runs between corners, each solved as an open curve, or the
/// whole curve as one run without a corner.
pub(super) fn solve_corners(
    points: &[(f64, f64)],
    corners: &[bool],
    closed: bool,
) -> Result<Vec<[f64; 4]>, Failure> {
    if points.len() != corners.len() || points.len() < if closed { 3 } else { 2 } {
        return Err(Failure::Invalid);
    }
    if !corners.iter().any(|corner| *corner) {
        return solve(points, closed);
    }
    let segments = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut coefficients = vec![[0.0; 4]; segments];
    for indices in runs(corners, closed) {
        let run: Vec<_> = indices.iter().map(|&i| points[i]).collect();
        for (&i, k) in indices.iter().zip(solve(&run, false)?) {
            coefficients[i] = k;
        }
    }
    Ok(coefficients)
}

/// A segment or control count as a float: exact, the counts are far below 2^52.
#[allow(clippy::cast_precision_loss)]
const fn count(n: usize) -> f64 {
    n as f64
}
