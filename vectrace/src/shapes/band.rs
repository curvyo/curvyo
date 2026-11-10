//! The centre-line primitive of a traced band of constant width (`specs/0088-trace-recognition`
//! Part B criterion 16, `adrs.md` decision 21): an outer and an inner primitive of one kind that
//! are parallel offsets of one primitive become that primitive with a stroke of the band's width.

// The formulas' own names (a, b, d, h, p, r, ...) read better than long ones here.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::f64::consts::PI;

use crate::{Angle, Mm};
use kurbo::Point;

use crate::shapes::fit::Fitted;
use crate::shapes::join::MITER_LIMIT;
use crate::shapes::math::sin_cos;
use crate::shapes::sample::length;
use crate::shapes::{Primitive, valid_ratio};

/// The centre-line primitive and the stroke width of the band between `outer` and `inner`, when
/// they are parallel offsets of it within `tolerance`; `None` for any other pair. `traced` is the
/// traced band's area (a star's width is the one that covers it).
pub(crate) fn band_of(
    outer: &Fitted,
    inner: &Fitted,
    traced: f64,
    tolerance: f64,
) -> Option<(Fitted, f64)> {
    let ((o, o_turn), (i, i_turn)) = (outer, inner);
    let (mid, width) = match (*o, *i) {
        (
            Primitive::Ellipse {
                centre: c1,
                rx: a1,
                ry: b1,
            },
            Primitive::Ellipse {
                centre: c2,
                rx: a2,
                ry: b2,
            },
        ) => ellipse_band(
            o,
            (c1, a1.as_mm(), b1.as_mm()),
            (c2, a2.as_mm(), b2.as_mm()),
            (*o_turn, *i_turn),
            tolerance,
        )?,
        (
            Primitive::Rect {
                origin: f,
                width: w1,
                height: h1,
                corner_radius: fr,
            },
            Primitive::Rect {
                origin: g,
                width: w2,
                height: h2,
                corner_radius: gr,
            },
        ) => rect_band(
            (f, w1, h1),
            fr.as_mm(),
            (g, w2, h2),
            gr.as_mm(),
            (*o_turn, *i_turn),
            tolerance,
        )?,
        (
            Primitive::Polygon {
                centre: c1,
                radius: r1,
                angle: t1,
                points: n,
            },
            Primitive::Polygon {
                centre: c2,
                radius: r2,
                angle: t2,
                points: m,
            },
        ) if n == m => polygon_band(o, (c1, r1, t1), (c2, r2, t2), n, tolerance)?,
        (
            Primitive::Star {
                centre: c1,
                radius: r1,
                angle: t1,
                points: n,
                inner_ratio: q1,
            },
            Primitive::Star {
                centre: c2,
                radius: r2,
                angle: t2,
                points: m,
                inner_ratio: q2,
            },
        ) if n == m => star_band(
            o,
            ((c1, r1, t1), q1),
            ((c2, r2, t2), q2),
            n,
            traced,
            tolerance,
        )?,
        _ => return None,
    };
    (width > 0.0).then_some((mid, width))
}

/// Whether the turns `a` and `b` (radians, of a figure repeating every `period`) agree within
/// `tolerance` at `reach` from the centre.
fn same_turn(a: f64, b: f64, period: f64, reach: f64, tolerance: f64) -> bool {
    let d = (a - b).rem_euclid(period);
    d.min(period - d) * reach <= tolerance
}

/// Whether two centres agree within `tolerance`.
fn near(p: Point, q: Point, tolerance: f64) -> bool {
    length(p.x - q.x, p.y - q.y) <= tolerance
}

