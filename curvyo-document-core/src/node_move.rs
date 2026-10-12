//! Moving nodes to another place in the tree: among their siblings, into a container or out of it
//! (`specs/0050-layers-and-objects-panel` criteria 29 to 33, `adrs.md` decision 8). One commit of
//! tree moves, which keeps every id, register and link; the rules are checked before the first
//! write.

use std::collections::HashSet;

use loro::{LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{
    MAX_GROUP_DEPTH, children_of, group_parent, groups_above, groups_in, is_group_node,
    is_layer_node,
};
use crate::groups::remove_emptied_groups;
use crate::node_read::locked_through;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of a move's commit.
pub const MOVE_NODES_COMMIT_LABEL: &str = "move_nodes";

/// Where moved nodes land, in stacking terms (front is drawn on top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveTarget {
    /// Directly in front of this node, under its parent.
    Above(NodeId),
    /// Directly behind this node, under its parent.
    Below(NodeId),
    /// As the front-most children of this container (`None` is the top level).
    FrontOf(Option<NodeId>),
    /// As the back-most children of this container (`None` is the top level).
    BackOf(Option<NodeId>),
}

/// A move: the nodes, which keep their relative stacking order, and where they go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlan {
    /// The nodes to move, in any order.
    pub nodes: Vec<NodeId>,
    /// Where they land.
    pub target: MoveTarget,
}

/// Why a move is refused (criterion 30). Nothing is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoveRefusal {
    /// The plan names no node.
    #[error("nothing to move")]
    Nothing,
    /// A node or the target does not exist.
    #[error("no such node")]
    NoSuchNode,
    /// The target lies in a moved node, or is one.
    #[error("a node cannot go into itself")]
    IntoItself,
    /// A moved node is locked, by its own flag or a container's.
    #[error("a locked node cannot be moved")]
    Locked,
    /// The target container is locked, by its own flag or a container's.
    #[error("nothing can be dropped into a locked container")]
    LockedTarget,
    /// The target is a leaf, which holds no children.
    #[error("the target holds no children")]
    NotAContainer,
    /// A layer would go below the top level.
    #[error("layers stay at the top level")]
    LayerNotTopLevel,
    /// The move would put more than [`MAX_GROUP_DEPTH`] groups on one chain.
    #[error("groups can be nested 32 levels deep")]
    TooDeep,
}

/// The container a target puts the nodes into, and the sibling it is relative to.
fn target_parent(tree: &LoroTree, target: MoveTarget) -> Result<Option<TreeID>, MoveRefusal> {
    match target {
        MoveTarget::Above(sibling) | MoveTarget::Below(sibling) => {
            let sibling = tree_id_of(sibling);
            if !node_exists(tree, sibling) {
                return Err(MoveRefusal::NoSuchNode);
            }
            Ok(group_parent(tree, sibling))
        }
        MoveTarget::FrontOf(parent) | MoveTarget::BackOf(parent) => {
            let Some(parent) = parent.map(tree_id_of) else {
                return Ok(None);
            };
            if !node_exists(tree, parent) {
                return Err(MoveRefusal::NoSuchNode);
            }
            if !is_group_node(tree, parent) {
                return Err(MoveRefusal::NotAContainer);
            }
            Ok(Some(parent))
        }
    }
}

/// Whether `node` or a node above it is in `set`.
fn is_or_under(tree: &LoroTree, node: TreeID, set: &HashSet<TreeID>) -> bool {
    // A Loro tree holds no cycle, so the walk to the root ends; it is not capped, so no merged
    // tree deeper than a cap can make a cyclic move look allowed.
    let mut current = Some(node);
    while let Some(here) = current {
        if set.contains(&here) {
            return true;
        }
        current = group_parent(tree, here);
    }
    false
}

/// The moved nodes in stacking order (back to front), by their place in a pre-order walk.
fn in_stacking_order(tree: &LoroTree, nodes: &HashSet<TreeID>) -> Vec<TreeID> {
    let mut ordered = Vec::with_capacity(nodes.len());
    let mut stack: Vec<TreeID> = children_of(tree, None).into_iter().rev().collect();
    while let Some(node) = stack.pop() {
        if nodes.contains(&node) {
            ordered.push(node);
            if ordered.len() == nodes.len() {
                break;
            }
        }
        stack.extend(children_of(tree, Some(node)).into_iter().rev());
    }
    ordered
}

