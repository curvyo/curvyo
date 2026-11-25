//! A curved outline as footprint shapes: lines and fitted arcs, kinks rounded by fillets, corners
//! kept sharp (ADR 0083 §3, §13.2, §13.3).

#![allow(clippy::many_single_char_names)]

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

use super::arc_fit::{arcs, cusps, cut};
use super::classify::{Join, JoinKind, classify, starts, tangent};
use super::shape::{Shapes, Vertex};
use crate::contour::{circle_circle, line_circle, line_line};
use crate::footprint::Shape;
use crate::outline::polar;
use crate::shape_ops::{between, end, length};

/// A shape shorter than this (mm) is dropped.
const ZERO_SHAPE: f64 = 0.01;
/// The smallest fillet radius tried (mm).
const SMALLEST_FILLET: f64 = 1e-3;

/// The outline with a node at every cusp, so a cusp is a join like any other.
pub(crate) fn split_cusps(path: &ClosedPath) -> ClosedPath {
    let from = starts(path);
    let mut segments = Vec::new();
    for (i, cubic) in path.segments.iter().enumerate() {
        let mut rest = *cubic;
        let mut origin = from[i];
        let mut done = 0.0;
        for t in cusps(from[i], cubic) {
            let local = (t - done) / (1.0 - done);
            let (head, tail) = cut(origin, &rest, local);
            segments.push(head);
            origin = head.to;
            rest = tail;
            done = t;
        }
        segments.push(rest);
    }
    ClosedPath {
        start: path.start,
        segments,
    }
}

/// A shape of the ring with the cut that follows it.
#[derive(Debug, Clone, Copy)]
struct Item {
    shape: Shape,
    /// The join at the end of the shape when it ends a cubic and is no tangent join.
    cut: Option<Join>,
}

/// A fillet between two shapes: the circle, and where each side is trimmed: the shape (an index
/// into its chain, nearest the kink first) and the distance to cut from its end (before the kink)
/// or from its start (after it).
struct Round {
    arc: Shape,
    before: (usize, f64),
    after: (usize, f64),
}

fn unit(a: (f64, f64)) -> (f64, f64) {
    let l = libm::hypot(a.0, a.1).max(1e-12);
    (a.0 / l, a.1 / l)
}

/// The shape moved `d` to the right of the walk (towards the inside of a clockwise outline).
fn moved(shape: &Shape, d: f64) -> Option<Shape> {
    match *shape {
        Shape::Straight { from, to } => {
            let (dx, dy) = unit((to.x - from.x, to.y - from.y));
            let (nx, ny) = (-dy * d, dx * d);
            Some(Shape::Straight {
                from: Point::new(from.x + nx, from.y + ny),
                to: Point::new(to.x + nx, to.y + ny),
            })
        }
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => {
            let r = radius - sweep.signum() * d;
            (r > 1e-9).then_some(Shape::Bend {
                centre,
                radius: r,
                from,
                sweep,
            })
        }
    }
}

fn carrier_hits(a: &Shape, b: &Shape) -> Vec<Point> {
    match (*a, *b) {
        (Shape::Straight { from, to }, Shape::Straight { from: f2, to: t2 }) => {
            line_line(from, to, f2, t2).into_iter().collect()
        }
        (Shape::Straight { from, to }, Shape::Bend { centre, radius, .. })
        | (Shape::Bend { centre, radius, .. }, Shape::Straight { from, to }) => {
            line_circle(from, to, centre, radius)
        }
        (
            Shape::Bend {
                centre: c1,
                radius: r1,
                ..
            },
            Shape::Bend {
                centre: c2,
                radius: r2,
                ..
            },
        ) => circle_circle(c1, r1, c2, r2),
    }
}