/// The band of two ellipses: one centre and turn (none for circles), both radii shrunk alike.
fn ellipse_band(
    outer: &Primitive,
    (c1, a1, b1): (Point, f64, f64),
    (c2, a2, b2): (Point, f64, f64),
    turns: (Angle, Angle),
    tolerance: f64,
) -> Option<(Fitted, f64)> {
    let round = (a1 - b1).abs() <= tolerance && (a2 - b2).abs() <= tolerance;
    let turned = round
        || same_turn(
            turns.0.as_radians(),
            turns.1.as_radians(),
            PI,
            a1,
            tolerance,
        );
    let (da, db) = (a1 - a2, b1 - b2);
    if !near(c1, c2, tolerance) || !turned || (da - db).abs() > 2.0 * tolerance {
        return None;
    }
    let mut shape = *outer;
    if let Primitive::Ellipse { rx, ry, .. } = &mut shape {
        *rx = Mm::from_mm(f64::midpoint(a1, a2));
        *ry = Mm::from_mm(f64::midpoint(b1, b2));
    }
    Some(((shape, turns.0), f64::midpoint(da, db)))
}

/// The band of two regular polygons of `n` sides: one centre and turn; the centre line's apothem
/// halfway between theirs (a mitred stroke keeps the corners' angle).
fn polygon_band(
    outer: &Primitive,
    (c1, r1, t1): (Point, Mm, Angle),
    (c2, r2, t2): (Point, Mm, Angle),
    n: u32,
    tolerance: f64,
) -> Option<(Fitted, f64)> {
    let h = PI / f64::from(n);
    let (_, c) = sin_cos(h);
    let (p1, p2) = (r1.as_mm() * c, r2.as_mm() * c);
    let turned = same_turn(
        t1.as_radians(),
        t2.as_radians(),
        2.0 * h,
        r1.as_mm(),
        tolerance,
    );
    if !near(c1, c2, tolerance) || !turned {
        return None;
    }
    let mut shape = *outer;
    if let Primitive::Polygon { radius, .. } = &mut shape {
        *radius = Mm::from_mm(f64::midpoint(p1, p2) / c);
    }
    Some(((shape, Angle::from_radians(0.0)), p1 - p2))
}

/// The band of two rectangles: equal borders, the same turn; the centre line's corner radius is
/// the outer's less half the width (a square outer corner is a mitred square centre line), and
/// the inner's must be the centre line's less half the width, or square.
fn rect_band(
    f: (Point, Mm, Mm),
    r1: f64,
    g: (Point, Mm, Mm),
    r2: f64,
    turns: (Angle, Angle),
    tolerance: f64,
) -> Option<(Fitted, f64)> {
    let (w1, h1, w2, h2) = (f.1.as_mm(), f.2.as_mm(), g.1.as_mm(), g.2.as_mm());
    let centre = |(origin, width, height): (Point, Mm, Mm)| {
        Point::new(
            origin.x + width.as_mm() / 2.0,
            origin.y + height.as_mm() / 2.0,
        )
    };
    let (c1, c2) = (centre(f), centre(g));
    let d = f64::midpoint(w1 - w2, h1 - h2) / 2.0;
    let quarter = PI / 2.0;
    let off = (turns.0.as_radians() - turns.1.as_radians()).rem_euclid(quarter);
    let turned = off.min(quarter - off) * w1.max(h1) <= tolerance;
    if length(c1.x - c2.x, c1.y - c2.y) > tolerance
        || !turned
        || ((w1 - w2) - (h1 - h2)).abs() > 2.0 * tolerance
        || d <= 0.0
    {
        return None;
    }
    let r = if r1 <= tolerance { 0.0 } else { r1 - d / 2.0 };
    if r < -tolerance || (r2 - (r - d / 2.0).max(0.0)).abs() > tolerance {
        return None;
    }
    let (w, h) = (f64::midpoint(w1, w2), f64::midpoint(h1, h2));
    let shape = Primitive::Rect {
        origin: Point::new(c1.x - w / 2.0, c1.y - h / 2.0),
        width: Mm::from_mm(w),
        height: Mm::from_mm(h),
        corner_radius: Mm::from_mm(r.max(0.0)),
    };
    Some(((shape, turns.0), d))
}

