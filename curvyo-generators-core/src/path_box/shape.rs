//! The outline as a footprint of straights and fillets: kinks are rounded on the inner face,
//! corners stay sharp (ADR 0083 §3).

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

use super::classify::{JoinKind, classify};
use crate::footprint::Shape;

/// A straight shorter than this is no straight.
const ZERO_STRAIGHT: f64 = 0.01;

/// A vertex of the outline that matters: a kink or a corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vertex {
    pub(crate) at: Point,
    /// The turn in radians, positive clockwise on screen.
    pub(crate) turn: f64,
    pub(crate) kind: JoinKind,
}

#[allow(clippy::struct_field_names)]
/// The shapes of a footprint in walk order, which piece each belongs to and the corners it passes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shapes {
    pub(crate) shapes: Vec<Shape>,
    /// The wall piece (0 based) each shape belongs to.
    pub(crate) piece_of: Vec<usize>,
    /// The corner vertices in walk order; the first one is where the first shape starts.
    pub(crate) corners: Vec<Vertex>,
    /// Every fillet with the radius it got, as `(shape index, radius)`.
    pub(crate) fillets: Vec<(usize, f64)>,
    /// The signed tangent turn a flat replaces, per shape (0 for every shape that is not a flat;
    /// its neutral length holds half a wall thickness times it, ADR 0083 §13.5).
    pub(crate) flat_turns: Vec<f64>,
    /// Kinks no fillet fitted (left as sharp bends, the notice `fillet_failed`).
    pub(crate) unrounded: usize,
}

fn unit(a: Point, b: Point) -> (f64, f64, f64) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = libm::hypot(dx, dy);
    (dx / length, dy / length, length)
}

fn polygon_path(points: &[Point]) -> ClosedPath {
    ClosedPath {
        start: points[0],
        segments: (0..points.len())
            .map(|i| {
                let to = points[(i + 1) % points.len()];
                Cubic {
                    c1: points[i],
                    c2: to,
                    to,
                }
            })
            .collect(),
    }
}

/// The kinks and corners of the polygon `points` for the Corner angle: tangent vertices vanish.
pub(crate) fn vertices(points: &[Point], corner_degrees: f64) -> Vec<Vertex> {
    let classified = classify(&polygon_path(points), corner_degrees);
    let count = points.len();
    (0..count)
        .filter_map(|i| {
            let join = classified.joins[(i + count - 1) % count];
            (join.kind != JoinKind::Tangent).then_some(Vertex {
                at: points[i],
                turn: join.turn,
                kind: join.kind,
            })
        })
        .collect()
}

/// The fillet radius of every vertex: `2 t` at a kink, 0 at a corner, shortened where the straight
/// between two kinks is too short for both tangent lengths (ADR 0083 §3).
fn radii(verts: &[Vertex], wall: f64, min_arc: f64) -> Vec<f64> {
    let tan = |v: &Vertex| {
        if v.kind == JoinKind::Kink {
            libm::tan(v.turn.abs() / 2.0)
        } else {
            0.0
        }
    };
    let mut radii: Vec<f64> = verts
        .iter()
        .map(|v| {
            if v.kind == JoinKind::Kink {
                // Two slit columns at the least gap fit the arc (lead decision, Proposal).
                (2.0 * wall).max(min_arc / v.turn.abs())
            } else {
                0.0
            }
        })
        .collect();
    for i in 0..verts.len() {
        let j = (i + 1) % verts.len();
        let (_, _, length) = unit(verts[i].at, verts[j].at);
        let demand = tan(&verts[i]) + tan(&verts[j]);
        if demand > 0.0 {
            let limit = length / demand;
            for k in [i, j] {
                if radii[k] > 0.0 {
                    radii[k] = radii[k].min(limit);
                }
            }
        }
    }
    radii
}

/// The fillet at `v` between the walks `d_in` and `d_out` with radius `r`: its centre, where it
/// starts and the tangent length.
fn fillet(v: &Vertex, (d_in, _d_out): ((f64, f64), (f64, f64)), r: f64) -> Shape {
    let t = r * libm::tan(v.turn.abs() / 2.0);
    let start = Point::new(v.at.x - d_in.0 * t, v.at.y - d_in.1 * t);
    // The right-hand normal of the walk is the inside of a clockwise outline.
    let side = if v.turn > 0.0 { 1.0 } else { -1.0 };
    let centre = Point::new(start.x - d_in.1 * r * side, start.y + d_in.0 * r * side);
    Shape::Bend {
        centre,
        radius: r,
        from: libm::atan2(start.y - centre.y, start.x - centre.x),
        sweep: v.turn,
    }
}

