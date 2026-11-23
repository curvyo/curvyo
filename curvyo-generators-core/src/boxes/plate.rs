//! One plate of the box: a rectangle with finger notches along its joint edges, built as a walk
//! round the four edges with a square at each corner, then offset by half the kerf.
//!
//! Every plate is the full rectangle of its box side; where two plates meet, each edge strip is
//! cut into segments that alternate between the two plates, and the corner cubes of the bottom
//! and the lid belong to the bottom and the lid.

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

use crate::outline::polygon;

/// How a corner square of side `thickness` is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Corner {
    /// The plate owns the square (the bottom and the lid); the edge strips stop short of it.
    Owned,
    /// A plain corner of an open edge: no square, the strips run to the rectangle's corner.
    Open,
    /// The square belongs to the neighbouring plate: cut out.
    Notched,
}

/// The notch pattern of one joint edge: a strip of `segments` equal pieces from `from` to `to`
/// along the edge, the pieces with index parity `solid_parity` kept and the others notched.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fingers {
    pub(crate) from: f64,
    pub(crate) to: f64,
    pub(crate) segments: u32,
    /// Whether pieces 0, 2, 4 ... are the plate's own (true) or the notches (false).
    pub(crate) even_solid: bool,
}

/// One edge: plain or a finger strip. The pattern runs in the physical direction of the edge
/// (increasing x along the top and bottom, increasing y along the left and right).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Edge {
    Plain,
    Fingered(Fingers),
}

/// A plate: size, the four corners (top left, top right, bottom right, bottom left) and the four
/// edges (top, right, bottom, left).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plate {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) corners: [Corner; 4],
    pub(crate) edges: [Edge; 4],
    /// The notch depth, the material thickness.
    pub(crate) depth: f64,
    /// Total play of a joint: every notch widens by a quarter of it on each side.
    pub(crate) clearance: f64,
}

/// A run of one edge: from `a` to `b` along it, cut `inset` into the plate.
type Run = (f64, f64, f64);

impl Plate {
    /// The runs of `edge` over `lo..hi` in the physical direction, merged where the inset is equal.
    fn runs(&self, edge: Edge, lo: f64, hi: f64) -> Vec<Run> {
        let Edge::Fingered(fingers) = edge else {
            return vec![(lo, hi, 0.0)];
        };
        let mut runs: Vec<Run> = vec![(lo, fingers.from, 0.0)];
        let step = (fingers.to - fingers.from) / f64::from(fingers.segments);
        let widen = self.clearance / 4.0;
        for index in 0..fingers.segments {
            let notched = (index % 2 == 0) != fingers.even_solid;
            let a = fingers.from + step * f64::from(index);
            let b = if index + 1 == fingers.segments {
                fingers.to
            } else {
                a + step
            };
            if notched {
                let a = if index == 0 { a } else { a - widen };
                let b = if index + 1 == fingers.segments {
                    b
                } else {
                    b + widen
                };
                runs.push((a, b, self.depth));
            } else {
                runs.push((a, b, 0.0));
            }
        }
        runs.push((fingers.to, hi, 0.0));
        // Neighbouring notches that widening made touch or overlap become one.
        let mut merged: Vec<Run> = Vec::new();
        for run in runs.into_iter().filter(|run| run.1 - run.0 > 1e-12) {
            match merged.last_mut() {
                Some(last) if (last.2 - run.2).abs() < 1e-12 => last.1 = run.1,
                _ => merged.push(run),
            }
        }
        // A kept piece between two widened notches never goes negative: it was filtered above.
        merged
    }

    fn corner_size(&self, corner: Corner) -> f64 {
        match corner {
            Corner::Owned | Corner::Notched => self.depth,
            Corner::Open => 0.0,
        }
    }

