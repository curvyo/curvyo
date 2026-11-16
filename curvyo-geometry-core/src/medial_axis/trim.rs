//! The end trims of the raw axis graph (ADR 0063 §3): rounded ends are trimmed to their end
//! circle's centre, and runs that climb into a jaggy or a flat end's corner are cut back to the
//! stroke's own width.

use kurbo::Point;

use super::AxisSample;
use super::boundary::Boundary;
use super::graph::Graph;

/// How much a leaf's disc may stick out of another node's disc and still count as inside it, in
/// millimetres: the flattening tolerance (0.0125 mm) plus the search precision, with room to
/// spare.
const CONTAINED_SLACK_MM: f64 = 0.02;

/// A link whose interpolated room misses by less than this is searched with exact clearances:
/// at a rounded end's centre the clearance has a kink that interpolation cuts off.
const NEAR_MISS_MM: f64 = 0.05;

/// A settled end lies at least this share of the cut end's distance out from the widest node.
const SETTLE_REACH: f64 = 0.5;
/// The precision of a settled end's position and of the exact room search, in millimetres.
const SETTLE_PRECISION_MM: f64 = 0.001;

/// The golden ratio's inverse.
const INVERSE_PHI: f64 = 0.618_033_988_749_894_8;

/// Every node bucketed in small square cells, each with the largest clearance it holds, so a
/// search for a disc that holds a point skips the cells whose discs cannot reach it.
struct NodeGrid {
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    members: Vec<Vec<usize>>,
    widest_in: Vec<f64>,
    widest: f64,
}

impl NodeGrid {
    fn new(graph: &Graph) -> Self {
        let mut radii: Vec<f64> = graph.samples.iter().map(|s| s.radius).collect();
        radii.sort_by(f64::total_cmp);
        let median = radii.get(radii.len() / 2).copied().unwrap_or(0.0);
        let widest = radii.last().copied().unwrap_or(0.0);
        let (mut min, mut max) = (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for sample in &graph.samples {
            min = Point::new(min.x.min(sample.point.x), min.y.min(sample.point.y));
            max = Point::new(max.x.max(sample.point.x), max.y.max(sample.point.y));
        }
        let span = (max.x - min.x).max(max.y - min.y).max(1e-3);
        let cell = (2.0 * median)
            .max(widest / 4.0)
            .max(span / 1024.0)
            .max(1e-3);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (columns, rows) = (
            ((max.x - min.x) / cell).floor() as usize + 1,
            ((max.y - min.y) / cell).floor() as usize + 1,
        );
        let mut grid = Self {
            origin: min,
            cell,
            columns,
            rows,
            members: vec![Vec::new(); columns * rows],
            widest_in: vec![0.0; columns * rows],
            widest,
        };
        for (n, sample) in graph.samples.iter().enumerate() {
            let (c, r) = grid.key(sample.point);
            let at = r * grid.columns + c;
            grid.members[at].push(n);
            grid.widest_in[at] = grid.widest_in[at].max(sample.radius);
        }
        grid
    }

    fn key(&self, point: Point) -> (usize, usize) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = |value: f64, count: usize| (value.max(0.0).floor() as usize).min(count - 1);
        (
            index((point.x - self.origin.x) / self.cell, self.columns),
            index((point.y - self.origin.y) / self.cell, self.rows),
        )
    }

    /// The nodes of every cell that may hold a disc around `point` reaching `radius` minus
    /// `slack` beyond it.
    fn candidates(&self, point: Point, radius: f64, slack: f64) -> Vec<usize> {
        let (column, row) = self.key(point);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let reach = ((self.widest + slack) / self.cell).ceil() as usize + 1;
        let mut found = Vec::new();
        for r in row.saturating_sub(reach)..=(row + reach).min(self.rows - 1) {
            for c in column.saturating_sub(reach)..=(column + reach).min(self.columns - 1) {
                let ring = r.abs_diff(row).max(c.abs_diff(column));
                #[allow(clippy::cast_precision_loss)]
                let nearest = ring.saturating_sub(1) as f64 * self.cell;
                let at = r * self.columns + c;
                if self.widest_in[at] + slack >= nearest + radius {
                    found.extend_from_slice(&self.members[at]);
                }
            }
        }
        found
    }
}

