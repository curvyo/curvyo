//! Node reduction after visioncortex (`adrs.md` 2026-10-28, second note, point 1): adjacent
//! segments of a closed outline become one cubic while it stays within a tolerance of them.
//!
//! Joints sharper than the Corners angle are never merged. A run grows greedily from a corner
//! (or the first anchor of a corner-free outline) while one cubic with the run's end points and
//! end tangent directions fits it: the two handle lengths by least squares on 8 samples per
//! original segment, chord-length parameters, at most 4 reparameterisation passes, accepted when
//! the sampled distance both ways is within the tolerance. Only `+ − × ÷` and `sqrt` (ADR 0023 §4:
//! run-to-run and native/wasm identical); no `hypot`, no kurbo fitting or nearest-point search.

use kurbo::{BezPath, PathEl, Point, Vec2};

/// Samples per original segment for the least-squares fit.
const FIT_SAMPLES: usize = 8;

/// Samples per original segment for the distance check, both ways.
const CHECK_SAMPLES: usize = 16;

/// Reparameterisation passes at most.
const REPARAMETERISE: usize = 4;

/// Segments in one run at most.
const MAX_RUN: usize = 48;

/// A merged cubic whose handles lie within this of the chord, in pixels, is a line (as in
/// `outline`).
const STRAIGHT_PX: f64 = 0.01;

/// One segment of an outline as a cubic; a line has its handles on its end points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Seg {
    p0: Point,
    c1: Point,
    c2: Point,
    p3: Point,
    line: bool,
}

impl Seg {
    fn eval(&self, t: f64) -> Point {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        Point::new(
            b0 * self.p0.x + b1 * self.c1.x + b2 * self.c2.x + b3 * self.p3.x,
            b0 * self.p0.y + b1 * self.c1.y + b2 * self.c2.y + b3 * self.p3.y,
        )
    }

    /// The direction leaving `p0` (not normalised; zero for a point).
    fn start_tangent(&self) -> Vec2 {
        [self.c1, self.c2, self.p3]
            .into_iter()
            .map(|p| p - self.p0)
            .find(|v| *v != Vec2::ZERO)
            .unwrap_or(Vec2::ZERO)
    }

    /// The direction arriving at `p3` (not normalised; zero for a point).
    fn end_tangent(&self) -> Vec2 {
        [self.c2, self.c1, self.p0]
            .into_iter()
            .map(|p| self.p3 - p)
            .find(|v| *v != Vec2::ZERO)
            .unwrap_or(Vec2::ZERO)
    }
}

fn length(v: Vec2) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

/// The segments of the first closed subpath of `path`, the closing line included when the last
/// segment does not end on the start.
fn segments(path: &BezPath) -> Vec<Seg> {
    let mut segs = Vec::new();
    let (mut start, mut current) = (Point::ZERO, Point::ZERO);
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                if !segs.is_empty() {
                    break;
                }
                (start, current) = (p, p);
            }
            PathEl::LineTo(p) => {
                segs.push(Seg {
                    p0: current,
                    c1: current,
                    c2: p,
                    p3: p,
                    line: true,
                });
                current = p;
            }
            PathEl::QuadTo(q, p) => {
                let c1 = current + (q - current) * (2.0 / 3.0);
                let c2 = p + (q - p) * (2.0 / 3.0);
                segs.push(Seg {
                    p0: current,
                    c1,
                    c2,
                    p3: p,
                    line: false,
                });
                current = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                segs.push(Seg {
                    p0: current,
                    c1,
                    c2,
                    p3: p,
                    line: false,
                });
                current = p;
            }
            PathEl::ClosePath => break,
        }
    }
    if current != start && !segs.is_empty() {
        segs.push(Seg {
            p0: current,
            c1: current,
            c2: start,
            p3: start,
            line: true,
        });
    }
    segs
}

/// Whether the joint from `before` into `after` turns less than the corner angle:
/// `dot ≥ corner_cos × |u| × |v|`.
fn smooth_joint(before: &Seg, after: &Seg, corner_cos: f64) -> bool {
    let (u, v) = (before.end_tangent(), after.start_tangent());
    let (lu, lv) = (length(u), length(v));
    if lu == 0.0 || lv == 0.0 {
        return false;
    }
    u.dot(v) >= corner_cos * lu * lv
}

