// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from Ink/Stitch `lib/stitches/running_stitch.py` and the repeats of
// `lib/elements/stroke.py` (https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors
// (see git history); changed for Curvyo (see `NOTICE.md`).

//! The even running stitch (`specs/0160-embroidery` criterion 7): needle points of near-equal
//! length along a polyline, kept within a tolerance of the path by Zhao-Saalfeld sleeve fitting.

use std::f64::consts::{PI, TAU};

use kurbo::{Point, Vec2};

use crate::Mm;

/// Ink/Stitch's default stitch tolerance (`running_stitch_tolerance_mm`, 0.2 mm).
pub const RUNNING_TOLERANCE: Mm = Mm::literal(0.2);

/// A modular angle interval: the whole circle or less than half of it.
#[derive(Debug, Clone, Copy)]
struct AngleInterval {
    a: f64,
    b: f64,
    all: bool,
}

impl AngleInterval {
    const ALL: Self = Self {
        a: 0.0,
        b: TAU,
        all: true,
    };

    /// The directions from the origin into a ball of radius `epsilon` around `p`.
    fn from_ball(p: Vec2, epsilon: f64) -> Self {
        let d = p.hypot();
        if d <= epsilon {
            return Self::ALL;
        }
        let centre = p.atan2();
        let delta = (epsilon / d).asin();
        Self {
            a: centre - delta,
            b: centre + delta,
            all: false,
        }
    }

    /// The directions from the origin onto the segment `a`, `b`.
    fn from_segment(a: Vec2, b: Vec2) -> Option<Self> {
        let (angle_a, angle_b) = (a.atan2(), b.atan2());
        let diff = (angle_b - angle_a).rem_euclid(TAU);
        // Exact as upstream: only a segment through the origin is degenerate.
        #[allow(clippy::float_cmp)]
        let degenerate = diff == 0.0 || diff == PI;
        if degenerate {
            None
        } else if diff < PI {
            // Slightly larger than normal against rounding errors, as upstream.
            Some(Self {
                a: angle_a - 1e-6,
                b: angle_b + 1e-6,
                all: false,
            })
        } else {
            Some(Self {
                a: angle_b - 1e-6,
                b: angle_a + 1e-6,
                all: false,
            })
        }
    }

    fn contains_angle(self, angle: f64) -> bool {
        self.all || (angle - self.a).rem_euclid(TAU) <= (self.b - self.a).rem_euclid(TAU)
    }

    fn contains_point(self, p: Vec2) -> bool {
        self.contains_angle(p.atan2())
    }

    fn intersect(self, other: Option<Self>) -> Option<Self> {
        let other = other?;
        if self.all {
            return Some(other);
        }
        if other.all {
            return Some(self);
        }
        let (a, b) = if self.contains_angle(other.a) {
            if other.contains_angle(self.b) {
                (other.a, self.b)
            } else {
                (other.a, other.b)
            }
        } else if other.contains_angle(self.a) {
            if self.contains_angle(other.b) {
                (self.a, other.b)
            } else {
                (self.a, self.b)
            }
        } else {
            return None;
        };
        Some(Self { a, b, all: false })
    }

    /// Where the segment `a`, `b` leaves this interval seen from `origin`.
    fn cut_segment(self, origin: Point, a: Point, b: Point) -> Option<Point> {
        if self.all {
            return None;
        }
        let Some(arc) = Self::from_segment(a - origin, b - origin) else {
            return Some(a);
        };
        if arc.contains_angle(self.a) {
            Some(cut_with_angle(origin, self.a, a, b))
        } else if arc.contains_angle(self.b) {
            Some(cut_with_angle(origin, self.b, a, b))
        } else {
            None
        }
    }
}

