//! Fitting a drawn stroke's samples to a cubic centre line with a width profile (ADR 0095
//! decision 5, `specs/0158-pen-display` slice 1 criteria 2, 5, 9).
//!
//! The curve keeps every sample within the accuracy (floored at [`MIN_ACCURACY_MM`]): each
//! corner-free run is covered by the fewest cubics a greedy farthest reach finds, each fitted
//! by the shared kurbo fitter behind its guard ([`crate::cubic_fit`]) and measured against every
//! sample it spans. A corner is a turn of more than [`CORNER_DEGREES`] between the chords to the
//! samples `reach` before and after.

use curvyo_document_core::{Length, Point as DocPoint};
use kurbo::{BezPath, ParamCurveNearest, PathSeg, Point};

use crate::centre_line::fit_profile_within;
use crate::cubic_fit::{anchors_of, fit_piece};
use crate::medial_axis::AxisSample;
use crate::{OutlineTriple, WidthProfile};

/// The least accuracy of a fit, in millimetres: Smoothing 0 stays within 0.1 mm (criterion 9).
pub const MIN_ACCURACY_MM: f64 = 0.05;
/// A turn sharper than this between the chords before and after a sample is a corner.
pub const CORNER_DEGREES: f64 = 60.0;
/// The corner reach in multiples of the accuracy, and its floor in millimetres (Proposal).
const CORNER_REACH: (f64, f64) = (4.0, 1.0);
/// A fixed width bound below this (mm) is taken as this, so equal Min and Max still fit.
const MIN_WIDTH_BOUND_MM: f64 = 1e-6;
/// Accuracy shares a run is refitted at before it is split at its worst sample.
const RETRY_SHARES: [f64; 2] = [1.0, 0.5];
/// How many segments past the last nearest one a sample is measured against first.
const SEGMENT_WINDOW: usize = 3;
/// The fit tries every accuracy on this grid (mm, one Smoothing percent) from the asked one down
/// to [`LADDER_FLOOR`] of it and keeps the fewest nodes, so a larger Smoothing never gives more
/// nodes than a smaller one on the grid (criterion 9): its candidates include all of the smaller's
/// but the lowest.
const LADDER_STEP_MM: f64 = 0.01;
/// The lowest share of the asked accuracy the ladder tries.
const LADDER_FLOOR: f64 = 0.5;
/// Samples this close (mm) are one sample.
const COINCIDENT_MM: f64 = 1e-6;

/// One sample of a drawn stroke: where it is and how wide the line is there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeSample {
    /// The document point.
    pub point: DocPoint,
    /// The line width at the sample.
    pub width: Length,
}

/// A fitted stroke: the open centre line, its widest sampled width and its profile on that
/// width (multipliers of `width`), `None` when no width tolerance was asked for.
#[derive(Debug, Clone)]
pub struct FittedStroke {
    /// The anchors as `(point, handle_in, handle_out)`.
    pub anchors: Vec<OutlineTriple>,
    /// The widest sampled width.
    pub width: Length,
    /// The width profile, multipliers of `width` on normalised arc length.
    pub profile: Option<WidthProfile>,
}

/// Fits `samples` to an open cubic centre line within `accuracy` (floored at
/// [`MIN_ACCURACY_MM`]) of every sample, and, with a `width_tolerance`, a width profile within it
/// of every sample's width. `None` for fewer than two distinct points.
#[must_use]
pub fn fit_stroke(
    samples: &[StrokeSample],
    accuracy: Length,
    width_tolerance: Option<Length>,
) -> Option<FittedStroke> {
    let mut kept: Vec<StrokeSample> = Vec::with_capacity(samples.len());
    for sample in samples {
        let finite = sample.point.x.is_finite() && sample.point.y.is_finite();
        let apart = kept
            .last()
            .is_none_or(|last| distance(last.point, sample.point) >= COINCIDENT_MM);
        if finite && apart {
            kept.push(*sample);
        }
    }
    if kept.len() < 2 {
        return None;
    }
    let accuracy = accuracy.as_mm().max(MIN_ACCURACY_MM);
    let points: Vec<Point> = kept
        .iter()
        .map(|s| Point::new(s.point.x, s.point.y))
        .collect();
    let mut cuts = corners(&points, accuracy);
    cuts.insert(0, 0);
    cuts.push(points.len() - 1);
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for pair in cuts.windows(2) {
        for element in ladder_fit(&points[pair[0]..=pair[1]], accuracy)
            .elements()
            .iter()
            .skip(1)
        {
            path.push(*element);
        }
    }
    let anchors = anchors_of(&path, false);
    if anchors.len() < 2 {
        return None;
    }
    let base = kept.iter().map(|s| s.width.as_mm()).fold(0.0_f64, f64::max);
    let profile = match width_tolerance {
        Some(bound) if base > 0.0 => {
            let axis: Vec<AxisSample> = kept
                .iter()
                .zip(&points)
                .map(|(s, &point)| AxisSample {
                    point,
                    radius: s.width.as_mm() / 2.0,
                })
                .collect();
            fit_profile_within(&anchors, &axis, base, bound.as_mm().max(MIN_WIDTH_BOUND_MM))
        }
        _ => None,
    };
    Some(FittedStroke {
        anchors,
        width: Length::from_mm(base),
        profile,
    })
}

