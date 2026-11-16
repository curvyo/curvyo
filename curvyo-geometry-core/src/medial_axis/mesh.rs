//! The constrained Delaunay triangulation of a region, densified in place until every boundary
//! edge is no longer than the height of its triangle, reduced to the triangles inside the region.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use kurbo::Point;
use spade::handles::{
    DirectedEdgeHandle, FaceHandle, FixedFaceHandle, FixedVertexHandle, InnerTag, PossiblyOuterTag,
};
use spade::{CdtEdge, ConstrainedDelaunayTriangulation, Point2, Triangulation};

use super::{CentreLineError, Region};
use crate::boolean_grid::GRID_MM;

/// The most densify rounds (ADR 0063 §2 as amended 2026-11-16); most outlines need two to four.
const DENSIFY_ROUNDS: usize = 8;
/// The most pieces one round cuts an edge into.
const MOST_PIECES: usize = 8;
/// The shortest piece densify cuts, in grid units: the floor of `T(w)`, 0.05 mm (a shorter piece
/// cannot move the axis by a tolerance).
const SHORTEST_PIECE_UNITS: f64 = 50.0;
/// An outline edge of an end triangle this many times longer than the triangle's other outline
/// edge is measured against the triangle across the chord.
const EAR_RATIO: f64 = 2.0;

/// The triangles inside a region.
#[derive(Debug, Clone)]
pub(super) struct Mesh {
    /// Vertex positions in millimetres.
    pub(super) points: Vec<Point>,
    /// Triangles inside the region.
    pub(super) triangles: Vec<Triangle>,
}

/// One triangle inside the region.
#[derive(Debug, Clone, Copy)]
pub(super) struct Triangle {
    /// Its corners, counter-clockwise in grid coordinates.
    pub(super) corners: [usize; 3],
    /// The triangle across the edge from `corners[i]` to `corners[(i + 1) % 3]`; `None` where that
    /// edge is the outline.
    pub(super) neighbours: [Option<usize>; 3],
}

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

/// Triangulates `region`, densifying its outline for at most [`DENSIFY_ROUNDS`] rounds. The
/// CDT is built once; each round splits the planned outline edges in place.
pub(super) fn triangulate(region: &Region) -> Result<Mesh, CentreLineError> {
    #[allow(clippy::cast_precision_loss)] // grid units are below 10^10, exact in an f64
    let rings: Vec<Vec<[f64; 2]>> = region
        .rings()
        .map(|ring| ring.iter().map(|p| [p.x as f64, p.y as f64]).collect())
        .collect();
    let mut cdt = build(&rings)?;
    for _ in 0..DENSIFY_ROUNDS {
        let inside = inside_faces(&cdt);
        let parts = densify_plan(&cdt, &inside);
        if parts.is_empty() {
            return Ok(mesh_of(&cdt, &inside));
        }
        split(&mut cdt, &parts)?;
    }
    let inside = inside_faces(&cdt);
    Ok(mesh_of(&cdt, &inside))
}

/// The CDT of `rings`, every ring edge a constraint; duplicate positions are shared.
fn build(rings: &[Vec<[f64; 2]>]) -> Result<Cdt, CentreLineError> {
    let mut index: BTreeMap<(u64, u64), usize> = BTreeMap::new();
    let mut vertices: Vec<Point2<f64>> = Vec::new();
    let mut edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    for ring in rings {
        let ids: Vec<usize> = ring
            .iter()
            .map(|&[x, y]| {
                let key = ((x + 0.0).to_bits(), (y + 0.0).to_bits());
                *index.entry(key).or_insert_with(|| {
                    vertices.push(Point2::new(x, y));
                    vertices.len() - 1
                })
            })
            .collect();
        for i in 0..ids.len() {
            let (a, b) = (ids[i], ids[(i + 1) % ids.len()]);
            if a != b {
                edges.insert((a.min(b), a.max(b)));
            }
        }
    }
    let edges: Vec<[usize; 2]> = edges.into_iter().map(|(a, b)| [a, b]).collect();
    let mut conflict = false;
    let cdt = Cdt::try_bulk_load_cdt(vertices, edges, |_| conflict = true)
        .map_err(|_| CentreLineError::Triangulation)?;
    if conflict {
        return Err(CentreLineError::Triangulation);
    }
    Ok(cdt)
}

