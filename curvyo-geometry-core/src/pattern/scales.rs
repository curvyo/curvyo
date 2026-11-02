//! Sea waves, Seigaiha (`specs/0071-pattern-fill` criterion 10): concentric half circles above
//! their centre, rows one scale radius apart, every second row shifted by one radius (half a
//! scale's width), so each scale rests on the two below.

use curvyo_document_core::{Length, PatternCount, Point, Vec2};

use super::cells::KAPPA;
use super::lines::count_between;
use super::{Bounds, Raw};

/// The rings that have a radius: `radius - k x spacing > 0` for `k` below `rings`.
fn radii(radius: f64, rings: PatternCount, spacing: f64) -> Vec<f64> {
    (0..rings.get())
        .map(|k| radius - f64::from(k) * spacing)
        .take_while(|r| *r > 0.0)
        .collect()
}

/// The upper half circle of radius `r` around `c` as two cubics, left to right; an inner ring
/// `follows` the outer one of its scale.
fn half_circle(c: Point, r: f64, follows: bool) -> Raw {
    let k = KAPPA * r;
    let make = if follows { Raw::following } else { Raw::open };
    make(vec![
        (
            Point::new(c.x - r, c.y),
            Vec2::new(0.0, k),
            Vec2::new(0.0, -k),
        ),
        (
            Point::new(c.x, c.y - r),
            Vec2::new(-k, 0.0),
            Vec2::new(k, 0.0),
        ),
        (
            Point::new(c.x + r, c.y),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
    ])
}

/// The scales over the box.
pub(crate) fn sea_waves(
    bounds: Bounds,
    radius: Length,
    rings: PatternCount,
    ring_spacing: Length,
) -> Vec<Raw> {
    let (lo, hi) = bounds;
    let r = radius.as_mm();
    let ring_radii = radii(r, rings, ring_spacing.as_mm());
    let mut out = Vec::new();
    #[allow(clippy::cast_possible_truncation)] // bounded by the count limit before generation
    let rows = (lo.y / r).floor() as i64..=((hi.y + r) / r).ceil() as i64;
    for j in rows {
        let shift = if j.rem_euclid(2) == 1 { r } else { 0.0 };
        #[allow(clippy::cast_possible_truncation)]
        let columns = ((lo.x - r - shift) / (2.0 * r)).floor() as i64
            ..=((hi.x + r - shift) / (2.0 * r)).ceil() as i64;
        for i in columns {
            #[allow(clippy::cast_precision_loss)]
            let centre = Point::new(i as f64 * 2.0 * r + shift, j as f64 * r);
            out.extend(
                ring_radii
                    .iter()
                    .enumerate()
                    .map(|(k, &ring)| half_circle(centre, ring, k > 0)),
            );
        }
    }
    out
}

/// How many rings of a scale have a radius: the arcs one scale draws.
pub(crate) fn rings_drawn(radius: Length, rings: PatternCount, ring_spacing: Length) -> usize {
    radii(radius.as_mm(), rings, ring_spacing.as_mm()).len()
}

/// How many arcs cover the box: only the rings that have a radius are drawn and counted.
pub(crate) fn sea_waves_count(
    bounds: Bounds,
    radius: Length,
    rings: PatternCount,
    ring_spacing: Length,
) -> usize {
    let (lo, hi) = bounds;
    let r = radius.as_mm();
    count_between(lo.x - 2.0 * r, hi.x + 2.0 * r, 2.0 * r)
        .saturating_mul(count_between(lo.y - r, hi.y + r, r))
        .saturating_mul(rings_drawn(radius, rings, ring_spacing))
}