/// `path` (a closed outline in pixels) with runs of adjacent segments merged into single cubics
/// that stay within `tolerance_px` of them, both ways; joints turning by the corner angle or more
/// (`corner_cos` is its cosine) stay anchors. Returns a clone when nothing merges.
pub(crate) fn reduce(path: &BezPath, tolerance_px: f64, corner_cos: f64) -> BezPath {
    let segs = segments(path);
    let n = segs.len();
    if n < 3 || tolerance_px <= 0.0 {
        return path.clone();
    }
    // joint i is between segment i - 1 and segment i.
    let smooth: Vec<bool> = (0..n)
        .map(|i| smooth_joint(&segs[(i + n - 1) % n], &segs[i], corner_cos))
        .collect();
    let first = smooth.iter().position(|s| !s).unwrap_or(0);
    let order: Vec<Seg> = (0..n).map(|k| segs[(first + k) % n]).collect();
    let mergeable = |k: usize| smooth[(first + k) % n];
    let mut out: Vec<Seg> = Vec::with_capacity(n);
    let mut k = 0;
    let mut merged_any = false;
    while k < n {
        let mut best = order[k];
        let mut end = k + 1;
        // A run never covers the whole outline: at least two segments stay.
        while end < n && mergeable(end) && end - k < MAX_RUN && !(k == 0 && end + 1 == n) {
            let Some(fitted) = fit(&order[k..=end], tolerance_px) else {
                break;
            };
            best = fitted;
            end += 1;
        }
        merged_any |= end - k > 1;
        out.push(best);
        k = end;
    }
    if !merged_any {
        return path.clone();
    }
    let mut reduced = BezPath::new();
    reduced.move_to(out[0].p0);
    for seg in &out {
        if seg.line {
            reduced.line_to(seg.p3);
        } else {
            reduced.curve_to(seg.c1, seg.c2, seg.p3);
        }
    }
    reduced.close_path();
    reduced
}

/// The samples of `run` (`per` per segment plus the last end point).
fn samples(run: &[Seg], per: usize) -> Vec<Point> {
    let mut points = Vec::with_capacity(run.len() * per + 1);
    #[allow(clippy::cast_precision_loss)] // small counts
    let step = 1.0 / per as f64;
    for seg in run {
        for m in 0..per {
            #[allow(clippy::cast_precision_loss)]
            points.push(seg.eval(m as f64 * step));
        }
    }
    if let Some(last) = run.last() {
        points.push(last.p3);
    }
    points
}

/// One cubic for `run` that keeps its end points and end tangent directions and lies within
/// `tolerance` of it both ways; `None` when there is none.
fn fit(run: &[Seg], tolerance: f64) -> Option<Seg> {
    let (first, last) = (run.first()?, run.last()?);
    let (p0, p3) = (first.p0, last.p3);
    let (s, e) = (first.start_tangent(), last.end_tangent());
    let (ls, le) = (length(s), length(e));
    if ls == 0.0 || le == 0.0 || p0 == p3 {
        return None;
    }
    let (t0, t1) = (s / ls, e / le);
    let points = samples(run, FIT_SAMPLES);
    let mut params = chord_parameters(&points)?;
    let original = samples(run, CHECK_SAMPLES);
    for pass in 0..=REPARAMETERISE {
        let candidate = handles(&points, &params, p0, p3, t0, t1);
        if within(&candidate, run.len(), &original, tolerance) {
            return Some(straightened(candidate));
        }
        if pass < REPARAMETERISE {
            for (u, q) in params.iter_mut().zip(&points) {
                *u = newton(&candidate, *q, *u);
            }
        }
    }
    None
}

/// Chord-length parameters of `points` in 0 to 1; `None` for no length.
fn chord_parameters(points: &[Point]) -> Option<Vec<f64>> {
    let mut params = Vec::with_capacity(points.len());
    let mut total = 0.0;
    params.push(0.0);
    for pair in points.windows(2) {
        total += length(pair[1] - pair[0]);
        params.push(total);
    }
    if total == 0.0 {
        return None;
    }
    for u in &mut params {
        *u /= total;
    }
    Some(params)
}

/// The cubic from `p0` along `t0` to `p3` along `t1` whose handle lengths fit `points` at
/// `params` by least squares; Schneider's chord / 3 when the system is singular or a length is
/// not positive.
fn handles(points: &[Point], params: &[f64], p0: Point, p3: Point, t0: Vec2, t1: Vec2) -> Seg {
    let (mut c11, mut c12, mut c22, mut x1, mut x2) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (q, &u) in points.iter().zip(params) {
        let s = 1.0 - u;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * u, 3.0 * s * u * u, u * u * u);
        let a1 = t0 * b1;
        let a2 = t1 * (-b2);
        let base = Point::new(
            (b0 + b1) * p0.x + (b2 + b3) * p3.x,
            (b0 + b1) * p0.y + (b2 + b3) * p3.y,
        );
        let r = *q - base;
        c11 += a1.dot(a1);
        c12 += a1.dot(a2);
        c22 += a2.dot(a2);
        x1 += a1.dot(r);
        x2 += a2.dot(r);
    }
    let det = c11 * c22 - c12 * c12;
    let chord = length(p3 - p0);
    let (mut a, mut b) = if det.abs() > 1e-12 * c11 * c22 {
        ((x1 * c22 - x2 * c12) / det, (c11 * x2 - c12 * x1) / det)
    } else {
        (chord / 3.0, chord / 3.0)
    };
    if a <= 1e-6 * chord || b <= 1e-6 * chord {
        (a, b) = (chord / 3.0, chord / 3.0);
    }
    Seg {
        p0,
        c1: p0 + t0 * a,
        c2: p3 - t1 * b,
        p3,
        line: false,
    }
}