/// How far from the end of `shape` the circle centred `c` touches it, `None` when it touches
/// beyond the shape. `from_end` measures from the end, else from the start.
fn touch(shape: &Shape, c: Point, from_end: bool) -> Option<(Point, f64)> {
    let l = length(shape);
    match *shape {
        Shape::Straight { from, to } => {
            let (dx, dy) = unit((to.x - from.x, to.y - from.y));
            let along = (c.x - from.x) * dx + (c.y - from.y) * dy;
            if along < -1e-9 || along > l + 1e-9 {
                return None;
            }
            let point = Point::new(from.x + dx * along, from.y + dy * along);
            Some((point, if from_end { l - along } else { along }))
        }
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => {
            let angle = libm::atan2(c.y - centre.y, c.x - centre.x);
            // A concave fillet's centre lies beyond the circle's centre: the touch is opposite.
            let angle = if libm::hypot(c.x - centre.x, c.y - centre.y) < 1e-12 {
                from
            } else {
                angle
            };
            let along = ((angle - from) * sweep.signum()).rem_euclid(std::f64::consts::TAU);
            let along = if along > std::f64::consts::TAU - 1e-9 {
                0.0
            } else {
                along
            };
            if along > sweep.abs() + 1e-9 {
                return None;
            }
            let at = along * radius;
            Some((
                polar(centre, radius, angle),
                if from_end { l - at } else { at },
            ))
        }
    }
}

/// The fillet of radius `r` at the kink that turns by `turn`, tangent to a shape of `before` and
/// one of `after` (each chain nearest the kink first), within `caps` of trim along each chain.
fn round(
    (before, after): (&[Shape], &[Shape]),
    turn: f64,
    r: f64,
    caps: (f64, f64),
) -> Option<Round> {
    let side = turn.signum();
    let corner = end(&before[0]);
    let mut best: Option<(f64, Round)> = None;
    let mut behind = 0.0;
    for (i, a) in before.iter().enumerate() {
        let mut ahead = 0.0;
        if let Some(ma) = moved(a, side * r) {
            for (j, b) in after.iter().enumerate() {
                if let Some(found) = pair(&ma, a, b, (i, j), (side, r, corner)) {
                    let total = found.before.1 + behind + found.after.1 + ahead;
                    let within =
                        found.before.1 + behind <= caps.0 && found.after.1 + ahead <= caps.1;
                    if within && best.as_ref().is_none_or(|(t, _)| total < *t) {
                        best = Some((total, found));
                    }
                }
                ahead += length(b);
            }
        }
        behind += length(a);
    }
    best.map(|(_, found)| found)
}

/// The fillet between chain shapes `a` (moved: `ma`) and `b`, when it touches both within them.
fn pair(
    ma: &Shape,
    a: &Shape,
    b: &Shape,
    (i, j): (usize, usize),
    (side, r, corner): (f64, f64, Point),
) -> Option<Round> {
    let mb = moved(b, side * r)?;
    let centre = carrier_hits(ma, &mb).into_iter().min_by(|x, y| {
        let dx = libm::hypot(x.x - corner.x, x.y - corner.y);
        let dy = libm::hypot(y.x - corner.x, y.y - corner.y);
        dx.total_cmp(&dy)
    })?;
    let (pa, trim_before) = touch(a, centre, true)?;
    let (pb, trim_after) = touch(b, centre, false)?;
    let (from, to) = (
        libm::atan2(pa.y - centre.y, pa.x - centre.x),
        libm::atan2(pb.y - centre.y, pb.x - centre.x),
    );
    let sweep = side * ((to - from) * side).rem_euclid(std::f64::consts::TAU);
    (sweep.abs() < std::f64::consts::PI).then_some(Round {
        arc: Shape::Bend {
            centre,
            radius: r,
            from,
            sweep,
        },
        before: (i, trim_before),
        after: (j, trim_after),
    })
}

/// The largest fillet radius up to `wanted` that fits (ADR 0083 §13.3: lowered by bisection).
fn fit_round(
    chains: (&[Shape], &[Shape]),
    turn: f64,
    wanted: f64,
    caps: (f64, f64),
) -> Option<Round> {
    if let Some(found) = round(chains, turn, wanted, caps) {
        return Some(found);
    }
    let (mut low, mut high) = (0.0, wanted);
    let mut best = round(chains, turn, SMALLEST_FILLET, caps)?;
    for _ in 0..40 {
        let mid = f64::midpoint(low, high);
        match round(chains, turn, mid.max(SMALLEST_FILLET), caps) {
            Some(found) => {
                best = found;
                low = mid;
            }
            None => high = mid,
        }
    }
    Some(best)
}

