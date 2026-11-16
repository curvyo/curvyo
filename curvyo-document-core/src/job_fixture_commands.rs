//! The commands that write the job fixtures of a laser page, its own fixtures in no set (ADR 0059
//! §2, §6; `specs/0113` criteria 13, 14, 31 and 32), each one history step with its own label.

use crate::document::Document;
use crate::group_tree::node_id;
use crate::manufacturing::{ManufacturingError, next_numbered};
use crate::manufacturing_codec::KEY_FIXTURE_LOCKED;
use crate::manufacturing_fixture_commands::NewFixture;
use crate::manufacturing_fixture_sets::MAX_JOB_FIXTURES;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of a job fixture dropped by armed placement.
pub const ADD_JOB_FIXTURE_COMMIT_LABEL: &str = "add_job_fixture";
/// The label of Lock on a job fixture.
pub const LOCK_FIXTURE_COMMIT_LABEL: &str = "lock_fixture";
/// The label of Lock off on a job fixture.
pub const UNLOCK_FIXTURE_COMMIT_LABEL: &str = "unlock_fixture";

impl Document {
    /// Adds a job fixture "`label` N" (N the lowest number no fixture of the same kind on this
    /// page uses, in a set or not) at `new.matrix`, Active, unlocked, after the page's job fixtures, in one step
    /// "Add job fixture"; a pin or a screw gets its two role jobs (criteria 13, 27).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing;
    /// [`ManufacturingError::TooManyJobFixtures`] when the page holds 64 job fixtures already;
    /// [`ManufacturingError::OutOfRange`] for a matrix that is not finite or passes ±1e7.
    pub fn add_job_fixture(
        &self,
        manufacturing: NodeId,
        new: &NewFixture<'_>,
    ) -> Result<NodeId, ManufacturingError> {
        let snapshot = self
            .manufacturing(manufacturing)
            .ok_or(ManufacturingError::NotFound)?;
        if snapshot.job_fixtures().count() >= MAX_JOB_FIXTURES {
            return Err(ManufacturingError::TooManyJobFixtures);
        }
        let taken: Vec<String> = snapshot
            .fixtures
            .iter()
            .filter(|fixture| fixture.kind == new.kind)
            .map(|fixture| fixture.name.clone())
            .collect();
        let record = new.record(next_numbered(new.label, &taken, false))?;
        let node = self.insert_fixture(tree_id_of(manufacturing), &record)?;
        self.commit_with_label(ADD_JOB_FIXTURE_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Locks or unlocks the fixture `id` (`fixture_locked`), a job fixture or a set fixture, one
    /// step "Lock fixture" or "Unlock fixture" (criteria 31, 32). `Ok(false)` when its own Lock
    /// already reads so.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_locked(&self, id: NodeId, locked: bool) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        let fixture = self.fixture(id).ok_or(ManufacturingError::NoSuchFixture)?;
        if fixture.locked == locked {
            return Ok(false);
        }
        meta.insert(KEY_FIXTURE_LOCKED, locked)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        self.commit_with_label(if locked {
            LOCK_FIXTURE_COMMIT_LABEL
        } else {
            UNLOCK_FIXTURE_COMMIT_LABEL
        });
        Ok(true)
    }
}
