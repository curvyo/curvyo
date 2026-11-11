//! The Fixture jobs section's view (`specs/0103-fixture-jobs` criteria 1 to 5, 8, 15, 17, 21;
//! feature `adrs.md` §6): one serde type, the only place that builds this JSON. The shared
//! fixtures in `curvyo-editor-wasm/tests/fixtures/fixture_jobs_view/` pin its shape.

use curvyo_laser_core::FixturePart;
use serde::Serialize;

use super::super::super::Session;
use super::super::FixtureKindView;
use super::super::placed::{PageFixtures, PlacedFixture};
use super::parts::{part_name, part_node, parts_of};
use super::{PlacedJob, page_jobs, running};

/// The whole Fixture jobs section of the active laser page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureJobsView {
    /// The jobs in the section, Unknown ones included: the header "Fixture jobs (N)".
    pub count: usize,
    /// The one selected fixture the five add buttons add to; `None` hides them (criterion 2).
    pub add_to: Option<AddTarget>,
    /// The fixtures with at least one job, in creation order, each with its jobs (criterion 3).
    pub groups: Vec<JobGroupView>,
    /// The job whose Parts list is open.
    pub selected: Option<String>,
    /// The open Parts list, `None` without a selected job (criterion 8).
    pub parts: Option<PartsView>,
    /// How many jobs run: the summary's "1 fixture job runs." (criterion 17).
    pub running: usize,
    /// The fixtures of the running jobs, for the summary's count button (criterion 17).
    pub running_fixtures: Vec<String>,
    /// The values of the jobs that did not read, all fixtures (criterion 22).
    pub damaged: usize,
    /// One notice per fixture with damaged job values: "1 value damaged on Holder 1 job."
    /// (criterion 22). Shown by the section; the Fixtures block keeps its own notice.
    pub damage_notices: Vec<String>,
}

/// The job damage notice of one fixture (criterion 22: "<n> values damaged on <name> job.").
fn damage_text(n: usize, name: &str) -> String {
    match n {
        1 => format!("1 value damaged on {name} job."),
        _ => format!("{n} values damaged on {name} job."),
    }
}

/// The fixture the add buttons name ("Add Cut job to Holder 1").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddTarget {
    /// The fixture's full id text.
    pub fixture: String,
    /// Its name.
    pub name: String,
}

/// One fixture's group: its heading and its jobs in order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobGroupView {
    /// The fixture's full id text.
    pub fixture: String,
    /// Its name.
    pub name: String,
    /// Its kind, for the heading's symbol.
    pub kind: FixtureKindView,
    /// Its jobs in order.
    pub jobs: Vec<JobRowView>,
}

/// A job's kind: the stored text (the symbol's name, as `0101`) and its word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct JobKindView {
    /// `cut`, `score`, `engrave`, `engrave_outline` or `cut_out`.
    pub text: &'static str,
    /// "Cut", "Score", "Engrave", "Engrave + Outline" or "Cut-out".
    pub word: &'static str,
}

/// One job's row (criteria 4, 15, 21).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)] // the row's switches and states
pub struct JobRowView {
    /// The job's full id text.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its kind; `None` for an Unknown job (rename and delete only).
    pub kind: Option<JobKindView>,
    /// The Show switch.
    pub show: bool,
    /// The Output switch.
    pub output: bool,
    /// Whether its fixture is Active; `false` reads "not on the bed".
    pub fixture_active: bool,
    /// Whether it runs (the run rule's answer, criterion 13).
    pub runs: bool,
    /// Its ticked readable parts.
    pub parts_ticked: usize,
    /// Its fixture's readable parts.
    pub parts_total: usize,
    /// Its place among its fixture's jobs, from 0, for the arrows.
    pub position: usize,
    /// How many jobs its fixture has.
    pub of: usize,
}

/// The Parts list of the selected job (criterion 8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PartsView {
    /// The job's full id text.
    pub job: String,
    /// The rows in Parts-list order.
    pub rows: Vec<PartRowView>,
}

/// One part's row and the look of the part (criterion 18).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PartRowView {
    /// The part's token, sent back by a tick or a Select.
    pub token: String,
    /// The row name.
    pub name: String,
    /// Whether the job ticks it.
    pub ticked: bool,
    /// The node "Select" selects: the slot, else the fixture.
    pub select: String,
    /// `running`, `output_off`, `show_off`, `not_on_bed` or `unticked`.
    pub state: &'static str,
    /// The job's kind text while the part runs; `None` otherwise.
    pub kind: Option<&'static str>,
}

/// A job row under its fixture in the Layers tree (UX notes 8, J15): the chevron shows when a
/// fixture has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayerJobView {
    /// The job's full id text.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its kind text; `None` for an Unknown job.
    pub kind: Option<&'static str>,
    /// The Output switch: "Output off" shows when it is off.
    pub output: bool,
}

fn kind_view(job: &PlacedJob) -> Option<JobKindView> {
    job.kind.known().map(|kind| JobKindView {
        text: kind.as_str(),
        word: kind.label(),
    })
}

