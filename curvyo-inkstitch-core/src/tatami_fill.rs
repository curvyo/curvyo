// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from Ink/Stitch `lib/stitches/fill.py` (`legacy_fill`;
// https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors (see git history); changed
// for Curvyo (see `NOTICE.md`).

//! The tatami fill (`specs/0160-embroidery` criterion 8): an area cut by a grating of parallel
//! rows, the rows pulled into contiguous sections stitched back and forth with staggered ends.

use std::f64::consts::FRAC_PI_2;

use kurbo::{Point, Vec2};

use crate::{Angle, Mm};

/// Rows are staggered this many times before the pattern repeats (Ink/Stitch's default 4).
const STAGGERS: f64 = 4.0;
/// The last stitch of a row is left out when it would be shorter than this (upstream 0.1 mm).
const MIN_LAST_STITCH_MM: f64 = 0.1;

/// One row run: where it begins and ends.
type Segment = (Point, Point);

/// `v` turned by `angle` (upstream `Point.rotate`).
fn rotate(v: Vec2, angle: f64) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.y * cos + v.x * sin)
}

/// The direction along a row, to the right.
fn east(angle: f64) -> Vec2 {
    rotate(Vec2::new(1.0, 0.0), -angle)
}

/// The direction across the rows.
fn north(angle: f64) -> Vec2 {
    rotate(east(angle), FRAC_PI_2)
}

/// Whether `p` lies inside the even-odd area of `rings`.
fn inside(rings: &[Vec<Point>], p: Point) -> bool {
    let mut odd = false;
    for ring in rings {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                if p.x < x {
                    odd = !odd;
                }
            }
        }
    }
    odd
}

/// The runs of the line `p0`, `p1` inside the even-odd area of `rings`, from `p0` on (the
/// port's own scanline clip in place of upstream's shapely intersection).
fn clip_line(rings: &[Vec<Point>], p0: Point, p1: Point) -> Vec<Segment> {
    let along = p1 - p0;
    let length = along.hypot();
    if length == 0.0 {
        return Vec::new();
    }
    let unit = along / length;
    let normal = Vec2::new(-unit.y, unit.x);
    let mut hits = Vec::new();
    for ring in rings {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            let (sa, sb) = ((a - p0).dot(normal), (b - p0).dot(normal));
            if (sa > 0.0) != (sb > 0.0) {
                let crossing = a + (b - a) * (sa / (sa - sb));
                hits.push((crossing - p0).dot(unit).clamp(0.0, length));
            }
        }
    }
    hits.sort_by(f64::total_cmp);
    hits.as_chunks::<2>()
        .0
        .iter()
        .filter(|pair| pair[1] > pair[0])
        .map(|pair| (p0 + unit * pair[0], p0 + unit * pair[1]))
        .collect()
}

