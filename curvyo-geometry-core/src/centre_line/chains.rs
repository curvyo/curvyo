//! The chains of a medial axis (ADR 0063 §4): junctions closer than their radius merge, every
//! junction lets the pair of edges with the least change of direction and width continue, and
//! the chains that follow those pairs become the main path and its branches.

use kurbo::{Point, Vec2};

use super::junctions::{
    Junction, Measure, Side, best_pair, crest_tip, grid_key, junction_index, measure,
    merge_junctions, outward,
};

use crate::medial_axis::{AxisSample, MedialAxis};

/// How a chain ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ChainEnd {
    /// A free end of the shape.
    Free,
    /// An end at a junction where the chain did not continue: a branch start.
    Junction {
        /// The merged junction's index.
        junction: usize,
        /// The junction's centre.
        centre: Point,
        /// The junction zone's radius.
        radius: f64,
        /// Unit direction of the edge measure, pointing away from the junction.
        direction: Vec2,
        /// The width of the edge measure.
        width: f64,
        /// The chain passing through the junction, if any.
        parent: Option<usize>,
    },
    /// An end where the chain meets a part kept filled at Thick parts.
    Cut,
}

/// A run of the axis that becomes one path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Chain {
    /// The samples, junction zones left out.
    pub(crate) samples: Vec<AxisSample>,
    /// Whether the chain is a closed loop.
    pub(crate) closed: bool,
    /// How the first sample's end ends.
    pub(crate) start: ChainEnd,
    /// How the last sample's end ends.
    pub(crate) end: ChainEnd,
    /// Whether this is the main path of its region.
    pub(crate) main: bool,
}

impl Chain {
    /// The polyline length.
    pub(crate) fn length(&self) -> f64 {
        self.samples
            .windows(2)
            .map(|w| (w[1].point - w[0].point).hypot())
            .sum()
    }
}

/// The chains of `axis`, main path marked.
pub(crate) fn chains(axis: &MedialAxis) -> Vec<Chain> {
    let degree = vertex_degrees(axis);
    let junctions = merge_junctions(axis, &degree);
    let junction_of = junction_index(axis, &junctions);
    // An edge between two members of one merged junction is part of the junction, unless it
    // leaves the zone: then it is a loop around a hole, back to the same junction.
    let edges: Vec<usize> = (0..axis.edges.len())
        .filter(|&e| {
            let edge = &axis.edges[e];
            match (junction_of[edge.from], junction_of[edge.to]) {
                (Some(a), Some(b)) if a == b => edge
                    .samples
                    .iter()
                    .any(|s| !junctions[a].covers(axis, s.point)),
                _ => true,
            }
        })
        .collect();
    let mut sides: Vec<Vec<Side>> = vec![Vec::new(); junctions.len()];
    for &e in &edges {
        if let Some(j) = junction_of[axis.edges[e].from] {
            sides[j].push(Side {
                edge: e,
                at_start: true,
            });
        }
        if let Some(j) = junction_of[axis.edges[e].to] {
            sides[j].push(Side {
                edge: e,
                at_start: false,
            });
        }
    }
    let far = |side: Side| {
        let edge = &axis.edges[side.edge];
        if side.at_start { edge.to } else { edge.from }
    };
    let measures: Vec<Vec<Measure>> = sides
        .iter()
        .zip(&junctions)
        .map(|(list, junction)| {
            list.iter()
                .map(|&s| measure(axis, junction, s, degree[far(s)] == 1))
                .collect()
        })
        .collect();
    let mut pairs: Vec<Option<(Side, Side)>> = sides
        .iter()
        .zip(&measures)
        .map(|(list, m)| best_pair(list, m))
        .collect();
    // A sharp crest runs through its tip: its arms pair, its spur is walked as the tip point.
    let mut used = vec![false; axis.edges.len()];
    let mut tips: Vec<Option<AxisSample>> = vec![None; junctions.len()];
    for (j, junction) in junctions.iter().enumerate() {
        let tip_of = |side: Side| {
            let samples = &axis.edges[side.edge].samples;
            if side.at_start {
                samples[samples.len() - 1]
            } else {
                samples[0]
            }
        };
        if let Some((spur, a, b)) = crest_tip(junction, &sides[j], &measures[j], tip_of) {
            pairs[j] = Some((a, b));
            used[spur.edge] = true;
            tips[j] = Some(AxisSample {
                point: tip_of(spur).point,
                radius: 0.0,
            });
        }
    }
    let mut walker = Walker {
        axis,
        junctions: &junctions,
        junction_of: &junction_of,
        sides: &sides,
        measures: &measures,
        pairs: &pairs,
        tips: &tips,
        used,
        chains: Vec::new(),
        through: vec![None; junctions.len()],
    };
    walker.walk_all(&edges);
    let Walker {
        mut chains,
        through,
        ..
    } = walker;
    for ring in &axis.rings {
        chains.push(Chain {
            samples: ring.clone(),
            closed: true,
            start: ChainEnd::Free,
            end: ChainEnd::Free,
            main: false,
        });
    }
    attach_parents(&mut chains, &through);
    mark_main(&mut chains);
    chains
}

