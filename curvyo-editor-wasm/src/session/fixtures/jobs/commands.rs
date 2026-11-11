//! The commands of a fixture job row and of its Parts list (`specs/0103-fixture-jobs` criteria 2
//! to 5, 9, 21): each one history step with the label of ADR 0052 §8. An Unknown job is refused
//! everything but rename and delete; the document commands do not refuse it, this module does.

use curvyo_document_core::{FixtureJobSwitch, NodeId};
use curvyo_laser_core::operations::{OperationKind, next_operation_name};

use super::super::super::Session;
use super::parts::{part_key, part_node, part_of_token};

impl Session {
    /// One of the five kind buttons (criterion 2): a job "`Kind` N" (the lowest N no job of
    /// that kind on this page uses) at the end of the one selected fixture's jobs, Show on,
    /// Output off, nothing ticked, selected, in one step "Add fixture job". `None` for an unknown
    /// kind text or without exactly one selected fixture.
    pub fn add_fixture_job(&mut self, kind: &str) -> Option<NodeId> {
        let kind = OperationKind::parse(kind)?;
        let fixture = self.selected_fixture()?;
        let fixtures = self.active_fixtures()?;
        let names: Vec<&str> = fixtures
            .fixtures
            .iter()
            .flat_map(|f| f.jobs.iter().map(|job| job.name.as_str()))
            .collect();
        let name = next_operation_name(kind, names);
        let id = self.recording_unguarded(|this| {
            this.document
                .add_fixture_job(fixture, kind.as_str(), &name)
                .ok()
        })?;
        self.fixtures.job = Some(id);
        Some(id)
    }

    /// Renames the job `id`; an empty name reverts (criterion 4). Allowed on an Unknown job.
    pub fn rename_fixture_job(&mut self, id: NodeId, name: &str) -> bool {
        self.page_job(id).is_some()
            && self.recording_unguarded(|this| {
                this.document.rename_fixture_job(id, name).unwrap_or(false)
            })
    }

    /// Deletes the job `id` with its ticks (criteria 4, 11). Allowed on an Unknown job.
    pub fn delete_fixture_job(&mut self, id: NodeId) -> bool {
        if self.page_job(id).is_none() {
            return false;
        }
        let deleted = self.recording_unguarded(|this| this.document.delete_fixture_job(id).is_ok());
        if deleted && self.fixtures.job == Some(id) {
            self.fixtures.job = None;
        }
        deleted
    }

    /// The Show or Output switch of the job `id` (criteria 4, 5). `false` when it already reads
    /// so or the job is Unknown.
    pub fn set_fixture_job_switch(
        &mut self,
        id: NodeId,
        which: FixtureJobSwitch,
        on: bool,
    ) -> bool {
        self.editable_job(id)
            && self.recording_unguarded(|this| {
                this.document
                    .set_fixture_job_switch(id, which, on)
                    .unwrap_or(false)
            })
    }

    /// Moves the job `id` to `to` among its fixture's jobs (criterion 3: arrows and drag, inside
    /// its group only). `false` when it already sits there or the job is Unknown.
    pub fn move_fixture_job(&mut self, id: NodeId, to: usize) -> bool {
        self.editable_job(id)
            && self
                .recording_unguarded(|this| this.document.move_fixture_job(id, to).unwrap_or(false))
    }

    /// Ticks (`ticked` true) or unticks the part `token` (`board`, `slot:0`, `grid.lines`, ...)
    /// of the job `id`'s fixture (criterion 9): one step "Assign to fixture job" or "Remove from
    /// fixture job". `false` for a token that names no part, an Unknown job, or no change.
    pub fn set_fixture_job_part(&mut self, id: NodeId, token: &str, ticked: bool) -> bool {
        if !self.editable_job(id) {
            return false;
        }
        let Some((_, fixture, _)) = self.page_job(id) else {
            return false;
        };
        let Some(key) = part_of_token(&fixture, token).and_then(|part| part_key(&fixture, part))
        else {
            return false;
        };
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_job_targets(id, &[key], ticked)
                .unwrap_or(false)
        })
    }

    /// "Select" on a part row (criterion 9): selects the slot, or the fixture, on the canvas; the
    /// job stays selected. Not an undo step.
    pub fn select_fixture_job_part(&mut self, id: NodeId, token: &str) -> bool {
        let Some((_, fixture, job)) = self.page_job(id) else {
            return false;
        };
        let Some(part) = part_of_token(&fixture, token) else {
            return false;
        };
        let Some(node) = part_node(&fixture, part) else {
            return false;
        };
        let selected = if node == fixture.id {
            self.select_fixture(node)
        } else {
            self.select_slot(node)
        };
        if selected && !job.read_only() {
            self.fixtures.job = Some(id);
        }
        selected
    }

    /// Whether the job `id` of the active page may take a command other than rename and delete.
    fn editable_job(&self, id: NodeId) -> bool {
        self.page_job(id)
            .is_some_and(|(_, _, job)| !job.read_only())
    }
}
