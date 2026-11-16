//! Aligns a page's set mirror with its profile's sets (ADR 0059 §10, `specs/0113` criteria 44 and
//! 47): sets, fixtures and jobs matched by their profile ids, the page's own choices kept.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::fixture_job::insert_job;
use crate::fixture_job_codec::{
    FixturePartKey, JobRecord, KEY_JOB_ROLE, TARGET_PREFIX, is_job_meta, target_key,
};
use crate::group_tree::node_id;
use crate::history::LABEL_ALIGN_FIXTURE_SETS;
use crate::manufacturing::ManufacturingError;
use crate::manufacturing_codec::{
    KEY_FIXTURE_KIND, KEY_FIXTURE_LOCKED, KEY_FIXTURE_MATRIX, KEY_FIXTURE_PARAMS, KEY_FIXTURE_UID,
    KEY_JOB_UID, KEY_SET_LOCKED, KEY_SET_UID, KEY_SETS_MODIFIED, KEY_SLOT_PARAMS, SHAPE_FIXTURE,
    SHAPE_FIXTURE_SLOT, has_shape, read_sets_modified, string, write_plain_map,
};
use crate::manufacturing_fixture_sets::NewSetRegisters;
use crate::manufacturing_fixtures::{
    FixtureRecord, FixtureSetRecord, SlotRecord, read_flag, read_matrix, read_whole_map,
    write_matrix,
};
use crate::node_registers::KEY_NAME;
use crate::operation_codec::{KEY_OP_KIND, KEY_OP_SHOW};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// How an alignment is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignCommit<'a> {
    /// As one commit with the reserved label `align_fixture_sets`: no step of the stacks, no
    /// history row (the guarded alignment at open, library reload and page activation).
    Reserved,
    /// As part of the step labelled so: inside a batch it joins the batch's step (the other
    /// pages of a set edit made in this project).
    Step(&'a str),
}

/// The profile id `key` of `meta`, `None` when absent, empty or not a text.
fn uid(meta: &LoroMap, key: &str) -> Option<String> {
    string(meta, key).filter(|uid| !uid.is_empty())
}

