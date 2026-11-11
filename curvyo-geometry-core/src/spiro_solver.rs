//! Solves Spiro interpolation constraints with residual validation.

mod constraints;
mod linear;
mod newton;

use constraints::residual;
use newton::{newton, newton_near};

#[derive(Debug)]
pub(super) struct Solution {
    pub coefficients: Vec<[f64; 4]>,
    pub max_residual: f64,
}

#[derive(Debug)]
pub(super) enum Failure {
    Invalid,
    NoConvergence,
}

/// Newton solves a continuation may spend before an open curve is given up.
const MAX_CONTINUATION_SOLVES: usize = 8;
/// The longest run the wound last resort is tried on; longer ones fail fast instead.
const MAX_WOUND_SEGMENTS: usize = 8;
/// The smallest continuation step; a smaller one gives the curve up.
const MIN_CONTINUATION_STEP: f64 = 1e-3;
/// Newton solves the growing of an open curve may spend before it is given up.
const MAX_GROWN_SOLVES: usize = 64;

/// Solves one smooth run. With a `seed` (one coefficient set per segment, from an earlier solve of
/// nearby controls) only Newton's iteration from it is tried, so the answer stays on the seed's
/// branch, and given up early; without one, from zero and then the fallbacks.
fn solve(
    controls: &[(f64, f64)],
    closed: bool,
    seed: Option<&[[f64; 4]]>,
) -> Result<Solution, Failure> {
    if controls.len() < if closed { 3 } else { 2 }
        || controls
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return Err(Failure::Invalid);
    }
    let (mut left, mut right, mut top, mut bottom) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in controls {
        left = left.min(x);
        right = right.max(x);
        top = top.min(y);
        bottom = bottom.max(y);
    }
    let scale = (right - left).max(bottom - top);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(Failure::Invalid);
    }
    let points: Vec<_> = controls
        .iter()
        .map(|&(x, y)| ((x - left) / scale, (y - top) / scale))
        .collect();
    let nseg = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    if nseg == 1 {
        return Ok(Solution {
            coefficients: vec![[0.0; 4]],
            max_residual: 0.0,
        });
    }
    if residual(&points, closed, &vec![0.0; 4 * nseg]).is_none() {
        return Err(Failure::Invalid);
    }
    if let Some(seed) = seed {
        // The coefficients are scale-free: a seed of moved controls is a start near the answer.
        let start: Vec<f64> = seed.iter().flatten().copied().collect();
        if start.len() != 4 * nseg {
            return Err(Failure::Invalid);
        }
        return newton_near(&points, closed, start);
    }
    // Levien's plain iteration from zero solves most curves in a few steps. Where its full steps
    // run away, an open curve is grown one control at a time (as the Pen draws it) and then
    // followed from its chord to its control polygon; a closed one gets steps that never raise
    // the residual.
    let zero = vec![0.0; 4 * nseg];
    newton(&points, closed, false, zero.clone(), false)
        .or_else(|_| {
            if closed {
                newton(&points, closed, false, zero, true)
            } else {
                grown(&points).or_else(|_| continuation(&points))
            }
        })
        .or_else(|_| wound(&points, closed))
}

/// Solves an open curve by adding its controls one at a time, each solve starting from the
/// previous one's coefficients and a straight new segment: the curve the Pen drew point by point.
fn grown(points: &[(f64, f64)]) -> Result<Solution, Failure> {
    let mut x = vec![0.0; 4];
    let mut solves = 0;
    for end in 3..=points.len() {
        x.extend([0.0; 4]);
        solves += 1;
        if solves > MAX_GROWN_SOLVES {
            return Err(Failure::NoConvergence);
        }
        let solution = newton(&points[..end], false, false, x, false)?;
        if end == points.len() {
            return Ok(solution);
        }
        x = solution.coefficients.into_iter().flatten().collect();
    }
    Err(Failure::NoConvergence)
}

/// The last resort for a short curve that has no solution without an extra turn at some knot (the
/// looping answers of the libspiro fork): Newton on the wrapped constraints from a few seeds that
/// bend the middle segments, keeping the answer with the smallest coefficients.
fn wound(points: &[(f64, f64)], closed: bool) -> Result<Solution, Failure> {
    let nseg = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    if nseg > MAX_WOUND_SEGMENTS {
        return Err(Failure::NoConvergence);
    }
    let cost = |solution: &Solution| {
        solution
            .coefficients
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0, f64::max)
    };
    [4.0_f64, 6.0, 8.0, -4.0, -6.0, -8.0]
        .into_iter()
        .filter_map(|outer| {
            let seed = (0..4 * nseg)
                .map(|i| {
                    let segment = i / 4;
                    let distance = count(segment.min(nseg - 1 - segment));
                    let normalized = 1.0 - 2.0 * distance / count(nseg - 1);
                    if i % 4 == 0 {
                        outer * (0.11 + 0.89 * normalized * normalized)
                    } else {
                        0.0
                    }
                })
                .collect();
            newton(points, closed, true, seed, false).ok()
        })
        .min_by(|a, b| cost(a).total_cmp(&cost(b)))
        .ok_or(Failure::NoConvergence)
}

