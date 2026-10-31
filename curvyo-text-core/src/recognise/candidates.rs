//! Builds a line's glyph candidates, runs of 1 to 3 neighbouring shapes in line space (`adrs.md`
//! decision 3, stage 3).

use kurbo::{Point, Rect};

use super::blobs::{Blob, Loop, bounds};
use super::descriptor::Descriptor;
use super::distance::SegmentGrid;
use super::lines::{Frame, along, median, median_slope};
use super::sample::Ring;

/// Neighbouring shapes this many line heights apart may still form one glyph (Proposal).
const JOIN_GAP: f64 = 0.3;
/// A shape lower than this share of the line's median height is a mark (dot, accent, comma):
/// only runs holding a mark may be one glyph.
const MARK_SHARE: f64 = 0.6;
/// Coarse points per median shape height, for ranking.
pub(crate) const COARSE_PER_HEIGHT: f64 = 25.0;
/// The cost of cutting between two shapes that overlap along the line (Proposal).
const CUT_PENALTY: f64 = 1.0;

/// One run of neighbouring shapes that may be one glyph, in line space.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    /// The position of its first shape in the line.
    pub first: usize,
    /// The number of shapes.
    pub len: usize,
    /// Its fine points (every sample).
    pub points: Vec<Point>,
    /// Its coarse rings, for telling inside from outside.
    pub coarse_rings: Vec<Ring>,
    /// Every n-th point, for ranking.
    pub coarse: Vec<Point>,
    /// The grid of its segments.
    pub grid: SegmentGrid,
    /// The grid of the segments between its coarse points, for ranking.
    pub coarse_grid: SegmentGrid,
    /// Its box in line space.
    pub bbox: Rect,
    pub descriptor: Descriptor,
    /// The input outlines its rings came from.
    pub outlines: Vec<usize>,
}

/// One line ready for matching: the frame, its shapes in order and their candidates.
#[derive(Debug, Clone)]
pub(crate) struct Prepared {
    pub frame: Frame,
    /// The blobs of the line in reading order.
    pub blobs: Vec<usize>,
    pub candidates: Vec<Candidate>,
    /// The median shape height, mm.
    pub height: f64,
    /// Per position, the cost of starting a glyph there: shapes that overlap their left
    /// neighbour by more than half the narrower width (a dot over its stem) belong together.
    pub cuts: Vec<f64>,
}

/// Every `stride`-th point of each ring (at least three per ring).
pub(crate) fn thin(rings: &[Ring], stride: usize) -> Vec<Ring> {
    rings
        .iter()
        .map(|ring| {
            let step = stride.max(1).min((ring.len() / 3).max(1));
            ring.iter().step_by(step).copied().collect()
        })
        .collect()
}

/// The stride that thins points `spacing` apart to about `wanted` apart.
pub(crate) fn stride(spacing: f64, wanted: f64) -> usize {
    let n = (wanted / spacing).round();
    if n.is_finite() && n >= 1.0 {
        // invariant: clamped to a small positive whole number.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.min(64.0) as usize;
        n
    } else {
        1
    }
}

/// The line's shapes in reading order along `frame`, each with its rings in line space.
fn ordered(
    frame: Frame,
    line: &[usize],
    blobs: &[Blob],
    loops: &[Loop],
) -> Vec<(usize, Vec<Ring>)> {
    let mut order: Vec<(f64, f64, usize)> = line
        .iter()
        .map(|&b| {
            let (left, right) = along(&blobs[b], &frame);
            (left, right, b)
        })
        .collect();
    order.sort_by(|a, b| {
        f64::midpoint(a.0, a.1)
            .total_cmp(&f64::midpoint(b.0, b.1))
            .then(a.2.cmp(&b.2))
    });
    order
        .into_iter()
        .map(|(_, _, b)| {
            let rings = blobs[b]
                .loops
                .iter()
                .map(|&l| loops[l].points.iter().map(|&p| frame.to_line(p)).collect())
                .collect();
            (b, rings)
        })
        .collect()
}

/// The shapes `first..=last` of `shapes` as one candidate.
fn candidate(
    shapes: &[(usize, Vec<Ring>)],
    (first, last): (usize, usize),
    blobs: &[Blob],
    loops: &[Loop],
    coarse_stride: usize,
    cap: f64,
) -> Candidate {
    let run = &shapes[first..=last];
    let rings: Vec<Ring> = run.iter().flat_map(|(_, r)| r.iter().cloned()).collect();
    let refs: Vec<&[Point]> = rings.iter().map(Vec::as_slice).collect();
    let points: Vec<Point> = rings.iter().flatten().copied().collect();
    let bbox = bounds(&points);
    let holes = run.iter().map(|&(b, _)| blobs[b].holes()).sum();
    let mut outlines: Vec<usize> = run
        .iter()
        .flat_map(|&(b, _)| blobs[b].loops.iter().map(|&l| loops[l].outline))
        .collect();
    outlines.sort_unstable();
    outlines.dedup();
    let coarse_rings = thin(&rings, coarse_stride);
    let coarse_refs: Vec<&[Point]> = coarse_rings.iter().map(Vec::as_slice).collect();
    Candidate {
        first,
        len: last - first + 1,
        coarse: coarse_rings.iter().flatten().copied().collect(),
        coarse_rings: coarse_rings.clone(),
        coarse_grid: SegmentGrid::new(&coarse_refs, cap),
        grid: SegmentGrid::new(&refs, cap),
        descriptor: Descriptor::of(&coarse_refs, bbox, holes),
        points,
        bbox,
        outlines,
    }
}

