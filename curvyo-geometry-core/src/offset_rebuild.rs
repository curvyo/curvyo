//! Turns raw offset loops into result outlines of exact pieces: `i_overlay` decides which parts of
//! the loops survive (Positive fill rule on their flattened, snapped vertices), and every surviving
//! ring is mapped back onto the pieces it came from (`specs/0038-path-offset` criteria 33, 34, 39;
//! ADR 0003 §4 as amended on 2026-10-12).
//!
//! Each flattened vertex carries a tag (its loop, piece and parameter), found again in a ring by
//! its exact grid coordinates. Consecutive tagged vertices of one loop form a run, which is written
//! as the loop's own pieces; between runs lies a crossing, which is solved on the two exact pieces.
//! A ring that cannot be mapped within the tolerance is written as its polyline (the fallback).

use std::cell::OnceCell;
use std::collections::HashMap;

use curvyo_document_core::Point as DocPoint;
pub(crate) use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{IntOverlayOptions, Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::core::solver::Solver;
use kurbo::{ParamCurve, ParamCurveDeriv, PathSeg, Point, Vec2};

use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{Path64, Paths64, Point64, snap, to_millimetres};
use crate::offset_finish::{finish, lines_of_mm, set_end, set_start};

/// At most this many flattening steps per cubic piece.
const MAX_STEPS: f64 = 4096.0;

/// One flattened vertex of a raw loop.
#[derive(Debug, Clone, Copy)]
struct Sample {
    point: Point64,
    piece: usize,
    t: f64,
}

/// The surviving outlines, each a closed list of lines and cubics, and how many were written as
/// polylines because they could not be mapped.
#[derive(Debug, Default)]
pub(crate) struct Rebuilt {
    pub outlines: Vec<Vec<PathSeg>>,
    pub fallback: usize,
}

/// Overlays `loops` with `fill_rule` (`Positive` for an offset, `NonZero` for a stroke) and maps every surviving ring back onto the loops'
/// pieces. `flatten_mm` is the flattening tolerance, `tolerance_mm` the largest distance a mapped
/// outline may lie from its ring. `keep` decides which rings count (the caller drops slivers).
pub(crate) fn rebuild(
    loops: &[Vec<PathSeg>],
    flatten_mm: f64,
    tolerance_mm: f64,
    fill_rule: FillRule,
    keep: impl Fn(Paths64) -> Paths64,
) -> Rebuilt {
    let samples: Vec<Vec<Sample>> = loops
        .iter()
        .map(|pieces| flatten_loop(pieces, flatten_mm))
        .collect();
    let mut tags: HashMap<(i64, i64), (usize, usize)> = HashMap::new();
    for (loop_index, loop_samples) in samples.iter().enumerate() {
        for (index, sample) in loop_samples.iter().enumerate() {
            tags.entry((sample.point.x, sample.point.y))
                .or_insert((loop_index, index));
        }
    }
    let capacity = samples.iter().map(Vec::len).sum();
    let mut overlay = Overlay::<i64>::new_custom(
        capacity,
        IntOverlayOptions::keep_all_points(),
        Solver::default(),
    );
    for loop_samples in &samples {
        let contour: Path64 = loop_samples.iter().map(|sample| sample.point).collect();
        if contour.len() >= 3 {
            overlay.add_contour(&contour, ShapeType::Subject);
        }
    }
    let rings: Paths64 = overlay
        .overlay(OverlayRule::Subject, fill_rule)
        .into_iter()
        .flatten()
        .collect();
    let context = Context {
        loops,
        samples: &samples,
        tags: &tags,
        tolerance_mm,
        cells: OnceCell::new(),
    };
    let mut rebuilt = Rebuilt::default();
    for ring in keep(rings) {
        match context.ring(&ring).map(|pieces| finish(&pieces)) {
            Some(pieces) if pieces.len() >= 2 => rebuilt.outlines.push(pieces),
            _ => {
                let polyline = cleanup(&vec![ring]);
                rebuilt.fallback += polyline.len();
                rebuilt
                    .outlines
                    .extend(polyline.iter().map(|path| lines_of_mm(path)));
            }
        }
    }
    rebuilt
}

/// The flattened, snapped vertices of one loop, each tagged with its piece and parameter, without
/// consecutive duplicates (the wrap-around pair included).
fn flatten_loop(pieces: &[PathSeg], tolerance_mm: f64) -> Vec<Sample> {
    let mut samples: Vec<Sample> = Vec::new();
    for (piece, segment) in pieces.iter().enumerate() {
        let steps = step_count(segment, tolerance_mm);
        for step in 0..steps {
            #[allow(clippy::cast_precision_loss)] // far below 2^53
            let t = step as f64 / steps as f64;
            let point = grid_of(segment.eval(t));
            if samples.last().is_none_or(|last| last.point != point) {
                samples.push(Sample { point, piece, t });
            }
        }
    }
    while samples.len() > 1 && samples.first().map(|s| s.point) == samples.last().map(|s| s.point) {
        samples.pop();
    }
    samples
}

/// The number of equal parameter steps that keep every chord of `segment` within `tolerance_mm`
/// (the bound of `flatten::step_count`: `h² max|B''| / 8`), with `sqrt` only.
fn step_count(segment: &PathSeg, tolerance_mm: f64) -> usize {
    let PathSeg::Cubic(c) = *segment else {
        return 1;
    };
    let d0 = (c.p2.to_vec2() - c.p1.to_vec2() * 2.0 + c.p0.to_vec2())
        .hypot2()
        .sqrt();
    let d1 = (c.p3.to_vec2() - c.p2.to_vec2() * 2.0 + c.p1.to_vec2())
        .hypot2()
        .sqrt();
    let steps = (6.0 * d0.max(d1) / (8.0 * tolerance_mm)).sqrt().ceil();
    // Not `clamp`: NaN becomes one step through `max`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::manual_clamp
    )]
    {
        steps.max(1.0).min(MAX_STEPS) as usize
    }
}

