//! The width profile of a fitted centre path (ADR 0063 §5): the fewest smoothstep points that
//! keep the measured widths within a bound the caller gives.

use curvyo_document_core::Point as DocPoint;

use super::density::{PointMerge, merge_points};
use super::fit::{Budget, MERGE_SLACK, tolerance};
use crate::medial_axis::AxisSample;
use crate::{OutlineTriple, PathMeasure, WidthPoint, WidthProfile};

/// The greedy profile pass looks this many samples past its last fit before it stops.
const GREEDY_LOOK_AHEAD: usize = 64;
/// How far beyond twice its step a sample's projection may lie, in millimetres.
const PROJECTION_SLACK_MM: f64 = 0.5;

/// How far the profile may stray from a sample's width (ADR 0095 decision 5: the caller says).
#[derive(Debug, Clone, Copy)]
pub(crate) enum WidthBound {
    /// A share of `T(w)` of the sample's own width: traced centre lines.
    Share(f64),
    /// A fixed distance in millimetres: a drawn stroke (5 % of Max − Min).
    Fixed(f64),
}

impl WidthBound {
    /// The bound at a sample of width `w` (mm).
    fn at(self, w: f64) -> f64 {
        match self {
            Self::Share(share) => tolerance(w) * share,
            Self::Fixed(bound) => bound,
        }
    }
}

/// The profile: each sample projected onto the fitted path, then the fewest points (a greedy
/// farthest reach) whose smoothstep stays within `share` of `T(w)` of every sample.
pub(super) fn fit_profile(
    anchors: &[OutlineTriple],
    samples: &[AxisSample],
    closed: bool,
    base: f64,
    (budget, inner): (Budget, std::ops::Range<usize>),
) -> Option<WidthProfile> {
    let share = budget.share;
    let (at, total) = projected(anchors, samples, closed)?;
    let bound = WidthBound::Share(share);
    let chosen = greedy(&at, bound);
    let values = refine(&at, &chosen, closed, bound);
    let (chosen, values) = merge_points(
        &at,
        chosen,
        values,
        &PointMerge {
            distance: budget.merge / total,
            share: share + MERGE_SLACK,
            inner,
        },
    );
    profile_of(&at, &chosen, values, closed, base)
}

/// The profile of an open drawn stroke: the fewest smoothstep points within the fixed `bound`
/// (mm) of every sample's width, as multipliers of `base`; no merge.
pub(crate) fn fit_profile_within(
    anchors: &[OutlineTriple],
    samples: &[AxisSample],
    base: f64,
    bound: f64,
) -> Option<WidthProfile> {
    let (at, _) = projected(anchors, samples, false)?;
    let bound = WidthBound::Fixed(bound);
    let chosen = greedy(&at, bound);
    let values = refine(&at, &chosen, false, bound);
    profile_of(&at, &chosen, values, false, base)
}

/// Each sample's normalised arc length on the fitted path and its width, and the path's length.
fn projected(
    anchors: &[OutlineTriple],
    samples: &[AxisSample],
    closed: bool,
) -> Option<(Vec<(f64, f64)>, f64)> {
    let measure = PathMeasure::new(anchors, closed);
    let total = measure.length().as_mm();
    if total <= 0.0 {
        return None;
    }
    let mut at: Vec<(f64, f64)> = Vec::with_capacity(samples.len() + 1);
    let mut previous = 0.0_f64;
    for (i, sample) in samples.iter().enumerate() {
        let s = if i == 0 {
            0.0
        } else if !closed && i == samples.len() - 1 {
            1.0
        } else {
            // The samples run along the curve, so each lies near the stretch after the last
            // one: twice the step between them, plus room for the fit error.
            let step = (sample.point - samples[i - 1].point).hypot();
            let along = measure.nearest_ahead(
                DocPoint::new(sample.point.x, sample.point.y),
                previous * total,
                2.0 * step + PROJECTION_SLACK_MM,
            ) / total;
            along.clamp(previous, 1.0)
        };
        previous = s;
        at.push((s, 2.0 * sample.radius));
    }
    if closed {
        at.retain(|&(s, _)| s < 1.0);
        at.push((1.0, 2.0 * samples[0].radius));
    }
    Some((at, total))
}

