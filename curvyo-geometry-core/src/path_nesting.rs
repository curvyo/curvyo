//! The nesting of flattened machine paths (ADR 0064 §5, `0029` criterion 22, `0159` `adrs.md`
//! PL4): the innermost closed path that contains each path.

use curvyo_document_core::Vec2;

use crate::outline_nesting::outline_nesting;
use crate::point_um::PointUm;
use crate::{Outline, OutlineTriple};

/// One unit to nest: a flattened path in µm (a closed one does not repeat its first point) and
/// whether it is closed; `None` for a unit that is no path (a laser hatch), which has no parent
/// and holds no child.
pub type NestPath<'a> = Option<(&'a [PointUm], bool)>;

/// The innermost closed path that contains each unit: by `outline_nesting` for closed paths of
/// three points or more, by all nodes inside for open paths; a unit that is no path has none.
#[must_use]
pub fn path_parents(units: &[NestPath<'_>]) -> Vec<Option<usize>> {
    let closed: Vec<usize> = (0..units.len())
        .filter(|&i| matches!(units[i], Some((points, true)) if points.len() >= 3))
        .collect();
    let triples: Vec<Vec<OutlineTriple>> = closed
        .iter()
        .map(|&i| {
            ring(units[i])
                .iter()
                .map(|p| (p.to_mm(), Vec2::ZERO, Vec2::ZERO))
                .collect()
        })
        .collect();
    let outlines: Vec<Outline<'_>> = triples.iter().map(|t| Outline::new(t, true)).collect();
    let nesting = outline_nesting(&outlines);
    let mut parents = vec![None; units.len()];
    for (at, &unit) in closed.iter().enumerate() {
        parents[unit] = nesting[at].parent.map(|p| closed[p]);
    }
    let boxes: Vec<(PointUm, PointUm)> = closed.iter().map(|&i| bounds(ring(units[i]))).collect();
    for (index, unit) in units.iter().enumerate() {
        let Some((points, false)) = *unit else {
            continue;
        };
        let own = bounds(points);
        parents[index] = closed
            .iter()
            .enumerate()
            .filter(|&(at, _)| inside_box(own, boxes[at]))
            .filter(|&(_, &q)| points.iter().all(|&p| winding(ring(units[q]), p) != 0))
            .max_by_key(|&(at, _)| (nesting[at].depth, std::cmp::Reverse(at)))
            .map(|(_, &q)| q);
    }
    parents
}

fn ring(unit: NestPath<'_>) -> &[PointUm] {
    unit.map_or(&[], |(points, _)| points)
}

fn bounds(points: &[PointUm]) -> (PointUm, PointUm) {
    points.iter().fold(
        (
            PointUm::new(i64::MAX, i64::MAX),
            PointUm::new(i64::MIN, i64::MIN),
        ),
        |(lo, hi), p| {
            (
                PointUm::new(lo.x.min(p.x), lo.y.min(p.y)),
                PointUm::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

fn inside_box(inner: (PointUm, PointUm), outer: (PointUm, PointUm)) -> bool {
    inner.0.x >= outer.0.x
        && inner.0.y >= outer.0.y
        && inner.1.x <= outer.1.x
        && inner.1.y <= outer.1.y
}

/// The winding number of the closed polygon `ring` around `p`, exact in integers.
fn winding(ring: &[PointUm], p: PointUm) -> i32 {
    let mut wind = 0;
    for (index, &a) in ring.iter().enumerate() {
        let b = ring[(index + 1) % ring.len()];
        let cross = i128::from(b.x - a.x) * i128::from(p.y - a.y)
            - i128::from(p.x - a.x) * i128::from(b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && cross > 0 {
                wind += 1;
            }
        } else if b.y <= p.y && cross < 0 {
            wind -= 1;
        }
    }
    wind
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i64, size: i64) -> Vec<PointUm> {
        vec![
            PointUm::new(x, x),
            PointUm::new(x + size, x),
            PointUm::new(x + size, x + size),
            PointUm::new(x, x + size),
        ]
    }

    #[test]
    fn a_path_inside_a_closed_path_has_it_as_parent_and_a_hatch_has_none() {
        let outer = square(0, 100);
        let inner = square(10, 20);
        let line = vec![PointUm::new(50, 50), PointUm::new(60, 60)];
        let units = [
            Some((outer.as_slice(), true)),
            Some((inner.as_slice(), true)),
            Some((line.as_slice(), false)),
            None,
        ];
        assert_eq!(path_parents(&units), [None, Some(0), Some(0), None]);
    }
}
