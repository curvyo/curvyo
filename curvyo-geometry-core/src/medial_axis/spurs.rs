//! The spur prune of the raw axis graph (ADR 0063 §3): leaf runs and corner trees shorter than
//! twice their junction's local radius are removed, and a corner left behind is settled.

use kurbo::Point;

use super::AxisSample;
use super::boundary::Boundary;
use super::graph::Graph;

/// A settled corner's edges are measured as straight lines from one to this many junction
/// radii out.
const CORNER_RUN: f64 = 3.0;

/// Edges whose directions differ by less than this sine (30 degrees) form no corner.
const CORNER_SIN: f64 = 0.5;

/// A settled corner lies at most this many junction radii from the junction.
const CORNER_REACH: f64 = 1.5;

/// A leaf's run to the nearest node of another degree than two.
struct Spur {
    nodes: Vec<usize>,
    junction: Option<usize>,
    length: f64,
}

fn walk(graph: &Graph, leaf: usize) -> Spur {
    let mut nodes = vec![leaf];
    let mut length = 0.0;
    let mut previous = usize::MAX;
    let mut current = leaf;
    // A run from a leaf through degree-two nodes cannot come back on itself; the bound keeps a
    // malformed graph from spinning.
    for _ in 0..graph.samples.len() {
        let Some(next) = graph.neighbours(current).find(|&n| n != previous) else {
            return Spur {
                nodes,
                junction: None,
                length,
            };
        };
        length += (graph.samples[next].point - graph.samples[current].point).hypot();
        if graph.degree(next) != 2 {
            return Spur {
                nodes,
                junction: (graph.degree(next) >= 3).then_some(next),
                length,
            };
        }
        nodes.push(next);
        previous = current;
        current = next;
    }
    Spur {
        nodes,
        junction: None,
        length,
    }
}

/// Removes leaf runs shorter than twice their junction's clearance, shortest first, until none
/// is left; with `subtrees`, also the short trees of [`prune_short_subtrees`], once.
pub(super) fn prune_spurs(graph: &mut Graph, boundary: &Boundary, subtrees: bool) {
    // The trees are tried once, after the leaf runs: the leaf runs the trees leave are pruned
    // by the passes after.
    let mut trees_left = subtrees;
    loop {
        let reach = local_radii(graph);
        let mut candidates: Vec<(f64, (f64, f64), usize)> = (0..graph.samples.len())
            .filter(|&n| graph.alive[n] && graph.degree(n) == 1)
            .filter_map(|leaf| {
                let spur = walk(graph, leaf);
                let junction = spur.junction?;
                let point = graph.samples[leaf].point;
                (spur.length < 2.0 * reach[junction]).then_some((
                    spur.length,
                    (point.x, point.y),
                    leaf,
                ))
            })
            .collect();
        if candidates.is_empty() {
            if std::mem::take(&mut trees_left) && prune_short_subtrees(graph, boundary, &reach) {
                continue;
            }
            return;
        }
        candidates.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then(a.1.0.total_cmp(&b.1.0))
                .then(a.1.1.total_cmp(&b.1.1))
        });
        // All short spurs of one junction go together, so that one of two corner forks of a
        // flat end never becomes the chain's end once its sibling is gone; a junction keeps at
        // least one other edge.
        let mut removed_any = false;
        for (_, _, leaf) in &candidates {
            if !graph.alive[*leaf] || graph.degree(*leaf) != 1 {
                continue;
            }
            let Some(junction) = walk(graph, *leaf).junction else {
                continue;
            };
            let spurs: Vec<Spur> = graph
                .neighbours(junction)
                .collect::<Vec<_>>()
                .into_iter()
                .filter_map(|first| spur_from(graph, junction, first))
                .filter(|spur| spur.length < 2.0 * reach[junction])
                .collect();
            let keep_one = spurs.len() >= graph.degree(junction);
            let mut spurs = spurs;
            if keep_one {
                // Every edge is a short spur: the longest stays as the shape's axis.
                spurs.sort_by(|a, b| a.length.total_cmp(&b.length));
                spurs.pop();
            }
            let pruned = !spurs.is_empty();
            for spur in spurs {
                for node in spur.nodes {
                    graph.alive[node] = false;
                }
                removed_any = true;
            }
            if pruned && graph.degree(junction) == 2 {
                settle_corner(graph, boundary, junction, reach[junction]);
            }
        }
        if removed_any {
            continue;
        }
        if !(std::mem::take(&mut trees_left) && prune_short_subtrees(graph, boundary, &reach)) {
            return;
        }
    }
}

