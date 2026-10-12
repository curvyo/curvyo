//! Reads of the object tree's group structure, open-file validation included
//! (`specs/0023-groups`).
//!
//! A group is a node of the object tree (ADR 0002 section 5) whose meta map holds the one register
//! `group: true`. Its tree children are its members, and their sibling order is their z-order
//! inside it. It has no style, no transform and no anchors. A node that is not a group never has
//! children. At most [`MAX_GROUP_DEPTH`] groups lie on one root-to-leaf chain. The writes that
//! change the structure are in `crate::groups`.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::subpath_codec::total_anchor_count;

/// The most groups one chain from the document root to a leaf may hold
/// (`specs/0023-groups` criterion 5).
pub const MAX_GROUP_DEPTH: usize = 32;

/// The register that marks a tree node as a group.
pub(crate) const KEY_GROUP: &str = "group";

pub(crate) fn node_id(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
}

/// Whether `meta` marks a group: the register `group` holds `true`.
pub(crate) fn meta_is_group(meta: &LoroMap) -> bool {
    matches!(
        meta.get(KEY_GROUP),
        Some(ValueOrContainer::Value(LoroValue::Bool(true)))
    )
}

pub(crate) fn is_group_node(tree: &LoroTree, id: TreeID) -> bool {
    tree.get_meta(id).is_ok_and(|meta| meta_is_group(&meta))
}

/// Whether `id` is a layer: a group at the top level of the tree with the `layer` marker
/// (`specs/0050-layers-and-objects-panel/adrs.md` decision 1). A marker deeper down is merge
/// residue and the node is a plain group (ADR 0015 §4).
pub(crate) fn is_layer_node(tree: &LoroTree, id: TreeID) -> bool {
    group_parent(tree, id).is_none()
        && tree.get_meta(id).is_ok_and(|meta| {
            meta_is_group(&meta) && crate::node_registers::meta_has_layer_marker(&meta)
        })
}

/// The group that holds `id`, or `None` for a node at the document root.
pub(crate) fn group_parent(tree: &LoroTree, id: TreeID) -> Option<TreeID> {
    match tree.parent(id)? {
        TreeParentId::Node(parent) => Some(parent),
        _ => None,
    }
}

pub(crate) fn children_of(tree: &LoroTree, parent: Option<TreeID>) -> Vec<TreeID> {
    let parent = parent.map_or(TreeParentId::Root, TreeParentId::Node);
    tree.children(parent).unwrap_or_default()
}

/// The leaves under `parent` (`None` is the document root) in paint order: depth first, siblings in
/// z-order, a group contributing its leaves and never itself. An empty group contributes nothing.
pub(crate) fn leaves_under(tree: &LoroTree, parent: Option<TreeID>) -> Vec<TreeID> {
    let mut leaves = Vec::new();
    let mut stack = vec![children_of(tree, parent).into_iter()];
    while let Some(level) = stack.last_mut() {
        let Some(node) = level.next() else {
            stack.pop();
            continue;
        };
        if is_group_node(tree, node) {
            stack.push(children_of(tree, Some(node)).into_iter());
        } else {
            leaves.push(node);
        }
    }
    leaves
}

/// How many groups lie above `id` on its chain to the document root.
pub(crate) fn groups_above(tree: &LoroTree, id: TreeID) -> usize {
    let mut count = 0;
    let mut current = id;
    while let Some(parent) = group_parent(tree, current) {
        count += 1;
        current = parent;
        if count > MAX_GROUP_DEPTH * 4 {
            break;
        }
    }
    count
}

/// The most groups on one chain that starts at `id`, `id` counted when it is a group.
pub(crate) fn groups_in(tree: &LoroTree, id: TreeID) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(id, 0_usize)];
    while let Some((node, above)) = stack.pop() {
        if is_group_node(tree, node) {
            let here = above + 1;
            deepest = deepest.max(here);
            stack.extend(children_of(tree, Some(node)).into_iter().map(|c| (c, here)));
        } else {
            deepest = deepest.max(above);
        }
    }
    deepest
}

/// Whether an ancestor of `id` is in `set`.
pub(crate) fn has_ancestor_in(tree: &LoroTree, id: TreeID, set: &[TreeID]) -> bool {
    let mut current = id;
    let mut steps = 0;
    while let Some(parent) = group_parent(tree, current) {
        if set.contains(&parent) {
            return true;
        }
        current = parent;
        steps += 1;
        if steps > MAX_GROUP_DEPTH * 4 {
            break;
        }
    }
    false
}

/// The number of anchors in all leaves under `id` (`id` itself for a leaf).
pub(crate) fn anchors_under(tree: &LoroTree, id: TreeID) -> usize {
    let leaves = if is_group_node(tree, id) {
        leaves_under(tree, Some(id))
    } else {
        vec![id]
    };
    leaves
        .into_iter()
        .filter_map(|leaf| tree.get_meta(leaf).ok())
        .filter(|meta| crate::shape_codec::read_shape_tag(meta).is_none())
        .map(|meta| total_anchor_count(&meta))
        .sum()
}

/// The group parents of `ids`, for [`remove_emptied_groups`], read before the writes.
pub(crate) fn parents_of(tree: &LoroTree, ids: &[TreeID]) -> Vec<Option<TreeID>> {
    let mut parents = Vec::new();
    for &id in ids {
        let parent = group_parent(tree, id);
        if !parents.contains(&parent) {
            parents.push(parent);
        }
    }
    parents
}

