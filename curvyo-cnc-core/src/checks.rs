//! The checks that block a CNC Export (`specs/0161-cnc-output` criteria 4, 7 and 10, `adrs.md`
//! CN8): items outside the stock, a toolpath outside the work area and more than one tool.

use curvyo_document_core::{DocumentSize, Tolerance};
use curvyo_geometry_core::flatten_outline;

use crate::job::{CncJob, JobOperation};
use crate::profile_toolpath::{FLATTEN_TOLERANCE_MM, ItemOutline};
use crate::stock::Stock;
use crate::work::work_to_page;

/// The distance a point may lie beyond an edge and still count as inside, in mm.
pub const EDGE_TOLERANCE_MM: f64 = 0.01;

fn inside(x: f64, y: f64, width: f64, height: f64) -> bool {
    let t = EDGE_TOLERANCE_MM;
    (-t..=width + t).contains(&x) && (-t..=height + t).contains(&y)
}

/// Whether an item of `outlines` (page mm) has a point outside `stock` (criterion 10).
#[must_use]
pub fn outside_stock(outlines: &[ItemOutline], stock: &Stock) -> bool {
    let (w, h) = (stock.width.as_mm(), stock.height.as_mm());
    outlines.iter().any(|o| {
        flatten_outline(
            &o.anchors,
            o.closed,
            Tolerance::from_mm(FLATTEN_TOLERANCE_MM),
        )
        .iter()
        .any(|p| !inside(p.x, p.y, w, h))
    })
}

/// The operations of `job` with a move outside the page's `work_area` (criterion 10).
#[must_use]
pub fn outside_work_area(job: &CncJob, stock: &Stock, work_area: DocumentSize) -> Vec<usize> {
    let (w, h) = (work_area.width.as_mm(), work_area.height.as_mm());
    let mut out: Vec<usize> = Vec::new();
    for m in &job.moves {
        let p = work_to_page(m.to, stock.height.as_mm());
        if !inside(p.x, p.y, w, h) && !out.contains(&m.operation) {
            out.push(m.operation);
        }
    }
    out
}

/// The operations whose tool differs from the first operation's (criterion 4: one tool per
/// job for now).
#[must_use]
pub fn other_tools(operations: &[JobOperation]) -> Vec<usize> {
    let Some(first) = operations.first() else {
        return Vec::new();
    };
    (0..operations.len())
        .filter(|&i| operations[i].tool.id != first.tool.id)
        .collect()
}
