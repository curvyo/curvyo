//! The dot picture of the Before and after pane's After half (`specs/0135` criteria 12, 13; ADR
//! 0073 §8): the job's own dots of the item while the job is current, else a live picture of the
//! current stack on a coarser dot grid at the reduced level it fits, until the job settles.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::NodeId;
use curvyo_laser_core::job::{
    JobInput, MoveKind, NumbersContext, PointUm, Sweep, block_numbers, read_output_settings,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::Interval;

use super::super::Session;
use super::super::object_cache::DocumentRead;
use super::job_cache::PageJob;
use super::raster_picture::RasterPicture;
use super::raster_view::{
    by_item, capped_step, preview_numbers, raster_area_mm2, raster_block, retain_preview_plans,
};

/// A raster item (its node and stacking place) with its sweeps.
type ItemSweeps = ((NodeId, u32), Vec<Arc<Sweep>>);

/// The most dots of the reduced After picture (Proposal): a quarter of a tick of dithering, so
/// the frame of an edit draws it with the reduced level its dot grid fits.
const PANE_DOTS: f64 = 100_000.0;

impl Session {
    /// The After picture of picture `node` in operation `op`, with whether it is the reduced
    /// live picture (criterion 13); `None` when the item has no dots there. Kept while it is
    /// drawn from the same sweeps.
    pub(in crate::session) fn pane_after_picture(
        &self,
        read: &Arc<DocumentRead>,
        op: NodeId,
        node: NodeId,
    ) -> Option<(RasterPicture, bool)> {
        let (job, stale) = self.settled_job_in(Arc::clone(read))?;
        let picture = if stale {
            self.reduced_item_picture(read, op, node)?
        } else {
            self.job_item_picture(&job, op, node)?
        };
        Some((picture, stale))
    }

    /// The dot picture of picture `node` in the block of operation `op` of `job` (criterion 12:
    /// the job's own dots).
    fn job_item_picture(&self, job: &PageJob, op: NodeId, node: NodeId) -> Option<RasterPicture> {
        let index = job.input.blocks.iter().position(|b| b.id == op)?;
        let block = (*job.built.job_block.get(index)?)?;
        let mut seen = std::collections::HashSet::new();
        let mut sweeps = Vec::new();
        let mut stacking = 0;
        for m in &job.built.job.moves {
            let MoveKind::Sweep {
                node: at,
                stacking: place,
                sweep,
                ..
            } = m.kind
            else {
                continue;
            };
            if m.block != block || at != node || !seen.insert(sweep) {
                continue;
            }
            stacking = place;
            if let Some(sweep) = usize::try_from(sweep)
                .ok()
                .and_then(|i| job.built.job.sweeps.get(i))
            {
                sweeps.push(Arc::clone(sweep));
            }
        }
        self.kept_pane_picture((node, stacking), sweeps)
    }

    /// The live picture of picture `node` in operation `op` with the current registers: the
    /// item's dots at the smallest power-of-two multiple of the Interval with at most
    /// [`PANE_DOTS`] dots, its picture adjusted at the reduced level that grid fits.
    fn reduced_item_picture(
        &self,
        read: &Arc<DocumentRead>,
        op: NodeId,
        node: NodeId,
    ) -> Option<RasterPicture> {
        let (item, sweeps) =
            self.with_page_job_input_in(read, |input| self.reduced_sweeps(input, op, node))??;
        self.kept_pane_picture(item, sweeps)
    }

    /// The reduced sweeps of picture `node` in the block of operation `op` of `input`.
    fn reduced_sweeps(&self, input: &JobInput, op: NodeId, node: NodeId) -> Option<ItemSweeps> {
        let manufacturing = self.active_manufacturing()?;
        let block = input.blocks.iter().find(|b| b.id == op)?;
        let mut raster = raster_block(block)?;
        raster.leaves.retain(|leaf| leaf.node == node);
        if raster.leaves.is_empty() {
            return None;
        }
        let fixture_material = read_output_settings(&manufacturing).fixture_material;
        let context = NumbersContext {
            materials: self.materials().reads(),
            profile_id: &input.machine.profile_id,
            fixture_material: fixture_material.as_deref(),
            max_feed: input.machine.values.max_feed,
        };
        let read = block_numbers(&raster, context);
        let mut numbers =
            preview_numbers(&raster, &read).filter(|n| n.kind == OperationKind::Image)?;
        let k = capped_step(1, numbers.interval, raster_area_mm2(&raster), PANE_DOTS);
        let hundredths = i64::from(numbers.interval.hundredths()) * k;
        numbers.interval =
            Interval::from_hundredths(hundredths.min(i64::from(Interval::MAX_HUNDREDTHS)))
                .unwrap_or(numbers.interval);
        let mut held = self.output.raster.pane_plans.borrow_mut();
        let plans = held.get_or_insert_with(|| self.output.job.sharing_pyramids());
        let plan = plans.plan(&raster, &numbers, PointUm::default());
        retain_preview_plans(plans, input, &[raster.id]);
        let mut stacking = 0;
        let sweeps = plan
            .runs
            .iter()
            .flat_map(|run| &run.sweeps)
            .filter(|planned| planned.node == node)
            .map(|planned| {
                stacking = planned.stacking;
                Arc::clone(&planned.sweep)
            })
            .collect();
        Some(((node, stacking), sweeps))
    }

    /// The pane's picture of `item` from `sweeps`: the last one when it was drawn from the same
    /// sweeps, else a new one; `None` without sweeps.
    fn kept_pane_picture(
        &self,
        item: (NodeId, u32),
        sweeps: Vec<Arc<Sweep>>,
    ) -> Option<RasterPicture> {
        if sweeps.is_empty() {
            return None;
        }
        let cache = &self.output.raster;
        let last = cache.pane.borrow_mut().take();
        let old: HashMap<_, &RasterPicture> = by_item(last.iter());
        let picture = cache.picture(item, sweeps, &old)?;
        *cache.pane.borrow_mut() = Some(picture.clone());
        Some(picture)
    }
}
