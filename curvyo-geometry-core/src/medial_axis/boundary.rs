//! A uniform grid over a region's outline segments that answers the exact distance from a point
//! to the nearest segment.

use kurbo::Point;

/// At most this many cells per segment: a big traced ring of short segments would otherwise ask
/// for millions of cells.
const MOST_CELLS_PER_SEGMENT: f64 = 4.0;
/// The cell side as a multiple of the mean segment length.
const CELL_SCALE: f64 = 2.0;

/// The outline's segments, bucketed in square cells.
#[derive(Debug, Clone)]
pub(crate) struct Boundary {
    segments: Vec<(Point, Point)>,
    /// Ring `k` holds the segments `ring_starts[k]..ring_starts[k + 1]`.
    ring_starts: Vec<usize>,
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    /// Cell `c` holds `members[starts[c]..starts[c + 1]]`.
    starts: Vec<usize>,
    members: Vec<u32>,
}

impl Boundary {
    /// The index of the closed `rings`.
    pub(crate) fn new(rings: &[Vec<Point>]) -> Self {
        let mut segments = Vec::new();
        let mut ring_starts = vec![0];
        for ring in rings {
            for (i, &a) in ring.iter().enumerate() {
                segments.push((a, ring[(i + 1) % ring.len()]));
            }
            ring_starts.push(segments.len());
        }
        let (mut min, mut max) = (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for &(a, _) in &segments {
            min = Point::new(min.x.min(a.x), min.y.min(a.y));
            max = Point::new(max.x.max(a.x), max.y.max(a.y));
        }
        #[allow(clippy::cast_precision_loss)]
        let count = segments.len().max(1) as f64;
        let (width, height) = ((max.x - min.x).max(1e-3), (max.y - min.y).max(1e-3));
        let mean = segments.iter().map(|(a, b)| (*b - *a).hypot()).sum::<f64>() / count;
        // Cells about as long as the segments: line art fills little of its box, so a cell
        // sized by the box would hold a whole stretch of stroke on both sides.
        let cell = (mean * CELL_SCALE)
            .min((width * height / count).sqrt())
            .max((width * height / (MOST_CELLS_PER_SEGMENT * count)).sqrt())
            .max(1e-3);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (columns, rows) = (
            (width / cell).floor() as usize + 1,
            (height / cell).floor() as usize + 1,
        );
        let mut index = Self {
            segments,
            ring_starts,
            origin: min,
            cell,
            columns,
            rows,
            starts: Vec::new(),
            members: Vec::new(),
        };
        index.fill();
        index
    }

    /// Buckets every segment into the cells its bounding box overlaps: a counting pass, then
    /// one flat array.
    fn fill(&mut self) {
        let cells = self.columns * self.rows;
        let spans: Vec<((usize, usize), (usize, usize))> = self
            .segments
            .iter()
            .map(|&(a, b)| {
                (
                    self.cell_of(Point::new(a.x.min(b.x), a.y.min(b.y))),
                    self.cell_of(Point::new(a.x.max(b.x), a.y.max(b.y))),
                )
            })
            .collect();
        let mut counts = vec![0_usize; cells + 1];
        for &((c0, r0), (c1, r1)) in &spans {
            for row in r0..=r1 {
                for column in c0..=c1 {
                    counts[row * self.columns + column + 1] += 1;
                }
            }
        }
        for i in 1..counts.len() {
            counts[i] += counts[i - 1];
        }
        let mut next = counts.clone();
        self.members = vec![0; counts[cells]];
        for (i, &((c0, r0), (c1, r1))) in spans.iter().enumerate() {
            for row in r0..=r1 {
                for column in c0..=c1 {
                    let at = &mut next[row * self.columns + column];
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        self.members[*at] = i as u32;
                    }
                    *at += 1;
                }
            }
        }
        self.starts = counts;
    }

    /// The cell holding `point`, clamped to the grid.
    fn cell_of(&self, point: Point) -> (usize, usize) {
        let clamp = |value: f64, count: usize| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let index = value.max(0.0).floor() as usize;
            index.min(count - 1)
        };
        (
            clamp((point.x - self.origin.x) / self.cell, self.columns),
            clamp((point.y - self.origin.y) / self.cell, self.rows),
        )
    }

    /// The distance from `point` to the nearest outline segment.
    pub(super) fn distance(&self, point: Point) -> f64 {
        self.nearest_where(point, |_, _| true)
            .map_or(f64::INFINITY, |(d, _, _)| d)
    }

    /// The distance from `point` to the nearest segment of a ring other than `ring`.
    pub(super) fn distance_without(&self, point: Point, ring: usize) -> f64 {
        self.nearest_without(point, ring)
            .map_or(f64::INFINITY, |(d, _, _)| d)
    }

    /// The nearest point of a ring other than `ring`: its distance, the point and its ring.
    pub(super) fn nearest_without(&self, point: Point, ring: usize) -> Option<(f64, Point, usize)> {
        let skip = self.ring_starts[ring]..self.ring_starts[ring + 1];
        self.nearest_where(point, |segment, _| !skip.contains(&segment))
    }

    /// The nearest outline point to `point` that `accept` takes (given the segment index and
    /// the point): its distance, the point and its ring.
    pub(crate) fn nearest_where(
        &self,
        point: Point,
        accept: impl Fn(usize, Point) -> bool,
    ) -> Option<(f64, Point, usize)> {
        let (column, row) = self.cell_of(point);
        let mut best: Option<(f64, Point, usize)> = None;
        let reach = self.columns.max(self.rows);
        for ring in 0..=reach {
            self.visit_ring(column, row, ring, |index| {
                let foot = foot(point, self.segments[index]);
                let squared = (foot - point).hypot2();
                if best.is_none_or(|(b, _, _)| squared < b) && accept(index, foot) {
                    best = Some((squared, foot, index));
                }
            });
            #[allow(clippy::cast_precision_loss)]
            let covered = ring as f64 * self.cell;
            if best.is_some_and(|(b, _, _)| b <= covered * covered) {
                break;
            }
        }
        best.map(|(squared, foot, index)| {
            let ring = self.ring_starts.partition_point(|&start| start <= index) - 1;
            (squared.sqrt(), foot, ring)
        })
    }

    /// Calls `visit` for every segment of the cells at Chebyshev distance `ring` from a cell.
    fn visit_ring(&self, column: usize, row: usize, ring: usize, mut visit: impl FnMut(usize)) {
        let (c0, c1) = (
            column.saturating_sub(ring),
            (column + ring).min(self.columns - 1),
        );
        let (r0, r1) = (row.saturating_sub(ring), (row + ring).min(self.rows - 1));
        for r in r0..=r1 {
            let edge_row = r + ring == row || r == row + ring;
            for c in c0..=c1 {
                if !edge_row && c + ring != column && c != column + ring {
                    continue;
                }
                let cell = r * self.columns + c;
                for &member in &self.members[self.starts[cell]..self.starts[cell + 1]] {
                    visit(member as usize);
                }
            }
        }
    }
}

/// The point of the segment nearest to `point`.
fn foot(point: Point, (a, b): (Point, Point)) -> Point {
    let d = b - a;
    let length_squared = d.hypot2();
    let t = if length_squared > 0.0 {
        ((point - a).dot(d) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + d * t
}
