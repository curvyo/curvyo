//! Outlines the unit tests of the path box share (the drawings of the spec: coffin, shield, heart,
//! circle, ellipse).

#![allow(clippy::many_single_char_names)]

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

pub(crate) type Triple = ((f64, f64), (f64, f64), (f64, f64));

pub(crate) fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A closed outline from `(c1, c2, to)` triples after `start`.
pub(crate) fn curve(start: Point, cubics: &[Triple]) -> ClosedPath {
    ClosedPath {
        start,
        segments: cubics
            .iter()
            .map(|&(c1, c2, to)| Cubic {
                c1: p(c1.0, c1.1),
                c2: p(c2.0, c2.1),
                to: p(to.0, to.1),
            })
            .collect(),
    }
}

/// A flat top, two straight sides and a bottom of two cubics that meets the sides tangentially.
pub(crate) fn shield() -> ClosedPath {
    curve(
        p(-30.0, -40.0),
        &[
            ((-30.0, -40.0), (30.0, -40.0), (30.0, -40.0)),
            ((30.0, -40.0), (30.0, 10.0), (30.0, 10.0)),
            ((30.0, 25.0), (10.0, 40.0), (0.0, 40.0)),
            ((-10.0, 40.0), (-30.0, 25.0), (-30.0, 10.0)),
            ((-30.0, 10.0), (-30.0, -40.0), (-30.0, -40.0)),
        ],
    )
}

/// A bottom tip and two lobes meeting in a top notch of about 100 degrees: open enough for the
/// exact cover (ADR 0083 §13.9).
pub(crate) fn heart() -> ClosedPath {
    curve(
        p(0.0, 40.0),
        &[
            ((-10.0, 25.0), (-40.0, 10.0), (-40.0, -10.0)),
            ((-40.0, -35.0), (-12.0, -32.0), (0.0, -22.0)),
            ((12.0, -32.0), (40.0, -35.0), (40.0, -10.0)),
            ((40.0, 10.0), (10.0, 25.0), (0.0, 40.0)),
        ],
    )
}

/// An ellipse of half axes `a` and `b` from four cubics.
pub(crate) fn ellipse(a: f64, b: f64) -> ClosedPath {
    let k = 0.552_284_75;
    curve(
        p(a, 0.0),
        &[
            ((a, k * b), (k * a, b), (0.0, b)),
            ((-k * a, b), (-a, k * b), (-a, 0.0)),
            ((-a, -k * b), (-k * a, -b), (0.0, -b)),
            ((k * a, -b), (a, -k * b), (a, 0.0)),
        ],
    )
}

/// A circle of radius `r` from four cubics.
pub(crate) fn circle(r: f64) -> ClosedPath {
    ellipse(r, r)
}

/// A square of side 80 whose sides are cubics bulging outwards by `bulge` degrees at each end, so
/// every vertex turns by `90 - 2 bulge` degrees.
pub(crate) fn bulging_square(bulge: f64) -> ClosedPath {
    let (cos, sin) = (
        libm::cos(bulge.to_radians()) * 25.0,
        libm::sin(bulge.to_radians()) * 25.0,
    );
    // Clockwise on screen from the top left: right, down, left, up; outwards is the left of the walk.
    curve(
        p(0.0, 0.0),
        &[
            ((cos, -sin), (80.0 - cos, -sin), (80.0, 0.0)),
            ((80.0 + sin, cos), (80.0 + sin, 80.0 - cos), (80.0, 80.0)),
            ((80.0 - cos, 80.0 + sin), (cos, 80.0 + sin), (0.0, 80.0)),
            ((-sin, 80.0 - cos), (-sin, cos), (0.0, 0.0)),
        ],
    )
}

/// Two convex lobes of radius 20 joined by two concave waist arcs of radius 2 (below the wall
/// thickness of 3 mm), clockwise on screen: a peanut.
pub(crate) fn peanut() -> ClosedPath {
    use crate::outline::{Builder, polar};
    let (lobe, waist, spread) = (20.0, 2.0, 15.0);
    let height = libm::sqrt((lobe + waist) * (lobe + waist) - spread * spread);
    let (left, right) = (p(-spread, 0.0), p(spread, 0.0));
    let (top, bottom) = (p(0.0, -height), p(0.0, height));
    let angle = |from: Point, to: Point| libm::atan2(to.y - from.y, to.x - from.x);
    // The waist circle touches a lobe on the line between their centres.
    let tau = std::f64::consts::TAU;
    let (a_lobe, a_waist) = (angle(left, top), angle(top, left));
    let a_start = a_lobe + 0.0;
    let touch = |centre: Point, a: f64, r: f64| polar(centre, r, a);
    let mut b = Builder::new(touch(left, a_start, lobe));
    // The waist from the left lobe to the right one, concave: the angle falls.
    let a_right_lobe = angle(right, top);
    let a_right_waist = angle(top, right);
    b.arc(top, waist, a_waist, a_right_waist);
    // The right lobe, clockwise on screen: the angle rises, round the right side.
    let a_right_bottom = angle(right, bottom);
    b.arc(right, lobe, a_right_lobe, a_right_bottom + tau * 0.0);
    // The bottom waist back to the left.
    let a_bottom_right = angle(bottom, right);
    let a_bottom_left = angle(bottom, left);
    b.arc(bottom, waist, a_bottom_right, a_bottom_left);
    // The left lobe round the left side back to the start.
    let a_left_bottom = angle(left, bottom);
    b.arc(left, lobe, a_left_bottom, a_lobe + tau);
    b.snap_end();
    b.finish()
}

/// A rectangle `w` x `h` with corner arcs of radius `r`, clockwise on screen, centred on 0.
pub(crate) fn rounded_rect(w: f64, h: f64, r: f64) -> ClosedPath {
    use crate::outline::Builder;
    let (hw, hh) = (w / 2.0, h / 2.0);
    let q = std::f64::consts::FRAC_PI_2;
    let mut b = Builder::new(p(-hw + r, -hh));
    b.line_to(p(hw - r, -hh));
    b.arc(p(hw - r, -hh + r), r, -q, 0.0);
    b.line_to(p(hw, hh - r));
    b.arc(p(hw - r, hh - r), r, 0.0, q);
    b.line_to(p(-hw + r, hh));
    b.arc(p(-hw + r, hh - r), r, q, 2.0 * q);
    b.line_to(p(-hw, -hh + r));
    b.arc(p(-hw + r, -hh + r), r, 2.0 * q, 3.0 * q);
    b.snap_end();
    b.finish()
}
