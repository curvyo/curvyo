//! Cuts a label map into its shared edges (`specs/0089` adrs.md §10): every maximal chain of
//! crack edges with one label pair, ending at junction vertices (pixel corners where three or
//! four labels meet) or closed, and the loops of each colour as edge ids with their direction.
//!
//! An edge runs with its larger label on the right (`y` down, clockwise around that label), so
//! a colour region's loop takes its own edges forward and the edges of a larger neighbour
//! backward. A 2-label checkerboard vertex is passed through, around the pixels of the label
//! that does not connect (`segment::wins`); at a junction where one colour holds two diagonal
//! pixels its loop goes on through the vertex (the other two labels differ), as `segment.rs`
//! connects them.

use crate::segment::wins;

/// A pixel corner, `x` right, `y` down.
pub(crate) type Vertex = (i32, i32);

/// The unit steps right, down, left, up (direction 0 to 3).
const STEPS: [Vertex; 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// One shared edge: its pixel corners in order (a closed edge does not repeat its first), the
/// label on its left and on its right (`left < right`), whether it is closed, the direction of
/// its first step and of its last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Edge {
    pub vertices: Vec<Vertex>,
    pub left: u32,
    pub right: u32,
    pub closed: bool,
    first: u8,
    last: u8,
}

/// One closed loop around a colour: the edges in order, each forward (the colour on its right)
/// or backward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Loop {
    pub colour: u32,
    pub parts: Vec<(usize, bool)>,
}

/// The labels of a map with the outside as background.
struct Map<'a> {
    labels: &'a [u32],
    width: usize,
    height: usize,
}

