//! Where contours cross, touch or overlap each other and themselves, as parameters on the exact
//! curves (`specs/0036-cut-at-crossings`, criteria 2 to 4 and 11).
//!
//! Every segment of every contour is a cubic Bézier (a line is a cubic with both handles exactly
//! zero, and keeps its linear parameter, as `subdivide_at_parameter` splits it). Candidate pairs
//! of segments come from a sweep over their control-point boxes. A pair is subdivided at `t = 0.5`
//! (de Casteljau) until both pieces are flat within [`FLAT_MM`], and the chords of the flat pieces
//! are intersected exactly: a crossing is one point, chords closer than the touch distance meet
//! at their closest points, and collinear chords give the two ends of their shared stretch.
//! Overlap ends that chain along both curves merge into one stretch whose two ends are the cut
//! points. Only `+ - * /` and `sqrt` are used, so the result is bit-identical on every target
//! (`0036` criterion 11).
//!
//! The hit on a chord is off the true curve by at most the flat tolerance (0.0005 mm), far inside
//! the 0.01 mm the cut is promised to. Splitting at the reported parameter is exact: the halves
//! lie on the original curve.

use curvyo_document_core::{Point, Vec2};

use crate::curve_chords::{CUT_MERGE_MM, Chord, Cubic, chord_hit, dist, lerp};
use crate::curve_settle::settle;
use crate::outline_touch::TOUCH_DISTANCE_MM;
use crate::{Outline, OutlineTriple};

/// A piece is flat when its inner control points are this close to its chord (millimetres).
const FLAT_MM: f64 = 0.0005;

/// Boxes and chords this close can still meet: the touch distance plus two flat tolerances.
const REACH_MM: f64 = TOUCH_DISTANCE_MM + 2.0 * FLAT_MM;

/// Two segments of one contour with a run of at most this length between them are neighbours,
/// like the chords of `touching_outlines`.
const NEIGHBOUR_MM: f64 = 2.0 * TOUCH_DISTANCE_MM;

/// The recursion stops here whatever the flatness (a guard; a parameter step of 2^-40).
const MAX_DEPTH: u32 = 40;

/// A place on a contour: the segment (from node `segment` to the next one) and the parameter on
/// it. `t` is `0.0` exactly for a cut at a node, otherwise strictly between 0 and 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutParameter {
    /// The segment's index: the segment from node `segment` to node `segment + 1` (wrapping for
    /// the closing segment of a closed contour).
    pub segment: usize,
    /// The parameter on the segment: `0.0` is node `segment`.
    pub t: f64,
}

/// One segment of a contour.
pub(crate) struct Seg {
    pub(crate) curve: Cubic,
    /// Both handles are exactly zero: the parameter is linear along the chord.
    line: bool,
}

impl Seg {
    fn of(from: OutlineTriple, to: OutlineTriple) -> Self {
        let line = from.2 == Vec2::ZERO && to.1 == Vec2::ZERO;
        Self {
            curve: Cubic([
                from.0,
                from.0.translated(from.2),
                to.0.translated(to.1),
                to.0,
            ]),
            line,
        }
    }

    /// The velocity at parameter `t`, consistent with [`Seg::point_at`].
    fn velocity_at(&self, t: f64) -> (f64, f64) {
        let [start, c1, c2, end] = self.curve.0;
        if self.line {
            return (end.x - start.x, end.y - start.y);
        }
        let rest = 1.0 - t;
        let (w0, w1, w2) = (3.0 * rest * rest, 6.0 * rest * t, 3.0 * t * t);
        (
            w0 * (c1.x - start.x) + w1 * (c2.x - c1.x) + w2 * (end.x - c2.x),
            w0 * (c1.y - start.y) + w1 * (c2.y - c1.y) + w2 * (end.y - c2.y),
        )
    }

    /// The point at parameter `t`, as `subdivide_at_parameter` places it.
    pub(crate) fn point_at(&self, t: f64) -> Point {
        if self.line {
            lerp(self.curve.0[0], self.curve.0[3], t)
        } else {
            self.curve.eval(t)
        }
    }
}

/// The segments of a contour: one per pair of consecutive nodes, plus the closing one.
fn segments_of(outline: &Outline<'_>) -> Vec<Seg> {
    let anchors = outline.anchors;
    let count = anchors.len();
    if count < 2 {
        return Vec::new();
    }
    let n_seg = if outline.closed { count } else { count - 1 };
    (0..n_seg)
        .map(|k| Seg::of(anchors[k], anchors[(k + 1) % count]))
        .collect()
}

