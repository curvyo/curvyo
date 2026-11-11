//! The fixture jobs of the active laser page as the session reads them (`specs/0103-fixture-jobs`,
//! ADR 0052, feature `adrs.md` §6): each fixture's jobs typed once with their ticked parts, the
//! one session field (the job whose Parts list is open), and the run rule's answer.

mod commands;
mod look;
mod parts;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_document_core::{FixtureJobSnapshot, JobTarget, NodeId};
use curvyo_laser_core::operations::{FixtureJobRead, KindRead, fixture_block};
use curvyo_laser_core::{FixtureKind, FixturePart, job_parts};

pub use view_json::{
    AddTarget, FixtureJobsView, JobGroupView, JobKindView, JobRowView, PartRowView, PartsView,
};

use super::super::Session;
use super::placed::{PageFixtures, PlacedFixture, PlacedSlot};

/// One fixture job of a placed fixture, typed.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct PlacedJob {
    pub id: NodeId,
    pub name: String,
    /// The kind as read; Unknown for a missing, damaged or unknown kind text (criterion 21).
    pub kind: KindRead,
    pub show: bool,
    pub output: bool,
    /// The ticked parts that read, in Parts-list order (criteria 8, 12).
    pub parts: Vec<FixturePart>,
}

impl PlacedJob {
    /// Whether the job can only be renamed or deleted (criterion 21).
    pub(in crate::session) fn read_only(&self) -> bool {
        self.kind.known().is_none()
    }
}

/// The jobs of a fixture of `kind` (node `fixture`, readable slots `slots`) and how many of
/// their ticks name no part of the kind (each one damaged value, ADR 0052 §3). A tick of a slot
/// that does not read is left out and is not damage (criterion 12).
pub(super) fn read_jobs(
    kind: FixtureKind,
    fixture: NodeId,
    slots: &[PlacedSlot],
    snapshots: &[FixtureJobSnapshot],
) -> (Vec<PlacedJob>, usize) {
    let mut damaged = 0;
    let jobs = snapshots
        .iter()
        .map(|job| {
            let targets: Vec<JobTarget> = job
                .targets
                .iter()
                .filter(|t| t.ticked)
                .filter_map(|t| {
                    let slot = if t.key.node == fixture {
                        None
                    } else {
                        Some(slots.iter().position(|s| s.id == t.key.node)?)
                    };
                    Some(JobTarget {
                        slot,
                        suffix: t.key.suffix.clone(),
                    })
                })
                .collect();
            let (parts, bad) = job_parts(kind, slots.len(), &targets);
            // An unknown fixture's jobs are opaque: their ticks are kept, not judged.
            if kind != FixtureKind::Unknown {
                damaged += bad;
            }
            PlacedJob {
                id: job.id,
                name: job.name.clone(),
                kind: KindRead::of(job.kind.as_deref()),
                show: job.show,
                output: job.output,
                parts,
            }
        })
        .collect();
    (jobs, damaged)
}

/// The jobs of a page in fixture creation order, then job order, with their fixtures.
pub(super) fn page_jobs(fixtures: &PageFixtures) -> Vec<(&PlacedFixture, &PlacedJob)> {
    fixtures
        .fixtures
        .iter()
        .flat_map(|f| f.jobs.iter().map(move |job| (f, job)))
        .collect()
}

/// The positions in [`page_jobs`] of the jobs that run, in run order (criteria 13, 14).
pub(super) fn running(jobs: &[(&PlacedFixture, &PlacedJob)], cuts_last: bool) -> Vec<usize> {
    let reads: Vec<FixtureJobRead> = jobs
        .iter()
        .map(|(fixture, job)| FixtureJobRead {
            kind: job.kind,
            output: job.output,
            fixture_active: fixture.active,
            parts: job.parts.clone(),
        })
        .collect();
    fixture_block(&reads, cuts_last)
}

impl Session {
    /// The job `id` of the active laser page with its fixture.
    pub(super) fn page_job(&self, id: NodeId) -> Option<(PageFixtures, PlacedFixture, PlacedJob)> {
        let fixtures = self.active_fixtures()?;
        let (fixture, job) = fixtures.fixtures.iter().find_map(|f| {
            f.jobs
                .iter()
                .find(|job| job.id == id)
                .map(|job| (f.clone(), job.clone()))
        })?;
        Some((fixtures, fixture, job))
    }

    /// The job whose Parts list is open: it exists on the active laser page and its kind is
    /// known (criterion 8).
    #[must_use]
    pub fn selected_fixture_job(&self) -> Option<NodeId> {
        let id = self.fixtures.job?;
        self.page_job(id)
            .filter(|(_, _, job)| !job.read_only())
            .map(|_| id)
    }

    /// Selects the job `id` (its Parts list opens); an Unknown job selects nothing (criterion 21).
    /// Not an undo step; the canvas selection is kept.
    pub fn select_fixture_job(&mut self, id: NodeId) -> bool {
        if self.page_job(id).is_none_or(|(_, _, job)| job.read_only()) {
            return false;
        }
        self.fixtures.job = Some(id);
        true
    }

    /// Closes the open Parts list. `false` when none was open.
    pub fn deselect_fixture_job(&mut self) -> bool {
        self.fixtures.job.take().is_some()
    }

    /// The fixture job of the active laser page named by its full id text.
    #[must_use]
    pub fn fixture_job_named(&self, name: &str) -> Option<NodeId> {
        let fixtures = self.active_fixtures()?;
        fixtures
            .fixtures
            .iter()
            .flat_map(|f| f.jobs.iter())
            .find(|job| job.id.full_id() == name)
            .map(|job| job.id)
    }

    /// How many fixture jobs of the active laser page run (criterion 17): jobs, not parts.
    #[must_use]
    pub fn running_fixture_jobs(&self) -> usize {
        self.active_fixtures().map_or(0, |fixtures| {
            running(&page_jobs(&fixtures), self.fixture_cuts_last(&fixtures)).len()
        })
    }

    /// The Cuts last switch of the page of `fixtures`.
    fn fixture_cuts_last(&self, fixtures: &PageFixtures) -> bool {
        self.document
            .manufacturing(fixtures.manufacturing)
            .is_some_and(|m| curvyo_laser_core::operations::cuts_last(&m))
    }
}
