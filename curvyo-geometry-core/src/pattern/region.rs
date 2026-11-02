//! The region a pattern is clipped to (`specs/0071-pattern-fill` criteria 13 to 15): the shape's
//! outlines on the 0.001 mm grid, painted with the nonzero rule, inset by the margin, with a coarse
//! cell map that tells pieces wholly inside or outside from pieces that meet the boundary.

use curvyo_document_core::Point;

use crate::boolean::{Outline, grid_paths};
use crate::boolean_grid::{Path64, Paths64, Point64, normalize, snap, to_millimetres};
use crate::offset_polyline::inset_region;

/// Cells along the longer side of the region's box.
const CELLS_ALONG: i64 = 256;

/// What a cell is: crossed by the boundary, or not, with the winding number all over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Winding(i32),
    Boundary,
}

const OUTSIDE: Cell = Cell::Winding(0);

/// Where a box lies against the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// Every point of the box is inside.
    Inside,
    /// No point of the box is inside.
    Outside,
    /// The box meets the boundary.
    Boundary,
}

/// The clip region with its cell map.
pub(crate) struct Region {
    /// The contours, outer ones with positive area, holes negative.
    pub(crate) contours: Paths64,
    /// The smallest and largest grid point.
    pub(crate) min: Point64,
    pub(crate) max: Point64,
    cell: i64,
    columns: i64,
    rows: i64,
    cells: Vec<Cell>,
}

impl Region {
    /// The region of closed `outlines` flattened at `flatten_tolerance_mm`, inset by `margin_mm`;
    /// `None` when nothing is left.
    pub(crate) fn new(
        outlines: &[Outline<'_>],
        margin_mm: f64,
        flatten_tolerance_mm: f64,
    ) -> Option<Self> {
        let mut contours = normalize(&grid_paths(outlines, flatten_tolerance_mm));
        let margin = snap(Point::new(margin_mm, 0.0)).x;
        if margin > 0 && !contours.is_empty() {
            contours = inset_region(&contours, margin, flatten_tolerance_mm);
        }
        Self::of_contours(contours)
    }

    fn of_contours(contours: Paths64) -> Option<Self> {
        let points = || contours.iter().flatten();
        let min_x = points().map(|p| p.x).min()?;
        let min_y = points().map(|p| p.y).min()?;
        let max_x = points().map(|p| p.x).max()?;
        let max_y = points().map(|p| p.y).max()?;
        let longest = (max_x - min_x).max(max_y - min_y).max(1);
        let cell = (longest / CELLS_ALONG).max(1) + 1;
        let columns = (max_x - min_x) / cell + 1;
        let rows = (max_y - min_y) / cell + 1;
        let mut region = Self {
            contours,
            min: Point64::new(min_x, min_y),
            max: Point64::new(max_x, max_y),
            cell,
            columns,
            rows,
            cells: Vec::new(),
        };
        region.classify();
        Some(region)
    }

    /// The region's box in millimetres.
    pub(crate) fn bounds_mm(&self) -> (Point, Point) {
        (to_millimetres(&self.min), to_millimetres(&self.max))
    }

    /// The largest outer contour (by area), for Interpolation waves.
    pub(crate) fn largest_outer(&self) -> Option<&Path64> {
        self.contours
            .iter()
            .filter(|c| crate::boolean_grid::double_area(c) > 0)
            .max_by_key(|c| crate::boolean_grid::double_area(c))
    }

    fn index(&self, column: i64, row: i64) -> usize {
        // Both are inside the map, so the product is too.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = (row * self.columns + column) as usize;
        at
    }

    /// Marks the cells the boundary crosses, then every other cell by the winding at its centre.
    fn classify(&mut self) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total = (self.columns * self.rows) as usize;
        self.cells = vec![OUTSIDE; total];
        let edges: Vec<(Point64, Point64)> = self
            .contours
            .iter()
            .flat_map(|c| (0..c.len()).map(move |i| (c[i], c[(i + 1) % c.len()])))
            .collect();
        for &(a, b) in &edges {
            self.mark_edge(a, b);
        }
        // The edges by the rows they span, so each row looks at its own edges only.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let mut by_row: Vec<Vec<usize>> = vec![Vec::new(); self.rows as usize];
        let row_of = |y: i64| ((y - self.min.y) / self.cell).clamp(0, self.rows - 1);
        for (id, &(a, b)) in edges.iter().enumerate() {
            for row in row_of(a.y.min(b.y))..=row_of(a.y.max(b.y)) {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                by_row[row as usize].push(id);
            }
        }
        for (row, ids) in by_row.iter().enumerate() {
            let near: Vec<(Point64, Point64)> = ids.iter().map(|&id| edges[id]).collect();
            #[allow(clippy::cast_possible_wrap)]
            self.fill_row(row as i64, &near);
        }
    }