/// One side of a star in its own frame: the unit normal of the side from the tip (angle 0) to
/// the valley (angle π/n) and its distance from the centre.
pub(crate) fn side_line(outer: f64, inner: f64, n: u32) -> Option<([f64; 2], f64)> {
    let (us, uc) = sin_cos(PI / f64::from(n));
    let (ex, ey) = (inner * uc - outer, inner * us);
    let len = length(ex, ey);
    if len == 0.0 {
        return None;
    }
    let normal = [ey / len, -ex / len];
    Some((normal, normal[0] * outer))
}

/// The band of two stars of `n` points (frame and inner ratio each): their sides parallel within
/// the tolerance over a side's length, the centre line's sides halfway between, no corner sharper
/// than the miter limit. The width is the one whose stroke covers the `traced` area: the two
/// fits' distance is that within the tolerance, but on a band a few pixels wide each fit's error
/// is a large part of it, while the area sums the whole band.
fn star_band(
    outer: &Primitive,
    ((c1, r1, t1), q1): ((Point, Mm, Angle), f64),
    ((c2, r2, t2), q2): ((Point, Mm, Angle), f64),
    n: u32,
    traced: f64,
    tolerance: f64,
) -> Option<(Fitted, f64)> {
    let (r1, r2) = (r1.as_mm(), r2.as_mm());
    let period = 2.0 * PI / f64::from(n);
    let turned = same_turn(t1.as_radians(), t2.as_radians(), period, r1, tolerance);
    if !near(c1, c2, tolerance) || !turned {
        return None;
    }
    let (n1, p1) = side_line(r1, r1 * q1, n)?;
    let (n2, p2) = side_line(r2, r2 * q2, n)?;
    let side = r1 * (1.0 - q1).max(q1);
    if (n1[0] * n2[1] - n1[1] * n2[0]).abs() * side > tolerance || p1 <= p2 {
        return None;
    }
    let (nx, ny) = (n1[0] + n2[0], n1[1] + n2[1]);
    let len = length(nx, ny);
    let normal = [nx / len, ny / len];
    let p = f64::midpoint(p1, p2);
    let (us, uc) = sin_cos(PI / f64::from(n));
    let tip = p / normal[0];
    let valley = p / (normal[0] * uc + normal[1] * us);
    // The side's direction from the tip; the tip's half angle against the x axis, the valley's
    // against the middle line.
    let (ex, ey) = (valley * uc - tip, valley * us);
    let e = length(ex, ey);
    let tip_sine = ey.abs() / e;
    let (vx, vy) = (tip - valley * uc, -valley * us);
    let valley_cosine = (vx * uc + vy * us) / e;
    let valley_sine = (1.0 - valley_cosine * valley_cosine).max(0.0).sqrt();
    if tip_sine * MITER_LIMIT < 1.0 || valley_sine * MITER_LIMIT < 1.0 {
        return None;
    }
    // The stroke of width w covers the offsets' areas k(p ± w/2)² apart: 2kp·w.
    let k = f64::from(n) * us / (normal[0] * (normal[0] * uc + normal[1] * us));
    let width = traced / (2.0 * k * p);
    if (width - (p1 - p2)).abs() > tolerance {
        return None;
    }
    let mut shape = *outer;
    if let Primitive::Star {
        radius,
        inner_ratio,
        ..
    } = &mut shape
    {
        *radius = Mm::from_mm(tip);
        *inner_ratio = valid_ratio(valley / tip)?;
    }
    Some(((shape, Angle::from_radians(0.0)), width))
}