/// The number of edge ends at each vertex.
fn vertex_degrees(axis: &MedialAxis) -> Vec<usize> {
    let mut degree = vec![0; axis.vertices.len()];
    for edge in &axis.edges {
        degree[edge.from] += 1;
        degree[edge.to] += 1;
    }
    degree
}

/// Follows the pairs from edge to edge.
struct Walker<'a> {
    axis: &'a MedialAxis,
    junctions: &'a [Junction],
    junction_of: &'a [Option<usize>],
    sides: &'a [Vec<Side>],
    measures: &'a [Vec<Measure>],
    pairs: &'a [Option<(Side, Side)>],
    /// The tip each crest junction's path runs through, radius 0.
    tips: &'a [Option<AxisSample>],
    used: Vec<bool>,
    chains: Vec<Chain>,
    /// The chain that continues through each junction.
    through: Vec<Option<usize>>,
}

impl Walker<'_> {
    fn vertex_at(&self, side: Side) -> usize {
        let edge = &self.axis.edges[side.edge];
        if side.at_start { edge.from } else { edge.to }
    }

    fn partner(&self, side: Side) -> Option<Side> {
        let j = self.junction_of[self.vertex_at(side)]?;
        let (a, b) = self.pairs[j]?;
        if a == side {
            Some(b)
        } else if b == side {
            Some(a)
        } else {
            None
        }
    }

    fn end_of(&self, side: Side) -> ChainEnd {
        let Some(j) = self.junction_of[self.vertex_at(side)] else {
            return ChainEnd::Free;
        };
        // invariant: every side at a junction is in that junction's list (built from the same
        // edges); 0 is never reached.
        let index = self.sides[j].iter().position(|&s| s == side).unwrap_or(0);
        let measure = self.measures[j][index];
        ChainEnd::Junction {
            junction: j,
            centre: self.junctions[j].centre,
            radius: self.junctions[j].radius,
            direction: measure.direction,
            width: measure.width,
            parent: None,
        }
    }

    fn walk_all(&mut self, edges: &[usize]) {
        // Open chains start at a free or unpaired end.
        let mut starts: Vec<Side> = Vec::new();
        for &e in edges {
            for at_start in [true, false] {
                let side = Side { edge: e, at_start };
                if self.partner(side).is_none() {
                    starts.push(side);
                }
            }
        }
        for side in starts {
            if !self.used[side.edge] {
                self.walk(side, false);
            }
        }
        for &e in edges {
            if !self.used[e] {
                self.walk(
                    Side {
                        edge: e,
                        at_start: true,
                    },
                    true,
                );
            }
        }
    }

    /// Walks from `first` until an unpaired end or back to the start.
    fn walk(&mut self, first: Side, closed: bool) {
        let index = self.chains.len();
        let mut samples: Vec<AxisSample> = Vec::new();
        let mut side = first;
        let end;
        // The junctions this chain passes, with what they held before, so a chain too short to
        // keep leaves no parent index behind.
        let mut passed: Vec<(usize, Option<usize>)> = Vec::new();
        // Where a crest's tip was put in.
        let mut tip_at: Vec<usize> = Vec::new();
        loop {
            self.used[side.edge] = true;
            let piece = outward(self.axis, side);
            let exit = Side {
                edge: side.edge,
                at_start: !side.at_start,
            };
            // Only the junctions at the edge's own ends can cover its samples.
            let ends = [
                self.junction_of[self.vertex_at(side)],
                self.junction_of[self.vertex_at(exit)],
            ];
            for sample in piece {
                if !self.in_zone(&ends, sample.point) {
                    samples.push(sample);
                }
            }
            match self.partner(exit) {
                Some(next) if !self.used[next.edge] => {
                    if let Some(j) = self.junction_of[self.vertex_at(exit)] {
                        passed.push((j, self.through[j].replace(index)));
                        if let Some(tip) = self.tips[j] {
                            tip_at.push(samples.len());
                            samples.push(tip);
                        }
                    }
                    side = next;
                }
                Some(_) => {
                    if let Some(j) = self.junction_of[self.vertex_at(exit)] {
                        passed.push((j, self.through[j].replace(index)));
                    }
                    end = ChainEnd::Free;
                    break;
                }
                None => {
                    end = self.end_of(exit);
                    break;
                }
            }
        }
        for &at in tip_at.iter().rev() {
            straight_to_tip(&mut samples, at, closed);
        }
        samples.dedup_by(|a, b| a.point == b.point);
        let start = if closed {
            ChainEnd::Free
        } else {
            self.end_of(first)
        };
        let mut chain = Chain {
            samples,
            closed,
            start,
            end: if closed { ChainEnd::Free } else { end },
            main: false,
        };
        if chain.samples.len() < 2 {
            for (j, before) in passed.into_iter().rev() {
                self.through[j] = before;
            }
            return;
        }
        if closed {
            normalise_ring(&mut chain.samples);
        } else if grid_key(chain.samples[chain.samples.len() - 1].point)
            < grid_key(chain.samples[0].point)
        {
            chain.samples.reverse();
            std::mem::swap(&mut chain.start, &mut chain.end);
        }
        self.chains.push(chain);
    }

    fn in_zone(&self, ends: &[Option<usize>; 2], point: Point) -> bool {
        ends.iter()
            .flatten()
            .any(|&j| self.junctions[j].covers(self.axis, point))
    }
}

