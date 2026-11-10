//! The simplest primitive a sampled traced outline is within a tolerance of
//! (`specs/0088-trace-recognition` Part B criteria 1 to 5 and 7): circle, ellipse, rectangle,
//! rounded rectangle, then the regular polygon or star of `trace_shape_star`; the first that
//! fits wins.

// The formulas' own names (x, y, u, v, a, b, r, ...) read better than long ones here.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use crate::{Angle, Mm};
use kurbo::Point;

use crate::shapes::Primitive;
use crate::shapes::math::atan2;
use crate::shapes::sample::{Sampled, length};

/// How far from an axis a piece of outline may turn and still count as a rectangle's side
/// (cosine of 10 degrees).
const SIDE_COSINE: f64 = 0.984_807_753_012_208;

/// A primitive with its rotation about its frame's centre, in document millimetres.
pub(crate) type Fitted = (Primitive, Angle);

/// The simplest primitive every point of `outline` is within `tolerance` of, if any.
pub(crate) fn fit_outline(outline: &Sampled, tolerance: f64) -> Option<Fitted> {
    circle(outline, tolerance)
        .or_else(|| ellipse(outline, tolerance))
        .or_else(|| rectangle(outline, tolerance))
        .or_else(|| crate::shapes::star::polygon_or_star(outline, tolerance))
}

/// Whether every point of `outline` is within `tolerance` of the curve `distance` measures
/// for point `i` (signed, about the centroid), and the curve's `area` is the outline's within the tolerance
/// band along its length (so a sliver along the curve does not count).
pub(crate) fn within(
    outline: &Sampled,
    tolerance: f64,
    area: f64,
    mut distance: impl FnMut(usize, [f64; 2]) -> f64,
) -> bool {
    (area - outline.area()).abs() <= tolerance * outline.perimeter
        && outline
            .points
            .iter()
            .enumerate()
            .all(|(i, &p)| distance(i, p).abs() <= tolerance)
}

/// A document point from a point about the outline's centroid.
pub(crate) fn at(outline: &Sampled, p: [f64; 2]) -> Point {
    Point::new(outline.centre[0] + p[0], outline.centre[1] + p[1])
}

fn ellipse_shape(centre: Point, rx: f64, ry: f64) -> Primitive {
    Primitive::Ellipse {
        centre,
        rx: Mm::from_mm(rx),
        ry: Mm::from_mm(ry),
    }
}

/// The circle about the centroid with the mean distance of the outline (by length).
fn circle(outline: &Sampled, tolerance: f64) -> Option<Fitted> {
    let (mut sum, mut weight) = (0.0, 0.0);
    for i in 0..outline.points.len() {
        let (a, b) = outline.segment(i);
        let w = length(b[0] - a[0], b[1] - a[1]);
        sum += w * (length(a[0], a[1]) + length(b[0], b[1])) / 2.0;
        weight += w;
    }
    let r = sum / weight;
    let area = std::f64::consts::PI * r * r;
    within(outline, tolerance, area, |_, p| length(p[0], p[1]) - r).then(|| {
        (
            ellipse_shape(at(outline, [0.0, 0.0]), r, r),
            Angle::from_radians(0.0),
        )
    })
}

/// The cosine and sine of half the angle whose cosine and sine are `c` and `s` (the half angle
/// in (−π/2, π/2]).
pub(crate) fn half_angle(c: f64, s: f64) -> (f64, f64) {
    let ch = f64::midpoint(1.0, c).max(0.0).sqrt();
    let sh = ((1.0 - c) / 2.0).max(0.0).sqrt();
    (ch, if s < 0.0 { -sh } else { sh })
}

/// The ellipse of the outline's second moments (a solid ellipse of semi-axes a and b has
/// a²/4 and b²/4), turned along its major axis.
fn ellipse(outline: &Sampled, tolerance: f64) -> Option<Fitted> {
    let (sxx, syy, sxy) = (outline.sxx, outline.syy, outline.sxy);
    let d = length(sxx - syy, 2.0 * sxy);
    let (l1, l2) = ((sxx + syy + d) / 2.0, (sxx + syy - d) / 2.0);
    if d == 0.0 || l2 <= 0.0 {
        return None;
    }
    let (a, b) = (2.0 * l1.sqrt(), 2.0 * l2.sqrt());
    let (c, s) = half_angle((sxx - syy) / d, 2.0 * sxy / d);
    let distance = |_, p: [f64; 2]| {
        let (u, v) = (p[0] * c + p[1] * s, -p[0] * s + p[1] * c);
        let f = u * u / (a * a) + v * v / (b * b) - 1.0;
        let g = 2.0 * length(u / (a * a), v / (b * b));
        if g == 0.0 { a } else { f / g }
    };
    let area = std::f64::consts::PI * a * b;
    within(outline, tolerance, area, distance).then(|| {
        (
            ellipse_shape(at(outline, [0.0, 0.0]), a, b),
            Angle::from_radians(atan2(s, c)),
        )
    })
}

