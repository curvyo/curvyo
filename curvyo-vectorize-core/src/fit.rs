//! Fits cubic Béziers to a chain between two nodes within the deviation tolerance (`specs/0089`
//! adrs.md §5, trial D2 candidate (a)): Schneider's least-squares fit with fixed end tangent
//! directions, up to four Newton reparameterisations, then a split at the point of largest
//! deviation with one shared tangent (G1). A piece is accepted only after the measured check
//! both ways (§4): every chain point to the curve at its parameter, and the curve sampled every
//! 0.1 px to the chain near that parameter; both over-estimate the true distance, so an
//! accepted piece is never further than the tolerance. Basic operations and `sqrt` only.

use kurbo::{Point, Vec2};

use crate::line::{distance, segment_distance, unit};

/// The curve is sampled at most this far apart for the check, px.
const SAMPLE_PX: f64 = 0.1;

/// Chain segments on each side of a sample's parameter that the check looks at.
const WINDOW: usize = 4;

/// Newton reparameterisations per piece.
const REPARAMETERISE: usize = 4;

/// A cubic whose handles lie within this of its chord, px, is a line.
const STRAIGHT_PX: f64 = 0.01;

/// The arc over which a free or split tangent is estimated, source px.
pub(crate) const TANGENT_ARC: f64 = 5.0;

/// One segment of a fitted outline, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Seg {
    Line(Point, Point),
    Cubic(Point, Point, Point, Point),
}

