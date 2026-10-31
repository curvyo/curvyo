//! Measures the two-sided distance between two sampled outlines.
//!
//! Every point of one side is measured to the nearest segment of the other side's closed rings,
//! found through a uniform grid of `Vec` buckets (no hash map). The maximum of both directions
//! (Hausdorff) is compared with the tolerance, the mean ranks candidates (`adrs.md` decision 4).
//! A translation can be applied to the query points, so a glyph is moved without rebuilding.

use kurbo::{Point, Rect, Vec2};

use super::blobs::bounds;
use super::sample::f;

/// The grid's cells along its longer side.
const CELLS: f64 = 40.0;

/// The segments of closed rings in grid buckets.
#[derive(Debug, Clone)]
pub(crate) struct SegmentGrid {
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    segments: Vec<(Point, Point)>,
    buckets: Vec<Vec<u32>>,
    /// Distances beyond this are reported as this: far points only need to count as far.
    cap: f64,
}

fn index_of(v: f64, cells: usize) -> usize {
    if v <= 0.0 || !v.is_finite() {
        return 0;
    }
    // invariant: clamped below the cell count, which fits a usize.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = v.min(f(cells - 1)) as usize;
    at
}

impl SegmentGrid {
    /// The grid of the rings' segments; distances are measured up to `cap`.
    pub(crate) fn new(rings: &[&[Point]], cap: f64) -> Self {
        let points: Vec<Point> = rings.iter().flat_map(|r| r.iter().copied()).collect();
        let bbox: Rect = bounds(&points);
        let side = bbox.width().max(bbox.height()).max(1e-6);
        let cell = side / CELLS;
        let columns = index_of(bbox.width() / cell, 4096) + 1;
        let rows = index_of(bbox.height() / cell, 4096) + 1;
        let mut grid = Self {
            origin: Point::new(bbox.x0, bbox.y0),
            cell,
            columns,
            rows,
            segments: Vec::with_capacity(points.len()),
            buckets: vec![Vec::new(); columns * rows],
            cap,
        };
        for ring in rings {
            for (i, &a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                let id = u32::try_from(grid.segments.len()).unwrap_or(u32::MAX);
                grid.segments.push((a, b));
                let (c0, r0) = grid.cell_of(Point::new(a.x.min(b.x), a.y.min(b.y)));
                let (c1, r1) = grid.cell_of(Point::new(a.x.max(b.x), a.y.max(b.y)));
                for row in r0..=r1 {
                    for column in c0..=c1 {
                        grid.buckets[row * grid.columns + column].push(id);
                    }
                }
            }
        }
        grid
    }

    /// The same grid measuring every distance in full, however far (the notice's deviation).
    pub(crate) fn uncapped(&self) -> Self {
        Self {
            // Finite, so the search reach is the whole grid (a kilometre is beyond any drawing).
            cap: 1e6,
            ..self.clone()
        }
    }

    fn cell_of(&self, p: Point) -> (usize, usize) {
        (
            index_of((p.x - self.origin.x) / self.cell, self.columns),
            index_of((p.y - self.origin.y) / self.cell, self.rows),
        )
    }

    /// The nearest point of the segments to `p` and its squared distance.
    #[allow(clippy::many_single_char_names)]
    pub(crate) fn nearest(&self, p: Point) -> (Point, f64) {
        let (c, r) = self.cell_of(p);
        // How far `p` lies outside the grid: rings closer than that cannot be skipped.
        let outside = {
            let x0 = self.origin.x;
            let y0 = self.origin.y;
            let x1 = x0 + f(self.columns) * self.cell;
            let y1 = y0 + f(self.rows) * self.cell;
            let dx = (x0 - p.x).max(p.x - x1).max(0.0);
            let dy = (y0 - p.y).max(p.y - y1).max(0.0);
            dx.max(dy)
        };
        let mut best = (p, self.cap * self.cap);
        // How far `p` lies from the borders of its own cell: every cell `radius` rings out is
        // at least `radius - 1` cells plus that far away.
        let inner = {
            let fx = (p.x - self.origin.x) / self.cell - f(c);
            let fy = (p.y - self.origin.y) / self.cell - f(r);
            fx.min(1.0 - fx).min(fy).min(1.0 - fy).clamp(0.0, 1.0) * self.cell
        };
        let reach = self
            .columns
            .max(self.rows)
            .min(index_of(self.cap / self.cell, 1 << 20) + 2);
        for radius in 0..=reach {
            // Every segment not yet seen lies at least this far away.
            let bound = if radius == 0 {
                0.0
            } else {
                outside.max((f(radius) - 1.0) * self.cell + inner)
            };
            if best.1 <= bound * bound {
                break;
            }
            let (r0, r1) = (r.saturating_sub(radius), (r + radius).min(self.rows - 1));
            let (c0, c1) = (c.saturating_sub(radius), (c + radius).min(self.columns - 1));
            for row in r0..=r1 {
                let edge_row = row.abs_diff(r) == radius;
                // Inside the ring only its two side cells, on its top and bottom every cell.
                let columns: &[usize] = if edge_row {
                    &[]
                } else {
                    &[c.wrapping_sub(radius), c + radius]
                };
                let mut visit = |column: usize| {
                    for &id in &self.buckets[row * self.columns + column] {
                        let (a, b) = self.segments[id as usize];
                        let q = closest_on_segment(p, a, b);
                        let d = (q - p).hypot2();
                        if d < best.1 {
                            best = (q, d);
                        }
                    }
                };
                if edge_row {
                    for column in c0..=c1 {
                        visit(column);
                    }
                } else {
                    for &column in columns {
                        if radius > 0 && column < self.columns && column.abs_diff(c) == radius {
                            visit(column);
                        }
                    }
                }
            }
        }
        best
    }
}

