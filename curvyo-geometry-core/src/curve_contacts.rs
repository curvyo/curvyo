//! Groups the meetings of two contours into contacts and cuts each contact once
//! (`specs/0036-cut-at-crossings`, criteria 2 and 4).
//!
//! A contact is a run of meetings between which the two curves stay within the cut tolerance
//! (0.01 mm) of each other, wherever the segments start and end. Curves that only come near each
//! other (tangent circles, inside or outside) touch along a stretch; two curves that cross at a
//! shallow angle come within reach of each other before and after the crossing, also in the
//! neighbouring segments. Such a contact is cut once: at its crossings, where a lens thinner than
//! 0.01 mm is cut at its clear crossings, or once in its middle when all its crossings are
//! shallow, or in the middle of its touches when it has no crossing.
//! Only `+ - * /` and `sqrt` are used, and every sort has a total order, so the result does not
//! depend on the platform.

use curvyo_document_core::Point;

use crate::curve_chords::{CUT_MERGE_MM, dist};
use crate::curve_intersections::Seg;

/// Two curves cross at a clear angle when the sine of the angle between them is at least this
/// (about 5.7 degrees): such a crossing is a cut even inside a lens.
const CLEAR_SINE: f64 = 0.1;

/// A place on a contour: the segment and the parameter on it.
pub(crate) type Place = (usize, f64);

/// One meeting of two contours, before grouping.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Candidate {
    /// The place on the first contour.
    pub(crate) a: Place,
    /// The place on the second contour.
    pub(crate) b: Place,
    /// Whether the curves cross there (exact to 1e-9 mm), not only come near.
    pub(crate) crossing: bool,
}

/// A contour as the grouping reads it: its segments and whether it is closed.
pub(crate) type Contour<'a> = (&'a [Seg], bool);

/// A meeting in unrolled parameters: `segment + t` on each contour.
#[derive(Debug, Clone, Copy)]
struct Unrolled {
    a: f64,
    b: f64,
    crossing: bool,
}

/// `f64` index of a segment; the counts here are far below 2^53.
#[allow(clippy::cast_precision_loss)]
fn index(segment: usize) -> f64 {
    segment as f64
}

/// The place of the unrolled parameter `s` on a contour of `count` segments: a closed contour
/// wraps, an open one is clamped to its ends.
fn place_of(s: f64, count: usize, closed: bool) -> Place {
    let n = index(count);
    let s = if closed {
        s.rem_euclid(n)
    } else {
        s.clamp(0.0, n)
    };
    // A non-negative float below 2^53: the cast cannot wrap.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let segment = (s.floor() as usize).min(count - 1);
    (segment, s - index(segment))
}

/// The unrolled parameter a fraction `f` of the way from `from` to `to` on a contour; on a closed
/// contour the shorter way round.
fn between(from: f64, to: f64, f: f64, count: usize, closed: bool) -> f64 {
    let n = index(count);
    let mut step = to - from;
    if closed && step.abs() > n * 0.5 {
        step -= n * step.signum();
    }
    from + f * step
}