/// One Newton step of the parameter of `target`'s nearest point on `curve`, from `param`, kept in
/// 0 to 1.
fn newton(curve: &Seg, target: Point, param: f64) -> f64 {
    let (q0, q1, q2, q3) = (
        curve.p0.to_vec2(),
        curve.c1.to_vec2(),
        curve.c2.to_vec2(),
        curve.p3.to_vec2(),
    );
    let (u, s) = (param, 1.0 - param);
    let d1 = (q1 - q0) * (3.0 * s * s) + (q2 - q1) * (6.0 * s * u) + (q3 - q2) * (3.0 * u * u);
    let d2 = (q2 - q1 * 2.0 + q0) * (6.0 * s) + (q3 - q2 * 2.0 + q1) * (6.0 * u);
    let r = curve.eval(u) - target;
    let denominator = d1.dot(d1) + r.dot(d2);
    if denominator == 0.0 {
        return u;
    }
    (u - r.dot(d1) / denominator).clamp(0.0, 1.0)
}

/// Whether `candidate` (standing for `segments` originals) and the `original` samples lie
/// within `tolerance` of each other, both ways, each sampled point to the other's polyline.
fn within(candidate: &Seg, segments: usize, original: &[Point], tolerance: f64) -> bool {
    let steps = segments * CHECK_SAMPLES;
    #[allow(clippy::cast_precision_loss)] // small counts
    let ours: Vec<Point> = (0..=steps)
        .map(|i| candidate.eval(i as f64 / steps as f64))
        .collect();
    directed_within(&ours, original, tolerance) && directed_within(original, &ours, tolerance)
}

/// Whether every point of `from` lies within `tolerance` of the polyline `to`; the search starts
/// at the matching share of `to` and stops at the first segment near enough.
fn directed_within(from: &[Point], to: &[Point], tolerance: f64) -> bool {
    if to.len() < 2 {
        return false;
    }
    let squared = tolerance * tolerance;
    let last = to.len() - 2;
    from.iter().enumerate().all(|(i, &p)| {
        let guess = i * last / (from.len() - 1).max(1);
        (0..=last).any(|offset| {
            let near = |j: usize| distance_squared(p, to[j], to[j + 1]) <= squared;
            (guess + offset <= last && near(guess + offset))
                || (offset <= guess && near(guess - offset))
        })
    })
}

/// The squared distance from `point` to the segment `from`–`to`.
fn distance_squared(point: Point, from: Point, to: Point) -> f64 {
    let along = to - from;
    let offset = point - from;
    let squared = along.dot(along);
    let share = if squared == 0.0 {
        0.0
    } else {
        (offset.dot(along) / squared).clamp(0.0, 1.0)
    };
    let rest = offset - along * share;
    rest.dot(rest)
}

/// `seg` as a line when both handles lie within [`STRAIGHT_PX`] of its chord.
fn straightened(seg: Seg) -> Seg {
    let limit = STRAIGHT_PX * STRAIGHT_PX;
    if distance_squared(seg.c1, seg.p0, seg.p3) <= limit
        && distance_squared(seg.c2, seg.p0, seg.p3) <= limit
    {
        Seg {
            c1: seg.p0,
            c2: seg.p3,
            line: true,
            ..seg
        }
    } else {
        seg
    }
}

#[cfg(test)]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
mod tests {
    use super::*;
    use crate::outline::node_count;
    use kurbo::{ParamCurve, Shape};

    /// A closed circle of `n` cubic arcs (radius `r` around `c`), each arc's handles of the
    /// standard length.
    fn circle(c: Point, r: f64, n: usize) -> BezPath {
        let step = std::f64::consts::TAU / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
        let at = |i: usize| {
            let a = step * i as f64;
            (
                c + Vec2::new(a.cos(), a.sin()) * r,
                Vec2::new(-a.sin(), a.cos()),
            )
        };
        let mut path = BezPath::new();
        path.move_to(at(0).0);
        for i in 0..n {
            let ((p0, d0), (p3, d3)) = (at(i), at(i + 1));
            path.curve_to(
                p0 + d0 * k,
                p3 - d3 * k,
                if i + 1 == n { at(0).0 } else { p3 },
            );
        }
        path.close_path();
        path
    }

