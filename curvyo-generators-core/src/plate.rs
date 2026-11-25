//! One plate of a generated box: a rectangle with finger notches, tabs or a finger hollow along its
//! edges and a square, open, notched or round corner, built as a walk round the four edges and
//! offset by half the kerf.
//!
//! Every plate is the full rectangle of its box side; where two plates meet, each edge strip is
//! cut into segments that alternate between the two plates, and the corner cubes belong to the
//! plate that owns them. A notch is as deep as the mating plate is thick, so every edge has its
//! own depth (ADR 0082 §4, §11).

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

use crate::contour::Contour;
use crate::fingers::is_notched;
use crate::wall_plates::Hollow;

/// How a corner of the plate is treated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Corner {
    /// The plate owns the square (the bottom and the lid); the edge strips stop short of it.
    Owned,
    /// A plain corner of an open edge: no square, the strips run to the rectangle's corner.
    Open,
    /// The square belongs to the neighbouring plate: cut out.
    Notched,
    /// A quarter arc of this radius, tangent to both edges.
    Round(f64),
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

/// One edge: plain, a finger strip or tabs. The pattern runs in the physical direction of the edge
/// (increasing x along the top and bottom, increasing y along the left and right).
#[derive(Debug, Clone)]
pub(crate) enum Edge {
    Plain,
    Fingered(Fingers),
    /// Tabs standing out of the edge by the edge's depth: `(from, to)` along it.
    Tabs(Vec<(f64, f64)>),
}

/// A plate: size, the four corners (top left, top right, bottom right, bottom left) and the four
/// edges (top, right, bottom, left).
#[derive(Debug, Clone)]
pub(crate) struct Plate {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) corners: [Corner; 4],
    pub(crate) edges: [Edge; 4],
    /// The depth of each edge's notches and tabs (top, right, bottom, left): the thickness of the
    /// plate that meets it. A square corner takes the depths of its two edges.
    pub(crate) depths: [f64; 4],
    /// Total play of a joint: every notch widens by a quarter of it on each side.
    pub(crate) clearance: f64,
    /// A U notch centred in the top edge.
    pub(crate) hollow: Option<Hollow>,
}

/// A run of one edge: from `a` to `b` along it, cut `inset` into the plate (negative: a tab).
type Run = (f64, f64, f64);

/// One step of the walk: a corner point, or an arc that joins the points before and after it.
#[derive(Debug, Clone, Copy)]
enum Node {
    Pt(Point),
    Arc {
        centre: Point,
        radius: f64,
        from: f64,
        sweep: f64,
    },
}

/// The corners at the physical start and end of each edge (increasing x or y).
const ENDS: [(usize, usize); 4] = [(0, 1), (1, 2), (3, 2), (0, 3)];

