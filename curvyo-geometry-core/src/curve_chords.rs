//! The geometry under the curve intersections: a cubic and its flatness, and what two straight
//! chords do to each other (`specs/0036-cut-at-crossings`). Only `+ - * /` and `sqrt` are used, so
//! the same input gives a bit-identical result on every target.

use curvyo_document_core::Point;

/// Cut points closer than this along a contour are one (millimetres, the kernel tolerance of
/// `0016` criterion 25). A shared stretch shorter than this is a touch.
pub(crate) const CUT_MERGE_MM: f64 = 0.01;

#[derive(Clone, Copy)]
pub(crate) struct Cubic(pub(crate) [Point; 4]);

fn mid(a: Point, b: Point) -> Point {
    Point::new(a.x.midpoint(b.x), a.y.midpoint(b.y))
}

pub(crate) fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

pub(crate) fn dist(a: Point, b: Point) -> f64 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    (dx * dx + dy * dy).sqrt()
}

/// The parameter of the point of segment `a b` nearest to `point`, in `[0, 1]`.
pub(crate) fn nearest_on(point: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length_squared = dx * dx + dy * dy;
    if length_squared > 0.0 {
        (((point.x - a.x) * dx + (point.y - a.y) * dy) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl Cubic {
    pub(crate) fn split_half(&self) -> (Self, Self) {
        let [p0, p1, p2, p3] = self.0;
        let (ab, bc, cd) = (mid(p0, p1), mid(p1, p2), mid(p2, p3));
        let (abc, bcd) = (mid(ab, bc), mid(bc, cd));
        let centre = mid(abc, bcd);
        (Self([p0, ab, abc, centre]), Self([centre, bcd, cd, p3]))
    }

    pub(crate) fn eval(&self, t: f64) -> Point {
        let [p0, p1, p2, p3] = self.0;
        let (a, b, c) = (lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t));
        lerp(lerp(a, b, t), lerp(b, c, t), t)
    }

    /// The box of the control points (the curve lies inside it).
    pub(crate) fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut b = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in self.0 {
            b = (b.0.min(p.x), b.1.max(p.x), b.2.min(p.y), b.3.max(p.y));
        }
        b
    }

    /// How far the inner control points are from the chord: the curve is within this of it.
    pub(crate) fn deviation(&self) -> f64 {
        let [p0, p1, p2, p3] = self.0;
        let far = |p: Point| dist(p, lerp(p0, p3, nearest_on(p, p0, p3)));
        far(p1).max(far(p2))
    }
}

/// What two chords do to each other.
pub(crate) enum Chord {
    None,
    /// They meet at the fractions `(u, v)` of the first and the second chord.
    At(f64, f64),
    /// They share a stretch: the fractions `(u, v)` of its two ends, `u` increasing.
    Span((f64, f64), (f64, f64)),
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn dot(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn vector(from: Point, to: Point) -> (f64, f64) {
    (to.x - from.x, to.y - from.y)
}

/// The distance from `point` to the infinite line through `origin` with direction `d` of length
/// `length`.
fn line_distance(point: Point, origin: Point, d: (f64, f64), length: f64) -> f64 {
    cross(d, vector(origin, point)).abs() / length
}

/// How the chords `p0 p1` and `q0 q1` meet, counting chords within `reach` as meeting.
pub(crate) fn chord_hit(p0: Point, p1: Point, q0: Point, q1: Point, reach: f64) -> Chord {
    let (r, s) = (vector(p0, p1), vector(q0, q1));
    let (rr, ss) = (dot(r, r), dot(s, s));
    if rr <= 0.0 || ss <= 0.0 {
        return point_chord(p0, p1, q0, q1, rr <= 0.0, ss <= 0.0, reach);
    }
    let (length_r, length_s) = (rr.sqrt(), ss.sqrt());
    let collinear = line_distance(q0, p0, r, length_r).max(line_distance(q1, p0, r, length_r))
        <= reach
        && line_distance(p0, q0, s, length_s).max(line_distance(p1, q0, s, length_s)) <= reach;
    if collinear {
        return shared_stretch(p0, p1, q0, q1, reach);
    }
    let denominator = cross(r, s);
    if denominator != 0.0 {
        let qp = vector(p0, q0);
        let u = cross(qp, s) / denominator;
        let v = cross(qp, r) / denominator;
        if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
            return Chord::At(u, v);
        }
    }
    let candidates = [
        (0.0, nearest_on(p0, q0, q1)),
        (1.0, nearest_on(p1, q0, q1)),
        (nearest_on(q0, p0, p1), 0.0),
        (nearest_on(q1, p0, p1), 1.0),
    ];
    let mut best = (f64::INFINITY, 0.0, 0.0);
    for (u, v) in candidates {
        let d = dist(lerp(p0, p1, u), lerp(q0, q1, v));
        if d < best.0 {
            best = (d, u, v);
        }
    }
    if best.0 <= reach {
        Chord::At(best.1, best.2)
    } else {
        Chord::None
    }
}

/// The meeting when one chord (or both) is a point.
fn point_chord(
    p0: Point,
    p1: Point,
    q0: Point,
    q1: Point,
    p_is_point: bool,
    q_is_point: bool,
    reach: f64,
) -> Chord {
    if p_is_point && q_is_point {
        return if dist(p0, q0) <= reach {
            Chord::At(0.0, 0.0)
        } else {
            Chord::None
        };
    }
    if p_is_point {
        let v = nearest_on(p0, q0, q1);
        return if dist(p0, lerp(q0, q1, v)) <= reach {
            Chord::At(0.0, v)
        } else {
            Chord::None
        };
    }
    let u = nearest_on(q0, p0, p1);
    if dist(q0, lerp(p0, p1, u)) <= reach {
        Chord::At(u, 0.0)
    } else {
        Chord::None
    }
}

/// Two chords on one line (within `reach`): the stretch they share, or the point where they
/// touch end to end.
fn shared_stretch(p0: Point, p1: Point, q0: Point, q1: Point, reach: f64) -> Chord {
    let r = vector(p0, p1);
    let rr = dot(r, r);
    let length = rr.sqrt();
    let (uq0, uq1) = (dot(vector(p0, q0), r) / rr, dot(vector(p0, q1), r) / rr);
    let low = uq0.min(uq1).max(0.0);
    let high = uq0.max(uq1).min(1.0);
    if low > high + reach / length {
        return Chord::None;
    }
    let v_of = |u: f64| nearest_on(lerp(p0, p1, u), q0, q1);
    if low > high || (high - low) * length <= CUT_MERGE_MM {
        let u = low.midpoint(high).clamp(0.0, 1.0);
        return Chord::At(u, v_of(u));
    }
    Chord::Span((low, v_of(low)), (high, v_of(high)))
}
