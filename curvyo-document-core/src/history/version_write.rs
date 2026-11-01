//! Writes a past version as a new step: applying it to the document or to one object, and cloning
//! an object's version into a new object (`specs/0042-history-branches` criteria 10, 15, 22 to 25
//! and 31). Versions are read under the checkout guard (ADR 0014 §12) and written through the
//! restore engine (`restore_plan.rs`, `restore.rs`, `revive.rs`, `merge.rs`); an apply has no
//! three-way check: it is a deliberate return, so every field takes the version's value.

use std::collections::BTreeMap;

use loro::VersionVector;

use super::checkout::Detached;
use super::error::HistoryError;
use super::lineage::{KEY_CLONE_OF, KEY_HISTORY_FLOOR, encode_clone_of, encode_node};
use super::merge::{Tally, restore_map};
use super::node_value::{Node, read_map};
use super::object_rows::{node_id, tree_id};
use super::object_timeline::EntryClass;
use super::restore::read_versions_between;
use super::restore_plan::{Snapshot, plan_return};
use super::stack_machine::Status;
use super::step_header::{KEY_OBJECT, KEY_VERSION, LABEL_CLONE_VERSION, LABEL_GO_TO_VERSION};
use super::step_id::StepRef;
use super::step_index::is_row;
use super::step_log::find_step;
use super::wipe::KEY_HISTORY_WIPED;
use crate::document::{Document, KEY_FORMAT_VERSION, OBJECTS_TREE, ROOT_MAP};
use crate::group_tree::KEY_GROUP;
use crate::objects::renumber_anchors;
use crate::path_model::{AnchorId, NodeId};
use crate::shape_codec::read_shape_tag;
use crate::subpath_codec::total_anchor_count;

/// What applying a version did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToReport {
    /// The step written; `None` when the document or the object already matched the version.
    pub step: Option<StepRef>,
    /// How many later steps of other authors it went back over (criterion 15).
    pub others_reverted: usize,
}

/// An object's meta map without the keys that belong to the object and not to a version: its
/// lineage and its floor stay as they are.
fn own_fields(mut meta: BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    meta.remove(KEY_CLONE_OF);
    meta.remove(KEY_HISTORY_FLOOR);
    meta
}

/// The root keys an apply leaves as they are: the history wipe mark and the format version
/// (ADR 0014 §8, amendment of 2026-10-14).
fn settings(mut root: BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    root.remove(KEY_HISTORY_WIPED);
    root.remove(KEY_FORMAT_VERSION);
    root
}

/// The snapshot with the lineage keys and the kept root keys taken out.
fn comparable(snapshot: &mut Snapshot) {
    for node in snapshot.nodes.values_mut() {
        node.meta = own_fields(std::mem::take(&mut node.meta));
    }
    snapshot.root = settings(std::mem::take(&mut snapshot.root));
}