/// Where the ray from `origin` at `angle` crosses the segment `a`, `b` (clamped onto it).
#[allow(clippy::many_single_char_names)] // upstream's names
fn cut_with_angle(origin: Point, angle: f64, a: Point, b: Point) -> Point {
    let p = a - origin;
    let d = b - a;
    let c = Vec2::new(angle.cos(), angle.sin());
    let t = (p.y * c.x - p.x * c.y) / (d.x * c.y - d.y * c.x);
    a + d * t.clamp(0.0, 1.0)
}

/// Where the segment `a` (inside) to `b` (outside) crosses the circle of radius `r` around
/// `origin` (clamped onto the segment).
#[allow(clippy::many_single_char_names)] // upstream's names
fn cut_with_circle(origin: Point, r: f64, a: Point, b: Point) -> Point {
    let p = a - origin;
    let d = b - a;
    let (p2, d2, pd) = (p.dot(p), d.dot(d), p.dot(d));
    if d2 == 0.0 {
        return a;
    }
    let t = ((pd * pd + r * r * d2 - p2 * d2).max(0.0).sqrt() - pd) / d2;
    a + d * t.clamp(0.0, 1.0)
}

/// One stitch from `start` along `points[idx..]` (Zhao-Saalfeld sleeve fitting with an early end
/// at the stitch length): the needle point and the index to go on from, `None` at the end.
fn take_stitch(
    start: Point,
    points: &[Point],
    idx: usize,
    stitch_length: f64,
    tolerance: f64,
) -> (Option<Point>, Option<usize>) {
    if idx >= points.len() {
        return (None, None);
    }
    let mut sleeve = Some(AngleInterval::ALL);
    let mut last = start;
    for (i, &p) in points.iter().enumerate().skip(idx) {
        if sleeve.is_some_and(|s| s.contains_point(p - start)) {
            if start.distance(p) < stitch_length {
                sleeve = sleeve.and_then(|s| {
                    s.intersect(Some(AngleInterval::from_ball(p - start, tolerance)))
                });
                last = p;
                continue;
            }
            return (
                Some(cut_with_circle(start, stitch_length, last, p)),
                Some(i),
            );
        }
        // Upstream assumes a cut exists; the last point inside the sleeve stands in otherwise.
        let fallback = if last == start { p } else { last };
        let mut cut = sleeve
            .and_then(|s| s.cut_segment(start, last, p))
            .unwrap_or(fallback);
        if start.distance(cut) > stitch_length {
            cut = cut_with_circle(start, stitch_length, last, p);
        }
        return (Some(cut), Some(i));
    }
    (points.last().copied(), None)
}

/// The stitches of one curve with lengths adapted so they come out even; the end point included,
/// the start point not.
fn stitch_curve_evenly(points: &[Point], stitch_length: f64, tolerance: f64) -> Vec<Point> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut dist_left = vec![0.0; points.len()];
    for j in (0..points.len() - 1).rev() {
        dist_left[j] = dist_left[j + 1] + points[j].distance(points[j + 1]);
    }
    let mut i = Some(1);
    let mut last = points[0];
    let mut stitches = Vec::new();
    while let Some(index) = i.filter(|&index| index < points.len()) {
        let d = last.distance(points[index]) + dist_left[index];
        if d == 0.0 {
            return stitches;
        }
        // The correction for rounding error is upstream's.
        let length = d / (d / stitch_length).ceil() + 0.000_001;
        let (stitch, next) = take_stitch(last, points, index, length, tolerance);
        i = next;
        if let Some(stitch) = stitch {
            stitches.push(stitch);
            last = stitch;
        }
    }
    stitches
}