/// The indices of corner samples: where the chords to the samples `reach` before and after
/// turn by more than [`CORNER_DEGREES`]; the sharpest of each run within `reach` is kept.
fn corners(points: &[Point], accuracy: f64) -> Vec<usize> {
    let reach = (CORNER_REACH.0 * accuracy).max(CORNER_REACH.1);
    let limit = CORNER_DEGREES.to_radians().cos();
    let n = points.len();
    // The turn at each sample as the cosine between its chords, `None` near the ends.
    let mut back = 0;
    let mut ahead = 0;
    let mut found: Vec<(usize, f64)> = Vec::new();
    for i in 1..n.saturating_sub(1) {
        while back + 1 < i && (points[i] - points[back + 1]).hypot() >= reach {
            back += 1;
        }
        ahead = ahead.max(i + 1);
        while ahead < n - 1 && (points[ahead] - points[i]).hypot() < reach {
            ahead += 1;
        }
        let (a, b) = (points[i] - points[back], points[ahead] - points[i]);
        let (la, lb) = (a.hypot(), b.hypot());
        if la < reach || lb < reach {
            continue;
        }
        let cos = a.dot(b) / (la * lb);
        if cos < limit {
            found.push((i, cos));
        }
    }
    let mut kept: Vec<(usize, f64)> = Vec::new();
    for (i, cos) in found {
        match kept.last_mut() {
            Some(last) if (points[i] - points[last.0]).hypot() < reach => {
                if cos < last.1 {
                    *last = (i, cos);
                }
            }
            _ => kept.push((i, cos)),
        }
    }
    kept.into_iter().map(|(i, _)| i).collect()
}

/// The verified fit of a corner-free run with the fewest segments among the accuracies of the
/// ladder from `accuracy` down (see [`LADDER_STEP_MM`]); a tie keeps the larger accuracy.
fn ladder_fit(points: &[Point], accuracy: f64) -> BezPath {
    let lowest = (accuracy * LADDER_FLOOR).max(MIN_ACCURACY_MM);
    let mut best = verified_fit(points, accuracy);
    // The grid values below the asked accuracy, highest first.
    let mut k = (accuracy / LADDER_STEP_MM - 1e-9).floor();
    while k * LADDER_STEP_MM >= lowest - 1e-12 {
        let candidate = verified_fit(points, k * LADDER_STEP_MM);
        if candidate.elements().len() < best.elements().len() {
            best = candidate;
        }
        k -= 1.0;
    }
    best
}

/// The piece fit of a corner-free run, measured against every one of its points: refitted at a
/// tighter accuracy, then split at its worst point, until every point lies within `accuracy`.
fn verified_fit(points: &[Point], accuracy: f64) -> BezPath {
    let mut worst = (0, 0.0);
    for share in RETRY_SHARES {
        let path = fit_piece(points, accuracy * share);
        worst = farthest(&path, points, accuracy);
        if worst.1 <= accuracy {
            return path;
        }
    }
    if points.len() < 3 {
        let mut line = BezPath::new();
        line.move_to(points[0]);
        line.line_to(points[points.len() - 1]);
        return line;
    }
    let at = worst.0.clamp(1, points.len() - 2);
    let mut out = verified_fit(&points[..=at], accuracy);
    for element in verified_fit(&points[at..], accuracy)
        .elements()
        .iter()
        .skip(1)
    {
        out.push(*element);
    }
    out
}

/// The point of `points` farthest from `path` and its distance; the points run along the path,
/// so each is measured against a few segments past the last one's nearest first, and against
/// all of them only when those are farther than `limit`.
fn farthest(path: &BezPath, points: &[Point], limit: f64) -> (usize, f64) {
    let segments: Vec<PathSeg> = path.segments().collect();
    if segments.is_empty() {
        return (0, 0.0);
    }
    let near = |seg: &PathSeg, p: Point| seg.nearest(p, 1e-9).distance_sq.sqrt();
    let mut current = 0;
    let mut worst = (0, 0.0_f64);
    for (i, &p) in points.iter().enumerate() {
        let window = current..(current + SEGMENT_WINDOW).min(segments.len());
        let (mut best, mut at) = (f64::INFINITY, current);
        for k in window {
            let d = near(&segments[k], p);
            if d < best {
                (best, at) = (d, k);
            }
        }
        if best > limit {
            // Far from the window: the full scan decides (rare; a fit that doubles back).
            for (k, seg) in segments.iter().enumerate() {
                let d = near(seg, p);
                if d < best {
                    (best, at) = (d, k);
                }
            }
        }
        current = at;
        if best > worst.1 {
            worst = (i, best);
        }
    }
    worst
}

fn distance(a: DocPoint, b: DocPoint) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[cfg(test)]
#[path = "stroke_fit_tests.rs"]
mod tests;