    /// The largest distance from samples every 0.05 px of `a` to the flattened `b`.
    fn directed(a: &BezPath, b: &BezPath) -> f64 {
        let mut line = Vec::new();
        kurbo::flatten(b.iter(), 1e-4, |el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => line.push(p),
            PathEl::ClosePath => line.push(line[0]),
            _ => {}
        });
        let mut worst = 0.0_f64;
        for seg in a.segments() {
            let steps = (kurbo::ParamCurveArclen::arclen(&seg, 1e-6) / 0.05).ceil() as usize;
            for i in 0..=steps {
                let p = seg.eval(i as f64 / steps as f64);
                let d = line
                    .windows(2)
                    .map(|w| distance_squared(p, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt();
                worst = worst.max(d);
            }
        }
        worst
    }

    const COS_60: f64 = 0.5;

    #[test]
    fn a_circle_of_many_arcs_becomes_few_within_the_tolerance() {
        let original = circle(Point::new(100.0, 100.0), 40.0, 32);
        let reduced = reduce(&original, 0.5, COS_60);
        let nodes = node_count(&reduced);
        assert!((2..32).contains(&nodes), "{nodes} nodes");
        assert!(directed(&reduced, &original) <= 0.5 + 1e-3);
        assert!(directed(&original, &reduced) <= 0.5 + 1e-3);
        assert!((reduced.area() - original.area()).abs() / original.area().abs() < 0.01);
    }

    #[test]
    fn a_tighter_tolerance_keeps_more_nodes() {
        let original = circle(Point::new(100.0, 100.0), 40.0, 32);
        let loose = node_count(&reduce(&original, 0.5, COS_60));
        let tight = node_count(&reduce(&original, 0.01, COS_60));
        assert!(tight > loose, "{tight} vs {loose}");
        assert!(directed(&reduce(&original, 0.01, COS_60), &original) <= 0.01 + 1e-4);
    }

    #[test]
    fn sharp_corners_stay_and_a_square_is_unchanged() {
        let mut square = BezPath::new();
        square.move_to((0.0, 0.0));
        square.line_to((10.0, 0.0));
        square.line_to((10.0, 10.0));
        square.line_to((0.0, 10.0));
        square.close_path();
        assert_eq!(reduce(&square, 1.0, COS_60), square);
    }

    #[test]
    fn collinear_lines_merge_into_one_line() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        for x in 1..=10 {
            path.line_to((f64::from(x), 0.0));
        }
        path.line_to((10.0, 10.0));
        path.line_to((0.0, 10.0));
        path.close_path();
        let reduced = reduce(&path, 0.1, COS_60);
        assert_eq!(node_count(&reduced), 4, "{reduced:?}");
        assert!(
            reduced
                .elements()
                .iter()
                .all(|e| !matches!(e, PathEl::CurveTo(..)))
        );
        assert!((reduced.area() - path.area()).abs() < 1e-9);
    }

    #[test]
    fn a_corner_between_two_arcs_stays_an_anchor() {
        // A half disc: the arc of 16 cubics, then the diameter; the two 90° joints are corners.
        let full = circle(Point::ZERO, 20.0, 32);
        let mut half = BezPath::new();
        let mut segs = full.segments().take(16);
        let first = segs.next().unwrap();
        half.move_to(first.start());
        for seg in std::iter::once(first).chain(segs) {
            if let kurbo::PathSeg::Cubic(c) = seg {
                half.curve_to(c.p1, c.p2, c.p3);
            }
        }
        half.close_path();
        let reduced = reduce(&half, 0.2, COS_60);
        let anchors: Vec<Point> = reduced.segments().map(|s| s.start()).collect();
        assert!(
            anchors
                .iter()
                .any(|p| (p.x - 20.0).abs() < 1e-9 && p.y.abs() < 1e-9)
        );
        assert!(
            anchors
                .iter()
                .any(|p| (p.x + 20.0).abs() < 1e-9 && p.y.abs() < 1e-6)
        );
        assert!(node_count(&reduced) < 17);
    }

    #[test]
    fn a_run_has_at_most_48_segments() {
        let original = circle(Point::ZERO, 1000.0, 200);
        let reduced = reduce(&original, 1000.0, COS_60);
        assert!(
            node_count(&reduced) >= 200usize.div_ceil(MAX_RUN),
            "{reduced:?}"
        );
    }

    #[test]
    fn no_tolerance_changes_nothing() {
        let original = circle(Point::ZERO, 10.0, 12);
        assert_eq!(reduce(&original, 0.0, COS_60), original);
    }

    #[test]
    fn reduction_is_identical_on_every_run() {
        let original = circle(Point::new(3.3, 7.7), 33.3, 40);
        assert_eq!(
            reduce(&original, 0.3, COS_60),
            reduce(&original, 0.3, COS_60)
        );
    }
}
