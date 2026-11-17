//! The order of the paths inside one block (ADR 0064 §5; criterion 22, Frage 225 A): a path runs
//! before every closed path that contains it, then the nearest start to the head goes next, ties
//! by stacking order, outline index and node index.

use curvyo_document_core::Vec2;
use curvyo_geometry_core::{Outline, OutlineTriple, outline_nesting};

use crate::job_hatch::{HatchLines, Span};
use crate::job_input::PointUm;

/// One thing to order: a flattened path, or the hatch of one area.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitShape {
    /// A path in µm; a closed one does not repeat its first point.
    Path {
        /// The points.
        points: Vec<PointUm>,
        /// Whether it is closed.
        closed: bool,
    },
    /// The hatch of one area: one set of lines, or two with Crosshatch.
    Hatch(Vec<HatchLines>),
}

/// One unit with its tie-breakers.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// The shape.
    pub shape: UnitShape,
    /// The leaf's stacking order.
    pub stacking: u32,
    /// The outline's index in its leaf.
    pub outline: u32,
}

/// How the chosen unit is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A closed path from node `n`, in its drawn direction.
    Closed(usize),
    /// An open path, reversed when its end was nearer.
    Open { reversed: bool },
    /// A hatch, its first set by line order and direction.
    Hatch { reversed: bool, flipped: bool },
}

/// One ordered burn path: its points in run order and the unit it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedPath {
    /// The points; a closed path ends on its first point.
    pub points: Vec<PointUm>,
    /// The unit's index in the input.
    pub unit: usize,
    /// Whether it is a hatch span.
    pub hatch: bool,
}

/// The ordered burn paths of `units` from `head`, and where the head stands at the end.
/// `bidirectional` alternates the hatch direction line by line.
#[must_use]
pub fn order(units: &[Unit], head: PointUm, bidirectional: bool) -> (Vec<OrderedPath>, PointUm) {
    let parents = parents(units);
    let mut pending = vec![0_usize; units.len()];
    for parent in parents.iter().flatten() {
        pending[*parent] += 1;
    }
    let candidates: Vec<Vec<(PointUm, Entry)>> = units
        .iter()
        .map(|unit| candidates(&unit.shape, bidirectional))
        .collect();
    let mut done = vec![false; units.len()];
    let mut head = head;
    let mut out = Vec::new();
    for _ in 0..units.len() {
        let mut best: Option<(i128, u32, u32, usize, usize, Entry)> = None;
        for (index, unit) in units.iter().enumerate() {
            if done[index] || pending[index] > 0 {
                continue;
            }
            for (tie, &(start, entry)) in candidates[index].iter().enumerate() {
                let key = (
                    head.distance_squared(start),
                    unit.stacking,
                    unit.outline,
                    tie,
                    index,
                );
                if best.is_none_or(|b| key < (b.0, b.1, b.2, b.3, b.4)) {
                    best = Some((key.0, key.1, key.2, key.3, key.4, entry));
                }
            }
        }
        let Some((.., index, entry)) = best else {
            break;
        };
        done[index] = true;
        if let Some(parent) = parents[index] {
            pending[parent] -= 1;
        }
        head = emit(&units[index].shape, index, entry, bidirectional, &mut out).unwrap_or(head);
    }
    (out, head)
}

