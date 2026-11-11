//! Turns a chain of centre-line samples with half widths into positive polygons: one strip ring
//! per run of steps whose two sides both advance, one ring per side of each folding run, a join
//! at each corner and a cap at each open end (ADR 0053, amendment of 2026-11-10, item 2).

use kurbo::{Point, Vec2};

use crate::offset_pieces::miter_within_limit;
use curvyo_document_core::{LineCap, LineJoin};

/// Distances below this, in millimetres, count as one point.
const COINCIDENT_MM: f64 = 1e-9;

/// A turn smaller than this, in radians, is no corner.
const CORNER_RADIANS: f64 = 1e-3;

/// One node of a chain: a centre-line point, its unit direction and half width.
#[derive(Debug, Clone, Copy)]
pub(super) struct Node {
    /// The point on the centre line.
    pub point: Point,
    /// The unit direction of travel.
    pub tangent: Vec2,
    /// Half the stroke width there.
    pub half: f64,
    /// The node begins a new segment at the point of the node before it.
    pub junction: bool,
}

impl Node {
    fn left(&self) -> Point {
        self.point + Vec2::new(-self.tangent.y, self.tangent.x) * self.half
    }

    fn right(&self) -> Point {
        self.point + Vec2::new(self.tangent.y, -self.tangent.x) * self.half
    }
}

/// The style a chain is built with.
#[derive(Debug, Clone, Copy)]
pub(super) struct Shape {
    /// The join at corners.
    pub join: LineJoin,
    /// The cap at open ends.
    pub cap: LineCap,
    /// The largest sagitta of a flattened arc, millimetres.
    pub sagitta: f64,
}

/// The polygons of a chain: simple positive rings, and rings that may cross themselves (the side
/// of a run that folds back), which the caller fills on their own with the nonzero rule.
#[derive(Debug, Default)]
pub(super) struct Polygons {
    /// Positive rings.
    pub simple: Vec<Vec<Point>>,
    /// Rings to normalise one by one before they join the union.
    pub folded: Vec<Vec<Point>>,
}

/// The polygons of `chain`, capped at both ends when `capped`, appended to `out`: a strip ring
/// per run of steps whose sides both advance; for a run where a side folds back, the half ring
/// of each side (centre line forward, side back), the folding one filled on its own.
pub(super) fn chain_polygons(chain: &[Node], capped: bool, shape: Shape, out: &mut Polygons) {
    let (Some(first), Some(last)) = (chain.first(), chain.last()) else {
        return;
    };
    if capped {
        cap(first, -first.tangent, shape, &mut out.simple);
        cap(last, last.tangent, shape, &mut out.simple);
    }
    let mut run = Run::default();
    run.nodes.push(*first);
    for pair in chain.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b.junction {
            if is_corner(a, b, shape.sagitta) {
                run.flush(out);
                join(a, b, shape, &mut out.simple);
                run.nodes.push(b);
            } else {
                run.nodes.push(b);
            }
            continue;
        }
        let (left, right) = folds(a, b);
        let folding = left || right;
        if folding != run.folding && run.nodes.len() > 1 {
            run.flush(out);
            run.nodes.push(a);
        }
        run.folding = folding;
        run.left |= left;
        run.right |= right;
        run.nodes.push(b);
    }
    run.flush(out);
}

/// Consecutive nodes whose steps all fold or all advance.
#[derive(Debug, Default)]
struct Run {
    nodes: Vec<Node>,
    folding: bool,
    left: bool,
    right: bool,
}

impl Run {
    /// Writes the run's rings and starts an empty one.
    fn flush(&mut self, out: &mut Polygons) {
        if self.nodes.len() >= 2 {
            if self.folding {
                for (side, folded) in [
                    (Node::left as fn(&Node) -> Point, self.left),
                    (Node::right, self.right),
                ] {
                    let mut ring: Vec<Point> = self.nodes.iter().map(|n| n.point).collect();
                    ring.extend(self.nodes.iter().rev().map(side));
                    if folded {
                        out.folded.push(ring);
                    } else {
                        if area(&ring) < 0.0 {
                            ring.reverse();
                        }
                        out.simple.push(ring);
                    }
                }
            } else {
                // The centre points at both ends keep the ring's end edges identical to those of
                // the neighbouring half rings, so the union leaves no crack between them.
                let mut ring: Vec<Point> = self.nodes.iter().map(Node::right).collect();
                ring.extend(self.nodes.last().map(|n| n.point));
                ring.extend(self.nodes.iter().rev().map(Node::left));
                ring.extend(self.nodes.first().map(|n| n.point));
                out.simple.push(ring);
            }
        }
        *self = Self::default();
    }
}

/// Twice the signed area of `ring`.
fn area(ring: &[Point]) -> f64 {
    (0..ring.len())
        .map(|i| {
            ring[i]
                .to_vec2()
                .cross(ring[(i + 1) % ring.len()].to_vec2())
        })
        .sum()
}

