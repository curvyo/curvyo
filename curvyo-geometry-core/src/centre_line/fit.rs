//! The fit of a chain's samples (ADR 0063 §5): cubic Béziers within half the tolerance between
//! corner anchors, and a width profile with the fewest smoothstep points that stay within half
//! the tolerance of the measured widths.

use curvyo_document_core::{Length, Point as DocPoint, Vec2 as DocVec};
use kurbo::{BezPath, PathEl, Point};

use super::density::{Merge, merge_anchors, running_median};
use super::profile::fit_profile;
use super::safe_fit::{STRAY, simplify, strays};
use crate::medial_axis::AxisSample;
use crate::{OutlineTriple, WidthProfile};

/// `T(w) = max(0.05 mm, 3 % of w)`.
pub(crate) fn tolerance(width: f64) -> f64 {
    0.05_f64.max(0.03 * width)
}

/// A turn sharper than this within one local width is a corner (45°, as its cosine).
const CORNER_COS: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// A turn between two steps of a piece whose cosine is below this (120 degrees) cuts the piece.
const HAIRPIN_COS: f64 = -0.5;
/// Within this many local widths of a corner the curve keeps the tight accuracy.
const CORNER_REACH: f64 = 2.0;
/// The curve's tolerance is `T` of the width below which this inverse share of samples lies
/// (the lowest quarter).
const NARROW_QUANTILE: usize = 4;
/// Points of a piece closer than this are one point, in millimetres.
const COINCIDENT_MM: f64 = 1e-6;

/// A fitted path: anchors, closure, base width and profile.
#[derive(Debug, Clone)]
pub(crate) struct Fitted {
    pub(crate) anchors: Vec<OutlineTriple>,
    pub(crate) closed: bool,
    pub(crate) width: Length,
    pub(crate) profile: WidthProfile,
}

/// How `CentreOptions::optimise` (0 to 1) loosens the fit (ADR 0063 amendment, customer
/// 2026-11-17, CENTRE-DEFAULT A): at 0 the curve and the profile keep a quarter of `T(w)`, at
/// 0.5 half of it (so lowering Optimise follows the shape closer at every step); to the default
/// 0.75 their share grows to all of `T(w)` (criterion 5's 4 % painted area); to 1 it grows to
/// three times `T(w)`. Near corners the curve keeps at most half of `T(w)` at every setting.
///
/// Above 0.5 neighbouring anchors and width points closer than `merge` times the local width
/// merge (ADR 0063 amendment 2026-11-18, D): `merge` is 0 up to 0.5, 0.5 at the default and
/// 0.75 at 1, and a merge may add half of `T(w)` on smooth stretches.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Budget {
    /// The share of `T(w)` the curve and the profile may stray.
    pub(crate) share: f64,
    /// The merge distance as a share of the local width.
    pub(crate) merge: f64,
}

impl Budget {
    pub(crate) fn of(optimise: f64) -> Self {
        let o = optimise.clamp(0.0, 1.0);
        Self {
            share: if o <= 0.5 {
                CLOSEST_SHARE + (CORNER_SHARE - CLOSEST_SHARE) * o / 0.5
            } else if o <= 0.75 {
                0.5 + 2.0 * (o - 0.5)
            } else {
                1.0 + 8.0 * (o - 0.75)
            },
            merge: if o <= 0.5 {
                0.0
            } else if o <= 0.75 {
                2.0 * (o - 0.5)
            } else {
                0.5 + (o - 0.75)
            },
        }
    }
}

/// The share of `T(w)` the fit keeps at Optimise 0.
const CLOSEST_SHARE: f64 = 0.25;
/// The share of `T(w)` the fit keeps at Optimise 0.5 and near corners at every setting above.
const CORNER_SHARE: f64 = 0.5;

/// The share of `T(w)` a merge may add on smooth stretches.
pub(super) const MERGE_SLACK: f64 = 0.5;

/// Fits `samples` (at least two) within the [`Budget`].
pub(crate) fn fit(samples: &[AxisSample], closed: bool, budget: Budget) -> Option<Fitted> {
    fit_with_tips(samples, closed, budget, [0, 0])
}