impl Document {
    /// Applies the version right after `version` (`specs/0042-history-branches` criterion 10):
    /// to the whole document, or with `object` to that object only, as one step "Go to version"
    /// that Ctrl+Z takes back. It is one run of the restore engine with the version as "before"
    /// and now as "after", over every object a step since the version touched (ADR 0014 §8):
    /// objects created since are removed, objects deleted since come back with their id, fields,
    /// parents, order and document settings go back. `source` is the object an inherited row of
    /// `object` belongs to: its fields at the version stand for the object's.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if `version` is not in the log, [`HistoryError::NoSuchObject`]
    /// if `object` does not exist, [`HistoryError::Unreadable`] if a version cannot be checked
    /// out, [`HistoryError::Revive`] if a deleted object could not
    /// come back (then nothing was written).
    pub fn go_to_version(
        &self,
        version: StepRef,
        object: Option<NodeId>,
        source: Option<NodeId>,
    ) -> Result<GoToReport, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, version).ok_or(HistoryError::NoSuchStep)?;
        if let Some(id) = object
            && !super::restore_plan::alive(doc, tree_id(id))
        {
            return Err(HistoryError::NoSuchObject);
        }
        let then = log.after();
        let since = doc.frontiers_to_vv(&then).ok_or(HistoryError::Unreadable)?;
        let mut touched = {
            let mut index = self.container_index.borrow_mut();
            index.extend(doc);
            let changes = doc
                .export_json_updates_without_peer_compression(&since, &doc.oplog_vv())
                .changes;
            index.touched_all(doc, &changes)
        };
        if let Some(id) = object {
            touched = touched.only(tree_id(id));
        }
        let (mut before, mut after) =
            read_versions_between(doc, &then, &doc.oplog_frontiers(), &touched)?;
        if let (Some(id), Some(source)) = (object, source.filter(|s| Some(*s) != object)) {
            // An inherited row: the source's fields at the version, in the object's place now.
            let at = Detached::at(doc, &then)?.node(tree_id(source));
            let place = after.nodes.get(&tree_id(id)).cloned();
            if let Some(mut node) = place {
                node.meta = at.meta;
                before.nodes.insert(tree_id(id), node);
            }
        }
        // A deleted object reads as empty at a version; it comes back with the fields it had when
        // it went, so those are what "now" holds for it.
        let tree = doc.get_tree(OBJECTS_TREE);
        for (id, node) in &mut after.nodes {
            if node.meta.is_empty()
                && let Ok(meta) = tree.get_meta(*id)
            {
                node.meta = read_map(&meta);
            }
        }
        comparable(&mut before);
        comparable(&mut after);
        let plan = plan_return(doc, &touched, &before, &after);
        let root = touched.root && before.root != after.root;
        if !plan.writes() && !root {
            return Ok(GoToReport {
                step: None,
                others_reverted: 0,
            });
        }
        let others_reverted = self.later_steps_of_others(object, &since);
        let mut extra = vec![(KEY_VERSION, version.encode())];
        if let Some(id) = object {
            extra.push((KEY_OBJECT, encode_node(tree_id(id))));
        }
        let header = self.next_header(LABEL_GO_TO_VERSION, &extra);
        let first = doc.oplog_vv().get(&doc.peer_id()).copied().unwrap_or(0);
        let revived = self.revive_planned(&plan, &before, &after, &header)?;
        self.apply_plan(&plan, &after, &before);
        if root {
            let mut tally = Tally::default();
            restore_map(
                &doc.get_map(ROOT_MAP),
                &after.root,
                &before.root,
                &mut tally,
            );
        }
        let wrote = revived || doc.get_pending_txn_len() > 0;
        self.commit_with_header(&header);
        let step = wrote.then(|| {
            let step = StepRef::new(doc.peer_id(), first);
            self.commit_state
                .register_own(step, LABEL_GO_TO_VERSION, None);
            step
        });
        Ok(GoToReport {
            step,
            others_reverted,
        })
    }

    /// Creates a new object with the state `source` had right after `version` (or right before
    /// it, for a step that deleted it), on top of the stack, as one step "Clone version"
    /// (criteria 22 and 24). A path's anchors get ids from `mint`; the clone's lineage names
    /// `source` at that version (criterion 23). Returns the clone.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if `version` is not in the log, [`HistoryError::NoSuchObject`]
    /// if the object exists at neither version, [`HistoryError::Group`] for a group (criterion
    /// 25), [`HistoryError::Unreadable`] if the version cannot be checked out. Nothing is written
    /// when it fails.
    ///
    /// # Panics
    /// Does not panic in practice: after the checks every write goes to the node it creates.
    pub fn clone_version(
        &self,
        source: NodeId,
        version: StepRef,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> Result<NodeId, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, version).ok_or(HistoryError::NoSuchStep)?;
        let src = tree_id(source);
        let (at, read_at) = {
            let guard = Detached::at(doc, &log.after())?;
            let after = guard.node(src);
            if after.meta.is_empty() {
                // The step deleted it: the clone is the object just before (criterion 24).
                guard.goto(&log.before)?;
                (guard.node(src), log.before.clone())
            } else {
                (after, log.after())
            }
        };
        if at.meta.is_empty() {
            return Err(HistoryError::NoSuchObject);
        }
        if at.meta.get(KEY_GROUP) == Some(&Node::Value(loro::LoroValue::Bool(true))) {
            return Err(HistoryError::Group);
        }
        // Everything that can fail is behind this point: from here on every write goes to the node
        // created here, so a refusal never leaves a half-made clone in the pending changes.
        let tree = doc.get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root cannot fail, and reading the meta map of a
        // node just created cannot fail (as in `objects::copy_subtree`).
        #[allow(clippy::unwrap_used)]
        let created = tree.create(loro::TreeParentId::Root).unwrap();
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(created).unwrap();
        let mut tally = Tally::default();
        restore_map(&meta, &BTreeMap::new(), &own_fields(at.meta), &mut tally);
        if read_shape_tag(&meta).is_none() {
            let ids: Vec<AnchorId> = (0..total_anchor_count(&meta)).map(|_| mint()).collect();
            renumber_anchors(&meta, &ids);
        }
        // invariant: the key is new in the meta map just created and the value is a plain string.
        #[allow(clippy::unwrap_used)]
        meta.insert(KEY_CLONE_OF, encode_clone_of(src, &read_at))
            .unwrap();
        let extra = [
            (KEY_OBJECT, encode_node(src)),
            (KEY_VERSION, version.encode()),
        ];
        // A copied version may hold a dimension.
        self.forget_dimension_scan();
        self.commit_step(LABEL_CLONE_VERSION, &extra);
        Ok(node_id(created))
    }

    /// The steps of other authors, in effect, that are not in the past of `since`: on `object`
    /// only, or on the whole document.
    fn later_steps_of_others(&self, object: Option<NodeId>, since: &VersionVector) -> usize {
        self.with_reader(|reader, me| {
            let later = |step: StepRef| {
                step.peer() != me
                    && reader.steps.status(step) == Status::Applied
                    && reader
                        .steps
                        .get(step)
                        .is_some_and(|log| since.get(&step.peer()).is_none_or(|end| *end < log.end))
            };
            match object {
                Some(id) => reader
                    .timeline(tree_id(id))
                    .entries
                    .iter()
                    .filter(|entry| entry.class == EntryClass::Do && later(entry.step))
                    .count(),
                None => reader
                    .steps
                    .steps()
                    .filter(|log| is_row(log) && later(log.step))
                    .count(),
            }
        })
    }
}
