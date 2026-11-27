//! The moves of a CNC job (`specs/0161-cnc-output` criteria 8 and 9, `adrs.md` CN3): each Profile
//! operation's tool loops in list order, holes before their outline then nearest-next, each loop
//! in its passes at equal depth steps, in work coordinates.

use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{OrderPath, PointUm, order_paths, signed_area_mm2};

use crate::operation::{Direction, ProfileSettings, Side};
use crate::profile_toolpath::{ItemLoops, ItemOutline, ToolLoop, profile_loops};
use crate::stock::Stock;
use crate::tools::Tool;
use crate::units::FeedMmMin;
use crate::work::{WorkPoint, page_to_work, work_to_page};

/// One Profile operation to run.
#[derive(Debug, Clone, PartialEq)]
pub struct JobOperation {
    /// Its numbers.
    pub settings: ProfileSettings,
    /// Its tool.
    pub tool: Tool,
    /// Its items' outlines in page mm, in stacking order.
    pub items: Vec<Vec<ItemOutline>>,
}

/// What a move is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// `G0` at the machine's rates.
    Rapid,
    /// `G1` down into the stock at the plunge feed.
    Plunge,
    /// `G1` along the loop at the cutting feed.
    Cut,
}

/// One move to a work point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    /// What it is.
    pub kind: MoveKind,
    /// Where it ends.
    pub to: WorkPoint,
    /// The feed of a `G1`; `None` for a rapid.
    pub feed: Option<FeedMmMin>,
    /// The operation's index in the job's list.
    pub operation: usize,
    /// The pass it belongs to, from 1; 0 for a rapid between passes.
    pub pass: u32,
}

/// What one operation made of its items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OperationReport {
    /// Items with an open outline, not milled.
    pub open: usize,
    /// Items too narrow for the tool.
    pub too_narrow: usize,
    /// Loops milled.
    pub loops: usize,
    /// Passes per loop.
    pub passes: u32,
}

/// The moves of a job and what each operation made of its items.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CncJob {
    /// The moves from the start, the head at work zero.
    pub moves: Vec<Move>,
    /// Per operation of the input.
    pub reports: Vec<OperationReport>,
}

impl CncJob {
    /// Whether nothing is milled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.moves.iter().any(|m| m.kind == MoveKind::Cut)
    }

    /// The cutting (plunges included) and the rapid lengths in mm.
    #[must_use]
    pub fn lengths(&self) -> (f64, f64) {
        let mut from = WorkPoint::mm(0.0, 0.0, 0.0);
        let (mut cut, mut rapid) = (0.0, 0.0);
        for m in &self.moves {
            let d = from.distance(m.to);
            if m.kind == MoveKind::Rapid {
                rapid += d;
            } else {
                cut += d;
            }
            from = m.to;
        }
        (cut, rapid)
    }
}

/// Whether a loop should run with positive area in work coordinates: the region-positive
/// orientation (shape on its left) is conventional milling outside and climb milling inside, as
/// the finished wall is on the tool's right in climb milling with the spindle clockwise.
fn work_positive(side: Side, direction: Direction, outer: bool) -> bool {
    let region_positive = (side == Side::Outside) == (direction == Direction::Conventional);
    region_positive == outer
}

/// `l` in the page direction that gives the wanted work direction (the Y flip mirrors it).
fn directed(l: &ToolLoop, side: Side, direction: Direction) -> Vec<Point> {
    let page_positive = !work_positive(side, direction, l.outer);
    let mut points = l.points.clone();
    if (signed_area_mm2(&points) > 0.0) != page_positive {
        points.reverse();
    }
    points
}

/// The depth of pass `k` of `n` to `depth_mm`, the last exactly at the depth.
fn pass_depth(depth_mm: f64, k: u32, n: u32) -> f64 {
    if k == n {
        depth_mm
    } else {
        depth_mm * f64::from(k) / f64::from(n)
    }
}

/// Builds the moves of `operations` on `stock` with rapids at `safe`.
#[must_use]
pub fn build_job(operations: &[JobOperation], stock: &Stock, safe: Length) -> CncJob {
    let h = stock.height.as_mm();
    let safe = safe.as_mm();
    let mut job = CncJob::default();
    job.moves.push(Move {
        kind: MoveKind::Rapid,
        to: WorkPoint::mm(0.0, 0.0, safe),
        feed: None,
        operation: 0,
        pass: 0,
    });
    let mut head = PointUm::of_mm(work_to_page(WorkPoint::mm(0.0, 0.0, 0.0), h));
    for (index, op) in operations.iter().enumerate() {
        let s = &op.settings;
        let mut report = OperationReport {
            passes: s.passes(&op.tool),
            ..OperationReport::default()
        };
        let mut paths = Vec::new();
        for (item, outlines) in op.items.iter().enumerate() {
            match profile_loops(outlines, s.side, op.tool.radius_mm()) {
                ItemLoops::OpenPath => report.open += 1,
                ItemLoops::TooNarrow => report.too_narrow += 1,
                ItemLoops::Loops(loops) => {
                    for l in &loops {
                        let points = directed(l, s.side, s.direction);
                        paths.push(OrderPath {
                            points: points.into_iter().map(PointUm::of_mm).collect(),
                            closed: true,
                            rank: (u32::try_from(item).unwrap_or(u32::MAX), 0),
                        });
                    }
                }
            }
        }
        report.loops = paths.len();
        let (runs, exit) = order_paths(&paths, head);
        head = exit;
        let feed = s.feed.unwrap_or(op.tool.feed);
        let plunge = s.plunge.unwrap_or(op.tool.plunge);
        for run in runs {
            let points: Vec<WorkPoint> = run
                .points
                .iter()
                .map(|p| page_to_work(p.to_mm(), 0.0, h))
                .collect();
            push_loop(
                &mut job.moves,
                &points,
                (index, report.passes),
                s,
                (feed, plunge),
                safe,
            );
        }
        job.reports.push(report);
    }
    job
}

/// The passes of one loop `points` (closed, ending on its start).
fn push_loop(
    moves: &mut Vec<Move>,
    points: &[WorkPoint],
    (operation, n): (usize, u32),
    s: &ProfileSettings,
    (feed, plunge): (FeedMmMin, FeedMmMin),
    safe: f64,
) {
    let Some(&start) = points.first() else {
        return;
    };
    let rapid = |to| Move {
        kind: MoveKind::Rapid,
        to,
        feed: None,
        operation,
        pass: 0,
    };
    for k in 1..=n {
        let z = -pass_depth(s.depth.get(), k, n);
        moves.push(rapid(start.at_z(safe)));
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: start.at_z(z),
            feed: Some(plunge),
            operation,
            pass: k,
        });
        for p in &points[1..] {
            moves.push(Move {
                kind: MoveKind::Cut,
                to: p.at_z(z),
                feed: Some(feed),
                operation,
                pass: k,
            });
        }
        let end = points[points.len() - 1];
        moves.push(rapid(end.at_z(safe)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_depths_are_equal_steps_ending_exactly_at_the_depth() {
        let depths: Vec<f64> = (1..=4).map(|k| pass_depth(10.0, k, 4)).collect();
        assert_eq!(depths, [2.5, 5.0, 7.5, 10.0]);
    }

    #[test]
    fn conventional_outside_runs_counter_clockwise_in_work_coordinates() {
        assert!(work_positive(Side::Outside, Direction::Conventional, true));
        assert!(!work_positive(Side::Outside, Direction::Climb, true));
        assert!(work_positive(Side::Inside, Direction::Climb, true));
        assert!(!work_positive(
            Side::Outside,
            Direction::Conventional,
            false
        ));
    }
}