/// [`fit`] where the first and last `tips` samples draw round tips (CENTRE-TIP A): their widths
/// are neither smoothed nor merged.
pub(crate) fn fit_with_tips(
    samples: &[AxisSample],
    closed: bool,
    budget: Budget,
    tips: [usize; 2],
) -> Option<Fitted> {
    if samples.len() < 2 {
        return None;
    }
    let share = budget.share;
    // The curve's tolerance comes from a low quantile of the widths, not the least one: a
    // tapering end reaches width 0 and would hold the whole curve to the 0.05 mm floor.
    let mut widths: Vec<f64> = samples.iter().map(|s| 2.0 * s.radius).collect();
    widths.sort_by(f64::total_cmp);
    let narrow = widths[widths.len() / NARROW_QUANTILE];
    let accuracy = tolerance(narrow) * share;
    let tight = tolerance(narrow) * CORNER_SHARE.min(share);
    let merging = budget.merge > 0.0;
    let slack = tolerance(narrow) * MERGE_SLACK;
    let anchors = fit_curve(samples, closed, (accuracy, tight), (budget.merge, slack));
    if anchors.len() < 2 {
        return None;
    }
    let base = samples
        .iter()
        .map(|s| 2.0 * s.radius)
        .fold(0.0_f64, f64::max);
    // A line thinner than the smallest tolerance is tracing noise, not a stroke (Proposal).
    if base < tolerance(0.0) {
        return None;
    }
    // Merging width points works on widths cleared of the traced stairs.
    let inner = tips[0].min(samples.len())..samples.len().saturating_sub(tips[1]).max(tips[0]);
    let widths = if merging && !inner.is_empty() {
        let mut widths = samples.to_vec();
        widths.splice(inner.clone(), running_median(&samples[inner.clone()]));
        widths
    } else {
        samples.to_vec()
    };
    let profile = fit_profile(&anchors, &widths, closed, base, (budget, inner))?;
    Some(Fitted {
        anchors,
        closed,
        width: Length::from_mm(base),
        profile,
    })
}

/// The indices of corner samples.
pub(crate) fn corners(samples: &[AxisSample]) -> Vec<usize> {
    let mut found = Vec::new();
    let n = samples.len();
    for i in 1..n.saturating_sub(1) {
        let reach = 2.0 * samples[i].radius;
        let here = samples[i].point;
        let back = (0..i)
            .rev()
            .find(|&k| (samples[k].point - here).hypot() >= reach)
            .unwrap_or(0);
        let ahead = (i + 1..n)
            .find(|&k| (samples[k].point - here).hypot() >= reach)
            .unwrap_or(n - 1);
        let (a, b) = (here - samples[back].point, samples[ahead].point - here);
        let (la, lb) = (a.hypot(), b.hypot());
        if la > 0.0 && lb > 0.0 && a.dot(b) / (la * lb) < CORNER_COS {
            found.push(i);
        }
    }
    // Keep the sharpest of each run of neighbouring corner samples.
    let mut kept: Vec<usize> = Vec::new();
    for i in found {
        match kept.last() {
            Some(&last)
                if (samples[i].point - samples[last].point).hypot() < 2.0 * samples[i].radius =>
            {
                if turn(samples, i) > turn(samples, last) {
                    kept.pop();
                    kept.push(i);
                }
            }
            _ => kept.push(i),
        }
    }
    kept
}

/// The turn at sample `i` between its neighbours, as `1 - cos`.
fn turn(samples: &[AxisSample], i: usize) -> f64 {
    let (a, b) = (
        samples[i].point - samples[i - 1].point,
        samples[i + 1].point - samples[i].point,
    );
    let (la, lb) = (a.hypot(), b.hypot());
    if la == 0.0 || lb == 0.0 {
        return 0.0;
    }
    1.0 - a.dot(b) / (la * lb)
}

