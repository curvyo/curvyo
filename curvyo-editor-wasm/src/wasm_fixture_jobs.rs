//! Thin host bridge of the fixture jobs of the active laser page (`specs/0103-fixture-jobs`): the
//! Fixture jobs section's view and the commands of its rows and Parts list. Ids are the full id
//! texts the view JSON carries; parts are named by their tokens (`board`, `slot:0`, ...).

use curvyo_document_core::{FixtureJobSwitch, NodeId};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The Fixture jobs section of the active laser page as JSON; `null` on another page.
    pub fn fixture_jobs_view_json(&self) -> String {
        self.session.fixture_jobs_view_json()
    }

    /// One of the five kind buttons (`cut`, `score`, `engrave`, `engrave_outline`, `cut_out`):
    /// a job on the one selected fixture (criterion 2). The new job's id, `None` when none was
    /// added.
    pub fn add_fixture_job(&mut self, kind: &str) -> Option<String> {
        self.session.add_fixture_job(kind).map(NodeId::full_id)
    }

    /// Selects the job `id`: its Parts list opens (criterion 8).
    pub fn select_fixture_job(&mut self, id: &str) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.select_fixture_job(id))
    }

    /// Closes the open Parts list.
    pub fn deselect_fixture_job(&mut self) -> bool {
        self.session.deselect_fixture_job()
    }

    /// Renames the job `id` (criterion 4).
    pub fn rename_fixture_job(&mut self, id: &str, name: &str) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.rename_fixture_job(id, name))
    }

    /// Deletes the job `id` with its ticks: the second press of its armed trash (criterion 4).
    pub fn delete_fixture_job(&mut self, id: &str) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.delete_fixture_job(id))
    }

    /// The Show switch of the job `id` (criterion 4).
    pub fn set_fixture_job_show(&mut self, id: &str, on: bool) -> bool {
        self.job(id).is_some_and(|id| {
            self.session
                .set_fixture_job_switch(id, FixtureJobSwitch::Show, on)
        })
    }

    /// The Output switch of the job `id` (criteria 4, 5).
    pub fn set_fixture_job_output(&mut self, id: &str, on: bool) -> bool {
        self.job(id).is_some_and(|id| {
            self.session
                .set_fixture_job_switch(id, FixtureJobSwitch::Output, on)
        })
    }

    /// Moves the job `id` to position `to` among its fixture's jobs: an arrow or a drop
    /// (criterion 3).
    pub fn move_fixture_job(&mut self, id: &str, to: u32) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.move_fixture_job(id, to as usize))
    }

    /// Ticks or unticks the part `token` in the job `id` (criterion 9).
    pub fn set_fixture_job_part(&mut self, id: &str, token: &str, ticked: bool) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.set_fixture_job_part(id, token, ticked))
    }

    /// "Select" on the part `token` of the job `id`: the slot or the fixture is selected on the
    /// canvas (criterion 9).
    pub fn select_fixture_job_part(&mut self, id: &str, token: &str) -> bool {
        self.job(id)
            .is_some_and(|id| self.session.select_fixture_job_part(id, token))
    }

    fn job(&self, id: &str) -> Option<NodeId> {
        self.session.fixture_job_named(id)
    }
}
