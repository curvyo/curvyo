//! The dot grid of one raster item (ADR 0068 §2, `specs/0123` criterion 5): rows on the hatch's
//! scan lines, columns half an Interval inside the item's box, a dot wherever its centre lies
//! inside the item.

use curvyo_document_core::Point;

use crate::job_hatch::{ScanLines, scan_frame, scan_lines};
use crate::job_input::PointUm;

/// One row of the dot grid: its stretches of dots inside the item.
#[derive(Debug, Clone, PartialEq)]
pub struct GridRow {
    /// The row's index from the first row (the Bayer matrix and the diffusions use it).
    pub k: i64,
    /// Its across-coordinate in µm.
    pub s: f64,
    /// The stretches `[j0, j1)` of columns whose centre lies inside, increasing.
    pub stretches: Vec<(i64, i64)>,
}

/// The dot grid of one raster item.
#[derive(Debug, Clone, PartialEq)]
pub struct DotGrid {
    scan: ScanLines,
    /// The Interval in µm: dot size and row spacing.
    pub interval_um: i64,
    /// The along-coordinate of column 0's centre, in µm.
    pub first_t: i64,
    /// The number of columns, from 0.
    pub columns: usize,
    /// The rows with at least one dot, in order across the item.
    pub rows: Vec<GridRow>,
}

impl DotGrid {
    /// The grid of the area bounded by `rings` at `interval_um` and `angle_deg`, `None` when no
    /// dot centre falls inside.
    #[must_use]
    pub fn new(rings: &[Vec<PointUm>], interval_um: i64, angle_deg: f64) -> Option<Self> {
        let scan = scan_lines(rings, interval_um, angle_deg)?;
        let first_t = round_um(scan.along.0) + interval_um / 2;
        let mut columns = 0_i64;
        let rows: Vec<GridRow> = scan
            .lines
            .iter()
            .filter_map(|line| {
                let stretches: Vec<(i64, i64)> = line
                    .spans
                    .iter()
                    .map(|&(t0, t1)| {
                        let column = |t: f64| ceil_div(round_um(t) - first_t, interval_um).max(0);
                        (column(t0), column(t1))
                    })
                    .filter(|(j0, j1)| j1 > j0)
                    .collect();
                columns = columns.max(stretches.last().map_or(0, |s| s.1));
                (!stretches.is_empty()).then_some(GridRow {
                    k: line.k,
                    s: line.s,
                    stretches,
                })
            })
            .collect();
        (!rows.is_empty()).then(|| Self {
            scan,
            interval_um,
            first_t,
            columns: usize::try_from(columns).unwrap_or(0),
            rows,
        })
    }

    /// The along-coordinate of column `j`'s centre in µm.
    #[must_use]
    pub const fn centre_t(&self, j: i64) -> i64 {
        self.first_t + j * self.interval_um
    }

    /// The page point (mm) of the centre of dot `j` in the row at across-coordinate `s`.
    #[must_use]
    pub fn centre_mm(&self, s: f64, j: i64) -> Point {
        let along = as_f64(self.centre_t(j));
        let (normal, dir) = (self.scan.normal, self.scan.dir);
        Point::new(
            (s * normal.0 + along * dir.0) / 1000.0,
            (s * normal.1 + along * dir.1) / 1000.0,
        )
    }

    /// The page point of along-coordinate `t` µm on the row at `s`, rounded to µm.
    #[must_use]
    pub fn point(&self, s: f64, t: i64) -> PointUm {
        self.scan.point(s, as_f64(t))
    }
}

