//! The output group's view of the gate (`specs/0029` criteria 1 to 7, 10, 25, 38, 39; UX notes §6;
//! ADR 0064 §8, §10), with the per-job `TabFacts` the Operations tab reads from the same gate; the
//! shared fixtures in `curvyo-editor-wasm/tests/fixtures/output_view/` pin its JSON.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::job::{
    Cause, CheckText, Issue, Names, NumbersContext, NumbersLine, Ring, RunFacts, Severity, Start,
    Targets, block_numbers, check_text, duration_text, estimate_text, extension, file_type,
    lengths_text, machine_line, numbers_lines, numbers_text, read_output_settings, start_text,
};
use curvyo_laser_core::operations::{KindRead, Summary};
use serde::Serialize;

use super::super::Session;
use super::super::operations::ActiveOperations;
use super::job_cache::PageJob;

/// The output group of the Operations tab of the active laser page (UX notes §6.3 to §6.6).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutputGroupView {
    /// "Diode 10 W · GRBL", " · air assist" when on; "Machine profile damaged" (P30).
    pub machine: String,
    /// The Start pick.
    pub start: StartView,
    /// The checks: blocks first, then warns; empty when there is none.
    pub checks: Vec<CheckView>,
    /// The total, `None` when nothing runs.
    pub total: Option<TotalView>,
    /// The Export button.
    pub export: ExportView,
    /// The Send to machine button; absent without a connector (`0125` S1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send: Option<super::send::SendView>,
    /// Whether the page's Toolpath view is pressed.
    pub toolpath_view: bool,
}

/// The Start pick (criterion 38; UX P29): one value of three, no disabled state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartView {
    /// The stored value's spelling; `None` while it is damaged (no option checked).
    pub value: Option<&'static str>,
    /// Whether the stored value is one this build does not read (the check `StartDamaged`).
    pub damaged: bool,
    /// The three options in order.
    pub options: Vec<StartOptionView>,
}

/// One option of the Start pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartOptionView {
    /// Its spelling, sent back by `set_start`.
    pub value: &'static str,
    /// Its words ("Home, then machine origin").
    pub label: &'static str,
}

/// One numbers line ("35 %, 6,000 mm/min, 1 ×, 0.25 mm"; "typed" after each typed number).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NumbersLineView {
    /// The line.
    pub text: String,
    /// The typed fields (`power`, `speed`, `passes`).
    pub typed: Vec<&'static str>,
}

/// What a check's button does (UX P15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckTargetView {
    /// The line has no button.
    None,
    /// Selects these items on the canvas.
    Items {
        /// The items.
        ids: Vec<NodeId>,
    },
    /// Selects these operations; one unfolds and scrolls its row.
    Operations {
        /// The operations.
        ids: Vec<NodeId>,
    },
    /// Expands the Fixture jobs section and focuses its pick.
    FixturePick,
    /// Opens the Start pick.
    StartPick,
}

/// One check line (criteria 4, 5; UX notes §6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckView {
    /// `block` (`OctagonX`) or `warn` (`TriangleAlert`).
    pub severity: &'static str,
    /// The whole text.
    pub text: String,
    /// The text before the button.
    pub before: String,
    /// The button's words (a name or a count), empty when the line has no button.
    pub button: String,
    /// The text after the button.
    pub after: String,
    /// The count.
    pub count: usize,
    /// What the button does.
    pub target: CheckTargetView,
}

/// The total (criterion 10; UX notes §6.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TotalView {
    /// "About 9 min 51 s".
    pub time: String,
    /// "Runs 3 operations, 16 items." (the `0101` summary's words), `None` when no work
    /// operation runs (`0103` criterion 17, lead decision J7).
    pub runs: Option<RunsLineView>,
    /// "1 fixture job runs." (`0103` criterion 17), `None` when no fixture job runs.
    pub fixtures: Option<FixtureLineView>,
    /// "Burn 38.29 m · travel 2.26 m".
    pub lengths: String,
}

/// The work line of the total: "Runs 3 operations, 16 items.", its item count a button that
/// selects the running items (`specs/0101` criterion 17; `select_operation_items("running")`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunsLineView {
    /// The whole text.
    pub text: String,
    /// The text before the button: "Runs 3 operations, ".
    pub before: String,
    /// The button's words: "16 items".
    pub button: String,
    /// The text after the button: ".".
    pub after: String,
}