/// A piece of a segment: the sub-curve over `[t0, t1]` of the segment's parameter.
#[derive(Clone, Copy)]
struct Branch {
    curve: Cubic,
    t0: f64,
    t1: f64,
}

impl Branch {
    fn whole(seg: &Seg) -> Self {
        Self {
            curve: seg.curve,
            t0: 0.0,
            t1: 1.0,
        }
    }

    fn halves(&self) -> (Self, Self) {
        let (left, right) = self.curve.split_half();
        let centre = self.t0.midpoint(self.t1);
        (
            Self {
                curve: left,
                t0: self.t0,
                t1: centre,
            },
            Self {
                curve: right,
                t0: centre,
                t1: self.t1,
            },
        )
    }

    fn chord(&self) -> (Point, Point) {
        (self.curve.0[0], self.curve.0[3])
    }

    /// The segment's parameter at the fraction `u` of the chord: exact for a line, found by
    /// bisection on the progress along the chord otherwise (a piece is flat, so that is
    /// monotone).
    fn param_at(&self, seg: &Seg, u: f64) -> f64 {
        if u <= 0.0 {
            return self.t0;
        }
        if u >= 1.0 {
            return self.t1;
        }
        if seg.line {
            return self.t0 + u * (self.t1 - self.t0);
        }
        let (start, end) = self.chord();
        let (dx, dy) = (end.x - start.x, end.y - start.y);
        let length_squared = dx * dx + dy * dy;
        if length_squared <= 0.0 {
            return self.t0 + u * (self.t1 - self.t0);
        }
        let target = u * length_squared;
        let (mut low, mut high) = (self.t0, self.t1);
        for _ in 0..40 {
            let centre = low.midpoint(high);
            let p = seg.curve.eval(centre);
            if (p.x - start.x) * dx + (p.y - start.y) * dy < target {
                low = centre;
            } else {
                high = centre;
            }
        }
        low.midpoint(high)
    }
}

/// One end of a stretch, or a point: the parameters on the first and the second segment.
type Meeting = (f64, f64);

/// What two segments do to each other, before merging.
#[derive(Default)]
struct Hits {
    points: Vec<Meeting>,
    spans: Vec<(Meeting, Meeting)>,
}

/// A crossing found on chords made exact by Newton's method on `a(ta) - b(tb)`, so the two
/// contours are cut at the same point. A pair that does not converge (a touch, a tangent) keeps
/// the chord result.
fn refined(a: &Seg, b: &Seg, found: Meeting) -> Meeting {
    let miss = |m: Meeting| dist(a.point_at(m.0), b.point_at(m.1));
    let mut current = found;
    for _ in 0..6 {
        let (pa, pb) = (a.point_at(current.0), b.point_at(current.1));
        let f = (pb.x - pa.x, pb.y - pa.y);
        let (da, db) = (a.velocity_at(current.0), b.velocity_at(current.1));
        let det = db.0 * da.1 - da.0 * db.1;
        let scale = (da.0 * da.0 + da.1 * da.1).sqrt() * (db.0 * db.0 + db.1 * db.1).sqrt();
        if det.abs() <= 1e-6 * scale {
            break;
        }
        // Solves da * dt_a - db * dt_b = f.
        let step = (
            (f.0 * -db.1 + db.0 * f.1) / (da.0 * -db.1 + db.0 * da.1),
            (da.0 * f.1 - da.1 * f.0) / (da.0 * -db.1 + db.0 * da.1),
        );
        let next = (
            (current.0 + step.0).clamp(0.0, 1.0),
            (current.1 + step.1).clamp(0.0, 1.0),
        );
        if miss(next) >= miss(current) {
            break;
        }
        current = next;
    }
    if miss(current) < miss(found) && miss(current) <= 1e-9 {
        current
    } else {
        found
    }
}