fn forced_direction(
    path: &ClosedPath,
    from: &[Point],
    curved: &[bool],
    (i, j): (usize, usize),
) -> (f64, f64) {
    let line = |k: usize| {
        let d = (
            path.segments[k].to.x - from[k].x,
            path.segments[k].to.y - from[k].y,
        );
        unit(d)
    };
    let (a, b) = (
        unit(tangent(from[i], &path.segments[i], true)),
        unit(tangent(from[j], &path.segments[j], false)),
    );
    if !curved[i] {
        line(i)
    } else if !curved[j] {
        line(j)
    } else {
        unit((a.0 + b.0, a.1 + b.1))
    }
}

/// The shapes of every cubic in order, the join after each one on the last of them.
fn items(path: &ClosedPath, corner_degrees: f64) -> Vec<Item> {
    let from = starts(path);
    let n = path.segments.len();
    let classified = classify(path, corner_degrees);
    let mut out: Vec<Item> = Vec::new();
    for i in 0..n {
        let cubic = &path.segments[i];
        let shapes: Vec<Shape> = if classified.curved[i] {
            let tangent_before = classified.joins[(i + n - 1) % n].kind == JoinKind::Tangent;
            let tangent_after = classified.joins[i].kind == JoinKind::Tangent;
            let ends = (
                tangent_before.then(|| {
                    forced_direction(path, &from, &classified.curved, ((i + n - 1) % n, i))
                }),
                tangent_after
                    .then(|| forced_direction(path, &from, &classified.curved, (i, (i + 1) % n))),
            );
            arcs(from[i], cubic, ends)
        } else if libm::hypot(cubic.to.x - from[i].x, cubic.to.y - from[i].y) > 1e-9 {
            vec![Shape::Straight {
                from: from[i],
                to: cubic.to,
            }]
        } else {
            Vec::new()
        };
        let join = classified.joins[i];
        let count = shapes.len();
        for (k, shape) in shapes.into_iter().enumerate() {
            let last = k + 1 == count;
            out.push(Item {
                shape,
                cut: (last && join.kind != JoinKind::Tangent).then_some(join),
            });
        }
        // A cubic without shape still carries its join onto the item before it.
        if count == 0
            && join.kind != JoinKind::Tangent
            && let Some(previous) = out.last_mut()
        {
            previous.cut = Some(join);
        }
    }
    merge_straights(out)
}

/// Two straights with a tangent join between them are one straight.
fn merge_straights(items: Vec<Item>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for item in items {
        if let (Some(last), Shape::Straight { to, .. }) = (out.last_mut(), item.shape)
            && last.cut.is_none()
            && let Shape::Straight { from, .. } = last.shape
            && matches!(item.shape, Shape::Straight { .. })
        {
            last.shape = Shape::Straight { from, to };
            last.cut = item.cut;
            continue;
        }
        out.push(item);
    }
    // The ring's last straight may continue into the first.
    if out.len() > 1
        && out[out.len() - 1].cut.is_none()
        && let (Shape::Straight { from, .. }, Shape::Straight { to, .. }) =
            (out[out.len() - 1].shape, out[0].shape)
    {
        out.pop();
        out[0].shape = Shape::Straight { from, to };
    }
    out
}

/// The items either side of the kink at the end of item `k`, nearest first: a chain runs on over
/// tangent joins and stops at the next kink or corner.
fn chains(order: &[Item], k: usize) -> (Vec<usize>, Vec<usize>) {
    let count = order.len();
    let mut back = vec![k];
    while back.len() < count - 1 {
        let before = (back[back.len() - 1] + count - 1) % count;
        if order[before].cut.is_some() {
            break;
        }
        back.push(before);
    }
    let mut ahead = vec![(k + 1) % count];
    while ahead.len() + back.len() < count {
        let last = ahead[ahead.len() - 1];
        if order[last].cut.is_some() {
            break;
        }
        ahead.push((last + 1) % count);
    }
    (back, ahead)
}

/// How much of a chain a fillet may take: all of it, half when a kink shares its far end.
fn chain_cap(order: &[Item], chain: &[usize], backwards: bool) -> f64 {
    let count = order.len();
    let total: f64 = chain.iter().map(|&i| length(&order[i].shape)).sum();
    let far = chain[chain.len() - 1];
    let shared = if backwards {
        order[(far + count - 1) % count]
            .cut
            .is_some_and(|j| j.kind == JoinKind::Kink)
    } else {
        order[far].cut.is_some_and(|j| j.kind == JoinKind::Kink)
    };
    if shared { total / 2.0 } else { total }
}