    /// The closed outline, clockwise on screen, with the top-left corner of the rectangle at the
    /// origin, before the kerf offset.
    fn polygon(&self) -> Vec<Point> {
        let (w, h) = (self.width, self.height);
        let size = self.corners.map(|corner| self.corner_size(corner));
        let corner_at = [
            Point::new(0.0, 0.0),
            Point::new(w, 0.0),
            Point::new(w, h),
            Point::new(0.0, h),
        ];
        // Inward unit vectors of each edge: top (0,1), right (-1,0), bottom (0,-1), left (1,0).
        let inward = [(0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (1.0, 0.0)];
        // The corners at the physical start and end of each edge (increasing x or y).
        let ends = [(0, 1), (1, 2), (3, 2), (0, 3)];
        let lengths = [w, h, w, h];
        // Per edge: the runs in walking order and the coordinates where the walk starts and ends.
        let walks: Vec<(Vec<Run>, f64, f64)> = (0..4)
            .map(|edge| {
                let lo = size[ends[edge].0];
                let hi = lengths[edge] - size[ends[edge].1];
                let runs = self.runs(self.edges[edge], lo, hi);
                if edge < 2 {
                    (runs, lo, hi)
                } else {
                    let reversed = runs.into_iter().rev().map(|(a, b, i)| (b, a, i)).collect();
                    (reversed, hi, lo)
                }
            })
            .collect();
        let mut points: Vec<Point> = Vec::new();
        for edge in 0..4 {
            let prev = (edge + 3) % 4;
            let at = corner_at[edge];
            match self.corners[edge] {
                Corner::Notched => points.push(Point::new(
                    at.x + (inward[prev].0 + inward[edge].0) * self.depth,
                    at.y + (inward[prev].1 + inward[edge].1) * self.depth,
                )),
                Corner::Owned | Corner::Open => {
                    points.push(edge_point(prev, walks[prev].2, 0.0, w, h));
                    points.push(at);
                    points.push(edge_point(edge, walks[edge].1, 0.0, w, h));
                }
            }
            for &(from, to, inset) in &walks[edge].0 {
                points.push(edge_point(edge, from, inset, w, h));
                points.push(edge_point(edge, to, inset, w, h));
            }
        }
        simplify(points)
    }

    /// The outline with every edge moved outward by half the kerf (all corners are right angles).
    pub(crate) fn outline(&self, kerf: f64) -> ClosedPath {
        let points = self.polygon();
        polygon(&offset(&points, kerf / 2.0))
    }

    /// The along-edge coordinates where the outline steps in or out along the edge `edge`, for
    /// tests of mating plates.
    #[cfg(test)]
    pub(crate) fn steps(&self, edge: usize) -> Vec<f64> {
        let size = self.corners.map(|corner| self.corner_size(corner));
        let length = if edge.is_multiple_of(2) {
            self.width
        } else {
            self.height
        };
        let ends = [(0, 1), (1, 2), (3, 2), (0, 3)];
        let runs = self.runs(
            self.edges[edge],
            size[ends[edge].0],
            length - size[ends[edge].1],
        );
        let mut steps = Vec::new();
        for pair in runs.windows(2) {
            steps.push(pair[0].1);
        }
        steps
    }
}

/// The point at `along` on `edge` cut `inset` into the plate.
fn edge_point(edge: usize, along: f64, inset: f64, w: f64, h: f64) -> Point {
    match edge {
        0 => Point::new(along, inset),
        1 => Point::new(w - inset, along),
        2 => Point::new(along, h - inset),
        _ => Point::new(inset, along),
    }
}

/// Drops repeated points and the middle of every straight run.
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut unique: Vec<Point> = Vec::new();
    for point in points {
        if unique
            .last()
            .is_none_or(|last| (last.x - point.x).abs() > 1e-9 || (last.y - point.y).abs() > 1e-9)
        {
            unique.push(point);
        }
    }
    if unique.len() > 1 {
        let (first, last) = (unique[0], unique[unique.len() - 1]);
        if (first.x - last.x).abs() <= 1e-9 && (first.y - last.y).abs() <= 1e-9 {
            unique.pop();
        }
    }
    let n = unique.len();
    let keep: Vec<Point> = (0..n)
        .filter(|&i| {
            let (a, b, c) = (unique[(i + n - 1) % n], unique[i], unique[(i + 1) % n]);
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            cross.abs() > 1e-9
        })
        .map(|i| unique[i])
        .collect();
    keep
}

/// The polygon with every edge moved outward by `distance`: each vertex moves along the sum of
/// the outward normals of its two edges (right angles only).
fn offset(points: &[Point], distance: f64) -> Vec<Point> {
    if distance == 0.0 || points.len() < 3 {
        return points.to_vec();
    }
    let n = points.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    // Y grows downward on screen; `area > 0` is clockwise there, so the outward normal of an edge
    // with direction (dx, dy) is (dy, -dx); the other way round for a negative area.
    let sign = if area > 0.0 { 1.0 } else { -1.0 };
    let normal = |a: Point, b: Point| {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = libm::hypot(dx, dy);
        (sign * dy / length, -sign * dx / length)
    };
    (0..n)
        .map(|i| {
            let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            let (n1, n2) = (normal(a, b), normal(b, c));
            Point::new(
                b.x + (n1.0 + n2.0) * distance,
                b.y + (n1.1 + n2.1) * distance,
            )
        })
        .collect()
}