    /// Marks every cell the segment `a b` passes through.
    fn mark_edge(&mut self, a: Point64, b: Point64) {
        let mut marked = Vec::new();
        self.segment_cells(a, b, |at| marked.push(at));
        for at in marked {
            self.cells[at] = Cell::Boundary;
        }
    }

    /// Calls `visit` with the index of every cell the segment `a b` passes through, row band by
    /// row band, rounded outward (a cell may come twice); cells beyond the map are clamped to it.
    pub(crate) fn segment_cells(&self, a: Point64, b: Point64, mut visit: impl FnMut(usize)) {
        let row_of = |y: i64| ((y - self.min.y) / self.cell).clamp(0, self.rows - 1);
        let column_of = |x: i64| ((x - self.min.x) / self.cell).clamp(0, self.columns - 1);
        let (top, bottom) = (row_of(a.y.min(b.y)), row_of(a.y.max(b.y)));
        for row in top..=bottom {
            let (lo, hi) = if a.y == b.y {
                (a.x.min(b.x), a.x.max(b.x))
            } else {
                let band = |y: i64| y.clamp(a.y.min(b.y), a.y.max(b.y));
                let y0 = band(self.min.y + row * self.cell);
                let y1 = band(self.min.y + (row + 1) * self.cell);
                // The x where the segment is at height `y`, rounded outward below.
                #[allow(clippy::cast_precision_loss)]
                let x_at = |y: i64| {
                    a.x as f64 + (y - a.y) as f64 / (b.y - a.y) as f64 * (b.x - a.x) as f64
                };
                let (xa, xb) = (x_at(y0), x_at(y1));
                #[allow(clippy::cast_possible_truncation)]
                let range = (xa.min(xb).floor() as i64, xa.max(xb).ceil() as i64);
                range
            };
            for column in column_of(lo)..=column_of(hi) {
                visit(self.index(column, row));
            }
        }
    }

    /// The number of cells in the map.
    pub(crate) fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The rows of the map and the height of one, grid units.
    pub(crate) const fn rows(&self) -> (i64, i64) {
        (self.rows, self.cell)
    }

    /// The cells the boundary crosses, and all cells of the map.
    pub(crate) fn boundary_cells(&self) -> (usize, usize) {
        let boundary = self.cells.iter().filter(|c| **c == Cell::Boundary).count();
        (boundary, self.cells.len())
    }

    /// Whether the boundary crosses cell `at`.
    pub(crate) fn is_boundary(&self, at: usize) -> bool {
        self.cells[at] == Cell::Boundary
    }

    /// The column and row of the cell holding the point `doubled / 2`; `None` off the map.
    pub(crate) fn cell_of(&self, doubled: Point64) -> Option<(i64, i64)> {
        let (x, y) = (doubled.x - 2 * self.min.x, doubled.y - 2 * self.min.y);
        let size = 2 * self.cell;
        if x < 0 || y < 0 || x >= size * self.columns || y >= size * self.rows {
            return None;
        }
        Some((x / size, y / size))
    }

    /// The columns of the map and the left side of `column`, grid units.
    pub(crate) const fn column_left(&self, column: i64) -> i64 {
        self.min.x + column * self.cell
    }

    /// The columns of the map.
    pub(crate) const fn columns(&self) -> i64 {
        self.columns
    }

    /// The index of a cell in the map.
    pub(crate) fn cell_index(&self, column: i64, row: i64) -> usize {
        self.index(column, row)
    }

    /// The winding number all over cell `at`; `None` when the boundary crosses it.
    pub(crate) fn winding(&self, at: usize) -> Option<i32> {
        match self.cells[at] {
            Cell::Winding(w) => Some(w),
            Cell::Boundary => None,
        }
    }

