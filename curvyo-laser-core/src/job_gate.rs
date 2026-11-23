//! The gate of the output group (ADR 0064 §8, amended 2026-11-17; criteria 2, 4 to 7, 21, 33,
//! 38): one text-free list of the machine's facts, the runs and the ordered blocks and warns.

use std::collections::HashSet;

use curvyo_document_core::NodeId;

use crate::GENERIC_LASER_ID;
use crate::job_build::Built;
use crate::job_gate_cause::{Cause, Severity};
use crate::job_gate_raster::{bounding_points, raster_issues, raster_leaves};
use crate::job_gate_rows::{RunFacts, run_facts};
use crate::job_input::{BlockInput, BlockSource, JobInput, MachineInput, PointUm};
use crate::job_moves::{Frame, MoveKind};
use crate::job_numbers::{NumbersRead, NumbersSource, RunNumbers};
use crate::job_timeline::Timeline;
use crate::machine_profile::OutputFormat;
use crate::operation_settings::Field;
use crate::operation_settings_rule::StoredReason;

/// The bed check's tolerance in µm (criterion 4 b).
pub const BED_TOLERANCE_UM: i64 = 10;

/// What the session knows of the page that the job does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GateFacts {
    /// Items on the page that are Not assigned.
    pub not_assigned: Vec<NodeId>,
    /// Items that run while hidden.
    pub hidden_running: Vec<NodeId>,
    /// Operations with Output off that hold items: the check's words name them.
    pub output_off: Vec<NodeId>,
    /// The items in those operations: what the check's button selects (`0101` criterion 17).
    pub output_off_items: Vec<NodeId>,
    /// Per Image operation with Output on, the leaves assigned to it that are no picture and
    /// burn nothing there (`specs/0123` criterion 48), in list order.
    pub not_pictures: Vec<(NodeId, Vec<NodeId>)>,
}

/// What a check line's button does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Targets {
    /// The line has no button.
    None,
    /// Selects these items.
    Items(Vec<NodeId>),
    /// Selects these operations (one unfolds and scrolls its row, UX P15).
    Operations(Vec<NodeId>),
    /// Expands the Fixture jobs section and focuses its material pick.
    FixturePick,
    /// Opens the Start pick.
    StartPick,
}

/// One check line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// The cause.
    pub cause: Cause,
    /// Its count.
    pub count: usize,
    /// What its button does.
    pub targets: Targets,
}

/// The machine as the machine line reads it (P30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineFacts {
    /// The profile copy's name.
    pub name: String,
    /// The output format.
    pub format: OutputFormat,
    /// Whether Air assist is on.
    pub air_assist: bool,
    /// Whether the copy is damaged or unreadable.
    pub damaged: bool,
}

/// The gate: one list that the operation rows (ring, time) and the checks both read.
#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    /// The machine.
    pub machine: MachineFacts,
    /// The runs in run order (blocks with items and readable settings).
    pub runs: Vec<RunFacts>,
    /// The checks, blocks first, in the order of criteria 4 and 5.
    pub issues: Vec<Issue>,
}

impl Gate {
    /// Whether Export is enabled: no block.
    #[must_use]
    pub fn export_allowed(&self) -> bool {
        self.first_block().is_none()
    }

    /// The first block, whose text is Export's tooltip.
    #[must_use]
    pub fn first_block(&self) -> Option<&Issue> {
        self.issues
            .iter()
            .find(|issue| issue.cause.severity() == Severity::Block)
    }

    /// Whether `cause` is among the checks.
    #[must_use]
    pub fn has(&self, cause: &Cause) -> bool {
        self.issues.iter().any(|issue| issue.cause == *cause)
    }
}

/// The gate of the job `built` of `input`.
#[must_use]
pub fn gate(input: &JobInput, built: &Built, timeline: &Timeline, facts: &GateFacts) -> Gate {
    let machine = &input.machine;
    let mut issues = Vec::new();
    let mut runs = Vec::new();
    let mut damaged = 0;
    for (index, block) in input.blocks.iter().enumerate() {
        let read = &built.numbers[index];
        if matches!(read, NumbersRead::Damaged) {
            damaged += 1;
            continue;
        }
        if !built.runs[index] {
            continue;
        }
        let time_s = built.job_block[index].map(|at| timeline.block_s(at));
        runs.push(run_facts(block, read, time_s));
        block_issues(
            block,
            read,
            built,
            index,
            machine.values.motorised_z,
            &mut issues,
        );
    }
    page_issues(input, built, &mut issues);
    raster_issues(input, built, &mut issues);
    over_limit(input, &mut runs, &mut issues);
    fact_issues(facts, &mut issues);
    if damaged > 0 {
        issues.push(issue(Cause::DamagedSettings, damaged, Targets::None));
    }
    dedupe(&mut issues);
    issues.sort_by_key(|issue| issue.cause.rank());
    Gate {
        machine: machine_facts(machine),
        runs,
        issues,
    }
}

