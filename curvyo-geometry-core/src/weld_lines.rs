//! Weld's chaining on the grid (`specs/0068-laserable-lettering/adrs.md` decision 3): inner line
//! pieces become open outlines between junctions and closed loops, each coincident grid segment
//! once, in a fixed order.

use std::collections::{BTreeMap, BTreeSet};

use curvyo_document_core::Point;

use crate::boolean_cleanup::{cleanup_open, distance_squared_to_segment, remove_near_collinear};
use crate::boolean_grid::{Path64, Paths64, Point64, double_area, to_millimetres};
use crate::region_contact::EdgeIndex;
use crate::stack_regions::GridBox;

/// An open line shorter than the kernel tolerance (0.01 mm, in grid units) that runs along the
/// union's boundary is a rounding remnant, a near-double of that boundary, and is dropped
/// (`adrs.md` decision 3, dated note of 2026-10-20). A short line elsewhere is real visible
/// boundary, such as the trim edge where a shape's corner just pokes out, and is kept.
const SHORT_LINE_UNITS: f64 = 10.0;

/// How close, in grid units, a line must run along the union's boundary to count as a double of
/// it: two units, one of rounding and one of cleanup.
const REMNANT_UNITS: i64 = 2;

/// A grid point as a sortable key.
type Key = (i64, i64);

fn key(point: Point64) -> Key {
    (point.x, point.y)
}

fn point(key: Key) -> Point64 {
    Point64::new(key.0, key.1)
}

/// An undirected segment, smaller end first.
type Segment = (Key, Key);

fn segment(a: Key, b: Key) -> Segment {
    if a <= b { (a, b) } else { (b, a) }
}

/// The lines of `pieces` as `(points, closed)` in millimetres: closed loops (wound with a
/// positive area, starting at their smallest point) by first node, then open lines from their
/// smaller end by first and second node.
///
/// A grid segment that one edge of `boundary` holds within two units (both ends and the
/// midpoint) is a double of that edge and is dropped, whatever its length: the fold and the
/// final union may round the same crossing to neighbouring grid points (`adrs.md` decision 3,
/// dated note of 2026-10-20). Of two inner segments that hold each other so, the longer stays.
pub(crate) fn chain_lines(pieces: &[Path64], boundary: &Paths64) -> Vec<(Vec<Point>, bool)> {
    let index = EdgeIndex::new(boundary, GridBox::of(boundary));
    let segments: BTreeSet<Segment> = pieces
        .iter()
        .flat_map(|piece| piece.windows(2))
        .filter(|pair| pair[0] != pair[1] && !index.hugs(pair[0], pair[1], REMNANT_UNITS))
        .map(|pair| segment(key(pair[0]), key(pair[1])))
        .collect();
    let segments = without_inner_doubles(segments);
    let mut neighbours: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    for &(a, b) in &segments {
        neighbours.entry(a).or_default().push(b);
        neighbours.entry(b).or_default().push(a);
    }
    for list in neighbours.values_mut() {
        list.sort_unstable();
    }
    let mut visited: BTreeSet<Segment> = BTreeSet::new();
    let open = open_chains(&neighbours, &mut visited);
    let mut closed = closed_loops(&neighbours, &mut visited);
    let mut open = without_short_doubles(open, boundary);
    closed.retain(|ring| ring.len() >= 3);
    closed.sort_by_key(|ring| ring.iter().copied().map(key).collect::<Vec<_>>());
    open.sort_by_key(|line| (key(line[0]), key(line[1])));
    let millimetres = |path: &Path64| path.iter().map(to_millimetres).collect::<Vec<_>>();
    closed
        .iter()
        .map(|ring| (millimetres(ring), true))
        .chain(open.iter().map(|line| (millimetres(line), false)))
        .collect()
}

/// Every chain from a junction (a point of degree other than 2) to the next, in `(x, y)` order
/// of the junctions and of the far points; each stored from its smaller end and cleaned.
fn open_chains(
    neighbours: &BTreeMap<Key, Vec<Key>>,
    visited: &mut BTreeSet<Segment>,
) -> Vec<Path64> {
    let mut chains = Vec::new();
    for (&junction, list) in neighbours.iter().filter(|(_, list)| list.len() != 2) {
        for &first in list {
            if visited.contains(&segment(junction, first)) {
                continue;
            }
            let mut chain = vec![junction];
            let mut current = first;
            visited.insert(segment(junction, first));
            chain.push(current);
            while neighbours[&current].len() == 2 {
                let Some(&next) = neighbours[&current]
                    .iter()
                    .find(|&&next| !visited.contains(&segment(current, next)))
                else {
                    break;
                };
                visited.insert(segment(current, next));
                current = next;
                chain.push(current);
            }
            if chain[chain.len() - 1] < chain[0] {
                chain.reverse();
            }
            chains.push(cleanup_open(&chain.into_iter().map(point).collect()));
        }
    }
    chains
}

/// The cycles left after the open chains: every point on them has degree 2. Each starts at its
/// smallest point, is cleaned and is wound with a positive area.
fn closed_loops(
    neighbours: &BTreeMap<Key, Vec<Key>>,
    visited: &mut BTreeSet<Segment>,
) -> Vec<Path64> {
    let mut loops = Vec::new();
    for (&start, list) in neighbours {
        let Some(&first) = list
            .iter()
            .find(|&&n| !visited.contains(&segment(start, n)))
        else {
            continue;
        };
        let mut ring = vec![start];
        let mut current = first;
        visited.insert(segment(start, first));
        while current != start {
            ring.push(current);
            let Some(&next) = neighbours[&current]
                .iter()
                .find(|&&next| !visited.contains(&segment(current, next)))
            else {
                break;
            };
            visited.insert(segment(current, next));
            current = next;
        }
        let mut ring = remove_near_collinear(&ring.into_iter().map(point).collect());
        if double_area(&ring) < 0 {
            ring[1..].reverse();
        }
        loops.push(ring);
    }
    loops
}

