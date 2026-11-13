//! Newton's iteration on the curvature-continuous (G2) Spiro constraints of one smooth run, as
//! Inkscape draws its Spiro: clothoid segments between smooth knots, circular arcs at open ends.

use super::constraints::{Layout, maximum, segment_ends};
use super::linear::solve_linear;

/// Newton iterations of one solve: Levien's `solve_spiro` stops after ten, whatever the residual,
/// and its last iterate is the curve (`specs/0114`, customer 2026-11-12).
const ITERATIONS: usize = 10;
/// Bisections of a Newton step that leaves the drawable iterates, for the part of it that stays.
const BISECTIONS: usize = 40;
/// The squared length of a Newton step below which the iteration has converged (Levien's).
const CONVERGED: f64 = 1e-12;
/// The series-integral residual below which the remaining iterations use the accurate integral.
const POLISH: f64 = 1e-9;
/// The relative step of the central differences of the Jacobian.
const JACOBIAN_STEP: f64 = 1e-6;
/// The largest bend (Levien's `|k0| + |k1|/2 + |k2|/8 + |k3|/48`, radians) of a segment of a
/// drawable iterate: four turns. An iterate beyond it is finite but wild; the one before it is
/// the curve.
const MAX_BEND: f64 = 8.0 * std::f64::consts::PI;
/// The largest sweep of a segment of a drawable iterate: three quarters of a turn (Proposal). An
/// end arc that sweeps nearly a whole turn is a circle metres wide (the pointer just behind the
/// last control's tangent); the iterate before it is the curve.
const MAX_SWEEP: f64 = 1.5 * std::f64::consts::PI;

/// Where a run's unknowns sit: for each segment the index of its first unknown and how many it
/// has (two for a clothoid between smooth knots, one for an arc at an open end, none for a lone
/// straight segment).
struct Unknowns {
    first: Vec<usize>,
    count: Vec<usize>,
    total: usize,
}

impl Unknowns {
    fn new(segments: usize, closed: bool) -> Self {
        let count: Vec<usize> = (0..segments)
            .map(|i| {
                if closed {
                    2
                } else if segments == 1 {
                    0
                } else if i == 0 || i == segments - 1 {
                    1
                } else {
                    2
                }
            })
            .collect();
        let mut first = Vec::with_capacity(segments);
        let mut total = 0;
        for &c in &count {
            first.push(total);
            total += c;
        }
        Self {
            first,
            count,
            total,
        }
    }

    /// The four curvature coefficients of segment `i` for unknowns `x`.
    fn coefficients(&self, i: usize, x: &[f64]) -> [f64; 4] {
        let mut k = [0.0; 4];
        for (j, value) in k.iter_mut().take(self.count[i]).enumerate() {
            *value = x[self.first[i] + j];
        }
        k
    }

    /// Reduces each segment's turn (its first coefficient) to within two turns after every step
    /// (`ks[0] = 2 mod_2pi(ks[0] / 2)`), as libspiro's and Inkscape's `spiro_iter` do (the
    /// procedure only, no source taken).
    fn wrap_turns(&self, x: &mut [f64]) {
        for (&first, &count) in self.first.iter().zip(&self.count) {
            if count > 0 {
                let half = x[first] / 2.0;
                x[first] = 2.0
                    * (half - std::f64::consts::TAU * (half / std::f64::consts::TAU + 0.5).floor());
            }
        }
    }
}

/// The smooth knots of a run, whose angle and curvature are constrained: every knot of a closed
/// run, the inner ones of an open run.
fn constrained_knots(segments: usize, closed: bool) -> std::ops::Range<usize> {
    if closed { 0..segments } else { 1..segments }
}

/// The ends of every segment for unknowns `x`.
fn all_ends(
    points: &[(f64, f64)],
    unknowns: &Unknowns,
    x: &[f64],
    accurate: bool,
) -> Option<Vec<[[f64; 4]; 2]>> {
    (0..unknowns.count.len())
        .map(|i| segment_ends(points, i, &unknowns.coefficients(i, x), accurate))
        .collect()
}

/// The angle (reduced to one turn) and curvature errors at every smooth knot of a run whose
/// segments end as `ends`.
fn knot_errors(layout: &Layout, closed: bool, ends: &[[[f64; 4]; 2]]) -> Option<Vec<f64>> {
    let mut out = Vec::with_capacity(2 * ends.len());
    for knot in constrained_knots(ends.len(), closed) {
        out.extend(layout.knot(knot, true, ends));
    }
    out.iter().all(|v| v.is_finite()).then_some(out)
}

fn residual(
    points: &[(f64, f64)],
    layout: &Layout,
    unknowns: &Unknowns,
    closed: bool,
    x: &[f64],
    accurate: bool,
) -> Option<Vec<f64>> {
    knot_errors(layout, closed, &all_ends(points, unknowns, x, accurate)?)
}

