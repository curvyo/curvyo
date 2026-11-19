//! Builds the job of one page from its input (ADR 0064 §7, §9): each block's numbers, its plan
//! from the cache entered where the previous block left the head, then the moves.

use std::sync::Arc;

use crate::job_input::{BlockInput, JobInput, LeafGeometry, PointUm, um_of_mm};
use crate::job_moves::{Block, Frame, Job, build_moves, running_z_um};
use crate::job_numbers::{BlockNumbers, NumbersContext, NumbersRead, block_numbers};
use crate::job_plan::{Plan, PlanCache};
use crate::material_record::MaterialRead;
use crate::output_settings::Start;

/// The job and what the gate reads besides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    /// The job.
    pub job: Job,
    /// The numbers of each input block, in input order.
    pub numbers: Vec<NumbersRead>,
    /// Whether each input block has a run (items and readable settings), in input order.
    pub runs: Vec<bool>,
    /// The index in [`Job::blocks`] of each input block that runs, in input order.
    pub job_block: Vec<Option<usize>>,
    /// The plan of each input block that runs, in input order.
    pub plans: Vec<Option<Arc<Plan>>>,
}

/// The job of `input` with the library `materials`, plans reused from `cache`.
#[must_use]
pub fn build_job(input: &JobInput, materials: &[MaterialRead], cache: &mut PlanCache) -> Built {
    let values = &input.machine.values;
    let frame = frame_of(input);
    let context = numbers_context(input, materials);
    let mut built = empty_built(input, frame);
    let mut entry: PointUm = frame.origin();
    for block in &input.blocks {
        let read = block_numbers(block, context);
        let run = block.has_items() && !matches!(read, NumbersRead::Damaged);
        let (job_block, plan) = match (run, read.numbers()) {
            (true, Some(numbers)) => {
                let plan = cache.plan(block, numbers, entry);
                entry = plan.exit;
                built
                    .job
                    .blocks
                    .push(job_block_of(block, numbers, values.motorised_z));
                (Some(built.job.blocks.len() - 1), Some(plan))
            }
            _ => (None, None),
        };
        built.numbers.push(read);
        built.runs.push(run);
        built.job_block.push(job_block);
        built.plans.push(plan);
    }
    add_moves(&mut built, frame, input.start);
    let ids: Vec<_> = input.blocks.iter().map(|b| b.id).collect();
    let assets: Vec<_> = input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .filter_map(|leaf| match &leaf.geometry {
            LeafGeometry::Image { asset, .. } => Some(asset),
            LeafGeometry::Outlines(_) | LeafGeometry::Gradient { .. } => None,
        })
        .collect();
    let items: Vec<_> = input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .map(|leaf| leaf.node)
        .collect();
    cache.retain(&ids, &assets, &items);
    built
}

/// Works up to about `dots` dots of the raster sweeps the job of `input` needs that `cache` does
/// not hold yet, in run order (ADR 0069 §12); whether all of them are held, so [`build_job`]
/// builds no dot. Building in steps gives the same job as building at once.
#[must_use]
pub fn prepare_job(
    input: &JobInput,
    materials: &[MaterialRead],
    cache: &mut PlanCache,
    mut dots: u64,
) -> bool {
    let context = numbers_context(input, materials);
    input.blocks.iter().all(|block| {
        let read = block_numbers(block, context);
        match read.numbers() {
            Some(numbers) if block.has_items() && !matches!(read, NumbersRead::Damaged) => {
                cache.prepare(block, numbers, &mut dots)
            }
            _ => true,
        }
    })
}

/// What the numbers of the blocks of `input` are read with.
fn numbers_context<'a>(input: &'a JobInput, materials: &'a [MaterialRead]) -> NumbersContext<'a> {
    let values = &input.machine.values;
    NumbersContext {
        materials,
        profile_id: &input.machine.profile_id,
        fixture_material: input.fixture_material.as_deref(),
        max_feed: values.max_feed,
    }
}

/// The bed of `input` in µm with its origin corner.
fn frame_of(input: &JobInput) -> Frame {
    Frame {
        corner: input.machine.values.origin,
        width: um_of_mm(input.machine.bed.width.as_mm()),
        height: um_of_mm(input.machine.bed.height.as_mm()),
    }
}

/// The job of `input` with no block and no move yet.
fn empty_built(input: &JobInput, frame: Frame) -> Built {
    let capacity = input.blocks.len();
    Built {
        job: Job {
            name: input.name.clone(),
            machine: input.machine.values.clone(),
            frame,
            start: input.start,
            home: frame.origin(),
            blocks: Vec::new(),
            moves: Vec::new(),
            sweeps: Vec::new(),
        },
        numbers: Vec::with_capacity(capacity),
        runs: Vec::with_capacity(capacity),
        job_block: Vec::with_capacity(capacity),
        plans: Vec::with_capacity(capacity),
    }
}

/// The job block of the input `block` that runs with `numbers`.
fn job_block_of(block: &BlockInput, numbers: &BlockNumbers, motorised_z: bool) -> Block {
    Block {
        id: block.id,
        ring: block.ring,
        name: block.name.clone(),
        kind: numbers.kind,
        numbers: *numbers,
        z: running_z_um(numbers, motorised_z),
    }
}

/// Fills the moves and the home of `built` from its blocks and plans.
fn add_moves(built: &mut Built, frame: Frame, start: Start) {
    let pairs: Vec<(Block, &Plan)> = built
        .job
        .blocks
        .iter()
        .cloned()
        .zip(built.plans.iter().flatten().map(AsRef::as_ref))
        .collect();
    let moves = build_moves(frame, start, &pairs);
    built.job.moves = moves.moves;
    built.job.home = moves.home;
    built.job.sweeps = moves.sweeps;
}
