//! The slice A geometry metrics of a trace against its truth (`specs/0089-trace-quality-and-colour`
//! criterion 3): two-sided boundary deviation sampled every 0.1 px, corner position error,
//! straightness or smoothness per annotated edge, self-intersections.

use kurbo::{BezPath, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathSeg, Point};

use super::metrics::{Grid, pixel_border, polylines};
use super::reference::Edge;

/// The sampling step along every outline, pixels.
pub const STEP: f64 = 0.1;

/// The segments of `paths` flattened to `tolerance`.
pub fn segments_of(paths: &[BezPath], tolerance: f64) -> Vec<(Point, Point)> {
    polylines(paths, tolerance)
        .iter()
        .flat_map(|l| l.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
        .collect()
}

/// Points every `STEP` along `paths` (each segment's start included, its end not).
pub fn samples_of(paths: &[BezPath]) -> Vec<Point> {
    let mut out = Vec::new();
    for path in paths {
        for seg in path.segments() {
            let steps = (seg.arclen(1e-3) / STEP).ceil().max(1.0) as usize;
            out.extend((0..steps).map(|i| seg.eval(i as f64 / steps as f64)));
        }
    }
    out
}

/// Points every `STEP` along line segments.
pub fn samples_along(segments: &[(Point, Point)]) -> Vec<Point> {
    let mut out = Vec::new();
    for &(a, b) in segments {
        let steps = (a.distance(b) / STEP).ceil().max(1.0) as usize;
        out.extend((0..steps).map(|i| a.lerp(b, i as f64 / steps as f64)));
    }
    out
}

/// The truth boundary as segments: the vector paths, or the border of the 4x truth scaled to
/// pixels (the 4x reference of criterion 3).
pub fn truth_segments(
    paths: Option<&[BezPath]>,
    truth_4x: &[bool],
    w: usize,
    h: usize,
) -> Vec<(Point, Point)> {
    match paths {
        Some(paths) => segments_of(paths, 1e-3),
        None => pixel_border(truth_4x, w * 4, h * 4)
            .into_iter()
            .map(|(a, b)| {
                (
                    Point::new(a.x / 4.0, a.y / 4.0),
                    Point::new(b.x / 4.0, b.y / 4.0),
                )
            })
            .collect(),
    }
}

/// Max and mean of the distances from `samples` to `segments`.
fn directed(samples: &[Point], segments: &[(Point, Point)], w: usize, h: usize) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    if segments.is_empty() {
        return (f64::INFINITY, f64::INFINITY);
    }
    let grid = Grid::new(segments, w, h, 4.0);
    let (mut max, mut sum) = (0.0_f64, 0.0);
    for &p in samples {
        let d = grid.distance(p);
        max = max.max(d);
        sum += d;
    }
    (max, sum / samples.len() as f64)
}

/// The two directions of the boundary deviation, pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Deviation {
    /// From the trace to the truth (extra or misplaced outline).
    pub trace_max: f64,
    pub trace_mean: f64,
    /// From the truth to the trace (lost or misplaced outline).
    pub truth_max: f64,
    pub truth_mean: f64,
}

/// The two-sided deviation of `trace` from the truth boundary `truth` (samples `truth_samples`).
pub fn deviation(
    trace: &[BezPath],
    truth: &[(Point, Point)],
    truth_samples: &[Point],
    w: usize,
    h: usize,
) -> Deviation {
    let (trace_max, trace_mean) = directed(&samples_of(trace), truth, w, h);
    let (truth_max, truth_mean) = directed(truth_samples, &segments_of(trace, 1e-3), w, h);
    Deviation {
        trace_max,
        trace_mean,
        truth_max,
        truth_mean,
    }
}

/// Corner position errors: the distance from each annotated corner to the nearest node.
pub fn corner_errors(trace: &[BezPath], corners: &[Point]) -> Vec<f64> {
    let nodes: Vec<Point> = trace
        .iter()
        .flat_map(|p| p.segments().map(|s| s.end()).collect::<Vec<_>>())
        .collect();
    corners
        .iter()
        .map(|c| {
            nodes
                .iter()
                .map(|n| n.distance(*c))
                .fold(f64::INFINITY, f64::min)
        })
        .collect()
}

