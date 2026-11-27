//! The nesting of the paths of one block (ADR 0064 §5, `0029` criterion 22): the innermost closed
//! path that contains each unit.

use curvyo_document_core::Vec2;
use curvyo_geometry_core::{Outline, OutlineTriple, outline_nesting};

use crate::job_input::PointUm;
use crate::job_order::{Unit, UnitShape};

/// The innermost closed path that contains each unit: by `outline_nesting` for closed paths, by
/// all nodes inside for open paths; hatches have none.
pub(crate) fn parents(units: &[Unit]) -> Vec<Option<usize>> {
    let closed: Vec<usize> = (0..units.len())
        .filter(|&i| matches!(&units[i].shape, UnitShape::Path { closed: true, points } if points.len() >= 3))
        .collect();
    let triples: Vec<Vec<OutlineTriple>> = closed
        .iter()
        .map(|&i| match &units[i].shape {
            UnitShape::Path { points, .. } => points
                .iter()
                .map(|p| (p.to_mm(), Vec2::ZERO, Vec2::ZERO))
                .collect(),
            UnitShape::Hatch(_) => Vec::new(),
        })
        .collect();
    let outlines: Vec<Outline<'_>> = triples.iter().map(|t| Outline::new(t, true)).collect();
    let nesting = outline_nesting(&outlines);
    let mut parents = vec![None; units.len()];
    for (at, &unit) in closed.iter().enumerate() {
        parents[unit] = nesting[at].parent.map(|p| closed[p]);
    }
    let boxes: Vec<(PointUm, PointUm)> = closed
        .iter()
        .map(|&i| bounds(path_points(&units[i])))
        .collect();
    for (index, unit) in units.iter().enumerate() {
        let UnitShape::Path {
            points,
            closed: false,
        } = &unit.shape
        else {
            continue;
        };
        let own = bounds(points);
        parents[index] = closed
            .iter()
            .enumerate()
            .filter(|&(at, _)| inside_box(own, boxes[at]))
            .filter(|&(_, &q)| {
                points
                    .iter()
                    .all(|&p| winding(path_points(&units[q]), p) != 0)
            })
            .max_by_key(|&(at, _)| (nesting[at].depth, std::cmp::Reverse(at)))
            .map(|(_, &q)| q);
    }
    parents
}

fn path_points(unit: &Unit) -> &[PointUm] {
    match &unit.shape {
        UnitShape::Path { points, .. } => points,
        UnitShape::Hatch(_) => &[],
    }
}

pub(crate) fn bounds(points: &[PointUm]) -> (PointUm, PointUm) {
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

pub(crate) fn inside_box(inner: (PointUm, PointUm), outer: (PointUm, PointUm)) -> bool {
    inner.0.x >= outer.0.x
        && inner.0.y >= outer.0.y
        && inner.1.x <= outer.1.x
        && inner.1.y <= outer.1.y
}

/// The winding number of the closed polygon `ring` around `p`, exact in integers.
pub(crate) fn winding(ring: &[PointUm], p: PointUm) -> i32 {
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
