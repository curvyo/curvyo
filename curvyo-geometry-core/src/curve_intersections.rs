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
//! points, if the curves coincide along it. Curves that only come near each other (tangent
//! circles) touch instead: the meetings of one contact, crossings of a lens thinner than 0.01 mm
//! included, are one cut in the middle. Only `+ - * /` and `sqrt` are used, so the same input gives bit-identical cuts on every
//! target (`0036` criterion 11; primitive outlines come from `sin` and `cos` of the platform, so the
//! input itself can differ by an ulp).
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
    /// Crossings, exact to 1e-9 mm.
    points: Vec<Meeting>,
    /// Places where the curves come within reach of each other without crossing.
    touches: Vec<Meeting>,
    spans: Vec<(Meeting, Meeting)>,
}

/// Whether a piece of a cubic cannot cross itself: no side of its control polygon goes backwards
/// along the chord, so neither does the curve (a sufficient test).
fn cannot_loop(curve: &Cubic) -> bool {
    let [p0, p1, p2, p3] = curve.0;
    let chord = (p3.x - p0.x, p3.y - p0.y);
    if chord.0 == 0.0 && chord.1 == 0.0 {
        return false;
    }
    [(p0, p1), (p1, p2), (p2, p3)]
        .iter()
        .all(|&(from, to)| (to.x - from.x) * chord.0 + (to.y - from.y) * chord.1 >= 0.0)
}

/// Where one cubic crosses itself: a piece that cannot loop is done; otherwise its halves are met
/// away from their join, and each half is searched again (a loop can lie inside one half).
fn self_meet(seg: &Seg, piece: Branch, depth: u32, out: &mut Vec<Meeting>) {
    if depth >= MAX_DEPTH || cannot_loop(&piece.curve) {
        return;
    }
    let (left, right) = piece.halves();
    let mut hits = Hits::default();
    meet(seg, seg, left, right, depth + 1, &mut hits);
    let joins = [seg.point_at(left.t1)];
    out.extend(
        merged(seg, seg, &hits)
            .into_iter()
            .filter(|&m| away_from(&joins, seg, seg, m)),
    );
    self_meet(seg, left, depth + 1, out);
    self_meet(seg, right, depth + 1, out);
}

/// A crossing found on chords made exact by Newton's method on `a(ta) - b(tb)`, so the two
/// contours are cut at the same point. A pair that does not converge (a touch, a tangent) keeps
/// the chord result. The flag says the result is a crossing, exact to 1e-9 mm.
fn refined(a: &Seg, b: &Seg, found: Meeting) -> (Meeting, bool) {
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
        (current, true)
    } else {
        (found, miss(found) <= 1e-9)
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
                match refined(a, b, found) {
                    (meeting, true) => hits.points.push(meeting),
                    (meeting, false) => hits.touches.push(meeting),
                }
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
fn merged(a: &Seg, b: &Seg, hits: &Hits) -> Vec<Meeting> {
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
    // Curves that only come near each other (tangent circles) touch along a stretch without
    // coinciding. Meetings of one contact (the curves stay within reach between them) are cut as
    // one: at its crossings if it has any, else once in the middle of its touches.
    let mut out: Vec<Meeting> = Vec::new();
    let mut meetings: Vec<(Meeting, bool)> = hits
        .points
        .iter()
        .map(|&m| (m, true))
        .chain(hits.touches.iter().map(|&m| (m, false)))
        .filter(|(m, _)| !stretches.iter().any(|s| s.0.0 <= m.0 && m.0 <= s.1.0))
        .collect();
    for &(low, high) in &stretches {
        if doubled(a, b, low, high) {
            out.push(low);
            out.push(high);
        } else {
            meetings.push((low, false));
            meetings.push((high, false));
        }
    }
    meetings.sort_by(|l, r| l.0.0.total_cmp(&r.0.0));
    let mut contacts: Vec<Vec<(Meeting, bool)>> = Vec::new();
    for meeting in meetings {
        match contacts.last_mut() {
            Some(last) if stays_near(a, b, last[last.len() - 1].0, meeting.0) => {
                last.push(meeting);
            }
            _ => contacts.push(vec![meeting]),
        }
    }
    for contact in contacts {
        let crossings: Vec<Meeting> = contact.iter().filter(|m| m.1).map(|m| m.0).collect();
        if crossings.is_empty() {
            out.push(middle(contact[0].0, contact[contact.len() - 1].0));
            continue;
        }
        // Crossings of a lens thinner than the cut tolerance (0.01 mm) (two circles that touch from
        // outside cross twice, a tenth of a millimetre apart, by their own rounding) are one.
        let mut group = vec![crossings[0]];
        for &next in &crossings[1..] {
            if stays_within(a, b, group[group.len() - 1], next, CUT_MERGE_MM) {
                group.push(next);
            } else {
                out.push(middle(group[0], group[group.len() - 1]));
                group = vec![next];
            }
        }
        out.push(middle(group[0], group[group.len() - 1]));
    }
    out
}

/// The meeting half way between two.
fn middle(first: Meeting, last: Meeting) -> Meeting {
    (first.0.midpoint(last.0), first.1.midpoint(last.1))
}

/// The distance from `point` to the segment `b`, found by Newton's method from the parameter
/// `guess`.
fn gap_to(b: &Seg, point: Point, guess: f64) -> f64 {
    let mut t = guess;
    for _ in 0..6 {
        let p = b.point_at(t);
        let v = b.velocity_at(t);
        let speed = v.0 * v.0 + v.1 * v.1;
        if speed <= 0.0 {
            break;
        }
        t = (t - ((p.x - point.x) * v.0 + (p.y - point.y) * v.1) / speed).clamp(0.0, 1.0);
    }
    dist(point, b.point_at(t))
}

/// Whether the curve `a` stays within `limit` of `b` along the stretch from `low` to `high`,
/// judged at its quarter points.
fn stays_within(a: &Seg, b: &Seg, low: Meeting, high: Meeting, limit: f64) -> bool {
    [0.25, 0.5, 0.75].iter().all(|&q| {
        let on_a = a.point_at(low.0 + q * (high.0 - low.0));
        gap_to(b, on_a, low.1 + q * (high.1 - low.1)) <= limit
    })
}

/// Whether the curves coincide along the stretch from `low` to `high` (to within 1e-6 mm), as
/// doubled lines and identical circles do. Curves that merely run close, like two circles
/// touching from outside, do not.
fn doubled(a: &Seg, b: &Seg, low: Meeting, high: Meeting) -> bool {
    stays_within(a, b, low, high, 1e-6)
}

/// Whether two meetings belong to one contact: the curves stay within the cut tolerance of each
/// other between them.
fn stays_near(a: &Seg, b: &Seg, low: Meeting, high: Meeting) -> bool {
    stays_within(a, b, low, high, CUT_MERGE_MM)
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
            // A cubic can cross itself.
            if !seg.line {
                let mut found = Vec::new();
                self_meet(seg, Branch::whole(seg), 0, &mut found);
                for (ta, tb) in found {
                    raw[c].push((k, ta));
                    raw[c].push((k, tb));
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
        for (ta, tb) in merged(a, b, &hits) {
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