/// Width, in grid units, of the x strips that [`without_inner_doubles`] files segments under.
const STRIP_UNITS: i64 = 1_000;

/// `segments` without those that a longer one holds within two units (both ends and the
/// midpoint): doubles of one line that rounding left a unit apart. Longest first, ties by the
/// segment, so the result does not depend on the input order.
fn without_inner_doubles(segments: BTreeSet<Segment>) -> BTreeSet<Segment> {
    let length_squared = |(a, b): Segment| {
        let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        dx * dx + dy * dy
    };
    let mut order: Vec<Segment> = segments.into_iter().collect();
    order.sort_by_key(|&s| (std::cmp::Reverse(length_squared(s)), s));
    #[allow(clippy::cast_precision_loss)] // a few units, exact
    let reach_squared = (REMNANT_UNITS * REMNANT_UNITS) as f64;
    let strips = |low: i64, high: i64| low.div_euclid(STRIP_UNITS)..=high.div_euclid(STRIP_UNITS);
    let mut filed: BTreeMap<i64, Vec<Segment>> = BTreeMap::new();
    let mut kept = BTreeSet::new();
    for s in order {
        let (a, b) = (point(s.0), point(s.1));
        let middle = Point64::new(i64::midpoint(a.x, b.x), i64::midpoint(a.y, b.y));
        let (low, high) = (a.x.min(b.x), a.x.max(b.x));
        let held = strips(low - REMNANT_UNITS, high + REMNANT_UNITS).any(|strip| {
            filed.get(&strip).is_some_and(|list| {
                list.iter().any(|&(c, d)| {
                    [a, b, middle].iter().all(|&p| {
                        distance_squared_to_segment(p, point(c), point(d)) <= reach_squared
                    })
                })
            })
        });
        if !held {
            for strip in strips(low, high) {
                filed.entry(strip).or_default().push(s);
            }
            kept.insert(s);
        }
    }
    kept
}

/// `open` without the short lines (under the kernel tolerance) that run along another line: every
/// vertex and segment midpoint of such a line lies within two units of the union's boundary or of
/// an open line not yet dropped. Lines are judged in order, so of two short doubles one stays.
fn without_short_doubles(open: Vec<Path64>, boundary: &Paths64) -> Vec<Path64> {
    let mut lines: Paths64 = boundary.clone();
    lines.extend(
        open.iter()
            .flat_map(|line| line.windows(2).map(<[Point64]>::to_vec)),
    );
    let index = EdgeIndex::new(&lines, GridBox::of(&lines));
    let mut dropped: BTreeSet<Segment> = BTreeSet::new();
    let mut kept = Vec::with_capacity(open.len());
    for line in open {
        let own: BTreeSet<Segment> = line
            .windows(2)
            .map(|pair| segment(key(pair[0]), key(pair[1])))
            .collect();
        let other = |c: Point64, d: Point64| {
            let edge = segment(key(c), key(d));
            !own.contains(&edge) && !dropped.contains(&edge)
        };
        let midpoints = line.windows(2).map(|pair| {
            Point64::new(
                i64::midpoint(pair[0].x, pair[1].x),
                i64::midpoint(pair[0].y, pair[1].y),
            )
        });
        let double = length_units(&line) < SHORT_LINE_UNITS
            && line
                .iter()
                .copied()
                .chain(midpoints)
                .all(|p| index.near(p, REMNANT_UNITS, other));
        if double {
            dropped.extend(own);
        } else {
            kept.push(line);
        }
    }
    kept
}

/// The length of a polyline in grid units.
fn length_units(line: &Path64) -> f64 {
    line.windows(2)
        .map(|pair| {
            // Precision loss above 2^53 only moves a threshold.
            #[allow(clippy::cast_precision_loss)]
            let (dx, dy) = (
                (pair[1].x - pair[0].x) as f64,
                (pair[1].y - pair[0].y) as f64,
            );
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(coordinates: &[(i64, i64)]) -> Path64 {
        coordinates
            .iter()
            .map(|&(x, y)| Point64::new(x, y))
            .collect()
    }

    /// Coincident pieces count once, a T junction splits lines, and lines run from the smaller end.
    #[test]
    fn coincident_pieces_are_kept_once_and_junctions_split_lines() {
        let lines = chain_lines(
            &[
                path(&[(0, 0), (0, 10_000), (0, 20_000)]),
                path(&[(0, 20_000), (0, 10_000), (0, 0)]),
                path(&[(0, 10_000), (15_000, 10_000)]),
            ],
            &vec![path(&[
                (90_000, 90_000),
                (99_000, 90_000),
                (99_000, 99_000),
            ])],
        );
        let open: Vec<Vec<Point>> = lines.into_iter().map(|(points, _)| points).collect();
        assert_eq!(
            open,
            vec![
                vec![Point::new(0.0, 0.0), Point::new(0.0, 10.0)],
                vec![Point::new(0.0, 10.0), Point::new(0.0, 20.0)],
                vec![Point::new(0.0, 10.0), Point::new(15.0, 10.0)],
            ]
        );
    }
}
