//! Angles and their sines from `+ − × ÷` and `sqrt` only, so the shape fit of Trace gives the
//! same bits natively and in wasm (`specs/0088-trace-recognition` Part B criterion 13; the rule of
//! `specs/0089` adrs.md §8: no platform `atan2`, `sin` or `cos`).

// The series' own names (r, s, c, a, b) read better than long ones here.
#![allow(clippy::many_single_char_names)]

use std::f64::consts::{FRAC_PI_2, PI};

/// Terms of the arctangent series after two halvings (the argument is then at most 0.2, so the
/// next term is below 1e-18).
const ATAN_TERMS: u32 = 12;

/// The angle of (`x`, `y`) from the x axis in (−π, π]; 0 at the origin.
pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let octant = if ay <= ax {
        atan_unit(ay / ax)
    } else {
        FRAC_PI_2 - atan_unit(ax / ay)
    };
    let half = if x >= 0.0 { octant } else { PI - octant };
    if y < 0.0 { -half } else { half }
}

/// The arctangent of `z` in [0, 1]: two halvings (`atan z = 2 atan(z / (1 + √(1 + z²)))`) and
/// the series.
fn atan_unit(z: f64) -> f64 {
    let half = |z: f64| z / (1.0 + (1.0 + z * z).sqrt());
    let z = half(half(z));
    let z2 = z * z;
    let mut power = z;
    let mut sum = 0.0;
    for k in 0..ATAN_TERMS {
        let term = power / f64::from(2 * k + 1);
        sum = if k % 2 == 0 { sum + term } else { sum - term };
        power *= z2;
    }
    4.0 * sum
}

/// The sine and cosine of `angle` (radians, a few turns at most): the quarter turn nearest to it
/// and the Taylor series of the rest (at most π/4, so the next term is below 1e-19).
pub(crate) fn sin_cos(angle: f64) -> (f64, f64) {
    let quarters = (angle / FRAC_PI_2).round();
    let r = angle - quarters * FRAC_PI_2;
    let r2 = r * r;
    // sin r = r − r³/3! + …, cos r = 1 − r²/2! + …, both to the 18th power.
    let (mut s, mut c) = (0.0, 0.0);
    let (mut s_term, mut c_term) = (r, 1.0);
    for k in 0..10_u32 {
        s += s_term;
        c += c_term;
        let (a, b) = (f64::from(2 * k + 2), f64::from(2 * k + 3));
        s_term *= -r2 / (a * b);
        c_term *= -r2 / ((a - 1.0) * a);
    }
    // invariant: `quarters` is a whole number of a few turns at most.
    #[allow(clippy::cast_possible_truncation)]
    match (quarters as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atan2_agrees_with_std_everywhere() {
        for i in 0..720 {
            let a = f64::from(i) * PI / 360.0 - PI + 1e-3;
            for r in [1e-6, 0.5, 3.0, 1e6] {
                let (y, x) = (r * a.sin(), r * a.cos());
                let ours = atan2(y, x);
                assert!((ours - y.atan2(x)).abs() < 1e-14, "{a}: {ours}");
            }
        }
        assert_eq!(atan2(0.0, 0.0), 0.0);
        assert!((atan2(0.0, -1.0) - PI).abs() < 1e-15);
        assert!((atan2(-1.0, 0.0) + FRAC_PI_2).abs() < 1e-15);
    }

    #[test]
    fn sin_cos_agrees_with_std_for_some_turns() {
        for i in -2000..2000 {
            let a = f64::from(i) * 0.01;
            let (s, c) = sin_cos(a);
            assert!((s - a.sin()).abs() < 1e-14, "{a}");
            assert!((c - a.cos()).abs() < 1e-14, "{a}");
        }
    }
}