/// What the mapping of one ring reads.
struct Context<'a> {
    loops: &'a [Vec<PathSeg>],
    samples: &'a [Vec<Sample>],
    tags: &'a HashMap<(i64, i64), (usize, usize)>,
    tolerance_mm: f64,
    /// The chords by grid cell, built the first time an edge has no tagged end.
    cells: OnceCell<ChordCells>,
}

/// The chords of all loops by square cell of the grid, for finding the chord of a ring edge
/// between two crossings without scanning every chord.
struct ChordCells {
    /// The cell side in grid units.
    side: i64,
    cells: HashMap<(i64, i64), Vec<Chord>>,
    /// Chords that would cover too many cells; always tried.
    long: Vec<Chord>,
}

/// A chord whose box covers more cells than this goes to the `long` list.
const MAX_CELLS_PER_CHORD: i64 = 64;

impl ChordCells {
    fn new(samples: &[Vec<Sample>]) -> Self {
        let count: usize = samples.iter().map(Vec::len).sum();
        let (mut min, mut max) = ((i64::MAX, i64::MAX), (i64::MIN, i64::MIN));
        for sample in samples.iter().flatten() {
            min = (min.0.min(sample.point.x), min.1.min(sample.point.y));
            max = (max.0.max(sample.point.x), max.1.max(sample.point.y));
        }
        // About one chord per cell: the side of a square of the box's area over the count.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let side = {
            let area = (max.0 - min.0).max(1) as f64 * (max.1 - min.1).max(1) as f64;
            ((area / count.max(1) as f64).sqrt().ceil() as i64).max(1)
        };
        let mut grid = Self {
            side,
            cells: HashMap::new(),
            long: Vec::new(),
        };
        let reach = ON_CHORD_REACH;
        for (lp, loop_samples) in samples.iter().enumerate() {
            for index in 0..loop_samples.len() {
                let (p, q) = (
                    loop_samples[index].point,
                    loop_samples[(index + 1) % loop_samples.len()].point,
                );
                let (x0, x1) = (p.x.min(q.x) - reach, p.x.max(q.x) + reach);
                let (y0, y1) = (p.y.min(q.y) - reach, p.y.max(q.y) + reach);
                let (cx0, cx1) = (x0.div_euclid(side), x1.div_euclid(side));
                let (cy0, cy1) = (y0.div_euclid(side), y1.div_euclid(side));
                if (cx1 - cx0 + 1) * (cy1 - cy0 + 1) > MAX_CELLS_PER_CHORD {
                    grid.long.push((lp, index));
                    continue;
                }
                for cx in cx0..=cx1 {
                    for cy in cy0..=cy1 {
                        grid.cells.entry((cx, cy)).or_default().push((lp, index));
                    }
                }
            }
        }
        grid
    }

    /// The chords that may hold the point `at`: those of its cell and the long ones.
    fn near(&self, at: Point64) -> impl Iterator<Item = Chord> + '_ {
        let cell = (at.x.div_euclid(self.side), at.y.div_euclid(self.side));
        self.cells
            .get(&cell)
            .into_iter()
            .flatten()
            .chain(&self.long)
            .copied()
    }
}

