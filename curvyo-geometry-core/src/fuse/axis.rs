//! A bridge's axis and its band (`specs/0156-fused-lettering` criterion 16a and 16b, `adrs.md`
//! decision 3 step 7): straight when both boundary normals lie within 5 degrees of the line
//! joining the gap points, else one cubic leaving each piece along its normal; the band is a
//! ribbon of the width around the axis, extended into each piece and clipped to it.

use i_overlay::core::overlay_rule::OverlayRule;

use crate::boolean_grid::{Path64, Paths64, normalize, run};
use crate::fuse::gap::Gap;
use crate::fuse::measure::{P, positive, ribbon_quad};

/// `cos 5°`, a literal so that no platform's `cos` takes part.
const COS_FIVE_DEGREES: f64 = 0.996_194_698_091_745_5;

/// How many times a cubic's handles are halved before the axis falls back to a straight line.
const MAX_HALVINGS: usize = 4;

/// Points per cubic axis.
const CUBIC_STEPS: usize = 48;

/// The axis from `gap.on_a` to `gap.on_b` as a polyline.
#[allow(clippy::many_single_char_names)] // the usual names of plane geometry
pub(crate) fn axis(gap: &Gap) -> Vec<P> {
    let (p, q) = (gap.on_a, gap.on_b);
    let span = q.sub(p);
    let d = span.length();
    if d == 0.0 {
        return vec![p, q];
    }
    let u = span.scale(1.0 / d);
    if gap.normal_a.dot(u) >= COS_FIVE_DEGREES
        && gap.normal_b.dot(u.scale(-1.0)) >= COS_FIVE_DEGREES
    {
        return vec![p, q];
    }
    let mut handle = d / 3.0;
    for _ in 0..=MAX_HALVINGS {
        let c1 = p.add(gap.normal_a.scale(handle));
        let c2 = q.add(gap.normal_b.scale(handle));
        if inflections(p, c1, c2, q) <= 1 {
            return (0..=CUBIC_STEPS)
                .map(|k| {
                    #[allow(clippy::cast_precision_loss)] // small counts
                    let t = k as f64 / CUBIC_STEPS as f64;
                    cubic_point(p, c1, c2, q, t)
                })
                .collect();
        }
        handle /= 2.0;
    }
    vec![p, q]
}

fn cubic_point(p0: P, p1: P, p2: P, p3: P, t: f64) -> P {
    let s = 1.0 - t;
    p0.scale(s * s * s)
        .add(p1.scale(3.0 * s * s * t))
        .add(p2.scale(3.0 * s * t * t))
        .add(p3.scale(t * t * t))
}

/// How often the curvature of the cubic changes sign, sampled.
fn inflections(p0: P, p1: P, p2: P, p3: P) -> usize {
    let first = |t: f64| {
        let s = 1.0 - t;
        p1.sub(p0)
            .scale(3.0 * s * s)
            .add(p2.sub(p1).scale(6.0 * s * t))
            .add(p3.sub(p2).scale(3.0 * t * t))
    };
    let second = |t: f64| {
        p2.sub(p1.scale(2.0))
            .add(p0)
            .scale(6.0 * (1.0 - t))
            .add(p3.sub(p2.scale(2.0)).add(p1).scale(6.0 * t))
    };
    let scale = p3.sub(p0).length().powi(3).max(1.0) * 1e-9;
    let mut last = 0.0_f64;
    let mut changes = 0;
    for k in 0..=CUBIC_STEPS {
        #[allow(clippy::cast_precision_loss)]
        let t = k as f64 / CUBIC_STEPS as f64;
        let c = first(t).cross(second(t));
        if c.abs() <= scale {
            continue;
        }
        if last != 0.0 && (c > 0.0) != (last > 0.0) {
            changes += 1;
        }
        last = c;
    }
    changes
}

/// A ribbon of half width `half` around the polyline `axis`, as one polygon.
fn ribbon(axis: &[P], half: f64) -> Path64 {
    if axis.len() == 2 {
        return ribbon_quad(axis[0], axis[1], half);
    }
    let side = |i: usize| {
        let before = axis[i.saturating_sub(1)];
        let after = axis[(i + 1).min(axis.len() - 1)];
        let t = after.sub(before).unit();
        P::new(-t.y, t.x).scale(half)
    };
    let mut path: Path64 = (0..axis.len())
        .map(|i| axis[i].add(side(i)).snap())
        .collect();
    path.extend((0..axis.len()).rev().map(|i| axis[i].sub(side(i)).snap()));
    path.dedup();
    positive(path)
}

/// The band of `gap` with `width` grid units: the ribbon around the axis, plus an extension of
/// one width into each piece along its inward normal, clipped to that piece (`a`, `b`: the
/// pieces' contours).
pub(crate) fn band(gap: &Gap, width: f64, a: &Paths64, b: &Paths64) -> Paths64 {
    let half = width / 2.0;
    let mut parts: Paths64 = vec![ribbon(&axis(gap), half)];
    for (at, normal, piece) in [(gap.on_a, gap.normal_a, a), (gap.on_b, gap.normal_b, b)] {
        let inward = at.sub(normal.scale(width));
        let extension = vec![ribbon_quad(at.add(normal.scale(half / 4.0)), inward, half)];
        parts.extend(run(OverlayRule::Intersect, &extension, piece));
    }
    normalize(&parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(on_a: P, normal_a: P, on_b: P, normal_b: P) -> Gap {
        Gap {
            distance: on_b.sub(on_a).length(),
            on_a,
            normal_a,
            on_b,
            normal_b,
        }
    }

    #[test]
    fn facing_normals_give_a_straight_axis() {
        let g = gap(
            P::new(0.0, 0.0),
            P::new(0.0, 1.0),
            P::new(0.0, 2_000.0),
            P::new(0.0, -1.0),
        );
        assert_eq!(axis(&g).len(), 2);
    }

    /// Criterion 16a: a vertical and a horizontal stroke get a curve leaving each along its
    /// normal, with at most one inflection.
    #[test]
    fn crossed_normals_give_one_cubic() {
        let g = gap(
            P::new(0.0, 0.0),
            P::new(1.0, 0.0),
            P::new(2_000.0, 2_000.0),
            P::new(0.0, -1.0),
        );
        let points = axis(&g);
        assert_eq!(points.len(), CUBIC_STEPS + 1);
        let start = points[1].sub(points[0]).unit();
        assert!(start.dot(P::new(1.0, 0.0)) > 0.99);
        let end = points[CUBIC_STEPS - 1].sub(points[CUBIC_STEPS]).unit();
        assert!(end.dot(P::new(0.0, -1.0)) > 0.99);
    }
}