/// The runs of shapes that may be one glyph: 1 to 3 neighbours, close together, holding a mark
/// when longer than one.
fn runs(boxes: &[Rect], height: f64) -> Vec<(usize, usize)> {
    let mark: Vec<bool> = boxes
        .iter()
        .map(|b| b.height() < MARK_SHARE * height)
        .collect();
    let mut out = Vec::new();
    for first in 0..boxes.len() {
        let mut reach = boxes[first].x1;
        let mut any_mark = mark[first];
        for last in first..(first + 3).min(boxes.len()) {
            if last > first {
                any_mark |= mark[last];
                if boxes[last].x0 - reach > JOIN_GAP * height || !any_mark {
                    break;
                }
                reach = reach.max(boxes[last].x1);
            }
            out.push((first, last));
        }
    }
    out
}

/// The cost of starting a glyph at each shape (see [`Prepared::cuts`]).
fn cuts(boxes: &[Rect]) -> Vec<f64> {
    (0..boxes.len())
        .map(|k| {
            if k == 0 {
                return 0.0;
            }
            let (a, b) = (boxes[k - 1], boxes[k]);
            let overlap = a.x1.min(b.x1) - a.x0.max(b.x0);
            if overlap > 0.5 * a.width().min(b.width()) {
                CUT_PENALTY
            } else {
                0.0
            }
        })
        .collect()
}

/// Shapes whose bottom is within this share of the median shape height of the median bottom
/// stand on the baseline (descenders, commas and raised marks do not): wide first, while the
/// frame may still be tilted, then narrow enough to leave out a letter a millimetre off
/// (overshoot of round letters is about 1.5 %).
const ON_BASELINE: [f64; 3] = [0.12, 0.02, 0.02];

/// `frame` turned so the line's baseline runs along it: the median slope through the lowest
/// points of the shapes that stand on the baseline. The grouping frame follows the shape centres, which
/// a short word of mixed heights ("Wolf") tilts by a degree or two; at large sizes that alone
/// puts its letters out of the tolerance (tester finding 2026-10-31). Unchanged when fewer than
/// two shapes stand on the baseline or they span less than one shape height.
pub(crate) fn levelled(frame: Frame, line: &[usize], blobs: &[Blob], loops: &[Loop]) -> Frame {
    let mut frame = frame;
    for band in ON_BASELINE {
        let shapes = ordered(frame, line, blobs, loops);
        let lowest: Vec<(Point, f64)> = shapes
            .iter()
            .filter_map(|(_, rings)| {
                let points: Vec<Point> = rings.iter().flatten().copied().collect();
                let low = points.iter().copied().max_by(|a, b| a.y.total_cmp(&b.y))?;
                Some((low, bounds(&points).height()))
            })
            .collect();
        let mut heights: Vec<f64> = lowest.iter().map(|l| l.1).collect();
        let height = median(&mut heights);
        let mut bottoms: Vec<f64> = lowest.iter().map(|l| l.0.y).collect();
        let bottom = median(&mut bottoms);
        let standing: Vec<Point> = lowest
            .iter()
            .filter(|(p, h)| *h >= 0.6 * height && (p.y - bottom).abs() <= band * height)
            .map(|(p, _)| *p)
            .collect();
        let span = standing
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            - standing.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        if standing.len() < 2 || span < height {
            break;
        }
        let Some(d) = median_slope(&standing, 0.5 * height) else {
            break;
        };
        let u = frame.u * d.x + frame.n() * d.y;
        let length = u.hypot2().sqrt();
        if length.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            break;
        }
        frame = Frame {
            origin: frame.origin,
            u: u / length,
        };
    }
    frame
}

/// Sorts the line's blobs along `frame` and builds the candidates (stage 3).
pub(crate) fn prepare(
    frame: Frame,
    line: &[usize],
    blobs: &[Blob],
    loops: &[Loop],
    spacing: f64,
    cap: f64,
) -> Prepared {
    let shapes = ordered(frame, line, blobs, loops);
    let boxes: Vec<Rect> = shapes
        .iter()
        .map(|(_, rings)| bounds(&rings.iter().flatten().copied().collect::<Vec<_>>()))
        .collect();
    let mut heights: Vec<f64> = boxes.iter().map(Rect::height).collect();
    let height = median(&mut heights);
    let coarse_stride = stride(spacing, (height / COARSE_PER_HEIGHT).max(spacing));
    let candidates = runs(&boxes, height)
        .into_iter()
        .map(|run| candidate(&shapes, run, blobs, loops, coarse_stride, cap))
        .collect();
    Prepared {
        frame,
        blobs: shapes.into_iter().map(|(b, _)| b).collect(),
        candidates,
        height,
        cuts: cuts(&boxes),
    }
}