/// The footprint shapes of `verts` for a wall of thickness `wall`, walking from the first corner
/// (or from the first vertex when there is none).
pub(crate) fn shapes(verts: &[Vertex], wall: f64, min_arc: f64) -> Shapes {
    let radii = radii(verts, wall, min_arc);
    let count = verts.len();
    let first = verts
        .iter()
        .position(|v| v.kind == JoinKind::Corner)
        .unwrap_or(0);
    let mut out = Shapes {
        shapes: Vec::new(),
        piece_of: Vec::new(),
        corners: Vec::new(),
        fillets: Vec::new(),
        flat_turns: Vec::new(),
        unrounded: 0,
    };
    let mut piece = 0;
    for step in 0..count {
        let (a, b) = ((first + step) % count, (first + step + 1) % count);
        let before = (first + step + count - 1) % count;
        let d_in = unit(verts[before].at, verts[a].at);
        let d_out = unit(verts[a].at, verts[b].at);
        if verts[a].kind == JoinKind::Corner {
            if step > 0 {
                piece += 1;
            }
            out.corners.push(verts[a]);
        } else if radii[a] > 0.0 {
            out.fillets.push((out.shapes.len(), radii[a]));
            out.shapes.push(fillet(
                &verts[a],
                ((d_in.0, d_in.1), (d_out.0, d_out.1)),
                radii[a],
            ));
            out.piece_of.push(piece);
        }
        let tangent = |k: usize| radii[k] * libm::tan(verts[k].turn.abs() / 2.0);
        let (ta, tb) = (tangent(a), tangent(b));
        let from = Point::new(verts[a].at.x + d_out.0 * ta, verts[a].at.y + d_out.1 * ta);
        let to = Point::new(verts[b].at.x - d_out.0 * tb, verts[b].at.y - d_out.1 * tb);
        if d_out.2 - ta - tb >= ZERO_STRAIGHT {
            out.shapes.push(Shape::Straight { from, to });
            out.piece_of.push(piece);
        }
    }
    out.flat_turns = vec![0.0; out.shapes.len()];
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(points: &[(f64, f64)]) -> Vec<Point> {
        points.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    fn square(side: f64) -> Vec<Point> {
        pts(&[(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)])
    }

    #[test]
    fn a_square_has_four_corners_four_straights_and_no_fillet() {
        let verts = vertices(&square(40.0), 25.0);
        let built = shapes(&verts, 3.0, 0.0);
        assert_eq!(built.corners.len(), 4);
        assert_eq!(built.shapes.len(), 4);
        assert_eq!(built.fillets, []);
        assert_eq!(built.piece_of, vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_kink_is_a_fillet_of_twice_the_wall_thickness_on_the_inner_face() {
        // A square whose top edge has a 20 degree kink in the middle.
        let verts = vertices(
            &pts(&[
                (0.0, 0.0),
                (20.0, -3.64),
                (40.0, 0.0),
                (40.0, 40.0),
                (0.0, 40.0),
            ]),
            25.0,
        );
        let built = shapes(&verts, 3.0, 0.0);
        assert_eq!(built.fillets.len(), 1);
        let Shape::Bend {
            radius,
            sweep,
            centre,
            ..
        } = built.shapes[built.fillets[0].0]
        else {
            panic!("a bend");
        };
        assert!((radius - 6.0).abs() < 1e-9);
        assert!(sweep > 0.0, "convex: clockwise");
        assert!(
            centre.y > -3.64,
            "the centre lies on the inside of the apex"
        );
    }

    #[test]
    fn a_straight_too_short_for_two_fillets_gives_both_the_radius_that_meets_in_its_middle() {
        // A 20-gon of circumradius 4 mm: edges of 1.25 mm cannot hold two fillets of 6 mm.
        let n = 20;
        let ring: Vec<Point> = (0..n)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / f64::from(n);
                Point::new(4.0 * libm::cos(a), 4.0 * libm::sin(a))
            })
            .collect();
        let built = shapes(&vertices(&ring, 25.0), 3.0, 0.0);
        assert_eq!(built.fillets.len(), 20);
        let edge = 2.0 * 4.0 * libm::sin(std::f64::consts::PI / 20.0);
        for (_, r) in &built.fillets {
            let half = libm::tan(std::f64::consts::TAU / 40.0);
            assert!(
                (2.0 * r * half - edge).abs() < 1e-6,
                "tangent points meet in the middle"
            );
        }
        assert!(
            built.shapes.iter().all(|s| matches!(s, Shape::Bend { .. })),
            "no straight is left"
        );
    }
}