/// The width points of the chosen samples at their values, as multipliers of `base`.
fn profile_of(
    at: &[(f64, f64)],
    chosen: &[usize],
    values: Vec<f64>,
    closed: bool,
    base: f64,
) -> Option<WidthProfile> {
    let points = chosen
        .iter()
        .zip(values)
        .filter(|&(&i, _)| !closed || at[i].0 < 1.0)
        .enumerate()
        .filter_map(|(k, (&i, value))| {
            WidthPoint::new(k as u128 + 1, at[i].0, value / base, closed)
        })
        .collect();
    WidthProfile::new(points, closed)
}

/// The width at each chosen point: the median of the samples nearest to it rather than its own
/// sample, which may be an outlier (the centre of a flattened round end measures its circle
/// a flattening tolerance small), kept only where every sample still fits within `bound`.
/// A closed path's seam keeps its own sample, so it stays periodic.
fn refine(at: &[(f64, f64)], chosen: &[usize], closed: bool, bound: WidthBound) -> Vec<f64> {
    let own: Vec<f64> = chosen.iter().map(|&i| at[i].1).collect();
    let last = chosen.len() - 1;
    let mut values: Vec<f64> = (0..chosen.len())
        .map(|k| {
            if closed && (k == 0 || k == last) {
                return own[k];
            }
            let low = if k == 0 {
                chosen[0]
            } else {
                usize::midpoint(chosen[k - 1], chosen[k])
            };
            let high = if k == last {
                chosen[last]
            } else {
                usize::midpoint(chosen[k], chosen[k + 1])
            };
            let mut widths: Vec<f64> = at[low..=high].iter().map(|&(_, w)| w).collect();
            widths.sort_by(f64::total_cmp);
            widths[widths.len() / 2]
        })
        .collect();
    // Each pass reverts the points next to a failing interval, until none fails or every such
    // point is back at its own sample, where the greedy pass made every interval fit.
    loop {
        let failing: Vec<usize> = (0..last)
            .filter(|&k| {
                !fits_values(
                    at,
                    chosen[k],
                    chosen[k + 1],
                    values[k],
                    values[k + 1],
                    bound,
                )
            })
            .collect();
        let mut changed = false;
        for k in failing {
            for at in [k, k + 1] {
                changed |= values[at].to_bits() != own[at].to_bits();
                values[at] = own[at];
            }
        }
        if !changed {
            return values;
        }
    }
}

/// Whether the smoothstep from `from` at `at[i]` to `to` at `at[j]` stays within `bound` of
/// every sample from `i` to `j`, both included.
pub(super) fn fits_values(
    at: &[(f64, f64)],
    i: usize,
    j: usize,
    from: f64,
    to: f64,
    bound: WidthBound,
) -> bool {
    let span = at[j].0 - at[i].0;
    at[i..=j].iter().all(|&(s, w)| {
        let t = if span > 0.0 {
            ((s - at[i].0) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let value = from + (to - from) * t * t * (3.0 - 2.0 * t);
        (value - w).abs() <= bound.at(w)
    })
}

/// Indices of the profile points: from each chosen point, the farthest sample such that the
/// smoothstep between them stays within `bound` of every sample in between.
fn greedy(at: &[(f64, f64)], bound: WidthBound) -> Vec<usize> {
    let mut chosen = vec![0];
    let mut i = 0;
    let last = at.len() - 1;
    while i < last {
        let mut reach = i + 1;
        for j in i + 2..=last {
            if fits(at, i, j, bound) {
                reach = j;
            } else if j > reach + GREEDY_LOOK_AHEAD {
                break;
            }
        }
        chosen.push(reach);
        i = reach;
    }
    chosen
}

/// Whether the smoothstep from `at[i]` to `at[j]` stays within `bound` of the samples between.
fn fits(at: &[(f64, f64)], i: usize, j: usize, bound: WidthBound) -> bool {
    let (from, to) = (at[i], at[j]);
    let span = to.0 - from.0;
    at[i + 1..j].iter().all(|&(s, w)| {
        let t = if span > 0.0 {
            ((s - from.0) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let smooth = t * t * (3.0 - 2.0 * t);
        let value = from.1 + (to.1 - from.1) * smooth;
        (value - w).abs() <= bound.at(w)
    })
}
