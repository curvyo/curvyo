//! The writes of a node's own registers: rename, hide or show, lock or unlock, and opacity
//! (`specs/0050-layers-and-objects-panel` criteria 16, 18, 20 and 26). Each is one commit and one
//! step, however many nodes it names, and writes one register per node.

use std::collections::HashSet;

use loro::{LoroValue, TreeID};

use crate::document::{Document, OBJECTS_TREE};
use crate::node_read::locked_through;
use crate::node_registers::{KEY_LOCKED, KEY_NAME, KEY_OPACITY, KEY_VISIBLE, clean_name};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::style_model::Opacity;

/// The label of a rename's commit.
pub const RENAME_NODE_COMMIT_LABEL: &str = "rename_node";
/// The label of a hide or show's commit.
pub const SET_NODE_VISIBLE_COMMIT_LABEL: &str = "set_node_visible";
/// The label of a lock or unlock's commit.
pub const SET_NODE_LOCKED_COMMIT_LABEL: &str = "set_node_locked";
/// The label of an opacity change's commit.
pub const SET_NODE_OPACITY_COMMIT_LABEL: &str = "set_node_opacity";

impl Document {
    /// The live tree ids of `ids`, each once, or [`ObjectEditError::NoSuchObject`].
    fn live_nodes(&self, ids: &[NodeId]) -> Result<Vec<TreeID>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut seen = HashSet::new();
        let nodes: Vec<TreeID> = ids
            .iter()
            .map(|&id| tree_id_of(id))
            .filter(|id| seen.insert(*id))
            .collect();
        if nodes.iter().any(|&id| !node_exists(&tree, id)) {
            return Err(ObjectEditError::NoSuchObject);
        }
        Ok(nodes)
    }

    /// Writes `value` (or removes the key for `None`) into one register of every node of `ids`,
    /// in one commit labelled `label`.
    fn write_register(
        &self,
        ids: &[NodeId],
        key: &str,
        value: Option<&LoroValue>,
        label: &str,
    ) -> Result<(), ObjectEditError> {
        let nodes = self.live_nodes(ids)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for node in nodes {
            // invariant: the node was confirmed live above, so its meta map exists.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(node).unwrap();
            // invariant: a map insert or delete on a live map does not fail.
            #[allow(clippy::unwrap_used)]
            match value {
                Some(value) => meta.insert(key, value.clone()).unwrap(),
                None => meta.delete(key).unwrap(),
            }
        }
        self.commit_with_label(label);
        Ok(())
    }

    /// Names the node `id`, in one commit labelled [`RENAME_NODE_COMMIT_LABEL`] (criterion 16).
    /// The name is cleaned first ([`crate::clean_name`]); nothing left removes the name. Allowed
    /// on a locked node.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if `id` does not exist (nothing is written).
    pub fn rename_node(&self, id: NodeId, name: &str) -> Result<(), ObjectEditError> {
        let value = clean_name(name).map(LoroValue::from);
        self.write_register(&[id], KEY_NAME, value.as_ref(), RENAME_NODE_COMMIT_LABEL)
    }

    /// Sets the own `visible` flag of every node of `ids`, in one commit labelled
    /// [`SET_NODE_VISIBLE_COMMIT_LABEL`] (criterion 18). Allowed on a locked node.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an id does not exist (nothing is written).
    pub fn set_node_visible(&self, ids: &[NodeId], visible: bool) -> Result<(), ObjectEditError> {
        self.write_register(
            ids,
            KEY_VISIBLE,
            Some(&LoroValue::Bool(visible)),
            SET_NODE_VISIBLE_COMMIT_LABEL,
        )
    }

    /// Sets the own `locked` flag of every node of `ids`, in one commit labelled
    /// [`SET_NODE_LOCKED_COMMIT_LABEL`] (criterion 20).
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an id does not exist (nothing is written).
    pub fn set_node_locked(&self, ids: &[NodeId], locked: bool) -> Result<(), ObjectEditError> {
        self.write_register(
            ids,
            KEY_LOCKED,
            Some(&LoroValue::Bool(locked)),
            SET_NODE_LOCKED_COMMIT_LABEL,
        )
    }

    /// Sets the own `opacity` of every node of `ids`, in one commit labelled
    /// [`SET_NODE_OPACITY_COMMIT_LABEL`] (criterion 26).
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an id does not exist; [`ObjectEditError::Locked`] if
    /// a node is locked by its own flag or a container's (criterion 20). Nothing is written.
    pub fn set_node_opacity(
        &self,
        ids: &[NodeId],
        opacity: Opacity,
    ) -> Result<(), ObjectEditError> {
        let nodes = self.live_nodes(ids)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if nodes.iter().any(|&node| locked_through(&tree, node)) {
            return Err(ObjectEditError::Locked);
        }
        self.write_register(
            ids,
            KEY_OPACITY,
            Some(&LoroValue::Double(opacity.get())),
            SET_NODE_OPACITY_COMMIT_LABEL,
        )
    }
}