impl Plate {
    /// The runs of `edge` (index `index`) over `lo..hi` in the physical direction, merged where the
    /// inset is equal.
    fn runs(&self, index: usize, lo: f64, hi: f64) -> Vec<Run> {
        let depth = self.depths[index];
        let fingers = match &self.edges[index] {
            Edge::Plain => return vec![(lo, hi, 0.0)],
            Edge::Tabs(tabs) => {
                let mut runs: Vec<Run> = Vec::new();
                let mut at = lo;
                for &(a, b) in tabs {
                    runs.push((at, a, 0.0));
                    runs.push((a, b, -depth));
                    at = b;
                }
                runs.push((at, hi, 0.0));
                runs.retain(|run| run.1 - run.0 > 1e-12);
                return runs;
            }
            Edge::Fingered(fingers) => *fingers,
        };
        let mut runs: Vec<Run> = vec![(lo, fingers.from, 0.0)];
        let step = (fingers.to - fingers.from) / f64::from(fingers.segments);
        let widen = self.clearance / 4.0;
        for index in 0..fingers.segments {
            let notched = is_notched(index, fingers.even_solid);
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
                runs.push((a, b, depth));
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

    /// How far the square, notch or arc of `corner` reaches along the edge `edge` that meets it.
    fn extent(&self, corner: usize, edge: usize) -> f64 {
        match self.corners[corner] {
            Corner::Open => 0.0,
            Corner::Round(radius) => radius,
            Corner::Owned | Corner::Notched => {
                // The square is as deep as the edge that meets this one at the corner.
                self.depths[if edge == corner {
                    (corner + 3) % 4
                } else {
                    corner
                }]
            }
        }
    }

    fn length(&self, edge: usize) -> f64 {
        if edge.is_multiple_of(2) {
            self.width
        } else {
            self.height
        }
    }

    /// The along-edge coordinates between which the edge's runs lie.
    fn span(&self, edge: usize) -> (f64, f64) {
        (
            self.extent(ENDS[edge].0, edge),
            self.length(edge) - self.extent(ENDS[edge].1, edge),
        )
    }

    /// The walk round the plate, clockwise on screen, with the top-left corner of the rectangle at
    /// the origin, before the kerf offset.
    fn nodes(&self) -> Vec<Node> {
        let (w, h) = (self.width, self.height);
        let corner_at = [
            Point::new(0.0, 0.0),
            Point::new(w, 0.0),
            Point::new(w, h),
            Point::new(0.0, h),
        ];
        // Per edge: the runs in walking order and the coordinates where the walk starts and ends.
        let walks: Vec<(Vec<Run>, f64, f64)> = (0..4)
            .map(|edge| {
                let (lo, hi) = self.span(edge);
                let runs = self.runs(edge, lo, hi);
                if edge < 2 {
                    (runs, lo, hi)
                } else {
                    let reversed = runs.into_iter().rev().map(|(a, b, i)| (b, a, i)).collect();
                    (reversed, hi, lo)
                }
            })
            .collect();
        let mut nodes: Vec<Node> = Vec::new();
        for edge in 0..4 {
            let prev = (edge + 3) % 4;
            let at = corner_at[edge];
            let from_prev = edge_point(prev, walks[prev].2, 0.0, w, h);
            let to_next = edge_point(edge, walks[edge].1, 0.0, w, h);
            nodes.extend(self.corner_nodes(edge, (from_prev, at, to_next)));
            if let (0, Some(hollow)) = (edge, self.hollow) {
                debug_assert!(
                    matches!(self.edges[0], Edge::Plain),
                    "a finger hollow needs a plain top edge"
                );
                self.hollow_nodes(&mut nodes, hollow);
                continue;
            }
            for &(from, to, inset) in &walks[edge].0 {
                nodes.push(Node::Pt(edge_point(edge, from, inset, w, h)));
                nodes.push(Node::Pt(edge_point(edge, to, inset, w, h)));
            }
        }
        simplify(nodes)
    }

    /// The nodes of the corner that starts `edge`: the corner square or notch, or the arc, between
    /// the point where the previous edge ends and the one where `edge` starts.
    fn corner_nodes(
        &self,
        edge: usize,
        (from_prev, at, to_next): (Point, Point, Point),
    ) -> Vec<Node> {
        let prev = (edge + 3) % 4;
        // Inward unit vectors of each edge: top (0,1), right (-1,0), bottom (0,-1), left (1,0).
        let inward = [(0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (1.0, 0.0)];
        match self.corners[edge] {
            Corner::Notched => vec![Node::Pt(Point::new(
                at.x + inward[prev].0 * self.depths[prev] + inward[edge].0 * self.depths[edge],
                at.y + inward[prev].1 * self.depths[prev] + inward[edge].1 * self.depths[edge],
            ))],
            Corner::Owned | Corner::Open => {
                vec![Node::Pt(from_prev), Node::Pt(at), Node::Pt(to_next)]
            }
            Corner::Round(radius) => {
                let centre = Point::new(
                    at.x + (inward[prev].0 + inward[edge].0) * radius,
                    at.y + (inward[prev].1 + inward[edge].1) * radius,
                );
                let from = libm::atan2(from_prev.y - centre.y, from_prev.x - centre.x);
                vec![
                    Node::Pt(from_prev),
                    Node::Arc {
                        centre,
                        radius,
                        from,
                        sweep: std::f64::consts::FRAC_PI_2,
                    },
                    Node::Pt(to_next),
                ]
            }
        }
    }

    /// The top edge with the U notch: down one side, a half circle over the end, up the other.
    fn hollow_nodes(&self, nodes: &mut Vec<Node>, hollow: Hollow) {
        let (lo, hi) = self.span(0);
        let centre = f64::midpoint(lo, hi);
        let radius = hollow.width / 2.0;
        let straight = (hollow.depth - radius).max(0.0);
        let at = |x: f64, y: f64| Node::Pt(Point::new(x, y));
        nodes.extend([
            at(lo, 0.0),
            at(centre - radius, 0.0),
            at(centre - radius, straight),
            Node::Arc {
                centre: Point::new(centre, straight),
                radius,
                from: std::f64::consts::PI,
                sweep: -std::f64::consts::PI,
            },
            at(centre + radius, straight),
            at(centre + radius, 0.0),
            at(hi, 0.0),
        ]);
    }

    /// The outline as a contour of lines and arcs, before the kerf offset.
    pub(crate) fn contour(&self) -> Contour {
        let nodes = self.nodes();
        let mut contour = Contour::new();
        for (index, node) in nodes.iter().enumerate() {
            match (*node, nodes[(index + 1) % nodes.len()]) {
                (Node::Pt(from), Node::Pt(to)) => contour.line_to(from, to),
                (
                    Node::Arc {
                        centre,
                        radius,
                        from,
                        sweep,
                    },
                    _,
                ) => contour.arc(centre, radius, from, sweep),
                // The arc that follows starts here.
                (Node::Pt(_), Node::Arc { .. }) => {}
            }
        }
        contour
    }

    /// The outline with every edge moved outward by half the kerf (`contour.rs`, ADR 0082 §2).
    pub(crate) fn outline(&self, kerf: f64) -> ClosedPath {
        self.contour().offset(kerf / 2.0).to_path()
    }

    /// The along-edge coordinates where the outline steps in or out along the edge `edge`, for
    /// tests of mating plates.
    #[cfg(test)]
    pub(crate) fn steps(&self, edge: usize) -> Vec<f64> {
        let (lo, hi) = self.span(edge);
        let runs = self.runs(edge, lo, hi);
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

fn same(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= 1e-9 && (a.y - b.y).abs() <= 1e-9
}

/// Drops repeated points and the middle of every straight run; an arc and its end points stay.
fn simplify(nodes: Vec<Node>) -> Vec<Node> {
    let mut unique: Vec<Node> = Vec::new();
    for node in nodes {
        match (unique.last(), node) {
            (Some(Node::Pt(last)), Node::Pt(point)) if same(*last, point) => {}
            _ => unique.push(node),
        }
    }
    if unique.len() > 1
        && let (Some(Node::Pt(first)), Some(Node::Pt(last))) = (unique.first(), unique.last())
        && same(*first, *last)
    {
        unique.pop();
    }
    let n = unique.len();
    let keep = |i: usize| match (unique[(i + n - 1) % n], unique[i], unique[(i + 1) % n]) {
        (Node::Pt(a), Node::Pt(b), Node::Pt(c)) => {
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            cross.abs() > 1e-9
        }
        _ => true,
    };
    (0..n).filter(|&i| keep(i)).map(|i| unique[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle() -> Plate {
        Plate {
            width: 40.0,
            height: 30.0,
            corners: [Corner::Open; 4],
            edges: [Edge::Plain, Edge::Plain, Edge::Plain, Edge::Plain],
            depths: [3.0, 2.0, 3.0, 2.0],
            clearance: 0.0,
            hollow: None,
        }
    }

    /// The area of the contour by the shoelace formula over its flattened cubics.
    fn area(contour: &Contour) -> f64 {
        let path = contour.to_path();
        let mut points = vec![(path.start.x, path.start.y)];
        let mut from = path.start;
        for s in &path.segments {
            for step in 1..=40 {
                let t = f64::from(step) / 40.0;
                let u = 1.0 - t;
                let mix = |a: f64, b: f64, c: f64, d: f64| {
                    u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
                };
                points.push((
                    mix(from.x, s.c1.x, s.c2.x, s.to.x),
                    mix(from.y, s.c1.y, s.c2.y, s.to.y),
                ));
            }
            from = s.to;
        }
        let n = points.len();
        (0..n)
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % n]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn a_round_corner_is_a_quarter_arc_that_takes_the_corner_square_away() {
        let mut plate = rectangle();
        plate.corners[1] = Corner::Round(10.0);
        let contour = plate.contour();
        let bounds = contour.bounds();
        assert!((bounds.2 - 40.0).abs() < 1e-9 && (bounds.1).abs() < 1e-9);
        // The rectangle less the square beside the arc plus the quarter disc.
        let want = 40.0 * 30.0 - 100.0 + std::f64::consts::PI * 100.0 / 4.0;
        assert!((area(&contour) - want).abs() < 0.5, "{}", area(&contour));
    }

    #[test]
    fn tabs_stand_out_of_an_edge_by_its_depth() {
        let mut plate = rectangle();
        plate.edges[1] = Edge::Tabs(vec![(5.0, 11.0), (18.0, 24.0)]);
        let bounds = plate.contour().bounds();
        assert!(
            (bounds.2 - 42.0).abs() < 1e-9,
            "tabs reach 2 mm past the edge"
        );
        let want = 40.0 * 30.0 + 2.0 * 6.0 * 2.0;
        assert!((area(&plate.contour()) - want).abs() < 1e-9);
    }

    #[test]
    fn a_finger_hollow_is_a_u_notch_centred_in_the_top_edge() {
        let mut plate = rectangle();
        plate.hollow = Some(Hollow {
            width: 10.0,
            depth: 8.0,
        });
        let notch = 10.0 * 3.0 + std::f64::consts::PI * 25.0 / 2.0;
        let want = 40.0 * 30.0 - notch;
        assert!((area(&plate.contour()) - want).abs() < 0.5);
    }
}