/// A dot of a zero-length dash at `node`.
pub(super) fn dot(node: &Node, shape: Shape, out: &mut Polygons) {
    let out = &mut out.simple;
    if node.half <= 0.0 {
        return;
    }
    match shape.cap {
        LineCap::Butt => {}
        LineCap::Round => out.push(disc(node.point, node.half, shape.sagitta)),
        LineCap::Square => {
            let (t, n) = (
                node.tangent * node.half,
                Vec2::new(-node.tangent.y, node.tangent.x) * node.half,
            );
            hull(
                &[
                    node.point - t - n,
                    node.point + t - n,
                    node.point + t + n,
                    node.point - t + n,
                ],
                out,
            );
        }
    }
}

/// Whether the tangents at a junction differ enough to need a join.
fn is_corner(a: Node, b: Node, sagitta: f64) -> bool {
    let angle = a
        .tangent
        .cross(b.tangent)
        .atan2(a.tangent.dot(b.tangent))
        .abs();
    angle > CORNER_RADIANS || a.half.max(b.half) * angle > sagitta
}

/// Whether the left and the right side of the step from `a` to `b` move against the centre line.
fn folds(a: Node, b: Node) -> (bool, bool) {
    let step = b.point - a.point;
    if step.hypot() <= COINCIDENT_MM {
        return (false, false);
    }
    (
        (b.left() - a.left()).dot(step) < 0.0,
        (b.right() - a.right()).dot(step) < 0.0,
    )
}

/// The join between the end of one segment (`a`) and the start of the next (`b`).
fn join(a: Node, b: Node, shape: Shape, out: &mut Vec<Vec<Point>>) {
    let half = a.half.max(b.half);
    if half <= 0.0 {
        return;
    }
    let left_turn = a.tangent.cross(b.tangent) > 0.0;
    let outer = |n: &Node| if left_turn { n.right() } else { n.left() };
    let outer_side = |t: Vec2| {
        if left_turn {
            Vec2::new(t.y, -t.x)
        } else {
            Vec2::new(-t.y, t.x)
        }
    };
    let corner = a.point;
    let (from, to) = (outer(&Node { half, ..a }), outer(&Node { half, ..b }));
    match shape.join {
        LineJoin::Round => out.push(disc(corner, half, shape.sagitta)),
        LineJoin::Miter if miter_within_limit(a.tangent.dot(b.tangent)) => {
            let bisector = outer_side(a.tangent) + outer_side(b.tangent);
            let cos_half = f64::midpoint(1.0, a.tangent.dot(b.tangent)).sqrt();
            if bisector.hypot() > COINCIDENT_MM && cos_half > 0.0 {
                let tip = corner + bisector / bisector.hypot() * (half / cos_half);
                hull(&[corner, from, tip, to], out);
            } else {
                hull(&[corner, from, to], out);
            }
        }
        LineJoin::Miter | LineJoin::Bevel => hull(&[corner, from, to], out),
    }
}

/// The cap at `node`, reaching out in the unit direction `outward`.
fn cap(node: &Node, outward: Vec2, shape: Shape, out: &mut Vec<Vec<Point>>) {
    if node.half <= 0.0 {
        return;
    }
    match shape.cap {
        LineCap::Butt => {}
        LineCap::Round => out.push(disc(node.point, node.half, shape.sagitta)),
        LineCap::Square => {
            let reach = outward * node.half;
            hull(
                &[
                    node.left(),
                    node.left() + reach,
                    node.right() + reach,
                    node.right(),
                ],
                out,
            );
        }
    }
}

/// A disc of `radius` around `centre`, flattened within `sagitta`, positive.
fn disc(centre: Point, radius: f64, sagitta: f64) -> Vec<Point> {
    let ratio = (1.0 - sagitta / radius).clamp(-1.0, 1.0);
    let steps = (std::f64::consts::PI / ratio.acos())
        .ceil()
        .clamp(8.0, 4096.0);
    // invariant: clamped to 8..=4096 above, so the cast is exact.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = steps as u32;
    (0..count)
        .map(|i| {
            let angle = std::f64::consts::TAU * f64::from(i) / f64::from(count);
            centre + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

/// The convex hull of `points`, positive, pushed when it has more than two corners.
fn hull(points: &[Point], out: &mut Vec<Vec<Point>>) {
    let mut sorted: Vec<Point> = points.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    sorted.dedup_by(|a, b| (*a - *b).hypot() <= COINCIDENT_MM);
    if sorted.len() < 3 {
        return;
    }
    let cross = |o: Point, a: Point, b: Point| (a - o).cross(b - o);
    let mut lower: Vec<Point> = Vec::new();
    for &p in &sorted {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point> = Vec::new();
    for &p in sorted.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    if lower.len() >= 3 {
        out.push(lower);
    }
}
