//! The commands that write laser operation nodes under a manufacturing (ADR 0046 §1 to §3,
//! `specs/0101-laser-operations`); every rule of what runs lives in the laser extension.

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{KEY_PROCESS, is_manufacturing_node, string};
use crate::node_registers::{KEY_NAME, clean_name};
use crate::operation_codec::{
    CUTS_LAST_FIELD, KEY_OP_KIND, KEY_OP_MARGIN, KEY_OP_OUTPUT, KEY_OP_SHOW, OperationSnapshot,
    SHAPE_OPERATION, member_key, path_is_live, read_operation,
};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;
use crate::units::Length;

/// The label of Add operation with no selection (criterion 2).
pub const ADD_OPERATION_COMMIT_LABEL: &str = "add_operation";
/// The label of an assignment, also of a kind press that creates an operation for the selection
/// (criterion 8).
pub const ASSIGN_TO_OPERATION_COMMIT_LABEL: &str = "assign_to_operation";
/// The label of a removal of items from one operation (criteria 8, 9).
pub const REMOVE_FROM_OPERATION_COMMIT_LABEL: &str = "remove_from_operation";
/// The label of a rename (criterion 3).
pub const RENAME_OPERATION_COMMIT_LABEL: &str = "rename_operation";
/// The label of a colour pick (criterion 3).
/// The label of the Show switch (criterion 15).
pub const SET_OPERATION_SHOW_COMMIT_LABEL: &str = "set_operation_show";
/// The label of the Output switch (criterion 16).
pub const SET_OPERATION_OUTPUT_COMMIT_LABEL: &str = "set_operation_output";
/// The label of the Cut-out margin field (criterion 23).
pub const SET_OPERATION_MARGIN_COMMIT_LABEL: &str = "set_operation_margin";
/// The label of Delete operation (criterion 4).
pub const DELETE_OPERATION_COMMIT_LABEL: &str = "delete_operation";
/// The label of a reorder (criteria 5, 6).
pub const MOVE_OPERATION_COMMIT_LABEL: &str = "move_operation";
/// The label of Cuts last switched on (criterion 6).
pub const CUTS_LAST_ON_COMMIT_LABEL: &str = "cuts_last_on";
/// The label of Cuts last switched off (criterion 6).
pub const CUTS_LAST_OFF_COMMIT_LABEL: &str = "cuts_last_off";
/// What a new operation stores. Kind and colour are opaque texts (ADR 0033 §3).
#[derive(Debug, Clone, Copy)]
pub struct NewOperation<'a> {
    /// The kind text.
    pub kind: &'a str,
    /// The name.
    pub name: &'a str,
}

/// The two switches of an operation row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationSwitch {
    /// Show: view only.
    Show,
    /// Output: what runs.
    Output,
}

/// A refused operation command; it writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OperationError {
    /// The id names no live manufacturing.
    #[error("no such manufacturing")]
    NoSuchManufacturing,
    /// The id names no live operation.
    #[error("no such operation")]
    NoSuchOperation,
    /// A target is not a live node.
    #[error("no such item")]
    NoSuchTarget,
    /// A margin that is not a finite number of at least 0 mm.
    #[error("the margin is zero or a positive value")]
    BadMargin,
    /// A test grid row: its one membership and its settings are written only by the grid
    /// commands (ADR 0054 §4, `specs/0028` criterion 6).
    #[error("a test grid row takes no items and no settings")]
    GridRow,
}

/// The operation nodes under `manufacturing`, in list (tree) order.
pub(crate) fn operation_nodes(tree: &LoroTree, manufacturing: TreeID) -> Vec<TreeID> {
    tree.children(TreeParentId::Node(manufacturing))
        .unwrap_or_default()
        .into_iter()
        .filter(|&child| is_operation_node(tree, child))
        .collect()
}

/// The tree ids of a member path.
fn tree_path(path: &[NodeId]) -> Vec<TreeID> {
    path.iter().map(|&node| tree_id_of(node)).collect()
}

/// Removes the membership key `key` of `op`, if it is there (uncommitted).
pub(super) fn delete_key(tree: &LoroTree, op: TreeID, key: &str) {
    if !node_exists(tree, op) {
        return;
    }
    if let Ok(meta) = tree.get_meta(op)
        && meta.get(key).is_some()
    {
        let _ = meta.delete(key);
    }
}

