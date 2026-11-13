//! Approximates polynomial spiral segments with cubic Bézier spans.

#![allow(clippy::many_single_char_names, clippy::cast_precision_loss)]

use crate::spiro_solver::Failure;

type ApproximateSpan = ([(f64, f64); 4], (f64, f64));

/// Experimental analytic error budgets in exact arithmetic; floating-point bounds remain unproved.
pub(super) fn approximate_spiro_segment(
    start: (f64, f64),
    end: (f64, f64),
    ks: [f64; 4],
    tolerance: f64,
) -> Result<Vec<ApproximateSpan>, Failure> {
    if !tolerance.is_finite()
        || tolerance <= 0.0
        || ks
            .iter()
            .chain([start.0, start.1, end.0, end.1].iter())
            .any(|x| !x.is_finite())
    {
        return Err(Failure::Invalid);
    }
    let coordinate_scale = start
        .0
        .abs()
        .max(start.1.abs())
        .max(end.0.abs())
        .max(end.1.abs())
        .max(1.0);
    if tolerance <= 16.0 * f64::EPSILON * coordinate_scale {
        return Err(Failure::NoConvergence);
    }
    let theta = |s: f64| {
        ks[0] * s + ks[1] * s * s / 2.0 + ks[2] * s.powi(3) / 6.0 + ks[3] * s.powi(4) / 24.0
    };
    // Bounds for curvature and its first three derivatives on [-1/2, 1/2].
    let k = ks[0].abs() + ks[1].abs() / 2.0 + ks[2].abs() / 8.0 + ks[3].abs() / 48.0;
    let k1 = ks[1].abs() + ks[2].abs() / 2.0 + ks[3].abs() / 8.0;
    let k2 = ks[2].abs() + ks[3].abs() / 2.0;
    let k3 = ks[3].abs();
    let fourth_integrand = k.powi(4) + 6.0 * k * k * k1 + 3.0 * k1 * k1 + 4.0 * k * k2 + k3;
    let fourth_curve = k.powi(3) + 3.0 * k * k1 + k2;
    if !fourth_integrand.is_finite() || !fourth_curve.is_finite() {
        return Err(Failure::NoConvergence);
    }
    let chord = (end.0 - start.0, end.1 - start.1);
    let chord_len = chord.0.hypot(chord.1);
    if !chord_len.is_finite() || chord_len == 0.0 {
        return Err(Failure::Invalid);
    }
    let mut n = 128_usize;
    let (base, norm) = loop {
        let value = integrate_spiral(&theta, -0.5, 0.5, n);
        let norm = value.0.hypot(value.1);
        if norm <= 64.0 * f64::EPSILON {
            return Err(Failure::NoConvergence);
        }
        let e = 2.0 * fourth_integrand / (180.0 * (n as f64).powi(4));
        if norm.is_finite() && norm > e {
            let error = chord_len * (e / (norm - e) + (1.0 + e) * e / (norm * (norm - e))) * 4.0;
            if error.is_finite() && error <= tolerance * 0.25 {
                break (value, norm);
            }
        }
        n = n.checked_mul(2).ok_or(Failure::NoConvergence)?;
        if n as f64 > 1.0 / f64::EPSILON.sqrt() {
            return Err(Failure::NoConvergence);
        }
    };
    let scale = chord.0.hypot(chord.1) / norm;
    let rot = chord.1.atan2(chord.0) - base.1.atan2(base.0);
    let (sin, cos) = rot.sin_cos();
    let place = |p: (f64, f64)| {
        (
            start.0 + scale * (cos * p.0 - sin * p.1),
            start.1 + scale * (sin * p.0 + cos * p.1),
        )
    };
    let depth = span_depth(scale * fourth_curve, tolerance)?;
    let pieces = 1_usize << depth;
    let width = 1.0 / pieces as f64;
    // Each piece is integrated on its own at the base step (an even count of at least two
    // Simpson intervals) and the pieces summed, so every span end is integrated once.
    let steps = (n / pieces).max(2);
    let mut spans = Vec::with_capacity(pieces.min(1 << 16));
    let mut sum = (0.0, 0.0);
    let mut p0 = place((0.0, 0.0));
    for i in 0..pieces {
        let lo = -0.5 + i as f64 * width;
        let hi = if i + 1 == pieces { 0.5 } else { lo + width };
        let piece = integrate_spiral(&theta, lo, hi, steps);
        sum = (sum.0 + piece.0, sum.1 + piece.1);
        let p3 = place(sum);
        let h = (hi - lo) * scale / 3.0;
        let (a_sin, a_cos) = (theta(lo) + rot).sin_cos();
        let (b_sin, b_cos) = (theta(hi) + rot).sin_cos();
        let span = [
            p0,
            (p0.0 + h * a_cos, p0.1 + h * a_sin),
            (p3.0 - h * b_cos, p3.1 - h * b_sin),
            p3,
        ];
        spans.push((span, (lo, hi)));
        p0 = p3;
    }
    Ok(spans)
}