/// The fillet of every kink and the trims it asks of the neighbouring shapes: per shape the
/// distance cut from its start and from its end, and how many kinks no fillet fitted.
fn fillets_of(
    order: &[Item],
    (wall, min_arc): (f64, f64),
) -> (Vec<(f64, f64)>, Vec<Option<Round>>, usize) {
    let count = order.len();
    let mut trims = vec![(0.0_f64, 0.0_f64); count];
    let mut rounds: Vec<Option<Round>> = (0..count).map(|_| None).collect();
    let mut unrounded = 0;
    for k in 0..count {
        let Some(join) = order[k].cut.filter(|j| j.kind == JoinKind::Kink) else {
            continue;
        };
        let (back, ahead) = chains(order, k);
        let shapes_of = |ids: &[usize]| ids.iter().map(|&i| order[i].shape).collect::<Vec<_>>();
        let (before, after) = (shapes_of(&back), shapes_of(&ahead));
        let caps = (
            chain_cap(order, &back, true),
            chain_cap(order, &ahead, false),
        );
        let wanted = (2.0 * wall).max(min_arc / join.turn.abs());
        if let Some(found) = fit_round((&before, &after), join.turn, wanted, caps) {
            for (m, &id) in back.iter().enumerate().take(found.before.0 + 1) {
                trims[id].1 = if m == found.before.0 {
                    found.before.1
                } else {
                    length(&order[id].shape)
                };
            }
            for (m, &id) in ahead.iter().enumerate().take(found.after.0 + 1) {
                trims[id].0 = if m == found.after.0 {
                    found.after.1
                } else {
                    length(&order[id].shape)
                };
            }
            rounds[k] = Some(found);
        } else {
            unrounded += 1;
        }
    }
    (trims, rounds, unrounded)
}

/// The shapes of the footprint of `path`, walking from its first corner (else from its first
/// shape).
pub(crate) fn curve_shapes(
    path: &ClosedPath,
    corner_degrees: f64,
    (wall, min_arc): (f64, f64),
) -> Shapes {
    let ring = items(&split_cusps(path), corner_degrees);
    let count = ring.len();
    let first = ring
        .iter()
        .position(|item| item.cut.is_some_and(|j| j.kind == JoinKind::Corner))
        .map_or(0, |k| (k + 1) % count);
    let order: Vec<Item> = (0..count).map(|k| ring[(first + k) % count]).collect();
    let (trims, rounds, unrounded) = fillets_of(&order, (wall, min_arc));
    let mut out = Shapes {
        shapes: Vec::new(),
        piece_of: Vec::new(),
        corners: Vec::new(),
        fillets: Vec::new(),
        flat_turns: Vec::new(),
        unrounded: 0,
    };
    let mut piece = 0;
    for k in 0..count {
        let shape = &order[k].shape;
        let l = length(shape);
        if l - trims[k].0 - trims[k].1 >= ZERO_SHAPE {
            out.shapes.push(between(shape, trims[k].0, l - trims[k].1));
            out.piece_of.push(piece);
        }
        if let Some(found) = &rounds[k] {
            out.fillets.push((out.shapes.len(), radius_of(&found.arc)));
            out.shapes.push(found.arc);
            out.piece_of.push(piece);
        }
        if let Some(join) = order[k].cut.filter(|j| j.kind == JoinKind::Corner) {
            out.corners.push(Vertex {
                at: end(shape),
                turn: join.turn,
                kind: JoinKind::Corner,
            });
            piece += 1;
        }
    }
    // The corner the walk started after is the last one met; it opens the first piece.
    if !out.corners.is_empty() {
        out.corners.rotate_right(1);
    }
    out.flat_turns = vec![0.0; out.shapes.len()];
    out.unrounded = unrounded;
    out
}

fn radius_of(shape: &Shape) -> f64 {
    match *shape {
        Shape::Bend { radius, .. } => radius,
        Shape::Straight { .. } => 0.0,
    }
}

#[cfg(test)]
mod tests;
