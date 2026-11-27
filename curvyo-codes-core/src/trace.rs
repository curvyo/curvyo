//! Traces the outline of the ink cells of a grid: one loop per connected stretch of ink and one
//! per hole, from the cell edges alone (no boolean library, ADR 0091 §4).

use curvyo_extension_core::{ClosedPath, Cubic};

use crate::shapes::{Frame, reversed};

/// The cubic constant of a quarter circle.
const K: f64 = 0.552_284_749_8;

/// The four directions in clockwise order on the screen: east, south, west, north.
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// The vertices of the loops that enclose the ink cells `ink(x, y)` of a `width` x `width` grid.
/// Ink stays on the right-hand side of the walk (clockwise on the screen), holes run the other
/// way. Cells touching only at a corner stay two loops (the walk prefers the turn that hugs its
/// own cell: 4-connectivity). Vertices on a straight run are left out.
pub(crate) fn loops(width: usize, ink: impl Fn(usize, usize) -> bool) -> Vec<Vec<(i32, i32)>> {
    let stride = width + 1;
    let mut out = vec![0u8; stride * stride];
    let mut set = |x: usize, y: usize, direction: usize| out[y * stride + x] |= 1 << direction;
    for y in 0..width {
        for x in 0..width {
            if !ink(x, y) {
                continue;
            }
            if y == 0 || !ink(x, y - 1) {
                set(x, y, 0);
            }
            if x + 1 == width || !ink(x + 1, y) {
                set(x + 1, y, 1);
            }
            if y + 1 == width || !ink(x, y + 1) {
                set(x + 1, y + 1, 2);
            }
            if x == 0 || !ink(x - 1, y) {
                set(x, y + 1, 3);
            }
        }
    }
    let mut used = vec![0u8; stride * stride];
    let mut result = Vec::new();
    for start in 0..stride * stride {
        for direction in 0..4 {
            let bit = 1 << direction;
            if out[start] & bit == 0 || used[start] & bit != 0 {
                continue;
            }
            result.push(walk(&out, &mut used, stride, start, direction));
        }
    }
    result
}

/// The next direction at a vertex with outgoing edges `mask` after arriving going `arrived`:
/// the turn toward the ink, straight on, then away from it.
fn pick(mask: u8, arrived: usize) -> usize {
    [(arrived + 1) % 4, arrived, (arrived + 3) % 4]
        .into_iter()
        .find(|direction| mask & (1 << direction) != 0)
        // invariant: a vertex we arrive at has an outgoing edge, every ink edge ends on one.
        .unwrap_or(arrived)
}

fn walk(out: &[u8], used: &mut [u8], stride: usize, start: usize, first: usize) -> Vec<(i32, i32)> {
    let corner = |at: usize| {
        (
            i32::try_from(at % stride).unwrap_or(0),
            i32::try_from(at / stride).unwrap_or(0),
        )
    };
    let mut points = vec![corner(start)];
    let (mut at, mut direction) = (start, first);
    loop {
        used[at] |= 1 << direction;
        let (dx, dy) = DIRECTIONS[direction];
        let (x, y) = corner(at);
        let next =
            usize::try_from(y + dy).unwrap_or(0) * stride + usize::try_from(x + dx).unwrap_or(0);
        let chosen = pick(out[next], direction);
        if next == start && chosen == first {
            break;
        }
        points.push(corner(next));
        at = next;
        direction = chosen;
    }
    straightened(&points)
}

/// `points` without the vertices that lie on a straight run.
fn straightened(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let count = points.len();
    let step =
        |from: (i32, i32), to: (i32, i32)| ((to.0 - from.0).signum(), (to.1 - from.1).signum());
    (0..count)
        .filter(|&at| {
            let before = points[(at + count - 1) % count];
            let after = points[(at + 1) % count];
            step(before, points[at]) != step(points[at], after)
        })
        .map(|at| points[at])
        .collect()
}

/// The sign of `value`, 0 for zero (`f64::signum` gives 1).
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Where the outline enters and leaves one vertex, and the arc between them when it is rounded.
struct Corner {
    entry: (f64, f64),
    exit: (f64, f64),
    arc: Option<((f64, f64), (f64, f64))>,
}

