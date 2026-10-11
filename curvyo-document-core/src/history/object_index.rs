//! Which steps touched which object (`specs/0041-object-history` criteria 21, 22, 35, 36).
//!
//! The index is derived from the operation log and never stored. It is built in one pass over the
//! log on the first request and then extended by the operations of each commit or import since:
//! a tree operation names its target, every other operation names a container that
//! [`ContainerIndex`] maps to an object, so deleted objects and their anchors resolve too.

use std::collections::HashMap;

use loro::{JsonOpContent, JsonTreeOp, LoroDoc, TreeID, VersionVector};

use super::step_id::StepRef;
use super::step_index::StepIndex;
use super::touched::{ContainerIndex, Owner};
use crate::document::Document;

/// The steps of every object, in the order the log was read.
#[derive(Debug, Default)]
pub(crate) struct ObjectIndex {
    indexed: VersionVector,
    lists: HashMap<TreeID, Vec<StepRef>>,
    built: bool,
}

impl ObjectIndex {
    /// Reads the operations committed or imported since the last call. `steps` and `containers`
    /// must already cover the same operations.
    pub(crate) fn extend(&mut self, doc: &LoroDoc, steps: &StepIndex, containers: &ContainerIndex) {
        self.built = true;
        let end = doc.oplog_vv();
        if end == self.indexed {
            return;
        }
        let updates = doc.export_json_updates_without_peer_compression(&self.indexed, &end);
        for change in &updates.changes {
            let peer = change.id.peer;
            for op in &change.ops {
                let object = match &op.content {
                    JsonOpContent::Tree(
                        JsonTreeOp::Create { target, .. }
                        | JsonTreeOp::Move { target, .. }
                        | JsonTreeOp::Delete { target },
                    ) => Some(*target),
                    _ => match containers.owner(doc, &op.container) {
                        Some(Owner::Node(id)) => Some(id),
                        _ => None,
                    },
                };
                let (Some(object), Some(step)) = (object, steps.step_at(peer, op.counter)) else {
                    continue;
                };
                let list = self.lists.entry(object).or_default();
                if list.last() != Some(&step) {
                    list.push(step);
                }
            }
        }
        self.indexed = end;
    }

    /// Whether the first pass has run.
    pub(crate) fn is_built(&self) -> bool {
        self.built
    }

    /// The steps that touched `object`, in no particular order and possibly with repeats.
    pub(crate) fn steps_of(&self, object: TreeID) -> &[StepRef] {
        self.lists.get(&object).map_or(&[], Vec::as_slice)
    }
}

impl Document {
    /// Brings the step, container and object indexes up to the log. Commits what is pending.
    pub(crate) fn sync_object_index(&self) {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let mut containers = self.container_index.borrow_mut();
        containers.extend(doc);
        let mut steps = self.step_index.borrow_mut();
        steps.extend(doc);
        steps.ensure_shown();
        self.object_index
            .borrow_mut()
            .extend(doc, &steps, &containers);
    }

    /// Whether the per-object index has been built since the document was opened. The editor
    /// announces the long-operation state before the first object undo when it has not.
    #[must_use]
    pub fn object_index_is_built(&self) -> bool {
        self.object_index.borrow().is_built()
    }
}
