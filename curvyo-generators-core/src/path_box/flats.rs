//! Flats beside a corner on a curve: the first stretch of the run, z + t long, is replaced by its
//! chord, so the wood that does not bend is straight and the finger joint is cut as the plates
//! meet it (ADR 0083 §13.7).

use curvyo_document_core::Point;

use super::corner::Corner;
use super::shape::Shapes;
use crate::footprint::Shape;
use crate::shape_ops::{Chain, between, direction, end, length, start, turn};

/// The joint as cut may differ from the tangent turn by this much (radians, 10 degrees).
const MAX_SKEW: f64 = 10.0 * std::f64::consts::PI / 180.0;
/// A flat replaces a stretch that turns by at most this much (radians, 45 degrees).
const MAX_STRETCH_TURN: f64 = 45.0 * std::f64::consts::PI / 180.0;
/// A flat shorter than this (mm) is no flat.
const ZERO_FLAT: f64 = 0.01;

/// The bends beside a corner, walking away from it (the lead of a piece) or towards it (the
/// trail), and the shape next to the corner when it is no bend.
struct Side {
    chain: Option<Chain>,
    /// The direction of the walk at the corner when the side is straight.
    straight: (f64, f64),
}

impl Side {
    /// The point `f` along the side from the corner.
    fn point(&self, f: f64, lead: bool) -> Option<Point> {
        let chain = self.chain.as_ref()?;
        Some(if lead {
            chain.at(f)
        } else {
            chain.at(chain.total - f)
        })
    }

    /// The most a flat of this side may be: a third of the run, turning by at most 45 degrees.
    fn allows(&self, f: f64, lead: bool) -> bool {
        let Some(chain) = &self.chain else {
            return true;
        };
        let turn = if lead {
            chain.turn_between(0.0, f)
        } else {
            chain.turn_between(chain.total - f, chain.total)
        };
        f <= chain.total / 3.0 + 1e-9 && turn.abs() <= MAX_STRETCH_TURN + 1e-9
    }
}

fn unit(a: Point, b: Point) -> (f64, f64) {
    let l = libm::hypot(b.x - a.x, b.y - a.y).max(1e-12);
    ((b.x - a.x) / l, (b.y - a.y) / l)
}

fn signed_turn(a: (f64, f64), b: (f64, f64)) -> f64 {
    libm::atan2(a.0 * b.1 - a.1 * b.0, a.0 * b.0 + a.1 * b.1)
}

/// The turn of the joint when both flats are `f` long.
fn cut_turn(at: Point, (trail, lead): (&Side, &Side), f: f64) -> f64 {
    let d_in = trail
        .point(f, false)
        .map_or(trail.straight, |p| unit(p, at));
    let d_out = lead.point(f, true).map_or(lead.straight, |p| unit(at, p));
    signed_turn(d_in, d_out)
}

