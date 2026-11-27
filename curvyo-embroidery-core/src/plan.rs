//! The stitch plan of an embroidery page (`specs/0160-embroidery` criteria 7 to 10, 12, `adrs.md`
//! EM2, EM3, EM6, EM10): each item's needle points from the port, tied in and off, placed from
//! the hoop centre and rounded once, the items of an operation in nearest-next order, and the one
//! stitch list the Toolpath view, the summary and the DST all read.

use std::ops::Range;

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{OrderUnit, OutlineTriple, PointUm, flatten_outline, nearest_order};
use curvyo_inkstitch_core::{
    Mm, RUNNING_TOLERANCE, repeat_passes, running_stitch, satin_zigzag, tatami_fill, tie_in,
    tie_off,
};

use crate::emit::Emitter;
use crate::operation::{EmbroiderySettings, OpKind, ThreadColour};
use crate::stitch::{Stitch, StitchPoint, TenthMm};

/// The flattening tolerance: a needle point on a flattened line stays within 0.01 mm of the
/// curve before rounding (as the plotter's).
pub const FLATTEN_MM: f64 = 0.0095;
/// A satin narrower than this runs as running stitch (criterion 9, Proposal).
pub const NARROW_SATIN_MM: f64 = 0.5;

/// One outline of an item in page mm, Y down.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemOutline {
    /// The anchors with their handles.
    pub anchors: Vec<OutlineTriple>,
    /// Whether it is closed.
    pub closed: bool,
}

/// One item of an operation.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanItem {
    /// Its outlines.
    pub outlines: Vec<ItemOutline>,
    /// Its stroke width: the satin's column width.
    pub stroke_width: Mm,
    /// Its stacking order, the nearest order's tie-breaker.
    pub stacking: u32,
}

/// One operation of the page, in list order.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanOperation {
    /// Its kind.
    pub kind: OpKind,
    /// Its numbers.
    pub settings: EmbroiderySettings,
    /// Its thread.
    pub thread: ThreadColour,
    /// Its items.
    pub items: Vec<PlanItem>,
}

/// One run of needle points without a jump: a line, an outline, a fill section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The operation's index.
    pub operation: usize,
    /// The item's index in its operation.
    pub item: usize,
    /// The thread.
    pub thread: ThreadColour,
    /// Its stitches in the list, from its first needle point to its tie-off's last.
    pub stitches: Range<usize>,
}

/// The plan of a page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StitchPlan {
    /// The stitch list, ending with an end record (empty when nothing is sewn).
    pub stitches: Vec<Stitch>,
    /// The blocks in sewing order.
    pub blocks: Vec<Block>,
    /// Per operation: how many of its items were too narrow for satin.
    pub narrow: Vec<usize>,
    /// Per operation: how many of its items had no closed area to fill.
    pub no_area: Vec<usize>,
}

/// Maps page mm (Y down) to the DST frame: from `centre`, Y up, rounded once (EM3, EM10).
fn to_stitch(centre: Point, p: kurbo::Point) -> StitchPoint {
    StitchPoint {
        x: TenthMm::of_mm(p.x - centre.x),
        y: TenthMm::of_mm(centre.y - p.y),
    }
}

/// The outline flattened in page mm; a closed one ends on its start.
fn polyline(outline: &ItemOutline) -> Vec<kurbo::Point> {
    let mut points: Vec<kurbo::Point> = flatten_outline(
        &outline.anchors,
        outline.closed,
        Tolerance::from_mm(FLATTEN_MM),
    )
    .into_iter()
    .map(|p| kurbo::Point::new(p.x, p.y))
    .collect();
    if outline.closed && points.len() > 1 && points.first() != points.last() {
        points.push(points[0]);
    }
    points
}

