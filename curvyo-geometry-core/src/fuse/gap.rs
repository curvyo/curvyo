//! The narrowest gap between the outer contours of two pieces, with the tie rule of
//! `specs/0156-fused-lettering` (Words, "Gap"; `adrs.md` decision 4).
//!
//! Points within the tie tolerance of the smallest distance tie. On each contour the tied points
//! form stretches; the first contour's stretch whose middle (by arc length) is smallest in
//! (x, y) is taken, with the second contour's stretch that faces it. The bridge's centre is half
//! way between the two middles, and its ends are the points of each contour nearest that centre.
//! Taking both middles keeps the centre where the tied stretch is symmetric, whatever the
//! rounding at its ends (criterion 9's bars).

use crate::boolean_grid::Path64;
use crate::fuse::measure::{Near, P, arc_at, arc_lengths, nearest_on, normal_at, point_at, vertex};

/// The narrowest gap between two contours `a` and `b` (positive area, apart).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Gap {
    /// The smallest distance between the contours, in grid units.
    pub(crate) distance: f64,
    /// The bridge's end on `a`.
    pub(crate) on_a: P,
    /// The outward normal of `a` there.
    pub(crate) normal_a: P,
    /// The bridge's end on `b`.
    pub(crate) on_b: P,
    /// The outward normal of `b` there.
    pub(crate) normal_b: P,
}

/// A closed interval of arc length on a contour (`start <= end`, `end` may pass the perimeter).
#[derive(Debug, Clone, Copy)]
struct Stretch {
    start: f64,
    end: f64,
}

/// The smallest distance between the two contours, in grid units.
fn smallest_distance(a: &Path64, b: &Path64) -> f64 {
    let mut best = f64::INFINITY;
    for (from, to) in [(a, b), (b, a)] {
        for index in 0..from.len() {
            let near = nearest_on(to, vertex(from, index));
            best = best.min(near.distance_squared);
        }
    }
    best.sqrt()
}

/// The parameters `t` in `0..=1` of `from + t (to - from)` within `r` of the segment `c d`: an
/// interval, as the capsule around the segment is convex.
#[allow(clippy::many_single_char_names)] // the usual names of plane geometry
fn capsule_interval(from: P, to: P, c: P, d: P, r: f64) -> Option<(f64, f64)> {
    let dir = to.sub(from);
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut take = |range: Option<(f64, f64)>| {
        if let Some((l, h)) = range {
            low = low.min(l);
            high = high.max(h);
        }
    };
    // The two end discs.
    for centre in [c, d] {
        let m = from.sub(centre);
        let qa = dir.dot(dir);
        let qb = 2.0 * m.dot(dir);
        let qc = m.dot(m) - r * r;
        if qa == 0.0 {
            take((qc <= 0.0).then_some((0.0, 1.0)));
            continue;
        }
        let disc = qb * qb - 4.0 * qa * qc;
        if disc >= 0.0 {
            let root = disc.sqrt();
            take(Some(((-qb - root) / (2.0 * qa), (-qb + root) / (2.0 * qa))));
        }
    }
    // The slab: projection on the segment within its length, distance to its line within r.
    let axis = d.sub(c);
    let length = axis.length();
    if length > 0.0 {
        let u = axis.scale(1.0 / length);
        let n = P::new(-u.y, u.x);
        let mut range = (f64::NEG_INFINITY, f64::INFINITY);
        for (value0, slope, lo, hi) in [
            (from.sub(c).dot(u), dir.dot(u), 0.0, length),
            (from.sub(c).dot(n), dir.dot(n), -r, r),
        ] {
            if slope == 0.0 {
                if value0 < lo || value0 > hi {
                    range = (1.0, 0.0);
                }
            } else {
                let (t0, t1) = ((lo - value0) / slope, (hi - value0) / slope);
                range = (range.0.max(t0.min(t1)), range.1.min(t0.max(t1)));
            }
        }
        take((range.0 <= range.1).then_some(range));
    }
    let (low, high) = (low.max(0.0), high.min(1.0));
    (low <= high).then_some((low, high))
}

/// The stretches of `a` within `limit` of `b`, by arc length, merged and in order.
fn stretches(a: &Path64, b: &Path64, limit: f64, lengths: &[f64]) -> Vec<Stretch> {
    let mut pieces: Vec<Stretch> = Vec::new();
    for edge in 0..a.len() {
        let (from, to) = (vertex(a, edge), vertex(a, edge + 1));
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        let mut intervals: Vec<(f64, f64)> = (0..b.len())
            .filter_map(|k| capsule_interval(from, to, vertex(b, k), vertex(b, k + 1), limit))
            .collect();
        intervals.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (l, h) in intervals {
            if l > high && high.is_finite() {
                pieces.push(Stretch {
                    start: arc_at(lengths, edge, low),
                    end: arc_at(lengths, edge, high),
                });
                low = l;
            } else if !low.is_finite() {
                low = l;
            }
            high = high.max(h);
        }
        if high.is_finite() {
            pieces.push(Stretch {
                start: arc_at(lengths, edge, low),
                end: arc_at(lengths, edge, high),
            });
        }
    }
    merge(pieces, lengths[lengths.len() - 1])
}