/// Cubic Béziers through the samples' points, split at corners; neighbouring anchors closer
/// than `merge` times the local width merge (loose pieces may stray `slack` more for it, pieces
/// near corners keep `tight`).
fn fit_curve(
    samples: &[AxisSample],
    closed: bool,
    (accuracy, tight): (f64, f64),
    (merge, slack): (f64, f64),
) -> Vec<OutlineTriple> {
    let mut points: Vec<Point> = samples.iter().map(|s| s.point).collect();
    if closed {
        points.push(points[0]);
    }
    let found = corners(samples);
    let mut cuts = found.clone();
    cuts.extend(leg_starts(samples, &found, tight));
    cuts.insert(0, 0);
    cuts.push(points.len() - 1);
    cuts.sort_unstable();
    cuts.dedup();
    let mut path = BezPath::new();
    path.move_to(points[0]);
    // The merge distance of a stretch: its share of the stretch's median width.
    let width_of = |a: usize, b: usize| {
        let mut w: Vec<f64> = (a..=b)
            .map(|i| 2.0 * samples[i % samples.len()].radius)
            .collect();
        w.sort_by(f64::total_cmp);
        w[w.len() / 2]
    };
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        // Near a corner the fit keeps the tight accuracy (CENTRE-DEFAULT A, customer
        // 2026-11-17: corners keep their detail); the loose one applies between.
        let near = |at: usize| CORNER_REACH * 2.0 * samples[at].radius;
        let head = if found.contains(&a) {
            reach_index(&points, a, b, near(a), true)
        } else {
            a
        };
        let tail = if found.contains(&b) {
            reach_index(&points, a, b, near(b), false)
        } else {
            b
        };
        let mut stretch = BezPath::new();
        stretch.move_to(points[a]);
        let mut push = |x: usize, y: usize, accuracy: f64| {
            for element in fit_piece(&points[x..=y], accuracy)
                .elements()
                .iter()
                .skip(1)
            {
                stretch.push(*element);
            }
        };
        if head >= tail {
            push(a, b, tight);
        } else {
            if head > a {
                push(a, head, tight);
            }
            push(head, tail, accuracy);
            if tail < b {
                push(tail, b, tight);
            }
        }
        // Neighbouring anchors of the stretch closer than the merge distance merge, across
        // the pieces; spans near a corner keep the tight accuracy.
        let merged = if merge > 0.0 {
            let length = |x: usize, y: usize| {
                points[x..=y]
                    .windows(2)
                    .map(|w| (w[1] - w[0]).hypot())
                    .sum::<f64>()
            };
            let total = length(a, b);
            merge_stretch(
                &points[a..=b],
                stretch,
                Merge {
                    distance: merge * width_of(a, b),
                    accuracy: accuracy + slack,
                    tight,
                    tight_ends: (
                        if head > a { length(a, head) } else { 0.0 },
                        if tail < b && head < tail {
                            total - length(tail, b)
                        } else if head >= tail {
                            0.0
                        } else {
                            total
                        },
                    ),
                },
            )
        } else {
            stretch
        };
        for element in merged.elements().iter().skip(1) {
            path.push(*element);
        }
    }
    anchors_of(&path, closed)
}

/// `stretch` (the fit of the corner-free `points`) with its anchors merged as `merge` says, in
/// coordinates local to its first point (see [`fit_piece`]).
fn merge_stretch(points: &[Point], stretch: BezPath, merge: Merge) -> BezPath {
    let kept = cleaned(points, merge.tight);
    if kept.len() < 3 || stretch.elements().len() < 3 {
        return stretch;
    }
    let origin = kept[0].to_vec2();
    let local: Vec<Point> = kept.iter().map(|&p| p - origin).collect();
    let to_local = kurbo::Affine::translate(-origin);
    let local_path: BezPath = stretch.elements().iter().map(|&e| to_local * e).collect();
    let merged = merge_anchors(&local, &smooth(&local), local_path, merge);
    let shift = kurbo::Affine::translate(origin);
    merged.elements().iter().map(|&e| shift * e).collect()
}