/// The state of `part` in `job` of `fixture`: what the canvas draws it as (criterion 18).
fn part_state(fixture: &PlacedFixture, job: &PlacedJob, part: FixturePart) -> &'static str {
    if !job.parts.contains(&part) {
        "unticked"
    } else if !job.show || job.read_only() {
        "show_off"
    } else if !fixture.active {
        "not_on_bed"
    } else if !job.output {
        "output_off"
    } else {
        "running"
    }
}

fn parts_view(fixture: &PlacedFixture, job: &PlacedJob) -> PartsView {
    let rows = parts_of(fixture)
        .into_iter()
        .map(|part| {
            let state = part_state(fixture, job, part);
            PartRowView {
                token: part.token(),
                name: part_name(fixture, part),
                ticked: job.parts.contains(&part),
                select: part_node(fixture, part)
                    .map_or_else(String::new, curvyo_document_core::NodeId::full_id),
                state,
                kind: (state == "running")
                    .then(|| {
                        job.kind
                            .known()
                            .map(curvyo_laser_core::operations::OperationKind::as_str)
                    })
                    .flatten(),
            }
        })
        .collect();
    PartsView {
        job: job.id.full_id(),
        rows,
    }
}

fn groups(fixtures: &PageFixtures, runs: &[bool]) -> Vec<JobGroupView> {
    let mut at = 0;
    fixtures
        .fixtures
        .iter()
        .filter(|f| !f.jobs.is_empty())
        .map(|fixture| {
            let total = parts_of(fixture).len();
            let jobs = fixture
                .jobs
                .iter()
                .enumerate()
                .map(|(position, job)| JobRowView {
                    id: job.id.full_id(),
                    name: job.name.clone(),
                    kind: kind_view(job),
                    show: job.show,
                    output: job.output,
                    fixture_active: fixture.active,
                    runs: runs[at + position],
                    parts_ticked: job.parts.len(),
                    parts_total: total,
                    position,
                    of: fixture.jobs.len(),
                })
                .collect();
            at += fixture.jobs.len();
            JobGroupView {
                fixture: fixture.id.full_id(),
                name: fixture.name.clone(),
                kind: super::super::view_json::kind_view(fixture.kind),
                jobs,
            }
        })
        .collect()
}

impl Session {
    /// The Fixture jobs section of the active laser page; `None` on any other page.
    #[must_use]
    pub fn fixture_jobs_view(&self) -> Option<FixtureJobsView> {
        let fixtures = self.active_fixtures()?;
        let jobs = page_jobs(&fixtures);
        let block = running(&jobs, self.fixture_cuts_last(&fixtures));
        let mut runs = vec![false; jobs.len()];
        let mut running_fixtures: Vec<String> = Vec::new();
        for &at in &block {
            runs[at] = true;
            let id = jobs[at].0.id.full_id();
            if !running_fixtures.contains(&id) {
                running_fixtures.push(id);
            }
        }
        let selected = self.selected_fixture_job();
        let parts = selected.and_then(|id| {
            jobs.iter()
                .find(|(_, job)| job.id == id)
                .map(|(fixture, job)| parts_view(fixture, job))
        });
        let add_to = self
            .selected_fixture()
            .and_then(|id| fixtures.get(id))
            .map(|f| AddTarget {
                fixture: f.id.full_id(),
                name: f.name.clone(),
            });
        let damaged_by_fixture = fixtures.fixtures.iter().filter(|f| f.job_damaged > 0);
        Some(FixtureJobsView {
            count: jobs.len(),
            add_to,
            groups: groups(&fixtures, &runs),
            selected: selected.map(curvyo_document_core::NodeId::full_id),
            parts,
            running: block.len(),
            running_fixtures,
            damaged: fixtures.fixtures.iter().map(|f| f.job_damaged).sum(),
            damage_notices: damaged_by_fixture
                .map(|f| damage_text(f.job_damaged, &f.name))
                .collect(),
        })
    }

    /// The jobs of the fixture `fixture` of the active laser page as its Layers rows; empty
    /// without jobs (no chevron).
    #[must_use]
    pub(in crate::session) fn fixture_layer_jobs(
        &self,
        fixture: curvyo_document_core::NodeId,
    ) -> Vec<LayerJobView> {
        self.active_fixtures()
            .and_then(|fixtures| {
                fixtures.get(fixture).map(|f| {
                    f.jobs
                        .iter()
                        .map(|job| LayerJobView {
                            id: job.id.full_id(),
                            name: job.name.clone(),
                            kind: job
                                .kind
                                .known()
                                .map(curvyo_laser_core::operations::OperationKind::as_str),
                            output: job.output,
                        })
                        .collect()
                })
            })
            .unwrap_or_default()
    }

    /// [`Session::fixture_jobs_view`] as JSON text, `null` off a laser page.
    #[must_use]
    pub fn fixture_jobs_view_json(&self) -> String {
        serde_json::to_string(&self.fixture_jobs_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
