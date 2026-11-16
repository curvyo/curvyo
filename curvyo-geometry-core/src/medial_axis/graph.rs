//! The raw axis graph of a triangulated region: a node per chord, per junction triangle and per
//! end tip, linked through the triangles.

use kurbo::Point;

use super::AxisSample;
use super::boundary::Boundary;
use super::chords::axis_point;
use super::mesh::Mesh;

/// What a node of the raw graph stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeKind {
    /// The axis point of a chord.
    Chord,
    /// The centre of a junction triangle.
    Junction,
    /// The tip of an end triangle, on the outline.
    Tip,
}

/// The axis graph with removable nodes.
#[derive(Debug, Clone)]
pub(super) struct Graph {
    pub(super) samples: Vec<AxisSample>,
    pub(super) kinds: Vec<NodeKind>,
    pub(super) links: Vec<Vec<usize>>,
    pub(super) alive: Vec<bool>,
}

impl Graph {
    /// The graph of `mesh`, with clearances measured against `boundary`.
    pub(super) fn build(mesh: &Mesh, boundary: &Boundary) -> Self {
        let mut graph = Self {
            samples: Vec::new(),
            kinds: Vec::new(),
            links: Vec::new(),
            alive: Vec::new(),
        };
        let chords = graph.chord_nodes(mesh, boundary);
        for (t, triangle) in mesh.triangles.iter().enumerate() {
            let inner: Vec<usize> = (0..3)
                .filter(|&i| triangle.neighbours[i].is_some())
                .collect();
            let corner = |i: usize| mesh.points[triangle.corners[i]];
            match inner.as_slice() {
                [i] => {
                    let tip = graph.add(corner((i + 2) % 3), 0.0, NodeKind::Tip);
                    graph.link(tip, chords[t][*i]);
                }
                [i, j] => graph.link(chords[t][*i], chords[t][*j]),
                [_, _, _] => {
                    let centre = junction_centre([corner(0), corner(1), corner(2)]);
                    let node = graph.add(centre, boundary.distance(centre), NodeKind::Junction);
                    for &chord in &chords[t] {
                        graph.link(node, chord);
                    }
                }
                _ => {}
            }
        }
        graph
    }

    /// One chord node per inner edge, as `chords[triangle][edge]` (`usize::MAX` on the outline).
    fn chord_nodes(&mut self, mesh: &Mesh, boundary: &Boundary) -> Vec<[usize; 3]> {
        let mut chords = vec![[usize::MAX; 3]; mesh.triangles.len()];
        for (t, triangle) in mesh.triangles.iter().enumerate() {
            for i in 0..3 {
                let Some(other) = triangle.neighbours[i] else {
                    continue;
                };
                if other > t {
                    let a = mesh.points[triangle.corners[i]];
                    let b = mesh.points[triangle.corners[(i + 1) % 3]];
                    let sample = axis_point(boundary, a, b);
                    chords[t][i] = self.add(sample.point, sample.radius, NodeKind::Chord);
                } else {
                    let back = mesh.triangles[other]
                        .neighbours
                        .iter()
                        .position(|n| *n == Some(t));
                    if let Some(j) = back {
                        chords[t][i] = chords[other][j];
                    }
                }
            }
        }
        chords
    }

    fn add(&mut self, point: Point, radius: f64, kind: NodeKind) -> usize {
        self.samples.push(AxisSample { point, radius });
        self.kinds.push(kind);
        self.links.push(Vec::new());
        self.alive.push(true);
        self.samples.len() - 1
    }

    fn link(&mut self, a: usize, b: usize) {
        if a == usize::MAX || b == usize::MAX || a == b {
            return;
        }
        self.links[a].push(b);
        self.links[b].push(a);
    }

    /// The live neighbours of `node`.
    pub(super) fn neighbours(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        self.links[node].iter().copied().filter(|&n| self.alive[n])
    }

    /// The number of live neighbours of `node`.
    pub(super) fn degree(&self, node: usize) -> usize {
        self.neighbours(node).count()
    }
}

/// The circumcentre of a junction triangle, or the midpoint of its longest edge when the
/// circumcentre lies outside it (ADR 0063 §2).
fn junction_centre([a, b, c]: [Point; 3]) -> Point {
    let (ab, ac) = (b - a, c - a);
    let d = 2.0 * ab.cross(ac);
    let edges = [(a, b), (b, c), (c, a)];
    let longest = edges
        .iter()
        .copied()
        .max_by(|x, y| (x.1 - x.0).hypot2().total_cmp(&(y.1 - y.0).hypot2()))
        .unwrap_or((a, b));
    let midpoint = longest.0.midpoint(longest.1);
    if d == 0.0 {
        return midpoint;
    }
    let (ab2, ac2) = (ab.hypot2(), ac.hypot2());
    let centre = a + kurbo::Vec2::new(ac.y * ab2 - ab.y * ac2, ab.x * ac2 - ac.x * ab2) / d;
    let inside = [(a, b), (b, c), (c, a)]
        .iter()
        .all(|&(p, q)| (q - p).cross(centre - p) * d.signum() >= 0.0);
    if inside { centre } else { midpoint }
}