/// The most halvings of a segment into spans: past them a piece is narrower than the f64
/// spacing near the segment's ends.
const MAX_DEPTH: i32 = 52;

/// How often a segment is halved into spans for `tolerance`: the cubic Hermite remainder of a
/// span (in exact arithmetic, `bound` the segment's scale times its fourth-derivative bound)
/// depends on its width alone, so the spans are 2^depth equal pieces.
fn span_depth(bound: f64, tolerance: f64) -> Result<i32, Failure> {
    for depth in 0..=MAX_DEPTH {
        let error = 2.0 * bound * 0.5_f64.powi(4 * depth) / 384.0;
        if !error.is_finite() {
            return Err(Failure::NoConvergence);
        }
        if error <= tolerance * 0.5 {
            return Ok(depth);
        }
    }
    Err(Failure::NoConvergence)
}

fn integrate_spiral(theta: &impl Fn(f64) -> f64, lo: f64, hi: f64, n: usize) -> (f64, f64) {
    let h = (hi - lo) / n as f64;
    let (mut x, mut y) = (0.0, 0.0);
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..=n {
        let angle = theta(lo + h * i as f64);
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 0 {
            2.0
        } else {
            4.0
        };
        let vx = w * angle.cos() - cx;
        let vy = w * angle.sin() - cy;
        let next_x = x + vx;
        let next_y = y + vy;
        cx = (next_x - x) - vx;
        cy = (next_y - y) - vy;
        x = next_x;
        y = next_y;
    }
    (x * h / 3.0, y * h / 3.0)
}

/// Adaptive Simpson quadrature with a fourth-derivative truncation budget.
pub(super) fn adaptive_integral(ks: [f64; 4], tolerance: f64) -> Option<(f64, f64)> {
    if !tolerance.is_finite() || tolerance <= 0.0 || ks.iter().any(|k| !k.is_finite()) {
        return None;
    }
    let k = ks[0].abs() + ks[1].abs() / 2.0 + ks[2].abs() / 8.0 + ks[3].abs() / 48.0;
    let k1 = ks[1].abs() + ks[2].abs() / 2.0 + ks[3].abs() / 8.0;
    let k2 = ks[2].abs() + ks[3].abs() / 2.0;
    let bound = k.powi(4) + 6.0 * k * k * k1 + 3.0 * k1 * k1 + 4.0 * k * k2 + ks[3].abs();
    if !bound.is_finite() {
        return None;
    }
    let mut n = 16_usize;
    while 2.0 * bound / (180.0 * (n as f64).powi(4)) > tolerance {
        n = n.checked_mul(2)?;
        if n as f64 > 1.0 / f64::EPSILON.sqrt() {
            return None;
        }
    }
    let theta = |s: f64| {
        ks[0] * s + ks[1] * s * s / 2.0 + ks[2] * s.powi(3) / 6.0 + ks[3] * s.powi(4) / 24.0
    };
    let result = integrate_spiral(&theta, -0.5, 0.5, n);
    (result.0.is_finite() && result.1.is_finite()).then_some(result)
}