/// Joins stretches that meet (within a millionth of a unit), the wrap-around included.
fn merge(pieces: Vec<Stretch>, perimeter: f64) -> Vec<Stretch> {
    const MEET: f64 = 1e-6;
    let mut merged: Vec<Stretch> = Vec::new();
    for piece in pieces {
        match merged.last_mut() {
            Some(last) if piece.start <= last.end + MEET => last.end = last.end.max(piece.end),
            _ => merged.push(piece),
        }
    }
    if merged.len() > 1 {
        let first = merged[0];
        if let Some(last) = merged.last_mut()
            && last.end + MEET >= perimeter
            && first.start <= MEET
        {
            last.end = perimeter + first.end;
            merged.remove(0);
        }
    }
    merged
}

/// The stretch's middle.
fn middle(stretch: Stretch) -> f64 {
    f64::midpoint(stretch.start, stretch.end)
}

/// The narrowest gap between the outer contours `a` and `b` (`a` the lower-indexed piece), with
/// ties within `tie` grid units.
pub(crate) fn narrowest_gap(a: &Path64, b: &Path64, tie: f64) -> Gap {
    let distance = smallest_distance(a, b);
    let limit = distance + tie;
    let lengths_a = arc_lengths(a);
    let lengths_b = arc_lengths(b);
    let middle_a = stretches(a, b, limit, &lengths_a)
        .into_iter()
        .map(|s| point_at(a, &lengths_a, middle(s)))
        .min_by(|x, y| x.cmp_xy(*y));
    let Some(middle_a) = middle_a else {
        return gap_at(a, b, nearest_on(b, vertex(a, 0)).point, distance);
    };
    let partner = nearest_on(b, middle_a).point;
    let middle_b = stretches(b, a, limit, &lengths_b)
        .into_iter()
        .map(|s| point_at(b, &lengths_b, middle(s)))
        .min_by(|x, y| {
            let dx = x.sub(partner).length();
            let dy = y.sub(partner).length();
            dx.total_cmp(&dy).then_with(|| x.cmp_xy(*y))
        })
        .unwrap_or(partner);
    gap_at(a, b, middle_a.add(middle_b).scale(0.5), distance)
}

/// The gap with its ends the points of `a` and `b` nearest `centre`.
fn gap_at(a: &Path64, b: &Path64, centre: P, distance: f64) -> Gap {
    let near_a: Near = nearest_on(a, centre);
    let near_b: Near = nearest_on(b, centre);
    Gap {
        distance,
        on_a: near_a.point,
        normal_a: normal_at(a, &near_a),
        on_b: near_b.point,
        normal_b: normal_at(b, &near_b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::Point64;

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Path64 {
        vec![
            Point64::new(x0, y0),
            Point64::new(x1, y0),
            Point64::new(x1, y1),
            Point64::new(x0, y1),
        ]
    }

    /// Criterion 9: two bars 2 x 10 mm, the second at (5, 3); the gap is 3 mm and its bridge is
    /// centred at y = 6.5 mm (the middle of the tied stretch y 3 to 10).
    #[test]
    fn the_bars_of_criterion_9_tie_along_a_stretch_and_bridge_its_middle() {
        let a = rect(0, 0, 2_000, 10_000);
        let b = rect(5_000, 3_000, 7_000, 13_000);
        let gap = narrowest_gap(&a, &b, 10.0);
        assert!((gap.distance - 3_000.0).abs() < 1e-9);
        assert!((gap.on_a.y - 6_500.0).abs() <= 10.0, "{gap:?}");
        assert!((gap.on_b.y - 6_500.0).abs() <= 10.0);
        assert_eq!(gap.on_a.x, 2_000.0);
        assert_eq!(gap.on_b.x, 5_000.0);
        assert_eq!(gap.normal_a, P::new(1.0, 0.0));
        assert_eq!(gap.normal_b, P::new(-1.0, 0.0));
    }

    /// One point closest: the corner to corner gap of two squares set diagonally.
    #[test]
    fn a_single_closest_pair_is_its_own_middle() {
        let a = rect(0, 0, 1_000, 1_000);
        let b = rect(2_000, 2_000, 3_000, 3_000);
        let gap = narrowest_gap(&a, &b, 10.0);
        assert!((gap.distance - 1_414.213_562_373_095).abs() < 1e-6);
        assert!(gap.on_a.sub(P::new(1_000.0, 1_000.0)).length() < 10.0);
        assert!(gap.on_b.sub(P::new(2_000.0, 2_000.0)).length() < 10.0);
    }

    #[test]
    fn a_capsule_interval_is_where_the_moving_point_is_near() {
        let (l, h) = capsule_interval(
            P::new(-10.0, 1.0),
            P::new(10.0, 1.0),
            P::new(0.0, 0.0),
            P::new(2.0, 0.0),
            2.0,
        )
        .unwrap();
        // x from -sqrt(3) to 2 + sqrt(3).
        assert!((l - (10.0 - 3.0_f64.sqrt()) / 20.0).abs() < 1e-12);
        assert!((h - (12.0 + 3.0_f64.sqrt()) / 20.0).abs() < 1e-12);
    }
}