/// Where the straight legs into each crest tip (a corner of width 0, CENTRE-THROUGH A) begin:
/// the farthest samples before and after it that lie within `accuracy` of the leg's line. The
/// curve is cut there too, so each leg is one straight piece.
fn leg_starts(samples: &[AxisSample], corners: &[usize], accuracy: f64) -> Vec<usize> {
    let n = samples.len();
    let mut out = Vec::new();
    for &tip in corners.iter().filter(|&&i| samples[i].radius == 0.0) {
        for step in [-1_isize, 1] {
            let next = |k: usize| k.checked_add_signed(step).filter(|&k| k < n);
            let Some(first) = next(tip) else { continue };
            let mut start = first;
            while let Some(k) = next(start) {
                let (from, to) = (samples[tip].point, samples[k].point);
                let chord = to - from;
                let straight = (k.min(tip)..=k.max(tip)).all(|m| {
                    (samples[m].point - from).cross(chord).abs() <= accuracy * chord.hypot()
                });
                if !straight {
                    break;
                }
                start = k;
            }
            out.push(start);
        }
    }
    out
}

/// The index in `a..=b` a distance `reach` along the points from `a` (`from_start`) or from `b`;
/// `a` or `b` itself for no reach (a crest's tip).
fn reach_index(points: &[Point], a: usize, b: usize, reach: f64, from_start: bool) -> usize {
    if reach <= 0.0 {
        return if from_start { a } else { b };
    }
    let mut travelled = 0.0;
    if from_start {
        for i in a + 1..=b {
            travelled += (points[i] - points[i - 1]).hypot();
            if travelled >= reach {
                return i;
            }
        }
        b
    } else {
        for i in (a..b).rev() {
            travelled += (points[i + 1] - points[i]).hypot();
            if travelled >= reach {
                return i;
            }
        }
        a
    }
}

/// Cubic Béziers through one corner-free piece. The simplifier runs in coordinates local to
/// the piece's first point (its moment integrals cancel catastrophically for a short piece far
/// from the origin) and behind [`super::safe_fit`] (its unbounded candidates overflow or stall
/// kurbo 0.13.1's root solver). A piece of fewer than three distinct points, or one within the
/// accuracy of its chord, is a line; a point at a local hairpin is dropped.
fn fit_piece(piece: &[Point], accuracy: f64) -> BezPath {
    fit_kept(&cleaned(piece, accuracy), accuracy)
}

/// The points of a piece the fit uses: steps shorter than half the accuracy dropped (noise to
/// the fit; their hairpins go with them), points at local hairpins dropped, coincident points
/// merged.
fn cleaned(piece: &[Point], accuracy: f64) -> Vec<Point> {
    // Steps shorter than half the accuracy are noise to the fit; their hairpins go with them.
    let mut kept: Vec<Point> = Vec::with_capacity(piece.len());
    for (i, &p) in piece.iter().enumerate() {
        let last = i + 1 == piece.len();
        match kept.last() {
            Some(&q) if !last && (p - q).hypot() < 0.5 * accuracy => {}
            _ => kept.push(p),
        }
    }
    if kept.len() >= 2 && kept.len() < piece.len() {
        let n = kept.len();
        if (kept[n - 1] - kept[n - 2]).hypot() < 0.5 * accuracy && n > 2 {
            kept.remove(n - 2);
        }
    } else {
        kept = piece.to_vec();
    }
    // A local hairpin (a turn sharper than 120 degrees between two steps) is a glitch of the
    // samples: the simplifier's candidates degenerate at it, so its point is dropped.
    while let Some(at) = (1..kept.len().saturating_sub(1)).find(|&i| {
        let (a, b) = (kept[i] - kept[i - 1], kept[i + 1] - kept[i]);
        a.dot(b) < HAIRPIN_COS * a.hypot() * b.hypot()
    }) {
        kept.remove(at);
    }
    // Near-coincident points give the Catmull-Rom curve zero-length pieces.
    kept.dedup_by(|a, b| (*a - *b).hypot() < COINCIDENT_MM);
    bridged(kept)
}

/// A step this many times longer than the piece's median step is a gap of the axis.
const GAP_STEPS: f64 = 3.0;