/// The rectangle along the outline's fourfold direction, its sides the mean of the outline's
/// straight pieces, square-cornered if that fits, else with the corner radius of its area.
fn rectangle(outline: &Sampled, tolerance: f64) -> Option<Fitted> {
    let (c, s) = fourfold_direction(outline)?;
    let local = |p: [f64; 2]| [p[0] * c + p[1] * s, -p[0] * s + p[1] * c];
    let mut box_ = sides(outline, &local, None)?;
    // Once more with the straight parts only, past the corner arcs the first estimate implies.
    let r = corner_radius(box_, outline.area());
    if r > 0.0 {
        box_ = sides(outline, &local, Some((box_, r)))?;
    }
    let [u0, v0, w, h] = box_;
    let centre = [u0 * c - v0 * s, u0 * s + v0 * c];
    let box_distance = |r: f64| {
        move |_, p: [f64; 2]| {
            let q = local(p);
            let (qx, qy) = (
                (q[0] - u0).abs() - (w / 2.0 - r),
                (q[1] - v0).abs() - (h / 2.0 - r),
            );
            length(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
        }
    };
    let shape = |r: f64| {
        let centre = at(outline, centre);
        (
            Primitive::Rect {
                origin: Point::new(centre.x - w / 2.0, centre.y - h / 2.0),
                width: Mm::from_mm(w),
                height: Mm::from_mm(h),
                corner_radius: Mm::from_mm(r),
            },
            Angle::from_radians(atan2(s, c)),
        )
    };
    if within(outline, tolerance, w * h, box_distance(0.0)) {
        return Some(shape(0.0));
    }
    let r = corner_radius(box_, outline.area());
    let area = w * h - (4.0 - std::f64::consts::PI) * r * r;
    (r > 0.0 && within(outline, tolerance, area, box_distance(r))).then(|| shape(r))
}

/// The corner radius of the rounded rectangle `box_` (centre u, v, width, height) with `area`:
/// a rounded rectangle has w·h − (4 − π)·r².
fn corner_radius(box_: [f64; 4], area: f64) -> f64 {
    let [_, _, w, h] = box_;
    let cut = (w * h - area) / (4.0 - std::f64::consts::PI);
    cut.max(0.0).sqrt().min(w.min(h) / 2.0)
}

/// The box (centre u, v, width, height, in the turned frame `local`) whose sides are the
/// length-weighted means of the outline's pieces along them; with `straight` (an earlier box and
/// its corner radius) only the pieces between its corner arcs count.
fn sides(
    outline: &Sampled,
    local: &impl Fn([f64; 2]) -> [f64; 2],
    straight: Option<([f64; 4], f64)>,
) -> Option<[f64; 4]> {
    // Right, left, bottom, top: sum of position by length and the length.
    let mut sums = [[0.0_f64; 2]; 4];
    for i in 0..outline.points.len() {
        let (a, b) = outline.segment(i);
        let (a, b) = (local(a), local(b));
        let (du, dv) = (b[0] - a[0], b[1] - a[1]);
        let len = length(du, dv);
        if len == 0.0 {
            continue;
        }
        let (mu, mv) = (f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1]));
        let inside = |along: f64, centre: f64, half: f64| {
            straight.is_none_or(|(_, r)| (along - centre).abs() <= half - r)
        };
        let [cu, cv, w, h] = straight.map_or([0.0; 4], |(b, _)| b);
        let side = if dv.abs() >= SIDE_COSINE * len && inside(mv, cv, h / 2.0) {
            if mu > cu { (0, mu) } else { (1, mu) }
        } else if du.abs() >= SIDE_COSINE * len && inside(mu, cu, w / 2.0) {
            if mv > cv { (2, mv) } else { (3, mv) }
        } else {
            continue;
        };
        sums[side.0][0] += len * side.1;
        sums[side.0][1] += len;
    }
    if sums.iter().any(|side| side[1] == 0.0) {
        return None;
    }
    let [right, left, high, low] = sums.map(|side| side[0] / side[1]);
    let (w, h) = (right - left, high - low);
    (w > 0.0 && h > 0.0).then_some([f64::midpoint(right, left), f64::midpoint(high, low), w, h])
}

