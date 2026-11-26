//! Stencil islands and where their bridges cross the lettering (`specs/0156-fused-lettering`
//! criteria 22, 23 and 27; spec issue 4): every hole of the lettering's union is an island; each
//! gets its bridges across the stroke to the piece's outer contour, the first at the narrowest
//! crossing, each further one the narrowest at least a quarter of the island's perimeter along
//! its boundary from every earlier one. The sheet beyond the outer contour is the surrounding
//! sheet or, for a piece inside another's hole, that island, which is bridged in turn (chains).

use crate::boolean_grid::Path64;
use crate::fuse::measure::{
    P, along_distance, arc_lengths, nearest_on, normal_at, point_at, ribbon_quad, segments_meet,
    vertex, winding,
};
use crate::fuse::pieces::Piece;

/// Arc-length step between sampled crossings on an island's boundary, in grid units (0.05 mm).
const SAMPLE_STEP_UNITS: f64 = 50.0;

/// One way across the stroke, from the island's boundary along its normal to the piece's outer
/// contour.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Crossing {
    /// The start on the island's boundary.
    pub(crate) from: P,
    /// The end on the outer contour.
    pub(crate) to: P,
    /// Its length in grid units.
    pub(crate) length: f64,
    /// Its start's arc length along the island's boundary.
    pub(crate) along: f64,
}

impl Crossing {
    /// The middle of the crossing.
    pub(crate) fn centre(&self) -> P {
        self.from.add(self.to).scale(0.5)
    }
}

/// The sample positions along `hole`: every vertex and points at most a step apart between.
fn samples(hole: &Path64, lengths: &[f64]) -> Vec<f64> {
    let mut at = Vec::new();
    for edge in 0..hole.len() {
        let (start, end) = (lengths[edge], lengths[edge + 1]);
        at.push(start);
        let span = end - start;
        // Small counts: an edge is at most 2·10¹⁰ units long.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (span / SAMPLE_STEP_UNITS).ceil() as usize;
        for k in 1..steps {
            #[allow(clippy::cast_precision_loss)]
            at.push(start + span * k as f64 / steps as f64);
        }
    }
    at
}

/// Whether the crossing runs through the stroke only: its middle is in the material and it
/// meets no other hole of the piece.
fn through_stroke(piece: &Piece, hole: usize, from: P, to: P) -> bool {
    let middle = from.add(to).scale(0.5);
    if winding(piece.outer(), middle) == 0 {
        return false;
    }
    piece.holes().iter().enumerate().all(|(index, other)| {
        winding(other, middle) == 0
            && (index == hole
                || (0..other.len())
                    .all(|e| !segments_meet(from, to, vertex(other, e), vertex(other, e + 1))))
    })
}

/// The first point where the ray from `from` along `direction` meets `contour`.
#[allow(clippy::many_single_char_names)] // the usual names of plane geometry
fn ray_hit(contour: &Path64, from: P, direction: P) -> Option<P> {
    let mut best: Option<f64> = None;
    for edge in 0..contour.len() {
        let (a, b) = (vertex(contour, edge), vertex(contour, edge + 1));
        let e = b.sub(a);
        let denominator = direction.cross(e);
        if denominator == 0.0 {
            continue;
        }
        let w = a.sub(from);
        let t = w.cross(e) / denominator;
        let s = w.cross(direction) / denominator;
        if t > 0.0 && (0.0..=1.0).contains(&s) && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    }
    best.map(|t| from.add(direction.scale(t)))
}

/// The crossings of island `hole` (an index into the piece's holes), at the sample positions.
pub(crate) fn crossings(piece: &Piece, hole: usize) -> Vec<Crossing> {
    let boundary = &piece.holes()[hole];
    let lengths = arc_lengths(boundary);
    samples(boundary, &lengths)
        .into_iter()
        .filter_map(|along| {
            let from = point_at(boundary, &lengths, along);
            // Into the stroke: the hole's normal points into the island.
            let inward = normal_at(boundary, &nearest_on(boundary, from)).scale(-1.0);
            let to = ray_hit(piece.outer(), from, inward)
                .unwrap_or_else(|| nearest_on(piece.outer(), from).point);
            through_stroke(piece, hole, from, to).then(|| Crossing {
                from,
                to,
                length: to.sub(from).length(),
                along,
            })
        })
        .collect()
}

