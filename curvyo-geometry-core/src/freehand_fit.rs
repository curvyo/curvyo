//! Fits an open cubic Bézier centre line to the samples of a freehand stroke
//! (`specs/0032-pen-tablet-input` criterion 7, `adrs.md` "the fit").
//!
//! The method is Schneider's ("An Algorithm for Automatically Fitting Digitized Curves",
//! Graphics Gems, 1990): one cubic is fitted to a run of samples by least squares with its end
//! tangents fixed; where a sample is farther than the accuracy from the cubic, the run is split
//! at the worst sample, with one tangent shared by both halves, so the result is smooth (G1).
//! The error is measured at each sample's own parameter, which is never less than its true
//! distance to the curve, so every sample lies within the accuracy of the result. Unlike
//! `kurbo::simplify_bezpath`, which `adrs.md` named first, it fits the samples rather than a
//! curve through them, so a hand's tremor does not become nodes.

use curvyo_document_core::{Point, Tolerance, Vec2};

use crate::OutlineTriple;

/// Samples closer than this to the previous kept sample are dropped (one grid pitch of the
/// boolean kernel): they add no shape and make chords of zero length.
const DUPLICATE_MM: f64 = 0.001;

/// End and split tangents are taken towards the first sample at least this far away, so the
/// tremor between two close samples does not turn them.
const TANGENT_REACH_MM: f64 = 0.5;

/// Newton steps that improve the parameters of a nearly good fit before it is split.
const REPARAMETERIZE_STEPS: usize = 4;

/// One cubic segment as its four absolute control points.
type Cubic = [Point; 4];

