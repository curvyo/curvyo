//! The regular polygon or star a sampled traced outline is within a tolerance of
//! (`specs/0088-trace-recognition` Part B criteria 3, 4 and 7): the count from the strongest
//! harmonic of the outline's radius, every side folded onto the first one and fitted as one line
//! (the turn where the folded sides agree best), a polygon before a star.

// The formulas' own names (x, y, u, v, a, b, r, ...) read better than long ones here.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::f64::consts::{PI, TAU};

use crate::{Angle, Mm};

use crate::shapes::fit::{Fitted, at, half_angle, within};
use crate::shapes::math::{atan2, sin_cos};
use crate::shapes::sample::{Sampled, length};
use crate::shapes::{POINTS, Primitive, valid_ratio};

/// The most sides of a polygon and points of a star looked for (Proposal; more sides read as a
/// circle within the tolerance).
const MOST_CORNERS: usize = 24;

/// The part of each side, about its middle, a line is fitted to (the corners are left out).
const SIDE_MIDDLE: f64 = 0.6;

/// The fewest points a side's line is fitted to.
const FEWEST_SIDE_POINTS: f64 = 3.0;

/// The regular polygon of the outline's count if it fits, else its star.
pub(crate) fn polygon_or_star(outline: &Sampled, tolerance: f64) -> Option<Fitted> {
    let angles = angles_about_centroid(outline)?;
    let (n, (cn, sn)) = strongest_harmonic(outline)?;
    let alpha = atan2(sn, cn) / f64::from(n);
    fit(outline, &angles, n, alpha, false, tolerance)
        .or_else(|| fit(outline, &angles, n, alpha, true, tolerance))
}

/// How far, in all, the outline may step back around its centroid and still count as turning one
/// way (a traced corner can step back by a fraction of a pixel; Proposal: 1 % of a turn).
const MOST_BACK: f64 = 0.01 * TAU;

/// Each point's angle seen from the centroid, when the outline turns one way around it (a star
/// shape seen from its centre), but for small steps back at traced corners.
fn angles_about_centroid(outline: &Sampled) -> Option<Vec<f64>> {
    let sign = outline.signed_area.signum();
    let mut back = 0.0;
    for i in 0..outline.points.len() {
        let (a, b) = outline.segment(i);
        let step = (a[0] * b[1] - a[1] * b[0]) * sign;
        if step < 0.0 {
            let (ra, rb) = (length(a[0], a[1]), length(b[0], b[1]));
            if ra == 0.0 || rb == 0.0 {
                return None;
            }
            back -= step / (ra * rb);
        }
    }
    (back <= MOST_BACK).then(|| outline.points.iter().map(|p| atan2(p[1], p[0])).collect())
}

/// The count from 3 to [`MOST_CORNERS`] whose harmonic of the radius over the angle is the
/// strongest, with the cosine and sine of its phase (`n·α`, α the angle of the corner farthest
/// out).
fn strongest_harmonic(outline: &Sampled) -> Option<(u32, (f64, f64))> {
    let mut sums = [[0.0_f64; 2]; MOST_CORNERS + 1];
    for i in 0..outline.points.len() {
        let (a, b) = outline.segment(i);
        let (ra, rb) = (length(a[0], a[1]), length(b[0], b[1]));
        if ra == 0.0 || rb == 0.0 {
            return None;
        }
        // The angle the piece spans (its sine; a piece is about a pixel long) and its radius.
        let step = ((a[0] * b[1] - a[1] * b[0]) / (ra * rb)).abs();
        let radius = f64::midpoint(ra, rb);
        let (x, y) = (a[0] / ra, a[1] / ra);
        let (mut zc, mut zs) = (1.0, 0.0);
        for sum in sums.iter_mut().skip(1) {
            (zc, zs) = (zc * x - zs * y, zc * y + zs * x);
            sum[0] += radius * step * zc;
            sum[1] += radius * step * zs;
        }
    }
    let mut best = 0;
    let mut best_power = 0.0;
    for (k, sum) in sums.iter().enumerate().skip(3) {
        let power = sum[0] * sum[0] + sum[1] * sum[1];
        if power > best_power {
            (best, best_power) = (k, power);
        }
    }
    let m = best_power.sqrt();
    if best == 0 || m == 0.0 {
        return None;
    }
    let n = u32::try_from(best).ok()?;
    Some((n, (sums[best][0] / m, sums[best][1] / m)))
}

/// The distance from `p` to the segment `a`–`b`.
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    };
    length(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)
}

/// A folded point and its folded angle.
type Folded = ([f64; 2], f64);

