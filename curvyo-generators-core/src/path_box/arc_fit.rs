//! Replaces a curved cubic by arcs within 0.01 mm of it (ADR 0083 §13.2).
//!
//! Screen coordinates; a positive sweep turns clockwise on screen, as `Shape::Bend`.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::Point;
use curvyo_extension_core::Cubic;

use crate::footprint::Shape;

/// The distance the arcs may stand from the cubic (mm); the fit aims a little below it so the
/// samples between two tests stay within the 0.01 mm of the ADR.
#[cfg(test)]
pub(crate) const FIT_TOLERANCE_MM: f64 = 0.01;
const ACCEPT_MM: f64 = 0.008;
/// Samples of one piece tested against its biarc.
const SAMPLES: usize = 16;
/// A piece is halved at most this often (64 biarcs per inflection-free piece).
const MAX_HALVINGS: u32 = 6;
/// An arc above this radius (mm) is emitted as a straight.
const STRAIGHT_RADIUS_MM: f64 = 1e6;
/// A derivative shorter than this (mm per unit parameter) is a cusp.
const CUSP_DERIVATIVE: f64 = 1e-9;
/// Roots this close to an end of the parameter range are no split.
const END_PARAMETER: f64 = 1e-6;

type V = (f64, f64);
/// The directions an arc fit is forced to at the start and the end of a cubic.
pub(crate) type Directions = (Option<V>, Option<V>);

fn v(p: Point) -> V {
    (p.x, p.y)
}

fn pt(a: V) -> Point {
    Point::new(a.0, a.1)
}

fn sub(a: V, b: V) -> V {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: V, b: V) -> V {
    (a.0 + b.0, a.1 + b.1)
}

fn scale(a: V, k: f64) -> V {
    (a.0 * k, a.1 * k)
}

fn cross(a: V, b: V) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn dot(a: V, b: V) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn len(a: V) -> f64 {
    libm::hypot(a.0, a.1)
}

fn lerp(a: V, b: V, t: f64) -> V {
    add(scale(a, 1.0 - t), scale(b, t))
}

/// A cubic as four points.
#[derive(Debug, Clone, Copy)]
struct Bez([V; 4]);

impl Bez {
    fn of(from: Point, c: &Cubic) -> Self {
        Self([v(from), v(c.c1), v(c.c2), v(c.to)])
    }

    fn at(&self, t: f64) -> V {
        let [p0, p1, p2, p3] = self.0;
        let (a, b, c) = (lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t));
        lerp(lerp(a, b, t), lerp(b, c, t), t)
    }

    fn derivative(&self, t: f64) -> V {
        let [p0, p1, p2, p3] = self.0;
        let (a, b, c) = (sub(p1, p0), sub(p2, p1), sub(p3, p2));
        scale(lerp(lerp(a, b, t), lerp(b, c, t), t), 3.0)
    }

    /// De Casteljau: the parts before and after `t`.
    fn split(&self, t: f64) -> (Self, Self) {
        let [p0, p1, p2, p3] = self.0;
        let (a, b, c) = (lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t));
        let (d, e) = (lerp(a, b, t), lerp(b, c, t));
        let m = lerp(d, e, t);
        (Self([p0, a, d, m]), Self([m, e, c, p3]))
    }

    /// The unit direction of travel at the start or the end; the first handle off the node.
    fn tangent(&self, at_end: bool) -> V {
        let [p0, p1, p2, p3] = self.0;
        let candidates = if at_end {
            [sub(p3, p2), sub(p3, p1), sub(p3, p0)]
        } else {
            [sub(p1, p0), sub(p2, p0), sub(p3, p0)]
        };
        let found = candidates
            .into_iter()
            .find(|a| len(*a) > 1e-9)
            .unwrap_or((1.0, 0.0));
        scale(found, 1.0 / len(found))
    }
}