/// The area a stroke of `width` along the closed outline of `shape` covers (mitred corners):
/// the outer offset's area less the inner offset's.
pub(crate) fn stroked_area(shape: &Primitive, width: f64) -> f64 {
    let t = width / 2.0;
    match *shape {
        Primitive::Ellipse { rx, ry, .. } => {
            // Steiner: a convex curve's offsets differ in area by its length times the width.
            let (a, b) = (rx.as_mm(), ry.as_mm());
            let perimeter = PI * (3.0 * (a + b) - ((3.0 * a + b) * (a + 3.0 * b)).sqrt());
            perimeter * width
        }
        Primitive::Rect {
            width: rect_w,
            height,
            corner_radius,
            ..
        } => {
            let (w, h, r) = (rect_w.as_mm(), height.as_mm(), corner_radius.as_mm());
            let area = |w: f64, h: f64, r: f64| w * h - (4.0 - PI) * r * r;
            let outer_r = if r > 0.0 { r + t } else { 0.0 };
            area(w + width, h + width, outer_r) - area(w - width, h - width, (r - t).max(0.0))
        }
        Primitive::Polygon { radius, points, .. } => {
            let n = f64::from(points);
            let (s, c) = sin_cos(PI / n);
            let apothem = radius.as_mm() * c;
            let area = |a: f64| n * a * a * s / c;
            area(apothem + t) - area(apothem - t)
        }
        Primitive::Star {
            radius,
            points,
            inner_ratio,
            ..
        } => {
            let n = points;
            let r = radius.as_mm();
            let Some((normal, p)) = side_line(r, r * inner_ratio, n) else {
                return 0.0;
            };
            let (us, uc) = sin_cos(PI / f64::from(n));
            let area = |p: f64| {
                let tip = p / normal[0];
                let valley = p / (normal[0] * uc + normal[1] * us);
                f64::from(n) * tip * valley * us
            };
            area(p + t) - area(p - t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(r: f64, q: f64) -> Fitted {
        (
            Primitive::Star {
                centre: Point::new(0.0, 0.0),
                radius: Mm::from_mm(r),
                angle: Angle::from_radians(0.3),
                points: 8,
                inner_ratio: q,
            },
            Angle::from_radians(0.0),
        )
    }

    /// The stroke's two offsets of a star are stars; the band of the two is the star again.
    #[test]
    fn a_stroked_star_is_found_from_its_two_offsets() {
        let (mid, _) = star(40.0, 0.5);
        let (normal, p) = side_line(40.0, 20.0, 8).unwrap();
        let (us, uc) = sin_cos(PI / 8.0);
        let offset = |p: f64| {
            let tip = p / normal[0];
            star(tip, p / (normal[0] * uc + normal[1] * us) / tip)
        };
        let area = |s: &Fitted| match s.0 {
            Primitive::Star {
                radius,
                inner_ratio,
                ..
            } => 8.0 * radius.as_mm().powi(2) * inner_ratio * us,
            _ => 0.0,
        };
        let between = area(&offset(p + 1.0)) - area(&offset(p - 1.0));
        let ((found, _), width) =
            band_of(&offset(p + 1.0), &offset(p - 1.0), between, 0.05).unwrap();
        assert!((width - 2.0).abs() < 1e-9);
        let (
            Primitive::Star {
                radius,
                angle,
                inner_ratio,
                ..
            },
            Primitive::Star { .. },
        ) = (found, mid)
        else {
            panic!("{found:?}");
        };
        assert!((radius.as_mm() - 40.0).abs() < 1e-9);
        assert!((inner_ratio - 0.5).abs() < 1e-9);
        assert!((angle.as_radians() - 0.3).abs() < 1e-12);
        assert!((stroked_area(&found, 2.0) - between).abs() < 1e-9);
        // An area whose width is not the offsets' distance within the tolerance is no band.
        assert_eq!(
            band_of(&offset(p + 1.0), &offset(p - 1.0), 1.1 * between, 0.05),
            None
        );
    }

    /// Two stars of different tip angles are no band.
    #[test]
    fn stars_of_other_angles_are_no_band() {
        assert_eq!(
            band_of(&star(40.0, 0.5), &star(30.0, 0.3), 500.0, 0.05),
            None
        );
    }
}
