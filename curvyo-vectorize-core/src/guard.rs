//! Finds the fitted outlines that cross themselves or come too close to another outline
//! (`specs/0089` adrs.md §5, criterion 10), so the trace can fall back for them.
//!
//! Every outline is flattened to 0.02 px; segments are put into 4 px cells by a sort (no hash
//! map, no grid of the whole picture) and every pair in a cell is tested in basic operations.
//! Segments of one outline conflict when they cross, touch, or come within [`SELF_GAP`] while
//! more than [`SELF_APART`] apart along the outline (the two sides of an acute tip come
//! arbitrarily close); segments of two outlines when they cross or come within
//! [`GAP`] (only a crossing counts where they share an end point, as pixel borders touching at
//! a corner do). The gaps cover the coarser flattening of the harness (0.05 px).

use kurbo::Point;

use crate::fit::Seg;
use crate::line::{distance, segment_distance};

/// The flattening tolerance, px.
const FLATTEN_PX: f64 = 0.02;

/// Two outlines must stay this far apart, px.
const GAP: f64 = 0.1;

/// Two parts of one outline closer than this touch, px (more than the flattening, so a
/// crossing of two cubics at a kink is found whatever the flattening of the harness).
const TOUCH: f64 = 0.03;

/// Two parts of one outline more than [`SELF_APART`] apart along it must stay this far apart,
/// px.
const SELF_GAP: f64 = 0.06;

/// The arc length along an outline beyond which its parts count as apart, px.
const SELF_APART: f64 = 1.0;

/// The cell size of the pair search, px.
const CELL_PX: f64 = 4.0;

/// The flattened points of an outline (a closed one's first point not repeated, an open one's
/// end point last), each with the index of the segment it starts a piece of.
fn flatten(outline: &[Seg], open: bool) -> Vec<(Point, u32)> {
    let mut points = Vec::new();
    for (index, seg) in outline.iter().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        match *seg {
            Seg::Line(a, _) => points.push((a, index)),
            Seg::Cubic(p0, p1, p2, p3) => {
                let d1 = p0.to_vec2() - p1.to_vec2() * 2.0 + p2.to_vec2();
                let d2 = p1.to_vec2() - p2.to_vec2() * 2.0 + p3.to_vec2();
                let bend = d1.dot(d1).max(d2.dot(d2)).sqrt();
                // invariant: a cubic of one outline bends by at most a few thousand pixels.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let steps = ((0.75 * bend / FLATTEN_PX).sqrt().ceil() as usize).clamp(1, 4096);
                for s in 0..steps {
                    #[allow(clippy::cast_precision_loss)]
                    let t = s as f64 / steps as f64;
                    points.push((crate::fit::eval(&[p0, p1, p2, p3], t), index));
                }
            }
        }
    }
    if open && let Some(last) = outline.last() {
        let index = u32::try_from(outline.len() - 1).unwrap_or(u32::MAX);
        points.push((
            match *last {
                Seg::Line(_, b) | Seg::Cubic(_, _, _, b) => b,
            },
            index,
        ));
    }
    points
}

/// The orientation of `r` against the line `p`→`q`.
fn orient(p: Point, q: Point, r: Point) -> f64 {
    (q - p).cross(r - p)
}

/// Whether `a`–`b` and `c`–`d` cross properly.
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    orient(a, b, c) * orient(a, b, d) < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0
}

/// The distance between the segments `a`–`b` and `c`–`d` when they do not cross.
fn gap(a: Point, b: Point, c: Point, d: Point) -> f64 {
    segment_distance(a, c, d)
        .min(segment_distance(b, c, d))
        .min(segment_distance(c, a, b))
        .min(segment_distance(d, a, b))
}

/// One flattened segment: its outline, its index among the outline's flattened segments, the
/// outline segment it belongs to, the arc length along the outline where it starts, its ends.
#[derive(Clone, Copy)]
struct Flat {
    outline: u32,
    index: u32,
    seg: u32,
    at: f64,
    a: Point,
    b: Point,
}

/// Whether two flattened segments conflict; `lengths` holds each outline's segment count and
/// arc length; between two `open` parts only a crossing counts (shared edges of a multicolour
/// trace end on common junctions and may touch).
fn conflict(one: &Flat, other: &Flat, lengths: &[(u32, f64)], open: bool) -> bool {
    let (a, b, c, d) = (one.a, one.b, other.a, other.b);
    let shared = a == c || a == d || b == c || b == d;
    let crossing = crosses(a, b, c, d);
    if one.outline == other.outline {
        let (count, length) = lengths[one.outline as usize];
        let steps = one.index.abs_diff(other.index);
        if steps <= 1 || steps + 1 == count || shared {
            return false;
        }
        // Apart along the outline but close: a touch that may cross once mapped and flattened
        // more coarsely. Close along the outline too: the two sides of an acute tip.
        let along = (one.at - other.at).abs();
        let apart = along.min(length - along) > SELF_APART;
        let space = gap(a, b, c, d);
        return crossing || space < TOUCH || (apart && space < SELF_GAP);
    }
    // Pixel borders touch where pixels meet at a corner, both turning there: a shared end
    // point. Any other touch may cross once mapped to millimetres.
    crossing || (!open && !shared && gap(a, b, c, d) < GAP)
}