/// `points` with each gap (where the axis passes a junction cluster, its samples are missing)
/// filled by points on its chord at the median step: next to short steps a long one makes the
/// Catmull-Rom curve overshoot into a kink, which the simplifier answers with a cluster of
/// anchors (`adrs.md` Round A, hypothesis 5).
fn bridged(points: Vec<Point>) -> Vec<Point> {
    if points.len() < 4 {
        return points;
    }
    let mut steps: Vec<f64> = points.windows(2).map(|w| (w[1] - w[0]).hypot()).collect();
    steps.sort_by(f64::total_cmp);
    let median = steps[steps.len() / 2];
    if median <= 0.0 || steps[steps.len() - 1] <= GAP_STEPS * median {
        return points;
    }
    let count = points.len();
    // The direction at a point, from its neighbours, as a unit vector.
    let direction = |i: usize| {
        let along = points[(i + 1).min(count - 1)] - points[i.saturating_sub(1)];
        let length = along.hypot();
        if length > 0.0 {
            along / length
        } else {
            kurbo::Vec2::ZERO
        }
    };
    let mut out = Vec::with_capacity(count);
    for i in 0..count - 1 {
        let (from, to) = (points[i], points[i + 1]);
        out.push(from);
        let step = (to - from).hypot();
        if step > GAP_STEPS * median {
            // A cubic Hermite across the gap, its tangents the neighbours' directions at the
            // gap's length: it leaves and meets the axis as the axis runs.
            let (leave, meet) = (direction(i) * step, direction(i + 1) * step);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let parts = (step / median).ceil() as usize;
            for k in 1..parts {
                #[allow(clippy::cast_precision_loss)]
                let t = k as f64 / parts as f64;
                let (t2, t3) = (t * t, t * t * t);
                let point = from.to_vec2() * (2.0 * t3 - 3.0 * t2 + 1.0)
                    + leave * (t3 - 2.0 * t2 + t)
                    + to.to_vec2() * (-2.0 * t3 + 3.0 * t2)
                    + meet * (t3 - t2);
                out.push(point.to_point());
            }
        }
    }
    out.push(points[points.len() - 1]);
    out
}

/// [`fit_piece`] on its cleaned points.
fn fit_kept(piece: &[Point], accuracy: f64) -> BezPath {
    let origin = piece[0].to_vec2();
    let local: Vec<Point> = piece.iter().map(|&p| p - origin).collect();
    let (first, last) = (local[0], local[local.len() - 1]);
    let chord = last - first;
    let straight = local.len() < 3
        || (chord.hypot() > COINCIDENT_MM
            && local
                .iter()
                .all(|&p| (p - first).cross(chord).abs() / chord.hypot() <= accuracy));
    if straight {
        let mut line = BezPath::new();
        line.move_to(piece[0]);
        line.line_to(piece[piece.len() - 1]);
        return line;
    }
    let fitted = simplify(&smooth(&local), accuracy);
    // kurbo's error measure casts rays from the source only, so it misses a candidate that
    // runs on past the piece's end along a straight run (a crest's leg into its tip): a fit
    // that strays from the piece's points by more than `STRAY` times the accuracy is split.
    if local.len() >= 4 && strays(&fitted, &local, STRAY * accuracy) {
        let mid = piece.len() / 2;
        let mut out = fit_kept(&piece[..=mid], accuracy);
        for element in fit_kept(&piece[mid..], accuracy).elements().iter().skip(1) {
            out.push(*element);
        }
        return out;
    }
    let shift = kurbo::Affine::translate(origin);
    let mut out = BezPath::new();
    for element in fitted.elements() {
        out.push(shift * *element);
    }
    out
}

/// A Catmull-Rom curve through `points`: tangent-continuous, so the simplifier can merge its
/// pieces (it keeps every corner of a polyline).
fn smooth(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    let n = points.len();
    let tangent = |i: usize| {
        let (a, b) = (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)]);
        (b - a) / 6.0
    };
    for i in 0..n - 1 {
        path.curve_to(
            points[i] + tangent(i),
            points[i + 1] - tangent(i + 1),
            points[i + 1],
        );
    }
    path
}