/// The bounds `(min, max)` of every point of `rings`.
fn bounds(rings: &[Vec<Point>]) -> Option<(Point, Point)> {
    let mut points = rings.iter().flatten();
    let first = *points.next()?;
    Some(points.fold((first, first), |(lo, hi), p| {
        (
            Point::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

/// The rows of the grating at `angle`, `spacing` apart, each with its runs inside the area
/// (upstream `intersect_region_with_grating`, without end row spacing and flip).
fn intersect_region_with_grating(
    rings: &[Vec<Point>],
    angle: f64,
    spacing: f64,
) -> Vec<Vec<Segment>> {
    let Some((lo, hi)) = bounds(rings) else {
        return Vec::new();
    };
    let half_length = lo.distance(hi) / 2.0;
    let direction = rotate(Vec2::new(-1.0, 0.0), -angle);
    let normal = rotate(direction, -FRAC_PI_2);
    let centre = lo.midpoint(hi);
    // The bounds of the area turned by `angle` about the centre: how far the rows reach.
    let (sin, cos) = angle.sin_cos();
    let turned_y = |p: &Point| centre.y + (p.x - centre.x) * sin + (p.y - centre.y) * cos;
    let (mut start, end) = rings
        .iter()
        .flatten()
        .fold((f64::MAX, f64::MIN), |(a, b), p| {
            (a.min(turned_y(p)), b.max(turned_y(p)))
        });
    start -= centre.y;
    let end = end - centre.y;
    if (end - start).abs() == 0.0 {
        return Vec::new();
    }
    // Rows on whole multiples of the spacing from the origin, so abutting areas line up.
    start -= (start + normal.dot(centre.to_vec2())).rem_euclid(spacing);
    let mut rows = Vec::new();
    let mut y = start;
    while y < end {
        let p0 = centre + normal * y + direction * half_length;
        let p1 = centre + normal * y - direction * half_length;
        let mut runs = clip_line(rings, p0, p1);
        if !runs.is_empty() {
            runs.sort_by(|a, b| a.0.distance(lo).total_cmp(&b.0.distance(lo)));
            rows.push(runs);
        }
        y += spacing;
    }
    rows
}

/// The distance from `p` to the segment `s`.
fn point_segment_distance(p: Point, s: Segment) -> f64 {
    let d = s.1 - s.0;
    let len2 = d.dot(d);
    if len2 == 0.0 {
        return p.distance(s.0);
    }
    let t = ((p - s.0).dot(d) / len2).clamp(0.0, 1.0);
    p.distance(s.0 + d * t)
}

/// Whether two runs of neighbouring rows belong to one section: close enough, overlapping along
/// the rows, and the area between them inside the shape (the port's stand-in for upstream's
/// 90 % quadrilateral area test).
#[allow(clippy::many_single_char_names)] // upstream's names
fn is_same_run(a: Segment, b: Segment, rings: &[Vec<Point>], spacing: f64, angle: f64) -> bool {
    let distance = [
        point_segment_distance(a.0, b),
        point_segment_distance(a.1, b),
        point_segment_distance(b.0, a),
        point_segment_distance(b.1, a),
    ]
    .into_iter()
    .fold(f64::MAX, f64::min);
    if distance > spacing * 1.1 {
        return false;
    }
    let e = east(angle);
    let span = |s: Segment| {
        let (x, y) = (s.0.to_vec2().dot(e), s.1.to_vec2().dot(e));
        (x.min(y), x.max(y))
    };
    let (sa, sb) = (span(a), span(b));
    let (from, to) = (sa.0.max(sb.0), sa.1.min(sb.1));
    if to <= from {
        return false;
    }
    let mid_a = a.0.midpoint(a.1);
    let mid_b = b.0.midpoint(b.1);
    inside(rings, mid_a.midpoint(mid_b))
}

/// The rows broken into contiguous sections by repeatedly pulling the first run off each row
/// (upstream `pull_runs`).
fn pull_runs(
    mut rows: Vec<Vec<Segment>>,
    rings: &[Vec<Point>],
    spacing: f64,
    angle: f64,
) -> Vec<Vec<Segment>> {
    for row in &mut rows {
        row.reverse();
    }
    let mut sections = Vec::new();
    while !rows.is_empty() {
        let mut section = Vec::new();
        let mut prev: Option<Segment> = None;
        for row in &mut rows {
            let Some(&first) = row.last() else {
                continue;
            };
            if prev.is_some_and(|p| !is_same_run(p, first, rings, spacing, angle)) {
                break;
            }
            section.push(first);
            prev = Some(first);
            row.pop();
        }
        sections.push(section);
        rows.retain(|row| !row.is_empty());
    }
    sections
}

/// The first stitch of a row moved back onto the stagger grid (upstream `adjust_stagger`).
fn adjust_stagger(stitch: Point, angle: f64, spacing: f64, max_stitch: f64) -> Point {
    let row = (stitch.to_vec2().dot(north(angle)) / spacing).round();
    let phase = (row / STAGGERS).rem_euclid(1.0);
    let stagger_offset = phase * max_stitch;
    let offset = (stitch.to_vec2().dot(east(angle)) - stagger_offset).rem_euclid(max_stitch);
    stitch - east(angle) * offset
}

/// The stitches of one row from `beg` to `end` onto `stitches` (upstream `stitch_row`, without
/// the random stitch length; the row start is pushed once, not twice).
fn stitch_row(
    stitches: &mut Vec<Point>,
    beg: Point,
    end: Point,
    angle: f64,
    spacing: f64,
    max_stitch: f64,
) {
    stitches.push(beg);
    if end == beg {
        return;
    }
    let along = end - beg;
    let row_length = along.hypot();
    let direction = along / row_length;
    let mut first = adjust_stagger(beg, angle, spacing, max_stitch);
    // The first stitch may lie just before this row: move it back in.
    if (first - beg).dot(direction) < 0.0 {
        first += direction * max_stitch;
    }
    let mut offset = (first - beg).hypot();
    while offset < row_length {
        stitches.push(beg + direction * offset);
        offset += max_stitch;
    }
    if stitches
        .last()
        .is_some_and(|last| end.distance(*last) > MIN_LAST_STITCH_MM)
    {
        stitches.push(end);
    }
}

/// The tatami fill of the even-odd area of `rings` (closed polygons in mm, Y down as on the
/// page): one needle point list per section, each row stitched against the last, so every stitch
/// is at most `max_stitch` long along a row. A section is entered by a jump (upstream
/// `legacy_fill`; no travel graph in slice 1).
#[must_use]
pub fn tatami_fill(
    rings: &[Vec<Point>],
    angle: Angle,
    row_spacing: Mm,
    max_stitch: Mm,
) -> Vec<Vec<Point>> {
    let angle = angle.radians();
    let spacing = row_spacing.get().max(0.1);
    let max_stitch = max_stitch.get().max(0.1);
    let rings: Vec<Vec<Point>> = rings.iter().filter(|r| r.len() >= 3).cloned().collect();
    let rows = intersect_region_with_grating(&rings, angle, spacing);
    pull_runs(rows, &rings, spacing, angle)
        .into_iter()
        .map(|section| {
            let mut stitches = Vec::new();
            for (index, &(beg, end)) in section.iter().enumerate() {
                let (beg, end) = if index % 2 == 1 {
                    (end, beg)
                } else {
                    (beg, end)
                };
                stitch_row(&mut stitches, beg, end, angle, spacing, max_stitch);
            }
            stitches
        })
        .filter(|stitches| !stitches.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Vec<Point> {
        vec![
            Point::new(x, y),
            Point::new(x + w, y),
            Point::new(x + w, y + h),
            Point::new(x, y + h),
        ]
    }

    fn fill(rings: &[Vec<Point>]) -> Vec<Vec<Point>> {
        tatami_fill(
            rings,
            Angle::from_degrees(0.0).unwrap(),
            Mm::new(0.25).unwrap(),
            Mm::new(3.0).unwrap(),
        )
    }

    #[test]
    fn a_rectangle_fills_with_horizontal_rows_inside_with_short_stitches() {
        let sections = fill(&[rect(10.0, 20.0, 50.0, 30.0)]);
        assert_eq!(sections.len(), 1);
        let points = &sections[0];
        for p in points {
            assert!(
                p.x >= 10.0 - 0.1 && p.x <= 60.0 + 0.1 && p.y >= 20.0 - 0.1 && p.y <= 50.0 + 0.1
            );
        }
        for pair in points.windows(2) {
            assert!(pair[0].distance(pair[1]) <= 3.0 + 1e-9, "criterion 8");
        }
        let mut ys: Vec<f64> = points.iter().map(|p| p.y).collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert!(ys.len() > 100);
        for pair in ys.windows(2) {
            assert!(
                (pair[1] - pair[0] - 0.25).abs() < 0.05,
                "rows 0.25 mm apart"
            );
        }
    }

    #[test]
    fn the_stitch_ends_of_neighbouring_rows_are_offset() {
        let sections = fill(&[rect(10.0, 20.0, 50.0, 30.0)]);
        let points = &sections[0];
        // The x of the first inner stitch of each row, modulo the stitch length.
        let mut phases = Vec::new();
        let mut row_y: Option<f64> = None;
        for p in points {
            if row_y.is_none_or(|y| (p.y - y).abs() > 1e-9) {
                row_y = Some(p.y);
                phases.push(None);
            } else if let Some(slot @ None) = phases.last_mut()
                && p.x > 10.0 + 1e-6
                && p.x < 60.0 - 1e-6
            {
                *slot = Some(p.x.rem_euclid(3.0));
            }
        }
        let phases: Vec<f64> = phases.into_iter().flatten().collect();
        for pair in phases[..8].windows(2) {
            assert!((pair[0] - pair[1]).abs() > 0.5, "staggered: {phases:?}");
        }
    }

    #[test]
    fn a_hole_stays_unstitched_and_splits_the_rows_into_sections() {
        let outer = rect(0.0, 0.0, 40.0, 40.0);
        let mut hole = rect(10.0, 10.0, 20.0, 20.0);
        hole.reverse();
        let sections = fill(&[outer, hole]);
        assert!(sections.len() >= 2, "{}", sections.len());
        for p in sections.iter().flatten() {
            let strictly_inside_hole = p.x > 10.01 && p.x < 29.99 && p.y > 10.01 && p.y < 29.99;
            assert!(!strictly_inside_hole, "{p:?}");
        }
        // No stitch inside a section crosses the hole.
        for section in &sections {
            for pair in section.windows(2) {
                let mid = pair[0].midpoint(pair[1]);
                assert!(!(mid.x > 10.01 && mid.x < 29.99 && mid.y > 10.01 && mid.y < 29.99));
            }
        }
    }

    #[test]
    fn an_angle_turns_the_rows() {
        let sections = tatami_fill(
            &[rect(0.0, 0.0, 30.0, 30.0)],
            Angle::from_degrees(90.0).unwrap(),
            Mm::new(0.5).unwrap(),
            Mm::new(3.0).unwrap(),
        );
        let points = &sections[0];
        // At 90 degrees the rows are vertical: the first stitches share one x.
        assert!((points[0].x - points[1].x).abs() < 1e-9);
    }
}