/// Removes the trees that hang off a junction and end within twice its local radius along
/// every path: the spur prune's rule for a whole tree. The round outer corner of a thick stroke
/// triangulates into such a tree of junction triangles near its arc; no single leaf run of it
/// is short against its own small junction. Returns whether anything was removed.
fn prune_short_subtrees(graph: &mut Graph, boundary: &Boundary, reach: &[f64]) -> bool {
    let mut removed = false;
    let mut seen = vec![0_usize; graph.samples.len()];
    let mut stamp = 0;
    for (junction, &radius) in reach.iter().enumerate() {
        if !graph.alive[junction] || graph.degree(junction) < 3 {
            continue;
        }
        let limit = 2.0 * radius;
        let firsts: Vec<usize> = graph.neighbours(junction).collect();
        let mut trees: Vec<Vec<usize>> = Vec::new();
        for &first in &firsts {
            stamp += 1;
            if let Some(tree) = short_tree(graph, junction, first, limit, &mut seen, stamp) {
                trees.push(tree);
            }
        }
        // A junction keeps at least two edges: the shape's axis runs through it.
        if firsts.len() - trees.len() < 2 {
            continue;
        }
        let pruned = !trees.is_empty();
        for tree in trees {
            for node in tree {
                graph.alive[node] = false;
            }
            removed = true;
        }
        if pruned && graph.degree(junction) == 2 {
            settle_corner(graph, boundary, junction, radius);
        }
    }
    removed
}

/// Moves a junction left with two edges after its corner tree was pruned to where the two edges'
/// straight runs meet: the corner of a round-joined L lies at the centre of its join, which the
/// medial axis passes at a distance (about 0.12 w for 90 degrees; ADR 0063 Decision 5 corners).
fn settle_corner(graph: &mut Graph, boundary: &Boundary, junction: usize, radius: f64) {
    let here = graph.samples[junction].point;
    let sides: Vec<usize> = graph.neighbours(junction).collect();
    let (Some((a_near, a1, a_bend)), Some((b_near, b1, b_bend))) = (
        corner_run(graph, junction, sides[0], radius),
        corner_run(graph, junction, sides[1], radius),
    ) else {
        return;
    };
    let (a0, b0) = (graph.samples[a_near].point, graph.samples[b_near].point);
    let (da, db) = (a0 - a1, b0 - b1);
    let cross = da.cross(db);
    if cross.abs() < CORNER_SIN * da.hypot() * db.hypot() {
        return;
    }
    let t = (b1 - a1).cross(db) / cross;
    let corner = a1 + da * t;
    if (corner - here).hypot() <= CORNER_REACH * radius {
        graph.samples[junction] = AxisSample {
            point: corner,
            radius: boundary.distance(corner),
        };
        for node in a_bend.into_iter().chain(b_bend) {
            graph.alive[node] = false;
        }
        for near in [a_near, b_near] {
            if !graph.links[junction].contains(&near) {
                graph.links[junction].push(near);
                graph.links[near].push(junction);
            }
        }
    }
}

/// One side of a settled corner, walked from `junction` through `first`: the first node a
/// junction radius out, the run's far point [`CORNER_RUN`] radii out, and the nodes before the
/// first, which bend round the corner. `None` when the run reaches another junction or an end
/// first, or comes back round to `junction` (a small closed axis loop: no straight run to
/// measure).
fn corner_run(
    graph: &Graph,
    junction: usize,
    first: usize,
    radius: f64,
) -> Option<(usize, Point, Vec<usize>)> {
    let here = graph.samples[junction].point;
    let (mut previous, mut current) = (junction, first);
    let mut near: Option<usize> = None;
    let mut bend = Vec::new();
    // `junction` has degree two, so a run of degree-two nodes can only come back to it; the
    // step bound also covers duplicated links.
    for _ in 0..graph.samples.len() {
        if current == junction {
            return None;
        }
        let point = graph.samples[current].point;
        let away = (point - here).hypot();
        if near.is_none() {
            if away >= radius {
                near = Some(current);
            } else {
                bend.push(current);
            }
        }
        if away >= CORNER_RUN * radius {
            return near.map(|n| (n, point, bend));
        }
        if graph.degree(current) != 2 {
            return None;
        }
        let next = graph.neighbours(current).find(|&n| n != previous)?;
        previous = current;
        current = next;
    }
    None
}