/// The parameters in (0, 1) where `cross(B', B'')` vanishes: the inflections and the cusps.
fn flex_parameters(b: &Bez) -> Vec<f64> {
    let [p0, p1, p2, p3] = b.0;
    let a = add(sub(p3, scale(p2, 3.0)), sub(scale(p1, 3.0), p0));
    let q = add(sub(p2, scale(p1, 2.0)), p0);
    let bb = scale(q, 3.0);
    let c = scale(sub(p1, p0), 3.0);
    let (c2, c1, c0) = (-6.0 * cross(a, bb), 6.0 * cross(c, a), 2.0 * cross(c, bb));
    let scale_of = c2.abs().max(c1.abs()).max(c0.abs());
    let mut roots = Vec::new();
    if scale_of < 1e-12 {
        return roots;
    }
    if c2.abs() < 1e-12 * scale_of {
        if c1.abs() > 1e-12 * scale_of {
            roots.push(-c0 / c1);
        }
    } else {
        let disc = c1 * c1 - 4.0 * c2 * c0;
        if disc >= 0.0 {
            let root = libm::sqrt(disc);
            roots.push((-c1 - root) / (2.0 * c2));
            roots.push((-c1 + root) / (2.0 * c2));
        }
    }
    roots.retain(|t| *t > END_PARAMETER && *t < 1.0 - END_PARAMETER);
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    roots
}

/// The parameters where the derivative of the cubic from `from` vanishes (cusps, ADR 0083 §13.2).
pub(crate) fn cusps(from: Point, cubic: &Cubic) -> Vec<f64> {
    let b = Bez::of(from, cubic);
    flex_parameters(&b)
        .into_iter()
        .filter(|t| len(b.derivative(*t)) < CUSP_DERIVATIVE)
        .collect()
}

/// The cubic cut at the parameter `t`: the two halves as cubics (the second one's start is the
/// first one's end).
pub(crate) fn cut(from: Point, cubic: &Cubic, t: f64) -> (Cubic, Cubic) {
    let (a, b) = Bez::of(from, cubic).split(t);
    let to_cubic = |z: Bez| Cubic {
        c1: pt(z.0[1]),
        c2: pt(z.0[2]),
        to: pt(z.0[3]),
    };
    (to_cubic(a), to_cubic(b))
}

fn turn_between(a: V, b: V) -> f64 {
    libm::atan2(cross(a, b), dot(a, b))
}

/// How far `p` stands from the shape (mm).
fn distance_to(shape: &Shape, p: V) -> f64 {
    match *shape {
        Shape::Straight { from, to } => {
            let (a, b) = (v(from), v(to));
            let d = sub(b, a);
            let l2 = dot(d, d);
            let t = if l2 < 1e-24 {
                0.0
            } else {
                (dot(sub(p, a), d) / l2).clamp(0.0, 1.0)
            };
            len(sub(p, add(a, scale(d, t))))
        }
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => {
            let c = v(centre);
            let r = sub(p, c);
            let angle = libm::atan2(r.1, r.0);
            let along = ((angle - from) * sweep.signum()).rem_euclid(std::f64::consts::TAU);
            if along <= sweep.abs() {
                return (len(r) - radius).abs();
            }
            let end = |a: f64| {
                len(sub(
                    p,
                    (c.0 + radius * libm::cos(a), c.1 + radius * libm::sin(a)),
                ))
            };
            end(from).min(end(from + sweep))
        }
    }
}

/// One arc from `start` leaving along `tangent` to `to`; a straight when it is nearly one.
fn arc_from(start: V, tangent: V, to: V) -> Shape {
    let d = sub(to, start);
    let phi = turn_between(tangent, d);
    let sine = libm::sin(phi);
    if sine.abs() < 1e-12 || len(d) / (2.0 * sine.abs()) > STRAIGHT_RADIUS_MM {
        return Shape::Straight {
            from: pt(start),
            to: pt(to),
        };
    }
    let radius = len(d) / (2.0 * sine.abs());
    let left = (-tangent.1, tangent.0);
    let side = phi.signum();
    let c = add(start, scale(left, radius * side));
    Shape::Bend {
        centre: pt(c),
        radius,
        from: libm::atan2(start.1 - c.1, start.0 - c.0),
        sweep: 2.0 * phi,
    }
}

