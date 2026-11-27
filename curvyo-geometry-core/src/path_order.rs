//! The run order of flattened machine paths (ADR 0064 §5, `0029` criterion 22, `0159` `adrs.md`
//! PL4): a path runs before every closed path that contains it, then the nearest start to the
//! head goes next, ties by rank, start index and unit index.

use crate::path_nesting::{NestPath, path_parents};
use crate::point_um::PointUm;

/// One unit to order: where it may be entered, its tie-breaking rank and its parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderUnit {
    /// The points it may be entered at, in tie order; a unit without one never runs and holds
    /// its parent back.
    pub starts: Vec<PointUm>,
    /// The rank that breaks a distance tie (the laser's stacking order and outline index).
    pub rank: (u32, u32),
    /// The innermost closed unit that contains it (see [`path_parents`]).
    pub parent: Option<usize>,
    /// Already run (or run by nothing): never chosen, and it holds no parent back.
    pub done: bool,
}

/// Runs `units` in nearest order from `head`: of the units not done whose children have all
/// run, the one whose start is nearest goes next; the key is the squared distance, the rank, the
/// start's index, the unit's index. `run(unit, start)` emits the unit entered at its start
/// `start` and returns where the head ends (`None` keeps it). Returns where the head stands at
/// the end.
pub fn nearest_order(
    units: &[OrderUnit],
    head: PointUm,
    mut run: impl FnMut(usize, usize) -> Option<PointUm>,
) -> PointUm {
    let mut pending = vec![0_usize; units.len()];
    for unit in units.iter().filter(|unit| !unit.done) {
        if let Some(parent) = unit.parent {
            pending[parent] += 1;
        }
    }
    let mut done: Vec<bool> = units.iter().map(|unit| unit.done).collect();
    let mut head = head;
    for _ in 0..units.len() {
        let mut best: Option<(i128, (u32, u32), usize, usize)> = None;
        for (index, unit) in units.iter().enumerate() {
            if done[index] || pending[index] > 0 {
                continue;
            }
            for (tie, &start) in unit.starts.iter().enumerate() {
                let key = (head.distance_squared(start), unit.rank, tie, index);
                if best.is_none_or(|b| key < b) {
                    best = Some(key);
                }
            }
        }
        let Some((_, _, tie, index)) = best else {
            break;
        };
        done[index] = true;
        if let Some(parent) = units[index].parent {
            pending[parent] -= 1;
        }
        head = run(index, tie).unwrap_or(head);
    }
    head
}

/// How a path is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathEntry {
    /// A closed path from node `n`, in its drawn direction.
    Closed(usize),
    /// An open path, reversed when its end was nearer.
    Open {
        /// Whether it runs from its last point.
        reversed: bool,
    },
}

/// The starts a path may be entered at, in tie order: every node of a closed path, the first
/// then the last point of an open one.
#[must_use]
pub fn path_starts(points: &[PointUm], closed: bool) -> Vec<(PointUm, PathEntry)> {
    if closed {
        return points
            .iter()
            .enumerate()
            .map(|(n, &p)| (p, PathEntry::Closed(n)))
            .collect();
    }
    match (points.first(), points.last()) {
        (Some(&first), Some(&last)) => vec![
            (first, PathEntry::Open { reversed: false }),
            (last, PathEntry::Open { reversed: true }),
        ],
        _ => Vec::new(),
    }
}

/// The points of a path entered by `entry`, in run order; a closed path ends on its start.
#[must_use]
pub fn path_run(points: &[PointUm], entry: PathEntry) -> Vec<PointUm> {
    match entry {
        PathEntry::Closed(n) => {
            let mut path: Vec<PointUm> = points[n..].iter().chain(&points[..n]).copied().collect();
            path.push(points[n]);
            path
        }
        PathEntry::Open { reversed } => {
            let mut path = points.to_vec();
            if reversed {
                path.reverse();
            }
            path
        }
    }
}

/// One path to order with [`order_paths`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPath {
    /// The points in µm; a closed path does not repeat its first point.
    pub points: Vec<PointUm>,
    /// Whether it is closed.
    pub closed: bool,
    /// The rank that breaks a distance tie.
    pub rank: (u32, u32),
}

/// One ordered path: its index in the input and its points in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedRun {
    /// The path's index in the input.
    pub index: usize,
    /// The points in run order; a closed path ends on its first point.
    pub points: Vec<PointUm>,
}

/// `paths` in run order from `head` (inner first, then nearest start), with where the head ends.
#[must_use]
pub fn order_paths(paths: &[OrderPath], head: PointUm) -> (Vec<OrderedRun>, PointUm) {
    let nest: Vec<NestPath<'_>> = paths
        .iter()
        .map(|p| Some((p.points.as_slice(), p.closed)))
        .collect();
    let parents = path_parents(&nest);
    let entries: Vec<Vec<(PointUm, PathEntry)>> = paths
        .iter()
        .map(|p| path_starts(&p.points, p.closed))
        .collect();
    let units: Vec<OrderUnit> = paths
        .iter()
        .zip(&entries)
        .zip(parents)
        .map(|((path, starts), parent)| OrderUnit {
            starts: starts.iter().map(|s| s.0).collect(),
            rank: path.rank,
            parent,
            done: false,
        })
        .collect();
    let mut out = Vec::new();
    let exit = nearest_order(&units, head, |index, tie| {
        let points = path_run(&paths[index].points, entries[index][tie].1);
        let end = points.last().copied();
        out.push(OrderedRun { index, points });
        end
    });
    (out, exit)
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

    fn closed(points: Vec<PointUm>) -> OrderPath {
        OrderPath {
            points,
            closed: true,
            rank: (0, 0),
        }
    }

    #[test]
    fn inner_paths_run_before_their_outlines_then_the_nearest_start_goes_next() {
        let paths = vec![
            closed(rect(0, 0, 100_000, 100_000)),
            closed(rect(20_000, 20_000, 10_000, 10_000)),
            closed(rect(200_000, 0, 10_000, 10_000)),
        ];
        let (runs, exit) = order_paths(&paths, PointUm::new(0, 0));
        let order: Vec<usize> = runs.iter().map(|r| r.index).collect();
        assert_eq!(order, [1, 0, 2]);
        assert_eq!(runs[0].points[0], PointUm::new(20_000, 20_000));
        assert_eq!(runs[0].points.first(), runs[0].points.last());
        assert_eq!(exit, PointUm::new(200_000, 0));
    }

    #[test]
    fn an_open_path_runs_from_its_nearer_end() {
        let paths = vec![OrderPath {
            points: vec![PointUm::new(10_000, 0), PointUm::new(0, 0)],
            closed: false,
            rank: (0, 0),
        }];
        let (runs, exit) = order_paths(&paths, PointUm::new(0, 0));
        assert_eq!(
            runs[0].points,
            [PointUm::new(0, 0), PointUm::new(10_000, 0)]
        );
        assert_eq!(exit, PointUm::new(10_000, 0));
    }
}