/// The innermost closed path that contains each unit: by `outline_nesting` for closed paths, by
/// all nodes inside for open paths; hatches have none.
fn parents(units: &[Unit]) -> Vec<Option<usize>> {
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

/// The start points a unit may be entered at, in tie order.
fn candidates(shape: &UnitShape, bidirectional: bool) -> Vec<(PointUm, Entry)> {
    match shape {
        UnitShape::Path {
            points,
            closed: true,
        } => points
            .iter()
            .enumerate()
            .map(|(n, &p)| (p, Entry::Closed(n)))
            .collect(),
        UnitShape::Path {
            points,
            closed: false,
        } => match (points.first(), points.last()) {
            (Some(&first), Some(&last)) => vec![
                (first, Entry::Open { reversed: false }),
                (last, Entry::Open { reversed: true }),
            ],
            _ => Vec::new(),
        },
        UnitShape::Hatch(sets) => match sets.first() {
            Some(lines) if !lines.is_empty() => {
                [(false, false), (false, true), (true, false), (true, true)]
                    .into_iter()
                    .filter_map(|(reversed, flipped)| {
                        hatch_spans(lines, reversed, flipped, bidirectional)
                            .first()
                            .map(|span| (span.0, Entry::Hatch { reversed, flipped }))
                    })
                    .collect()
            }
            _ => Vec::new(),
        },
    }
}

/// The spans of `lines` in run order: lines reversed in order with `reversed`, each line's
/// direction the base one, turned with `flipped` and on every odd line when `bidirectional`.
fn hatch_spans(
    lines: &HatchLines,
    reversed: bool,
    flipped: bool,
    bidirectional: bool,
) -> Vec<Span> {
    let order: Box<dyn Iterator<Item = &Vec<Span>>> = if reversed {
        Box::new(lines.iter().rev())
    } else {
        Box::new(lines.iter())
    };
    let mut out = Vec::new();
    for (k, line) in order.enumerate() {
        let back = flipped ^ (bidirectional && k % 2 == 1);
        if back {
            out.extend(line.iter().rev().map(|&(a, b)| (b, a)));
        } else {
            out.extend(line.iter().copied());
        }
    }
    out
}

/// Appends the paths of `shape` entered by `entry`; returns where the head ends.
fn emit(
    shape: &UnitShape,
    unit: usize,
    entry: Entry,
    bidirectional: bool,
    out: &mut Vec<OrderedPath>,
) -> Option<PointUm> {
    match (shape, entry) {
        (UnitShape::Path { points, .. }, Entry::Closed(n)) => {
            let mut path: Vec<PointUm> = points[n..].iter().chain(&points[..n]).copied().collect();
            path.push(points[n]);
            out.push(OrderedPath {
                points: path,
                unit,
                hatch: false,
            });
            Some(points[n])
        }
        (UnitShape::Path { points, .. }, Entry::Open { reversed }) => {
            let mut path = points.clone();
            if reversed {
                path.reverse();
            }
            let end = path.last().copied();
            out.push(OrderedPath {
                points: path,
                unit,
                hatch: false,
            });
            end
        }
        (UnitShape::Hatch(sets), Entry::Hatch { reversed, flipped }) => {
            let mut head = None;
            for (index, lines) in sets.iter().enumerate() {
                let spans = if index == 0 {
                    hatch_spans(lines, reversed, flipped, bidirectional)
                } else {
                    nearest_variant(lines, head, bidirectional)
                };
                for (a, b) in spans {
                    out.push(OrderedPath {
                        points: vec![a, b],
                        unit,
                        hatch: true,
                    });
                    head = Some(b);
                }
            }
            head
        }
        _ => None,
    }
}

/// The spans of a later hatch set (Crosshatch) entered nearest to `head`, ties in variant order.
fn nearest_variant(lines: &HatchLines, head: Option<PointUm>, bidirectional: bool) -> Vec<Span> {
    let variants = [(false, false), (false, true), (true, false), (true, true)];
    variants
        .into_iter()
        .map(|(reversed, flipped)| hatch_spans(lines, reversed, flipped, bidirectional))
        .filter(|spans| !spans.is_empty())
        .min_by_key(|spans| head.map_or(0, |h| h.distance_squared(spans[0].0)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i64, y: i64, w: i64, h: i64) -> Vec<PointUm> {
        vec![
            PointUm::new(x, y),
            PointUm::new(x + w, y),
            PointUm::new(x + w, y + h),
            PointUm::new(x, y + h),
        ]
    }

    fn closed(points: Vec<PointUm>, stacking: u32) -> Unit {
        Unit {
            shape: UnitShape::Path {
                points,
                closed: true,
            },
            stacking,
            outline: 0,
        }
    }

    fn firsts(paths: &[OrderedPath]) -> Vec<usize> {
        paths.iter().map(|p| p.unit).collect()
    }

    #[test]
    fn inner_shapes_run_before_their_outlines_to_any_depth() {
        // A rectangle, a hole inside it, a text-like square inside the hole (criterion 22 test).
        let units = vec![
            closed(rect(0, 0, 100_000, 100_000), 0),
            closed(rect(20_000, 20_000, 60_000, 60_000), 0),
            closed(rect(40_000, 40_000, 20_000, 20_000), 1),
        ];
        let (paths, _) = order(&units, PointUm::new(0, 0), true);
        assert_eq!(firsts(&paths), [2, 1, 0]);
    }

    #[test]
    fn the_nearest_start_goes_next_and_a_closed_path_starts_at_its_nearest_node() {
        let units = vec![
            closed(rect(50_000, 0, 10_000, 10_000), 0),
            closed(rect(10_000, 0, 10_000, 10_000), 1),
        ];
        let (paths, exit) = order(&units, PointUm::new(0, 20_000), true);
        assert_eq!(firsts(&paths), [1, 0]);
        // The first square starts at its bottom-left node (nearest to the head), drawn direction kept.
        assert_eq!(paths[0].points[0], PointUm::new(10_000, 10_000));
        assert_eq!(paths[0].points[1], PointUm::new(10_000, 0));
        assert_eq!(
            paths[0].points.last(),
            paths[0].points.first(),
            "closed returns to its start"
        );
        assert_eq!(exit, paths[1].points[0]);
    }

    #[test]
    fn an_open_path_runs_from_its_nearer_end() {
        let units = vec![Unit {
            shape: UnitShape::Path {
                points: vec![PointUm::new(0, 0), PointUm::new(10_000, 0)],
                closed: false,
            },
            stacking: 0,
            outline: 0,
        }];
        let (paths, exit) = order(&units, PointUm::new(12_000, 0), true);
        assert_eq!(
            paths[0].points,
            [PointUm::new(10_000, 0), PointUm::new(0, 0)]
        );
        assert_eq!(exit, PointUm::new(0, 0));
    }

    #[test]
    fn an_open_path_inside_a_closed_one_runs_first() {
        let units = vec![
            closed(rect(0, 0, 100_000, 100_000), 0),
            Unit {
                shape: UnitShape::Path {
                    points: vec![PointUm::new(40_000, 50_000), PointUm::new(60_000, 50_000)],
                    closed: false,
                },
                stacking: 1,
                outline: 0,
            },
        ];
        let (paths, _) = order(&units, PointUm::new(0, 0), true);
        assert_eq!(firsts(&paths), [1, 0]);
    }

    #[test]
    fn ties_go_to_the_lower_stacking_order() {
        let units = vec![
            closed(rect(10_000, 0, 10_000, 10_000), 5),
            closed(rect(-20_000, 0, 10_000, 10_000), 2),
        ];
        // Both nearest nodes lie 10 mm from the head.
        let (paths, _) = order(&units, PointUm::new(0, 0), true);
        assert_eq!(firsts(&paths), [1, 0]);
    }

    #[test]
    fn a_hatch_is_entered_at_its_nearest_corner_and_alternates() {
        let lines: HatchLines = (0..3)
            .map(|k| vec![(PointUm::new(0, k * 1_000), PointUm::new(10_000, k * 1_000))])
            .collect();
        let units = vec![Unit {
            shape: UnitShape::Hatch(vec![lines]),
            stacking: 0,
            outline: 0,
        }];
        let (paths, exit) = order(&units, PointUm::new(11_000, 3_000), true);
        let spans: Vec<(PointUm, PointUm)> =
            paths.iter().map(|p| (p.points[0], p.points[1])).collect();
        assert_eq!(
            spans,
            [
                (PointUm::new(10_000, 2_000), PointUm::new(0, 2_000)),
                (PointUm::new(0, 1_000), PointUm::new(10_000, 1_000)),
                (PointUm::new(10_000, 0), PointUm::new(0, 0)),
            ]
        );
        assert_eq!(exit, PointUm::new(0, 0));
        let (one_way, _) = order(&units, PointUm::new(0, 0), false);
        assert!(
            one_way.iter().all(|p| p.points[0].x == 0),
            "every line the same way"
        );
    }

    #[test]
    fn every_closed_path_runs_after_all_it_contains_on_random_fixtures() {
        // A small deterministic generator: nested and side-by-side squares.
        let mut seed: u64 = 7;
        let mut next = |m: i64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            i64::try_from((seed >> 33) % u64::try_from(m).unwrap()).unwrap()
        };
        for _ in 0..20 {
            let mut units = Vec::new();
            for i in 0..12 {
                let (x, y) = (next(200) * 1_000, next(200) * 1_000);
                let side = (1 + next(60)) * 1_000;
                units.push(closed(rect(x, y, side, side), i));
            }
            let (paths, _) = order(&units, PointUm::new(0, 0), true);
            assert_eq!(paths.len(), units.len());
            let parents = parents(&units);
            let at: Vec<usize> = (0..units.len())
                .map(|u| paths.iter().position(|p| p.unit == u).unwrap())
                .collect();
            for (child, parent) in parents.iter().enumerate() {
                if let Some(parent) = parent {
                    assert!(
                        at[child] < at[*parent],
                        "child {child} before parent {parent}"
                    );
                }
            }
            let (again, _) = order(&units, PointUm::new(0, 0), true);
            assert_eq!(paths, again, "deterministic");
        }
    }
}
