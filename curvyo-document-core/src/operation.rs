//! The commands that write laser operation nodes under a manufacturing (ADR 0046 §1 to §3,
//! `specs/0101-laser-operations`); every rule of what runs lives in the laser extension.

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{group_parent, node_id};
use crate::manufacturing_codec::{KEY_PROCESS, is_manufacturing_node, manufacturing_nodes, string};
use crate::node_registers::{KEY_NAME, clean_name};
use crate::operation_codec::{
    CUTS_LAST_FIELD, KEY_OP_KIND, KEY_OP_MARGIN, KEY_OP_OUTPUT, KEY_OP_SHOW, MEMBER_PREFIX,
    OperationSnapshot, SHAPE_OPERATION, decode_path, member_key, path_is_live, read_operation,
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
}

/// The operation nodes under `manufacturing`, in list (tree) order.
fn operation_nodes(tree: &LoroTree, manufacturing: TreeID) -> Vec<TreeID> {
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
fn delete_key(tree: &LoroTree, op: TreeID, key: &str) {
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
fn place_in_list(tree: &LoroTree, node: TreeID, others: &[TreeID], index: usize) {
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

    fn operation_meta(&self, id: NodeId) -> Result<LoroMap, OperationError> {
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
    /// is not a live node.
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
        if members.iter().any(|m| !path_is_live(&tree, &tree_path(m))) {
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
        self.write_and_commit(
            &meta,
            KEY_NAME,
            name.as_str(),
            RENAME_OPERATION_COMMIT_LABEL,
        )
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
    /// (criterion 4). One history step; undo revives the node with its keys (ADR 0014).
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn delete_operation(&self, id: NodeId) -> Result<(), OperationError> {
        self.operation_meta(id)?;
        self.loro()
            .get_tree(OBJECTS_TREE)
            .delete(tree_id_of(id))
            .map_err(|_| OperationError::NoSuchOperation)?;
        self.commit_with_label(DELETE_OPERATION_COMMIT_LABEL);
        Ok(())
    }

    /// Adds (`member` true) or removes the `targets` of the operation `id`, in one history step.
    /// `Ok(false)` and nothing written when no key changes.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`]; [`OperationError::NoSuchTarget`] when an added
    /// target is not a live node.
    pub fn set_operation_members(
        &self,
        id: NodeId,
        targets: &[Vec<NodeId>],
        member: bool,
    ) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if member && targets.iter().any(|t| !path_is_live(&tree, &tree_path(t))) {
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

    /// `id` and the nodes that hold it, nearest first, up to the top level of the drawing; a node
    /// of a manufacturing (a placement) stops there. Empty when `id` is not live. The chain the
    /// nearest rule walks (ADR 0046 §5, §6).
    #[must_use]
    pub fn node_chain(&self, id: NodeId) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut node = tree_id_of(id);
        if !node_exists(&tree, node) || is_manufacturing_node(&tree, node) {
            return Vec::new();
        }
        let mut chain = vec![id];
        while let Some(parent) = group_parent(&tree, node) {
            if is_manufacturing_node(&tree, parent) || chain.len() > crate::MAX_GROUP_DEPTH * 4 {
                break;
            }
            chain.push(node_id(parent));
            node = parent;
        }
        chain
    }

    /// Removes, uncommitted, every membership key whose path names a node that is no longer live
    /// (ADR 0046 §3, Frage 169 A). Called by every local step (`commit_step`), so a key goes in
    /// the step that deletes its node, whichever command deleted it, and undo brings it back with
    /// the node. Keys that do not read are left alone. A key left dead by a peer's merge goes at
    /// the next local step; reads ignore it until then.
    pub(crate) fn sweep_dead_member_keys(&self) {
        if !self.member_keys_may_exist.get() {
            return;
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let entries = self.member_entries();
        if entries.is_empty() {
            // Every key is gone: the next commits pay nothing again until a key is written.
            self.member_keys_may_exist.set(false);
            return;
        }
        for (op, key, path) in entries {
            if !path_is_live(&tree, &path) {
                delete_key(&tree, op, &key);
            }
        }
    }

    /// Removes, uncommitted, every membership key whose path `doomed` accepts. Used by the clone
    /// repairs, where a clone stops being a clone and its members' keys go (ADR 0046 §2 (c)).
    pub(crate) fn remove_member_keys_where(&self, doomed: impl Fn(&[TreeID]) -> bool) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for (op, key, path) in self.member_entries() {
            if doomed(&path) {
                delete_key(&tree, op, &key);
            }
        }
    }

    /// Removes, uncommitted, the member keys (paths of more than one node) of the clones `clones`,
    /// which stop being clones in the caller's step (ADR 0046 §2 (c)).
    pub(crate) fn remove_member_keys_of_clones(&self, clones: &[TreeID]) {
        self.remove_member_keys_where(|path| path.len() > 1 && clones.contains(&path[0]));
    }

    /// Every membership key of every operation whose text reads, with its operation and path.
    /// Keys that do not read are left out: they are kept in the file and never touched.
    pub(crate) fn member_entries(&self) -> Vec<(TreeID, String, Vec<TreeID>)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut out = Vec::new();
        for manufacturing in manufacturing_nodes(&tree) {
            for op in operation_nodes(&tree, manufacturing) {
                let Ok(meta) = tree.get_meta(op) else {
                    continue;
                };
                meta.for_each(|key, _| {
                    if let Some(path) = key.strip_prefix(MEMBER_PREFIX).and_then(decode_path) {
                        out.push((op, key.to_owned(), path));
                    }
                });
            }
        }
        out
    }

    /// The path of `member` through the clone `clone`: the clone's id, then the nodes between
    /// its original and `member` (the groups and `member` itself). `None` when `clone` is not a
    /// clone, or `member` is not below its original (ADR 0046 §2, Frage 167 B).
    #[must_use]
    pub fn clone_member_path(&self, clone: NodeId, member: NodeId) -> Option<Vec<NodeId>> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let links = self.clone_links();
        let (origin, _) = links.get(&clone)?;
        let mut below = Vec::new();
        let mut node = tree_id_of(member);
        if !node_exists(&tree, node) || node_id(node) == *origin {
            return None;
        }
        below.push(member);
        while let Some(parent) = group_parent(&tree, node) {
            if node_id(parent) == *origin {
                let mut path = vec![clone];
                path.extend(below.into_iter().rev());
                return Some(path);
            }
            below.push(node_id(parent));
            node = parent;
        }
        None
    }

    /// The membership keys a leaf is looked up by, nearest first (ADR 0046 §7, §8): the prefixes
    /// of `path` longest first (for a clone member, the member, the groups between it and the
    /// clone, the clone), then the groups above the first node. `path` is one id for an own
    /// item.
    #[must_use]
    pub fn key_chain(&self, path: &[NodeId]) -> Vec<Vec<NodeId>> {
        let mut chain: Vec<Vec<NodeId>> = (1..=path.len())
            .rev()
            .map(|len| path[..len].to_vec())
            .collect();
        if let Some(&first) = path.first() {
            chain.extend(
                self.node_chain(first)
                    .into_iter()
                    .skip(1)
                    .map(|above| vec![above]),
            );
        }
        chain
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

#[cfg(test)]
mod sweep_flag_tests {
    use crate::{
        Document, DocumentSize, NewOperation, NewOutputPage, Point, RectBounds, pack, unpack,
    };

    fn square(d: &Document, x: f64) -> crate::NodeId {
        d.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 5.0, 5.0),
        ))
    }

    /// `0101` ADR 0046 §3, performance: a document without membership keys skips the sweep of
    /// every commit (a template load commits once per node); one with keys keeps it on, and a
    /// reopened file reads its keys back.
    #[test]
    fn the_sweep_is_off_until_a_membership_key_exists_and_reads_one_from_a_file() {
        let d = Document::new(1);
        square(&d, 0.0);
        assert!(
            !d.member_keys_may_exist.get(),
            "no key: the sweep stays off"
        );

        let laser = d
            .add_output_page(NewOutputPage {
                process: "curvyo.laser",
                label: "Laser",
                profile: None,
                work_area: DocumentSize::from_mm(400.0, 300.0),
            })
            .unwrap();
        let m = laser.kind.manufacturing().unwrap();
        let rect = square(&d, 10.0);
        d.add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[vec![rect]],
        )
        .unwrap();
        assert!(
            d.member_keys_may_exist.get(),
            "a key was written: the sweep is on"
        );

        let back = unpack(2, &pack(&d, "flag").unwrap()).unwrap();
        assert!(
            back.member_keys_may_exist.get(),
            "the keys of a file are read at open"
        );
        let plain = Document::new(3);
        square(&plain, 0.0);
        let plain_back = unpack(4, &pack(&plain, "flag").unwrap()).unwrap();
        assert!(!plain_back.member_keys_may_exist.get());
    }
}