/// Cuts each planned outline edge, by its sorted vertex pair, into equal pieces in place: the
/// constraint is removed, the cut points are inserted and the pieces become constraints.
fn split(cdt: &mut Cdt, parts: &BTreeMap<(usize, usize), usize>) -> Result<(), CentreLineError> {
    for (&(a, b), &pieces) in parts {
        let (from, to) = (
            FixedVertexHandle::from_index(a),
            FixedVertexHandle::from_index(b),
        );
        let Some(edge) = cdt.get_edge_from_neighbors(from, to) else {
            continue;
        };
        let edge = edge.as_undirected().fix();
        let (p, q) = (cdt.vertex(from).position(), cdt.vertex(to).position());
        cdt.remove_constraint_edge(edge);
        let mut previous = from;
        for k in 1..=pieces {
            let next = if k == pieces {
                to
            } else {
                #[allow(clippy::cast_precision_loss)]
                let t = k as f64 / pieces as f64;
                cdt.insert(Point2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t))
                    .map_err(|_| CentreLineError::Triangulation)?
            };
            if previous != next && cdt.try_add_constraint(previous, next).is_empty() {
                return Err(CentreLineError::Triangulation);
            }
            previous = next;
        }
    }
    Ok(())
}

/// Which faces lie inside the region: an odd number of constraints crossed from outside.
fn inside_faces(cdt: &Cdt) -> Vec<bool> {
    let mut faces: Vec<Option<FaceRef<'_>>> = vec![None; cdt.num_all_faces()];
    for face in cdt.all_faces() {
        faces[face.fix().index()] = Some(face);
    }
    let mut depth: Vec<u32> = vec![u32::MAX; faces.len()];
    let mut queue: VecDeque<(usize, u32)> = VecDeque::new();
    let outer = cdt.outer_face().fix().index();
    depth[outer] = 0;
    queue.push_back((outer, 0));
    while let Some((index, d)) = queue.pop_front() {
        if depth[index] < d {
            continue;
        }
        let Some(face) = faces[index] else {
            continue;
        };
        let mut visit = |edge: EdgeRef<'_>| {
            let next = edge.rev().face().fix().index();
            let step = u32::from(edge.as_undirected().is_constraint_edge());
            let candidate = d + step;
            if candidate < depth[next] {
                depth[next] = candidate;
                if step == 0 {
                    queue.push_front((next, candidate));
                } else {
                    queue.push_back((next, candidate));
                }
            }
        };
        match face.as_inner() {
            Some(inner) => inner.adjacent_edges().into_iter().for_each(&mut visit),
            None => face_edges(face).into_iter().for_each(&mut visit),
        }
    }
    depth.iter().map(|d| *d != u32::MAX && d % 2 == 1).collect()
}

type FaceRef<'a> = FaceHandle<'a, PossiblyOuterTag, Point2<f64>, (), CdtEdge<()>, ()>;
type EdgeRef<'a> = DirectedEdgeHandle<'a, Point2<f64>, (), CdtEdge<()>, ()>;

