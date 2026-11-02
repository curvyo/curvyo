//! The lattice generators of closed pieces (`specs/0071-pattern-fill` criteria 6 to 8): Dots,
//! Honeycomb and Brick wall. Lattice points are counted from the frame's origin, so the pattern
//! moves with the shape; only `+ - * /` and `sqrt` are used.

use curvyo_document_core::{Length, Point, Vec2};

use super::lines::count_between;
use super::{Bounds, Raw};

/// The handle length of a quarter circle of radius 1 as one cubic.
pub(crate) const KAPPA: f64 = 0.552_284_749_830_793_6;

/// The rows and columns of a lattice of pitch `(dx, dy)`, with every second row shifted by
/// `shift`, whose cells of reach `reach` around their point can meet the box.
fn lattice(bounds: Bounds, dx: f64, dy: f64, shift: f64, reach: f64) -> Vec<Point> {
    let (lo, hi) = bounds;
    #[allow(clippy::cast_possible_truncation)] // bounded by the count limit before generation
    let rows = ((lo.y - reach) / dy).floor() as i64..=((hi.y + reach) / dy).ceil() as i64;
    let mut points = Vec::new();
    for j in rows {
        let offset = if j.rem_euclid(2) == 1 { shift } else { 0.0 };
        #[allow(clippy::cast_possible_truncation)]
        let columns = ((lo.x - reach - offset) / dx).floor() as i64
            ..=((hi.x + reach - offset) / dx).ceil() as i64;
        for i in columns {
            #[allow(clippy::cast_precision_loss)]
            points.push(Point::new(i as f64 * dx + offset, j as f64 * dy));
        }
    }
    points
}

/// How many lattice points of pitch `(dx, dy)` cover the box, roughly.
fn lattice_count(bounds: Bounds, dx: f64, dy: f64) -> usize {
    let (lo, hi) = bounds;
    count_between(lo.x - dx, hi.x + dx, dx).saturating_mul(count_between(lo.y - dy, hi.y + dy, dy))
}

/// A circle of radius `r` around `c` as four cubics.
pub(crate) fn circle(c: Point, r: f64) -> Raw {
    let k = KAPPA * r;
    let anchors = vec![
        (
            Point::new(c.x + r, c.y),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (
            Point::new(c.x, c.y + r),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
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
    ];
    Raw::closed(anchors)
}

fn polygon(points: &[Point]) -> Raw {
    Raw::closed(
        points
            .iter()
            .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
            .collect(),
    )
}

/// Dots: rows `spacing` apart, every second row shifted by half a spacing (criterion 6).
pub(crate) fn dots(bounds: Bounds, spacing: Length, diameter: Length) -> Vec<Raw> {
    let s = spacing.as_mm();
    let r = diameter.as_mm() / 2.0;
    lattice(bounds, s, s, s / 2.0, r)
        .into_iter()
        .map(|c| circle(c, r))
        .collect()
}

/// How many dots cover the box.
pub(crate) fn dots_count(bounds: Bounds, spacing: Length) -> usize {
    lattice_count(bounds, spacing.as_mm(), spacing.as_mm())
}

/// Honeycomb: pointed-top hexagons whose centres are `cell_size` apart, each `cell_size - wall`
/// across its flats, so neighbouring cells are `wall` apart (criterion 7).
pub(crate) fn honeycomb(bounds: Bounds, cell_size: Length, wall: Length) -> Vec<Raw> {
    let c = cell_size.as_mm();
    let half_root3 = 3.0_f64.sqrt() / 2.0;
    let across = c - wall.as_mm();
    let radius = across / 3.0_f64.sqrt();
    let (w, h) = (radius * half_root3, radius / 2.0);
    lattice(bounds, c, c * half_root3, c / 2.0, radius)
        .into_iter()
        .map(|p| {
            polygon(&[
                Point::new(p.x, p.y - radius),
                Point::new(p.x + w, p.y - h),
                Point::new(p.x + w, p.y + h),
                Point::new(p.x, p.y + radius),
                Point::new(p.x - w, p.y + h),
                Point::new(p.x - w, p.y - h),
            ])
        })
        .collect()
}

/// How many cells cover the box.
pub(crate) fn honeycomb_count(bounds: Bounds, cell_size: Length) -> usize {
    let c = cell_size.as_mm();
    lattice_count(bounds, c, c * 3.0_f64.sqrt() / 2.0)
}

/// Brick wall in running bond: bricks on a `width` by `height` pitch, each `joint` smaller in
/// both directions, every second row shifted by half a brick width (criterion 8).
pub(crate) fn bricks(bounds: Bounds, width: Length, height: Length, joint: Length) -> Vec<Raw> {
    let (w, h, j) = (width.as_mm(), height.as_mm(), joint.as_mm());
    let half = j / 2.0;
    lattice(bounds, w, h, w / 2.0, w.max(h))
        .into_iter()
        .map(|p| {
            polygon(&[
                Point::new(p.x + half, p.y + half),
                Point::new(p.x + w - half, p.y + half),
                Point::new(p.x + w - half, p.y + h - half),
                Point::new(p.x + half, p.y + h - half),
            ])
        })
        .collect()
}

/// How many bricks cover the box.
pub(crate) fn bricks_count(bounds: Bounds, width: Length, height: Length) -> usize {
    lattice_count(bounds, width.as_mm(), height.as_mm())
}
