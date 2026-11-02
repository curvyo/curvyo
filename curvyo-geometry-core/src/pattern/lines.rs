//! The line generators (`specs/0071-pattern-fill` criteria 5, 11): Hatch, and through it
//! Crosshatch and Grid, and Waves. Lines run over the region's box in the pattern's frame, one
//! spacing apart counted from the frame's origin, so they move with the shape.

use curvyo_document_core::{Angle, Length, Point, Vec2};

use super::{Bounds, Raw};

/// How far lines run past the box, mm, so their ends are cut by the clip and never short.
const OVERRUN_MM: f64 = 1.0;

/// The unit direction of `angle` and its normal, from `libm` (the same bits on every platform).
fn axes(angle: Angle) -> (Vec2, Vec2) {
    let (sin, cos) = (libm::sin(angle.as_radians()), libm::cos(angle.as_radians()));
    (Vec2::new(cos, sin), Vec2::new(-sin, cos))
}

fn dot(p: Point, v: Vec2) -> f64 {
    p.x * v.x + p.y * v.y
}

/// The range of the box's corners projected on `v`.
fn span(bounds: Bounds, v: Vec2) -> (f64, f64) {
    let (lo, hi) = bounds;
    let corners = [lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y)];
    let values = corners.map(|c| dot(c, v));
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

/// The line indices `k` whose offset `k x spacing` along the normal lies in the box widened by
/// `reach` on both sides.
fn indices(
    bounds: Bounds,
    normal: Vec2,
    spacing: f64,
    reach: f64,
) -> std::ops::RangeInclusive<i64> {
    let (min, max) = span(bounds, normal);
    #[allow(clippy::cast_possible_truncation)] // bounded by the count limit before generation
    let range = ((min - reach) / spacing).ceil() as i64..=((max + reach) / spacing).floor() as i64;
    range
}

/// How many lines Hatch draws over the box.
pub(crate) fn hatch_count(bounds: Bounds, spacing: Length, angle: Angle) -> usize {
    let (_, normal) = axes(angle);
    let (min, max) = span(bounds, normal);
    count_between(min, max, spacing.as_mm())
}

/// The number of multiples of `step` between `min` and `max`, saturating.
pub(crate) fn count_between(min: f64, max: f64, step: f64) -> usize {
    let n = ((max - min) / step).floor() + 1.0;
    if n.is_finite() && n > 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = n.min(1e15) as usize;
        count
    } else {
        usize::MAX
    }
}

fn point_at(along: Vec2, normal: Vec2, t: f64, offset: f64) -> Point {
    Point::new(
        along.x * t + normal.x * offset,
        along.y * t + normal.y * offset,
    )
}

/// Parallel straight lines `spacing` apart at `angle`.
pub(crate) fn hatch(bounds: Bounds, spacing: Length, angle: Angle) -> Vec<Raw> {
    let (along, normal) = axes(angle);
    let s = spacing.as_mm();
    let (t0, t1) = span(bounds, along);
    indices(bounds, normal, s, 0.0)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let offset = k as f64 * s;
            let a = point_at(along, normal, t0 - OVERRUN_MM, offset);
            let b = point_at(along, normal, t1 + OVERRUN_MM, offset);
            Raw::open(vec![
                (a, Vec2::ZERO, Vec2::ZERO),
                (b, Vec2::ZERO, Vec2::ZERO),
            ])
        })
        .collect()
}

/// The sine's value and slope factor at the quarter-wave node `m`: `sin` and `cos` of `m x π/2`.
const QUARTERS: [(f64, f64); 4] = [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)];

/// Parallel sine lines: Hatch's lines bent into waves of `wavelength` and `amplitude`. Each
/// quarter wave is one cubic through the sine's nodes with its exact end tangents (the Hermite
/// cubic, within 0.6 % of the amplitude). Amplitude 0 is Hatch itself (criterion 11).
pub(crate) fn waves(
    bounds: Bounds,
    wavelength: Length,
    amplitude: Length,
    spacing: Length,
    angle: Angle,
) -> Vec<Raw> {
    let a = amplitude.as_mm();
    if a == 0.0 {
        return hatch(bounds, spacing, angle);
    }
    let (along, normal) = axes(angle);
    let s = spacing.as_mm();
    let quarter = wavelength.as_mm() / 4.0;
    let slope = a * std::f64::consts::TAU / wavelength.as_mm();
    let (t0, t1) = span(bounds, along);
    #[allow(clippy::cast_possible_truncation)] // bounded by the count limit
    let (m0, m1) = (
        ((t0 - OVERRUN_MM) / quarter).floor() as i64,
        ((t1 + OVERRUN_MM) / quarter).ceil() as i64,
    );
    indices(bounds, normal, s, a)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let offset = k as f64 * s;
            let anchors = (m0..=m1)
                .map(|m| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (value, cosine) = QUARTERS[m.rem_euclid(4) as usize];
                    #[allow(clippy::cast_precision_loss)]
                    let t = m as f64 * quarter;
                    let point = point_at(along, normal, t, offset + a * value);
                    let h = quarter / 3.0;
                    let out = point_at(along, normal, h, h * slope * cosine);
                    let handle = Vec2::new(out.x, out.y);
                    (point, handle.negated(), handle)
                })
                .collect();
            Raw::open(anchors)
        })
        .collect()
}

/// How many lines Waves draws over the box, and how many quarter-wave nodes each has.
pub(crate) fn waves_count(
    bounds: Bounds,
    wavelength: Length,
    amplitude: Length,
    spacing: Length,
    angle: Angle,
) -> (usize, usize) {
    let (along, normal) = axes(angle);
    let (n0, n1) = span(bounds, normal);
    let lines = count_between(
        n0 - amplitude.as_mm(),
        n1 + amplitude.as_mm(),
        spacing.as_mm(),
    );
    let (t0, t1) = span(bounds, along);
    let nodes =
        count_between(t0 - OVERRUN_MM, t1 + OVERRUN_MM, wavelength.as_mm() / 4.0).saturating_add(1);
    (lines, nodes)
}