/// The dots an item would need, from its box in the scan frame (ADR 0068 §7): the box's area
/// over the Interval squared, before any work.
#[must_use]
pub fn estimated_dots(rings: &[Vec<PointUm>], interval_um: i64, angle_deg: f64) -> u64 {
    if interval_um <= 0 {
        return 0;
    }
    let (dir, normal) = scan_frame(angle_deg);
    let mut range = [(f64::INFINITY, f64::NEG_INFINITY); 2];
    for p in rings.iter().flatten() {
        let (x, y) = (as_f64(p.x), as_f64(p.y));
        for (axis, (lo, hi)) in [dir, normal].into_iter().zip(range.iter_mut()) {
            let v = x * axis.0 + y * axis.1;
            *lo = lo.min(v);
            *hi = hi.max(v);
        }
    }
    let i = as_f64(interval_um);
    let dots = ((range[0].1 - range[0].0) / i).max(0.0) * ((range[1].1 - range[1].0) / i).max(0.0);
    if dots.is_finite() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: finite and at least 0; `as` saturates
        let dots = dots.round() as u64;
        dots
    } else {
        0
    }
}

/// `n / d` rounded up, `d > 0`.
const fn ceil_div(n: i64, d: i64) -> i64 {
    n.div_euclid(d) + if n.rem_euclid(d) == 0 { 0 } else { 1 }
}

fn as_f64(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let value = um as f64;
    value
}

fn round_um(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; values are page coordinates in µm
    let um = value.round() as i64;
    um
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i64, y: i64, side: i64) -> Vec<PointUm> {
        vec![
            PointUm::new(x, y),
            PointUm::new(x + side, y),
            PointUm::new(x + side, y + side),
            PointUm::new(x, y + side),
        ]
    }

    #[test]
    fn a_square_grid_has_its_dots_half_an_interval_inside() {
        let grid = DotGrid::new(&[square(10_000, 20_000, 1_000)], 100, 0.0).unwrap();
        assert_eq!(grid.rows.len(), 10);
        assert_eq!(grid.columns, 10);
        assert!(grid.rows.iter().all(|r| r.stretches == vec![(0, 10)]));
        let c = grid.centre_mm(grid.rows[0].s, 0);
        assert!(
            (c.x - 10.05).abs() < 1e-9 && (c.y - 20.05).abs() < 1e-9,
            "{c:?}"
        );
        let last = grid.centre_mm(grid.rows[9].s, 9);
        assert!((last.x - 10.95).abs() < 1e-9 && (last.y - 20.95).abs() < 1e-9);
    }

    #[test]
    fn no_dot_lies_outside_a_triangle() {
        let ring = vec![
            PointUm::new(0, 0),
            PointUm::new(10_000, 0),
            PointUm::new(0, 10_000),
        ];
        let grid = DotGrid::new(std::slice::from_ref(&ring), 500, 0.0).unwrap();
        for row in &grid.rows {
            for &(j0, j1) in &row.stretches {
                for j in j0..j1 {
                    let c = grid.centre_mm(row.s, j);
                    assert!(c.x + c.y < 10.0, "{c:?} outside");
                }
            }
        }
        assert_eq!(
            grid.rows[0].stretches,
            vec![(0, 19)],
            "y 0.25: x up to 9.75"
        );
    }

    #[test]
    fn a_hole_splits_a_row_into_two_stretches() {
        let mut hole = square(4_000, 4_000, 2_000);
        hole.reverse();
        let grid = DotGrid::new(&[square(0, 0, 10_000), hole], 1_000, 0.0).unwrap();
        assert_eq!(grid.rows[4].stretches, vec![(0, 4), (6, 10)]);
    }

    #[test]
    fn the_scan_angle_turns_the_rows() {
        let grid = DotGrid::new(&[square(0, 0, 10_000)], 1_000, 90.0).unwrap();
        let row = &grid.rows[0];
        let (a, b) = (grid.centre_mm(row.s, 0), grid.centre_mm(row.s, 1));
        assert!((a.x - b.x).abs() < 1e-9, "a vertical row");
    }

    #[test]
    fn the_estimate_is_the_box_over_the_interval_squared() {
        let bed = square(0, 0, 300_000);
        assert_eq!(estimated_dots(&[bed], 10, 0.0), 900_000_000);
        assert_eq!(estimated_dots(&[square(0, 0, 10_000)], 100, 0.0), 10_000);
    }
}