/// The samples without consecutive near-duplicates.
fn distinct(samples: &[Point]) -> Vec<Point> {
    let mut kept: Vec<Point> = Vec::with_capacity(samples.len());
    for sample in samples {
        let far_enough = kept
            .last()
            .is_none_or(|last| last.vector_to(*sample).length() >= DUPLICATE_MM);
        if far_enough {
            kept.push(*sample);
        }
    }
    kept
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn eval(cubic: &Cubic, t: f64) -> Point {
    let mt = 1.0 - t;
    let w = [mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t];
    Point::new(
        w[0] * cubic[0].x + w[1] * cubic[1].x + w[2] * cubic[2].x + w[3] * cubic[3].x,
        w[0] * cubic[0].y + w[1] * cubic[1].y + w[2] * cubic[2].y + w[3] * cubic[3].y,
    )
}

/// The first and second derivative of `cubic` at `t`.
fn derivatives(cubic: &Cubic, t: f64) -> (Vec2, Vec2) {
    let d = [
        cubic[0].vector_to(cubic[1]).scaled(3.0),
        cubic[1].vector_to(cubic[2]).scaled(3.0),
        cubic[2].vector_to(cubic[3]).scaled(3.0),
    ];
    let mt = 1.0 - t;
    let first = d[0].scaled(mt * mt) + d[1].scaled(2.0 * mt * t) + d[2].scaled(t * t);
    let second = (d[1] - d[0]).scaled(2.0 * mt) + (d[2] - d[1]).scaled(2.0 * t);
    (first, second)
}

/// The unit direction from `points[from]` towards the first sample at least `TANGENT_REACH_MM`
/// away, walking forward (`forward`) or backward and stopping at `end` (inclusive).
fn tangent_towards(points: &[Point], from: usize, end: usize, forward: bool) -> Vec2 {
    let mut index = from;
    loop {
        if index == end {
            break;
        }
        index = if forward { index + 1 } else { index - 1 };
        if points[from].vector_to(points[index]).length() >= TANGENT_REACH_MM {
            break;
        }
    }
    points[from].vector_to(points[index]).normalized_to(1.0)
}

/// The tangent shared by the two halves at a split, pointing back along the stroke.
fn split_tangent(points: &[Point], split: usize, first: usize, last: usize) -> Vec2 {
    let back = tangent_towards(points, split, first, false);
    let ahead = tangent_towards(points, split, last, true);
    let tangent = (back - ahead).normalized_to(1.0);
    if tangent == Vec2::ZERO { back } else { tangent }
}

/// Parameters of `points` proportional to the chord length, from 0 to 1.
fn chord_parameters(points: &[Point]) -> Vec<f64> {
    let mut parameters = Vec::with_capacity(points.len());
    let mut total = 0.0;
    parameters.push(0.0);
    for pair in points.windows(2) {
        total += pair[0].vector_to(pair[1]).length();
        parameters.push(total);
    }
    if total > 0.0 {
        for parameter in &mut parameters {
            *parameter /= total;
        }
    }
    parameters
}

/// The least-squares cubic through the run's end samples with handles along `tangent_start`
/// and `tangent_end` (the latter pointing back from the end).
fn least_squares(
    points: &[Point],
    parameters: &[f64],
    tangent_start: Vec2,
    tangent_end: Vec2,
) -> Cubic {
    let (start, end) = (points[0], points[points.len() - 1]);
    let mut c = [[0.0; 2]; 2];
    let mut x = [0.0; 2];
    for (point, &t) in points.iter().zip(parameters) {
        let mt = 1.0 - t;
        let a0 = tangent_start.scaled(3.0 * mt * mt * t);
        let a1 = tangent_end.scaled(3.0 * mt * t * t);
        c[0][0] += dot(a0, a0);
        c[0][1] += dot(a0, a1);
        c[1][1] += dot(a1, a1);
        let on_chord = Point::new(
            start.x * (mt * mt * mt + 3.0 * mt * mt * t) + end.x * (3.0 * mt * t * t + t * t * t),
            start.y * (mt * mt * mt + 3.0 * mt * mt * t) + end.y * (3.0 * mt * t * t + t * t * t),
        );
        let residual = on_chord.vector_to(*point);
        x[0] += dot(a0, residual);
        x[1] += dot(a1, residual);
    }
    c[1][0] = c[0][1];
    let determinant = c[0][0] * c[1][1] - c[1][0] * c[0][1];
    let chord = start.vector_to(end).length();
    let (mut alpha_start, mut alpha_end) = if determinant.abs() > f64::EPSILON {
        (
            (x[0] * c[1][1] - c[0][1] * x[1]) / determinant,
            (c[0][0] * x[1] - x[0] * c[1][0]) / determinant,
        )
    } else {
        (0.0, 0.0)
    };
    // A handle of no or negative length, or one longer than the run, is the sign of a bad
    // solve: Schneider's fallback of a third of the chord is safe.
    let usable = |alpha: f64| alpha > chord * 1.0e-6 && alpha < chord * 3.0;
    if !usable(alpha_start) || !usable(alpha_end) {
        alpha_start = chord / 3.0;
        alpha_end = chord / 3.0;
    }
    [
        start,
        start.translated(tangent_start.scaled(alpha_start)),
        end.translated(tangent_end.scaled(alpha_end)),
        end,
    ]
}

/// The largest squared distance of a sample from the cubic at its parameter, and that sample's
/// index, never the first or last.
fn worst_sample(points: &[Point], parameters: &[f64], cubic: &Cubic) -> (f64, usize) {
    let mut worst = (0.0, points.len() / 2);
    for index in 1..points.len() - 1 {
        let distance = eval(cubic, parameters[index]).vector_to(points[index]);
        let squared = dot(distance, distance);
        if squared > worst.0 {
            worst = (squared, index);
        }
    }
    worst
}

/// One Newton step towards the parameter of the point on `cubic` nearest each sample.
fn reparameterize(points: &[Point], parameters: &mut [f64], cubic: &Cubic) {
    for (point, t) in points.iter().zip(parameters.iter_mut()) {
        let offset = point.vector_to(eval(cubic, *t));
        let (first, second) = derivatives(cubic, *t);
        let denominator = dot(first, first) + dot(offset, second);
        if denominator.abs() > f64::EPSILON {
            *t = (*t - dot(offset, first) / denominator).clamp(0.0, 1.0);
        }
    }
}

/// Fits cubics to `points[first..=last]` within `accuracy` (squared), appending them to `out`.
fn fit_run(
    points: &[Point],
    (first, last): (usize, usize),
    (tangent_start, tangent_end): (Vec2, Vec2),
    accuracy_squared: f64,
    out: &mut Vec<Cubic>,
) {
    let run = &points[first..=last];
    if run.len() == 2 {
        let reach = run[0].vector_to(run[1]).length() / 3.0;
        out.push([
            run[0],
            run[0].translated(tangent_start.scaled(reach)),
            run[1].translated(tangent_end.scaled(reach)),
            run[1],
        ]);
        return;
    }
    let mut parameters = chord_parameters(run);
    let mut cubic = least_squares(run, &parameters, tangent_start, tangent_end);
    let (mut error, mut split) = worst_sample(run, &parameters, &cubic);
    if error > accuracy_squared && error < 4.0 * accuracy_squared {
        for _ in 0..REPARAMETERIZE_STEPS {
            reparameterize(run, &mut parameters, &cubic);
            cubic = least_squares(run, &parameters, tangent_start, tangent_end);
            (error, split) = worst_sample(run, &parameters, &cubic);
            if error <= accuracy_squared {
                break;
            }
        }
    }
    if error <= accuracy_squared {
        out.push(cubic);
        return;
    }
    let split = first + split;
    let shared = split_tangent(points, split, first, last);
    fit_run(
        points,
        (first, split),
        (tangent_start, shared),
        accuracy_squared,
        out,
    );
    fit_run(
        points,
        (split, last),
        (shared.negated(), tangent_end),
        accuracy_squared,
        out,
    );
}

/// The anchors of consecutive cubics as `(point, handle_in, handle_out)`, handles relative.
fn anchors(cubics: &[Cubic]) -> Vec<OutlineTriple> {
    let mut anchors: Vec<OutlineTriple> = Vec::with_capacity(cubics.len() + 1);
    for cubic in cubics {
        match anchors.last_mut() {
            Some(last) => last.2 = cubic[0].vector_to(cubic[1]),
            None => anchors.push((cubic[0], Vec2::ZERO, cubic[0].vector_to(cubic[1]))),
        }
        anchors.push((cubic[3], cubic[3].vector_to(cubic[2]), Vec2::ZERO));
    }
    anchors
}

/// Whether `handle` lies on the line of `chord`, within a hundredth of `accuracy_mm`.
fn along(chord: Vec2, handle: Vec2, accuracy_mm: f64) -> bool {
    (chord.x * handle.y - chord.y * handle.x).abs() / chord.length() <= accuracy_mm / 100.0
}

/// The open centre line fitted to `samples`: few anchors, every sample within `accuracy` of the
/// result. Empty when there are fewer than two distinct samples. A fit of one straight segment
/// has two anchors with zero handles.
#[must_use]
pub fn fit_freehand(samples: &[Point], accuracy: Tolerance) -> Vec<OutlineTriple> {
    let points = distinct(samples);
    if points.len() < 2 {
        return Vec::new();
    }
    let last = points.len() - 1;
    let tangents = (
        tangent_towards(&points, 0, last, true),
        tangent_towards(&points, last, 0, false),
    );
    let mut cubics = Vec::new();
    let accuracy_mm = accuracy.as_mm();
    fit_run(
        &points,
        (0, last),
        tangents,
        accuracy_mm * accuracy_mm,
        &mut cubics,
    );
    let mut fitted = anchors(&cubics);
    if fitted.len() == 2 {
        let chord = fitted[0].0.vector_to(fitted[1].0);
        if along(chord, fitted[0].2, accuracy_mm) && along(chord, fitted[1].1, accuracy_mm) {
            fitted[0].2 = Vec2::ZERO;
            fitted[1].1 = Vec2::ZERO;
        }
    }
    fitted
}