/// The point of the cubic `b` at `t`.
pub(crate) fn eval(b: &[Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    let (w0, w1, w2, w3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
    Point::new(
        w0 * b[0].x + w1 * b[1].x + w2 * b[2].x + w3 * b[3].x,
        w0 * b[0].y + w1 * b[1].y + w2 * b[2].y + w3 * b[3].y,
    )
}

/// The first derivative of the cubic `b` at `t`.
fn derivative(b: &[Point; 4], t: f64) -> Vec2 {
    let s = 1.0 - t;
    (b[1] - b[0]) * (3.0 * s * s) + (b[2] - b[1]) * (6.0 * s * t) + (b[3] - b[2]) * (3.0 * t * t)
}

/// The second derivative of the cubic `b` at `t`.
fn second_derivative(b: &[Point; 4], t: f64) -> Vec2 {
    let s = 1.0 - t;
    let d1 = b[2].to_vec2() - b[1].to_vec2() * 2.0 + b[0].to_vec2();
    let d2 = b[3].to_vec2() - b[2].to_vec2() * 2.0 + b[1].to_vec2();
    (d1 * s + d2 * t) * 6.0
}

/// The length of the control polygon of `b` (at least its arc length).
pub(crate) fn control_length(b: &[Point; 4]) -> f64 {
    distance(b[0], b[1]) + distance(b[1], b[2]) + distance(b[2], b[3])
}

/// A cubic with its handles near its chord as a line, else as a cubic.
pub(crate) fn segment(b: [Point; 4]) -> Seg {
    if segment_distance(b[1], b[0], b[3]) <= STRAIGHT_PX
        && segment_distance(b[2], b[0], b[3]) <= STRAIGHT_PX
    {
        Seg::Line(b[0], b[3])
    } else {
        Seg::Cubic(b[0], b[1], b[2], b[3])
    }
}

/// The point at arc length `at` along the polyline `q` (clamped to its ends).
pub(crate) fn point_at(q: &[Point], at: f64) -> Point {
    let mut run = 0.0;
    for w in q.windows(2) {
        let d = distance(w[0], w[1]);
        if run + d >= at && d > 0.0 {
            return w[0] + (w[1] - w[0]) * ((at - run) / d).clamp(0.0, 1.0);
        }
        run += d;
    }
    if at <= 0.0 { q[0] } else { q[q.len() - 1] }
}

/// The arc length of the polyline `q`.
pub(crate) fn arc_length(q: &[Point]) -> f64 {
    q.windows(2).map(|w| distance(w[0], w[1])).sum()
}

/// One piece of a chain to fit: the chain points `q`, the tolerance of each of its segments
/// (`tols`, one fewer than points; a part is fitted within the smallest over its segments and
/// the piece's tolerance), the nodes it runs between (the chain's first and last point, or the
/// feet of a neighbouring line on them) and the unit tangents at both nodes, pointing into the
/// piece (`None`: free, estimated from the chain).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Piece<'a> {
    pub q: &'a [Point],
    pub tols: &'a [f64],
    pub start: Point,
    pub end: Point,
    pub start_tangent: Option<Vec2>,
    pub end_tangent: Option<Vec2>,
}

/// The cubics (and lines) of `piece` within `tolerance`, appended to `out` in order; `arc` is
/// the tangent estimation arc in traced pixels.
pub(crate) fn fit_cubics(piece: Piece<'_>, tolerance: f64, arc: f64, out: &mut Vec<Seg>) {
    // (first, last) index of q, start, end, tangents; the left piece is popped first.
    let mut stack = vec![(
        0,
        piece.q.len() - 1,
        piece.start,
        piece.end,
        piece.start_tangent,
        piece.end_tangent,
    )];
    while let Some((i, j, start, end, t0, t1)) = stack.pop() {
        let q = &piece.q[i..=j];
        let tolerance = piece.tols[i..j].iter().copied().fold(tolerance, f64::min);
        let t0 = t0.or_else(|| unit(point_at(q, arc.min(arc_length(q) / 3.0)) - start));
        let t1 = t1.or_else(|| {
            let length = arc_length(q);
            unit(point_at(q, length - arc.min(length / 3.0)) - end)
        });
        let (Some(t0), Some(t1)) = (t0, t1) else {
            out.push(Seg::Line(start, end));
            continue;
        };
        match fit_one(q, start, end, t0, t1, tolerance) {
            Ok(seg) => out.push(seg),
            Err(split) => {
                let k = i + split;
                let around = &piece.q[i..=j];
                let at: f64 = arc_length(&around[..=split]);
                let centre = unit(
                    point_at(around, at + arc.min((arc_length(around) - at) / 2.0))
                        - point_at(around, at - arc.min(at / 2.0)),
                );
                let node = piece.q[k];
                stack.push((k, j, node, end, centre, Some(t1)));
                stack.push((i, k, start, node, Some(t0), centre.map(|c| -c)));
            }
        }
    }
}

/// The chord-length parameters of `start`, the inner points of `q` and `end`.
fn chord_parameters(q: &[Point], start: Point, end: Point) -> Vec<f64> {
    let n = q.len();
    let mut u = Vec::with_capacity(n);
    let mut run = 0.0;
    let mut previous = start;
    for (k, &p) in q.iter().enumerate() {
        let p = match k {
            0 => start,
            _ if k == n - 1 => end,
            _ => p,
        };
        run += distance(previous, p);
        u.push(run);
        previous = p;
    }
    let total = run;
    if total > 0.0 {
        for v in &mut u {
            *v /= total;
        }
    }
    u
}

/// One cubic from `start` to `end` with tangents `t0`, `t1` for `q`, or the index to split at.
fn fit_one(
    q: &[Point],
    start: Point,
    end: Point,
    t0: Vec2,
    t1: Vec2,
    tolerance: f64,
) -> Result<Seg, usize> {
    let n = q.len();
    if n <= 2 {
        return Ok(Seg::Line(start, end));
    }
    let chord = chord_parameters(q, start, end);
    let mut u = chord.clone();
    let mut split = n / 2;
    for pass in 0..=REPARAMETERISE {
        if pass > 0 {
            reparameterise(q, &generate(q, &u, start, end, t0, t1), &mut u);
        }
        let b = generate(q, &u, start, end, t0, t1);
        let (error, worst) = max_error(q, &b, &u, start, end);
        split = worst;
        if error <= tolerance && well_formed(&b) && curve_follows(q, &b, &chord, tolerance) {
            return Ok(segment(b));
        }
        if error > 4.0 * tolerance {
            break;
        }
    }
    // Not near an end: a split there leaves a sliver whose tangent follows the pixel noise.
    let margin = (n / 4).max(1);
    Err(split.clamp(margin, n - 1 - margin))
}

/// Whether the cubic `b` has no loop or cusp from its handles: neither handle is longer than
/// the chord and the two handle legs do not cross.
fn well_formed(b: &[Point; 4]) -> bool {
    let chord = distance(b[0], b[3]);
    let orient = |p: Point, q: Point, r: Point| (q - p).cross(r - p);
    let crossing = orient(b[0], b[1], b[2]) * orient(b[0], b[1], b[3]) < 0.0
        && orient(b[2], b[3], b[0]) * orient(b[2], b[3], b[1]) < 0.0;
    distance(b[0], b[1]) <= chord && distance(b[2], b[3]) <= chord && !crossing
}

/// Schneider's least-squares handle lengths for the parameters `u`, or a third of the chord
/// when they come out degenerate.
fn generate(q: &[Point], u: &[f64], start: Point, end: Point, t0: Vec2, t1: Vec2) -> [Point; 4] {
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&p, &t) in q.iter().zip(u) {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        let (a0, a1) = (t0 * b1, t1 * b2);
        c00 += a0.dot(a0);
        c01 += a0.dot(a1);
        c11 += a1.dot(a1);
        let rest = p - (start.to_vec2() * (b0 + b1) + end.to_vec2() * (b2 + b3));
        x0 += a0.dot(rest.to_vec2());
        x1 += a1.dot(rest.to_vec2());
    }
    let det = c00 * c11 - c01 * c01;
    let chord = distance(start, end);
    let (mut alpha0, mut alpha1) = if det.abs() > 1e-12 {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (0.0, 0.0)
    };
    let epsilon = 1e-6 * chord;
    if alpha0 < epsilon || alpha1 < epsilon {
        alpha0 = chord / 3.0;
        alpha1 = chord / 3.0;
    }
    [start, start + t0 * alpha0, end + t1 * alpha1, end]
}

