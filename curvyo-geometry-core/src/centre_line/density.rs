//! Density-aware simplification inside Optimise (ADR 0063 amendment 2026-11-18, D): where the
//! anchors or width points of a fit lie closer together than a share of the local width, they
//! merge when the result stays within the setting's tolerance.

use super::safe_fit::fit_to_cubic;
use kurbo::simplify::SimplifyBezPath;
use kurbo::{BezPath, CubicBez, ParamCurve, ParamCurveNearest, PathEl, PathSeg, Point};

use super::profile::fits_values;
use crate::medial_axis::AxisSample;

/// The merge of one corner-free stretch of a fit: anchors closer than `distance` (mm) merge when
/// the span they bound refits as one cubic within `accuracy` (mm), or within `tight` where the
/// span reaches into the first or last stretch of `tight_ends` (mm along the stretch from its
/// start, and the distance from its start where the last one begins): the corner zones.
#[derive(Debug, Clone, Copy)]
pub(super) struct Merge {
    pub(super) distance: f64,
    pub(super) accuracy: f64,
    pub(super) tight: f64,
    pub(super) tight_ends: (f64, f64),
}

/// The fitted `path` of the corner-free `points` (the source the simplifier fitted, its
/// Catmull-Rom curve `source`) with neighbouring anchors closer than the merge distance merged,
/// closest pair first. The ends stay; a merge that misses the accuracy is not made.
pub(super) fn merge_anchors(
    points: &[Point],
    source: &BezPath,
    path: BezPath,
    merge: Merge,
) -> BezPath {
    let mut segments: Vec<PathSeg> = path.segments().collect();
    if merge.distance <= 0.0 || segments.len() < 2 {
        return path;
    }
    let ends = |s: &PathSeg| (s.start(), s.end());
    #[allow(clippy::cast_precision_loss)]
    let steps = (points.len() - 1) as f64;
    // Each anchor's parameter on the source (0 to 1 over its segments): the simplifier puts
    // anchors anywhere on it, so each is projected onto the source segments next to its
    // nearest point.
    let pieces: Vec<PathSeg> = source.segments().collect();
    let index_of = |anchor: Point| {
        let index = (0..points.len())
            .min_by(|&i, &j| {
                (points[i] - anchor)
                    .hypot2()
                    .total_cmp(&(points[j] - anchor).hypot2())
            })
            .unwrap_or(0);
        let (segment, t) = [index.saturating_sub(1), index.min(pieces.len() - 1)]
            .into_iter()
            .map(|piece| {
                let hit = pieces[piece].nearest(anchor, 1e-9);
                (piece, hit.t, hit.distance_sq)
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map_or((0, 0.0), |(piece, t, _)| (piece, t));
        #[allow(clippy::cast_precision_loss)]
        let at = (segment as f64 + t) / steps;
        at
    };
    let mut at: Vec<f64> = std::iter::once(segments[0].start())
        .chain(segments.iter().map(ParamCurve::end))
        .map(index_of)
        .collect();
    let mut merged = false;
    let curve = SimplifyBezPath::new(source.elements().iter().copied());
    // The distance along the points at a parameter, for the corner zones.
    let mut along = vec![0.0];
    for w in points.windows(2) {
        along.push(along[along.len() - 1] + (w[1] - w[0]).hypot());
    }
    let distance_at = |t: f64| {
        let x = t * steps;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let step = (x.floor() as usize).min(along.len() - 2);
        #[allow(clippy::cast_precision_loss)]
        let fraction = x - step as f64;
        along[step] + (along[step + 1] - along[step]) * fraction
    };
    let mut tried = vec![false; at.len()];
    loop {
        let candidate = (1..at.len() - 1)
            .filter(|&i| !tried[i])
            .map(|i| {
                let ((a, b), (c, d)) = (ends(&segments[i - 1]), ends(&segments[i]));
                let gap = (b - a).hypot().min((d - c).hypot());
                (i, gap)
            })
            .filter(|&(_, gap)| gap < merge.distance)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let Some((i, _)) = candidate else { break };
        tried[i] = true;
        if at[i - 1] >= at[i + 1] {
            continue;
        }
        let (from, to) = (distance_at(at[i - 1]), distance_at(at[i + 1]));
        let accuracy = if from < merge.tight_ends.0 || to > merge.tight_ends.1 {
            merge.tight
        } else {
            merge.accuracy
        };
        let Some((cubic, error2)) = fit_to_cubic(&curve, at[i - 1]..at[i + 1], accuracy) else {
            continue;
        };
        if error2.sqrt() > accuracy {
            continue;
        }
        // The fit runs on the source; its ends are the anchors themselves.
        let cubic = CubicBez::new(
            segments[i - 1].start(),
            cubic.p1,
            cubic.p2,
            segments[i].end(),
        );
        segments.splice(i - 1..=i, [PathSeg::Cubic(cubic)]);
        at.remove(i);
        tried.remove(i);
        merged = true;
    }
    if !merged {
        return path;
    }
    let mut out = BezPath::new();
    out.move_to(segments[0].start());
    for s in segments {
        out.push(match s {
            PathSeg::Line(l) => PathEl::LineTo(l.p1),
            PathSeg::Quad(q) => PathEl::QuadTo(q.p1, q.p2),
            PathSeg::Cubic(c) => PathEl::CurveTo(c.p1, c.p2, c.p3),
        });
    }
    out
}

/// The samples with each width the median over one local width of arc length around it: the
/// traced stairs go, steps longer than a width stay. The window stays symmetric, so it shrinks
/// toward the ends and a taper or a round tip keeps its values.
pub(super) fn running_median(samples: &[AxisSample]) -> Vec<AxisSample> {
    let mut along = Vec::with_capacity(samples.len());
    let mut s = 0.0;
    for (i, sample) in samples.iter().enumerate() {
        if i > 0 {
            s += (sample.point - samples[i - 1].point).hypot();
        }
        along.push(s);
    }
    let total = s;
    let mut window: Vec<f64> = Vec::new();
    samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let half = sample.radius.min(along[i]).min(total - along[i]);
            let low = along.partition_point(|&a| a < along[i] - half);
            let high = along.partition_point(|&a| a <= along[i] + half);
            window.clear();
            window.extend(samples[low..high.max(low + 1)].iter().map(|s| s.radius));
            window.sort_by(f64::total_cmp);
            AxisSample {
                point: sample.point,
                radius: window[window.len() / 2],
            }
        })
        .collect()
}