/// Checks the structure of a whole object tree on open: a node that is not a group has no
/// children, a group carries no anchors and no shape tag, and no chain holds more than
/// [`MAX_GROUP_DEPTH`] groups. `node_ok` validates what is stored in one leaf node. A cycle cannot
/// be stored in a Loro tree, so only the depth needs a bound.
pub(crate) fn validate_tree(tree: &LoroTree, node_ok: fn(&LoroTree, TreeID) -> bool) -> bool {
    let mut stack: Vec<(TreeID, usize)> = children_of(tree, None)
        .into_iter()
        .map(|id| (id, 0))
        .collect();
    while let Some((node, above)) = stack.pop() {
        let children = children_of(tree, Some(node));
        let Ok(meta) = tree.get_meta(node) else {
            return false;
        };
        if !crate::node_registers::registers_are_valid(&meta, meta_is_group(&meta)) {
            return false;
        }
        match meta.get(KEY_GROUP) {
            None => {
                if !children.is_empty() || !node_ok(tree, node) {
                    return false;
                }
            }
            Some(ValueOrContainer::Value(LoroValue::Bool(true))) => {
                let here = above + 1;
                if here > MAX_GROUP_DEPTH
                    || meta.get(crate::path_codec::KEY_ANCHORS).is_some()
                    || meta.get(crate::shape_codec::KEY_SHAPE).is_some()
                {
                    return false;
                }
                stack.extend(children.into_iter().map(|c| (c, here)));
            }
            Some(_) => return false,
        }
    }
    true
}

impl Document {
    /// Whether `id` is a group (and live).
    #[must_use]
    pub fn is_group(&self, id: NodeId) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        node_exists(&tree, tree_id) && is_group_node(&tree, tree_id)
    }

    /// Whether `id` names a live node of the object tree, a group or a leaf.
    #[must_use]
    pub fn contains_node(&self, id: NodeId) -> bool {
        node_exists(&self.loro().get_tree(OBJECTS_TREE), tree_id_of(id))
    }

    /// The children of `parent` (`None` is the document root) in z-order, groups and leaves alike.
    /// Empty for a leaf or a node that does not exist.
    #[must_use]
    pub fn children(&self, parent: Option<NodeId>) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if let Some(parent) = parent
            && !node_exists(&tree, tree_id_of(parent))
        {
            return Vec::new();
        }
        children_of(&tree, parent.map(tree_id_of))
            .into_iter()
            .map(node_id)
            .collect()
    }

    /// The group that directly holds `id`, or `None` for a node at the document root, a node
    /// directly in a layer, and one that does not exist. Layers are transparent here: their
    /// children are top-level objects of the document for selecting and entering
    /// (`specs/0050-layers-and-objects-panel` criterion 42). [`Document::parent_node`] gives the
    /// layer.
    #[must_use]
    pub fn parent_group(&self, id: NodeId) -> Option<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !node_exists(&tree, tree_id) {
            return None;
        }
        group_parent(&tree, tree_id)
            .filter(|&parent| !is_layer_node(&tree, parent))
            .map(node_id)
    }

    /// The groups that hold `id`, the nearest first. Empty for a node at the document root.
    #[must_use]
    pub fn ancestor_groups(&self, id: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut current = id;
        while let Some(parent) = self.parent_group(current) {
            found.push(parent);
            current = parent;
            if found.len() > MAX_GROUP_DEPTH * 4 {
                break;
            }
        }
        found
    }

    /// The leaves of `id` in paint order: `id` itself for a leaf, every leaf below it for a group.
    /// Empty when `id` does not exist or is an empty group.
    #[must_use]
    pub fn leaves_of(&self, id: NodeId) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !node_exists(&tree, tree_id) {
            return Vec::new();
        }
        if !is_group_node(&tree, tree_id) {
            return vec![id];
        }
        leaves_under(&tree, Some(tree_id))
            .into_iter()
            .map(node_id)
            .collect()
    }

    /// The groups among the children of `parent` (`None` is the document root), each with its
    /// leaves in paint order: what the Select tool needs to treat a group as one object. A leaf
    /// child is not listed; an empty group lists no leaf. At the document root layers are
    /// transparent: a layer is not listed, and the groups directly in it are
    /// (`specs/0050-layers-and-objects-panel` criterion 42).
    #[must_use]
    pub fn child_groups(&self, parent: Option<NodeId>) -> Vec<(NodeId, Vec<NodeId>)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if let Some(parent) = parent
            && !node_exists(&tree, tree_id_of(parent))
        {
            return Vec::new();
        }
        let children = children_of(&tree, parent.map(tree_id_of));
        let children: Vec<TreeID> = if parent.is_none() {
            children
                .into_iter()
                .flat_map(|child| {
                    if is_layer_node(&tree, child) {
                        children_of(&tree, Some(child))
                    } else {
                        vec![child]
                    }
                })
                .collect()
        } else {
            children
        };
        children
            .into_iter()
            .filter(|&child| is_group_node(&tree, child))
            .map(|child| {
                let leaves = leaves_under(&tree, Some(child)).into_iter().map(node_id);
                (node_id(child), leaves.collect())
            })
            .collect()
    }

    /// The number of anchors in all leaves of `id`: what a copy of `id` needs fresh anchor ids for.
    #[must_use]
    pub fn anchor_count(&self, id: NodeId) -> usize {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if node_exists(&tree, tree_id) {
            anchors_under(&tree, tree_id)
        } else {
            0
        }
    }
}
