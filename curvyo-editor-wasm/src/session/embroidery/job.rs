//! The stitch plan of the active embroidery page (`specs/0160-embroidery` criteria 4, 5, 9, 10,
//! `adrs.md` EM6, EM7): each operation's items from the output page's leaf index in page mm with
//! their stroke widths, the embroidery-core plan built from them, and the items outside the hoop.

use curvyo_document_core::NodeId;
use curvyo_embroidery_core::{ItemOutline, Mm, PlanItem, PlanOperation, StitchPlan, build_plan};
use curvyo_laser_core::job::LeafGeometry;

use super::super::Session;
use super::EmbroideryPage;

/// The plan of an embroidery page and what its checks read.
pub(in crate::session) struct EmbroideryJob {
    /// How many stored numbers of its operations read as their default (criterion 20).
    pub(in crate::session) damaged: usize,
    /// The stitch plan.
    pub(in crate::session) plan: StitchPlan,
    /// For each operation of the plan, its index in the page's list.
    pub(in crate::session) list_index: Vec<usize>,
    /// How many items each operation of the list holds.
    pub(in crate::session) counts: Vec<usize>,
    /// Per list operation: the nodes of its items with a stitch outside the hoop (criterion 4).
    pub(in crate::session) outside: Vec<Vec<NodeId>>,
    /// Per list operation: how many items ran as running stitch for a too narrow satin.
    pub(in crate::session) narrow: Vec<usize>,
    /// Per list operation: how many items had no closed area to fill.
    pub(in crate::session) no_area: Vec<usize>,
}

impl EmbroideryJob {
    /// Whether nothing is sewn.
    pub(in crate::session) fn is_empty(&self) -> bool {
        self.plan.blocks.is_empty()
    }
}

/// A leaf's outlines as embroidery reads them: a path's own outlines, nothing of an image.
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
    /// The items of `op` among `leaves` with their nodes: each leaf whose key chain one of its
    /// members names, with its outlines and stroke width.
    fn embroidery_items(
        &self,
        op: &curvyo_document_core::OperationSnapshot,
        leaves: &[super::super::operations::index::IndexedLeaf],
        geometry: &[Option<LeafGeometry>],
    ) -> (Vec<PlanItem>, Vec<Option<NodeId>>) {
        let mut items = Vec::new();
        let mut nodes = Vec::new();
        for (stacking, (leaf, geometry)) in leaves.iter().zip(geometry.iter()).enumerate() {
            if !leaf.chain.iter().any(|key| op.members.contains(key)) {
                continue;
            }
            let Some(outlines) = geometry.as_ref().map(outlines_of) else {
                continue;
            };
            if outlines.is_empty() {
                continue;
            }
            let node = super::super::output::select_id(leaf);
            let width = node
                .and_then(|id| self.document.object(id))
                .map_or(0.0, |o| {
                    let stroke = &o.style().stroke;
                    if stroke.enabled {
                        stroke.width.as_mm()
                    } else {
                        0.0
                    }
                });
            items.push(PlanItem {
                outlines,
                stroke_width: Mm::saturating(width),
                stacking: u32::try_from(stacking).unwrap_or(u32::MAX),
            });
            nodes.push(node);
        }
        (items, nodes)
    }

    /// The plan of `page`: its operations with Output on in list order, each with the leaves
    /// whose key chain one of its members names.
    pub(in crate::session) fn embroidery_job(&self, page: &EmbroideryPage) -> EmbroideryJob {
        let n = page.operations.len();
        let mut job = EmbroideryJob {
            damaged: 0,
            plan: StitchPlan::default(),
            list_index: Vec::new(),
            counts: vec![0; n],
            outside: vec![Vec::new(); n],
            narrow: vec![0; n],
            no_area: vec![0; n],
        };
        let Some(manufacturing) = self.document.manufacturing(page.id) else {
            return job;
        };
        let leaves = self.output_leaves(&manufacturing);
        let geometry = self.leaf_geometries(&manufacturing, &leaves);
        let mut operations = Vec::new();
        let mut nodes: Vec<Vec<Option<NodeId>>> = Vec::new();
        for (list, op) in page.operations.iter().enumerate() {
            let Some(kind) = EmbroideryPage::kind_of(op) else {
                continue;
            };
            let (items, item_nodes) = self.embroidery_items(op, &leaves, &geometry);
            job.counts[list] = items.len();
            if !op.output {
                continue;
            }
            let (settings, thread, bad) = EmbroideryPage::settings_of(kind, op);
            job.damaged += bad;
            operations.push(PlanOperation {
                kind,
                settings,
                thread,
                items,
            });
            nodes.push(item_nodes);
            job.list_index.push(list);
        }
        job.plan = build_plan(&operations, page.page_centre(), page.copy.values.max_stitch);
        for (plan_op, &list) in job.list_index.iter().enumerate() {
            job.narrow[list] = job.plan.narrow[plan_op];
            job.no_area[list] = job.plan.no_area[plan_op];
        }
        for block in &job.plan.blocks {
            let outside = job.plan.stitches[block.stitches.clone()]
                .iter()
                .any(|s| page.hoop.size.is_outside(s));
            let list = job.list_index[block.operation];
            if let Some(node) = nodes[block.operation][block.item]
                && outside
                && !job.outside[list].contains(&node)
            {
                job.outside[list].push(node);
            }
        }
        job
    }
}
