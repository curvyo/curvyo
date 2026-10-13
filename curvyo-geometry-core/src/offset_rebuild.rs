//! Turns raw offset loops into result outlines of exact pieces: `i_overlay` decides which parts of
//! the loops survive (Positive fill rule on their flattened, snapped vertices), and every surviving
//! ring is mapped back onto the pieces it came from (`specs/0038-path-offset` criteria 33, 34, 39;
//! ADR 0003 §4 as amended on 2026-10-12).
//!
//! Each flattened vertex carries a tag (its loop, piece and parameter), found again in a ring by
//! its exact grid coordinates. Consecutive tagged vertices of one loop form a run, which is written
//! as the loop's own pieces; between runs lies a crossing, which is solved on the two exact pieces.
//! A ring that cannot be mapped within the tolerance is written as its polyline (the fallback).

use std::collections::HashMap;

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{IntOverlayOptions, Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::core::solver::Solver;
use kurbo::{CubicBez, Line, ParamCurve, ParamCurveDeriv, PathSeg, Point, Vec2};

use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{Path64, Paths64, Point64};

/// Grid units per millimetre.
const UNITS_PER_MM: f64 = 1000.0;

/// At most this many flattening steps per cubic piece.
const MAX_STEPS: f64 = 4096.0;

/// Nodes closer than this, in millimetres, are merged (criterion 33).
const MIN_NODE_SPACING_MM: f64 = 0.001;

/// A node this close to the line between its straight neighbours is dropped, in millimetres.
const COLLINEAR_MM: f64 = 1e-6;

/// The sine of the largest tangent difference of a smooth node: 0.1° (criterion 33).
const SMOOTH_SINE: f64 = 0.001_745_328_365_898_309;

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

/// Overlays `loops` with the Positive rule and maps every surviving ring back onto the loops'
/// pieces. `flatten_mm` is the flattening tolerance, `tolerance_mm` the largest distance a mapped
/// outline may lie from its ring. `keep` decides which rings count (the caller drops slivers).
pub(crate) fn rebuild(
    loops: &[Vec<PathSeg>],
    flatten_mm: f64,
    tolerance_mm: f64,
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
        .overlay(OverlayRule::Subject, FillRule::Positive)
        .into_iter()
        .flatten()
        .collect();
    let context = Context {
        loops,
        samples: &samples,
        tags: &tags,
        tolerance_mm,
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
                    .extend(polyline.iter().map(|path| lines_of(path)));
            }
        }
    }
    rebuilt
}

/// A grid polygon as closed lines in millimetres.
fn lines_of(path: &[Point64]) -> Vec<PathSeg> {
    (0..path.len())
        .map(|i| PathSeg::Line(Line::new(to_mm(path[i]), to_mm(path[(i + 1) % path.len()]))))
        .collect()
}

#[allow(clippy::cast_precision_loss)] // |units| <= 10^10, exact in an f64
fn to_mm(point: Point64) -> Point {
    Point::new(point.x as f64 / UNITS_PER_MM, point.y as f64 / UNITS_PER_MM)
}