/// A point in millimetres snapped to the grid.
fn grid_of(point: Point) -> Point64 {
    snap(DocPoint::new(point.x, point.y))
}

/// A grid point in millimetres.
fn mm_of(point: Point64) -> Point {
    let mm = to_millimetres(&point);
    Point::new(mm.x, mm.y)
}

/// A place on a loop: piece index and parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Place {
    piece: usize,
    t: f64,
}

/// A chord of a flattened loop: the loop and the index of the vertex it starts at.
type Chord = (usize, usize);

/// Grid units a ring edge may lie off the chord it came from (the overlay rounds crossings).
const ON_CHORD_UNITS: f64 = 2.0;

/// [`ON_CHORD_UNITS`] as whole grid units, for the cell boxes.
const ON_CHORD_REACH: i64 = 2;

impl Context<'_> {
    fn tag(&self, point: Point64) -> Option<(usize, usize)> {
        self.tags.get(&(point.x, point.y)).copied()
    }

    /// The pieces of `ring`, or `None` if it cannot be mapped within the tolerance.
    ///
    /// Every edge of the ring is labelled with the flattened chord it lies on. Edges whose chords
    /// follow one another on one loop form a run; the vertex between two runs is a crossing of
    /// two exact pieces.
    fn ring(&self, ring: &[Point64]) -> Option<Vec<PathSeg>> {
        let n = ring.len();
        if n < 3 {
            return None;
        }
        let labels: Vec<Chord> = (0..n)
            .map(|i| self.label(ring[i], ring[(i + 1) % n]))
            .collect::<Option<_>>()?;
        let follows: Vec<bool> = (0..n)
            .map(|i| self.follows(labels[i], labels[(i + 1) % n]))
            .collect();
        if follows.iter().all(|&f| f) {
            return self.whole_loop(&labels);
        }
        // Runs: maximal chains of edges whose chords follow one another, as (first, last) edge.
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for start in 0..n {
            if follows[(start + n - 1) % n] {
                continue;
            }
            let mut end = start;
            while follows[end] {
                end = (end + 1) % n;
            }
            runs.push((start, end));
        }
        // The crossing at the end of each run, at the vertex where the next run starts.
        let mut ends: Vec<(Place, Point, Place)> = Vec::with_capacity(runs.len());
        for (k, &(_, last)) in runs.iter().enumerate() {
            let (next_first, _) = runs[(k + 1) % runs.len()];
            let vertex = mm_of(ring[next_first]);
            ends.push(self.crossing(labels[last], labels[next_first], vertex)?);
        }
        let mut pieces = Vec::new();
        for (k, &(first, _)) in runs.iter().enumerate() {
            let (_, start_point, start) = ends[(k + runs.len() - 1) % runs.len()];
            let (end, end_point, _) = ends[k];
            let mut span = self.span(labels[first].0, start, end);
            if span.is_empty() {
                continue;
            }
            set_start(&mut span[0], start_point);
            let last = span.len() - 1;
            set_end(&mut span[last], end_point);
            pieces.extend(span);
        }
        Some(pieces)
    }

    /// The chord the ring edge from `a` to `b` lies on: first the chords the tags of its ends
    /// suggest, then the chords near its middle.
    fn label(&self, a: Point64, b: Point64) -> Option<Chord> {
        let mut candidates: Vec<Chord> = Vec::with_capacity(2);
        if let Some((lp, index)) = self.tag(a) {
            candidates.push((lp, index));
        }
        if let Some((lp, index)) = self.tag(b) {
            let count = self.samples[lp].len();
            candidates.push((lp, (index + count - 1) % count));
        }
        if let Some(&chord) = candidates.iter().find(|&&c| self.on_chord(c, a, b)) {
            return Some(chord);
        }
        let middle = Point64::new(a.x + (b.x - a.x) / 2, a.y + (b.y - a.y) / 2);
        self.cells
            .get_or_init(|| ChordCells::new(self.samples))
            .near(middle)
            .find(|&chord| self.on_chord(chord, a, b))
    }

    /// Whether the ring edge from `a` to `b` lies on `chord`, in its direction.
    fn on_chord(&self, (lp, index): Chord, from: Point64, to: Point64) -> bool {
        let samples = &self.samples[lp];
        let (start, end) = (
            samples[index].point,
            samples[(index + 1) % samples.len()].point,
        );
        #[allow(clippy::cast_precision_loss)] // grid units <= 10^10
        let vector = |p: Point64, q: Point64| Vec2::new((q.x - p.x) as f64, (q.y - p.y) as f64);
        let chord = vector(start, end);
        let length2 = chord.hypot2();
        if length2 == 0.0 || vector(from, to).dot(chord) <= 0.0 {
            return false;
        }
        let length = length2.sqrt();
        [from, to].into_iter().all(|point| {
            let w = vector(start, point);
            let along = w.dot(chord) / length;
            w.cross(chord).abs() / length <= ON_CHORD_UNITS
                && along >= -ON_CHORD_UNITS
                && along <= length + ON_CHORD_UNITS
        })
    }

    /// Whether chord `b` is chord `a` or the next one on the same loop.
    fn follows(&self, (la, ia): Chord, (lb, ib): Chord) -> bool {
        la == lb && (ib == ia || ib == (ia + 1) % self.samples[la].len())
    }

    /// The whole of one loop, when a ring follows it once around without a crossing.
    fn whole_loop(&self, labels: &[Chord]) -> Option<Vec<PathSeg>> {
        let lp = labels[0].0;
        let count = self.samples[lp].len();
        let travelled: usize = (0..labels.len())
            .map(|i| (labels[(i + 1) % labels.len()].1 + count - labels[i].1) % count)
            .sum();
        (travelled == count).then(|| self.loops[lp].clone())
    }

    /// Where the exact pieces under chords `a` (ending a run) and `b` (starting the next) cross
    /// near the ring's `vertex`: the place on each and the crossing point; `None` if they do not
    /// cross within the tolerance of the vertex.
    fn crossing(&self, a: Chord, b: Chord, vertex: Point) -> Option<(Place, Point, Place)> {
        let pa = self.chord_place(a.0, a.1, vertex);
        let pb = self.chord_place(b.0, b.1, vertex);
        let (first, second) = (self.loops[a.0][pa.piece], self.loops[b.0][pb.piece]);
        let (sa, sb) = solve_crossing(&first, pa.t, &second, pb.t);
        let (xa, xb) = (first.eval(sa), second.eval(sb));
        let point = xa.midpoint(xb);
        let reach = self.tolerance_mm;
        // The ring's vertex is the crossing of two flattened chords, each within the tolerance of
        // its curve: where the curves cross at a shallow angle it slides along them by up to
        // `reach / sin(angle)`, so the vertex may lie that far (at most four times `reach`).
        let (da, db) = (derivative(&first, sa), derivative(&second, sb));
        let lengths = da.hypot() * db.hypot();
        let sine = if lengths > 0.0 {
            (da.cross(db).abs() / lengths).max(0.25)
        } else {
            1.0
        };
        if (xa - xb).hypot() > reach || (point - vertex).hypot() > reach / sine {
            return None;
        }
        Some((
            Place {
                piece: pa.piece,
                t: sa,
            },
            point,
            Place {
                piece: pb.piece,
                t: sb,
            },
        ))
    }

    /// The place on loop `lp` along the chord from flattened vertex `index` to the next one that
    /// is nearest to `point`.
    fn chord_place(&self, lp: usize, index: usize, point: Point) -> Place {
        let samples = &self.samples[lp];
        let from = samples[index];
        let to = samples[(index + 1) % samples.len()];
        let end_t = if to.piece == from.piece { to.t } else { 1.0 };
        let (a, b) = (mm_of(from.point), mm_of(to.point));
        let chord = b - a;
        let length2 = chord.hypot2();
        let along = if length2 > 0.0 {
            ((point - a).dot(chord) / length2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Place {
            piece: from.piece,
            t: from.t + along * (end_t - from.t),
        }
    }

    /// The pieces of loop `lp` from `start` forward to `end`, cut at both.
    fn span(&self, lp: usize, start: Place, end: Place) -> Vec<PathSeg> {
        let pieces = &self.loops[lp];
        let count = pieces.len();
        let mut out = Vec::new();
        let wraps = end.piece < start.piece || (end.piece == start.piece && end.t < start.t);
        if !wraps && end.piece == start.piece {
            push_part(&mut out, &pieces[start.piece], start.t, end.t);
            return out;
        }
        push_part(&mut out, &pieces[start.piece], start.t, 1.0);
        let mut piece = (start.piece + 1) % count;
        let mut guard = 0;
        while piece != end.piece && guard <= count {
            out.push(pieces[piece]);
            piece = (piece + 1) % count;
            guard += 1;
        }
        push_part(&mut out, &pieces[end.piece], 0.0, end.t);
        out
    }
}

/// Pushes the part of `piece` from `t0` to `t1` unless it is a point.
fn push_part(out: &mut Vec<PathSeg>, piece: &PathSeg, t0: f64, t1: f64) {
    if t1 - t0 <= 1e-12 {
        return;
    }
    if t0 <= 0.0 && t1 >= 1.0 {
        out.push(*piece);
    } else {
        out.push(piece.subsegment(t0..t1));
    }
}

fn derivative(segment: &PathSeg, t: f64) -> Vec2 {
    match *segment {
        PathSeg::Line(line) => line.p1 - line.p0,
        PathSeg::Cubic(cubic) => cubic.deriv().eval(t).to_vec2(),
        PathSeg::Quad(quad) => quad.deriv().eval(t).to_vec2(),
    }
}

/// Newton's method on `a(s) = b(u)` from the guesses, parameters kept within the pieces.
#[allow(clippy::many_single_char_names)] // the textbook names of the 2 x 2 system
fn solve_crossing(a: &PathSeg, s0: f64, b: &PathSeg, u0: f64) -> (f64, f64) {
    let (mut s, mut u) = (s0, u0);
    for _ in 0..12 {
        let f = a.eval(s) - b.eval(u);
        if f.hypot() <= 1e-10 {
            break;
        }
        let (da, db) = (derivative(a, s), derivative(b, u));
        // a(s) - b(u) = 0, Jacobian [da, -db].
        let det = -da.cross(db);
        if det.abs() <= 1e-18 {
            break;
        }
        // Solve [da -db] [ds du]^T = -f by Cramer's rule.
        let ds = (-f).cross(-db) / det;
        let du = da.cross(-f) / det;
        s = (s + ds).clamp(0.0, 1.0);
        u = (u + du).clamp(0.0, 1.0);
    }
    (s, u)
}