/// The path split at corners sharper than 45 degrees, so they are stitched exactly; a piece
/// shorter than `min_len` is not split again.
fn path_to_curves(points: &[Point], min_len: f64) -> Vec<Vec<Point>> {
    if points.len() < 3 {
        return vec![points.to_vec()];
    }
    let mut curves = Vec::new();
    let mut last = 0;
    let mut last_seg = points[1] - points[0];
    let mut seg_len = last_seg.hypot();
    for i in 1..points.len() - 1 {
        let a = last_seg;
        let b = points[i + 1] - points[i];
        let aabb = a.dot(a) * b.dot(b);
        let abab = a.dot(b) * a.dot(b).abs();
        if aabb > 0.0 && abab <= 0.5 * aabb {
            if seg_len >= min_len {
                curves.push(points[last..=i].to_vec());
                last = i;
            }
            seg_len = 0.0;
        }
        if b.dot(b) > 0.0 {
            last_seg = b;
        }
        seg_len += b.hypot();
    }
    curves.push(points[last..].to_vec());
    curves
}

/// The even running stitch of the polyline `points` (upstream `even_running_stitch`): the first
/// point, then needle points no further apart than `stitch_length` and within `tolerance` of the
/// path, the last point included. Empty for no points.
#[must_use]
pub fn running_stitch(points: &[Point], stitch_length: Mm, tolerance: Mm) -> Vec<Point> {
    let Some(&first) = points.first() else {
        return Vec::new();
    };
    let length = stitch_length.get().max(0.1);
    let tolerance = tolerance.get().max(0.01);
    let mut stitches = vec![first];
    for curve in path_to_curves(points, 2.0 * tolerance) {
        stitches.extend(stitch_curve_evenly(&curve, length, tolerance));
    }
    stitches
}

/// `stitches` run `repeats` times, every other pass backwards (upstream `Stroke.running_stitch`):
/// 3 is a triple stitch forward, back, forward.
#[must_use]
pub fn repeat_passes(stitches: &[Point], repeats: u8) -> Vec<Point> {
    let mut out = Vec::with_capacity(stitches.len() * usize::from(repeats.max(1)));
    for pass in 0..repeats.max(1) {
        if pass % 2 == 1 {
            out.extend(stitches.iter().rev());
        } else {
            out.extend(stitches.iter());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(v: f64) -> Mm {
        Mm::new(v).unwrap()
    }

    #[test]
    fn a_100_mm_line_at_2_5_mm_gives_41_points_2_5_mm_apart() {
        let line = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let stitches = running_stitch(&line, mm(2.5), RUNNING_TOLERANCE);
        assert_eq!(stitches.len(), 41, "criterion 7");
        assert_eq!(stitches[0], line[0]);
        assert!(stitches[40].distance(line[1]) < 1e-9);
        for pair in stitches.windows(2) {
            assert!((pair[0].distance(pair[1]) - 2.5).abs() < 0.1);
        }
    }

    #[test]
    fn on_a_curve_the_points_lie_on_it_and_no_stitch_is_longer_than_the_length() {
        let curve: Vec<Point> = (0..=180)
            .map(|i| {
                let a = f64::from(i).to_radians() / 2.0;
                Point::new(40.0 * a.cos(), 40.0 * a.sin())
            })
            .collect();
        let stitches = running_stitch(&curve, mm(2.5), RUNNING_TOLERANCE);
        assert!(stitches.len() > 20);
        for s in &stitches {
            let r = s.to_vec2().hypot();
            assert!((r - 40.0).abs() < 0.1, "criterion 7: {r}");
        }
        for pair in stitches.windows(2) {
            assert!(pair[0].distance(pair[1]) <= 2.5 + 1e-5);
        }
    }

    #[test]
    fn a_corner_is_stitched_exactly() {
        let path = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        let stitches = running_stitch(&path, mm(3.0), RUNNING_TOLERANCE);
        assert!(stitches.iter().any(|s| s.distance(path[1]) < 1e-9));
    }

    #[test]
    fn three_repeats_run_forward_back_forward() {
        let s = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
        ];
        let out = repeat_passes(&s, 3);
        let xs: Vec<f64> = out.iter().map(|p| p.x).collect();
        assert_eq!(xs, [0.0, 1.0, 2.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0]);
    }
}