/// The biarc from `a` (direction `ta`) to `b` (direction `tb`): the joint at the incentre of the
/// triangle `a`, `b` and the tangents' intersection. `None` when the tangents do not meet ahead
/// of `a` and behind `b`.
fn biarc(a: V, ta: V, b: V, tb: V) -> Option<[Shape; 2]> {
    let det = cross(ta, tb);
    if det.abs() < 1e-12 {
        return None;
    }
    let ab = sub(b, a);
    // T = a + ta u = b - tb v, with both distances positive.
    let (u, v_back) = (cross(ab, tb) / det, -cross(ab, ta) / det);
    if u <= 0.0 || v_back <= 0.0 {
        return None;
    }
    let t = add(a, scale(ta, u));
    let (la, lb, lt) = (v_back, u, len(ab));
    let joint = scale(
        add(add(scale(a, la), scale(b, lb)), scale(t, lt)),
        1.0 / (la + lb + lt),
    );
    let first = arc_from(a, ta, joint);
    // The second arc is the first one of the reversed walk.
    let second = arc_from(b, scale(tb, -1.0), joint);
    let reversed = match second {
        Shape::Straight { from, to } => Shape::Straight { from: to, to: from },
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => Shape::Bend {
            centre,
            radius,
            from: from + sweep,
            sweep: -sweep,
        },
    };
    Some([first, reversed])
}

fn within(piece: &Bez, shapes: &[Shape]) -> bool {
    (1..SAMPLES).all(|k| {
        #[allow(clippy::cast_precision_loss)]
        let p = piece.at(k as f64 / SAMPLES as f64);
        shapes
            .iter()
            .map(|s| distance_to(s, p))
            .fold(f64::MAX, f64::min)
            <= ACCEPT_MM
    })
}

/// Fits `piece` with biarcs between the directions `ends`, halving until it fits.
fn fit(piece: &Bez, ends: (V, V), depth: u32, out: &mut Vec<Shape>) {
    let (ta, tb) = ends;
    let (a, b) = (piece.0[0], piece.0[3]);
    let turn = turn_between(ta, tb);
    let wide = turn.abs() > FRAC_PI_2 + 1e-9;
    if !wide {
        let candidate: Option<Vec<Shape>> = if turn.abs() < 1e-9 {
            Some(vec![Shape::Straight {
                from: pt(a),
                to: pt(b),
            }])
        } else {
            biarc(a, ta, b, tb).map(|pair| pair.to_vec())
        };
        if let Some(shapes) = candidate
            && within(piece, &shapes)
        {
            out.extend(shapes);
            return;
        }
    }
    if depth >= MAX_HALVINGS {
        // The fixtures never get here; the best biarc stands (or the chord).
        match biarc(a, ta, b, tb) {
            Some(pair) if !wide => out.extend(pair),
            _ => out.push(Shape::Straight {
                from: pt(a),
                to: pt(b),
            }),
        }
        return;
    }
    let (first, second) = piece.split(0.5);
    let mid = piece.derivative(0.5);
    let tm = if len(mid) > 1e-9 {
        scale(mid, 1.0 / len(mid))
    } else {
        first.tangent(true)
    };
    fit(&first, (ta, tm), depth + 1, out);
    fit(&second, (tm, tb), depth + 1, out);
}

/// The arcs (and where a radius is above 1e6 mm, straights) for the curved cubic from `from`,
/// leaving along `start_dir` and arriving along `end_dir` when given (a tangent join is made exact
/// by the caller), else along the cubic's own tangents.
pub(crate) fn arcs(from: Point, cubic: &Cubic, (start_dir, end_dir): Directions) -> Vec<Shape> {
    let whole = Bez::of(from, cubic);
    let mut cuts = vec![0.0];
    cuts.extend(flex_parameters(&whole));
    cuts.push(1.0);
    let mut out = Vec::new();
    let last = cuts.len() - 2;
    let mut rest = whole;
    let mut done = 0.0;
    for (index, pair) in cuts.windows(2).enumerate() {
        let piece = if index == last {
            rest
        } else {
            let local = (pair[1] - done) / (1.0 - done);
            let (head, tail) = rest.split(local);
            rest = tail;
            done = pair[1];
            head
        };
        let ta = if index == 0 {
            start_dir.unwrap_or_else(|| piece.tangent(false))
        } else {
            piece.tangent(false)
        };
        let tb = if index == last {
            end_dir.unwrap_or_else(|| piece.tangent(true))
        } else {
            piece.tangent(true)
        };
        fit(&piece, (ta, tb), 0, &mut out);
    }
    out
}

#[cfg(test)]
mod tests;