/// The longest flat up to `wanted` that keeps the joint within its bounds (bisection; 0 always
/// meets them).
fn flat_length(
    at: Point,
    sides: (&Side, &Side),
    tangent_turn: f64,
    wanted: f64,
    corner_degrees: f64,
) -> f64 {
    let ok = |f: f64| {
        let turn = cut_turn(at, sides, f);
        sides.0.allows(f, false)
            && sides.1.allows(f, true)
            && turn.signum() == tangent_turn.signum()
            && turn.abs() >= corner_degrees.to_radians() - 1e-9
            && (turn.abs() - tangent_turn.abs()).abs() <= MAX_SKEW + 1e-9
    };
    if ok(wanted) {
        return wanted;
    }
    let (mut low, mut high) = (0.0, wanted);
    for _ in 0..40 {
        let mid = f64::midpoint(low, high);
        if ok(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// A shape of the rebuilt piece with the radius of its fillet and the turn a flat replaced.
type Item = (Shape, Option<f64>, f64);

/// Takes the first `f` of the items off the front and puts the flat in its place.
fn cut_front(items: &mut Vec<Item>, f: f64, to: Point) {
    let corner = start(&items[0].0);
    let (mut left, mut replaced) = (f, 0.0);
    while left > 1e-12 && !items.is_empty() {
        let l = length(&items[0].0);
        if l <= left + 1e-9 {
            replaced += turn(&items[0].0);
            left -= l;
            items.remove(0);
        } else {
            replaced += turn(&between(&items[0].0, 0.0, left));
            items[0].0 = between(&items[0].0, left, l);
            left = 0.0;
        }
    }
    items.insert(0, (Shape::Straight { from: corner, to }, None, replaced));
}

/// Takes the last `f` of the items off the back and puts the flat in its place.
fn cut_back(items: &mut Vec<Item>, f: f64, from: Point) {
    let corner = end(&items[items.len() - 1].0);
    let (mut left, mut replaced) = (f, 0.0);
    while left > 1e-12 && !items.is_empty() {
        let last = items.len() - 1;
        let l = length(&items[last].0);
        if l <= left + 1e-9 {
            replaced += turn(&items[last].0);
            left -= l;
            items.pop();
        } else {
            replaced += turn(&between(&items[last].0, l - left, l));
            items[last].0 = between(&items[last].0, 0.0, l - left);
            left = 0.0;
        }
    }
    items.push((Shape::Straight { from, to: corner }, None, replaced));
}

/// The leading (or trailing) bends of a piece as a chain, `None` when its first (last) shape is
/// straight.
fn side(piece: &[Shape], lead: bool) -> Side {
    let bends: Vec<Shape> = if lead {
        piece
            .iter()
            .take_while(|s| matches!(s, Shape::Bend { .. }))
            .copied()
            .collect()
    } else {
        let mut tail: Vec<Shape> = piece
            .iter()
            .rev()
            .take_while(|s| matches!(s, Shape::Bend { .. }))
            .copied()
            .collect();
        tail.reverse();
        tail
    };
    let next_to_corner = if lead { piece.first() } else { piece.last() };
    Side {
        chain: (!bends.is_empty()).then(|| Chain::new(bends)),
        straight: next_to_corner.map_or((1.0, 0.0), |s| direction(s, !lead)),
    }
}

/// Puts the flats beside the corners of `built` and updates each corner's turn to the joint as
/// cut.
pub(crate) fn apply(built: &mut Shapes, wall: f64, corner_degrees: f64) {
    let count = built.corners.len();
    if count == 0 {
        return;
    }
    let mut pieces: Vec<Vec<Item>> = vec![Vec::new(); count];
    for (index, shape) in built.shapes.iter().enumerate() {
        let radius = built
            .fillets
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r);
        pieces[built.piece_of[index]].push((*shape, radius, 0.0));
    }
    let shapes_of = |piece: &[Item]| piece.iter().map(|i| i.0).collect::<Vec<_>>();
    let sides: Vec<(Side, Side)> = (0..count)
        .map(|p| {
            let shapes = shapes_of(&pieces[p]);
            (side(&shapes, false), side(&shapes, true))
        })
        .collect();
    for k in 0..count {
        let previous = (k + count - 1) % count;
        let (trail, lead) = (&sides[previous].0, &sides[k].1);
        if trail.chain.is_none() && lead.chain.is_none() {
            continue;
        }
        let vertex = built.corners[k];
        let z = Corner::new(vertex.turn, wall).z;
        let f = flat_length(
            vertex.at,
            (trail, lead),
            vertex.turn,
            z + wall,
            corner_degrees,
        );
        if f >= ZERO_FLAT {
            if let Some(p) = lead.point(f, true) {
                cut_front(&mut pieces[k], f, p);
            }
            if let Some(p) = trail.point(f, false) {
                cut_back(&mut pieces[previous], f, p);
            }
            built.corners[k].turn = cut_turn(vertex.at, (trail, lead), f);
        }
    }
    rebuild(built, pieces);
}

/// Writes the pieces back as the shapes of `built`.
fn rebuild(built: &mut Shapes, pieces: Vec<Vec<Item>>) {
    let (mut shapes, mut piece_of, mut fillets, mut flat_turns) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (p, piece) in pieces.into_iter().enumerate() {
        for (shape, radius, replaced) in piece {
            if let Some(r) = radius {
                fillets.push((shapes.len(), r));
            }
            shapes.push(shape);
            piece_of.push(p);
            flat_turns.push(replaced);
        }
    }
    built.shapes = shapes;
    built.piece_of = piece_of;
    built.fillets = fillets;
    built.flat_turns = flat_turns;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_box::fixtures::{heart, shield};
    use crate::path_box::spans::curve_shapes;

    const WALL: f64 = 3.0;

    fn flats_of(built: &Shapes) -> Vec<usize> {
        (0..built.shapes.len())
            .filter(|&i| built.flat_turns[i] != 0.0)
            .collect()
    }

    #[test]
    fn a_heart_gets_a_flat_on_both_sides_of_both_corners_and_stays_closed() {
        let mut built = curve_shapes(&heart(), 25.0, (WALL, 8.0));
        let tangent: Vec<f64> = built.corners.iter().map(|c| c.turn).collect();
        apply(&mut built, WALL, 25.0);
        assert_eq!(flats_of(&built).len(), 4);
        let n = built.shapes.len();
        for i in 0..n {
            let (a, b) = (&built.shapes[i], &built.shapes[(i + 1) % n]);
            let gap = libm::hypot(end(a).x - start(b).x, end(a).y - start(b).y);
            assert!(gap < 1e-6, "shape {i} meets the next: {gap}");
        }
        for (cut, original) in built.corners.iter().zip(&tangent) {
            assert_eq!(cut.turn.signum(), original.signum(), "keeps its sign");
            assert!(cut.turn.abs() >= 25.0_f64.to_radians());
            assert!(
                (cut.turn.abs() - original.abs()).abs() <= MAX_SKEW + 1e-9,
                "within 10 degrees of the tangent turn"
            );
        }
    }

    #[test]
    fn a_flat_is_at_most_z_plus_t_long_and_at_most_a_third_of_its_run() {
        let mut built = curve_shapes(&heart(), 25.0, (WALL, 8.0));
        let before: f64 = built.shapes.iter().map(length).sum();
        apply(&mut built, WALL, 25.0);
        for i in flats_of(&built) {
            let chord = length(&built.shapes[i]);
            assert!(
                chord <= 2.0 * WALL + WALL + 1e-9,
                "z + t at most 3 t: {chord}"
            );
        }
        let after: f64 = built.shapes.iter().map(length).sum();
        assert!(
            after < before,
            "chords are shorter than the arcs they replace"
        );
        assert!(
            (before - after) < 0.5 * 4.0,
            "by the sagitta, a fraction of a millimetre each"
        );
    }

    #[test]
    fn a_shield_has_no_flat_its_corners_are_between_straights() {
        let mut built = curve_shapes(&shield(), 25.0, (WALL, 8.0));
        apply(&mut built, WALL, 25.0);
        assert_eq!(flats_of(&built), Vec::<usize>::new());
    }
}