/// The fixture line of the total: "1 fixture job runs.", its count a button that selects the
/// fixtures of the running jobs (`specs/0103` criterion 17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureLineView {
    /// The whole text.
    pub text: String,
    /// The button's words: "1 fixture job", "2 fixture jobs".
    pub button: String,
    /// The text after the button: " runs." or " run.".
    pub after: String,
    /// The fixtures the button selects.
    pub ids: Vec<NodeId>,
}

/// The Export button (criteria 1, 4, 27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportView {
    /// Whether it is enabled: no block.
    pub enabled: bool,
    /// The first block's text: the tooltip and description of the disabled button.
    pub tooltip: Option<String>,
    /// The save dialog's file type ("GRBL G-code").
    pub file_type: &'static str,
    /// The extension without its dot ("gcode").
    pub extension: &'static str,
}

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
    names: Names,
    /// The `0101` summary the total's work line counts.
    summary: Option<Summary>,
}

/// The run of one operation row (line 2, UX notes §6.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowRunView {
    /// The ring's digit: the row's digit (P26).
    pub digit: u32,
    /// The run's time ("3 min 21 s"; "–" with No settings).
    pub time: String,
}

/// The stored word of a typed field.
const fn field_word(field: curvyo_laser_core::settings::Field) -> &'static str {
    use curvyo_laser_core::settings::Field;
    match field {
        Field::Power => "power",
        Field::Speed => "speed",
        Field::Passes => "passes",
    }
}

/// The view of numbers lines.
pub(in crate::session) fn numbers_view(lines: &[NumbersLine]) -> Vec<NumbersLineView> {
    lines
        .iter()
        .map(|line| NumbersLineView {
            text: numbers_text(line),
            typed: line.typed.iter().map(|&f| field_word(f)).collect(),
        })
        .collect()
}

/// A run's time text: "–" with No settings.
pub(super) fn time_text(run: &RunFacts) -> String {
    run.time_s.map_or_else(|| "–".to_owned(), duration_text)
}

fn target_view(targets: &Targets) -> CheckTargetView {
    match targets {
        Targets::None => CheckTargetView::None,
        Targets::Items(ids) => CheckTargetView::Items { ids: ids.clone() },
        Targets::Operations(ids) => CheckTargetView::Operations { ids: ids.clone() },
        Targets::FixturePick => CheckTargetView::FixturePick,
        Targets::StartPick => CheckTargetView::StartPick,
    }
}

fn check_view(issue: &Issue, names: &Names) -> CheckView {
    let CheckText {
        before,
        button,
        after,
    } = check_text(issue, names);
    CheckView {
        severity: match issue.cause.severity() {
            Severity::Block => "block",
            Severity::Warn => "warn",
        },
        text: format!("{before}{button}{after}"),
        target: if button.is_empty() {
            CheckTargetView::None
        } else {
            target_view(&issue.targets)
        },
        before,
        button,
        after,
        count: issue.count,
    }
}

impl Session {
    /// The output group of the active laser page, `None` on another page or with its extension
    /// off.
    #[must_use]
    pub fn output_group_view(&self) -> Option<OutputGroupView> {
        let job = self.job_with_facts(None)?;
        let facts = job.tab.get()?;
        let manufacturing = self.document.manufacturing(job.manufacturing)?;
        let settings = read_output_settings(&manufacturing);
        let gate = &job.gate;
        let names = &facts.names;
        let format = job.input.machine.values.output_format;
        let first_block = gate.first_block().map(|i| check_text(i, names).whole());
        let send = self.send_view(&job, first_block.clone());
        Some(OutputGroupView {
            machine: machine_line(&gate.machine),
            start: StartView {
                value: (!settings.start_damaged).then(|| settings.start.as_str()),
                damaged: settings.start_damaged,
                options: Start::ALL
                    .into_iter()
                    .map(|start| StartOptionView {
                        value: start.as_str(),
                        label: start_text(start),
                    })
                    .collect(),
            },
            checks: gate.issues.iter().map(|i| check_view(i, names)).collect(),
            total: self.total_view(&job, facts.summary.as_ref()),
            export: ExportView {
                enabled: gate.export_allowed(),
                tooltip: first_block,
                file_type: file_type(format),
                extension: extension(format),
            },
            send,
            toolpath_view: self.toolpath_view_pressed(job.manufacturing),
        })
    }