/// One Newton step per point towards its nearest parameter on `bez`.
fn reparameterise(chain: &[Point], bez: &[Point; 4], params: &mut [f64]) {
    for k in 1..chain.len() - 1 {
        let at = params[k];
        let off = eval(bez, at) - chain[k];
        let d1 = derivative(bez, at);
        let numerator = off.dot(d1);
        let denominator = d1.dot(d1) + off.dot(second_derivative(bez, at));
        if denominator.abs() > 1e-12 {
            params[k] = (at - numerator / denominator).clamp(0.0, 1.0);
        }
    }
}

/// The largest distance of a chain point from its point on `bez` (the nodes' offsets
/// included) and the inner index where it is.
fn max_error(
    chain: &[Point],
    bez: &[Point; 4],
    params: &[f64],
    start: Point,
    end: Point,
) -> (f64, usize) {
    let len = chain.len();
    let mut worst = (
        distance(chain[0], start).max(distance(chain[len - 1], end)),
        len / 2,
    );
    for k in 1..len - 1 {
        let dist = distance(eval(bez, params[k]), chain[k]);
        if dist > worst.0 {
            worst = (dist, k);
        }
    }
    worst
}

/// Whether every point of `bez`, sampled every [`SAMPLE_PX`], lies within `tolerance` of the
/// chain near its chord parameter (`chord`, increasing).
fn curve_follows(chain: &[Point], bez: &[Point; 4], chord: &[f64], tolerance: f64) -> bool {
    let len = chain.len();
    // invariant: the control polygon is short (a piece of one outline), so this fits `usize`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let samples = ((control_length(bez) / SAMPLE_PX).ceil() as usize).max(1);
    #[allow(clippy::cast_precision_loss)] // a few thousand samples
    let step = 1.0 / samples as f64;
    let mut at = 0;
    for sample in 0..=samples {
        #[allow(clippy::cast_precision_loss)] // a few thousand samples
        let param = sample as f64 * step;
        while at + 1 < len - 1 && chord[at + 1] <= param {
            at += 1;
        }
        let point = eval(bez, param);
        let from = at.saturating_sub(WINDOW);
        let to = (at + WINDOW).min(len - 2);
        let near = (from..=to)
            .map(|k| segment_distance(point, chain[k], chain[k + 1]))
            .fold(f64::INFINITY, f64::min);
        if near > tolerance {
            return false;
        }
    }
    true
}

/// The deviation of `b` from its chain, for tests: the largest distance of its samples from
/// the polyline `q`.
#[cfg(test)]
pub(crate) fn sampled_deviation(q: &[Point], b: &[Point; 4]) -> f64 {
    (0..=1000)
        .map(|s| {
            let p = eval(b, f64::from(s) / 1000.0);
            q.windows(2)
                .map(|w| segment_distance(p, w[0], w[1]))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    fn arc_points(radius: f64, from: f64, to: f64, count: u32) -> Vec<Point> {
        (0..=count)
            .map(|k| {
                let angle = from + (to - from) * f64::from(k) / f64::from(count);
                Point::new(radius * angle.cos(), radius * angle.sin())
            })
            .collect()
    }

    #[test]
    fn a_quarter_circle_is_one_cubic_within_the_tolerance() {
        let q = arc_points(50.0, 0.0, std::f64::consts::FRAC_PI_2, 80);
        let mut out = Vec::new();
        let tols = vec![1.0; q.len() - 1];
        let piece = Piece {
            q: &q,
            tols: &tols,
            start: q[0],
            end: q[80],
            start_tangent: Some(Vec2::new(0.0, 1.0)),
            end_tangent: Some(Vec2::new(1.0, 0.0)),
        };
        fit_cubics(piece, 0.1, 2.0, &mut out);
        assert_eq!(out.len(), 1, "{out:?}");
        let Seg::Cubic(a, b, c, d) = out[0] else {
            panic!("{out:?}")
        };
        assert!(sampled_deviation(&q, &[a, b, c, d]) <= 0.1);
    }

    #[test]
    fn a_half_circle_splits_and_stays_g1() {
        let q = arc_points(30.0, 0.0, std::f64::consts::PI, 120);
        let mut out = Vec::new();
        let tols = vec![1.0; q.len() - 1];
        let piece = Piece {
            q: &q,
            tols: &tols,
            start: q[0],
            end: q[120],
            start_tangent: None,
            end_tangent: None,
        };
        fit_cubics(piece, 0.05, 2.0, &mut out);
        assert!(out.len() >= 2, "{out:?}");
        for w in out.windows(2) {
            let (Seg::Cubic(_, _, c2, p), Seg::Cubic(p2, c3, _, _)) = (w[0], w[1]) else {
                panic!("{out:?}")
            };
            assert_eq!(p, p2);
            assert!(
                (c2 - p).cross(c3 - p).abs() < 1e-9 * (c2 - p).hypot() * (c3 - p).hypot() + 1e-12
            );
        }
        for seg in &out {
            if let Seg::Cubic(a, b, c, d) = *seg {
                assert!(sampled_deviation(&q, &[a, b, c, d]) <= 0.05 + 1e-9);
            }
        }
    }
}