fn put<V: Into<LoroValue>>(meta: &LoroMap, key: &str, value: V) {
    // invariant: a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

/// Writes the text `value` under `key` when it differs.
fn set_text(meta: &LoroMap, key: &str, value: &str) -> bool {
    if string(meta, key).as_deref() == Some(value) {
        return false;
    }
    put(meta, key, value);
    true
}

/// Writes the flag `value` under `key` (absent reads off) when it reads otherwise.
fn set_flag(meta: &LoroMap, key: &str, value: bool) -> bool {
    if read_flag(meta, key, false) == (value, 0) {
        return false;
    }
    put(meta, key, value);
    true
}

/// Writes `params` whole under `key` when the stored map does not read as them.
fn set_map(meta: &LoroMap, key: &str, params: &serde_json::Map<String, serde_json::Value>) -> bool {
    if read_whole_map(meta, key).as_ref() == Some(params) {
        return false;
    }
    write_plain_map(meta, key, params);
    true
}

/// Writes the fixture matrix `matrix` (`None`: no matrix) when the stored one differs.
fn set_matrix(meta: &LoroMap, matrix: Option<Affine>) -> bool {
    let same = match matrix {
        Some(matrix) => read_matrix(meta) == (Some(matrix), 0),
        None => meta.get(KEY_FIXTURE_MATRIX).is_none(),
    };
    if same {
        return false;
    }
    match matrix {
        Some(matrix) => write_matrix(meta, matrix),
        None => {
            let _ = meta.delete(KEY_FIXTURE_MATRIX);
        }
    }
    true
}

fn meta_of(tree: &LoroTree, node: TreeID) -> Result<LoroMap, ManufacturingError> {
    tree.get_meta(node)
        .map_err(|_| ManufacturingError::NotFound)
}

fn children(tree: &LoroTree, node: TreeID) -> Vec<TreeID> {
    tree.children(TreeParentId::Node(node)).unwrap_or_default()
}

/// The nodes among `nodes` matched to `wanted` ids in order, and the rest (no id, an id the
/// record lacks, a second node of one id), which the alignment deletes.
fn match_by_uid(
    tree: &LoroTree,
    nodes: &[TreeID],
    key: &str,
    wanted: &[&str],
) -> (Vec<Option<TreeID>>, Vec<TreeID>) {
    let mut matched: Vec<Option<TreeID>> = vec![None; wanted.len()];
    let mut doomed = Vec::new();
    for &node in nodes {
        let found = tree
            .get_meta(node)
            .ok()
            .and_then(|meta| uid(&meta, key))
            .and_then(|id| wanted.iter().position(|w| !w.is_empty() && *w == id));
        match found {
            Some(at) if matched[at].is_none() => matched[at] = Some(node),
            _ => doomed.push(node),
        }
    }
    (matched, doomed)
}

/// Puts the siblings `wanted` into that order, the first where the first of them stood; `true`
/// when anything moved.
fn order(tree: &LoroTree, parent: TreeID, wanted: &[TreeID]) -> Result<bool, ManufacturingError> {
    let current: Vec<TreeID> = children(tree, parent)
        .into_iter()
        .filter(|child| wanted.contains(child))
        .collect();
    if current == wanted {
        return Ok(false);
    }
    tree.enable_fractional_index(0);
    let moved = |result: loro::LoroResult<()>| result.map_err(|_| ManufacturingError::NotFound);
    if let (Some(&first), Some(&at)) = (wanted.first(), current.first())
        && first != at
    {
        moved(tree.mov_before(first, at))?;
    }
    for pair in wanted.windows(2) {
        moved(tree.mov_after(pair[1], pair[0]))?;
    }
    Ok(true)
}

fn delete_all(tree: &LoroTree, nodes: &[TreeID]) -> Result<bool, ManufacturingError> {
    for &node in nodes {
        tree.delete(node)
            .map_err(|_| ManufacturingError::NotFound)?;
    }
    crate::groups::remove_emptied_groups(tree, &[None]);
    Ok(!nodes.is_empty())
}

impl Document {
    /// Aligns the set mirror of the manufacturing `id` with the profile's `sets` (ADR 0059 §10):
    /// sets matched by `set_uid`, fixtures by `fixture_uid` within their set and jobs by `job_uid`
    /// within their fixture. A kept node keeps its node id and the page's choices (On this page,
    /// On the bed, the eye, Output); its content registers are rewritten where they differ. A
    /// missing set is created off this page (Frage 218 A), a missing fixture On the bed, a missing
    /// job with Output off; a node the record lacks (or a second node of one id) is deleted with
    /// its subtree; the order follows the record; slots follow by place and ticks are rewritten
    /// from the record's parts. `mfg_fixture_sets_modified` becomes `modified`. Job fixtures are
    /// never touched. `Ok(false)` and nothing written when the mirror already is so.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `id` is not a live manufacturing.
    pub fn align_fixture_sets(
        &self,
        id: NodeId,
        sets: &[FixtureSetRecord],
        modified: i64,
        commit: AlignCommit<'_>,
    ) -> Result<bool, ManufacturingError> {
        if self.manufacturing(id).is_none() {
            return Err(ManufacturingError::NotFound);
        }
        match commit {
            AlignCommit::Reserved => {
                let mut result = Ok(false);
                self.commit_reserved(LABEL_ALIGN_FIXTURE_SETS, |doc| {
                    result = doc.write_alignment(tree_id_of(id), sets, modified);
                });
                result
            }
            AlignCommit::Step(label) => {
                let result = self.write_alignment(tree_id_of(id), sets, modified);
                self.commit_with_label(label);
                result
            }
        }
    }

    /// Gives every set, set fixture and set fixture job of the manufacturing `id` that has no
    /// profile id one from `next`, as one step labelled `label` (inside a batch: part of the
    /// batch's step). The caller makes the ids (ADR 0060 §1: `new_record_id`). `Ok(false)` when
    /// every node has one.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `id` is not a live manufacturing.
    pub fn fill_fixture_set_uids(
        &self,
        id: NodeId,
        next: &mut dyn FnMut() -> String,
        label: &str,
    ) -> Result<bool, ManufacturingError> {
        if self.manufacturing(id).is_none() {
            return Err(ManufacturingError::NotFound);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut changed = false;
        let mut fill = |meta: &LoroMap, key: &str| {
            if uid(meta, key).is_none() {
                put(meta, key, next().as_str());
                changed = true;
            }
        };
        for set in self.set_nodes(tree_id_of(id)) {
            fill(&meta_of(&tree, set)?, KEY_SET_UID);
            for fixture in children(&tree, set) {
                let meta = meta_of(&tree, fixture)?;
                if !has_shape(&meta, SHAPE_FIXTURE) {
                    continue;
                }
                fill(&meta, KEY_FIXTURE_UID);
                for job in children(&tree, fixture) {
                    let meta = meta_of(&tree, job)?;
                    if is_job_meta(&meta) {
                        fill(&meta, KEY_JOB_UID);
                    }
                }
            }
        }
        if changed {
            self.commit_with_label(label);
        }
        Ok(changed)
    }

    /// The writes of [`Document::align_fixture_sets`], uncommitted.
    fn write_alignment(
        &self,
        manufacturing: TreeID,
        sets: &[FixtureSetRecord],
        modified: i64,
    ) -> Result<bool, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let nodes = self.set_nodes(manufacturing);
        let wanted: Vec<&str> = sets.iter().map(|set| set.id.as_str()).collect();
        let (matched, doomed) = match_by_uid(&tree, &nodes, KEY_SET_UID, &wanted);
        let mut changed = delete_all(&tree, &doomed)?;
        let mut kept = Vec::with_capacity(sets.len());
        for (record, node) in sets.iter().zip(matched) {
            let node = if let Some(node) = node {
                let meta = meta_of(&tree, node)?;
                changed |= set_text(&meta, KEY_NAME, &record.name);
                changed |= set_flag(&meta, KEY_SET_LOCKED, record.locked);
                node
            } else {
                changed = true;
                let new = NewSetRegisters {
                    uid: &record.id,
                    name: &record.name,
                    locked: record.locked,
                    on_page: false,
                };
                self.insert_set(manufacturing, &new)?
            };
            changed |= self.align_set_fixtures(&tree, node, &record.fixtures)?;
            kept.push(node);
        }
        changed |= order(&tree, manufacturing, &kept)?;
        let meta = meta_of(&tree, manufacturing)?;
        if read_sets_modified(&meta) != modified
            || !matches!(
                meta.get(KEY_SETS_MODIFIED),
                Some(ValueOrContainer::Value(LoroValue::I64(_)))
            )
        {
            put(&meta, KEY_SETS_MODIFIED, modified);
            changed = true;
        }
        Ok(changed)
    }

    /// Aligns the fixtures of the set node `set` with `records`.
    fn align_set_fixtures(
        &self,
        tree: &LoroTree,
        set: TreeID,
        records: &[FixtureRecord],
    ) -> Result<bool, ManufacturingError> {
        let nodes: Vec<TreeID> = children(tree, set)
            .into_iter()
            .filter(|&child| {
                tree.get_meta(child)
                    .is_ok_and(|meta| has_shape(&meta, SHAPE_FIXTURE))
            })
            .collect();
        let wanted: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        let (matched, doomed) = match_by_uid(tree, &nodes, KEY_FIXTURE_UID, &wanted);
        let mut changed = delete_all(tree, &doomed)?;
        let mut kept = Vec::with_capacity(records.len());
        for (record, node) in records.iter().zip(matched) {
            let node = if let Some(node) = node {
                changed |= self.align_fixture(tree, node, record)?;
                node
            } else {
                changed = true;
                self.insert_fixture(set, record)?
            };
            kept.push(node);
        }
        changed |= order(tree, set, &kept)?;
        Ok(changed)
    }

    /// Rewrites the content of the kept fixture `node` from `record`: kind, name, values, matrix,
    /// Lock, its slots by place and its jobs by id. On the bed and the eye stay.
    fn align_fixture(
        &self,
        tree: &LoroTree,
        node: TreeID,
        record: &FixtureRecord,
    ) -> Result<bool, ManufacturingError> {
        let meta = meta_of(tree, node)?;
        let mut changed = set_text(&meta, KEY_FIXTURE_KIND, &record.kind);
        changed |= set_text(&meta, KEY_NAME, &record.name);
        changed |= set_map(&meta, KEY_FIXTURE_PARAMS, &record.params);
        changed |= set_matrix(&meta, record.matrix);
        changed |= set_flag(&meta, KEY_FIXTURE_LOCKED, record.locked);
        let (job_nodes, slot_nodes): (Vec<TreeID>, Vec<TreeID>) = children(tree, node)
            .into_iter()
            .partition(|&child| tree.get_meta(child).is_ok_and(|meta| is_job_meta(&meta)));
        let slots = self.align_slots(tree, node, &slot_nodes, &record.slots)?;
        changed |= slots.1;
        let wanted: Vec<&str> = record.jobs.iter().map(|j| j.id.as_str()).collect();
        let (matched, doomed) = match_by_uid(tree, &job_nodes, KEY_JOB_UID, &wanted);
        changed |= delete_all(tree, &doomed)?;
        let mut kept = Vec::with_capacity(record.jobs.len());
        for (job, found) in record.jobs.iter().zip(matched) {
            let ticks = ticks_of(job, node, &slots.0);
            let job_node = if let Some(job_node) = found {
                changed |= align_job(&meta_of(tree, job_node)?, job, &ticks);
                job_node
            } else {
                changed = true;
                insert_job(tree, node, job, &ticks).map_err(|_| ManufacturingError::NotFound)?
            };
            kept.push(job_node);
        }
        changed |= order(tree, node, &kept)?;
        Ok(changed)
    }

    /// The slots of `fixture` by place: the first `records.len()` slot nodes rewritten, missing
    /// ones created, the rest deleted. The slot nodes in order, and whether anything changed.
    fn align_slots(
        &self,
        tree: &LoroTree,
        fixture: TreeID,
        nodes: &[TreeID],
        records: &[SlotRecord],
    ) -> Result<(Vec<TreeID>, bool), ManufacturingError> {
        let mut changed = delete_all(tree, nodes.get(records.len()..).unwrap_or_default())?;
        let mut slots = Vec::with_capacity(records.len());
        for (index, record) in records.iter().enumerate() {
            if let Some(&node) = nodes.get(index) {
                let meta = meta_of(tree, node)?;
                changed |= set_text(&meta, crate::shape_codec::KEY_SHAPE, SHAPE_FIXTURE_SLOT);
                changed |= set_text(&meta, KEY_NAME, &record.name);
                changed |= set_map(&meta, KEY_SLOT_PARAMS, &record.params);
                slots.push(node);
            } else {
                changed = true;
                slots.push(self.insert_slot(fixture, record)?);
            }
        }
        Ok((slots, changed))
    }
}

/// The ticks of `job` by node: its parts mapped to the fixture and its slots by place.
fn ticks_of(job: &JobRecord, fixture: TreeID, slots: &[TreeID]) -> Vec<FixturePartKey> {
    job.targets
        .iter()
        .filter_map(|target| {
            let part = match target.slot {
                None => fixture,
                Some(index) => *slots.get(index)?,
            };
            Some(FixturePartKey {
                node: node_id(part),
                suffix: target.suffix.clone(),
            })
        })
        .collect()
}

/// Rewrites a kept job's content from `job`: kind, name, Show, role and ticks. Output stays.
fn align_job(meta: &LoroMap, job: &JobRecord, ticks: &[FixturePartKey]) -> bool {
    let mut changed = set_text(meta, KEY_OP_KIND, &job.kind);
    changed |= set_text(meta, KEY_NAME, &job.name);
    if read_flag(meta, KEY_OP_SHOW, true) != (job.show, 0) {
        put(meta, KEY_OP_SHOW, job.show);
        changed = true;
    }
    match job.role {
        Some(role) => changed |= set_text(meta, KEY_JOB_ROLE, role.stored()),
        None if meta.get(KEY_JOB_ROLE).is_some() => {
            let _ = meta.delete(KEY_JOB_ROLE);
            changed = true;
        }
        None => {}
    }
    let mut stored: Vec<(String, bool)> = Vec::new();
    meta.for_each(|key, value| {
        if key.starts_with(TARGET_PREFIX) {
            let ticked = matches!(value, ValueOrContainer::Value(LoroValue::Bool(true)));
            stored.push((key.to_owned(), ticked));
        }
    });
    let mut wanted: Vec<String> = ticks.iter().map(target_key).collect();
    wanted.sort_unstable();
    wanted.dedup();
    let mut current: Vec<String> = stored
        .iter()
        .filter(|(_, ticked)| *ticked)
        .map(|(key, _)| key.clone())
        .collect();
    current.sort_unstable();
    if current != wanted || stored.iter().any(|(_, ticked)| !ticked) {
        for (key, _) in &stored {
            let _ = meta.delete(key);
        }
        for key in &wanted {
            put(meta, key, true);
        }
        changed = true;
    }
    changed
}

#[cfg(test)]
#[path = "manufacturing_fixture_align_tests.rs"]
mod tests;