impl Map<'_> {
    fn at(&self, x: i32, y: i32) -> u32 {
        match (usize::try_from(x), usize::try_from(y)) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => self.labels[y * self.width + x],
            _ => 0,
        }
    }

    /// The pixels left and right of the crack from `v` in direction `d`.
    fn sides(&self, v: Vertex, d: u8) -> (u32, u32) {
        let (x, y) = v;
        match d {
            0 => (self.at(x, y - 1), self.at(x, y)),
            1 => (self.at(x, y), self.at(x - 1, y)),
            2 => (self.at(x - 1, y), self.at(x - 1, y - 1)),
            _ => (self.at(x - 1, y - 1), self.at(x, y - 1)),
        }
    }

    /// Whether the crack from `v` in direction `d` separates two labels.
    fn crack(&self, v: Vertex, d: u8) -> bool {
        let (l, r) = self.sides(v, d);
        l != r
    }

    /// The four pixels around `v`: top left, top right, bottom right, bottom left.
    fn around(&self, v: Vertex) -> [u32; 4] {
        let (x, y) = v;
        [
            self.at(x - 1, y - 1),
            self.at(x, y - 1),
            self.at(x, y),
            self.at(x - 1, y),
        ]
    }

    /// The index of the crack from `v` in direction `d`: horizontal ones first, then vertical.
    fn index(&self, v: Vertex, d: u8) -> usize {
        let (x, y) = match d {
            2 => (v.0 - 1, v.1),
            3 => (v.0, v.1 - 1),
            _ => v,
        };
        // invariant: cracks lie inside the vertex grid.
        let (x, y) = (
            usize::try_from(x).unwrap_or(0),
            usize::try_from(y).unwrap_or(0),
        );
        if d.is_multiple_of(2) {
            y * self.width + x
        } else {
            self.width * (self.height + 1) + y * (self.width + 1) + x
        }
    }

    /// The direction to leave `v` in after arriving in direction `d`, or `None` at a junction:
    /// the one other crack, or around the loser's pixel at a 2-label checkerboard.
    fn next(&self, v: Vertex, d: u8) -> Option<u8> {
        let back = (d + 2) % 4;
        let arms: Vec<u8> = (0..4).filter(|&a| self.crack(v, a)).collect();
        match arms.len() {
            2 => arms.into_iter().find(|&a| a != back),
            4 => {
                let [tl, tr, br, bl] = self.around(v);
                if tl != br || tr != bl {
                    return None;
                }
                // Arms around top right (up, right) and bottom left (left, down) when the
                // top-left label connects; else around top left (up, left) and bottom right.
                let pairs: [(u8, u8); 2] = if wins(tl, tr) {
                    [(3, 0), (2, 1)]
                } else {
                    [(3, 2), (0, 1)]
                };
                pairs.iter().find_map(|&(a, b)| {
                    if back == a {
                        Some(b)
                    } else if back == b {
                        Some(a)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

fn step(v: Vertex, d: u8) -> Vertex {
    let s = STEPS[usize::from(d)];
    (v.0 + s.0, v.1 + s.1)
}

/// Walks from `start` in direction `d` through every vertex that is no junction, marking the
/// cracks; returns the vertices after `start` (the last one a junction or `start` again), the
/// last direction, and whether it came back to the first crack.
fn walk(map: &Map<'_>, seen: &mut [bool], start: Vertex, d: u8) -> (Vec<Vertex>, u8, bool) {
    let (mut v, mut dir) = (start, d);
    let mut out = Vec::new();
    loop {
        seen[map.index(v, dir)] = true;
        v = step(v, dir);
        out.push(v);
        match map.next(v, dir) {
            Some(n) if (v, n) == (start, d) => return (out, dir, true),
            Some(n) => dir = n,
            None => return (out, dir, false),
        }
    }
}

/// The shared edges of `labels` (`width` x `height`, `0` the background) in scan order of their
/// first crack, and the loops of every colour.
pub(crate) fn edges(labels: &[u32], width: usize, height: usize) -> (Vec<Edge>, Vec<Loop>) {
    let map = Map {
        labels,
        width,
        height,
    };
    let mut seen = vec![false; width * (height + 1) + (width + 1) * height];
    let mut out = Vec::new();
    // invariant: pictures are at most 2 x 8192 px wide (`raster.rs`).
    let (w, h) = (
        i32::try_from(width).unwrap_or(i32::MAX),
        i32::try_from(height).unwrap_or(i32::MAX),
    );
    let starts = (0..=h)
        .flat_map(|y| (0..w).map(move |x| ((x, y), 0_u8)))
        .chain((0..h).flat_map(|y| (0..=w).map(move |x| ((x, y), 1_u8))));
    for (v, d) in starts {
        if !map.crack(v, d) || seen[map.index(v, d)] {
            continue;
        }
        // The larger label on the right.
        let (l, r) = map.sides(v, d);
        let (from, dir) = if r > l {
            (v, d)
        } else {
            (step(v, d), (d + 2) % 4)
        };
        let (ahead, last, closed) = walk(&map, &mut seen, from, dir);
        if closed {
            let mut vertices = vec![from];
            vertices.extend_from_slice(&ahead[..ahead.len() - 1]);
            out.push(Edge {
                vertices,
                left: l.min(r),
                right: l.max(r),
                closed: true,
                first: dir,
                last,
            });
            continue;
        }
        // Back from `from` against the edge, then reversed.
        let (behind, back_last, _) = walk(&map, &mut seen, step(from, dir), (dir + 2) % 4);
        let mut vertices: Vec<Vertex> = behind.iter().rev().copied().collect();
        vertices.extend_from_slice(&ahead);
        out.push(Edge {
            vertices,
            left: l.min(r),
            right: l.max(r),
            closed: false,
            first: (back_last + 2) % 4,
            last,
        });
    }
    let loops = loops(&out);
    (out, loops)
}

/// The quadrant (top left 0, top right 1, bottom right 2, bottom left 3) on the right of a step
/// leaving a vertex in direction `d`.
fn leaving(d: u8) -> u8 {
    [2, 3, 0, 1][usize::from(d)]
}

/// The quadrant on the right of a step arriving at a vertex in direction `d`.
fn arriving(d: u8) -> u8 {
    [3, 0, 1, 2][usize::from(d)]
}

/// A half-edge of one colour: its start vertex, the quadrant on its right there, the edge and
/// whether the colour takes it forward.
type Half = (Vertex, u8, usize, bool);

/// The loops of every colour, colours in order, each starting at its first unused edge.
fn loops(edges: &[Edge]) -> Vec<Loop> {
    let colours = edges.iter().map(|e| e.right).max().unwrap_or(0);
    let mut halves = half_edges(edges, colours);
    let mut out = Vec::new();
    for colour in 1..=colours {
        for (i, e) in edges.iter().enumerate() {
            if e.closed && (e.right == colour || e.left == colour) {
                out.push(Loop {
                    colour,
                    parts: vec![(i, e.right == colour)],
                });
            }
        }
        let list = &mut halves[colour as usize];
        list.sort_unstable();
        let mut used = vec![false; list.len()];
        for s in 0..list.len() {
            if !used[s] {
                let parts = follow(edges, list, &mut used, s);
                out.push(Loop { colour, parts });
            }
        }
    }
    out
}

/// The half-edges of the open edges per colour (`colours + 1` lists, the background's empty).
fn half_edges(edges: &[Edge], colours: u32) -> Vec<Vec<Half>> {
    let mut halves: Vec<Vec<Half>> = vec![Vec::new(); colours as usize + 1];
    for (i, e) in edges.iter().enumerate() {
        if e.closed {
            continue;
        }
        let (first, last) = (e.vertices[0], e.vertices[e.vertices.len() - 1]);
        halves[e.right as usize].push((first, leaving(e.first), i, true));
        if e.left != 0 {
            halves[e.left as usize].push((last, leaving((e.last + 2) % 4), i, false));
        }
    }
    halves
}

/// The loop of one colour from its sorted half-edges `list`, starting at `s`, marking the
/// halves it takes in `used`.
fn follow(edges: &[Edge], list: &[Half], used: &mut [bool], s: usize) -> Vec<(usize, bool)> {
    let mut parts = Vec::new();
    let mut at = s;
    loop {
        used[at] = true;
        let (_, _, edge, forward) = list[at];
        parts.push((edge, forward));
        let e = &edges[edge];
        let (end, arrive) = if forward {
            (e.vertices[e.vertices.len() - 1], e.last)
        } else {
            (e.vertices[0], (e.first + 2) % 4)
        };
        let quadrant = arriving(arrive);
        let from = list.partition_point(|h| h.0 < end);
        let to = list.partition_point(|h| h.0 <= end);
        let free: Vec<usize> = (from..to).filter(|&k| !used[k]).collect();
        // Two ways on: the colour holds two diagonal pixels and connects through.
        let next = match free[..] {
            [] => None,
            [one] => Some(one),
            _ => free.iter().copied().find(|&k| list[k].1 != quadrant),
        };
        match next {
            Some(n) => at = n,
            None => return parts,
        }
    }
}

#[cfg(test)]
#[allow(clippy::many_single_char_names)]
mod tests {
    use super::*;

    fn grid(rows: &[&str]) -> (Vec<u32>, usize, usize) {
        let labels = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c.to_digit(10).unwrap()))
            .collect();
        (labels, rows[0].len(), rows.len())
    }

    /// Twice the signed area of a closed vertex list (positive clockwise on screen).
    fn area2(v: &[Vertex]) -> i64 {
        (0..v.len())
            .map(|i| {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                i64::from(a.0) * i64::from(b.1) - i64::from(b.0) * i64::from(a.1)
            })
            .sum()
    }

    fn loop_vertices(edges: &[Edge], l: &Loop) -> Vec<Vertex> {
        let mut out = Vec::new();
        for &(e, forward) in &l.parts {
            let mut v = edges[e].vertices.clone();
            if !forward {
                v.reverse();
            }
            if edges[e].closed {
                out.extend(v);
            } else {
                out.extend_from_slice(&v[..v.len() - 1]);
            }
        }
        out
    }

    #[test]
    fn one_square_is_one_closed_edge_and_one_loop() {
        let (l, w, h) = grid(&["000", "010", "000"]);
        let (edges, loops) = edges(&l, w, h);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].closed);
        assert_eq!((edges[0].left, edges[0].right), (0, 1));
        assert_eq!(loops.len(), 1);
        assert_eq!(area2(&loop_vertices(&edges, &loops[0])), 2);
    }

    #[test]
    fn a_triple_junction_cuts_three_edges_and_every_loop_closes() {
        // 1 left, 2 top right, 3 bottom right; the outside is background.
        let (l, w, h) = grid(&["1122", "1122", "1133", "1133"]);
        let (edges, loops) = edges(&l, w, h);
        let inner: Vec<_> = edges.iter().filter(|e| e.left != 0).collect();
        // 1|2, 1|3, 2|3 inside; each colour against the background outside.
        assert_eq!(inner.len(), 3, "{edges:?}");
        assert!(inner.iter().all(|e| !e.closed));
        assert_eq!(loops.len(), 3);
        let areas: Vec<i64> = loops
            .iter()
            .map(|l| area2(&loop_vertices(&edges, l)))
            .collect();
        // 8, 4 and 4 pixels, clockwise.
        assert_eq!(areas, [16, 8, 8]);
        // Every loop is closed: consecutive parts meet.
        for l in &loops {
            let v = loop_vertices(&edges, l);
            for k in 0..v.len() {
                let (a, b) = (v[k], v[(k + 1) % v.len()]);
                assert_eq!((a.0 - b.0).abs() + (a.1 - b.1).abs(), 1, "{v:?}");
            }
        }
    }

    #[test]
    fn a_hole_is_walked_backward_by_its_surround() {
        let (l, w, h) = grid(&["11111", "12221", "12221", "11111"]);
        let (edges, loops) = edges(&l, w, h);
        let hole = loops
            .iter()
            .find(|l| l.colour == 1 && l.parts.len() == 1 && !l.parts[0].1);
        let hole = hole.expect("the hole of colour 1");
        assert!(area2(&loop_vertices(&edges, hole)) < 0);
        let outer: i64 = loops
            .iter()
            .filter(|l| l.colour == 1)
            .map(|l| area2(&loop_vertices(&edges, l)))
            .sum();
        // 20 pixels minus the 6 of colour 2.
        assert_eq!(outer, 28);
    }

    #[test]
    fn a_checkerboard_vertex_is_passed_and_a_diagonal_line_is_one_edge() {
        let (l, w, h) = grid(&["100", "010", "001"]);
        let (edges, loops) = edges(&l, w, h);
        assert_eq!(edges.len(), 1, "{edges:?}");
        assert!(edges[0].closed);
        assert_eq!(loops.len(), 1);
        assert_eq!(area2(&loop_vertices(&edges, &loops[0])), 6);
    }

    #[test]
    fn a_diagonal_touch_at_a_junction_goes_through() {
        // Colour 1 at top left and bottom right, 2 and 3 in the other corners.
        let (l, w, h) = grid(&["12", "31"]);
        let (edges, loops) = edges(&l, w, h);
        let ones: Vec<_> = loops.iter().filter(|l| l.colour == 1).collect();
        assert_eq!(ones.len(), 1, "{loops:?} {edges:?}");
        assert_eq!(area2(&loop_vertices(&edges, ones[0])), 4);
    }
}
