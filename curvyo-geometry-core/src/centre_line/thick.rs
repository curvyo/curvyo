//! The cut at Thick parts (ADR 0063 amendment 2026-11-18, K): the chains of a region's skeleton
//! are cut where the local width passes the limit, on the skeleton the build made.

use kurbo::Point;

use super::chains::{Chain, ChainEnd};
use crate::medial_axis::{AxisSample, HoleMeasure};

/// How far a sample's disc may miss a thick small hole and still count as touching it, in mm:
/// the flattening plus the search precision.
const TOUCH_MM: f64 = 0.05;

/// A region's chains cut at its thick parts.
#[derive(Debug, Clone)]
pub(super) struct Cut {
    /// The thin pieces that may stay strokes; their ends at a thick part are [`ChainEnd::Cut`].
    pub(super) pieces: Vec<Chain>,
    /// The samples whose discs make up the thick parts: the thick ones and those of the
    /// pieces absorbed into them.
    pub(super) discs: Vec<AxisSample>,
    /// Per hole, whether it is a small hole inside a thick part.
    pub(super) thick_holes: Vec<bool>,
}

/// The chains of a region cut where they are wider than `limit` (mm): `None` when nothing is.
/// `holes` are the region's hole measures and `hole_rings` their rings in mm.
pub(super) fn cut(
    chains: &[Chain],
    holes: &[HoleMeasure],
    hole_rings: &[Vec<Point>],
    limit: f64,
) -> Option<Cut> {
    let thick_holes: Vec<bool> = holes
        .iter()
        .map(|h| h.filled.is_some_and(|w| w > limit))
        .collect();
    let flags: Vec<Vec<bool>> = chains
        .iter()
        .map(|chain| {
            chain
                .samples
                .iter()
                .map(|s| 2.0 * s.radius > limit || touches_thick_hole(s, hole_rings, &thick_holes))
                .collect()
        })
        .collect();
    if !flags.iter().flatten().any(|&f| f) {
        return None;
    }
    let mut discs: Vec<AxisSample> = chains
        .iter()
        .zip(&flags)
        .flat_map(|(chain, flags)| chain.samples.iter().zip(flags))
        .filter_map(|(s, &f)| f.then_some(*s))
        .collect();
    // The thin runs of every chain, with the chain they came from; a thin sample inside the
    // thick discs belongs to the thick part.
    let mut pieces: Vec<(usize, Chain)> = Vec::new();
    for (index, (chain, flags)) in chains.iter().zip(&flags).enumerate() {
        for mut piece in runs(chain, flags) {
            let inside = settle_cut_ends(&mut piece, &discs);
            discs.extend(inside);
            pieces.push((index, piece));
        }
    }
    // A piece that hangs off a thick part with a free end and is shorter than the limit is the
    // thick part's own corner or tail, not a stroke.
    let mut kept: Vec<(usize, Chain)> = Vec::new();
    for (index, piece) in pieces {
        let ends = [piece.start, piece.end];
        let absorbed = piece.samples.len() < 2
            || (ends.contains(&ChainEnd::Cut)
                && !ends.iter().any(|e| matches!(e, ChainEnd::Junction { .. }))
                && piece.length() < limit);
        if absorbed {
            discs.extend(piece.samples);
        } else {
            kept.push((index, piece));
        }
    }
    Some(Cut {
        pieces: reparent(&kept),
        discs,
        thick_holes,
    })
}

/// Drops the samples at each cut end of `piece` that lie inside the `thick` discs, so the cut
/// lies where the stroke leaves the thick part (a stroke entering a wide bar beside its corner
/// would otherwise be cut inside the bar); the dropped samples belong to the thick part.
fn settle_cut_ends(piece: &mut Chain, thick: &[AxisSample]) -> Vec<AxisSample> {
    let mut dropped = Vec::new();
    let inside = |p: Point| {
        thick
            .iter()
            .any(|s| (p - s.point).hypot2() < s.radius * s.radius)
    };
    if piece.start == ChainEnd::Cut {
        let keep = piece
            .samples
            .iter()
            .position(|s| !inside(s.point))
            .unwrap_or(piece.samples.len());
        dropped.extend(piece.samples.drain(..keep));
    }
    if piece.end == ChainEnd::Cut {
        let keep = piece
            .samples
            .iter()
            .rposition(|s| !inside(s.point))
            .map_or(0, |i| i + 1);
        dropped.extend(piece.samples.drain(keep..));
    }
    dropped
}

/// Whether the disc of `sample` touches a thick small hole.
fn touches_thick_hole(sample: &AxisSample, rings: &[Vec<Point>], thick: &[bool]) -> bool {
    rings.iter().zip(thick).any(|(ring, &thick)| {
        thick && {
            let reach = sample.radius + TOUCH_MM;
            ring.iter().enumerate().any(|(i, &a)| {
                let b = ring[(i + 1) % ring.len()];
                distance_to_segment(sample.point, a, b) <= reach
            })
        }
    })
}

fn distance_to_segment(point: Point, from: Point, to: Point) -> f64 {
    let along = to - from;
    let length = along.hypot2();
    let share = if length > 0.0 {
        ((point - from).dot(along) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (from + along * share - point).hypot()
}

/// The thin runs of `chain` (where `flags` is false), each a chain whose ends at a thick run
/// are cuts. A closed chain with a thick run is opened there.
fn runs(chain: &Chain, flags: &[bool]) -> Vec<Chain> {
    if !flags.contains(&true) {
        return vec![chain.clone()];
    }
    let (samples, flags) = if chain.closed {
        // invariant: a thick sample exists (checked above); 0 is never taken.
        let first = flags.iter().position(|&f| f).unwrap_or(0);
        let mut samples = chain.samples.clone();
        let mut flags = flags.to_vec();
        samples.rotate_left(first);
        flags.rotate_left(first);
        (samples, flags)
    } else {
        (chain.samples.clone(), flags.to_vec())
    };
    let n = samples.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if flags[i] {
            i += 1;
            continue;
        }
        let a = i;
        while i < n && !flags[i] {
            i += 1;
        }
        let b = i - 1;
        let start = if a == 0 && !chain.closed {
            chain.start
        } else {
            ChainEnd::Cut
        };
        let end = if b == n - 1 && !chain.closed {
            chain.end
        } else {
            ChainEnd::Cut
        };
        out.push(Chain {
            samples: samples[a..=b].to_vec(),
            closed: false,
            start,
            end,
            main: chain.main,
        });
    }
    out
}

/// The pieces with their junction ends pointed at the piece of their parent chain nearest the
/// junction; a junction end whose parent has no piece left lies in a thick part and is a cut.
fn reparent(pieces: &[(usize, Chain)]) -> Vec<Chain> {
    let mut out: Vec<Chain> = pieces.iter().map(|(_, p)| p.clone()).collect();
    for piece in &mut out {
        for end in [&mut piece.start, &mut piece.end] {
            let ChainEnd::Junction { centre, parent, .. } = end else {
                continue;
            };
            let Some(chain) = *parent else { continue };
            let centre = *centre;
            let nearest = pieces
                .iter()
                .enumerate()
                .filter(|(_, (c, _))| *c == chain)
                .map(|(k, (_, p))| {
                    let d = p
                        .samples
                        .iter()
                        .map(|s| (s.point - centre).hypot())
                        .fold(f64::INFINITY, f64::min);
                    (k, d)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            match nearest {
                Some((k, _)) => *parent = Some(k),
                None => *end = ChainEnd::Cut,
            }
        }
    }
    out
}