    /// Gives the cells of `row` that the boundary misses the winding number at the row's centre
    /// line.
    fn fill_row(&mut self, row: i64, edges: &[(Point64, Point64)]) {
        #[allow(clippy::cast_precision_loss)]
        let y = (self.min.y + row * self.cell) as f64 + self.cell as f64 / 2.0;
        let mut crossings: Vec<(f64, i32)> = edges
            .iter()
            .filter_map(|&(a, b)| {
                #[allow(clippy::cast_precision_loss)]
                let (ay, by) = (a.y as f64, b.y as f64);
                let up = ay <= y && y < by;
                let down = by <= y && y < ay;
                if !(up || down) {
                    return None;
                }
                #[allow(clippy::cast_precision_loss)]
                let x = a.x as f64 + (y - ay) / (by - ay) * (b.x - a.x) as f64;
                Some((x, if up { 1 } else { -1 }))
            })
            .collect();
        crossings.sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut next = 0;
        let mut winding = 0;
        for column in 0..self.columns {
            #[allow(clippy::cast_precision_loss)]
            let x = (self.min.x + column * self.cell) as f64 + self.cell as f64 / 2.0;
            while next < crossings.len() && crossings[next].0 < x {
                winding += crossings[next].1;
                next += 1;
            }
            let at = self.index(column, row);
            if self.cells[at] != Cell::Boundary {
                self.cells[at] = Cell::Winding(winding);
            }
        }
    }

    /// Where the box from `lo` to `hi` (grid units) lies against the region.
    pub(crate) fn place(&self, lo: Point64, hi: Point64) -> Place {
        if hi.x < self.min.x || hi.y < self.min.y || lo.x > self.max.x || lo.y > self.max.y {
            return Place::Outside;
        }
        if lo.x < self.min.x || lo.y < self.min.y || hi.x > self.max.x || hi.y > self.max.y {
            return Place::Boundary;
        }
        let column = |x: i64| (x - self.min.x) / self.cell;
        let row = |y: i64| (y - self.min.y) / self.cell;
        let (mut inside, mut outside) = (false, false);
        for r in row(lo.y)..=row(hi.y) {
            for c in column(lo.x)..=column(hi.x) {
                match self.cells[self.index(c, r)] {
                    Cell::Boundary => return Place::Boundary,
                    Cell::Winding(0) => outside = true,
                    Cell::Winding(_) => inside = true,
                }
                if inside && outside {
                    return Place::Boundary;
                }
            }
        }
        if inside {
            Place::Inside
        } else {
            Place::Outside
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutlineTriple;
    use curvyo_document_core::Vec2;

    fn square(x: f64, y: f64, side: f64, reversed: bool) -> Vec<OutlineTriple> {
        let mut corners = vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
        if reversed {
            corners.reverse();
        }
        corners
            .into_iter()
            .map(|(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
            .collect()
    }

    fn at(x: f64, y: f64) -> Point64 {
        snap(Point::new(x, y))
    }

    #[test]
    fn cells_tell_inside_outside_and_the_hole() {
        let outer = square(0.0, 0.0, 100.0, false);
        let hole = square(40.0, 40.0, 20.0, true);
        let outlines = [Outline::new(&outer, true), Outline::new(&hole, true)];
        let region = Region::new(&outlines, 0.0, 0.008).unwrap();
        assert_eq!(region.place(at(10.0, 10.0), at(12.0, 12.0)), Place::Inside);
        assert_eq!(region.place(at(45.0, 45.0), at(47.0, 47.0)), Place::Outside);
        assert_eq!(
            region.place(at(38.0, 45.0), at(42.0, 47.0)),
            Place::Boundary
        );
        assert_eq!(region.place(at(-5.0, 10.0), at(1.0, 12.0)), Place::Boundary);
        assert_eq!(
            region.place(at(200.0, 10.0), at(210.0, 12.0)),
            Place::Outside
        );
    }

    #[test]
    fn a_margin_insets_the_outline_and_grows_the_hole() {
        let outer = square(0.0, 0.0, 100.0, false);
        let hole = square(40.0, 40.0, 20.0, true);
        let outlines = [Outline::new(&outer, true), Outline::new(&hole, true)];
        let region = Region::new(&outlines, 5.0, 0.008).unwrap();
        let (lo, hi) = region.bounds_mm();
        assert!((lo.x - 5.0).abs() < 1e-9 && (hi.y - 95.0).abs() < 1e-9);
        assert_eq!(region.place(at(36.0, 50.0), at(37.0, 51.0)), Place::Outside);
        assert!(Region::new(&outlines[..1], 60.0, 0.008).is_none());
    }
}
