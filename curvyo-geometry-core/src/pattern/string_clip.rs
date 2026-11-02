//! Cutting open lines at the region (`specs/0071-pattern-fill` criteria 13 to 15): each line on
//! its own, so lines that cross never join (acceptance report finding 3), in time linear in the
//! line's length and the boundary edges in the cells it passes. The parts inside the region or on its boundary
//! are kept (nonzero rule), on the 0.001 mm grid with exact integer predicates.

use curvyo_document_core::Point;

use crate::boolean_grid::{Path64, Point64};

use super::region::Region;

/// A region edge, `from` to `to`, grid units.
type Edge = (Point64, Point64);

/// The region's edges indexed by the cells and the rows of its map.
pub(crate) struct StringClipper<'r> {
    region: &'r Region,
    edges: Vec<Edge>,
    /// Edge ids per boundary cell: the ids of cell `c` are `cell_items[cell_start[c]..cell_start[c + 1]]`.
    cell_start: Vec<usize>,
    cell_items: Vec<u32>,
    /// Edges tested so far: the work of the clipping.
    pub(crate) tests: u64,
    /// The last segment that tested each edge, so an edge in several cells is tested once.
    stamp: Vec<u32>,
    query: u32,
}

/// Where a line meets the boundary: segment, parameter on it, and the point on the grid.
#[derive(Clone, Copy)]
struct Split {
    segment: usize,
    t: f64,
    point: Point64,
}

/// `(b - a) x (c - a)`, exact.
fn cross(a: Point64, b: Point64, c: Point64) -> i128 {
    i128::from(b.x - a.x) * i128::from(c.y - a.y) - i128::from(b.y - a.y) * i128::from(c.x - a.x)
}

