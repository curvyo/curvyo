//! What a successful export of one job writes to the project (ADR 0064 §11; criteria 28 to 30).

use curvyo_document_core::NodeId;

use crate::job_build::Built;
use crate::job_input::{BlockSource, JobInput};
use crate::material_record::MaterialRead;
use crate::operation_settings::StoredSettings;
use crate::operation_settings_rule::refresh;

/// The effects of an export, taken from the exported job (so an edit while the save dialog is
/// open changes nothing of what is recorded).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportEffects {
    /// Every fixture job of the job, to switch off (criterion 28 a), in run order.
    pub fixture_jobs_off: Vec<NodeId>,
    /// The refreshed stored numbers beside each running operation's material link, typed numbers
    /// kept (criterion 28 b; `refresh(.., guard: false)`), in run order; only those that differ.
    pub refreshed: Vec<(NodeId, StoredSettings)>,
    /// Every test grid with at least one derived operation that wrote a move (criteria 28 c,
    /// 29), in run order, each once.
    pub lasered_grids: Vec<NodeId>,
}

/// The effects of exporting `built`, the job of `input` with the library `materials`.
#[must_use]
pub fn export_effects(
    input: &JobInput,
    built: &Built,
    materials: &[MaterialRead],
) -> ExportEffects {
    let mut effects = ExportEffects::default();
    for (index, block) in input.blocks.iter().enumerate() {
        let Some(job_block) = built.job_block.get(index).copied().flatten() else {
            continue;
        };
        match &block.source {
            BlockSource::Fixture { jobs } => effects.fixture_jobs_off.extend(jobs.iter().copied()),
            BlockSource::Operation { settings, grid, .. } => {
                if let Some(kind) = block.kind.known()
                    && let Some(value) =
                        refresh(settings, kind, materials, &input.machine.profile_id, false)
                {
                    effects.refreshed.push((block.id, value));
                }
                let moved = built.job.moves.iter().any(|m| m.block == job_block);
                if let Some(grid) = grid
                    && moved
                    && !effects.lasered_grids.contains(grid)
                {
                    effects.lasered_grids.push(*grid);
                }
            }
        }
    }
    effects
}
