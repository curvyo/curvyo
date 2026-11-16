//! The cleaned axis graph cut into edges between junctions and ends, and closed rings.

use super::graph::Graph;
use super::{AxisEdge, AxisSample, MedialAxis};

/// The edges between the vertices (live nodes of degree other than two) and the rings.
pub(super) fn extract(graph: &Graph) -> MedialAxis {
    let count = graph.samples.len();
    let mut vertex_of = vec![usize::MAX; count];
    let mut axis = MedialAxis::default();
    for (node, vertex) in vertex_of.iter_mut().enumerate() {
        if graph.alive[node] && graph.degree(node) != 2 && graph.degree(node) != 0 {
            *vertex = axis.vertices.len();
            axis.vertices.push(graph.samples[node]);
        }
    }
    let mut used = vec![false; count];
    for node in 0..count {
        if vertex_of[node] == usize::MAX {
            continue;
        }
        let mut starts: Vec<usize> = graph.neighbours(node).collect();
        starts.sort_unstable();
        starts.dedup();
        for first in starts {
            if vertex_of[first] != usize::MAX {
                if node < first {
                    axis.edges.push(AxisEdge {
                        from: vertex_of[node],
                        to: vertex_of[first],
                        samples: vec![graph.samples[node], graph.samples[first]],
                    });
                }
                continue;
            }
            if used[first] {
                continue;
            }
            let mut samples = vec![graph.samples[node]];
            let (mut previous, mut current) = (node, first);
            // From a vertex through degree-two nodes the run meets a vertex or an end, never
            // itself; `used` bounds it all the same.
            while vertex_of[current] == usize::MAX && !used[current] {
                used[current] = true;
                samples.push(graph.samples[current]);
                let Some(next) = graph.neighbours(current).find(|&n| n != previous) else {
                    break;
                };
                previous = current;
                current = next;
            }
            if vertex_of[current] != usize::MAX {
                samples.push(graph.samples[current]);
                axis.edges.push(AxisEdge {
                    from: vertex_of[node],
                    to: vertex_of[current],
                    samples,
                });
            }
        }
    }
    axis.rings = rings(graph, &mut used, &vertex_of);
    axis
}

/// The loops of live degree-two nodes no edge has visited, each from its smallest point.
fn rings(graph: &Graph, used: &mut [bool], vertex_of: &[usize]) -> Vec<Vec<AxisSample>> {
    let mut rings = Vec::new();
    for start in 0..graph.samples.len() {
        if used[start]
            || !graph.alive[start]
            || vertex_of[start] != usize::MAX
            || graph.degree(start) != 2
        {
            continue;
        }
        let mut ring = Vec::new();
        let (mut previous, mut current) = (usize::MAX, start);
        loop {
            used[current] = true;
            ring.push(graph.samples[current]);
            let Some(next) = graph
                .neighbours(current)
                .find(|&n| n != previous && (!used[n] || n == start))
            else {
                break;
            };
            if next == start {
                break;
            }
            previous = current;
            current = next;
        }
        if ring.len() >= 3 {
            let smallest = (0..ring.len())
                .min_by(|&a, &b| {
                    let (p, q) = (ring[a].point, ring[b].point);
                    p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y))
                })
                .unwrap_or(0);
            ring.rotate_left(smallest);
            rings.push(ring);
        }
    }
    rings
}