/// The nearest point on the polyline `points`: its distance and its arc position.
fn on_polyline(p: Point, points: &[Point]) -> (f64, f64) {
    let (mut best, mut at, mut run) = (f64::INFINITY, 0.0, 0.0);
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let d = b - a;
        let len2 = d.hypot2();
        let t = if len2 == 0.0 {
            0.0
        } else {
            ((p - a).dot(d) / len2).clamp(0.0, 1.0)
        };
        let dist = p.distance(a + d * t);
        if dist < best {
            best = dist;
            at = run + t * len2.sqrt();
        }
        run += len2.sqrt();
    }
    (best, at)
}

fn length(points: &[Point]) -> f64 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// What one annotated edge measured.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgeResult {
    /// Less than half of the edge has trace within its window.
    pub lost: bool,
    /// The largest distance of the trace within the window from the true edge.
    pub deviation: f64,
    /// Trace segments touching the edge's inner part.
    pub segments: usize,
    /// Curvature sign flips along the edge (staircase wobble).
    pub flips: usize,
    /// Nodes on the edge with a tangent turn over 30 degrees (corners on a smooth curve).
    pub corners: usize,
}

/// Whether the tangent turns by more than 30 degrees from `a`'s end to `b`'s start.
fn is_corner(a: &PathSeg, b: &PathSeg) -> bool {
    let tangent = |s: &PathSeg, t: f64| -> kurbo::Vec2 {
        let d = match s {
            PathSeg::Line(l) => l.p1 - l.p0,
            PathSeg::Quad(q) => q.deriv().eval(t).to_vec2(),
            PathSeg::Cubic(c) => c.deriv().eval(t).to_vec2(),
        };
        if d.hypot() < 1e-9 {
            // A degenerate handle: use the chord.
            s.end() - s.start()
        } else {
            d
        }
    };
    let (u, v) = (tangent(a, 1.0), tangent(b, 0.0));
    let (lu, lv) = (u.hypot(), v.hypot());
    lu > 0.0 && lv > 0.0 && u.dot(v) / (lu * lv) < 30.0_f64.to_radians().cos()
}

/// Whether `seg` at `t` runs within 30 degrees of the straight edge `points` (either way).
fn along(seg: &PathSeg, t: f64, points: &[Point]) -> bool {
    let d = match seg {
        PathSeg::Line(l) => l.p1 - l.p0,
        PathSeg::Quad(q) => q.deriv().eval(t).to_vec2(),
        PathSeg::Cubic(c) => c.deriv().eval(t).to_vec2(),
    };
    let e = points[points.len() - 1] - points[0];
    let len = d.hypot() * e.hypot();
    len == 0.0 || d.dot(e).abs() / len >= 30.0_f64.to_radians().cos()
}

/// Measures `edge` against `trace`.
pub fn measure_edge(trace: &[BezPath], edge: &Edge) -> EdgeResult {
    let len = length(&edge.points);
    let window = (edge.clearance / 2.0).min(3.0);
    let margin = (0.1 * len).clamp(2.0, 10.0);
    let inner = len - 2.0 * margin;
    let mut result = EdgeResult::default();
    if inner <= 0.0 {
        return result;
    }
    let (lo, hi) = edge.points.iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let near_box = |p: Point| {
        p.x >= lo.x - window && p.x <= hi.x + window && p.y >= lo.y - window && p.y <= hi.y + window
    };
    let on = |p: Point| -> Option<(f64, f64)> {
        if !near_box(p) {
            return None;
        }
        let (d, at) = on_polyline(p, &edge.points);
        (d <= window && at >= margin && at <= len - margin).then_some((d, at))
    };
    let bins = inner.ceil() as usize;
    let mut hit = vec![false; bins.max(1)];
    for path in trace {
        let segs: Vec<PathSeg> = path.segments().collect();
        for (k, seg) in segs.iter().enumerate() {
            let steps = (seg.arclen(1e-3) / STEP).ceil().max(1.0) as usize;
            let mut touched = false;
            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                // Along a straight edge only trace running its way counts: near an acute tip
                // the other side passes within the window.
                if edge.straight && !along(seg, t, &edge.points) {
                    continue;
                }
                if let Some((d, at)) = on(seg.eval(t)) {
                    touched = true;
                    result.deviation = result.deviation.max(d);
                    let last = hit.len() - 1;
                    hit[((at - margin) as usize).min(last)] = true;
                }
            }
            result.segments += usize::from(touched);
            if !edge.straight
                && on(seg.end()).is_some()
                && is_corner(seg, &segs[(k + 1) % segs.len()])
            {
                result.corners += 1;
            }
        }
    }
    result.lost = (hit.iter().filter(|h| **h).count() as f64) < 0.5 * hit.len() as f64;
    if edge.straight {
        result.flips = flips(trace, &on);
    }
    result
}

