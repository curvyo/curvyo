//! Groups word boxes into text lines with one quad each.
//!
//! Two boxes belong to one line when they run in the same direction, are of similar height, sit
//! on the same level and the gap between them is at most one line height (Proposals, ADR 0027
//! §3). Fixed order, no hash maps.

use crate::detect::RotatedBox;

/// Directions that differ by less than this many degrees are one direction.
const SAME_DIRECTION_DEGREES: f32 = 5.0;
/// Heights within this ratio are one height.
const SAME_HEIGHT: f32 = 1.6;
/// The centres may be this share of the lower height apart across the line.
const SAME_LEVEL: f32 = 0.5;
/// The gap between two boxes of a line may be this share of the lower height.
const MAX_GAP: f32 = 1.0;

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn joins(a: &RotatedBox, b: &RotatedBox) -> bool {
    if dot(a.u, b.v()).abs() > SAME_DIRECTION_DEGREES.to_radians().sin() {
        return false;
    }
    let (low, high) = (a.half_v.min(b.half_v), a.half_v.max(b.half_v));
    if high > SAME_HEIGHT * low {
        return false;
    }
    let offset = [b.centre[0] - a.centre[0], b.centre[1] - a.centre[1]];
    let across = dot(offset, a.v()).abs();
    let gap = dot(offset, a.u).abs() - a.half_u - b.half_u;
    across <= SAME_LEVEL * 2.0 * low && gap <= MAX_GAP * 2.0 * low
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The box around `members` along the direction of the longest of them.
#[allow(clippy::many_single_char_names)] // the frame u, v and its coordinates a, b
fn enclose(members: &[RotatedBox]) -> Option<RotatedBox> {
    let longest = members
        .iter()
        .max_by(|a, b| a.half_u.total_cmp(&b.half_u))?;
    let (u, v) = (longest.u, longest.v());
    let origin = longest.centre;
    let (mut a0, mut a1, mut b0, mut b1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for corner in members.iter().flat_map(RotatedBox::quad) {
        let d = [corner[0] - origin[0], corner[1] - origin[1]];
        let (a, b) = (dot(d, u), dot(d, v));
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    let (cu, cv) = (f32::midpoint(a0, a1), f32::midpoint(b0, b1));
    Some(RotatedBox {
        centre: [
            origin[0] + u[0] * cu + v[0] * cv,
            origin[1] + u[1] * cu + v[1] * cv,
        ],
        u,
        half_u: 0.5 * (a1 - a0),
        half_v: 0.5 * (b1 - b0),
    })
}

/// The lines of `boxes`, top to bottom, then left to right.
pub(crate) fn group(boxes: &[RotatedBox]) -> Vec<RotatedBox> {
    let n = boxes.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in i + 1..n {
            if joins(&boxes[i], &boxes[j]) || joins(&boxes[j], &boxes[i]) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                if ri != rj {
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
        }
    }
    let mut lines = Vec::new();
    for r in 0..n {
        if root(&mut parent, r) != r {
            continue;
        }
        let members: Vec<RotatedBox> = (0..n)
            .filter(|&i| root(&mut parent, i) == r)
            .map(|i| boxes[i])
            .collect();
        lines.extend(enclose(&members));
    }
    let top = |b: &RotatedBox| b.quad().iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    lines.sort_by(|a, b| {
        top(a)
            .total_cmp(&top(b))
            .then(a.centre[0].total_cmp(&b.centre[0]))
    });
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(x: f32, y: f32, half_u: f32) -> RotatedBox {
        RotatedBox {
            centre: [x, y],
            u: [1.0, 0.0],
            half_u,
            half_v: 10.0,
        }
    }

    #[test]
    fn words_on_one_level_join_and_lines_are_ordered() {
        let lines = group(&[
            word(300.0, 100.0, 40.0),
            word(100.0, 20.0, 50.0),
            word(200.0, 102.0, 50.0),
            word(600.0, 100.0, 40.0),
        ]);
        assert_eq!(lines.len(), 3);
        assert!((lines[0].centre[1] - 20.0).abs() < 1e-3);
        // 150 to 340 joined; the word at 600 is too far away.
        assert!((lines[1].half_u - 95.0).abs() < 1e-3);
        assert!((lines[2].centre[0] - 600.0).abs() < 1e-3);
    }
}