/// The distance from `point` to the segment `b`, found by Newton's method from the parameter
/// `guess`.
pub(crate) fn gap_to(b: &Seg, point: Point, guess: f64) -> f64 {
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

/// The distance from `point` to the contour `b` near the unrolled parameter `guess`: the segment
/// there and its two neighbours.
fn gap_near(b: Contour<'_>, same: bool, point: Point, guess: f64) -> f64 {
    let (segs, closed) = b;
    let count = segs.len();
    let (segment, t) = place_of(guess, count, closed);
    let mut best = gap_to(&segs[segment], point, t);
    if same {
        return best;
    }
    let neighbours = [(segment + count - 1) % count, (segment + 1) % count];
    for (k, other) in neighbours.into_iter().enumerate() {
        let wraps = if k == 0 {
            other > segment
        } else {
            other < segment
        };
        if !closed && wraps {
            continue;
        }
        let start = if k == 0 { 1.0 } else { 0.0 };
        best = best.min(gap_to(&segs[other], point, start));
    }
    best
}

/// Whether the curves stay within `limit` of each other between two meetings, judged at the
/// quarter points of both stretches: each curve's stretch must stay near the other curve. One
/// direction is not enough: a curve that passes close to where another crosses itself meets both
/// passes of it, and stays near the other curve all the way, while the stretch of the other curve
/// between the two meetings is its whole loop.
/// `same` says the two contours are one (a contour meeting itself): then the distance is measured
/// on the guessed segment only, from the guessed place, so that it finds the other pass and not
/// the point itself.
fn stays_within(
    (a, b, same): (Contour<'_>, Contour<'_>, bool),
    from: Unrolled,
    to: Unrolled,
    limit: f64,
) -> bool {
    if same && passes_overlap(a, from, to) {
        // The two passes of one contour run over the same stretch here (a small loop): the
        // distance would find the point itself.
        return false;
    }
    let swap = |m: Unrolled| Unrolled {
        a: m.b,
        b: m.a,
        ..m
    };
    stretch_near((a, b, same), from, to, limit)
        && stretch_near((b, a, same), swap(from), swap(to), limit)
}

/// Whether the stretch of contour `a` between two meetings stays within `limit` of contour `b`,
/// judged at its quarter points.
fn stretch_near(
    (a, b, same): (Contour<'_>, Contour<'_>, bool),
    from: Unrolled,
    to: Unrolled,
    limit: f64,
) -> bool {
    [0.25, 0.5, 0.75].iter().all(|&f| {
        let (segment, t) = place_of(between(from.a, to.a, f, a.0.len(), a.1), a.0.len(), a.1);
        let on_a = a.0[segment].point_at(t);
        let guess = between(from.b, to.b, f, b.0.len(), b.1);
        gap_near(b, same, on_a, guess) <= limit
    })
}

/// Whether, on one contour, the stretch between the first places of two meetings and the stretch
/// between their second places overlap (with a closed contour, also round its start).
fn passes_overlap(contour: Contour<'_>, from: Unrolled, to: Unrolled) -> bool {
    let (lo_a, hi_a) = (from.a.min(to.a), from.a.max(to.a));
    let (lo_b, hi_b) = (from.b.min(to.b), from.b.max(to.b));
    let n = index(contour.0.len());
    let shifts: &[f64] = if contour.1 { &[-1.0, 0.0, 1.0] } else { &[0.0] };
    shifts
        .iter()
        .any(|&k| lo_a.max(lo_b + k * n) <= hi_a.min(hi_b + k * n))
}

/// The meeting half way between two, mapped back to places.
fn middle(a: Contour<'_>, b: Contour<'_>, first: Unrolled, last: Unrolled) -> (Place, Place) {
    (
        place_of(
            between(first.a, last.a, 0.5, a.0.len(), a.1),
            a.0.len(),
            a.1,
        ),
        place_of(
            between(first.b, last.b, 0.5, b.0.len(), b.1),
            b.0.len(),
            b.1,
        ),
    )
}

/// The cuts one contact makes: its crossings, where the crossings of a lens count as one lens,
/// or the middle of its touches when it has no crossing.
fn cuts_of(
    (a, b, same): (Contour<'_>, Contour<'_>, bool),
    contact: &[Unrolled],
    out: &mut Vec<(Place, Place)>,
) {
    let crossings: Vec<Unrolled> = contact.iter().copied().filter(|m| m.crossing).collect();
    if crossings.is_empty() {
        out.push(middle(a, b, contact[0], contact[contact.len() - 1]));
        return;
    }
    let mut lens = vec![crossings[0]];
    for &next in &crossings[1..] {
        // A contour that crosses itself twice close by has two crossings, not a lens.
        if !same && stays_within((a, b, same), lens[lens.len() - 1], next, CUT_MERGE_MM) {
            lens.push(next);
        } else {
            lens_cuts(a, b, &lens, out);
            lens = vec![next];
        }
    }
    lens_cuts(a, b, &lens, out);
}

/// The cuts of a lens (crossings between which the curves stay within 0.01 mm of each other):
/// each clear crossing, as the curves really part there; a lens of shallow crossings only (two
/// curves that nearly touch) is cut once, in its middle.
fn lens_cuts(a: Contour<'_>, b: Contour<'_>, lens: &[Unrolled], out: &mut Vec<(Place, Place)>) {
    let before = out.len();
    for &m in lens {
        let (at_a, at_b) = (place_of(m.a, a.0.len(), a.1), place_of(m.b, b.0.len(), b.1));
        if clear(&a.0[at_a.0], &b.0[at_b.0], at_a.1, at_b.1) {
            out.push((at_a, at_b));
        }
    }
    if out.len() == before {
        out.push(middle(a, b, lens[0], lens[lens.len() - 1]));
    }
}

/// Whether two segments cross at a clear angle at the parameters `ta` and `tb`: the sine of the
/// angle between their directions is at least [`CLEAR_SINE`].
fn clear(a: &Seg, b: &Seg, ta: f64, tb: f64) -> bool {
    let (da, db) = (a.velocity_at(ta), b.velocity_at(tb));
    let scale = (da.0 * da.0 + da.1 * da.1).sqrt() * (db.0 * db.0 + db.1 * db.1).sqrt();
    scale > 0.0 && (da.0 * db.1 - da.1 * db.0).abs() >= CLEAR_SINE * scale
}

/// The cuts that the meetings `list` of the contours `a` and `b` make, one or more per contact.
pub(crate) fn contact_cuts(
    a: Contour<'_>,
    b: Contour<'_>,
    same: bool,
    list: Vec<Candidate>,
) -> Vec<(Place, Place)> {
    let mut meetings: Vec<Unrolled> = list
        .into_iter()
        .map(|c| Unrolled {
            a: index(c.a.0) + c.a.1,
            b: index(c.b.0) + c.b.1,
            crossing: c.crossing,
        })
        .collect();
    meetings.sort_by(|l, r| l.a.total_cmp(&r.a).then(l.b.total_cmp(&r.b)));
    let mut contacts: Vec<Vec<Unrolled>> = Vec::new();
    for meeting in meetings {
        match contacts.last_mut() {
            Some(open)
                if stays_within((a, b, same), open[open.len() - 1], meeting, CUT_MERGE_MM) =>
            {
                open.push(meeting);
            }
            _ => contacts.push(vec![meeting]),
        }
    }
    // On a closed first contour the last contact may run on over its start into the first.
    if a.1 && contacts.len() > 1 {
        let tail = contacts[contacts.len() - 1][contacts[contacts.len() - 1].len() - 1];
        let mut head = contacts[0][0];
        head.a += index(a.0.len());
        if stays_within((a, b, same), tail, head, CUT_MERGE_MM) {
            // invariant: `contacts` has more than one entry.
            #[allow(clippy::unwrap_used)]
            let mut joined = contacts.pop().unwrap();
            joined.extend(contacts[0].iter().map(|m| Unrolled {
                a: m.a + index(a.0.len()),
                ..*m
            }));
            contacts[0] = joined;
        }
    }
    let mut out = Vec::new();
    for contact in &contacts {
        cuts_of((a, b, same), contact, &mut out);
    }
    out
}