/// The outline's points folded onto the first corner's wedge: each point turned back by its
/// sector of 2π/n from `alpha`, and mirrored about the wedge's middle line when it lies past it,
/// so every side of a regular figure lands on one segment from the first corner (angle 0) to the
/// angle π/n. Each folded point comes with its folded angle in [0, π/n].
fn folded(outline: &Sampled, angles: &[f64], n: u32, alpha: f64) -> Vec<Folded> {
    let period = TAU / f64::from(n);
    let half = period / 2.0;
    let turns: Vec<(f64, f64)> = (0..n)
        .map(|k| sin_cos(alpha + period * f64::from(k)))
        .collect();
    let (us, uc) = sin_cos(half);
    outline
        .points
        .iter()
        .zip(angles)
        .map(|(q, &angle)| {
            let rel = (angle - alpha).rem_euclid(TAU);
            let sector = (rel / period).floor();
            // invariant: `rel` lies in [0, 2π], so the sector is below n + 1.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (s, c) = turns[(sector as usize) % turns.len()];
            let v = [q[0] * c + q[1] * s, -q[0] * s + q[1] * c];
            let within = rel - sector * period;
            if within <= half {
                (v, within)
            } else {
                let d = 2.0 * (v[0] * uc + v[1] * us);
                ([d * uc - v[0], d * us - v[1]], period - within)
            }
        })
        .collect()
}

/// The figure's outer and inner radius (a polygon's inner is its apothem) from the folded points
/// `keep` selects, and the sum of their squared distances from it: a polygon's side is
/// perpendicular to the wedge's middle line, a star's side is the least-squares line.
fn radii(
    points: &[Folded],
    half: f64,
    star: bool,
    keep: impl Fn([f64; 2], f64) -> bool,
) -> Option<(f64, f64, f64)> {
    let (us, uc) = sin_cos(half);
    let (mut count, mut x, mut y, mut xx, mut yy, mut xy, mut dd, mut d1) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for &(p, angle) in points {
        if !keep(p, angle) {
            continue;
        }
        let d = p[0] * uc + p[1] * us;
        count += 1.0;
        x += p[0];
        y += p[1];
        xx += p[0] * p[0];
        yy += p[1] * p[1];
        xy += p[0] * p[1];
        d1 += d;
        dd += d * d;
    }
    if count < FEWEST_SIDE_POINTS {
        return None;
    }
    if !star {
        let apothem = d1 / count;
        return Some((apothem / uc, apothem, dd - d1 * d1 / count));
    }
    let (mx, my) = (x / count, y / count);
    let (cxx, cyy, cxy) = (
        xx / count - mx * mx,
        yy / count - my * my,
        xy / count - mx * my,
    );
    let d = length(cxx - cyy, 2.0 * cxy);
    if d == 0.0 {
        return None;
    }
    let (c, s) = half_angle((cxx - cyy) / d, 2.0 * cxy / d);
    // The spread across the line: the smaller eigenvalue.
    let residual = (f64::midpoint(cxx + cyy, -d)).max(0.0) * count;
    if s.abs() < 1e-12 {
        return None;
    }
    // Where the line meets the x axis (the corner) and the wedge's middle line (the valley).
    let outer = mx - my / s * c;
    let across = c * us - s * uc;
    if across.abs() < 1e-12 {
        return None;
    }
    let t = -(mx * us - my * uc) / across;
    let inner = length(mx + t * c, my + t * s);
    (outer > 0.0 && inner > 0.0 && inner < outer).then_some((outer, inner, residual))
}

/// How far the turn may be from the harmonic's before the search stops looking (2°).
const TURN_SEARCH: f64 = 0.034_906_585_039_886_59;

/// Steps of the search for the turn (each keeps two thirds of the range).
const TURN_STEPS: u32 = 50;