fn snap(point: Point) -> Point64 {
    // Inside the kernel's range after the caller's checks, so the casts are exact.
    #[allow(clippy::cast_possible_truncation)]
    Point64::new(
        (point.x * UNITS_PER_MM).round() as i64,
        (point.y * UNITS_PER_MM).round() as i64,
    )
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
            let point = snap(segment.eval(t));
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
            let vertex = to_mm(ring[next_first]);
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
    /// suggest, then every chord.
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
        (0..self.samples.len())
            .flat_map(|lp| (0..self.samples[lp].len()).map(move |index| (lp, index)))
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
        if (xa - xb).hypot() > reach || (point - vertex).hypot() > reach {
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
        let (a, b) = (to_mm(from.point), to_mm(to.point));
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

fn set_start(segment: &mut PathSeg, point: Point) {
    match segment {
        PathSeg::Line(line) => line.p0 = point,
        PathSeg::Cubic(cubic) => {
            let shift = point - cubic.p0;
            cubic.p0 = point;
            cubic.p1 += shift;
        }
        PathSeg::Quad(quad) => quad.p0 = point,
    }
}

fn set_end(segment: &mut PathSeg, point: Point) {
    match segment {
        PathSeg::Line(line) => line.p1 = point,
        PathSeg::Cubic(cubic) => {
            let shift = point - cubic.p3;
            cubic.p3 = point;
            cubic.p2 += shift;
        }
        PathSeg::Quad(quad) => quad.p2 = point,
    }
}

fn start_point(segment: &PathSeg) -> Point {
    segment.eval(0.0)
}

fn end_point(segment: &PathSeg) -> Point {
    segment.eval(1.0)
}

/// The size of a segment: the largest distance of a control point from its start.
fn extent(segment: &PathSeg) -> f64 {
    match *segment {
        PathSeg::Line(line) => (line.p1 - line.p0).hypot(),
        PathSeg::Cubic(c) => (c.p1 - c.p0)
            .hypot()
            .max((c.p2 - c.p0).hypot())
            .max((c.p3 - c.p0).hypot()),
        PathSeg::Quad(q) => (q.p1 - q.p0).hypot().max((q.p2 - q.p0).hypot()),
    }
}

/// The output rules of criterion 33 on a closed list of pieces: no two nodes closer than
/// 0.001 mm, no node on the straight line between two straight neighbours, every joint closed.
pub(crate) fn finish(pieces: &[PathSeg]) -> Vec<PathSeg> {
    let mut out: Vec<PathSeg> = Vec::with_capacity(pieces.len());
    for piece in pieces {
        if extent(piece) < MIN_NODE_SPACING_MM {
            continue;
        }
        if let Some(previous) = out.last() {
            let joint = end_point(previous);
            let mut piece = *piece;
            set_start(&mut piece, joint);
            out.push(piece);
        } else {
            out.push(*piece);
        }
    }
    // Merge straight runs and close the outline.
    let mut merged: Vec<PathSeg> = Vec::with_capacity(out.len());
    for piece in out {
        if let (Some(PathSeg::Line(previous)), PathSeg::Line(line)) = (merged.last(), piece)
            && is_straight_through(previous.p0, previous.p1, line.p1)
        {
            let start = previous.p0;
            merged.pop();
            merged.push(PathSeg::Line(Line::new(start, line.p1)));
            continue;
        }
        merged.push(piece);
    }
    while merged.len() > 2 {
        let (PathSeg::Line(last), PathSeg::Line(first)) = (merged[merged.len() - 1], merged[0])
        else {
            break;
        };
        if !is_straight_through(last.p0, first.p0, first.p1) {
            break;
        }
        merged.pop();
        merged[0] = PathSeg::Line(Line::new(last.p0, first.p1));
    }
    if let (Some(first), Some(last)) = (merged.first().copied(), merged.last_mut()) {
        let joint = start_point(&first);
        if (end_point(last) - joint).hypot() < MIN_NODE_SPACING_MM {
            set_end(last, joint);
        } else {
            let from = end_point(last);
            merged.push(PathSeg::Line(Line::new(from, joint)));
        }
    }
    merged
}

/// Whether `middle` lies on the straight line from `a` to `b`, between them.
fn is_straight_through(a: Point, middle: Point, b: Point) -> bool {
    let chord = b - a;
    let length = chord.hypot();
    if length <= COLLINEAR_MM {
        return false;
    }
    let off = (middle - a).cross(chord).abs() / length;
    let along = (middle - a).dot(chord);
    off <= COLLINEAR_MM && along > 0.0 && along < chord.hypot2()
}

/// Whether the joint from `before` into `after` is smooth: the tangents differ by at most 0.1°.
pub(crate) fn is_smooth(before: &PathSeg, after: &PathSeg) -> bool {
    let (u1, u2) = (
        crate::offset_pieces::end_tangent(before),
        crate::offset_pieces::start_tangent(after),
    );
    u1.dot(u2) > 0.0 && u1.cross(u2).abs() <= SMOOTH_SINE
}

/// Twice the area of a closed list of pieces in square millimetres, positive for an outer
/// outline: the polygon of the end points plus each cubic's area against its chord.
pub(crate) fn double_area_mm2(pieces: &[PathSeg]) -> f64 {
    let mut sum = 0.0;
    for piece in pieces {
        let (a, b) = (start_point(piece), end_point(piece));
        sum += a.x * b.y - b.x * a.y;
        if let PathSeg::Cubic(c) = *piece {
            sum += 2.0 * cubic_chord_area(&c);
        }
    }
    sum
}

/// The signed area between a cubic and its chord (positive for a bulge to the left).
fn cubic_chord_area(c: &CubicBez) -> f64 {
    // Green's theorem on the cubic minus the chord's own term, in closed form.
    let (p0, p1, p2, p3) = (
        c.p0.to_vec2(),
        c.p1.to_vec2(),
        c.p2.to_vec2(),
        c.p3.to_vec2(),
    );
    let whole = (p0.cross(p1) * 6.0
        + p0.cross(p2) * 3.0
        + p0.cross(p3)
        + p1.cross(p2) * 3.0
        + p1.cross(p3) * 3.0
        + p2.cross(p3) * 6.0)
        / 20.0;
    whole - p0.cross(p3) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_area_of_a_cubic_circle_is_near_pi_r_squared() {
        let k = 0.552_284_749_830_793_4 * 10.0;
        let p = |x: f64, y: f64| Point::new(x, y);
        let quarter = [
            CubicBez::new(p(10.0, 0.0), p(10.0, k), p(k, 10.0), p(0.0, 10.0)),
            CubicBez::new(p(0.0, 10.0), p(-k, 10.0), p(-10.0, k), p(-10.0, 0.0)),
            CubicBez::new(p(-10.0, 0.0), p(-10.0, -k), p(-k, -10.0), p(0.0, -10.0)),
            CubicBez::new(p(0.0, -10.0), p(k, -10.0), p(10.0, -k), p(10.0, 0.0)),
        ];
        let pieces: Vec<PathSeg> = quarter.into_iter().map(PathSeg::Cubic).collect();
        let area = double_area_mm2(&pieces) / 2.0;
        assert!((area - std::f64::consts::PI * 100.0).abs() < 0.1, "{area}");
    }

    #[test]
    fn straight_runs_merge_and_tiny_pieces_go() {
        let p = |x: f64, y: f64| Point::new(x, y);
        let pieces = [
            PathSeg::Line(Line::new(p(0.0, 0.0), p(5.0, 0.0))),
            PathSeg::Line(Line::new(p(5.0, 0.0), p(10.0, 0.0))),
            PathSeg::Line(Line::new(p(10.0, 0.0), p(10.0, 0.0005))),
            PathSeg::Line(Line::new(p(10.0, 0.0005), p(10.0, 10.0))),
            PathSeg::Line(Line::new(p(10.0, 10.0), p(0.0, 10.0))),
            PathSeg::Line(Line::new(p(0.0, 10.0), p(0.0, 0.0))),
        ];
        let finished = finish(&pieces);
        assert_eq!(finished.len(), 4, "{finished:?}");
    }
}