/// Subdivides the pair until both pieces are flat, collecting what their chords do.
fn meet(a: &Seg, b: &Seg, pa: Branch, pb: Branch, depth: u32, hits: &mut Hits) {
    let (ba, bb) = (pa.curve.bounds(), pb.curve.bounds());
    if ba.0 > bb.1 + REACH_MM
        || bb.0 > ba.1 + REACH_MM
        || ba.2 > bb.3 + REACH_MM
        || bb.2 > ba.3 + REACH_MM
    {
        return;
    }
    let (dev_a, dev_b) = (pa.curve.deviation(), pb.curve.deviation());
    let (flat_a, flat_b) = (dev_a <= FLAT_MM, dev_b <= FLAT_MM);
    if (flat_a && flat_b) || depth >= MAX_DEPTH {
        let (p0, p1) = pa.chord();
        let (q0, q1) = pb.chord();
        let reach = TOUCH_DISTANCE_MM + dev_a + dev_b;
        match chord_hit(p0, p1, q0, q1, reach) {
            Chord::None => {}
            Chord::At(u, v) => {
                let found = (pa.param_at(a, u), pb.param_at(b, v));
                hits.points.push(refined(a, b, found));
            }
            Chord::Span((u0, v0), (u1, v1)) => hits.spans.push((
                (pa.param_at(a, u0), pb.param_at(b, v0)),
                (pa.param_at(a, u1), pb.param_at(b, v1)),
            )),
        }
        return;
    }
    let depth = depth + 1;
    match (flat_a, flat_b) {
        (false, false) => {
            let ((a0, a1), (b0, b1)) = (pa.halves(), pb.halves());
            for left in [a0, a1] {
                for right in [b0, b1] {
                    meet(a, b, left, right, depth, hits);
                }
            }
        }
        (false, true) => {
            let (a0, a1) = pa.halves();
            meet(a, b, a0, pb, depth, hits);
            meet(a, b, a1, pb, depth, hits);
        }
        _ => {
            let (b0, b1) = pb.halves();
            meet(a, b, pa, b0, depth, hits);
            meet(a, b, pa, b1, depth, hits);
        }
    }
}

/// The points where `hits` cut the two segments: shared stretches merged into their two ends, and
/// points inside a stretch dropped.
fn merged(a: &Seg, hits: &Hits) -> Vec<Meeting> {
    let mut spans: Vec<(Meeting, Meeting)> = hits
        .spans
        .iter()
        .map(|&(x, y)| if x.0 <= y.0 { (x, y) } else { (y, x) })
        .collect();
    spans.sort_by(|l, r| l.0.0.total_cmp(&r.0.0));
    let mut stretches: Vec<(Meeting, Meeting)> = Vec::new();
    for span in spans {
        match stretches.last_mut() {
            Some(last)
                if span.0.0 <= last.1.0
                    || dist(a.point_at(span.0.0), a.point_at(last.1.0)) <= CUT_MERGE_MM =>
            {
                if span.1.0 > last.1.0 {
                    last.1 = span.1;
                }
            }
            _ => stretches.push(span),
        }
    }
    let mut out: Vec<Meeting> = hits
        .points
        .iter()
        .copied()
        .filter(|point| {
            !stretches
                .iter()
                .any(|s| s.0.0 <= point.0 && point.0 <= s.1.0)
        })
        .collect();
    for (low, high) in stretches {
        out.push(low);
        out.push(high);
    }
    out
}

/// The sweep's entry for one segment: its box, grown so that boxes within reach overlap.
struct Item {
    id: SegmentId,
    bounds: (f64, f64, f64, f64),
}

/// Whether the polyline length between two segments of one contour is short enough that they are
/// neighbours, and the points where they join (meetings there are the join, not a cut).
fn junctions(outline: &Outline<'_>, along: &[f64], segs: &[Seg], i: usize, j: usize) -> Vec<Point> {
    let (low, high) = (i.min(j), i.max(j));
    let mut joins = Vec::new();
    let forward = along[high] - along[low + 1];
    if forward <= NEIGHBOUR_MM {
        joins.push(segs[low].curve.0[3]);
    }
    if outline.closed {
        let total = along[segs.len()];
        let wrap = along[low] + (total - along[high + 1]);
        if wrap <= NEIGHBOUR_MM {
            joins.push(segs[low].curve.0[0]);
        }
    }
    joins
}

/// The cumulative chord length at the start of each segment, and in total at the end.
fn cumulative(segs: &[Seg]) -> Vec<f64> {
    let mut along = Vec::with_capacity(segs.len() + 1);
    along.push(0.0);
    for seg in segs {
        let last = along[along.len() - 1];
        along.push(last + dist(seg.curve.0[0], seg.curve.0[3]));
    }
    along
}