/// The nodes of the tree reached from `junction` through `first`, if it never comes back to
/// `junction` and every node lies within `limit` of it along the tree. `seen` marks the nodes
/// visited, by `stamp`, so that no call clears it.
fn short_tree(
    graph: &Graph,
    junction: usize,
    first: usize,
    limit: f64,
    seen: &mut [usize],
    stamp: usize,
) -> Option<Vec<usize>> {
    let mut nodes = vec![first];
    seen[first] = stamp;
    let mut stack = vec![(
        first,
        junction,
        (graph.samples[first].point - graph.samples[junction].point).hypot(),
    )];
    while let Some((node, from, length)) = stack.pop() {
        if length >= limit {
            return None;
        }
        for next in graph.neighbours(node) {
            if next == from {
                continue;
            }
            if next == junction || seen[next] == stamp {
                return None;
            }
            seen[next] = stamp;
            nodes.push(next);
            let step = (graph.samples[next].point - graph.samples[node].point).hypot();
            stack.push((next, node, length + step));
        }
    }
    Some(nodes)
}

/// The run from `junction` through `first` to a leaf, if it ends at a leaf.
fn spur_from(graph: &Graph, junction: usize, first: usize) -> Option<Spur> {
    let mut nodes = Vec::new();
    let (mut previous, mut current) = (junction, first);
    let mut length = (graph.samples[first].point - graph.samples[junction].point).hypot();
    // Bounded as `walk` is: a run that comes back to `junction` meets a node of degree three.
    for _ in 0..graph.samples.len() {
        nodes.push(current);
        match graph.degree(current) {
            1 => {
                return Some(Spur {
                    nodes,
                    junction: Some(junction),
                    length,
                });
            }
            2 => {
                let next = graph.neighbours(current).find(|&n| n != previous)?;
                length += (graph.samples[next].point - graph.samples[current].point).hypot();
                previous = current;
                current = next;
            }
            _ => return None,
        }
    }
    None
}

/// The local radius at every junction (live node of degree three or more): the largest clearance
/// of a live node whose disc holds the junction. A junction triangle in a rounded end sits near
/// the outline with a small clearance of its own, inside the end circle's disc.
fn local_radii(graph: &Graph) -> Vec<f64> {
    let mut radii: Vec<f64> = graph.samples.iter().map(|s| s.radius).collect();
    let live: Vec<usize> = (0..graph.samples.len())
        .filter(|&n| graph.alive[n])
        .collect();
    let widest = live
        .iter()
        .map(|&n| graph.samples[n].radius)
        .fold(0.0_f64, f64::max);
    if widest <= 0.0 {
        return radii;
    }
    let cell_of = |n: usize| {
        let p = graph.samples[n].point;
        #[allow(clippy::cast_possible_truncation)]
        ((p.x / widest).floor() as i64, (p.y / widest).floor() as i64)
    };
    let mut cells: std::collections::BTreeMap<(i64, i64), Vec<usize>> =
        std::collections::BTreeMap::new();
    for &n in &live {
        cells.entry(cell_of(n)).or_default().push(n);
    }
    for &j in &live {
        if graph.degree(j) < 3 {
            continue;
        }
        let (cx, cy) = cell_of(j);
        let here = graph.samples[j].point;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &k in cells
                    .get(&(cx + dx, cy + dy))
                    .map_or(&[][..], Vec::as_slice)
                {
                    let sample = graph.samples[k];
                    if (sample.point - here).hypot() < sample.radius && sample.radius > radii[j] {
                        radii[j] = sample.radius;
                    }
                }
            }
        }
    }
    radii
}