/// The runs of one item in page mm, without ties, and whether it was narrow or had no area.
fn item_runs(
    kind: OpKind,
    s: EmbroiderySettings,
    item: &PlanItem,
) -> (Vec<Vec<kurbo::Point>>, bool, bool) {
    let lines = || item.outlines.iter().map(polyline).filter(|l| l.len() >= 2);
    match kind {
        OpKind::Running => {
            let runs = lines()
                .map(|l| {
                    repeat_passes(
                        &running_stitch(&l, s.stitch_length(), RUNNING_TOLERANCE),
                        s.repeats(),
                    )
                })
                .collect();
            (runs, false, false)
        }
        OpKind::Satin if item.stroke_width.get() < NARROW_SATIN_MM => {
            let length = EmbroiderySettings::default().stitch_length();
            let runs = lines()
                .map(|l| running_stitch(&l, length, RUNNING_TOLERANCE))
                .collect();
            (runs, true, false)
        }
        OpKind::Satin => {
            let runs = lines()
                .map(|l| satin_zigzag(&l, item.stroke_width, s.zigzag_spacing()))
                .collect();
            (runs, false, false)
        }
        OpKind::Tatami => {
            let rings: Vec<Vec<kurbo::Point>> = item
                .outlines
                .iter()
                .filter(|o| o.closed)
                .map(|o| {
                    let mut ring = polyline(o);
                    ring.pop();
                    ring
                })
                .filter(|r| r.len() >= 3)
                .collect();
            if rings.is_empty() {
                return (Vec::new(), false, true);
            }
            (
                tatami_fill(&rings, s.angle(), s.row_spacing(), s.max_stitch()),
                false,
                false,
            )
        }
    }
}

/// A run tied in and off (criterion 10) and placed in the DST frame.
fn tied(run: &[kurbo::Point], centre: Point) -> Vec<StitchPoint> {
    let mut points = tie_in(run);
    points.extend_from_slice(run);
    points.extend(tie_off(run));
    let mut out: Vec<StitchPoint> = Vec::with_capacity(points.len());
    for p in points {
        let at = to_stitch(centre, p);
        if out.last() != Some(&at) {
            out.push(at);
        }
    }
    out
}

fn um(p: StitchPoint) -> PointUm {
    PointUm::new(i64::from(p.x.get()) * 100, i64::from(p.y.get()) * 100)
}

/// The plan of `operations` (in list order) on a page whose hoop centre is `centre` (page mm):
/// operations in order, the items of each nearest-next from where the last one ended, each run a
/// block with a tie-in and a tie-off, no stitch longer than `max_stitch`, and an end.
#[must_use]
pub fn build_plan(operations: &[PlanOperation], centre: Point, max_stitch: TenthMm) -> StitchPlan {
    let mut plan = StitchPlan::default();
    let mut emitter = Emitter::new(max_stitch);
    let mut head = StitchPoint::ORIGIN;
    for (op_index, op) in operations.iter().enumerate() {
        let mut narrow = 0;
        let mut no_area = 0;
        let items: Vec<Vec<Vec<StitchPoint>>> = op
            .items
            .iter()
            .map(|item| {
                let (runs, is_narrow, is_empty) = item_runs(op.kind, op.settings, item);
                narrow += usize::from(is_narrow);
                no_area += usize::from(is_empty);
                runs.iter()
                    .filter(|r| !r.is_empty())
                    .map(|r| tied(r, centre))
                    .collect()
            })
            .collect();
        plan.narrow.push(narrow);
        plan.no_area.push(no_area);
        let units: Vec<OrderUnit> = items
            .iter()
            .zip(&op.items)
            .map(|(runs, item)| OrderUnit {
                starts: runs
                    .first()
                    .and_then(|r| r.first())
                    .map(|&p| um(p))
                    .into_iter()
                    .collect(),
                rank: (item.stacking, 0),
                parent: None,
                done: runs.is_empty(),
            })
            .collect();
        nearest_order(&units, um(head), |index, _| {
            for run in &items[index] {
                emitter.enter(run[0], op.thread);
                let start = emitter.len() - 1;
                emitter.sew(&run[1..]);
                plan.blocks.push(Block {
                    operation: op_index,
                    item: index,
                    thread: op.thread,
                    stitches: start..emitter.len(),
                });
                head = *run.last().unwrap_or(&head);
            }
            Some(um(head))
        });
    }
    plan.stitches = emitter.finish();
    plan
}