/// Trims the ends (ADR 0063 §3, end trim): a leaf whose disc lies inside the disc of another
/// axis point is not on the medial axis and is dropped, and the new leaf behind it is tested in
/// turn. A rounded end then stops at the centre of its end circle, every sample in the cap
/// having a disc inside that circle; a tapering end keeps reaching its tip, since there every
/// disc sticks out of the wider ones.
pub(super) fn trim_ends(graph: &mut Graph, boundary: &Boundary) {
    let grid = NodeGrid::new(graph);
    let mut work: Vec<usize> = (0..graph.samples.len())
        .rev()
        .filter(|&n| graph.alive[n] && graph.degree(n) == 1)
        .collect();
    while let Some(leaf) = work.pop() {
        if !graph.alive[leaf] || graph.degree(leaf) != 1 || !contained(graph, boundary, &grid, leaf)
        {
            continue;
        }
        let neighbour = graph.neighbours(leaf).next();
        graph.alive[leaf] = false;
        if let Some(next) = neighbour.filter(|&n| graph.degree(n) == 1) {
            work.push(next);
        }
    }
}

/// Whether the disc of `leaf` lies inside the disc of another live node or of a point on a
/// link between live nodes (links at the leaf itself excluded).
fn contained(graph: &Graph, boundary: &Boundary, grid: &NodeGrid, leaf: usize) -> bool {
    let own = graph.samples[leaf];
    for node in grid.candidates(own.point, own.radius, NEAR_MISS_MM) {
        if node == leaf || !graph.alive[node] {
            continue;
        }
        let sample = graph.samples[node];
        let lack = own.radius - (sample.radius - (sample.point - own.point).hypot());
        if lack <= CONTAINED_SLACK_MM {
            return true;
        }
        if lack > NEAR_MISS_MM {
            continue;
        }
        for next in graph.neighbours(node) {
            if next == leaf {
                continue;
            }
            let other = graph.samples[next];
            let linear = room(own.point, sample, other);
            if linear + CONTAINED_SLACK_MM >= own.radius
                || exact_room(boundary, own.point, sample.point, other.point) + CONTAINED_SLACK_MM
                    >= own.radius
            {
                return true;
            }
        }
    }
    false
}

/// The largest `r(q) - |q - p|` over the link from `a` to `b`, position and radius interpolated
/// linearly: how wide a disc around `p` fits inside a disc of the link. The function is concave
/// along the link, so its maximum is where its derivative vanishes, clamped to the link.
#[allow(clippy::many_single_char_names)] // the textbook names of the derivation
fn room(p: Point, a: AxisSample, b: AxisSample) -> f64 {
    let d = b.point - a.point;
    let length = d.hypot();
    let value = |t: f64| a.radius + t * (b.radius - a.radius) - (a.point + d * t - p).hypot();
    if length == 0.0 {
        return value(0.0);
    }
    let u = d / length;
    let slope = (b.radius - a.radius) / length;
    let along = (a.point - p).dot(u);
    let across = (a.point - p).cross(u).abs();
    let t = if slope.abs() < 1.0 {
        (slope * across / (1.0 - slope * slope).sqrt() - along) / length
    } else if slope > 0.0 {
        1.0
    } else {
        0.0
    };
    value(t.clamp(0.0, 1.0))
}

/// The largest `clearance(q) - |q - p|` over the segment from `a` to `b`, by a golden-section
/// search to 0.001 mm.
fn exact_room(boundary: &Boundary, p: Point, a: Point, b: Point) -> f64 {
    let value = |t: f64| {
        let q = a.lerp(b, t);
        boundary.distance(q) - (q - p).hypot()
    };
    let length = (b - a).hypot();
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    while (high - low) * length > SETTLE_PRECISION_MM {
        let left = high - INVERSE_PHI * (high - low);
        let right = low + INVERSE_PHI * (high - low);
        if value(left) < value(right) {
            low = left;
        } else {
            high = right;
        }
    }
    value(f64::midpoint(low, high))
        .max(value(0.0))
        .max(value(1.0))
}

/// The clearance growth per millimetre of axis above which an end run leads into a jaggy, not
/// along the stroke: a run straight into a jaggy grows by about 1, into the corner of a flat end
/// by 0.71, along a 60° point by 0.5.
const STEEP_END: f64 = 0.65;

