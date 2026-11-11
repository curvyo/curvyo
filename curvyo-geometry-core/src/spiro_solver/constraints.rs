//! The Spiro interpolation constraints of a curve at unit scale: their errors for given segment
//! coefficients.

use crate::spiro_approx::adaptive_integral;

/// A chord this short (unit scale) has no direction: its segment cannot be measured.
const CHORD_FLOOR: f64 = 1e-12;
/// The tolerance of the accurate integral that polishes a solution.
const ACCURATE_INTEGRAL_TOLERANCE: f64 = 1e-10;

/// Unit-scale smooth Spiro constraints: G4 at each smooth knot and natural open ends.
// The short names are those of Levien's formulas (`k` the coefficients, `l` the length ratio).
#[allow(clippy::many_single_char_names)]
pub(super) fn segment_ends(
    points: &[(f64, f64)],
    i: usize,
    parameters: &[f64],
    accurate: bool,
) -> Option<[[f64; 4]; 2]> {
    let a = points[i];
    let b = points[(i + 1) % points.len()];
    let chord = (b.0 - a.0).hypot(b.1 - a.1);
    if !chord.is_finite() || chord <= CHORD_FLOOR {
        return None;
    }
    let k = [parameters[0], parameters[1], parameters[2], parameters[3]];
    let xy = if accurate {
        adaptive_integral(k, ACCURATE_INTEGRAL_TOLERANCE)?
    } else {
        let mut value = [0.0; 2];
        spiro_inner::integrate_spiro(k, &mut value);
        (value[0], value[1])
    };
    let l = xy.0.hypot(xy.1) / chord;
    let th = xy.1.atan2(xy.0);
    let angle_even = 0.5 * k[0] + k[2] / 48.0;
    let angle_odd = 0.125 * k[1] + k[3] / 384.0 - th;
    let curvature_even = l * (k[0] + 0.125 * k[2]);
    let curvature_odd = l * (0.5 * k[1] + k[3] / 48.0);
    let derivative_even = l * l * (k[1] + 0.125 * k[3]);
    let derivative_odd = l * l * 0.5 * k[2];
    let second_even = l * l * l * k[2];
    let second_odd = l * l * l * 0.5 * k[3];
    let values = [
        [
            angle_even - angle_odd,
            curvature_even - curvature_odd,
            derivative_even - derivative_odd,
            second_even - second_odd,
        ],
        [
            angle_even + angle_odd,
            curvature_even + curvature_odd,
            derivative_even + derivative_odd,
            second_even + second_odd,
        ],
    ];
    values
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(values)
}

/// The constraint errors of a curve whose segments end as `ends`. With `wrapped`, a knot's angle
/// error is reduced to one turn, so a curve that winds a full turn more at a knot also solves.
pub(super) fn assemble_residual(
    points: &[(f64, f64)],
    closed: bool,
    wrapped: bool,
    ends: &[[[f64; 4]; 2]],
) -> Option<Vec<f64>> {
    let segments = ends.len();
    let angles: Vec<_> = (0..segments)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            (b.1 - a.1).atan2(b.0 - a.0)
        })
        .collect();
    let mut out = Vec::with_capacity(4 * segments);
    if !closed {
        out.push(ends[0][0][2]);
        out.push(ends[0][0][3]);
    }
    let first = usize::from(!closed);
    for knot in first..segments {
        let prev = (knot + segments - 1) % segments;
        let mut angle = angles[knot] - angles[prev];
        angle -= std::f64::consts::TAU * (angle / std::f64::consts::TAU + 0.5).floor();
        // As in Levien's `spiro_iter`: the bend is reduced to one turn, the error is not, so a
        // solution a full turn away is no solution unless `wrapped`.
        let delta = angle - ends[prev][1][0] - ends[knot][0][0];
        out.push(if wrapped {
            delta - std::f64::consts::TAU * (delta / std::f64::consts::TAU + 0.5).floor()
        } else {
            delta
        });
        for (start, end) in ends[knot][0][1..].iter().zip(&ends[prev][1][1..]) {
            out.push(start - end);
        }
    }
    if !closed {
        out.push(ends[segments - 1][1][2]);
        out.push(ends[segments - 1][1][3]);
    }
    (out.len() == 4 * segments && out.iter().all(|v| v.is_finite())).then_some(out)
}

pub(super) fn residual(
    points: &[(f64, f64)],
    closed: bool,
    parameters: &[f64],
) -> Option<Vec<f64>> {
    residual_mode(points, closed, false, parameters, true)
}

pub(super) fn residual_mode(
    points: &[(f64, f64)],
    closed: bool,
    wrapped: bool,
    parameters: &[f64],
    accurate: bool,
) -> Option<Vec<f64>> {
    let segments = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let ends: Option<Vec<_>> = (0..segments)
        .map(|i| segment_ends(points, i, &parameters[4 * i..4 * i + 4], accurate))
        .collect();
    assemble_residual(points, closed, wrapped, &ends?)
}

pub(super) fn maximum(values: &[f64]) -> f64 {
    values.iter().map(|v| v.abs()).fold(0.0, f64::max)
}
pub(super) fn energy(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum()
}