/// Checks `plan` against the rules of criterion 30 and returns the moved nodes in stacking order
/// with their container.
fn checked(tree: &LoroTree, plan: &MovePlan) -> Result<(Vec<TreeID>, Option<TreeID>), MoveRefusal> {
    if plan.nodes.is_empty() {
        return Err(MoveRefusal::Nothing);
    }
    let named: HashSet<TreeID> = plan.nodes.iter().map(|&id| tree_id_of(id)).collect();
    if named.iter().any(|&id| !node_exists(tree, id)) {
        return Err(MoveRefusal::NoSuchNode);
    }
    // A node under another moved node goes with it and keeps its place inside it.
    let moved: HashSet<TreeID> = named
        .iter()
        .copied()
        .filter(|&id| group_parent(tree, id).is_none_or(|p| !is_or_under(tree, p, &named)))
        .collect();
    let parent = target_parent(tree, plan.target)?;
    if let MoveTarget::Above(sibling) | MoveTarget::Below(sibling) = plan.target
        && moved.contains(&tree_id_of(sibling))
    {
        return Err(MoveRefusal::IntoItself);
    }
    if parent.is_some_and(|p| is_or_under(tree, p, &moved)) {
        return Err(MoveRefusal::IntoItself);
    }
    if moved.iter().any(|&id| locked_through(tree, id)) {
        return Err(MoveRefusal::Locked);
    }
    if parent.is_some_and(|p| locked_through(tree, p)) {
        return Err(MoveRefusal::LockedTarget);
    }
    if parent.is_some() && moved.iter().any(|&id| is_layer_node(tree, id)) {
        return Err(MoveRefusal::LayerNotTopLevel);
    }
    let above = parent.map_or(0, |p| groups_above(tree, p) + 1);
    if moved
        .iter()
        .any(|&id| above + groups_in(tree, id) > MAX_GROUP_DEPTH)
    {
        return Err(MoveRefusal::TooDeep);
    }
    Ok((in_stacking_order(tree, &moved), parent))
}

impl Document {
    /// Whether `plan` would be carried out by [`Document::move_nodes`]: the rules of criterion 30,
    /// read from the document as it is now.
    ///
    /// # Errors
    /// The [`MoveRefusal`] that [`Document::move_nodes`] would give.
    pub fn check_move(&self, plan: &MovePlan) -> Result<(), MoveRefusal> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        checked(&tree, plan).map(|_| ())
    }

    /// Moves the nodes of `plan` to its target, keeping their relative stacking order, in one
    /// commit labelled [`MOVE_NODES_COMMIT_LABEL`] (criteria 29 to 33). Every node keeps its id,
    /// geometry, style, registers and links (criterion 31). A group that a move empties is
    /// removed (`specs/0023-groups` criterion 14); a layer stays (criterion 37).
    ///
    /// # Errors
    /// A [`MoveRefusal`] when a rule of criterion 30 is broken; nothing is written.
    ///
    /// # Panics
    /// Does not panic in practice: every tree operation acts on nodes just confirmed live, and
    /// no node goes under itself.
    pub fn move_nodes(&self, plan: &MovePlan) -> Result<(), MoveRefusal> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let (nodes, parent) = checked(&tree, plan)?;
        let old_parents = crate::group_tree::parents_of(&tree, &nodes);
        let moved: HashSet<TreeID> = nodes.iter().copied().collect();
        let others: Vec<TreeID> = children_of(&tree, parent)
            .into_iter()
            .filter(|id| !moved.contains(id))
            .collect();
        tree.enable_fractional_index(0);
        let parent_id = parent.map_or(TreeParentId::Root, TreeParentId::Node);
        let (first, rest) = nodes.split_first().ok_or(MoveRefusal::Nothing)?;
        // invariant: every node and anchor is live, and no node goes under itself (checked).
        #[allow(clippy::unwrap_used)]
        match plan.target {
            MoveTarget::Above(sibling) => tree.mov_after(*first, tree_id_of(sibling)).unwrap(),
            MoveTarget::Below(sibling) => tree.mov_before(*first, tree_id_of(sibling)).unwrap(),
            MoveTarget::FrontOf(_) => match others.last() {
                Some(&last) => tree.mov_after(*first, last).unwrap(),
                None => tree.mov_to(*first, parent_id, 0).unwrap(),
            },
            MoveTarget::BackOf(_) => match others.first() {
                Some(&front) => tree.mov_before(*first, front).unwrap(),
                None => tree.mov_to(*first, parent_id, 0).unwrap(),
            },
        }
        let mut after = *first;
        for &node in rest {
            // invariant: as above; `after` was just placed under the target container.
            #[allow(clippy::unwrap_used)]
            tree.mov_after(node, after).unwrap();
            after = node;
        }
        remove_emptied_groups(&tree, &old_parents);
        self.commit_with_label(MOVE_NODES_COMMIT_LABEL);
        Ok(())
    }
}
