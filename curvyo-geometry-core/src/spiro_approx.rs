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
    let integrate = |lo: f64, hi: f64| integrate_spiral(&theta, lo, hi, n);
    let scale = chord.0.hypot(chord.1) / norm;
    let rot = chord.1.atan2(chord.0) - base.1.atan2(base.0);
    let position = |s: f64| {
        let p = integrate(-0.5, s);
        (
            start.0 + scale * (rot.cos() * p.0 - rot.sin() * p.1),
            start.1 + scale * (rot.sin() * p.0 + rot.cos() * p.1),
        )
    };
    let mut stack = vec![(-0.5_f64, 0.5_f64)];
    let mut spans = Vec::new();
    while let Some((lo, hi)) = stack.pop() {
        let p0 = position(lo);
        let p3 = position(hi);
        let h = (hi - lo) * scale / 3.0;
        let a = theta(lo) + rot;
        let b = theta(hi) + rot;
        let span = [
            p0,
            (p0.0 + h * a.cos(), p0.1 + h * a.sin()),
            (p3.0 - h * b.cos(), p3.1 - h * b.sin()),
            p3,
        ];
        // Cubic Hermite interpolation remainder in exact arithmetic.
        let error = 2.0 * scale * fourth_curve * (hi - lo).powi(4) / 384.0;
        if !error.is_finite() {
            return Err(Failure::NoConvergence);
        }
        if error <= tolerance * 0.5 {
            spans.push((span, (lo, hi)));
        } else {
            let mid = f64::midpoint(lo, hi);
            #[allow(clippy::float_cmp)] // Detect subdivision that cannot advance in f64.
            if mid == lo || mid == hi {
                return Err(Failure::NoConvergence);
            }
            stack.push((mid, hi));
            stack.push((lo, mid));
        }
    }
    Ok(spans)
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
