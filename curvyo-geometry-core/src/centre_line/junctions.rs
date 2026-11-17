//! The junctions of a medial axis (ADR 0063 §4): vertices closer than their radius merge, each
//! edge end is measured by its direction and width, and the pair with the least change
//! continues.

use kurbo::{Point, Vec2};

use crate::medial_axis::{AxisSample, MedialAxis};

/// Pair costs within this of each other count as equal.
/// An edge is measured on its samples up to this many junction radii out (ADR 0063 §4).
const MEASURE_BAND: f64 = 3.0;

/// Pair costs within this of each other count as equal.
pub(super) const COST_TIE: f64 = 0.02;

/// A merged junction.
pub(super) struct Junction {
    pub(super) centre: Point,
    pub(super) radius: f64,
    pub(super) members: Vec<usize>,
}

impl Junction {
    pub(super) fn covers(&self, axis: &MedialAxis, point: Point) -> bool {
        self.members
            .iter()
            .any(|&m| (point - axis.vertices[m].point).hypot() < axis.vertices[m].radius)
    }
}

/// One end of an edge at a junction: the edge and whether its samples start there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Side {
    pub(super) edge: usize,
    pub(super) at_start: bool,
}

/// The edge's samples oriented away from `side`'s end.
pub(super) fn outward(axis: &MedialAxis, side: Side) -> Vec<AxisSample> {
    let mut samples = axis.edges[side.edge].samples.clone();
    if !side.at_start {
        samples.reverse();
    }
    samples
}

