//! Which pairs of pieces get a bridge (`specs/0156-fused-lettering` criteria 7 and 15, `adrs.md`
//! decision 3 step 5): every dot to its nearest non-dot within Reach, then a minimum spanning
//! forest of the remaining pieces, each edge tested for obstruction only when it would be taken.

use std::cmp::Ordering;

use crate::fuse::gap::Gap;

/// A candidate bridge between the pieces `a < b`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) gap: Gap,
}

/// The forest's order: exact distance, then the gap point on `a` (x, y), then the indices.
pub(crate) fn order(x: &Edge, y: &Edge) -> Ordering {
    x.gap
        .distance
        .total_cmp(&y.gap.distance)
        .then_with(|| x.gap.on_a.cmp_xy(y.gap.on_a))
        .then_with(|| (x.a, x.b).cmp(&(y.a, y.b)))
}

/// Union-find over piece indices.
struct Sets {
    parent: Vec<usize>,
}

impl Sets {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Joins the sets of `a` and `b`; false when they were one already.
    fn join(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        let (low, high) = (ra.min(rb), ra.max(rb));
        self.parent[high] = low;
        true
    }
}

/// The bridges to make, in the order they were taken. `edges` are the pairs within Reach, `dots`
/// which pieces are dots; `obstructed` says whether an edge's band would cross a third piece.
pub(crate) fn choose(
    count: usize,
    dots: &[bool],
    mut edges: Vec<Edge>,
    mut obstructed: impl FnMut(&Edge) -> bool,
) -> Vec<Edge> {
    edges.sort_by(order);
    let mut sets = Sets::new(count);
    let mut taken: Vec<Edge> = Vec::new();
    let mut attached = vec![false; count];
    // Rule 7a: each dot to its nearest non-dot.
    for dot in (0..count).filter(|&i| dots[i]) {
        let host = edges.iter().find(|edge| {
            let other = if edge.a == dot { edge.b } else { edge.a };
            (edge.a == dot || edge.b == dot) && !dots[other] && !obstructed(edge)
        });
        if let Some(edge) = host {
            sets.join(edge.a, edge.b);
            attached[dot] = true;
            taken.push(*edge);
        }
    }
    // Rule 7b: the spanning forest over the rest.
    for edge in &edges {
        if attached[edge.a] || attached[edge.b] {
            continue;
        }
        if sets.find(edge.a) == sets.find(edge.b) || obstructed(edge) {
            continue;
        }
        sets.join(edge.a, edge.b);
        taken.push(*edge);
    }
    taken
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuse::measure::P;

    fn edge(a: usize, b: usize, distance: f64) -> Edge {
        let zero = P::new(0.0, 0.0);
        Edge {
            a,
            b,
            gap: Gap {
                distance,
                on_a: P::new(f64::from(u32::try_from(a).unwrap()), 0.0),
                normal_a: zero,
                on_b: zero,
                normal_b: zero,
            },
        }
    }

    fn pairs(edges: &[Edge]) -> Vec<(usize, usize)> {
        edges.iter().map(|e| (e.a, e.b)).collect()
    }

    /// Criterion 10, rule 7a: the dots of a ü go to the u, never to each other.
    #[test]
    fn dots_attach_to_the_nearest_non_dot_and_make_no_cycle() {
        let edges = vec![edge(0, 1, 1.0), edge(0, 2, 3.0), edge(1, 2, 3.0)];
        let taken = choose(3, &[true, true, false], edges, |_| false);
        assert_eq!(pairs(&taken), vec![(0, 2), (1, 2)]);
    }

    /// Criterion 15: an obstructed edge is skipped and the pieces join through the middle one.
    #[test]
    fn an_obstructed_edge_is_skipped() {
        let edges = vec![edge(0, 2, 1.0), edge(0, 1, 2.0), edge(1, 2, 2.0)];
        let taken = choose(3, &[false; 3], edges, |e| (e.a, e.b) == (0, 2));
        assert_eq!(pairs(&taken), vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn the_forest_takes_the_shortest_edges_without_cycles() {
        let edges = vec![
            edge(0, 1, 1.0),
            edge(1, 2, 1.0),
            edge(0, 2, 1.5),
            edge(3, 4, 9.0),
        ];
        let taken = choose(5, &[false; 5], edges, |_| false);
        assert_eq!(pairs(&taken), vec![(0, 1), (1, 2), (3, 4)]);
    }
}