/// `numerator / denominator` rounded to the nearest integer, halves away from zero.
pub(super) fn round_div(numerator: i128, denominator: i128) -> i128 {
    let (n, d) = if denominator < 0 {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    };
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

fn to_i64(value: i128) -> i64 {
    // invariant: the value lies between two grid points, which are i64.
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

/// The point `a + (b - a) x numerator / denominator` on the grid.
fn point_at(a: Point64, b: Point64, numerator: i128, denominator: i128) -> Point64 {
    let x = i128::from(a.x) + round_div(i128::from(b.x - a.x) * numerator, denominator);
    let y = i128::from(a.y) + round_div(i128::from(b.y - a.y) * numerator, denominator);
    Point64::new(to_i64(x), to_i64(y))
}

#[allow(clippy::cast_precision_loss)] // only orders the splits along one segment
fn ratio(numerator: i128, denominator: i128) -> f64 {
    numerator as f64 / denominator as f64
}

/// The points where segment `from to` meets `edge`, as `(t, point)` along the segment.
fn meet(from: Point64, to: Point64, edge: Edge, out: &mut Vec<(f64, Point64)>) {
    let (start, end) = edge;
    let wide = |v: i64| i128::from(v);
    let along = (wide(to.x - from.x), wide(to.y - from.y));
    let side = (wide(end.x - start.x), wide(end.y - start.y));
    let gap = (wide(start.x - from.x), wide(start.y - from.y));
    let denominator = along.0 * side.1 - along.1 * side.0;
    let off_line = gap.0 * along.1 - gap.1 * along.0;
    if denominator == 0 {
        if off_line != 0 {
            return; // parallel, apart
        }
        // Collinear: the overlap's ends.
        let length2 = along.0 * along.0 + along.1 * along.1;
        if length2 == 0 {
            return;
        }
        let project = |c: Point64| wide(c.x - from.x) * along.0 + wide(c.y - from.y) * along.1;
        let (ts, te) = (project(start), project(end));
        let (lo, hi) = (ts.min(te).max(0), ts.max(te).min(length2));
        if lo <= hi {
            out.push((ratio(lo, length2), point_at(from, to, lo, length2)));
            out.push((ratio(hi, length2), point_at(from, to, hi, length2)));
        }
        return;
    }
    let on_segment = gap.0 * side.1 - gap.1 * side.0;
    let within = |n: i128| {
        if denominator > 0 {
            (0..=denominator).contains(&n)
        } else {
            (denominator..=0).contains(&n)
        }
    };
    if within(on_segment) && within(off_line) {
        out.push((
            ratio(on_segment, denominator),
            point_at(from, to, on_segment, denominator),
        ));
    }
}

impl<'r> StringClipper<'r> {
    /// The index of `region`'s edges.
    pub(crate) fn new(region: &'r Region) -> Self {
        let edges: Vec<Edge> = region
            .contours
            .iter()
            .flat_map(|c| (0..c.len()).map(move |i| (c[i], c[(i + 1) % c.len()])))
            .collect();
        let mut cells_of: Vec<Vec<usize>> = Vec::with_capacity(edges.len());
        for &(a, b) in &edges {
            let mut cells = Vec::new();
            region.segment_cells(a, b, |at| cells.push(at));
            cells.sort_unstable();
            cells.dedup();
            cells_of.push(cells);
        }
        let (cell_start, cell_items) = bucket(region.cell_count(), &cells_of);
        Self {
            region,
            stamp: vec![u32::MAX; edges.len()],
            edges,
            cell_start,
            cell_items,
            tests: 0,
            query: 0,
        }
    }

    /// The parts of a line inside the region or on its boundary, each at least two points, in the
    /// line's order. `grid` is the line on the grid (no two consecutive points equal) and `exact`
    /// the same points before snapping: the inside is decided on the grid, the parts are given on
    /// `exact`, cut where the grid line meets the boundary, so an uncut stretch keeps its exact
    /// points and direction.
    pub(crate) fn clip(&mut self, grid: &Path64, exact: &[Point]) -> Vec<Vec<Point>> {
        if grid.len() < 2 || exact.len() != grid.len() {
            return Vec::new();
        }
        let splits = self.splits(grid);
        let mut parts = Parts::default();
        let mut run = Run::start(grid[0], exact[0], false);
        let mut next = 0;
        for segment in 0..grid.len() - 1 {
            let (a, b) = (exact[segment], exact[segment + 1]);
            while next < splits.len() && splits[next].segment == segment {
                let split = splits[next];
                next += 1;
                let at = Point::new(a.x + (b.x - a.x) * split.t, a.y + (b.y - a.y) * split.t);
                run.push(split.point, at);
                let kept = run.grid.len() >= 2 && self.kept(&run.grid);
                parts.close(
                    std::mem::replace(
                        &mut run,
                        Run::start(split.point, at, split.t > 0.0 && split.t < 1.0),
                    ),
                    kept,
                );
            }
            run.push(grid[segment + 1], b);
        }
        let kept = run.grid.len() >= 2 && self.kept(&run.grid);
        parts.close(run, kept);
        parts.finish()
    }

    /// Whether a run between two splits lies inside or on the boundary, by the middle of its
    /// longest segment.
    fn kept(&mut self, run: &Path64) -> bool {
        let longest = run
            .windows(2)
            .max_by_key(|w| {
                let (dx, dy) = (i128::from(w[1].x - w[0].x), i128::from(w[1].y - w[0].y));
                dx * dx + dy * dy
            })
            .map_or((run[0], run[0]), |w| (w[0], w[1]));
        let doubled = Point64::new(longest.0.x + longest.1.x, longest.0.y + longest.1.y);
        self.inside_or_on(doubled)
    }

    /// Whether the point `doubled / 2` has a nonzero winding or lies on an edge: the winding of
    /// the first cell to its right that no edge crosses, plus the edges crossed on the way there.
    fn inside_or_on(&mut self, doubled: Point64) -> bool {
        let Some((column, row)) = self.region.cell_of(doubled) else {
            return false;
        };
        let first = self.region.cell_index(column, row);
        if let Some(winding) = self.region.winding(first) {
            return winding != 0;
        }
        self.query = self.query.wrapping_add(1);
        let query = self.query;
        let twice = |p: Point64| Point64::new(2 * p.x, 2 * p.y);
        let mut through = column;
        // The winding beyond the cells crossed, counted to the right (the cell map counts to the
        // left: the two are opposite, a horizontal line crosses as often up as down).
        let mut beyond = 0_i64;
        while through < self.region.columns() {
            let at = self.region.cell_index(through, row);
            if let Some(winding) = self.region.winding(at) {
                beyond = -i64::from(winding);
                break;
            }
            through += 1;
        }
        let stop = Point64::new(2 * self.region.column_left(through), doubled.y);
        let mut winding = beyond;
        for c in column..through {
            let at = self.region.cell_index(c, row);
            for &id in &self.cell_items[self.cell_start[at]..self.cell_start[at + 1]] {
                if self.stamp[id as usize] == query {
                    continue;
                }
                self.stamp[id as usize] = query;
                self.tests += 1;
                let (a, b) = self.edges[id as usize];
                let (a, b) = (twice(a), twice(b));
                let side = cross(a, b, doubled);
                if side == 0
                    && doubled.x >= a.x.min(b.x)
                    && doubled.x <= a.x.max(b.x)
                    && doubled.y >= a.y.min(b.y)
                    && doubled.y <= a.y.max(b.y)
                {
                    return true;
                }
                let beyond_stop = cross(a, b, stop);
                if a.y <= doubled.y {
                    if b.y > doubled.y
                        && side > 0
                        && (through == self.region.columns() || beyond_stop <= 0)
                    {
                        winding += 1;
                    }
                } else if b.y <= doubled.y
                    && side < 0
                    && (through == self.region.columns() || beyond_stop >= 0)
                {
                    winding -= 1;
                }
            }
        }
        winding != 0
    }

    /// Whether the grid point `point` lies inside the region or on its boundary.
    pub(crate) fn contains(&mut self, point: Point64) -> bool {
        self.inside_or_on(Point64::new(2 * point.x, 2 * point.y))
    }

    /// How often `path` meets the boundary: about how often a strip along it crosses it.
    pub(crate) fn crossings(&mut self, path: &Path64) -> u64 {
        self.splits(path).len() as u64
    }

    /// Every point where `string` meets an edge, in order along the string.
    fn splits(&mut self, string: &Path64) -> Vec<Split> {
        let mut splits = Vec::new();
        let mut found = Vec::new();
        let (lo, hi) = (self.region.min, self.region.max);
        for (segment, w) in string.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            if a.x.max(b.x) < lo.x
                || a.y.max(b.y) < lo.y
                || a.x.min(b.x) > hi.x
                || a.y.min(b.y) > hi.y
            {
                continue;
            }
            self.query = self.query.wrapping_add(1);
            let query = self.query;
            found.clear();
            let mut boundary_cells = Vec::new();
            self.region.segment_cells(a, b, |at| {
                if self.region.is_boundary(at) {
                    boundary_cells.push(at);
                }
            });
            for at in boundary_cells {
                for &id in &self.cell_items[self.cell_start[at]..self.cell_start[at + 1]] {
                    if self.stamp[id as usize] == query {
                        continue;
                    }
                    self.stamp[id as usize] = query;
                    self.tests += 1;
                    meet(a, b, self.edges[id as usize], &mut found);
                }
            }
            found.sort_by(|p, q| {
                p.0.total_cmp(&q.0)
                    .then(p.1.x.cmp(&q.1.x))
                    .then(p.1.y.cmp(&q.1.y))
            });
            found.dedup_by(|p, q| p.1 == q.1);
            splits.extend(found.iter().map(|&(t, point)| Split { segment, t, point }));
        }
        debug_assert!(
            splits
                .windows(2)
                .all(|w| (w[0].segment, w[0].t) <= (w[1].segment, w[1].t))
        );
        splits
    }
}

/// A stretch of a line from one split to the next, on the grid and exact.
struct Run {
    grid: Path64,
    exact: Vec<Point>,
    /// The run starts at a split inside a segment of the line, not at one of its points.
    mid_segment: bool,
}

impl Run {
    fn start(grid: Point64, exact: Point, mid_segment: bool) -> Self {
        Self {
            grid: vec![grid],
            exact: vec![exact],
            mid_segment,
        }
    }

    /// Appends a point unless it repeats the last one on the grid.
    fn push(&mut self, grid: Point64, exact: Point) {
        if self.grid.last() != Some(&grid) {
            self.grid.push(grid);
            self.exact.push(exact);
        }
    }
}

/// The kept parts of a line: consecutive kept runs joined into one part.
#[derive(Default)]
struct Parts {
    done: Vec<Vec<Point>>,
    current: Option<Vec<Point>>,
}

impl Parts {
    /// Ends `run`: joined to the current part when kept (a split inside a segment between the two
    /// is no point of the line and is dropped); a dropped run of nonzero length ends the part.
    fn close(&mut self, run: Run, kept: bool) {
        if run.grid.len() < 2 {
            return; // zero length on the grid: changes nothing
        }
        if kept {
            match &mut self.current {
                Some(part) => {
                    if run.mid_segment && part.len() > 1 {
                        part.pop();
                    }
                    part.extend(run.exact.into_iter().skip(1));
                }
                None => self.current = Some(run.exact),
            }
        } else if let Some(part) = self.current.take() {
            self.done.push(part);
        }
    }

    fn finish(mut self) -> Vec<Vec<Point>> {
        self.done.extend(self.current.take());
        self.done
    }
}

/// Compressed lists: for each of `buckets` buckets the ids of the items listing it.
fn bucket(buckets: usize, lists: &[Vec<usize>]) -> (Vec<usize>, Vec<u32>) {
    let mut start = vec![0_usize; buckets + 1];
    for list in lists {
        for &b in list {
            start[b + 1] += 1;
        }
    }
    for i in 0..buckets {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let mut items = vec![0_u32; start[buckets]];
    for (id, list) in lists.iter().enumerate() {
        for &b in list {
            items[fill[b]] = u32::try_from(id).unwrap_or(u32::MAX);
            fill[b] += 1;
        }
    }
    (start, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutlineTriple;
    use crate::boolean::Outline;
    use crate::boolean_grid::snap;
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

    #[allow(clippy::cast_precision_loss)]
    fn p(x: i64, y: i64) -> Point {
        Point::new(x as f64, y as f64)
    }

    fn clip(clipper: &mut StringClipper<'_>, line: &[Point]) -> Vec<Vec<Point>> {
        let grid: Path64 = line.iter().map(|&q| snap(q)).collect();
        clipper.clip(&grid, line)
    }

    fn holed() -> Region {
        let outer = square(0.0, 0.0, 100.0, false);
        let hole = square(40.0, 40.0, 20.0, true);
        Region::new(
            &[Outline::new(&outer, true), Outline::new(&hole, true)],
            0.0,
            0.008,
        )
        .unwrap()
    }

    #[test]
    fn a_line_through_a_hole_gives_two_parts_ending_on_the_boundary() {
        let region = holed();
        let mut clipper = StringClipper::new(&region);
        let parts = clip(&mut clipper, &[p(-10, 50), p(110, 50)]);
        assert_eq!(
            parts,
            vec![vec![p(0, 50), p(40, 50)], vec![p(60, 50), p(100, 50)]]
        );
    }

    #[test]
    fn lines_on_the_boundary_are_kept_and_outside_dropped() {
        let region = holed();
        let mut clipper = StringClipper::new(&region);
        assert_eq!(
            clip(&mut clipper, &[p(-10, 0), p(110, 0)]),
            vec![vec![p(0, 0), p(100, 0)]]
        );
        assert_eq!(
            clip(&mut clipper, &[p(-10, -5), p(110, -5)]),
            Vec::<Vec<Point>>::new()
        );
        assert_eq!(
            clip(&mut clipper, &[p(45, 45), p(55, 55)]),
            Vec::<Vec<Point>>::new()
        );
    }

    #[test]
    fn a_bent_line_stays_one_part_while_inside() {
        let region = holed();
        let mut clipper = StringClipper::new(&region);
        let parts = clip(
            &mut clipper,
            &[p(-10, 10), p(20, 10), p(20, 30), p(110, 30)],
        );
        assert_eq!(
            parts,
            vec![vec![p(0, 10), p(20, 10), p(20, 30), p(100, 30)]]
        );
    }

    #[test]
    fn a_diagonal_through_corners_is_cut_at_them() {
        let region = holed();
        let mut clipper = StringClipper::new(&region);
        let parts = clip(&mut clipper, &[p(-10, -10), p(110, 110)]);
        assert_eq!(
            parts,
            vec![vec![p(0, 0), p(40, 40)], vec![p(60, 60), p(100, 100)]]
        );
    }
}