/// Junction vertices (three or more ends) merged when closer than the larger of their radii.
pub(super) fn merge_junctions(axis: &MedialAxis, degree: &[usize]) -> Vec<Junction> {
    let ids: Vec<usize> = (0..axis.vertices.len())
        .filter(|&v| degree[v] >= 3)
        .collect();
    let mut parent: Vec<usize> = (0..ids.len()).collect();
    // Pairs closer than the larger radius, found by a sweep over x.
    let widest = ids
        .iter()
        .map(|&v| axis.vertices[v].radius)
        .fold(0.0_f64, f64::max);
    let mut order: Vec<usize> = (0..ids.len()).collect();
    order.sort_by(|&a, &b| {
        axis.vertices[ids[a]]
            .point
            .x
            .total_cmp(&axis.vertices[ids[b]].point.x)
    });
    for (k, &a) in order.iter().enumerate() {
        let p = axis.vertices[ids[a]];
        for &b in &order[k + 1..] {
            let q = axis.vertices[ids[b]];
            if q.point.x - p.point.x >= widest {
                break;
            }
            if (p.point - q.point).hypot() < p.radius.max(q.radius) {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut junctions: Vec<Junction> = Vec::new();
    let mut slot = vec![usize::MAX; ids.len()];
    for (i, &id) in ids.iter().enumerate() {
        let r = root(&mut parent, i);
        if slot[r] == usize::MAX {
            slot[r] = junctions.len();
            junctions.push(Junction {
                centre: axis.vertices[id].point,
                radius: axis.vertices[id].radius,
                members: Vec::new(),
            });
        }
        let junction = &mut junctions[slot[r]];
        junction.members.push(id);
        let sample = axis.vertices[id];
        if sample.radius > junction.radius {
            junction.centre = sample.point;
            junction.radius = sample.radius;
        }
    }
    junctions
}

/// The representative of `i` in a union-find forest.
pub(super) fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The merged junction of each vertex.
pub(super) fn junction_index(axis: &MedialAxis, junctions: &[Junction]) -> Vec<Option<usize>> {
    let mut of = vec![None; axis.vertices.len()];
    for (j, junction) in junctions.iter().enumerate() {
        for &m in &junction.members {
            of[m] = Some(j);
        }
    }
    of
}

/// The edge measure at a junction (ADR 0063 §4).
#[derive(Debug, Clone, Copy)]
pub(super) struct Measure {
    pub(super) direction: Vec2,
    pub(super) width: f64,
    pub(super) length: f64,
    pub(super) start: Point,
    /// Whether the edge ends free (a spur, a fin, a stroke's end) rather than at a junction.
    pub(super) leaf: bool,
}

pub(super) fn measure(axis: &MedialAxis, junction: &Junction, side: Side, leaf: bool) -> Measure {
    let samples = outward(axis, side);
    let attach = samples[0].point;
    let radius = junction.radius;
    let outside: Vec<AxisSample> = samples
        .iter()
        .copied()
        .filter(|s| !junction.covers(axis, s.point))
        .collect();
    let mut band: Vec<AxisSample> = outside
        .iter()
        .copied()
        .filter(|s| (s.point - attach).hypot() <= MEASURE_BAND * radius)
        .collect();
    if band.len() < 2 {
        band = if outside.len() >= 2 {
            outside.clone()
        } else {
            samples.clone()
        };
    }
    let first = band[0].point;
    let last = band[band.len() - 1].point;
    let chord = last - first;
    let direction = if chord.hypot() > 0.0 {
        chord / chord.hypot()
    } else {
        let fallback = last - attach;
        fallback / fallback.hypot().max(1e-12)
    };
    let mut widths: Vec<f64> = band.iter().map(|s| 2.0 * s.radius).collect();
    widths.sort_by(f64::total_cmp);
    let length = samples
        .windows(2)
        .map(|w| (w[1].point - w[0].point).hypot())
        .sum();
    Measure {
        direction,
        width: widths[widths.len() / 2],
        length,
        start: outside.first().map_or(attach, |s| s.point),
        leaf,
    }
}

/// The direction term `(1 + d_a·d_b) / 2`: 0 straight on, 1 turning back.
fn turn_cost(a: &Measure, b: &Measure) -> f64 {
    f64::midpoint(1.0, a.direction.dot(b.direction))
}

/// `(1 + d_a·d_b) / 2 + |w_a - w_b| / max(w_a, w_b)`.
pub(super) fn cost(a: &Measure, b: &Measure) -> f64 {
    turn_cost(a, b) + (a.width - b.width).abs() / a.width.max(b.width).max(1e-12)
}

/// Pairs whose direction terms differ by more than this are decided by direction alone
/// (CENTRE-THROUGH A, ADR 0063 amendment 2026-11-20): the width only decides between pairs
/// that continue about equally smoothly, so a stroke runs on into the thin fin that continues
/// it rather than turning into the other arm of its own width.
pub(super) const TURN_BAND: f64 = 0.1;

/// A spur this many times longer than the miter of its arms, `r_J / sin(θ/2)`, is no tip.
const TIP_MITER: f64 = 1.5;
/// A tip's end is at most this share of the junction radius wide (it falls to a point).
const TIP_END_SHARE: f64 = 0.25;
/// A tip points at least this close to the arms' outer bisector (30°, as its cosine).
const TIP_BISECTOR_COS: f64 = 0.866;
/// Arms closer to a straight line than this (`|d_a + d_b|`, about 150°) form no crest.
const TIP_CREST_MIN: f64 = 0.5;
/// Arms closer together than this (15°, as the cosine of their angle) form no crest.
const TIP_CREST_MAX_COS: f64 = 0.966;

/// The tip of a sharp crest at a junction of three edges (CENTRE-THROUGH A): a free spur that
/// points out along the bisector of the other two edges, falls to a point and is no longer
/// than the miter where their outer edges meet. Returns the spur and the two arms, which then
/// continue as one path through the spur's end.
pub(super) fn crest_tip(
    junction: &Junction,
    sides: &[Side],
    measures: &[Measure],
    tip: impl Fn(Side) -> AxisSample,
) -> Option<(Side, Side, Side)> {
    if sides.len() != 3 {
        return None;
    }
    (0..3).find_map(|k| {
        if !measures[k].leaf {
            return None;
        }
        let (a, b) = ((k + 1) % 3, (k + 2) % 3);
        let end = tip(sides[k]);
        let reach = end.point - junction.centre;
        let length = reach.hypot();
        let mean = measures[a].direction + measures[b].direction;
        let spread = measures[a].direction.dot(measures[b].direction);
        let arms = measures[a].length.min(measures[b].length);
        if length <= 0.0
            || mean.hypot() < TIP_CREST_MIN
            || spread > TIP_CREST_MAX_COS
            || measures[k].length >= arms
        {
            return None;
        }
        let half_sine = ((1.0 - spread) / 2.0).max(0.0).sqrt();
        let miter = junction.radius / half_sine.max(1e-9);
        let pointed = end.radius <= TIP_END_SHARE * junction.radius;
        let along = -reach.dot(mean) / (length * mean.hypot()) >= TIP_BISECTOR_COS;
        let short = length <= TIP_MITER * miter + super::fit::tolerance(2.0 * junction.radius);
        (pointed && along && short).then_some((sides[k], sides[a], sides[b]))
    })
}

/// Grid-position key of a point for tie-breaks.
pub(super) fn grid_key(point: Point) -> (i64, i64) {
    #[allow(clippy::cast_possible_truncation)]
    (
        (point.x * 1000.0).round() as i64,
        (point.y * 1000.0).round() as i64,
    )
}

/// The pair that continues at a junction: lowest cost, then longer, then lower start.
/// A candidate pair: cost, combined length, start key and the two side indices.
pub(super) type Candidate = (f64, f64, (i64, i64), usize, usize);

pub(super) fn best_pair(sides: &[Side], measures: &[Measure]) -> Option<(Side, Side)> {
    let mut best: Option<Candidate> = None;
    for a in 0..sides.len() {
        for b in a + 1..sides.len() {
            let c = cost(&measures[a], &measures[b]);
            let turn = turn_cost(&measures[a], &measures[b]);
            let length = measures[a].length + measures[b].length;
            let key = grid_key(measures[a].start).min(grid_key(measures[b].start));
            let better = match best {
                None => true,
                Some((bc, bl, bk, ba, bb)) => {
                    let best_turn = turn_cost(&measures[ba], &measures[bb]);
                    if (turn - best_turn).abs() > TURN_BAND {
                        turn < best_turn
                    } else if (c - bc).abs() > COST_TIE {
                        c < bc
                    } else if (length - bl).abs() > 1e-9 {
                        length > bl
                    } else {
                        key < bk
                    }
                }
            };
            if better {
                best = Some((c, length, key, a, b));
            }
        }
    }
    best.map(|(_, _, _, a, b)| (sides[a], sides[b]))
}