/// The turn near `alpha` whose folded sides lie best on one line, the figure's radii there, and
/// the folded points: a ternary search over the folding's spread (a wrong turn folds the two
/// sides of a corner onto two crossing lines).
fn best_turn(
    outline: &Sampled,
    angles: &[f64],
    n: u32,
    alpha: f64,
    star: bool,
) -> Option<(f64, f64, f64, Vec<Folded>)> {
    let half = PI / f64::from(n);
    let middle = |_: [f64; 2], angle: f64| (angle / half - 0.5).abs() <= SIDE_MIDDLE / 2.0;
    let spread = |a: f64| {
        radii(&folded(outline, angles, n, a), half, star, middle).map_or(f64::INFINITY, |r| r.2)
    };
    let reach = TURN_SEARCH.min(half / 4.0);
    let (mut lo, mut hi) = (alpha - reach, alpha + reach);
    for _ in 0..TURN_STEPS {
        let (a, b) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
        if spread(a) <= spread(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    let turn = f64::midpoint(lo, hi);
    let points = folded(outline, angles, n, turn);
    let (outer, inner, _) = radii(&points, half, star, middle)?;
    Some((turn, outer, inner, points))
}

/// The polygon (or with `star` the star) of `n` corners whose first corner points about along
/// `alpha` from the centroid, fitted to the outline's folded sides, if every point is within
/// `tolerance`. A second fit leaves out the ends of the side the tracer rounds or cuts: a sharp
/// tip loses about half a pixel over the tangent of its half angle.
fn fit(
    outline: &Sampled,
    angles: &[f64],
    n: u32,
    alpha: f64,
    star: bool,
    tolerance: f64,
) -> Option<Fitted> {
    let half = PI / f64::from(n);
    let (turn, outer, inner, points) = best_turn(outline, angles, n, alpha, star)?;
    let (us, uc) = sin_cos(half);
    let (tip, valley) = ([outer, 0.0], [inner * uc, inner * us]);
    let side = length(valley[0] - tip[0], valley[1] - tip[1]);
    // The tangent of the half angle at the first corner.
    let slope = valley[1] / (tip[0] - valley[0]).max(f64::MIN_POSITIVE);
    let (cut_tip, cut_valley) = (
        (2.0 * tolerance / slope).min(0.4 * side),
        (2.0 * tolerance).min(0.4 * side),
    );
    let ends = |p: [f64; 2], _: f64| {
        length(p[0] - tip[0], p[1] - tip[1]) > cut_tip
            && length(p[0] - valley[0], p[1] - valley[1]) > cut_valley
    };
    let (outer, inner, _) = radii(&points, half, star, ends).unwrap_or((outer, inner, 0.0));
    let m = if star { 2 * n } else { n };
    let step = TAU / f64::from(m);
    let vertices: Vec<[f64; 2]> = (0..m)
        .map(|j| {
            let r = if j % 2 == 1 && star { inner } else { outer };
            let (s, c) = sin_cos(turn + step * f64::from(j));
            [r * c, r * s]
        })
        .collect();
    let area = vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        .abs()
        / 2.0;
    let count = vertices.len();
    let distance = |i: usize, p: [f64; 2]| {
        let rel = (angles[i] - turn).rem_euclid(TAU) / step;
        // invariant: `rel` lies in [0, m].
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let j = (rel.floor() as usize) % count;
        [count - 1, 0, 1]
            .iter()
            .map(|k| {
                let side = (j + k) % count;
                segment_distance(p, vertices[side], vertices[(side + 1) % count])
            })
            .fold(f64::INFINITY, f64::min)
    };
    let shape = figure_shape(outline, n, star, turn, outer, inner)?;
    within(outline, tolerance, area, distance).then_some((shape, Angle::from_radians(0.0)))
}

/// The polygon (or with `star` the star) of `n` corners about the outline's centroid, its first
/// outer corner at `turn`, of radii `outer` and `inner`.
fn figure_shape(
    outline: &Sampled,
    n: u32,
    star: bool,
    turn: f64,
    outer: f64,
    inner: f64,
) -> Option<Primitive> {
    let centre = at(outline, [0.0, 0.0]);
    let radius = Mm::from_mm(outer);
    let angle = Angle::from_radians(turn);
    let points = POINTS.contains(&n).then_some(n)?;
    Some(if star {
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio: valid_ratio(inner / outer)?,
        }
    } else {
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::sample::sample_closed;
    use kurbo::BezPath;

    fn figure(n: u32, outer: f64, inner: Option<f64>, turn: f64) -> BezPath {
        let m = if inner.is_some() { 2 * n } else { n };
        let mut p = BezPath::new();
        for j in 0..m {
            let r = if j % 2 == 1 {
                inner.unwrap_or(outer)
            } else {
                outer
            };
            let t = turn + TAU * f64::from(j) / f64::from(m);
            let q = (50.0 + r * t.cos(), 60.0 + r * t.sin());
            if j == 0 {
                p.move_to(q);
            } else {
                p.line_to(q);
            }
        }
        p.close_path();
        p
    }

    #[test]
    fn a_hexagon_is_a_polygon_of_six() {
        let s = sample_closed(&figure(6, 30.0, None, 0.2), 0.1).unwrap();
        let (shape, _) = polygon_or_star(&s, 0.2).unwrap();
        let Primitive::Polygon {
            radius,
            angle,
            points,
            ..
        } = shape
        else {
            panic!("{shape:?}");
        };
        assert_eq!(points, 6);
        assert!((radius.as_mm() - 30.0).abs() < 1e-6);
        let off = (angle.as_radians() - 0.2).rem_euclid(TAU / 6.0);
        assert!(off.min(TAU / 6.0 - off) < 1e-7, "{angle:?}");
    }

    #[test]
    fn a_five_point_star_has_its_radii_and_turn() {
        let s = sample_closed(&figure(5, 40.0, Some(20.0), -0.5), 0.1).unwrap();
        let (shape, _) = polygon_or_star(&s, 0.2).unwrap();
        let Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } = shape
        else {
            panic!("{shape:?}");
        };
        assert_eq!(points, 5);
        assert!((radius.as_mm() - 40.0).abs() < 1e-6);
        assert!((inner_ratio - 0.5).abs() < 1e-6);
        assert!((centre.x - 50.0).abs() < 1e-6 && (centre.y - 60.0).abs() < 1e-6);
        let off = (angle.as_radians() + 0.5).rem_euclid(TAU / 5.0);
        assert!(off.min(TAU / 5.0 - off) < 1e-7, "{angle:?}");
    }
}