/// Solves an open curve by moving its controls from the chord to their places in steps, each
/// solve starting from the previous one's coefficients; a step that fails is halved.
fn continuation(points: &[(f64, f64)]) -> Result<Solution, Failure> {
    let start = points[0];
    let end = points[points.len() - 1];
    let last = count(points.len() - 1);
    let blend = |amount: f64| -> Vec<(f64, f64)> {
        points
            .iter()
            .enumerate()
            .map(|(i, &(px, py))| {
                let t = count(i) / last;
                let base = (
                    start.0 + t * (end.0 - start.0),
                    start.1 + t * (end.1 - start.1),
                );
                (
                    base.0 + amount * (px - base.0),
                    base.1 + amount * (py - base.1),
                )
            })
            .collect()
    };
    let mut x = vec![0.0; 4 * (points.len() - 1)];
    let mut progress = 0.0_f64;
    let mut increment = 0.25_f64;
    let mut solves = 0;
    while progress < 1.0 {
        solves += 1;
        if solves > MAX_CONTINUATION_SOLVES {
            return Err(Failure::NoConvergence);
        }
        let next = (progress + increment).min(1.0);
        if let Ok(solution) = newton(&blend(next), false, false, x.clone(), false) {
            if next >= 1.0 {
                return Ok(solution);
            }
            x = solution.coefficients.into_iter().flatten().collect();
            progress = next;
            increment = (increment * 1.5).min(0.5);
        } else {
            increment *= 0.5;
            if increment < MIN_CONTINUATION_STEP {
                return Err(Failure::NoConvergence);
            }
        }
    }
    Err(Failure::NoConvergence)
}

/// G4 runs separated at corner controls, each solved as an open curve.
/// With a `seed` (one coefficient set per segment) each run is solved from its part of it only.
pub(super) fn solve_corners(
    points: &[(f64, f64)],
    corners: &[bool],
    closed: bool,
    seed: Option<&[[f64; 4]]>,
) -> Result<Solution, Failure> {
    if points.len() != corners.len()
        || points.len() < if closed { 3 } else { 2 }
        || seed.is_some_and(|seed| {
            seed.len()
                != if closed {
                    points.len()
                } else {
                    points.len() - 1
                }
        })
    {
        return Err(Failure::Invalid);
    }
    if !corners.iter().any(|corner| *corner) {
        return solve(points, closed, seed);
    }
    let segments = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut coefficients = vec![[0.0; 4]; segments];
    let mut max_residual = 0.0_f64;
    if closed {
        let first = corners
            .iter()
            .position(|corner| *corner)
            .ok_or(Failure::Invalid)?;
        let mut start = first;
        loop {
            let mut indices = vec![start];
            let mut next = (start + 1) % points.len();
            while next != first && !corners[next] {
                indices.push(next);
                next = (next + 1) % points.len();
            }
            indices.push(next);
            let run: Vec<_> = indices.iter().map(|&i| points[i]).collect();
            let run_seed: Option<Vec<_>> = seed.map(|seed| {
                indices[..indices.len() - 1]
                    .iter()
                    .map(|&i| seed[i])
                    .collect()
            });
            let result = solve(&run, false, run_seed.as_deref())?;
            max_residual = max_residual.max(result.max_residual);
            for (i, value) in indices.iter().zip(result.coefficients) {
                coefficients[*i] = value;
            }
            start = next;
            if start == first {
                break;
            }
        }
    } else {
        let mut start = 0;
        for end in 1..points.len() {
            if corners[end] || end == points.len() - 1 {
                let result = solve(
                    &points[start..=end],
                    false,
                    seed.map(|seed| &seed[start..end]),
                )?;
                max_residual = max_residual.max(result.max_residual);
                coefficients[start..end].copy_from_slice(&result.coefficients);
                start = end;
            }
        }
    }
    Ok(Solution {
        coefficients,
        max_residual,
    })
}

/// A segment or control count as a float: exact, the counts are far below 2^52.
#[allow(clippy::cast_precision_loss)]
const fn count(n: usize) -> f64 {
    n as f64
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
mod reference_tests {
    use super::*;

    #[test]
    fn matches_independent_open_and_closed_libspiro_coefficients() {
        for (closed, fixture) in [
            (
                false,
                include_str!("../tests/fixtures/spiro-libspiro-20220722-open.txt"),
            ),
            (
                true,
                include_str!("../tests/fixtures/spiro-libspiro-20220722-closed.txt"),
            ),
        ] {
            for (case, line) in fixture.lines().enumerate() {
                let points: Vec<_> = (0..5)
                    .map(|i| {
                        if closed {
                            let angle = std::f64::consts::TAU * i as f64 / 5.0;
                            let radius = 1.0 + 0.12 * ((case * 13 + i * 7) as f64).sin();
                            (radius * angle.cos(), radius * angle.sin())
                        } else {
                            (i as f64, ((case * 17 + i * 11) as f64).sin() * 0.3)
                        }
                    })
                    .collect();
                let expected: Vec<f64> = line
                    .split_whitespace()
                    .skip(1)
                    .map(|value| value.parse().unwrap())
                    .collect();
                let result = solve(&points, closed, None).unwrap();
                assert_eq!(result.coefficients.len() * 4, expected.len());
                for (actual, expected) in result.coefficients.iter().flatten().zip(expected) {
                    assert!(
                        (actual - expected).abs() < 1e-4,
                        "case {case}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}
