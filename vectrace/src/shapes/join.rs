//! The offset outlines of a stroke along a star or regular polygon, mitred or bevelled at its
//! joins (`specs/0088-trace-recognition` Part B criterion 16, `adrs.md` decision 22).

// The formulas' own names (a, b, d, n, p, s, t, ...) read better than long ones here.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::f64::consts::{PI, TAU};

use crate::Mm;

use crate::shapes::band::side_line;
use crate::shapes::math::{atan2, sin_cos};
use crate::shapes::sample::length;
use crate::shapes::{Join, Primitive, valid_ratio};

/// The pieces of a round join's arc.
const ARC_STEPS: u32 = 8;

/// The longest miter the renderer draws, in stroke widths (SVG's 4, `curvyo-render-core`).
pub(crate) const MITER_LIMIT: f64 = 4.0;

/// The corners of a star or regular polygon in document millimetres, turning counter-clockwise
/// (positive area); `None` for any other kind.
pub(crate) fn corners(shape: &Primitive) -> Option<Vec<[f64; 2]>> {
    let (centre, radius, angle, n, ratio) = match *shape {
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => (centre, radius, angle, points, Some(inner_ratio)),
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        } => (centre, radius, angle, points, None),
        _ => return None,
    };
    let m = if ratio.is_some() { 2 * n } else { n };
    let r = radius.as_mm();
    let step = TAU / f64::from(m);
    Some(
        (0..m)
            .map(|j| {
                let radius = match ratio {
                    Some(q) if j % 2 == 1 => r * q,
                    _ => r,
                };
                let (s, c) = sin_cos(angle.as_radians() + step * f64::from(j));
                [centre.x + radius * c, centre.y + radius * s]
            })
            .collect(),
    )
}

/// The star or polygon whose sides are those of `shape` moved outwards by `t` (inwards for a
/// negative `t`): the same corner angles, so the centre line of a stroke whose inner outline is
/// `shape` and whose half width is `t`.
pub(crate) fn widened(shape: &Primitive, t: f64) -> Option<Primitive> {
    let mut out = *shape;
    match &mut out {
        Primitive::Star {
            radius,
            points,
            inner_ratio,
            ..
        } => {
            let n = *points;
            let r = radius.as_mm();
            let (normal, p) = side_line(r, r * *inner_ratio, n)?;
            let (us, uc) = sin_cos(PI / f64::from(n));
            let p = p + t;
            let tip = p / normal[0];
            let valley = p / (normal[0] * uc + normal[1] * us);
            if !(tip > 0.0 && valley > 0.0) {
                return None;
            }
            *radius = Mm::from_mm(tip);
            *inner_ratio = valid_ratio(valley / tip)?;
        }
        Primitive::Polygon { radius, points, .. } => {
            let (_, c) = sin_cos(PI / f64::from(*points));
            let apothem = radius.as_mm() * c + t;
            if apothem <= 0.0 {
                return None;
            }
            *radius = Mm::from_mm(apothem / c);
        }
        _ => return None,
    }
    Some(out)
}

/// The unit normal of side `i` (from corner `i` to the next), pointing out of a
/// counter-clockwise outline.
fn normal(corners: &[[f64; 2]], i: usize) -> [f64; 2] {
    let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = length(dx, dy).max(f64::MIN_POSITIVE);
    [dy / len, -dx / len]
}

/// `corners` with every side moved out by `s` (in for a negative `s`); a corner the stroke turns
/// round on this side gets `join` (a bevel is two points, a round join an arc), the others the
/// sides' meeting point.
pub(crate) fn offset(corners: &[[f64; 2]], s: f64, join: Join) -> Vec<[f64; 2]> {
    let m = corners.len();
    let mut out = Vec::with_capacity(2 * m);
    for i in 0..m {
        let (na, nb) = (normal(corners, (i + m - 1) % m), normal(corners, i));
        let v = corners[i];
        let convex = na[0] * nb[1] - na[1] * nb[0] > 0.0;
        if join == Join::Bevel && convex == (s > 0.0) {
            out.push([v[0] + s * na[0], v[1] + s * na[1]]);
            out.push([v[0] + s * nb[0], v[1] + s * nb[1]]);
        } else if join == Join::Round && convex == (s > 0.0) {
            // The arc about the corner from one side's end to the next one's.
            let turn = atan2(na[0] * nb[1] - na[1] * nb[0], na[0] * nb[0] + na[1] * nb[1]);
            for k in 0..=ARC_STEPS {
                let (sn, cs) = sin_cos(turn * f64::from(k) / f64::from(ARC_STEPS));
                let n = [na[0] * cs - na[1] * sn, na[0] * sn + na[1] * cs];
                out.push([v[0] + s * n[0], v[1] + s * n[1]]);
            }
        } else {
            let k = s / (1.0 + na[0] * nb[0] + na[1] * nb[1]).max(f64::MIN_POSITIVE);
            out.push([v[0] + k * (na[0] + nb[0]), v[1] + k * (na[1] + nb[1])]);
        }
    }
    out
}