/// The cuts at `contour` that a pair of segments makes: the meetings that are not just the
/// point where two neighbours join.
fn away_from(joins: &[Point], a: &Seg, b: &Seg, meeting: Meeting) -> bool {
    let (pa, pb) = (a.point_at(meeting.0), b.point_at(meeting.1));
    !joins
        .iter()
        .any(|&j| dist(pa, j) <= CUT_MERGE_MM && dist(pb, j) <= CUT_MERGE_MM)
}

/// Where each contour is cut by the others and by itself: per contour, the cut parameters in
/// order along it, cuts within 0.01 mm of each other merged, cuts within 0.001 mm of a
/// node made at the node, and the two end nodes of an open contour left out (a cut at an end adds
/// no piece). A closed contour that passes one point twice (a figure eight) has a cut for each
/// pass. Segments that share a stretch are cut at the two ends of it.
#[must_use]
pub fn cut_parameters(contours: &[Outline<'_>]) -> Vec<Vec<CutParameter>> {
    let segs: Vec<Vec<Seg>> = contours.iter().map(segments_of).collect();
    let along: Vec<Vec<f64>> = segs.iter().map(|s| cumulative(s)).collect();
    let mut raw: Vec<Vec<(usize, f64)>> = vec![Vec::new(); contours.len()];

    for (c, list) in segs.iter().enumerate() {
        for (k, seg) in list.iter().enumerate() {
            // A cubic can cross itself: its two halves meet away from their join.
            if !seg.line {
                let (left, right) = Branch::whole(seg).halves();
                let mut hits = Hits::default();
                meet(seg, seg, left, right, 0, &mut hits);
                let joins = [seg.point_at(0.5)];
                for (ta, tb) in merged(seg, &hits) {
                    if away_from(&joins, seg, seg, (ta, tb)) {
                        raw[c].push((k, ta));
                        raw[c].push((k, tb));
                    }
                }
            }
        }
    }
    for (first, second) in candidate_pairs(&segs) {
        let (a, b) = (&segs[first.0][first.1], &segs[second.0][second.1]);
        let mut hits = Hits::default();
        meet(a, b, Branch::whole(a), Branch::whole(b), 0, &mut hits);
        let joins = if first.0 == second.0 {
            junctions(
                &contours[first.0],
                &along[first.0],
                &segs[first.0],
                first.1,
                second.1,
            )
        } else {
            Vec::new()
        };
        for (ta, tb) in merged(a, &hits) {
            if away_from(&joins, a, b, (ta, tb)) {
                raw[first.0].push((first.1, ta));
                raw[second.0].push((second.1, tb));
            }
        }
    }

    contours
        .iter()
        .zip(&segs)
        .zip(&along)
        .zip(raw)
        .map(|(((outline, list), along), cuts)| settle(outline, list, along, cuts))
        .collect()
}

/// A segment as `(contour, segment)`.
type SegmentId = (usize, usize);

/// The pairs of segments whose boxes are within reach of each other, each pair once, found by a
/// sweep over the boxes sorted by their minimum x. The first of a pair is the smaller id.
fn candidate_pairs(segs: &[Vec<Seg>]) -> Vec<(SegmentId, SegmentId)> {
    let mut items: Vec<Item> = Vec::new();
    for (c, list) in segs.iter().enumerate() {
        for (k, seg) in list.iter().enumerate() {
            let b = seg.curve.bounds();
            let grow = REACH_MM * 0.5;
            items.push(Item {
                id: (c, k),
                bounds: (b.0 - grow, b.1 + grow, b.2 - grow, b.3 + grow),
            });
        }
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&l, &r| {
        items[l]
            .bounds
            .0
            .total_cmp(&items[r].bounds.0)
            .then(l.cmp(&r))
    });
    let mut pairs = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    for &index in &order {
        let item = &items[index];
        active.retain(|&held| items[held].bounds.1 >= item.bounds.0);
        for &held in &active {
            let other = &items[held];
            if other.bounds.2 <= item.bounds.3 && item.bounds.2 <= other.bounds.3 {
                pairs.push((other.id.min(item.id), other.id.max(item.id)));
            }
        }
        active.push(index);
    }
    pairs
}