/// The path's anchors as `(point, handle_in, handle_out)`.
fn anchors_of(path: &BezPath, closed: bool) -> Vec<OutlineTriple> {
    let doc = |p: Point| DocPoint::new(p.x, p.y);
    let mut anchors: Vec<OutlineTriple> = Vec::new();
    let mut last = Point::ZERO;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                anchors.push((doc(p), DocVec::ZERO, DocVec::ZERO));
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                if let Some(previous) = anchors.last_mut() {
                    previous.2 = DocVec::new(c1.x - last.x, c1.y - last.y);
                }
                anchors.push((doc(p), DocVec::new(c2.x - p.x, c2.y - p.y), DocVec::ZERO));
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                let c1 = last + (c - last) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                if let Some(previous) = anchors.last_mut() {
                    previous.2 = DocVec::new(c1.x - last.x, c1.y - last.y);
                }
                anchors.push((doc(p), DocVec::new(c2.x - p.x, c2.y - p.y), DocVec::ZERO));
                last = p;
            }
            PathEl::ClosePath => {}
        }
    }
    if closed
        && anchors.len() > 2
        && let Some(end) = anchors.pop()
    {
        anchors[0].1 = end.1;
    }
    anchors
}

#[cfg(test)]
mod piece_tests {
    use super::*;

    /// The piece of the traced whale (flattened at 0.0125 mm) on which the simplifier
    /// overflowed `1u64 << 64` in absolute coordinates.
    #[test]
    fn a_short_piece_far_from_the_origin_fits_without_overflow() {
        let piece = [
            Point::new(46.067_442_857_142_86, 138.096_807_142_857_13),
            Point::new(46.166_226_605_196_954, 138.183_016_122_646_3),
            Point::new(46.251_75, 138.085_750_000_000_02),
        ];
        let path = fit_piece(&piece, 0.025);
        let ends: Vec<Point> = path
            .elements()
            .iter()
            .filter_map(PathEl::end_point)
            .collect();
        assert!((ends[0] - piece[0]).hypot() < 1e-9);
        assert!((ends[ends.len() - 1] - piece[2]).hypot() < 1e-9);
        let segments = path.segments().count();
        assert!(segments >= 1, "{segments} segments");
        for p in piece {
            let nearest = path
                .segments()
                .map(|s| kurbo::ParamCurveNearest::nearest(&s, p, 1e-9).distance_sq)
                .fold(f64::INFINITY, f64::min)
                .sqrt();
            assert!(nearest <= 0.025 + 1e-9, "sample {p:?} is {nearest} away");
        }
    }

    #[test]
    fn degenerate_pieces_are_lines() {
        let at = Point::new(120.0, 80.0);
        for piece in [
            vec![at, at, at],
            vec![
                at,
                at + kurbo::Vec2::new(1e-9, 0.0),
                Point::new(121.0, 80.0),
            ],
            vec![at, Point::new(121.0, 80.01), Point::new(122.0, 80.0)],
        ] {
            let path = fit_piece(&piece, 0.025);
            assert_eq!(path.elements().len(), 2, "{piece:?}");
            assert_eq!(path.elements()[1].end_point(), Some(piece[piece.len() - 1]));
        }
    }

    /// A piece of `steps` (length, turn) from `start`, as the property test below builds it.
    fn walked(start: Point, steps: &[(f64, f64)]) -> Vec<Point> {
        let mut piece = vec![start];
        let mut heading = 0.0_f64;
        for &(length, turn) in steps {
            heading += turn;
            let last = piece[piece.len() - 1];
            piece.push(last + kurbo::Vec2::new(heading.cos(), heading.sin()) * length);
        }
        piece
    }

    /// A start, its steps and an accuracy.
    type Case = (Point, Vec<(f64, f64)>, f64);