/// The merge of width points: neighbours closer than `distance` times their width (as a share
/// of the path's length per millimetre) merge when the profile stays within `share` of `T(w)`.
#[derive(Debug, Clone)]
pub(super) struct PointMerge {
    pub(super) distance: f64,
    pub(super) share: f64,
    /// The samples whose points may merge: not those of a round tip.
    pub(super) inner: std::ops::Range<usize>,
}

/// The profile points `chosen` (indices into the samples `at`, as position and width) with
/// their `values`, with neighbours closer than the merge distance merged, closest first: two
/// inner points become one at their middle sample with the median width between them; a point
/// next to an end (or a closed path's seam) goes. A merge that misses the tolerance is not made.
pub(super) fn merge_points(
    at: &[(f64, f64)],
    mut chosen: Vec<usize>,
    mut values: Vec<f64>,
    merge: &PointMerge,
) -> (Vec<usize>, Vec<f64>) {
    if merge.distance <= 0.0 {
        return (chosen, values);
    }
    let mut tried: Vec<bool> = vec![false; chosen.len()];
    loop {
        let last = chosen.len() - 1;
        let movable = |k: usize| merge.inner.contains(&chosen[k]);
        let candidate = (0..last)
            .filter(|&k| !tried[k] && movable(k) && movable(k + 1))
            .map(|k| {
                let gap = at[chosen[k + 1]].0 - at[chosen[k]].0;
                let width = f64::midpoint(values[k], values[k + 1]);
                (k, gap, gap < merge.distance * width)
            })
            .filter(|&(_, _, close)| close)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let Some((k, _, _)) = candidate else { break };
        tried[k] = true;
        let fits = |chosen: &[usize], values: &[f64], from: usize, to: usize| {
            (from..to).all(|i| {
                fits_values(
                    at,
                    chosen[i],
                    chosen[i + 1],
                    values[i],
                    values[i + 1],
                    merge.share,
                )
            })
        };
        let (mut c, mut v) = (chosen.clone(), values.clone());
        let (first_end, second_end) = (k == 0, k + 1 == last);
        if first_end && second_end {
            continue;
        }
        if first_end || second_end {
            // Into the end: the inner one goes.
            let gone = if first_end { k + 1 } else { k };
            c.remove(gone);
            v.remove(gone);
            let from = gone.saturating_sub(1);
            if !fits(&c, &v, from, (from + 1).min(c.len() - 1)) {
                continue;
            }
        } else {
            let middle = usize::midpoint(chosen[k], chosen[k + 1]);
            let mut widths: Vec<f64> = at[chosen[k]..=chosen[k + 1]].iter().map(|a| a.1).collect();
            widths.sort_by(f64::total_cmp);
            c.splice(k..=k + 1, [middle]);
            v.splice(k..=k + 1, [widths[widths.len() / 2]]);
            if !fits(&c, &v, k - 1, k + 1) {
                continue;
            }
        }
        chosen = c;
        values = v;
        tried = vec![false; chosen.len()];
    }
    (chosen, values)
}
