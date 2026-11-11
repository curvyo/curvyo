//! The commands that write the fixture jobs of a fixture (ADR 0052 §4, `specs/0103-fixture-jobs`
//! criteria 2 to 5, 7, 9 to 11), each one history step with its own label.

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::fixture_job_codec::{
    FixtureJobSnapshot, FixturePartKey, JobRecord, PART_SUFFIXES, is_job_meta, parse_target_key,
    target_key, write_job_registers,
};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{SHAPE_FIXTURE, is_manufacturing_node, string};
use crate::manufacturing_fixtures::fixture_name;
use crate::node_registers::{KEY_NAME, read_registers};
use crate::operation_codec::{KEY_OP_OUTPUT, KEY_OP_SHOW};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;

/// The label of Add fixture job.
pub const ADD_FIXTURE_JOB_COMMIT_LABEL: &str = "add_fixture_job";
/// The label of a job's rename.
pub const RENAME_FIXTURE_JOB_COMMIT_LABEL: &str = "rename_fixture_job";
/// The label of Delete fixture job.
pub const DELETE_FIXTURE_JOB_COMMIT_LABEL: &str = "delete_fixture_job";
/// The label of a job's Show switched on.
pub const SET_FIXTURE_JOB_SHOW_ON_COMMIT_LABEL: &str = "set_fixture_job_show_on";
/// The label of a job's Show switched off.
pub const SET_FIXTURE_JOB_SHOW_OFF_COMMIT_LABEL: &str = "set_fixture_job_show_off";
/// The label of a job's Output switched on.
pub const SET_FIXTURE_JOB_OUTPUT_ON_COMMIT_LABEL: &str = "set_fixture_job_output_on";
/// The label of a job's Output switched off.
pub const SET_FIXTURE_JOB_OUTPUT_OFF_COMMIT_LABEL: &str = "set_fixture_job_output_off";
/// The label of a job's reorder inside its fixture.
pub const MOVE_FIXTURE_JOB_COMMIT_LABEL: &str = "move_fixture_job";
/// The label of a tick.
pub const ASSIGN_FIXTURE_TARGET_COMMIT_LABEL: &str = "assign_fixture_target";
/// The label of an untick.
pub const REMOVE_FIXTURE_TARGET_COMMIT_LABEL: &str = "remove_fixture_target";

/// A refused fixture job command; it writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FixtureJobError {
    /// The id names no live fixture of a manufacturing.
    #[error("no such fixture")]
    NoSuchFixture,
    /// The id names no live fixture job.
    #[error("no such fixture job")]
    NoSuchJob,
    /// A part is not the job's fixture or one of its slots, or its suffix is not a part suffix.
    #[error("no such fixture part")]
    NoSuchPart,
}

/// The two switches of a job row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureJobSwitch {
    /// Show: view only.
    Show,
    /// Output: whether the job runs.
    Output,
}

/// The parent node of `node`, when it has one.
fn parent_of(tree: &LoroTree, node: TreeID) -> Option<TreeID> {
    match tree.parent(node) {
        Some(TreeParentId::Node(parent)) => Some(parent),
        _ => None,
    }
}

/// Whether `node` is a live fixture node of a manufacturing.
fn is_fixture_node(tree: &LoroTree, node: TreeID) -> bool {
    node_exists(tree, node)
        && parent_of(tree, node).is_some_and(|parent| is_manufacturing_node(tree, parent))
        && tree
            .get_meta(node)
            .is_ok_and(|meta| string(&meta, KEY_SHAPE).as_deref() == Some(SHAPE_FIXTURE))
}

/// The job children of the fixture `fixture`, in tree order.
fn job_nodes(tree: &LoroTree, fixture: TreeID) -> Vec<TreeID> {
    tree.children(TreeParentId::Node(fixture))
        .unwrap_or_default()
        .into_iter()
        .filter(|&child| tree.get_meta(child).is_ok_and(|meta| is_job_meta(&meta)))
        .collect()
}