    /// The two saved cases of `proptest-regressions/centre_line/fit.txt`: kurbo's candidate
    /// cubic had unbounded arms (a shift overflow in debug, a hang in release) before
    /// `safe_fit` split the range.
    #[test]
    fn the_saved_noisy_pieces_fit_without_hanging() {
        let cases: [Case; 2] = [
            (
                Point::new(0.0, 1_065.138_721_138_120_7),
                vec![
                    (0.01, -3.062_465_023_190_271_5),
                    (0.01, -1.685_421_801_429_187),
                    (0.281_204_464_656_485_63, 1.760_627_367_675_149_6),
                    (0.242_729_863_845_956_57, -0.452_612_336_410_226_9),
                    (0.426_468_119_909_004_13, -1.649_840_421_435_529),
                    (0.299_987_449_443_992_36, 1.380_667_848_413_750_5),
                    (0.216_826_204_702_339_6, 0.0),
                    (0.188_794_972_245_194_54, -2.615_569_443_653_767),
                ],
                0.146_722_644_121_186_5,
            ),
            (
                Point::new(0.0, 0.0),
                vec![
                    (0.423_509_162_144_970_75, 0.0),
                    (0.129_050_739_580_458_3, -1.124_185_097_282_606_8),
                    (0.323_062_945_871_448_46, -1.008_852_209_607_703_5),
                    (0.249_448_339_433_615_36, 1.780_923_594_729_882_5),
                    (0.078_902_756_625_578_43, 0.779_353_326_668_300_9),
                    (0.497_144_031_831_673_66, -0.652_823_514_426_568_8),
                    (0.343_676_819_692_803, 0.0),
                    (0.149_677_550_776_962_32, 1.468_377_900_734_901_2),
                    (0.268_048_546_077_603_5, 0.0),
                ],
                0.199_799_104_880_900_89,
            ),
        ];
        for (start, steps, accuracy) in cases {
            let piece = walked(start, &steps);
            let path = fit_piece(&piece, accuracy);
            let ends: Vec<Point> = path
                .elements()
                .iter()
                .filter_map(PathEl::end_point)
                .collect();
            assert!((ends[0] - piece[0]).hypot() < 1e-6);
            assert!((ends[ends.len() - 1] - piece[piece.len() - 1]).hypot() < 1e-6);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    proptest::proptest! {
        /// Noisy short pieces far from the origin fit without the kurbo 0.13.1 shift overflow
        /// (a trap in debug builds), and keep their ends.
        #[test]
        fn noisy_pieces_far_from_the_origin_fit(
            x in 0.0_f64..2000.0,
            y in 0.0_f64..2000.0,
            steps in proptest::collection::vec((0.01_f64..0.5, -3.2_f64..3.2), 2..12),
            accuracy in 0.01_f64..0.2,
        ) {
            let piece = walked(Point::new(x, y), &steps);
            let path = fit_piece(&piece, accuracy);
            let ends: Vec<Point> = path.elements().iter().filter_map(PathEl::end_point).collect();
            proptest::prop_assert!((ends[0] - piece[0]).hypot() < 1e-6);
            proptest::prop_assert!((ends[ends.len() - 1] - piece[piece.len() - 1]).hypot() < 1e-6);
        }

        /// The anchor merge of Optimise refits spans of the same noisy pieces with kurbo's
        /// `fit_to_cubic` (behind `safe_fit`), and keeps their ends.
        #[test]
        fn noisy_pieces_merge_their_anchors(
            x in 0.0_f64..2000.0,
            y in 0.0_f64..2000.0,
            steps in proptest::collection::vec((0.01_f64..0.5, -3.2_f64..3.2), 2..12),
            accuracy in 0.01_f64..0.2,
        ) {
            let piece = walked(Point::new(x, y), &steps);
            let stretch = fit_piece(&piece, accuracy);
            let merged = merge_stretch(
                &piece,
                stretch,
                Merge { distance: 10.0, accuracy: 1.5 * accuracy, tight: accuracy, tight_ends: (0.0, 1e9) },
            );
            let ends: Vec<Point> = merged.elements().iter().filter_map(PathEl::end_point).collect();
            proptest::prop_assert!((ends[0] - piece[0]).hypot() < 1e-6);
            proptest::prop_assert!((ends[ends.len() - 1] - piece[piece.len() - 1]).hypot() < 1e-6);
        }
    }
}