/// Drops the end runs that climb from the outline into the stroke faster than [`STEEP_END`]:
/// spurs into a jaggy that the end trim left as a chain's end once its cap was gone
/// (`specs/0117-centre-lines` criterion 10, a spur shorter than its width only shapes the end).
pub(super) fn trim_steep_ends(graph: &mut Graph, boundary: &Boundary) {
    let leaves: Vec<usize> = (0..graph.samples.len())
        .filter(|&n| graph.alive[n] && graph.degree(n) == 1)
        .collect();
    for leaf in leaves {
        if !graph.alive[leaf] || graph.degree(leaf) != 1 {
            continue;
        }
        let start = graph.samples[leaf];
        let (mut previous, mut current, mut length) = (usize::MAX, leaf, 0.0);
        let mut run = vec![leaf];
        let mut cut: Option<usize> = None;
        // A run from a leaf through degree-two nodes cannot come back on itself; this walk and
        // those of `settle_end` are bounded by the node count all the same.
        for _ in 0..graph.samples.len() {
            let Some(next) = graph.neighbours(current).find(|&n| n != previous) else {
                break;
            };
            length += (graph.samples[next].point - graph.samples[current].point).hypot();
            let sample = graph.samples[next];
            if length > 2.0 * sample.radius.max(start.radius) || graph.degree(next) != 2 {
                break;
            }
            if sample.radius - start.radius >= STEEP_END * length && length > 0.0 {
                cut = Some(run.len());
            }
            run.push(next);
            previous = current;
            current = next;
        }
        if let Some(cut) = cut {
            for &node in &run[..cut] {
                graph.alive[node] = false;
            }
            settle_end(graph, boundary, run[cut], run[cut - 1]);
        }
    }
}

/// Moves an end left on a corner's bisector by the steep trim to where the stroke's own disc
/// touches the end: from the widest node near the end, straight on along the stroke while the
/// clearance keeps that width. A flat end then ends `w/2` before its edge, as a rounded one does
/// (criterion 8 Proposal). `cut` is the node just trimmed off beyond `end`.
fn settle_end(graph: &mut Graph, boundary: &Boundary, end: usize, cut: usize) {
    // The nodes inward from the end, up to twice its clearance and through degree-two nodes.
    let reach = 2.0 * graph.samples[end].radius;
    let (mut previous, mut current, mut length) = (cut, end, 0.0);
    let mut inward = vec![end];
    for _ in 0..graph.samples.len() {
        let Some(next) = graph.neighbours(current).find(|&n| n != previous) else {
            break;
        };
        length += (graph.samples[next].point - graph.samples[current].point).hypot();
        inward.push(next);
        if length > reach || graph.degree(next) != 2 {
            break;
        }
        previous = current;
        current = next;
    }
    let Some(widest) = (1..inward.len()).max_by(|&a, &b| {
        graph.samples[inward[a]]
            .radius
            .total_cmp(&graph.samples[inward[b]].radius)
    }) else {
        return;
    };
    let stable = graph.samples[inward[widest]];
    if stable.radius <= graph.samples[end].radius {
        return;
    }
    // The stroke's direction at the widest node: from a node at least half a width further in.
    let mut behind = None;
    let (mut previous, mut current) = (inward[widest - 1], inward[widest]);
    for _ in 0..graph.samples.len() {
        let Some(next) = graph.neighbours(current).find(|&n| n != previous) else {
            break;
        };
        if (graph.samples[next].point - stable.point).hypot() >= 0.5 * stable.radius {
            behind = Some(graph.samples[next].point);
            break;
        }
        if graph.degree(next) != 2 {
            break;
        }
        previous = current;
        current = next;
    }
    let Some(behind) = behind else {
        return;
    };
    let direction = (stable.point - behind) / (stable.point - behind).hypot();
    let keeps = |t: f64| {
        boundary.distance(stable.point + direction * t) >= stable.radius - CONTAINED_SLACK_MM
    };
    let (mut low, mut high) = (
        0.0_f64,
        (graph.samples[end].point - stable.point).hypot() + stable.radius,
    );
    if keeps(high) {
        return;
    }
    while high - low > SETTLE_PRECISION_MM {
        let middle = f64::midpoint(low, high);
        if keeps(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    // A tapering end narrows all the way: there the stroke's disc leaves at once, and the end
    // stays where the trim cut it.
    if low < SETTLE_REACH * (graph.samples[end].point - stable.point).hypot() {
        return;
    }
    let point = stable.point + direction * low;
    for &node in &inward[1..widest] {
        graph.alive[node] = false;
    }
    graph.samples[end] = AxisSample {
        point,
        radius: boundary.distance(point),
    };
    let anchor = inward[widest];
    graph.links[end].retain(|&n| n == anchor);
    if !graph.links[end].contains(&anchor) {
        graph.links[end].push(anchor);
        graph.links[anchor].push(end);
    }
}