impl Document {
    /// The job `id` with its meta map and its fixture node.
    fn job_meta(&self, id: NodeId) -> Result<(LoroMap, TreeID), FixtureJobError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let fixture = parent_of(&tree, node).filter(|&f| is_fixture_node(&tree, f));
        let meta = tree.get_meta(node).ok().filter(is_job_meta);
        match (node_exists(&tree, node), fixture, meta) {
            (true, Some(fixture), Some(meta)) => Ok((meta, fixture)),
            _ => Err(FixtureJobError::NoSuchJob),
        }
    }

    /// The job `id` as read, `None` when it is not a live fixture job.
    #[must_use]
    pub fn fixture_job(&self, id: NodeId) -> Option<FixtureJobSnapshot> {
        let (_, fixture) = self.job_meta(id).ok()?;
        self.fixture(node_id(fixture))?
            .jobs
            .into_iter()
            .find(|job| job.id == id)
    }

    /// Adds a job of the kind text `kind` named `name` (cleaned, cut to 40 characters) at the end
    /// of the fixture's jobs: Show on, Output off, no tick, in one step (criteria 2, 5).
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchFixture`] when `fixture` is not a live fixture.
    pub fn add_fixture_job(
        &self,
        fixture: NodeId,
        kind: &str,
        name: &str,
    ) -> Result<NodeId, FixtureJobError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let parent = tree_id_of(fixture);
        if !is_fixture_node(&tree, parent) {
            return Err(FixtureJobError::NoSuchFixture);
        }
        let record = JobRecord {
            kind: kind.to_owned(),
            name: fixture_name(name).unwrap_or_else(|| kind.to_owned()),
            show: true,
            targets: Vec::new(),
        };
        let node = insert_job(&tree, parent, &record, &[])?;
        self.commit_with_label(ADD_FIXTURE_JOB_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Renames the job `id`, cleaned and cut to 40 characters. `Ok(false)` when the name is
    /// unchanged or nothing is left of it (criterion 4).
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchJob`].
    pub fn rename_fixture_job(&self, id: NodeId, name: &str) -> Result<bool, FixtureJobError> {
        let (meta, _) = self.job_meta(id)?;
        let Some(name) = fixture_name(name) else {
            return Ok(false);
        };
        if read_registers(&meta).name.as_deref() == Some(name.as_str()) {
            return Ok(false);
        }
        meta.insert(KEY_NAME, name.as_str())
            .map_err(|_| FixtureJobError::NoSuchJob)?;
        self.commit_with_label(RENAME_FIXTURE_JOB_COMMIT_LABEL);
        Ok(true)
    }

    /// Deletes the job `id` with its ticks in one step; undo revives both (criteria 4, 11).
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchJob`].
    pub fn delete_fixture_job(&self, id: NodeId) -> Result<(), FixtureJobError> {
        self.job_meta(id)?;
        self.loro()
            .get_tree(OBJECTS_TREE)
            .delete(tree_id_of(id))
            .map_err(|_| FixtureJobError::NoSuchJob)?;
        self.commit_with_label(DELETE_FIXTURE_JOB_COMMIT_LABEL);
        Ok(())
    }

    /// Sets the Show or Output switch of the job `id`. `Ok(false)` when it already reads so.
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchJob`].
    pub fn set_fixture_job_switch(
        &self,
        id: NodeId,
        which: FixtureJobSwitch,
        on: bool,
    ) -> Result<bool, FixtureJobError> {
        let (meta, _) = self.job_meta(id)?;
        let job = self.fixture_job(id).ok_or(FixtureJobError::NoSuchJob)?;
        let (key, now, labels) = match which {
            FixtureJobSwitch::Show => (
                KEY_OP_SHOW,
                job.show,
                [
                    SET_FIXTURE_JOB_SHOW_ON_COMMIT_LABEL,
                    SET_FIXTURE_JOB_SHOW_OFF_COMMIT_LABEL,
                ],
            ),
            FixtureJobSwitch::Output => (
                KEY_OP_OUTPUT,
                job.output,
                [
                    SET_FIXTURE_JOB_OUTPUT_ON_COMMIT_LABEL,
                    SET_FIXTURE_JOB_OUTPUT_OFF_COMMIT_LABEL,
                ],
            ),
        };
        if now == on {
            return Ok(false);
        }
        meta.insert(key, on)
            .map_err(|_| FixtureJobError::NoSuchJob)?;
        self.commit_with_label(if on { labels[0] } else { labels[1] });
        Ok(true)
    }

    /// Moves the job `id` to `index` among its fixture's jobs (or last), one tree move in one
    /// step (criterion 3). `Ok(false)` when it already sits there.
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchJob`].
    pub fn move_fixture_job(&self, id: NodeId, index: usize) -> Result<bool, FixtureJobError> {
        let (_, fixture) = self.job_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let list = job_nodes(&tree, fixture);
        let others: Vec<TreeID> = list.iter().copied().filter(|&n| n != node).collect();
        if list.iter().position(|&n| n == node) == Some(index.min(others.len())) {
            return Ok(false);
        }
        tree.enable_fractional_index(0);
        let moved = match (others.get(index), others.last()) {
            (Some(&before), _) => tree.mov_before(node, before),
            (None, Some(&after)) => tree.mov_after(node, after),
            (None, None) => Ok(()),
        };
        moved.map_err(|_| FixtureJobError::NoSuchJob)?;
        self.commit_with_label(MOVE_FIXTURE_JOB_COMMIT_LABEL);
        Ok(true)
    }

    /// Ticks (`ticked` true) or unticks the `parts` of the job `id` in one step (criterion 9).
    /// `Ok(false)` and nothing written when no key changes.
    ///
    /// # Errors
    /// [`FixtureJobError::NoSuchJob`]; [`FixtureJobError::NoSuchPart`] when a part is not the
    /// job's fixture or one of its slots, or its suffix is not one of [`PART_SUFFIXES`].
    pub fn set_fixture_job_targets(
        &self,
        id: NodeId,
        parts: &[FixturePartKey],
        ticked: bool,
    ) -> Result<bool, FixtureJobError> {
        let (meta, fixture) = self.job_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let part_ok = |part: &FixturePartKey| {
            let node = tree_id_of(part.node);
            let suffix_ok = part
                .suffix
                .as_deref()
                .is_none_or(|s| PART_SUFFIXES.contains(&s));
            let node_ok = node == fixture
                || (node_exists(&tree, node)
                    && parent_of(&tree, node) == Some(fixture)
                    && !tree.get_meta(node).is_ok_and(|m| is_job_meta(&m)));
            suffix_ok && node_ok
        };
        if ticked && !parts.iter().all(part_ok) {
            return Err(FixtureJobError::NoSuchPart);
        }
        let mut changed = false;
        for part in parts {
            let key = target_key(part);
            let present = meta.get(&key);
            let is_true = matches!(
                present,
                Some(loro::ValueOrContainer::Value(loro::LoroValue::Bool(true)))
            );
            let written = match (ticked, is_true, present.is_some()) {
                (true, false, _) => meta.insert(&key, true),
                (false, _, true) => meta.delete(&key),
                _ => continue,
            };
            written.map_err(|_| FixtureJobError::NoSuchJob)?;
            changed = true;
        }
        if changed {
            self.commit_with_label(if ticked {
                ASSIGN_FIXTURE_TARGET_COMMIT_LABEL
            } else {
                REMOVE_FIXTURE_TARGET_COMMIT_LABEL
            });
        }
        Ok(changed)
    }

    /// Removes, uncommitted, every tick of the slot `slot` from the jobs of its fixture
    /// `fixture`: the sweep of a slot's removal, in the caller's step (ADR 0052 §4).
    pub(crate) fn remove_slot_ticks(&self, fixture: TreeID, slot: TreeID) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for job in job_nodes(&tree, fixture) {
            let Ok(meta) = tree.get_meta(job) else {
                continue;
            };
            let mut doomed = Vec::new();
            meta.for_each(|key, _| {
                if parse_target_key(key).is_some_and(|(node, _)| node == slot) {
                    doomed.push(key.to_owned());
                }
            });
            for key in doomed {
                let _ = meta.delete(&key);
            }
        }
    }
}

/// Creates, uncommitted, a job node from `record` at the end of the fixture `parent`, with one
/// tick per entry of `ticks` and Output off (Pick and Reset from profile, ADR 0052 §7).
pub(crate) fn insert_job(
    tree: &LoroTree,
    parent: TreeID,
    record: &JobRecord,
    ticks: &[FixturePartKey],
) -> Result<TreeID, FixtureJobError> {
    tree.enable_fractional_index(0);
    let node = tree
        .create(TreeParentId::Node(parent))
        .map_err(|_| FixtureJobError::NoSuchFixture)?;
    let meta = tree
        .get_meta(node)
        .map_err(|_| FixtureJobError::NoSuchFixture)?;
    write_job_registers(&meta, record, ticks);
    Ok(node)
}
