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

/// Where the constraints of a curve sit in its residual, and the bend of its chords at each knot.
pub(super) struct Layout {
    closed: bool,
    segments: usize,
    /// The turn from the chord before a knot to the chord after it, reduced to one turn.
    bends: Vec<f64>,
}

impl Layout {
    pub(super) fn new(points: &[(f64, f64)], closed: bool) -> Self {
        let segments = if closed {
            points.len()
        } else {
            points.len() - 1
        };
        let angles: Vec<_> = (0..segments)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                (b.1 - a.1).atan2(b.0 - a.0)
            })
            .collect();
        let bends = (0..segments)
            .map(|knot| {
                let prev = (knot + segments - 1) % segments;
                let angle = angles[knot] - angles[prev];
                angle - std::f64::consts::TAU * (angle / std::f64::consts::TAU + 0.5).floor()
            })
            .collect();
        Self {
            closed,
            segments,
            bends,
        }
    }

    /// The knots whose constraints segment `segment`'s coefficients move: its start and its end.
    pub(super) fn knots_of(&self, segment: usize) -> [usize; 2] {
        if self.closed {
            [segment, (segment + 1) % self.segments]
        } else {
            [segment, segment + 1]
        }
    }

    /// The first residual row of `knot`'s constraints and their values: two at an open end, four
    /// at any other knot. `ends` are the ends of every segment.
    pub(super) fn knot(
        &self,
        knot: usize,
        wrapped: bool,
        ends: &[[[f64; 4]; 2]],
    ) -> (usize, [f64; 4], usize) {
        if !self.closed && knot == 0 {
            let start = &ends[0][0];
            return (0, [start[2], start[3], 0.0, 0.0], 2);
        }
        if !self.closed && knot == self.segments {
            let end = &ends[self.segments - 1][1];
            return (4 * self.segments - 2, [end[2], end[3], 0.0, 0.0], 2);
        }
        let prev = (knot + self.segments - 1) % self.segments;
        let (before, after) = (&ends[prev][1], &ends[knot][0]);
        // As in Levien's `spiro_iter`: the bend is reduced to one turn, the error is not, so a
        // solution a full turn away is no solution unless `wrapped`.
        let delta = self.bends[knot] - before[0] - after[0];
        let angle = if wrapped {
            delta - std::f64::consts::TAU * (delta / std::f64::consts::TAU + 0.5).floor()
        } else {
            delta
        };
        let row = if self.closed { 4 * knot } else { 4 * knot - 2 };
        (
            row,
            [
                angle,
                after[1] - before[1],
                after[2] - before[2],
                after[3] - before[3],
            ],
            4,
        )
    }

    /// The constraint errors of a curve whose segments end as `ends`. With `wrapped`, a knot's
    /// angle error is reduced to one turn, so a curve that winds a full turn more at a knot also
    /// solves.
    pub(super) fn residual(&self, wrapped: bool, ends: &[[[f64; 4]; 2]]) -> Option<Vec<f64>> {
        let mut out = vec![0.0; 4 * self.segments];
        let knots = if self.closed {
            self.segments
        } else {
            self.segments + 1
        };
        for knot in 0..knots {
            let (row, values, count) = self.knot(knot, wrapped, ends);
            out[row..row + count].copy_from_slice(&values[..count]);
        }
        out.iter().all(|v| v.is_finite()).then_some(out)
    }
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
    Layout::new(points, closed).residual(wrapped, &ends?)
}

pub(super) fn maximum(values: &[f64]) -> f64 {
    values.iter().map(|v| v.abs()).fold(0.0, f64::max)
}
pub(super) fn energy(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum()
}