/// The perimeter of island `hole`.
pub(crate) fn perimeter(piece: &Piece, hole: usize) -> f64 {
    let lengths = arc_lengths(&piece.holes()[hole]);
    lengths[lengths.len() - 1]
}

/// Top-most, then left-most (page coordinates, y down).
fn top_left(x: &Crossing, y: &Crossing) -> std::cmp::Ordering {
    let (a, b) = (x.centre(), y.centre());
    a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x))
}

/// Up to `count` crossings by the rule of criterion 23: the narrowest (ties within `tie`: the
/// top-most, then the left-most); each further one the narrowest of those at least a quarter
/// perimeter along the boundary from every earlier one (ties: the farthest from them, then
/// top-most, then left-most).
pub(crate) fn choose(all: &[Crossing], count: usize, perimeter: f64, tie: f64) -> Vec<Crossing> {
    let mut chosen: Vec<Crossing> = Vec::new();
    while chosen.len() < count {
        let spacing = |c: &Crossing| {
            chosen
                .iter()
                .map(|k| along_distance(c.along, k.along, perimeter))
                .fold(f64::INFINITY, f64::min)
        };
        let open: Vec<&Crossing> = all
            .iter()
            .filter(|c| spacing(c) >= perimeter / 4.0 - 1e-6)
            .collect();
        let Some(narrowest) = open.iter().map(|c| c.length).reduce(f64::min) else {
            break;
        };
        let best = open
            .into_iter()
            .filter(|c| c.length <= narrowest + tie)
            .min_by(|x, y| {
                let far = if chosen.is_empty() {
                    std::cmp::Ordering::Equal
                } else {
                    spacing(y).total_cmp(&spacing(x))
                };
                far.then_with(|| top_left(x, y))
            });
        match best {
            Some(c) => chosen.push(*c),
            None => break,
        }
    }
    chosen
}

/// The band of a crossing: a ribbon `width` wide along the stroke, from one width inside the
/// island to one width beyond the outer contour.
pub(crate) fn band(crossing: &Crossing, width: f64) -> Path64 {
    let u = crossing.to.sub(crossing.from).unit();
    ribbon_quad(
        crossing.from.sub(u.scale(width)),
        crossing.to.add(u.scale(width)),
        width / 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::{Point64, normalize};
    use crate::fuse::pieces::pieces_of;

    fn polygon(points: &[(f64, f64)]) -> Path64 {
        points.iter().map(|&(x, y)| P::new(x, y).snap()).collect()
    }

    fn circle(r: f64, n: usize) -> Path64 {
        let points: Vec<(f64, f64)> = (0..n)
            .map(|k| {
                #[allow(clippy::cast_precision_loss)]
                let a = std::f64::consts::TAU * k as f64 / n as f64;
                (r * a.sin(), -r * a.cos())
            })
            .collect();
        polygon(&points)
    }

    /// Criterion 24 at the level of crossings: the ring's two bridges are at the top and at the
    /// bottom.
    #[test]
    fn a_concentric_ring_gets_its_bridges_at_the_top_and_the_bottom() {
        let mut inner = circle(6_000.0, 256);
        inner.reverse();
        let union = normalize(&vec![circle(10_000.0, 256), inner]);
        let pieces = pieces_of(&union);
        let all = crossings(&pieces[0], 0);
        let chosen = choose(&all, 2, perimeter(&pieces[0], 0), 10.0);
        assert_eq!(chosen.len(), 2);
        let top = chosen[0].centre();
        let bottom = chosen[1].centre();
        assert!(top.sub(P::new(0.0, -8_000.0)).length() < 10.0, "{top:?}");
        assert!(
            bottom.sub(P::new(0.0, 8_000.0)).length() < 10.0,
            "{bottom:?}"
        );
        let _ = Point64::new(0, 0);
    }
}
