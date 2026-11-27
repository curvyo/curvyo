//! The job of the active CNC page (`specs/0161-cnc-output` criteria 4 to 12, 18, `adrs.md` CN5,
//! CN8): each Profile operation's items from the output page's leaf index, the cnc-core moves and
//! G-code built from them, the planner's time, and the checks that block Export.

use curvyo_cnc_core::{
    CncJob, CncValues, GrblHeader, ItemOutline, JobOperation, build_job, other_tools,
    outside_stock, outside_work_area, write_grbl_mapped,
};
use curvyo_connector_core::virtual_grbl::{GrblSettings, PlanTime, plan_time};
use curvyo_document_core::NodeId;
use curvyo_laser_core::job::LeafGeometry;

use super::super::Session;
use super::CncPage;

/// The job of a CNC page and what its checks read.
pub(in crate::session) struct CncJobView {
    /// How many stored numbers of its operations read as their default.
    pub(in crate::session) damaged: usize,
    /// The moves.
    pub(in crate::session) job: CncJob,
    /// The G-code Export writes and Send sends.
    pub(in crate::session) gcode: String,
    /// The planner's time of the G-code (criterion 18).
    pub(in crate::session) time: PlanTime,
    /// When each move of the job ends, in seconds of the planner's time.
    pub(in crate::session) move_end: Vec<f64>,
    /// For each operation of the job, its index in the page's list.
    pub(in crate::session) list_index: Vec<usize>,
    /// How many items each operation of the list holds.
    pub(in crate::session) counts: Vec<usize>,
    /// The items outside the stock (criterion 10).
    pub(in crate::session) outside_stock: Vec<NodeId>,
    /// List operations whose toolpath leaves the work area (criterion 10).
    pub(in crate::session) outside_area: Vec<usize>,
    /// List operations with another tool than the first (criterion 4).
    pub(in crate::session) other_tools: Vec<usize>,
}

impl CncJobView {
    /// Whether nothing is milled.
    pub(in crate::session) fn is_empty(&self) -> bool {
        self.job.is_empty()
    }

    /// Whether a check blocks Export.
    pub(in crate::session) fn blocked(&self) -> bool {
        self.is_empty()
            || !self.outside_stock.is_empty()
            || !self.outside_area.is_empty()
            || !self.other_tools.is_empty()
            || self.job.reports.iter().any(|r| r.too_narrow > 0)
    }
}

/// The `$` settings of a machine of `values`: the GRBL defaults with its rates, accelerations,
/// spindle and travel (the planner's machine, CN5).
pub(in crate::session) fn grbl_settings(values: &CncValues) -> GrblSettings {
    let mut s = GrblSettings::default();
    let travel = [
        values.work_area.width.as_mm(),
        values.work_area.height.as_mm(),
        values.z_travel.as_mm(),
    ];
    for (n, ((rate, accel), travel)) in
        (0_u16..).zip(values.max_rate.iter().zip(&values.acceleration).zip(travel))
    {
        let _ = s.set(110 + n, &rate.get().to_string());
        let _ = s.set(120 + n, &accel.get().to_string());
        let _ = s.set(130 + n, &travel.to_string());
    }
    let _ = s.set(30, &values.spindle_max.get().to_string());
    s
}

/// A leaf's outlines: a path's own outlines, nothing of an image.
fn outlines_of(geometry: &LeafGeometry) -> Vec<ItemOutline> {
    match geometry {
        LeafGeometry::Outlines(outlines) | LeafGeometry::Gradient { outlines, .. } => outlines
            .iter()
            .map(|o| ItemOutline {
                anchors: o.anchors.clone(),
                closed: o.closed,
            })
            .collect(),
        LeafGeometry::Image { .. } => Vec::new(),
    }
}

impl Session {
    /// The items of `op` among the page's leaves with their nodes.
    fn cnc_items(
        op: &curvyo_document_core::OperationSnapshot,
        leaves: &[super::super::operations::index::IndexedLeaf],
        geometry: &[Option<LeafGeometry>],
    ) -> (Vec<Vec<ItemOutline>>, Vec<Option<NodeId>>) {
        let mut items = Vec::new();
        let mut nodes = Vec::new();
        for (leaf, geometry) in leaves.iter().zip(geometry) {
            if !leaf.chain.iter().any(|key| op.members.contains(key)) {
                continue;
            }
            let outlines = geometry.as_ref().map(outlines_of).unwrap_or_default();
            if outlines.is_empty() {
                continue;
            }
            items.push(outlines);
            nodes.push(super::super::output::select_id(leaf));
        }
        (items, nodes)
    }

    /// The job of `page`: its Profile operations with Output on in list order.
    pub(in crate::session) fn cnc_job(&self, page: &CncPage) -> CncJobView {
        let n = page.operations.len();
        let mut counts = vec![0; n];
        let mut operations = Vec::new();
        let mut list_index = Vec::new();
        let mut outside = Vec::new();
        let mut damaged = 0;
        if let Some(manufacturing) = self.document.manufacturing(page.id) {
            let leaves = self.output_leaves(&manufacturing);
            let geometry = self.leaf_geometries(&manufacturing, &leaves);
            for (list, op) in page.operations.iter().enumerate() {
                if !CncPage::is_profile(op) {
                    continue;
                }
                let (items, nodes) = Self::cnc_items(op, &leaves, &geometry);
                counts[list] = items.len();
                if !op.output {
                    continue;
                }
                for (item, node) in items.iter().zip(&nodes) {
                    if let Some(node) = node
                        && outside_stock(item, &page.stock)
                        && !outside.contains(node)
                    {
                        outside.push(*node);
                    }
                }
                let (settings, bad) = page.settings_of(op);
                damaged += bad;
                let tool = page.tool(&settings.tool).clone();
                operations.push(JobOperation {
                    settings,
                    tool,
                    items,
                });
                list_index.push(list);
            }
        }
        let values = &page.copy.values;
        let job = build_job(&operations, &page.stock, values.safe_height);
        let tool = operations.first().map_or(&page.tools[0], |op| &op.tool);
        let header = GrblHeader {
            page: &page.page.name,
            tool,
            stock: &page.stock,
            safe: values.safe_height,
        };
        let (gcode, lines) = write_grbl_mapped(&job, &header, tool.rpm);
        let time = plan_time(&gcode, &grbl_settings(values));
        let move_end = lines
            .iter()
            .map(|&l| time.line_end.get(l).map_or(0.0, |t| t.get()))
            .collect();
        let to_list = |ops: Vec<usize>| ops.into_iter().map(|i| list_index[i]).collect();
        CncJobView {
            damaged,
            outside_area: to_list(outside_work_area(&job, &page.stock, values.work_area)),
            other_tools: to_list(other_tools(&operations)),
            job,
            gcode,
            time,
            move_end,
            list_index,
            counts,
            outside_stock: outside,
        }
    }
}