fn is_operation_node(tree: &LoroTree, node: TreeID) -> bool {
    tree.get_meta(node)
        .is_ok_and(|meta| string(&meta, KEY_SHAPE).as_deref() == Some(SHAPE_OPERATION))
}

/// Puts `node` at `index` of the operation list `others` (the list without `node`), or last.
pub(crate) fn place_in_list(tree: &LoroTree, node: TreeID, others: &[TreeID], index: usize) {
    let moved = match (others.get(index), others.last()) {
        (Some(&before), _) => tree.mov_before(node, before),
        (None, Some(&after)) => tree.mov_after(node, after),
        (None, None) => Ok(()),
    };
    // invariant: both nodes are live siblings under one manufacturing.
    #[allow(clippy::unwrap_used)]
    moved.unwrap();
}

impl Document {
    fn manufacturing_node(&self, id: NodeId) -> Result<TreeID, OperationError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        (node_exists(&tree, node) && is_manufacturing_node(&tree, node))
            .then_some(node)
            .ok_or(OperationError::NoSuchManufacturing)
    }

    pub(crate) fn operation_meta(&self, id: NodeId) -> Result<LoroMap, OperationError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let in_manufacturing = matches!(
            tree.parent(node),
            Some(TreeParentId::Node(parent)) if is_manufacturing_node(&tree, parent)
        );
        if !(node_exists(&tree, node) && in_manufacturing && is_operation_node(&tree, node)) {
            return Err(OperationError::NoSuchOperation);
        }
        tree.get_meta(node)
            .map_err(|_| OperationError::NoSuchOperation)
    }

    /// The operations of `manufacturing`, in list order (the run order before Cuts last).
    ///
    /// # Errors
    /// [`OperationError::NoSuchManufacturing`] when `manufacturing` is not a live manufacturing.
    pub fn operations(
        &self,
        manufacturing: NodeId,
    ) -> Result<Vec<OperationSnapshot>, OperationError> {
        let node = self.manufacturing_node(manufacturing)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        Ok(operation_nodes(&tree, node)
            .into_iter()
            .filter_map(|op| Some(read_operation(&tree, op, &tree.get_meta(op).ok()?)))
            .collect())
    }

    /// Adds an operation at `index` of the list (or last) with Show and Output on and the
    /// `members` assigned, in one history step: "add operation" with no member, else "assign".
    ///
    /// # Errors
    /// [`OperationError::NoSuchManufacturing`]; [`OperationError::NoSuchTarget`] when a member
    /// is not a live node or is a path of more than one node (ADR 0056 §2).
    ///
    /// # Panics
    /// Does not panic in practice: the meta map of a node just created exists.
    pub fn add_operation(
        &self,
        manufacturing: NodeId,
        new: &NewOperation<'_>,
        index: usize,
        members: &[Vec<NodeId>],
    ) -> Result<NodeId, OperationError> {
        let parent = self.manufacturing_node(manufacturing)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if members
            .iter()
            .any(|m| m.len() != 1 || !path_is_live(&tree, &tree_path(m)))
        {
            return Err(OperationError::NoSuchTarget);
        }
        let others = operation_nodes(&tree, parent);
        tree.enable_fractional_index(0);
        let node = tree
            .create(TreeParentId::Node(parent))
            .map_err(|_| OperationError::NoSuchManufacturing)?;
        place_in_list(&tree, node, &others, index);
        // invariant: the node was just created, so its meta map exists; plain values only.
        #[allow(clippy::unwrap_used)]
        {
            let meta = tree.get_meta(node).unwrap();
            meta.insert(KEY_SHAPE, SHAPE_OPERATION).unwrap();
            meta.insert(KEY_OP_KIND, new.kind).unwrap();
            meta.insert(KEY_NAME, new.name).unwrap();
            for member in members {
                meta.insert(&member_key(&tree_path(member)), true).unwrap();
                self.member_keys_may_exist.set(true);
            }
        }
        self.commit_with_label(if members.is_empty() {
            ADD_OPERATION_COMMIT_LABEL
        } else {
            ASSIGN_TO_OPERATION_COMMIT_LABEL
        });
        Ok(node_id(node))
    }

    /// Renames the operation `id`. `Ok(false)` and nothing written for an empty name (it
    /// reverts, criterion 3) or the same name.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn rename_operation(&self, id: NodeId, name: &str) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        let Some(name) = clean_name(name) else {
            return Ok(false);
        };
        if string(&meta, KEY_NAME).as_deref() == Some(name.as_str()) {
            return Ok(false);
        }
        let label = if crate::operation_codec::grid_switch(&meta) == Some(true) {
            crate::test_grid_commands::RENAME_TEST_GRID_COMMIT_LABEL
        } else {
            RENAME_OPERATION_COMMIT_LABEL
        };
        self.write_and_commit(&meta, KEY_NAME, name.as_str(), label)
    }

    /// Sets the colour text of the operation `id`. `Ok(false)` when it already is.
    ///
    /// Sets the Show or Output switch of the operation `id`. `Ok(false)` when it already reads so.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn set_operation_switch(
        &self,
        id: NodeId,
        which: OperationSwitch,
        on: bool,
    ) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        let (key, label) = match which {
            OperationSwitch::Show => (KEY_OP_SHOW, SET_OPERATION_SHOW_COMMIT_LABEL),
            OperationSwitch::Output => (KEY_OP_OUTPUT, SET_OPERATION_OUTPUT_COMMIT_LABEL),
        };
        let snapshot = self.read_operation_at(id, &meta);
        let now = match which {
            OperationSwitch::Show => snapshot.show,
            OperationSwitch::Output => snapshot.output,
        };
        if now == on {
            return Ok(false);
        }
        self.write_and_commit(&meta, key, on, label)
    }

    /// Sets the Cut-out margin of the operation `id`. `Ok(false)` when it already is.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`]; [`OperationError::BadMargin`] for a value that is not
    /// finite or below 0.
    pub fn set_operation_margin(&self, id: NodeId, margin: Length) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        let mm = margin.as_mm();
        if !mm.is_finite() || mm < 0.0 {
            return Err(OperationError::BadMargin);
        }
        #[allow(clippy::float_cmp)] // exact register equality avoids an empty step
        if self.read_operation_at(id, &meta).margin.as_mm() == mm {
            return Ok(false);
        }
        self.write_and_commit(&meta, KEY_OP_MARGIN, mm, SET_OPERATION_MARGIN_COMMIT_LABEL)
    }

    /// Deletes the operation `id` with every membership it holds; no drawing object is touched
    /// (criterion 4; a test grid row keeps its items, which then run nothing). One history step;
    /// undo revives the node with its keys (ADR 0014).
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn delete_operation(&self, id: NodeId) -> Result<(), OperationError> {
        let meta = self.operation_meta(id)?;
        let label = if crate::operation_codec::grid_switch(&meta) == Some(true) {
            crate::test_grid_commands::DELETE_TEST_GRID_ROW_COMMIT_LABEL
        } else {
            DELETE_OPERATION_COMMIT_LABEL
        };
        self.loro()
            .get_tree(OBJECTS_TREE)
            .delete(tree_id_of(id))
            .map_err(|_| OperationError::NoSuchOperation)?;
        self.commit_with_label(label);
        Ok(())
    }

    /// Adds (`member` true) or removes the `targets` of the operation `id`, in one history step
    /// "Assign to" or "Remove from"; a group assignment and its untick (ADR 0056 §1) are one such
    /// call with the group's and its members' keys. `Ok(false)` and nothing written when no key
    /// changes.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`]; [`OperationError::NoSuchTarget`] when an added
    /// target is not a live node or is a path of more than one node (a clone member, ADR 0056
    /// §2); [`OperationError::GridRow`] for a test grid row.
    pub fn set_operation_members(
        &self,
        id: NodeId,
        targets: &[Vec<NodeId>],
        member: bool,
    ) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        if crate::operation_codec::grid_switch(&meta) != Some(false) {
            return Err(OperationError::GridRow);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // A clone-member path key is legacy data only (ADR 0056 §2): never written again.
        if member
            && targets
                .iter()
                .any(|t| t.len() != 1 || !path_is_live(&tree, &tree_path(t)))
        {
            return Err(OperationError::NoSuchTarget);
        }
        let mut changed = false;
        for target in targets {
            let key = member_key(&tree_path(target));
            let present = meta.get(&key).is_some();
            let written = match (member, present) {
                (true, false) => {
                    self.member_keys_may_exist.set(true);
                    meta.insert(&key, true)
                }
                (false, true) => meta.delete(&key),
                _ => continue,
            };
            written.map_err(|_| OperationError::NoSuchOperation)?;
            changed = true;
        }
        if changed {
            self.commit_with_label(if member {
                ASSIGN_TO_OPERATION_COMMIT_LABEL
            } else {
                REMOVE_FROM_OPERATION_COMMIT_LABEL
            });
        }
        Ok(changed)
    }

    /// Moves the operation `id` to `index` of the list (or last), one tree move in one history
    /// step. `Ok(false)` when it already sits there.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn move_operation(&self, id: NodeId, index: usize) -> Result<bool, OperationError> {
        self.operation_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let Some(TreeParentId::Node(parent)) = tree.parent(node) else {
            return Err(OperationError::NoSuchOperation);
        };
        let list = operation_nodes(&tree, parent);
        let others: Vec<TreeID> = list.iter().copied().filter(|&n| n != node).collect();
        if list.iter().position(|&n| n == node) == Some(index.min(others.len())) {
            return Ok(false);
        }
        tree.enable_fractional_index(0);
        place_in_list(&tree, node, &others, index);
        self.commit_with_label(MOVE_OPERATION_COMMIT_LABEL);
        Ok(true)
    }

    /// Writes the Cuts last switch `x.<process>.cuts_last` of `manufacturing` and puts its
    /// operations in `order` (operation ids, list order; others keep their place after them), in
    /// one history step (criterion 6). `Ok(false)` when neither changes.
    ///
    /// # Errors
    /// [`OperationError::NoSuchManufacturing`]; [`OperationError::NoSuchOperation`] when `order`
    /// names a node that is not one of its operations.
    pub fn set_cuts_last(
        &self,
        manufacturing: NodeId,
        on: bool,
        order: &[NodeId],
    ) -> Result<bool, OperationError> {
        let parent = self.manufacturing_node(manufacturing)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree
            .get_meta(parent)
            .map_err(|_| OperationError::NoSuchManufacturing)?;
        let list = operation_nodes(&tree, parent);
        let wanted: Vec<TreeID> = order.iter().map(|&id| tree_id_of(id)).collect();
        if wanted.iter().any(|node| !list.contains(node)) {
            return Err(OperationError::NoSuchOperation);
        }
        let key = format!(
            "x.{}.{CUTS_LAST_FIELD}",
            string(&meta, KEY_PROCESS).unwrap_or_default()
        );
        let stored = match meta.get(&key) {
            Some(loro::ValueOrContainer::Value(loro::LoroValue::Bool(flag))) => Some(flag),
            _ => None,
        };
        let reorder = list.iter().take(wanted.len()).ne(wanted.iter());
        if stored == Some(on) && !reorder {
            return Ok(false);
        }
        if reorder {
            tree.enable_fractional_index(0);
            for (at, &node) in wanted.iter().enumerate() {
                let others: Vec<TreeID> = operation_nodes(&tree, parent)
                    .into_iter()
                    .filter(|&n| n != node)
                    .collect();
                place_in_list(&tree, node, &others, at);
            }
        }
        meta.insert(&key, on)
            .map_err(|_| OperationError::NoSuchManufacturing)?;
        self.commit_with_label(if on {
            CUTS_LAST_ON_COMMIT_LABEL
        } else {
            CUTS_LAST_OFF_COMMIT_LABEL
        });
        Ok(true)
    }

    fn read_operation_at(&self, id: NodeId, meta: &LoroMap) -> OperationSnapshot {
        read_operation(&self.loro().get_tree(OBJECTS_TREE), tree_id_of(id), meta)
    }

    fn write_and_commit(
        &self,
        meta: &LoroMap,
        key: &str,
        value: impl Into<loro::LoroValue>,
        label: &str,
    ) -> Result<bool, OperationError> {
        meta.insert(key, value)
            .map_err(|_| OperationError::NoSuchOperation)?;
        self.commit_with_label(label);
        Ok(true)
    }
}