/// The directed edges around a face (outer or inner), each with the face on its left.
fn face_edges(face: FaceRef<'_>) -> Vec<EdgeRef<'_>> {
    let Some(start) = face.adjacent_edge() else {
        return Vec::new();
    };
    let mut edges = vec![start];
    let mut edge = start.next();
    while edge.fix() != start.fix() {
        edges.push(edge);
        edge = edge.next();
    }
    edges
}

/// How many pieces each too long outline edge is cut into, by its sorted vertex pair.
fn densify_plan(cdt: &Cdt, inside: &[bool]) -> BTreeMap<(usize, usize), usize> {
    let mut parts: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for face in cdt.inner_faces() {
        if !inside[face.fix().index()] {
            continue;
        }
        let outline: Vec<_> = face
            .adjacent_edges()
            .into_iter()
            .filter(|e| e.as_undirected().is_constraint_edge())
            .collect();
        // An end triangle (two outline edges) has its apex next to the edge, so its height says
        // nothing about the stroke. On a smooth or jagged outline its outline edges are alike
        // and need no cut; an outline edge much longer than the other (a long side meeting a
        // round end) is measured against the triangle across the end triangle's chord instead.
        if outline.len() >= 2 {
            let length_of = |e: &EdgeRef<'_>| {
                let (from, to) = (e.from().position(), e.to().position());
                length_between(from, to)
            };
            let shortest = outline.iter().map(length_of).fold(f64::INFINITY, f64::min);
            let across = face
                .adjacent_edges()
                .into_iter()
                .find(|e| !e.as_undirected().is_constraint_edge())
                .and_then(|chord| {
                    let other = chord.rev().face().as_inner()?;
                    inside[other.fix().index()]
                        .then(|| 2.0 * other.area().abs() / length_of(&chord))
                });
            if let Some(height) = across {
                for edge in &outline {
                    let length = length_of(edge);
                    if length > EAR_RATIO * shortest {
                        plan(&mut parts, edge, length, height);
                    }
                }
            }
            continue;
        }
        let twice_area = 2.0 * face.area().abs();
        for edge in face.adjacent_edges() {
            if !edge.as_undirected().is_constraint_edge() {
                continue;
            }
            let (from, to) = (edge.from().position(), edge.to().position());
            let length = length_between(from, to);
            plan(&mut parts, &edge, length, twice_area / length);
        }
    }
    parts
}

/// Plans `edge` (of `length`) in pieces no longer than `height`, none under
/// [`SHORTEST_PIECE_UNITS`] and at most [`MOST_PIECES`] in one round: the height of a sliver
/// says little, and the next round measures the pieces again.
fn plan(parts: &mut BTreeMap<(usize, usize), usize>, edge: &EdgeRef<'_>, length: f64, height: f64) {
    if length <= height {
        return;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let wanted = (length / height).ceil() as usize;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let allowed = (length / SHORTEST_PIECE_UNITS).floor() as usize;
    let pieces = wanted.min(allowed).min(MOST_PIECES);
    if pieces >= 2 {
        let (a, b) = (edge.from().fix().index(), edge.to().fix().index());
        let entry = parts.entry((a.min(b), a.max(b))).or_insert(0);
        *entry = (*entry).max(pieces);
    }
}

/// The inside triangles of `cdt` with their neighbours across non-constraint edges.
fn mesh_of(cdt: &Cdt, inside: &[bool]) -> Mesh {
    let points = cdt
        .vertices()
        .map(|v| {
            let p = v.position();
            Point::new(p.x * GRID_MM, p.y * GRID_MM)
        })
        .collect();
    let mut slot: Vec<Option<usize>> = vec![None; inside.len()];
    let faces: Vec<FixedFaceHandle<InnerTag>> = cdt
        .fixed_inner_faces()
        .filter(|f| inside[f.index()])
        .collect();
    for (i, face) in faces.iter().enumerate() {
        slot[face.index()] = Some(i);
    }
    let triangles = faces
        .iter()
        .map(|&fixed| {
            let face = cdt.face(fixed);
            let edges = face.adjacent_edges();
            let corners = edges.map(|e| e.from().fix().index());
            let neighbours = edges.map(|e| {
                if e.as_undirected().is_constraint_edge() {
                    None
                } else {
                    slot[e.rev().face().fix().index()]
                }
            });
            Triangle {
                corners,
                neighbours,
            }
        })
        .collect();
    Mesh { points, triangles }
}

/// The length of an edge as `sqrt(dx² + dy²)`: `f64::hypot` comes from the platform's libm and
/// may differ in the last bit between native and wasm, and the length sets the densify piece
/// count, so the topology (ADR 0063 §9).
fn length_between(from: Point2<f64>, to: Point2<f64>) -> f64 {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    (dx * dx + dy * dy).sqrt()
}
