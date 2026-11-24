//! The per-job facts the Operations tab reads beside the gate (`specs/0029` criteria 3, 36 d;
//! `specs/0123` criterion 34): each row's run and numbers, the names the checks use, built once
//! per job.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::job::{
    LeafGeometry, LeafInput, Names, NumbersContext, Ring, block_numbers, numbers_lines,
    read_output_settings,
};
use curvyo_laser_core::operations::{KindRead, Summary};
use serde::Serialize;

use super::super::Session;
use super::super::operations::ActiveOperations;
use super::job_cache::PageJob;
use super::view_json::{NumbersLineView, numbers_view, time_text};

/// What the Operations tab reads of one job beyond its gate: built once per job, so a frame of
/// the tab reads it instead of resolving every row's numbers and the summary again (criterion
/// 36 d).
#[derive(Debug)]
pub(in crate::session) struct TabFacts {
    /// The run of each operation row that runs: its digit and time (line 2).
    pub runs: HashMap<NodeId, RowRunView>,
    /// The numbers of each operation row (line 3), `None` for an Unknown kind or damaged
    /// settings.
    pub numbers: HashMap<NodeId, Option<Vec<NumbersLineView>>>,
    /// The operation names the checks name.
    pub names: Names,
    /// The `0101` summary the total's work line counts.
    pub summary: Option<Summary>,
}

/// The run of one operation row (line 2, UX notes §6.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowRunView {
    /// The ring's digit: the row's digit (P26).
    pub digit: u32,
    /// The run's time ("3 min 21 s"; an operation always has one since `specs/0150`).
    pub time: String,
}

impl Session {
    /// The job the views read (see `settled_job`) with its [`TabFacts`] built and whether its
    /// times and lengths read as stale (after the host's 300 ms mark, UX notes P35), `None` on another page or with its extension off.
    /// `active` is the page's operations when the caller holds them already.
    pub(in crate::session) fn job_with_facts(
        &self,
        active: Option<&ActiveOperations>,
    ) -> Option<(Arc<PageJob>, bool)> {
        let (job, stale) = self.settled_job()?;
        if job.tab.get().is_none() {
            let fetched;
            let active = if let Some(active) = active {
                active
            } else {
                fetched = self.active_operations()?;
                &fetched
            };
            let operations = &active.operations;
            let _ = job.tab.set(TabFacts {
                runs: row_runs(&job),
                numbers: self.all_row_numbers(&job, operations),
                names: Names::of(
                    operations
                        .iter()
                        .map(|op| (op.id, op.name.clone()))
                        .chain(self.raster_item_names(&job)),
                ),
                summary: self.summary_of(active),
            });
        }
        Some((job, stale && self.stale_words_shown()))
    }

    /// The names of the raster items of `job` as the layers list shows them (its own name, else
    /// "Image 3f9a1c"), for the dot-limit line (`specs/0123` criterion 34, UX E3).
    fn raster_item_names(&self, job: &PageJob) -> Vec<(NodeId, String)> {
        let read = self.object_read();
        job.input
            .blocks
            .iter()
            .flat_map(|block| &block.leaves)
            .filter(|leaf| {
                matches!(
                    leaf.geometry,
                    LeafGeometry::Image { .. } | LeafGeometry::Gradient { .. }
                )
            })
            .map(|leaf| {
                let name = self
                    .document
                    .node_registers(leaf.node)
                    .and_then(|registers| registers.name)
                    .unwrap_or_else(|| curvyo_ui_core::object_name(&read, leaf.node));
                (leaf.node, name)
            })
            .collect()
    }

    /// The numbers of every operation row of the active laser page: a block the job holds keeps
    /// the numbers it was built with, any other row (Output off) is read with the same function.
    fn all_row_numbers(
        &self,
        job: &PageJob,
        operations: &[OperationSnapshot],
    ) -> HashMap<NodeId, Option<Vec<NumbersLineView>>> {
        let built: HashMap<NodeId, (&curvyo_laser_core::job::NumbersRead, &[LeafInput])> = job
            .input
            .blocks
            .iter()
            .zip(&job.built.numbers)
            .filter(|(block, _)| matches!(block.ring, Ring::Operation(_)))
            .map(|(block, read)| (block.id, (read, block.leaves.as_slice())))
            .collect();
        let context = NumbersContext {
            materials: self.materials().reads(),
            profile_id: &job.input.machine.profile_id,
            fixture_material: job.input.fixture_material.as_deref(),
            max_feed: job.input.machine.values.max_feed,
        };
        operations
            .iter()
            .map(|op| {
                let numbers = KindRead::of(op.kind.as_deref()).known().and_then(|_| {
                    let (read, leaves) = built.get(&op.id).map_or_else(
                        || {
                            let block = self.operation_block(op, std::iter::empty(), &[], &[], &[]);
                            (block_numbers(&block, context), Vec::new())
                        },
                        |(read, leaves)| ((*read).clone(), leaves.to_vec()),
                    );
                    row_numbers_view(&read, &leaves)
                });
                (op.id, numbers)
            })
            .collect()
    }

    /// The effective numbers of operation `op` as its row's line 3 shows them, running or not
    /// (criterion 3), on a page without a job: the same functions the job runs. `None` for an Unknown kind or damaged
    /// settings (no line 3).
    pub(in crate::session) fn row_numbers(
        &self,
        op: &OperationSnapshot,
    ) -> Option<Vec<NumbersLineView>> {
        KindRead::of(op.kind.as_deref()).known()?;
        let copy = self.active_copy()?;
        let manufacturing = self.active_manufacturing()?;
        let profile = &copy.read.copy;
        let profile_id = profile
            .id
            .clone()
            .unwrap_or_else(|| curvyo_laser_core::GENERIC_LASER_ID.to_owned());
        let fixture_material = read_output_settings(&manufacturing).fixture_material;
        let block = self.operation_block(op, std::iter::empty(), &[], &[], &[]);
        let read = block_numbers(
            &block,
            NumbersContext {
                materials: self.materials().reads(),
                profile_id: &profile_id,
                fixture_material: fixture_material.as_deref(),
                max_feed: profile.values.max_feed,
            },
        );
        row_numbers_view(&read, &block.leaves)
    }
}

/// The run of each operation row of `job` that runs, by operation id.
fn row_runs(job: &PageJob) -> HashMap<NodeId, RowRunView> {
    job.gate
        .runs
        .iter()
        .filter_map(|run| match run.ring {
            Ring::Operation(digit) => Some((
                run.block,
                RowRunView {
                    digit,
                    time: time_text(run),
                },
            )),
            Ring::Fixture => None,
        })
        .collect()
}

/// Line 3 of a row from its numbers, `None` for damaged settings.
fn row_numbers_view(
    read: &curvyo_laser_core::job::NumbersRead,
    leaves: &[LeafInput],
) -> Option<Vec<NumbersLineView>> {
    if matches!(read, curvyo_laser_core::job::NumbersRead::Damaged) {
        return None;
    }
    Some(numbers_view(&numbers_lines(read, leaves)))
}