/// An item over the dot limit blocks with its own line: it stands in for "Nothing runs", and a
/// row whose run burns nothing because of it reads no time ("–", `specs/0123` criterion 34).
fn over_limit(input: &JobInput, runs: &mut [RunFacts], issues: &mut Vec<Issue>) {
    let over: std::collections::HashSet<NodeId> = issues
        .iter()
        .filter_map(|issue| match issue.cause {
            Cause::TooManyDots { item, .. } => Some(item),
            _ => None,
        })
        .collect();
    if over.is_empty() {
        return;
    }
    issues.retain(|issue| issue.cause != Cause::NothingRuns);
    for run in runs.iter_mut().filter(|run| run.time_s == Some(0.0)) {
        let holds = input
            .blocks
            .iter()
            .find(|block| block.id == run.block)
            .is_some_and(|block| block.leaves.iter().any(|leaf| over.contains(&leaf.node)));
        if holds {
            run.time_s = None;
        }
    }
}

fn machine_facts(machine: &MachineInput) -> MachineFacts {
    MachineFacts {
        name: machine.values.name.clone(),
        format: machine.values.output_format,
        air_assist: machine.values.air_assist,
        damaged: machine.damaged,
    }
}

/// The issues of the page as a whole: the machine, Start, nothing runs, the bed, the Generic
/// laser.
fn page_issues(input: &JobInput, built: &Built, issues: &mut Vec<Issue>) {
    let machine = &input.machine;
    if machine.damaged {
        issues.push(issue(Cause::MachineDamaged, 1, Targets::None));
    }
    if input.start_damaged {
        issues.push(issue(Cause::StartDamaged, 1, Targets::StartPick));
    }
    // Only a running block's own block stands in for "nothing runs"; an operation without
    // items runs nothing, whatever its numbers (criterion 4 d).
    if !built.job.burns() && !built.unpriced() {
        issues.push(issue(Cause::NothingRuns, 1, Targets::None));
    }
    let outside = outside_bed(input, built);
    if !outside.is_empty() {
        let cause = Cause::OutsideBed {
            width_um: built.job.frame.width,
            height_um: built.job.frame.height,
        };
        issues.push(issue(cause, outside.len(), Targets::Items(outside)));
    }
    if machine.profile_id == GENERIC_LASER_ID {
        issues.push(issue(Cause::GenericLaser, 1, Targets::None));
    }
}

/// The issues of what the session knows beside the job (criterion 5; the `0101` summary's count
/// lines, P28).
fn fact_issues(facts: &GateFacts, issues: &mut Vec<Issue>) {
    if !facts.not_assigned.is_empty() {
        let items = facts.not_assigned.clone();
        issues.push(issue(
            Cause::NotAssigned,
            items.len(),
            Targets::Items(items),
        ));
    }
    if !facts.hidden_running.is_empty() {
        let items = facts.hidden_running.clone();
        issues.push(issue(Cause::HiddenRuns, items.len(), Targets::Items(items)));
    }
    if !facts.output_off.is_empty() {
        let ids = &facts.output_off;
        let op = (ids.len() == 1).then(|| ids[0]);
        issues.push(issue(
            Cause::OutputOff { op },
            ids.len(),
            Targets::Items(facts.output_off_items.clone()),
        ));
    }
    for (op, items) in &facts.not_pictures {
        let cause = Cause::NotPictures { op: *op };
        issues.push(issue(cause, items.len(), Targets::Items(items.clone())));
    }
}

fn issue(cause: Cause, count: usize, targets: Targets) -> Issue {
    Issue {
        cause,
        count,
        targets,
    }
}

/// Removes repeated fixture lines (one per page, not one per fixture block).
fn dedupe(issues: &mut Vec<Issue>) {
    let mut seen: Vec<Cause> = Vec::new();
    issues.retain(|issue| {
        let fixture = matches!(
            issue.cause,
            Cause::FixtureNoMaterial | Cause::FixtureNoSet { .. }
        );
        if fixture && seen.contains(&issue.cause) {
            return false;
        }
        seen.push(issue.cause.clone());
        true
    });
}

/// The issues of one running block.
fn block_issues(
    block: &BlockInput,
    read: &NumbersRead,
    built: &Built,
    index: usize,
    motorised_z: bool,
    issues: &mut Vec<Issue>,
) {
    let op = block.id;
    let operation = || Targets::Operations(vec![op]);
    let NumbersRead::Runs { numbers, source } = read else {
        if let Some(found) = blocking_issue(block, read) {
            issues.push(found);
        }
        return;
    };
    source_issues(op, source, issues);
    let over = std::iter::once(numbers.main)
        .chain(numbers.outline)
        .find(RunNumbers::over_feed);
    if let Some(set) = over {
        let cause = Cause::OverFeed {
            op,
            feed: set.feed.get(),
        };
        issues.push(issue(cause, 1, operation()));
    }
    let is_operation = matches!(block.source, BlockSource::Operation { .. });
    if is_operation && numbers.options.z_offset.tenths() != 0 && !motorised_z {
        issues.push(issue(Cause::ZWithoutAxis { op }, 1, operation()));
    }
    if let Some(plan) = &built.plans[index]
        && !plan.no_contour.is_empty()
    {
        let items = plan.no_contour.clone();
        let cause = Cause::CutOutNoContour { op };
        issues.push(issue(cause, items.len(), Targets::Items(items)));
    }
}