fn closest_on_segment(p: Point, a: Point, b: Point) -> Point {
    let ab = b - a;
    let length = ab.hypot2();
    if length <= 0.0 {
        return a;
    }
    let t = ((p - a).dot(ab) / length).clamp(0.0, 1.0);
    a + ab * t
}

/// The two-sided distance of two outlines.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Distance {
    /// The largest distance of any point to the other side.
    pub max: f64,
    /// The mean distance over the points of both sides.
    pub mean: f64,
}

/// One side of a comparison: its points and the grid of its segments.
pub(crate) struct Side<'a> {
    pub points: &'a [Point],
    pub grid: &'a SegmentGrid,
}

/// The distance of `moved` translated by `shift` and `fixed`, and the mean residual vector that
/// would move `moved` onto `fixed`.
pub(crate) fn two_sided(fixed: &Side<'_>, moved: &Side<'_>, shift: Vec2) -> (Distance, Vec2) {
    let mut max = 0.0_f64;
    let mut sum = 0.0;
    let mut pull = Vec2::ZERO;
    for &p in moved.points {
        let at = p + shift;
        let (q, d2) = fixed.grid.nearest(at);
        max = max.max(d2);
        sum += d2.sqrt();
        pull += q - at;
    }
    for &q in fixed.points {
        let (p, d2) = moved.grid.nearest(q - shift);
        max = max.max(d2);
        sum += d2.sqrt();
        pull += q - (p + shift);
    }
    let count = f(moved.points.len() + fixed.points.len()).max(1.0);
    (
        Distance {
            max: max.sqrt(),
            mean: sum / count,
        },
        pull / count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, side: f64, step: usize) -> Vec<Point> {
        let mut points = Vec::new();
        let corners = [
            Point::new(x, 0.0),
            Point::new(x + side, 0.0),
            Point::new(x + side, side),
            Point::new(x, side),
        ];
        for i in 0..4 {
            for k in 0..step {
                points.push(corners[i].lerp(corners[(i + 1) % 4], f(k) / f(step)));
            }
        }
        points
    }

    #[test]
    fn a_shifted_square_measures_its_shift_and_pulls_back() {
        let a = square(0.0, 10.0, 50);
        let b = square(0.3, 10.0, 50);
        let (ga, gb) = (
            SegmentGrid::new(&[&a], 100.0),
            SegmentGrid::new(&[&b], 100.0),
        );
        let fixed = Side {
            points: &a,
            grid: &ga,
        };
        let moved = Side {
            points: &b,
            grid: &gb,
        };
        let (d, pull) = two_sided(&fixed, &moved, Vec2::ZERO);
        assert!((d.max - 0.3).abs() < 1e-9, "{d:?}");
        assert!(pull.x < 0.0);
        let (d, _) = two_sided(&fixed, &moved, Vec2::new(-0.3, 0.0));
        assert!(d.max < 1e-9, "{d:?}");
    }

    #[test]
    fn far_points_find_their_segment() {
        let a = square(0.0, 1.0, 4);
        let grid = SegmentGrid::new(&[&a], 100.0);
        let (q, d2) = grid.nearest(Point::new(5.0, 0.5));
        assert!((q - Point::new(1.0, 0.5)).hypot() < 1e-12);
        assert!((d2 - 16.0).abs() < 1e-9);
    }
}