/// Samples spaced at most this share of the wider arm's radius on a crest's straight legs.
const TIP_STEP_SHARE: f64 = 0.5;
/// The most samples on one leg.
const TIP_STEPS_MAX: usize = 32;

/// Puts samples on the straight legs from the arm before `samples[at]` (a crest's tip, radius 0)
/// to it and on to the arm after it, their radii falling linearly to 0 at the tip: the path's
/// edges then run along the crest's straight outer edges into the point.
fn straight_to_tip(samples: &mut Vec<AxisSample>, at: usize, closed: bool) {
    let n = samples.len();
    let neighbour = |k: Option<usize>| k.map(|k| samples[k]);
    let before = neighbour(at.checked_sub(1).or(closed.then(|| n - 1)));
    let after = neighbour((at + 1 < n).then_some(at + 1).or(closed.then_some(0)));
    let tip = samples[at];
    let leg = |from: AxisSample| -> Vec<AxisSample> {
        let length = (tip.point - from.point).hypot();
        let step = (TIP_STEP_SHARE * from.radius).max(1e-3);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = ((length / step).ceil() as usize).clamp(1, TIP_STEPS_MAX);
        (1..count)
            .map(|k| {
                #[allow(clippy::cast_precision_loss)]
                let t = k as f64 / count as f64;
                AxisSample {
                    point: from.point.lerp(tip.point, t),
                    radius: from.radius * (1.0 - t),
                }
            })
            .collect()
    };
    if let Some(after) = after {
        let mut out = leg(after);
        out.reverse();
        let tail = samples.split_off(at + 1);
        samples.extend(out);
        samples.extend(tail);
    }
    if let Some(before) = before {
        samples.splice(at..at, leg(before));
    }
}

/// A closed chain starts at its smallest grid point and runs with positive orientation.
fn normalise_ring(samples: &mut [AxisSample]) {
    let mut twice_area = 0.0;
    for i in 0..samples.len() {
        let (a, b) = (samples[i].point, samples[(i + 1) % samples.len()].point);
        twice_area += a.x * b.y - b.x * a.y;
    }
    if twice_area < 0.0 {
        samples.reverse();
    }
    let smallest = (0..samples.len())
        .min_by_key(|&i| grid_key(samples[i].point))
        .unwrap_or(0);
    samples.rotate_left(smallest);
}

/// Each junction end attaches to the chain passing through that junction.
fn attach_parents(chains: &mut [Chain], through: &[Option<usize>]) {
    for (index, chain) in chains.iter_mut().enumerate() {
        for end in [&mut chain.start, &mut chain.end] {
            if let ChainEnd::Junction {
                junction, parent, ..
            } = end
            {
                *parent = through[*junction].filter(|&p| p != index);
            }
        }
    }
}

/// The main path: the longest chain with no junction end, else the longest; ties to the lower
/// start.
fn mark_main(chains: &mut [Chain]) {
    let pick = |filter: &dyn Fn(&Chain) -> bool, chains: &[Chain]| {
        let mut best: Option<usize> = None;
        for (i, chain) in chains.iter().enumerate() {
            if !filter(chain) {
                continue;
            }
            best = match best {
                None => Some(i),
                Some(b) => {
                    let (l, bl) = (chain.length(), chains[b].length());
                    if l > bl + 1e-9
                        || ((l - bl).abs() <= 1e-9
                            && grid_key(chain.samples[0].point)
                                < grid_key(chains[b].samples[0].point))
                    {
                        Some(i)
                    } else {
                        Some(b)
                    }
                }
            };
        }
        best
    };
    let free = |c: &Chain| {
        !matches!(c.start, ChainEnd::Junction { .. }) && !matches!(c.end, ChainEnd::Junction { .. })
    };
    if let Some(i) = pick(&free, chains).or_else(|| pick(&|_| true, chains)) {
        chains[i].main = true;
    }
}
