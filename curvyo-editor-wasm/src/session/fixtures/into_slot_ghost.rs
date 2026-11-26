//! The content outlines of a unit drawn fitted in a slot during a library drag
//! (`specs/0154-library-into-slots` UX notes 3.1 and V2): every leaf's outline flattened once at
//! the drag's start, mapped by the slot's fit on each frame.

use curvyo_document_core::{
    Affine, Document, NodeId, ObjectSnapshot, OutlineAnchor, Point, Vec2,
    drawing_outline_of_rotated,
};

/// Past this many leaves (unit leaves times lit slots) the box alone is drawn (the `0100` rule).
pub(super) const MAX_GHOST_LEAVES: usize = 2000;
/// The pieces each curved segment is flattened into.
const PIECES: u32 = 8;

/// The flattened outlines of a unit's leaves, in the source document's space.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct UnitOutlines {
    /// One polyline per subpath; a closed one repeats its first point at the end.
    lines: Vec<Vec<Point>>,
    /// How many leaves they come from.
    pub leaves: usize,
}

impl UnitOutlines {
    /// The outlines of the leaves of `roots` in `document`; `None` past [`MAX_GHOST_LEAVES`].
    pub(super) fn of(document: &Document, roots: &[NodeId]) -> Option<Self> {
        let leaves: Vec<NodeId> = roots
            .iter()
            .flat_map(|&id| {
                if document.is_group(id) {
                    document.leaves_of(id)
                } else {
                    vec![id]
                }
            })
            .collect();
        if leaves.len() > MAX_GHOST_LEAVES {
            return None;
        }
        let mut lines = Vec::new();
        for leaf in &leaves {
            if let Some(object) = document.object(*leaf) {
                push_outlines(&object, &mut lines);
            }
        }
        Some(Self {
            lines,
            leaves: leaves.len(),
        })
    }

    /// The outlines mapped by `m`, as segments.
    pub(super) fn segments(&self, m: Affine) -> Vec<(Point, Point)> {
        let mut out = Vec::new();
        for line in &self.lines {
            let mapped: Vec<Point> = line.iter().map(|&p| m.apply_point(p)).collect();
            out.extend(mapped.windows(2).map(|w| (w[0], w[1])));
        }
        out
    }
}

/// One anchor as point and handle offsets.
#[derive(Clone, Copy)]
struct Knot {
    point: Point,
    handle_in: Vec2,
    handle_out: Vec2,
}

/// Flattens `object`'s outlines into `lines`; an image or a dimension adds none.
fn push_outlines(object: &ObjectSnapshot, lines: &mut Vec<Vec<Point>>) {
    match object {
        ObjectSnapshot::Primitive(primitive) => {
            let outline = drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
            let knots: Vec<Knot> = outline.anchors.iter().map(knot_of_outline).collect();
            lines.push(flatten(&knots, outline.closed));
        }
        ObjectSnapshot::Image(_) | ObjectSnapshot::Dimension(_) => {}
        _ => {
            if let Some(path) = object.drawn_path() {
                for subpath in path.subpaths() {
                    let knots: Vec<Knot> = subpath
                        .anchors
                        .iter()
                        .map(|a| Knot {
                            point: a.point,
                            handle_in: a.handle_in,
                            handle_out: a.handle_out,
                        })
                        .collect();
                    lines.push(flatten(&knots, subpath.closed));
                }
            }
        }
    }
}

const fn knot_of_outline(a: &OutlineAnchor) -> Knot {
    Knot {
        point: a.point,
        handle_in: a.handle_in,
        handle_out: a.handle_out,
    }
}

/// The polyline through `knots`: straight segments as one piece, curved ones in [`PIECES`].
fn flatten(knots: &[Knot], closed: bool) -> Vec<Point> {
    let mut out = Vec::new();
    let Some(first) = knots.first() else {
        return out;
    };
    out.push(first.point);
    let count = if closed {
        knots.len()
    } else {
        knots.len().saturating_sub(1)
    };
    for i in 0..count {
        let a = knots[i];
        let b = knots[(i + 1) % knots.len()];
        let straight = a.handle_out.length() == 0.0 && b.handle_in.length() == 0.0;
        if straight {
            out.push(b.point);
            continue;
        }
        let c1 = a.point.translated(a.handle_out);
        let c2 = b.point.translated(b.handle_in);
        for k in 1..=PIECES {
            let t = f64::from(k) / f64::from(PIECES);
            out.push(cubic(a.point, c1, c2, b.point, t));
        }
    }
    out
}

/// The point at `t` of the cubic `p0`, `c1`, `c2`, `p3`.
fn cubic(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(
        w0 * p0.x + w1 * c1.x + w2 * c2.x + w3 * p3.x,
        w0 * p0.y + w1 * c1.y + w2 * c2.y + w3 * p3.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_square_flattens_to_its_corners_and_closes() {
        let k = |x: f64, y: f64| Knot {
            point: Point::new(x, y),
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
        };
        let line = flatten(&[k(0.0, 0.0), k(1.0, 0.0), k(1.0, 1.0), k(0.0, 1.0)], true);
        assert_eq!(line.len(), 5);
        assert_eq!(line[4], Point::new(0.0, 0.0));
    }

    #[test]
    fn a_curve_ends_on_its_end_point() {
        let a = Knot {
            point: Point::new(0.0, 0.0),
            handle_in: Vec2::ZERO,
            handle_out: Vec2::new(1.0, 1.0),
        };
        let b = Knot {
            point: Point::new(3.0, 0.0),
            handle_in: Vec2::new(-1.0, 1.0),
            handle_out: Vec2::ZERO,
        };
        let line = flatten(&[a, b], false);
        assert_eq!(line.len(), 1 + PIECES as usize);
        assert!(
            line.last()
                .unwrap()
                .vector_to(Point::new(3.0, 0.0))
                .length()
                < 1e-12
        );
    }
}