/// The Jacobian of [`residual`] by central differences, one segment at a time: a segment's
/// unknowns only move the constraints at its two ends, so only those rows are computed. The
/// angle errors are differenced unreduced, so a difference never straddles the reduction.
fn jacobian(
    points: &[(f64, f64)],
    layout: &Layout,
    unknowns: &Unknowns,
    closed: bool,
    x: &[f64],
) -> Option<Vec<Vec<f64>>> {
    let n = x.len();
    let segments = unknowns.count.len();
    let mut matrix = vec![vec![0.0; n]; n];
    let mut ends = all_ends(points, unknowns, x, false)?;
    let first_knot = constrained_knots(segments, closed).start;
    for segment in 0..segments {
        let base = ends[segment];
        for j in 0..unknowns.count[segment] {
            let col = unknowns.first[segment] + j;
            let step = JACOBIAN_STEP * (1.0 + x[col].abs());
            let mut k = unknowns.coefficients(segment, x);
            k[j] += step;
            let plus = segment_ends(points, segment, &k, false)?;
            k[j] -= 2.0 * step;
            let minus = segment_ends(points, segment, &k, false)?;
            for knot in layout.knots_of(segment) {
                if !constrained_knots(segments, closed).contains(&knot) {
                    continue;
                }
                ends[segment] = plus;
                let high = layout.knot(knot, false, &ends);
                ends[segment] = minus;
                let low = layout.knot(knot, false, &ends);
                let row = 2 * (knot - first_knot);
                for r in 0..2 {
                    matrix[row + r][col] = (high[r] - low[r]) / (2.0 * step);
                }
            }
            ends[segment] = base;
        }
    }
    Some(matrix)
}

/// Levien's bend of a segment: a bound of how far its tangent turns.
fn bend(k: &[f64; 4]) -> f64 {
    k[0].abs() + k[1].abs() / 2.0 + k[2].abs() / 8.0 + k[3].abs() / 48.0
}

/// How far the tangent of a clothoid segment sweeps along it: its turn `k0 s + k1 s^2 / 2` over
/// `s` in `[-1/2, 1/2]` spans the values at both ends and, inside, at its turning point.
fn sweep(k: &[f64; 4]) -> f64 {
    let turn = |s: f64| k[0] * s + k[1] * s * s / 2.0;
    let mut values = [turn(-0.5), turn(0.5), turn(0.5)];
    if k[1] != 0.0 && (k[0] / k[1]).abs() < 0.5 {
        values[2] = turn(-k[0] / k[1]);
    }
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    high - low
}

/// Whether a segment of an iterate is one to draw: it bends at most [`MAX_BEND`] and sweeps at
/// most [`MAX_SWEEP`].
fn drawable(k: &[f64; 4]) -> bool {
    bend(k) <= MAX_BEND && sweep(k) <= MAX_SWEEP
}

/// The iterate of the full Newton step `delta` from `x`, or of the longest part of it whose
/// iterate stays drawable, found by bisection so the iterate moves on smoothly with the controls;
/// `None` when no part of it is drawable.
fn drawable_step(unknowns: &Unknowns, x: &[f64], delta: &[f64]) -> Option<Vec<f64>> {
    let at = |t: f64| {
        let mut next: Vec<f64> = x.iter().zip(delta).map(|(a, d)| a + t * d).collect();
        unknowns.wrap_turns(&mut next);
        let fits = (0..unknowns.count.len()).all(|i| drawable(&unknowns.coefficients(i, &next)));
        fits.then_some(next)
    };
    at(1.0).or_else(|| {
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..BISECTIONS {
            let middle = f64::midpoint(low, high);
            if at(middle).is_some() {
                low = middle;
            } else {
                high = middle;
            }
        }
        (low > 0.0).then(|| at(low)).flatten()
    })
}

/// The coefficients of one smooth run at unit scale (one set per segment), as Levien's
/// `solve_spiro`: from straight segments (the control polygon), at most [`ITERATIONS`] full
/// Newton steps on the constraints with each knot's angle error reduced to one turn and each
/// segment's turn to two, stopping once a step is shorter than [`CONVERGED`]. The last iterate is
/// the answer, converged or not. A step to an iterate that is not drawable (constraints not
/// finite, a segment bending more than [`MAX_BEND`] or sweeping more than [`MAX_SWEEP`]) is
/// cut to its longest drawable part; when there is none the iterate before it is the answer.
/// The answer is a function of the controls alone.
pub(super) fn solve_g2(points: &[(f64, f64)], closed: bool) -> Vec<[f64; 4]> {
    let segments = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let unknowns = Unknowns::new(segments, closed);
    let layout = Layout::new(points, closed);
    let mut x = vec![0.0; unknowns.total];
    let answer = |x: &[f64]| (0..segments).map(|i| unknowns.coefficients(i, x)).collect();
    if unknowns.total == 0 {
        return answer(&x);
    }
    let mut accurate = false;
    let Some(mut r) = residual(points, &layout, &unknowns, closed, &x, accurate) else {
        return answer(&x);
    };
    for _ in 0..ITERATIONS {
        if !accurate && maximum(&r) < POLISH {
            accurate = true;
            match residual(points, &layout, &unknowns, closed, &x, accurate) {
                Some(polished) => r = polished,
                None => break,
            }
        }
        let Some(matrix) = jacobian(points, &layout, &unknowns, closed, &x) else {
            break;
        };
        let Some(delta) = solve_linear(matrix, r.iter().map(|v| -v).collect()) else {
            break;
        };
        let next = drawable_step(&unknowns, &x, &delta);
        let step = next.and_then(|next| {
            residual(points, &layout, &unknowns, closed, &next, accurate).map(|r| (next, r))
        });
        let Some((next, next_r)) = step else {
            break;
        };
        x = next;
        r = next_r;
        if accurate && delta.iter().map(|d| d * d).sum::<f64>() < CONVERGED {
            break;
        }
    }
    answer(&x)
}