/// Curvature sign flips of the flattened `trace` (0.02 px) between vertices on the edge.
fn flips(trace: &[BezPath], on: &impl Fn(Point) -> Option<(f64, f64)>) -> usize {
    let mut flips = 0;
    for line in polylines(trace, 0.02) {
        let mut last_sign = 0.0_f64;
        for t in line.windows(3) {
            if on(t[0]).is_none() || on(t[1]).is_none() || on(t[2]).is_none() {
                last_sign = 0.0;
                continue;
            }
            let (u, v) = (t[1] - t[0], t[2] - t[1]);
            let len = u.hypot() * v.hypot();
            if len == 0.0 || (u.cross(v) / len).abs() < 1e-3 {
                continue;
            }
            let sign = u.cross(v).signum();
            if last_sign != 0.0 && sign != last_sign {
                flips += 1;
            }
            last_sign = sign;
        }
    }
    flips
}

/// Whether the segments `a`–`b` and `c`–`d` cross properly (not merely touch: an end point on
/// the other segment within 1e-9 px, as two outlines sharing a pixel corner after the round
/// trip through millimetres, is a touch).
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o = |p: Point, q: Point, r: Point| (q - p).cross(r - p) / p.distance(q).max(1e-300);
    let (d1, d2) = (o(a, b, c), o(a, b, d));
    let (d3, d4) = (o(c, d, a), o(c, d, b));
    [d1, d2, d3, d4].iter().all(|v| v.abs() > 1e-9) && d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The crossings of the flattened outlines (0.05 px), within one outline and between outlines.
pub fn self_intersections(trace: &[BezPath], w: usize, h: usize) -> usize {
    self_intersection_points(trace, w, h).len()
}

/// Where the flattened outlines (0.05 px) cross, within one outline and between outlines: the
/// crossing point and the two outline indices.
pub fn self_intersection_points(
    trace: &[BezPath],
    w: usize,
    h: usize,
) -> Vec<(Point, usize, usize)> {
    let mut lines = polylines(trace, 0.05);
    // Flattening ends a cubic a rounding error away from the next segment's start; such a
    // piece of no length is no part of a crossing.
    for line in &mut lines {
        line.dedup_by(|b, a| a.distance(*b) < 1e-9);
    }
    // (line, index, a, b); a closed line's last segment neighbours its first.
    let mut segs: Vec<(usize, usize, Point, Point)> = Vec::new();
    let mut counts = Vec::new();
    for (l, line) in lines.iter().enumerate() {
        counts.push(line.len().saturating_sub(1));
        for (i, pair) in line.windows(2).enumerate() {
            segs.push((l, i, pair[0], pair[1]));
        }
    }
    let cell = 4.0;
    let (cols, rows) = (
        (w as f64 / cell) as usize + 3,
        (h as f64 / cell) as usize + 3,
    );
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); cols * rows];
    let index = |v: f64, n: usize| ((v / cell).floor() + 1.0).clamp(0.0, (n - 1) as f64) as usize;
    for (k, &(_, _, a, b)) in segs.iter().enumerate() {
        for r in index(a.y.min(b.y), rows)..=index(a.y.max(b.y), rows) {
            for c in index(a.x.min(b.x), cols)..=index(a.x.max(b.x), cols) {
                buckets[r * cols + c].push(k);
            }
        }
    }
    let mut pairs = Vec::new();
    for bucket in &buckets {
        for (n, &i) in bucket.iter().enumerate() {
            for &j in &bucket[n + 1..] {
                let (li, ii, a, b) = segs[i];
                let (lj, ij, c, d) = segs[j];
                if li == lj {
                    let last = counts[li] - 1;
                    let adjacent = ii.abs_diff(ij) == 1 || (ii.min(ij) == 0 && ii.max(ij) == last);
                    if adjacent {
                        continue;
                    }
                }
                if crosses(a, b, c, d) {
                    pairs.push((i.min(j), i.max(j)));
                }
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
        .into_iter()
        .map(|(i, j)| {
            let (li, _, a, b) = segs[i];
            let (lj, _, c, d) = segs[j];
            // Where `a`–`b` meets the line `c`–`d`.
            let t = (c - a).cross(d - c) / (b - a).cross(d - c);
            (a + (b - a) * t, li, lj)
        })
        .collect()
}