/// One loop as a closed outline: every convex corner rounded by `rounding` modules (at most half
/// a module, so two arcs on one edge meet but never cross), concave corners sharp.
pub(crate) fn outline(frame: Frame, vertices: &[(i32, i32)], rounding: f64) -> ClosedPath {
    let count = vertices.len();
    let at = |index: usize| {
        let (x, y) = vertices[index % count];
        (f64::from(x), f64::from(y))
    };
    let corners: Vec<Corner> = (0..count)
        .map(|index| {
            let (before, here, after) = (at(index + count - 1), at(index), at(index + 1));
            let towards = |a: (f64, f64), b: (f64, f64)| (sign(b.0 - a.0), sign(b.1 - a.1));
            let (inbound, outbound) = (towards(before, here), towards(here, after));
            let convex = inbound.0 * outbound.1 - inbound.1 * outbound.0 > 0.0;
            if rounding > 1e-9 && convex {
                let entry = (here.0 - inbound.0 * rounding, here.1 - inbound.1 * rounding);
                let exit = (
                    here.0 + outbound.0 * rounding,
                    here.1 + outbound.1 * rounding,
                );
                let k = K * rounding;
                let c1 = (entry.0 + inbound.0 * k, entry.1 + inbound.1 * k);
                let c2 = (exit.0 - outbound.0 * k, exit.1 - outbound.1 * k);
                Corner {
                    entry,
                    exit,
                    arc: Some((c1, c2)),
                }
            } else {
                Corner {
                    entry: here,
                    exit: here,
                    arc: None,
                }
            }
        })
        .collect();
    let start = corners[0].exit;
    let mut current = start;
    let mut segments = Vec::with_capacity(count * 2);
    for index in 1..=count {
        let corner = &corners[index % count];
        if (corner.entry.0 - current.0).abs() > 1e-9 || (corner.entry.1 - current.1).abs() > 1e-9 {
            let (from, to) = (
                frame.pt(current.0, current.1),
                frame.pt(corner.entry.0, corner.entry.1),
            );
            segments.push(Cubic {
                c1: from,
                c2: to,
                to,
            });
        }
        if let Some((c1, c2)) = corner.arc {
            segments.push(Cubic {
                c1: frame.pt(c1.0, c1.1),
                c2: frame.pt(c2.0, c2.1),
                to: frame.pt(corner.exit.0, corner.exit.1),
            });
        }
        current = corner.exit;
    }
    ClosedPath {
        start: frame.pt(start.0, start.1),
        segments,
    }
}

/// The loops of the ink cells as outlines; `inverted` runs every one the other way round.
pub(crate) fn outlines(
    frame: Frame,
    width: usize,
    ink: impl Fn(usize, usize) -> bool,
    rounding: f64,
    inverted: bool,
) -> Vec<ClosedPath> {
    loops(width, ink)
        .iter()
        .map(|vertices| {
            let path = outline(frame, vertices, rounding);
            if inverted { reversed(&path) } else { path }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&str]) -> impl Fn(usize, usize) -> bool {
        let rows: Vec<Vec<bool>> = rows
            .iter()
            .map(|row| row.chars().map(|c| c == '#').collect())
            .collect();
        move |x, y| rows[y][x]
    }

    fn shoelace(vertices: &[(i32, i32)]) -> f64 {
        let n = vertices.len();
        (0..n)
            .map(|i| {
                let (a, b) = (vertices[i], vertices[(i + 1) % n]);
                f64::from(a.0 * b.1 - b.0 * a.1)
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn one_cell_is_one_clockwise_square() {
        let found = loops(3, grid(&["...", ".#.", "..."]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].len(), 4, "collinear vertices are gone");
        assert!((shoelace(&found[0]) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_ring_is_an_outer_loop_and_a_hole_with_opposite_winding() {
        let found = loops(3, grid(&["###", "#.#", "###"]));
        assert_eq!(found.len(), 2);
        let mut areas: Vec<f64> = found.iter().map(|l| shoelace(l)).collect();
        areas.sort_by(f64::total_cmp);
        assert!((areas[0] + 1.0).abs() < 1e-12, "the hole is -1");
        assert!((areas[1] - 9.0).abs() < 1e-12, "the outer is 9");
    }

    #[test]
    fn cells_that_touch_only_at_a_corner_are_two_loops() {
        let found = loops(2, grid(&["#.", ".#"]));
        assert_eq!(found.len(), 2, "no pinch vertex");
        assert!(found.iter().all(|l| l.len() == 4));
        // and the light cells around a dark diagonal pair do not merge the dark ones
        let inverse = loops(2, grid(&[".#", "#."]));
        assert_eq!(inverse.len(), 2);
    }

    #[test]
    fn an_l_shape_keeps_only_its_six_corners() {
        let found = loops(2, grid(&["#.", "##"]));
        assert_eq!(found[0].len(), 6);
        assert!((shoelace(&found[0]) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rounding_replaces_convex_corners_only() {
        let frame = Frame {
            half: 1.0,
            module: 1.0,
        };
        let square = outline(frame, &[(0, 0), (1, 0), (1, 1), (0, 1)], 0.5);
        assert_eq!(
            square.segments.len(),
            4,
            "four arcs, the straight runs vanish"
        );
        let l = loops(2, grid(&["#.", "##"]));
        let path = outline(frame, &l[0], 0.3);
        let arcs = path.segments.iter().filter(|c| c.c2 != c.to).count();
        assert_eq!(arcs, 5, "five convex corners, one concave stays sharp");
    }
}
