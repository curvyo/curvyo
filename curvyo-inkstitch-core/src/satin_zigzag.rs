// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from Ink/Stitch `lib/elements/satin_column/stitches.py` (`_do_satin`) and the rail
// walk of `lib/elements/satin_column/rails.py` (https://github.com/inkstitch/inkstitch);
// Copyright (c) 2010 Authors (see git history); changed for Curvyo (see `NOTICE.md`).

//! The simple satin (`specs/0160-embroidery` criterion 9): a zigzag between two rails half the
//! width left and right of one line, one pair of needle points per station.

use kurbo::{Point, Vec2};

use crate::Mm;

/// The point `distance` along the polyline `line` and the line's direction there.
fn station(line: &[Point], lengths: &[f64], distance: f64) -> (Point, Vec2) {
    let index = lengths
        .partition_point(|&l| l < distance)
        .clamp(1, line.len() - 1);
    let (a, b) = (line[index - 1], line[index]);
    let span = lengths[index] - lengths[index - 1];
    let t = if span > 0.0 {
        (distance - lengths[index - 1]) / span
    } else {
        0.0
    };
    let unit = |v: Vec2| if v.hypot() > 0.0 { v / v.hypot() } else { v };
    let mut direction = unit(b - a);
    // At an inner vertex the two segments' directions are averaged, as upstream's zigzag does.
    if (distance - lengths[index]).abs() < 1e-9 && index + 1 < line.len() {
        direction = unit(direction + unit(line[index + 1] - b));
    }
    (a.lerp(b, t), direction)
}

/// The satin of the polyline `line` with the column `width`: stations every `spacing` or a little
/// less, so the last one lies on the line's end, each giving the left then the right rail point
/// (upstream `_do_satin`'s `a`, `b` pairs; no underlay, pull compensation or split stitches in
/// slice 1). Empty for a line without length.
#[must_use]
pub fn satin_zigzag(line: &[Point], width: Mm, spacing: Mm) -> Vec<Point> {
    let mut lengths = Vec::with_capacity(line.len());
    let mut total = 0.0;
    for (i, p) in line.iter().enumerate() {
        if i > 0 {
            total += line[i - 1].distance(*p);
        }
        lengths.push(total);
    }
    if line.len() < 2 || total <= 0.0 {
        return Vec::new();
    }
    let spacing = spacing.get().max(0.01);
    let half = width.get() / 2.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: a positive finite length over a spacing of at least 0.01 mm
    let steps = ((total / spacing - 1e-9).ceil() as u32).max(1);
    let mut out = Vec::with_capacity(2 * steps as usize + 2);
    for k in 0..=steps {
        let distance = total * f64::from(k) / f64::from(steps);
        let (centre, direction) = station(line, &lengths, distance);
        let left = Vec2::new(-direction.y, direction.x) * half;
        out.push(centre + left);
        out.push(centre - left);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_40_mm_line_with_a_3_mm_stroke_zigzags_1_5_mm_each_side_every_0_4_mm() {
        let line = [Point::new(0.0, 0.0), Point::new(40.0, 0.0)];
        let points = satin_zigzag(&line, Mm::new(3.0).unwrap(), Mm::new(0.4).unwrap());
        assert_eq!(points.len(), 2 * 101);
        for (i, p) in points.iter().enumerate() {
            let side = if i % 2 == 0 { 1.5 } else { -1.5 };
            assert!((p.y - side).abs() < 0.01, "criterion 9: {p:?}");
        }
        for pair in points.chunks(2).collect::<Vec<_>>().windows(2) {
            assert!((pair[1][0].x - pair[0][0].x - 0.4).abs() < 0.01);
        }
        assert!(
            points[0].x.abs() < 1e-9 && (points[201].x - 40.0).abs() < 1e-9,
            "end to end"
        );
    }
}