/// The block of a running block that has no numbers (criterion 4 a); `None` for damage, which
/// is counted once for the page.
fn blocking_issue(block: &BlockInput, read: &NumbersRead) -> Option<Issue> {
    match read {
        NumbersRead::NoSettings => Some(issue(
            Cause::NoSettings { op: block.id },
            1,
            Targets::Operations(vec![block.id]),
        )),
        NumbersRead::FixtureNoMaterial => {
            Some(issue(Cause::FixtureNoMaterial, 1, Targets::FixturePick))
        }
        NumbersRead::FixtureNoSet { material, kind } => {
            let cause = Cause::FixtureNoSet {
                material: material.clone(),
                kind: *kind,
            };
            Some(issue(cause, 1, Targets::FixturePick))
        }
        NumbersRead::Damaged | NumbersRead::Runs { .. } => None,
    }
}

/// The warns of where a block's numbers come from: stored numbers and typed overrides
/// (criterion 5).
fn source_issues(op: NodeId, source: &NumbersSource, issues: &mut Vec<Issue>) {
    let targets = || Targets::Operations(vec![op]);
    match source {
        NumbersSource::Stored {
            material,
            reason: StoredReason::NoSet(kind),
            ..
        } => issues.push(issue(
            Cause::NoSetForLaser {
                op,
                material: material.clone(),
                kind: *kind,
            },
            1,
            targets(),
        )),
        NumbersSource::Stored { material, .. } => issues.push(issue(
            Cause::MaterialNotFound {
                op,
                material: material.clone(),
            },
            1,
            targets(),
        )),
        NumbersSource::Live { .. } | NumbersSource::Own | NumbersSource::Fixture { .. } => {}
    }
    if let NumbersSource::Live { material, typed }
    | NumbersSource::Stored {
        material, typed, ..
    } = source
        && !typed.is_empty()
    {
        let fields = [Field::Power, Field::Speed, Field::Passes]
            .into_iter()
            .filter(|f| typed.iter().any(|(_, t)| t == f))
            .collect();
        let cause = Cause::TypedOverride {
            op,
            material: material.clone(),
            fields,
        };
        issues.push(issue(cause, 1, targets()));
    }
}

/// The bed as the check measures it: the machine frame with Machine origin, else the work area
/// placed with its origin corner on the matching corner of the burn moves' box (criterion 38).
/// The offset is added to every page point before [`Frame::holds`].
fn bed_offset(built: &Built) -> PointUm {
    let job = &built.job;
    if !job.start.relative() {
        return PointUm::default();
    }
    let mut from = job.home;
    let mut corners: Option<(PointUm, PointUm)> = None;
    for m in &job.moves {
        if matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }) {
            for p in [from, m.to] {
                let (lo, hi) = corners.get_or_insert((p, p));
                *lo = PointUm::new(lo.x.min(p.x), lo.y.min(p.y));
                *hi = PointUm::new(hi.x.max(p.x), hi.y.max(p.y));
            }
        }
        from = m.to;
    }
    let Some((lo, hi)) = corners else {
        return PointUm::default();
    };
    let frame = job.frame;
    let corner = PointUm::new(
        if frame.corner.is_right() { hi.x } else { lo.x },
        if frame.corner.is_bottom() { hi.y } else { lo.y },
    );
    let origin = frame.origin();
    PointUm::new(origin.x - corner.x, origin.y - corner.y)
}

/// Whether burn geometry of `built` lies outside the bed (4 b): the gate's measure, also for the
/// job a frame traces.
#[must_use]
pub fn outside_the_bed(input: &JobInput, built: &Built) -> bool {
    !outside_bed(input, built).is_empty()
}

/// The leaves with a burn point outside the bed, in move order, each once; then the raster
/// items whose frame reaches outside (`specs/0123` criterion 31: the frame counts even where its
/// edge is transparent).
fn outside_bed(input: &JobInput, built: &Built) -> Vec<NodeId> {
    let frame: Frame = built.job.frame;
    let offset = bed_offset(built);
    let holds = |p: PointUm| {
        frame.holds(
            PointUm::new(p.x + offset.x, p.y + offset.y),
            BED_TOLERANCE_UM,
        )
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut from = built.job.home;
    for m in &built.job.moves {
        if let MoveKind::Burn { node, .. } | MoveKind::Sweep { node, .. } = m.kind
            && (!holds(m.to) || !holds(from))
            && seen.insert(node)
        {
            out.push(node);
        }
        from = m.to;
    }
    for (index, block) in input.blocks.iter().enumerate() {
        if !built.runs[index] {
            continue;
        }
        for leaf in raster_leaves(block, &built.numbers[index]) {
            let outside = bounding_points(leaf)
                .into_iter()
                .any(|p| !holds(PointUm::of_mm(p)));
            if outside && seen.insert(leaf.node) {
                out.push(leaf.node);
            }
        }
    }
    out
}