/// Whether no corner of `corners` is sharper than the renderer's miter limit.
pub(crate) fn within_miter_limit(corners: &[[f64; 2]]) -> bool {
    let m = corners.len();
    (0..m).all(|i| {
        let (na, nb) = (normal(corners, (i + m - 1) % m), normal(corners, i));
        let cos = na[0] * nb[0] + na[1] * nb[1];
        // The miter is 1 / cos(φ/2) stroke widths long, φ the turn between the sides.
        2.0 <= MITER_LIMIT * MITER_LIMIT * (1.0 + cos)
    })
}

/// The area of the closed polygon `points`.
pub(crate) fn area(points: &[[f64; 2]]) -> f64 {
    let o = points[0];
    let mut sum = 0.0;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        sum += (a[0] - o[0]) * (b[1] - o[1]) - (b[0] - o[0]) * (a[1] - o[1]);
    }
    (sum / 2.0).abs()
}

/// The area a stroke of `width` along `corners` covers with `join`.
pub(crate) fn stroke_area(corners: &[[f64; 2]], width: f64, join: Join) -> f64 {
    let t = width / 2.0;
    area(&offset(corners, t, join)) - area(&offset(corners, -t, join))
}

/// The distance from `p` to the outline through `points`.
pub(crate) fn distance(p: [f64; 2], points: &[[f64; 2]]) -> f64 {
    let mut best = f64::INFINITY;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let u = if len2 == 0.0 {
            0.0
        } else {
            (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
        };
        best = best.min(length(p[0] - a[0] - u * dx, p[1] - a[1] - u * dy));
    }
    best
}

/// Whether `p` lies inside the outline through `points` (even-odd).
pub(crate) fn inside(p: [f64; 2], points: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Angle;
    use kurbo::Point;

    fn square() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
    }

    /// A mitred square stroke covers (10 + w)² − (10 − w)²; a bevelled one less the four corner
    /// triangles of (w/2)² / 2 each.
    #[test]
    fn a_square_stroke_covers_its_offsets() {
        let mitred = stroke_area(&square(), 2.0, Join::Miter);
        assert!((mitred - (144.0 - 64.0)).abs() < 1e-9);
        let bevelled = stroke_area(&square(), 2.0, Join::Bevel);
        assert!((bevelled - (80.0 - 2.0)).abs() < 1e-9);
        // A round corner of radius 1 instead of the square one: π − 4 less each, to the arc's
        // pieces.
        let round = stroke_area(&square(), 2.0, Join::Round);
        assert!(
            (round - (80.0 - (4.0 - std::f64::consts::PI))).abs() < 0.03,
            "{round}"
        );
        assert!((distance([5.0, -3.0], &square()) - 3.0).abs() < 1e-12);
        assert!(inside([5.0, 5.0], &square()) && !inside([5.0, -1.0], &square()));
    }

    /// Widening a star moves every side out by t: the corners of its mitred inward offset are the
    /// star's own.
    #[test]
    fn a_widened_star_has_the_star_as_its_mitred_inner_offset() {
        let star = Primitive::Star {
            centre: Point::new(5.0, 7.0),
            radius: Mm::from_mm(30.0),
            angle: Angle::from_radians(0.2),
            points: 8,
            inner_ratio: 0.55,
        };
        let wide = corners(&widened(&star, 1.5).unwrap()).unwrap();
        let back = offset(&wide, -1.5, Join::Miter);
        for (a, b) in back.iter().zip(corners(&star).unwrap()) {
            assert!(length(a[0] - b[0], a[1] - b[1]) < 1e-9, "{a:?} {b:?}");
        }
        assert!(within_miter_limit(&wide));
    }
}