/// The cosine and sine of the outline's sides' direction in (−45°, 45°]: the length-weighted
/// mean of four times each piece's direction, quartered (a rectangle's four sides and its corner
/// arcs agree on it, a square's too).
fn fourfold_direction(outline: &Sampled) -> Option<(f64, f64)> {
    let (mut c4, mut s4) = (0.0, 0.0);
    for i in 0..outline.points.len() {
        let (a, b) = outline.segment(i);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = length(dx, dy);
        if len == 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (c2, s2) = (ux * ux - uy * uy, 2.0 * ux * uy);
        c4 += len * (c2 * c2 - s2 * s2);
        s4 += len * 2.0 * c2 * s2;
    }
    let m = length(c4, s4);
    // A rectangle's sides make more than half its length; a circle's directions cancel.
    if m < 0.25 * outline.perimeter {
        return None;
    }
    let (c2, s2) = half_angle(c4 / m, s4 / m);
    Some(half_angle(c2, s2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::sample::sample_closed;
    use kurbo::{BezPath, Shape as _};

    fn sampled(path: &BezPath) -> Sampled {
        sample_closed(path, 0.1).unwrap()
    }

    fn polygon(points: &[(f64, f64)]) -> BezPath {
        let mut p = BezPath::new();
        p.move_to(points[0]);
        for q in &points[1..] {
            p.line_to(*q);
        }
        p.close_path();
        p
    }

    #[test]
    fn a_circle_is_a_circle_not_an_ellipse() {
        let path = kurbo::Circle::new((40.0, 30.0), 25.0).to_path(1e-4);
        let (shape, _) = fit_outline(&sampled(&path), 0.2).unwrap();
        let Primitive::Ellipse { centre, rx, ry } = shape else {
            panic!("{shape:?}");
        };
        assert!((rx.as_mm() - 25.0).abs() < 1e-3);
        assert_eq!(rx, ry);
        assert!((centre.x - 40.0).abs() < 1e-6 && (centre.y - 30.0).abs() < 1e-6);
    }

    #[test]
    fn a_turned_ellipse_has_its_radii_and_turn() {
        let mut path = kurbo::Ellipse::new((0.0, 0.0), (30.0, 20.0), 0.4).to_path(1e-4);
        path.close_path();
        let (shape, turn) = fit_outline(&sampled(&path), 0.2).unwrap();
        let Primitive::Ellipse { rx, ry, .. } = shape else {
            panic!("{shape:?}");
        };
        assert!((rx.as_mm() - 30.0).abs() < 1e-2, "{rx:?}");
        assert!((ry.as_mm() - 20.0).abs() < 1e-2, "{ry:?}");
        assert!((turn.as_radians() - 0.4).abs() < 1e-4);
    }

    #[test]
    fn a_square_cornered_rectangle_is_not_a_rounded_one() {
        let path = polygon(&[(0.0, 0.0), (200.0, 0.0), (200.0, 100.0), (0.0, 100.0)]);
        let (shape, turn) = fit_outline(&sampled(&path), 0.2).unwrap();
        let Primitive::Rect {
            width,
            height,
            corner_radius,
            ..
        } = shape
        else {
            panic!("{shape:?}");
        };
        assert!((width.as_mm() - 200.0).abs() < 1e-6);
        assert!((height.as_mm() - 100.0).abs() < 1e-6);
        assert_eq!(corner_radius, Mm::from_mm(0.0));
        assert!(turn.as_radians().abs() < 1e-9);
    }

    #[test]
    fn a_rounded_rectangle_has_its_corner_radius() {
        let rect = kurbo::RoundedRect::new(0.0, 0.0, 200.0, 100.0, 10.0);
        let path = kurbo::Affine::rotate(0.3) * rect.to_path(1e-4);
        let (shape, turn) = fit_outline(&sampled(&path), 0.2).unwrap();
        let Primitive::Rect {
            width,
            height,
            corner_radius,
            ..
        } = shape
        else {
            panic!("{shape:?}");
        };
        assert!((width.as_mm() - 200.0).abs() < 1e-3);
        assert!((height.as_mm() - 100.0).abs() < 1e-3);
        assert!(
            (corner_radius.as_mm() - 10.0).abs() < 0.05,
            "{corner_radius:?}"
        );
        assert!((turn.as_radians() - 0.3).abs() < 1e-6);
    }

    #[test]
    fn a_wobbly_outline_is_no_shape() {
        let points: Vec<(f64, f64)> = (0..720)
            .map(|k| {
                let t = f64::from(k) * std::f64::consts::TAU / 720.0;
                let r = 25.0 * (1.0 + 0.03 * (3.0 * t).sin());
                (r * t.cos(), r * t.sin())
            })
            .collect();
        assert_eq!(fit_outline(&sampled(&polygon(&points)), 0.2), None);
    }
}
