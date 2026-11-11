//! Newton's iteration on the Spiro constraints, with its finite-difference Jacobian.

use super::constraints::{assemble_residual, energy, maximum, residual_mode, segment_ends};
use super::linear::solve_linear;
use super::{Failure, Solution};

/// Newton iterations before a curve is given up (libspiro needs about ten from zero).
const MAX_ITERATIONS: usize = 16;
/// A squared residual this large means the iteration runs away; it is given up at once.
const RUNAWAY: f64 = 1e6;
/// Step halvings tried when a full Newton step does not lower the residual.
const MAX_HALVINGS: usize = 12;
/// The relative step of the central differences of the Jacobian.
const JACOBIAN_STEP: f64 = 1e-6;
/// The residual every constraint must reach, in unit-scale radians and curvatures.
const RESIDUAL: f64 = 1e-8;

/// The constraint Jacobian at `x` by central differences, one segment at a time: a segment's
/// four coefficients only move the constraints at its two ends.
fn jacobian(
    points: &[(f64, f64)],
    closed: bool,
    wrapped: bool,
    x: &[f64],
) -> Option<Vec<Vec<f64>>> {
    let n = x.len();
    let mut matrix = vec![vec![0.0; n]; n];
    let base: Vec<_> = (0..n / 4)
        .map(|i| segment_ends(points, i, &x[4 * i..4 * i + 4], false))
        .collect::<Option<_>>()?;
    for col in 0..n {
        let step = JACOBIAN_STEP * (1.0 + x[col].abs());
        let segment = col / 4;
        let mut ends = base.clone();
        let mut shifted = [
            x[4 * segment],
            x[4 * segment + 1],
            x[4 * segment + 2],
            x[4 * segment + 3],
        ];
        shifted[col % 4] += step;
        ends[segment] = segment_ends(points, segment, &shifted, false)?;
        let plus = assemble_residual(points, closed, wrapped, &ends)?;
        shifted[col % 4] -= 2.0 * step;
        ends[segment] = segment_ends(points, segment, &shifted, false)?;
        let minus = assemble_residual(points, closed, wrapped, &ends)?;
        for row in 0..n {
            matrix[row][col] = (plus[row] - minus[row]) / (2.0 * step);
        }
    }
    Some(matrix)
}

/// Levien's iteration: Newton steps from `x` on the series integral, then polished against the
/// accurate integral with the same Jacobian. A step is halved while its residual is not finite,
/// and with `damped` also while it does not lower the residual.
pub(super) fn newton(
    points: &[(f64, f64)],
    closed: bool,
    wrapped: bool,
    mut x: Vec<f64>,
    damped: bool,
) -> Result<Solution, Failure> {
    let mut accurate = false;
    let mut r = residual_mode(points, closed, wrapped, &x, accurate).ok_or(Failure::Invalid)?;
    for _ in 0..MAX_ITERATIONS {
        if maximum(&r) < RESIDUAL {
            if accurate {
                return Ok(Solution {
                    coefficients: x
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|a| [a[0], a[1], a[2], a[3]])
                        .collect(),
                    max_residual: maximum(&r),
                });
            }
            accurate = true;
            r = residual_mode(points, closed, wrapped, &x, accurate)
                .ok_or(Failure::NoConvergence)?;
            continue;
        }
        let matrix = jacobian(points, closed, wrapped, &x).ok_or(Failure::NoConvergence)?;
        let rhs: Vec<f64> = r.iter().map(|v| -v).collect();
        let delta = solve_linear(matrix, rhs).ok_or(Failure::NoConvergence)?;
        let old = energy(&r);
        let mut step = 1.0;
        let mut accepted = None;
        for _ in 0..MAX_HALVINGS {
            let candidate: Vec<f64> = x.iter().zip(&delta).map(|(a, d)| a + step * d).collect();
            if let Some(next) = residual_mode(points, closed, wrapped, &candidate, accurate)
                && (!damped || energy(&next) < old)
            {
                accepted = Some((candidate, next));
                break;
            }
            step *= 0.5;
        }
        let (candidate, next) = accepted.ok_or(Failure::NoConvergence)?;
        if energy(&next) > RUNAWAY {
            return Err(Failure::NoConvergence);
        }
        x = candidate;
        r = next;
    }
    Err(Failure::NoConvergence)
}