/// For every outline (closed, in pixels), the indices of its segments that conflict with
/// another part of it or another outline, ascending.
pub(crate) fn conflicts(outlines: &[Vec<Seg>]) -> Vec<Vec<usize>> {
    find(outlines, false)
}

/// For every open part (the fitted spans of a multicolour trace's edges, in pixels), the
/// indices of its segments that cross another part, or cross or touch itself.
pub(crate) fn crossings(parts: &[Vec<Seg>]) -> Vec<Vec<usize>> {
    find(parts, true)
}

/// [`conflicts`] of closed outlines, or [`crossings`] of `open` parts.
fn find(outlines: &[Vec<Seg>], open: bool) -> Vec<Vec<usize>> {
    let mut flats = Vec::new();
    let mut lengths: Vec<(u32, f64)> = Vec::with_capacity(outlines.len());
    for (o, outline) in outlines.iter().enumerate() {
        // Repeated points (a zero-length piece) are left out, so neighbours stay neighbours.
        let mut points = flatten(outline, open);
        points.dedup_by(|b, a| a.0 == b.0);
        while !open && points.len() > 1 && points[0].0 == points[points.len() - 1].0 {
            points.pop();
        }
        let n = points.len();
        let mut at = 0.0;
        let pieces = if open { n.saturating_sub(1) } else { n };
        for i in 0..pieces {
            let ((a, seg), (b, _)) = (points[i], points[(i + 1) % n]);
            flats.push(Flat {
                outline: u32::try_from(o).unwrap_or(u32::MAX),
                index: u32::try_from(i).unwrap_or(u32::MAX),
                seg,
                at,
                a,
                b,
            });
            at += distance(a, b);
        }
        lengths.push((u32::try_from(n).unwrap_or(u32::MAX), at));
    }
    // (cell, segment) for every cell a segment's box (grown by the gap) touches.
    let cell = |v: f64| -> i64 {
        // invariant: pictures are at most 2 x 8192 px, so cells fit easily.
        #[allow(clippy::cast_possible_truncation)]
        let c = (v / CELL_PX).floor() as i64;
        c
    };
    let mut keys: Vec<(i64, i64, u32)> = Vec::new();
    for (k, f) in flats.iter().enumerate() {
        let k = u32::try_from(k).unwrap_or(u32::MAX);
        let (x0, x1) = (cell(f.a.x.min(f.b.x) - GAP), cell(f.a.x.max(f.b.x) + GAP));
        let (y0, y1) = (cell(f.a.y.min(f.b.y) - GAP), cell(f.a.y.max(f.b.y) + GAP));
        for y in y0..=y1 {
            for x in x0..=x1 {
                keys.push((y, x, k));
            }
        }
    }
    keys.sort_unstable();
    let mut bad: Vec<Vec<usize>> = vec![Vec::new(); outlines.len()];
    let mut i = 0;
    while i < keys.len() {
        let mut j = i;
        while j < keys.len() && (keys[j].0, keys[j].1) == (keys[i].0, keys[i].1) {
            j += 1;
        }
        for a in i..j {
            for b in a + 1..j {
                let (s, t) = (&flats[keys[a].2 as usize], &flats[keys[b].2 as usize]);
                if conflict(s, t, &lengths, open) {
                    bad[s.outline as usize].push(s.seg as usize);
                    bad[t.outline as usize].push(t.seg as usize);
                }
            }
        }
        i = j;
    }
    for list in &mut bad {
        list.sort_unstable();
        list.dedup();
    }
    bad
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, side: f64) -> Vec<Seg> {
        let p = [
            Point::new(x, y),
            Point::new(x + side, y),
            Point::new(x + side, y + side),
            Point::new(x, y + side),
        ];
        (0..4).map(|i| Seg::Line(p[i], p[(i + 1) % 4])).collect()
    }

    #[test]
    fn apart_touching_at_a_corner_close_and_crossing() {
        let none: Vec<Vec<usize>> = vec![vec![], vec![]];
        let apart = [square(0.0, 0.0, 10.0), square(20.0, 0.0, 10.0)];
        assert_eq!(conflicts(&apart), none);
        let touching = [square(0.0, 0.0, 10.0), square(10.0, 10.0, 10.0)];
        assert_eq!(conflicts(&touching), none);
        // The right side of the first and the left side of the second, 0.05 apart (with the ends of
        // their neighbours).
        let close = [square(0.0, 0.0, 10.0), square(10.05, 0.0, 10.0)];
        assert_eq!(conflicts(&close), [vec![0, 1, 2], vec![0, 2, 3]]);
        let crossing = [
            square(0.0, 0.0, 10.0),
            square(5.0, 5.0, 10.0),
            square(40.0, 0.0, 1.0),
        ];
        assert_eq!(conflicts(&crossing), [vec![1, 2], vec![0, 3], vec![]]);
    }

    #[test]
    fn a_figure_eight_crosses_itself() {
        let p = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 20.0),
            Point::new(20.0, 0.0),
            Point::new(0.0, 20.0),
        ];
        let eight: Vec<Seg> = (0..4).map(|i| Seg::Line(p[i], p[(i + 1) % 4])).collect();
        assert_eq!(conflicts(&[eight]), [vec![0, 2]]);
    }
}