    /// The total, `None` while nothing runs.
    fn total_view(&self, job: &PageJob, summary: Option<&Summary>) -> Option<TotalView> {
        if job.gate.has(&Cause::NothingRuns) || job.built.job.moves.is_empty() {
            return None;
        }
        let summary = summary?;
        let fixtures = self.running_fixture_jobs();
        Some(TotalView {
            time: estimate_text(job.timeline.total_s()),
            runs: (summary.operations > 0).then(|| runs_line(summary.operations, summary.running)),
            fixtures: (fixtures > 0).then(|| fixture_line(fixtures, self.running_fixture_ids())),
            lengths: lengths_text(job.timeline.burn_mm(), job.timeline.travel_mm()),
        })
    }

    /// The output group as JSON, `null` on another page.
    #[must_use]
    pub fn output_group_json(&self) -> String {
        serde_json::to_string(&self.output_group_view()).unwrap_or_else(|_| "null".to_owned())
    }

    /// The active laser page's job with its [`TabFacts`] built, `None` on another page or with
    /// its extension off.
    /// `active` is the page's operations when the caller holds them already.
    pub(in crate::session) fn job_with_facts(
        &self,
        active: Option<&ActiveOperations>,
    ) -> Option<Arc<PageJob>> {
        let job = self.laser_job()?;
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
                names: Names::of(operations.iter().map(|op| (op.id, op.name.clone()))),
                summary: self.summary_of(active),
            });
        }
        Some(job)
    }

    /// The numbers of every operation row of the active laser page: a block the job holds keeps
    /// the numbers it was built with, any other row (Output off) is read with the same function.
    fn all_row_numbers(
        &self,
        job: &PageJob,
        operations: &[OperationSnapshot],
    ) -> HashMap<NodeId, Option<Vec<NumbersLineView>>> {
        let built: HashMap<NodeId, &curvyo_laser_core::job::NumbersRead> = job
            .input
            .blocks
            .iter()
            .zip(&job.built.numbers)
            .filter(|(block, _)| matches!(block.ring, Ring::Operation(_)))
            .map(|(block, read)| (block.id, read))
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
                    let read = built.get(&op.id).map_or_else(
                        || {
                            let block = self.operation_block(op, std::iter::empty(), &[], &[], &[]);
                            block_numbers(&block, context)
                        },
                        |read| (*read).clone(),
                    );
                    row_numbers_view(&read)
                });
                (op.id, numbers)
            })
            .collect()
    }

    /// The effective numbers of operation `op` as its row's line 3 shows them, running or not
    /// (criterion 3), on a page without a job: the same functions the job runs. `None` for an Unknown kind or damaged
    /// settings (no line 3); empty for "No settings".
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
        row_numbers_view(&read)
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
fn row_numbers_view(read: &curvyo_laser_core::job::NumbersRead) -> Option<Vec<NumbersLineView>> {
    if matches!(read, curvyo_laser_core::job::NumbersRead::Damaged) {
        return None;
    }
    Some(numbers_view(&numbers_lines(read)))
}

/// "Runs 3 operations, 16 items." (`specs/0101` criterion 17's words), "16 items" the button.
fn runs_line(operations: usize, items: usize) -> RunsLineView {
    let plural = |n: usize, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    let before = format!("Runs {}, ", plural(operations, "operation", "operations"));
    let button = plural(items, "item", "items");
    RunsLineView {
        text: format!("{before}{button}."),
        before,
        button,
        after: ".".to_owned(),
    }
}

/// "1 fixture job runs." or "<F> fixture jobs run." (`specs/0103` criterion 17's words).
fn fixture_line(count: usize, ids: Vec<NodeId>) -> FixtureLineView {
    let (button, after) = if count == 1 {
        ("1 fixture job".to_owned(), " runs.")
    } else {
        (format!("{count} fixture jobs"), " run.")
    };
    FixtureLineView {
        text: format!("{button}{after}"),
        button,
        after: after.to_owned(),
        ids,
    }
}

#[cfg(test)]
mod tests {
    use super::{fixture_line, runs_line};

    #[test]
    fn the_fixture_line_keeps_the_0103_words() {
        assert_eq!(fixture_line(1, Vec::new()).text, "1 fixture job runs.");
        assert_eq!(fixture_line(3, Vec::new()).text, "3 fixture jobs run.");
    }

    #[test]
    fn the_runs_line_keeps_the_summary_words_and_its_item_count_is_the_button() {
        let line = runs_line(3, 16);
        assert_eq!(line.text, "Runs 3 operations, 16 items.");
        assert_eq!(
            (
                line.before.as_str(),
                line.button.as_str(),
                line.after.as_str()
            ),
            ("Runs 3 operations, ", "16 items", ".")
        );
        assert_eq!(runs_line(1, 1).text, "Runs 1 operation, 1 item.");
    }
}
