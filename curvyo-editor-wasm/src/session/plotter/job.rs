//! The job of the active plotter page (`specs/0159-cutting-plotter` criteria 6, 7, 9, 10, `adrs.md`
//! PL5, PL8, PL9): each operation's items from the output page's leaf index in page mm, the
//! cutter-core job built from them, and the items outside the cut area.

use curvyo_cutter_core::{
    CutterJob, ItemOutline, JobItem, JobOperation, build_job, outside_cut_area,
};
use curvyo_document_core::{NodeId, Point};
use curvyo_laser_core::job::{LeafGeometry, LeafOutline};

use super::super::Session;
use super::PlotterPage;

/// The job of a plotter page and what its checks read.
pub(in crate::session) struct PlotterJob {
    /// How many stored numbers of its operations read as their default (criterion 17).
    pub(in crate::session) damaged: usize,
    /// The built job.
    pub(in crate::session) job: CutterJob,
    /// The nodes of the items outside the cut area (criterion 6).
    pub(in crate::session) outside: Vec<NodeId>,
    /// How many items each operation holds, in list order.
    pub(in crate::session) counts: Vec<usize>,
}

impl Session {
    /// The job of `page`: its operations in list order, each with the leaves whose key chain one
    /// of its members names, both Cut and Draw following the leaf's own outlines (PL8).
    pub(in crate::session) fn plotter_job(&self, page: &PlotterPage) -> PlotterJob {
        let Some(manufacturing) = self.document.manufacturing(page.id) else {
            return PlotterJob {
                damaged: 0,
                job: CutterJob::default(),
                outside: Vec::new(),
                counts: Vec::new(),
            };
        };
        let leaves = self.output_leaves(&manufacturing);
        let geometry = self.leaf_geometries(&manufacturing, &leaves);
        let mut operations = Vec::new();
        let mut items: Vec<(NodeId, (Point, Point))> = Vec::new();
        let mut counts = Vec::new();
        let mut damaged = 0;
        for op in &page.operations {
            let Some(kind) = PlotterPage::kind_of(op) else {
                counts.push(0);
                continue;
            };
            let mut job_items = Vec::new();
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
                if let (Some(node), Some(bounds)) =
                    (super::super::output::select_id(leaf), bounds_of(&outlines))
                    && op.output
                    && !items.iter().any(|(n, _)| *n == node)
                {
                    items.push((node, bounds));
                }
                job_items.push(JobItem {
                    outlines,
                    stacking: u32::try_from(stacking).unwrap_or(u32::MAX),
                });
            }
            counts.push(job_items.len());
            if !op.output {
                continue;
            }
            let (settings, bad) = page.settings_of(kind, op);
            damaged += bad;
            operations.push(JobOperation {
                name: op.name.clone(),
                kind,
                settings,
                items: job_items,
            });
        }
        let area = page.cut_area();
        let bounds: Vec<(Point, Point)> = items.iter().map(|(_, b)| *b).collect();
        let outside = outside_cut_area(&bounds, area)
            .into_iter()
            .map(|index| items[index].0)
            .collect();
        PlotterJob {
            damaged,
            job: build_job(&operations),
            outside,
            counts,
        }
    }
}

/// A leaf's outlines as the plotter follows them: a path's own outlines, an image's frame.
fn outlines_of(geometry: &LeafGeometry) -> Vec<ItemOutline> {
    let convert = |o: &LeafOutline| ItemOutline {
        anchors: o.anchors.clone(),
        closed: o.closed,
    };
    match geometry {
        LeafGeometry::Outlines(outlines) | LeafGeometry::Gradient { outlines, .. } => {
            outlines.iter().map(convert).collect()
        }
        LeafGeometry::Image { frame, .. } => vec![ItemOutline {
            anchors: frame
                .iter()
                .map(|&p| {
                    (
                        p,
                        curvyo_document_core::Vec2::ZERO,
                        curvyo_document_core::Vec2::ZERO,
                    )
                })
                .collect(),
            closed: true,
        }],
    }
}

/// The bounds of `outlines` in page mm, from the curve flattened as the job flattens it.
fn bounds_of(outlines: &[ItemOutline]) -> Option<(Point, Point)> {
    let tolerance = curvyo_document_core::Tolerance::from_mm(curvyo_cutter_core::FLATTEN_MM);
    let mut points = outlines
        .iter()
        .flat_map(|o| curvyo_geometry_core::flatten_outline(&o.anchors, o.closed, tolerance));
    let first = points.next()?;
    Some(points.fold((first, first), |(lo, hi), p| {
        (
            Point::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}
