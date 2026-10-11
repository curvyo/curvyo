//! Groups: the tree reads, the Group and Ungroup commands and the rules that keep a group tree
//! valid (`specs/0023-groups`).
//!
//! A group is a node of the object tree (ADR 0002 section 5) whose meta map holds the one register
//! `group: true`. Its tree children are its members, and their sibling order is their z-order
//! inside it. It has no style, no transform and no anchors. A node that is not a group never has
//! children. At most [`MAX_GROUP_DEPTH`] groups lie on one root-to-leaf chain.

use std::collections::HashSet;

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::subpath_codec::total_anchor_count;

/// The most groups one chain from the document root to a leaf may hold
/// (`specs/0023-groups` criterion 5).
pub const MAX_GROUP_DEPTH: usize = 32;

/// The register that marks a tree node as a group.
pub(crate) const KEY_GROUP: &str = "group";

/// The label of the Group command's commit.
pub const GROUP_COMMIT_LABEL: &str = "group";
/// The label of the Ungroup command's commit.
pub const UNGROUP_COMMIT_LABEL: &str = "ungroup";

fn node_id(id: TreeID) -> NodeId {
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

/// The group that holds `id`, or `None` for a node at the document root.
fn group_parent(tree: &LoroTree, id: TreeID) -> Option<TreeID> {
    match tree.parent(id)? {
        TreeParentId::Node(parent) => Some(parent),
        _ => None,
    }
}

fn children_of(tree: &LoroTree, parent: Option<TreeID>) -> Vec<TreeID> {
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
fn groups_above(tree: &LoroTree, id: TreeID) -> usize {
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
fn groups_in(tree: &LoroTree, id: TreeID) -> usize {
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

/// Removes every group that has no child left, starting from `parents` (the groups the caller took
/// children out of, before the caller's own writes) and going up while a removal empties the next
/// group (`specs/0023-groups` criterion 14). Called in the same commit as the delete or replace
/// that emptied them. A parent that is `None`, deleted or not a group is ignored.
pub(crate) fn remove_emptied_groups(tree: &LoroTree, parents: &[Option<TreeID>]) {
    for &start in parents {
        let mut current = start;
        while let Some(group) = current {
            let alive = tree.contains(group) && matches!(tree.is_node_deleted(&group), Ok(false));
            if !alive || !is_group_node(tree, group) || tree.children_num(group) != Some(0) {
                break;
            }
            let above = group_parent(tree, group);
            if tree.delete(group).is_err() {
                break;
            }
            current = above;
        }
    }
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

    /// The group that directly holds `id`, or `None` for a node at the document root or one that
    /// does not exist.
    #[must_use]
    pub fn parent_group(&self, id: NodeId) -> Option<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !node_exists(&tree, tree_id) {
            return None;
        }
        group_parent(&tree, tree_id).map(node_id)
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

    /// Every child of `parent` (`None` is the document root) with its leaves: one entry per object
    /// the Select tool sees at that level. A leaf lists itself; an empty group lists nothing.
    #[must_use]
    pub fn child_leaves(&self, parent: Option<NodeId>) -> Vec<(NodeId, Vec<NodeId>)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if let Some(parent) = parent
            && !node_exists(&tree, tree_id_of(parent))
        {
            return Vec::new();
        }
        children_of(&tree, parent.map(tree_id_of))
            .into_iter()
            .map(|child| {
                let leaves = if is_group_node(&tree, child) {
                    leaves_under(&tree, Some(child))
                        .into_iter()
                        .map(node_id)
                        .collect()
                } else {
                    vec![node_id(child)]
                };
                (node_id(child), leaves)
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

    /// The group that [`crate::Document::create_path`] and the other `create_*` methods put new
    /// objects into, on top of its children; `None` is the document root. A group that does not
    /// exist (any more) means the root. View state of the caller: it is not stored.
    pub fn set_draw_parent(&self, parent: Option<NodeId>) {
        self.draw_parent.set(parent.map(tree_id_of));
    }

    /// The node a new object is created under: the draw parent while it is a live group.
    pub(crate) fn creation_parent(&self, tree: &LoroTree) -> TreeParentId {
        match self.draw_parent.get() {
            Some(id) if node_exists(tree, id) && is_group_node(tree, id) => TreeParentId::Node(id),
            _ => TreeParentId::Root,
        }
    }

    /// Binds the objects `ids` into one new group, in **one commit** labelled
    /// [`GROUP_COMMIT_LABEL`] (`specs/0023-groups` criterion 1). The members keep their ids and
    /// their relative order. The group takes the place of the topmost member; objects between the
    /// members that are not named stay below it. Returns the group's id.
    ///
    /// Everything is checked before the first write.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an id does not exist;
    /// [`ObjectEditError::NotEnoughObjects`] for fewer than two different ids;
    /// [`ObjectEditError::NotSiblings`] if the objects do not share one parent;
    /// [`ObjectEditError::TooDeep`] if the group would put more than [`MAX_GROUP_DEPTH`] groups on
    /// one chain.
    ///
    /// # Panics
    /// Does not panic in practice: every tree operation below acts on nodes just confirmed live.
    pub fn group_objects(&self, ids: &[NodeId]) -> Result<NodeId, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut seen = HashSet::new();
        let members: Vec<TreeID> = ids
            .iter()
            .map(|&id| tree_id_of(id))
            .filter(|id| seen.insert(*id))
            .collect();
        if members.iter().any(|&id| !node_exists(&tree, id)) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if members.len() < 2 {
            return Err(ObjectEditError::NotEnoughObjects);
        }
        let parent = group_parent(&tree, members[0]);
        if members.iter().any(|&id| group_parent(&tree, id) != parent) {
            return Err(ObjectEditError::NotSiblings);
        }
        let inside = members
            .iter()
            .map(|&id| groups_in(&tree, id))
            .max()
            .unwrap_or(0);
        let above = parent.map_or(0, |p| groups_above(&tree, p) + 1);
        if above + 1 + inside > MAX_GROUP_DEPTH {
            return Err(ObjectEditError::TooDeep);
        }
        let siblings = children_of(&tree, parent);
        let mut ordered: Vec<(usize, TreeID)> = members
            .iter()
            .filter_map(|&id| siblings.iter().position(|&s| s == id).map(|at| (at, id)))
            .collect();
        ordered.sort_unstable_by_key(|&(at, _)| at);
        let topmost_at = ordered.last().map_or(0, |&(at, _)| at);

        tree.enable_fractional_index(0);
        let parent_id = parent.map_or(TreeParentId::Root, TreeParentId::Node);
        // invariant: the parent is live (it holds the confirmed members) and the index is one past
        // an existing child, so it is at most the child count.
        #[allow(clippy::unwrap_used)]
        let group = tree.create_at(parent_id, topmost_at + 1).unwrap();
        // invariant: the node was just created.
        #[allow(clippy::unwrap_used)]
        tree.get_meta(group)
            .unwrap()
            .insert(KEY_GROUP, true)
            .unwrap();
        for (_, member) in ordered {
            // invariant: the member is live and the new group is not below it.
            #[allow(clippy::unwrap_used)]
            tree.mov(member, group).unwrap();
        }
        self.commit_with_label(GROUP_COMMIT_LABEL);
        Ok(node_id(group))
    }

    /// Replaces each group of `ids` by its children at its place, in **one commit** labelled
    /// [`UNGROUP_COMMIT_LABEL`] (`specs/0023-groups` criterion 3). One level only: a group inside
    /// stays a group. Nothing about a child changes but its parent. Returns the released children
    /// in z-order, group by group.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an id does not exist; [`ObjectEditError::NotAGroup`] if
    /// an id is a leaf (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every tree operation below acts on nodes just confirmed live.
    pub fn ungroup(&self, ids: &[NodeId]) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut seen = HashSet::new();
        let groups: Vec<TreeID> = ids
            .iter()
            .map(|&id| tree_id_of(id))
            .filter(|id| seen.insert(*id))
            .collect();
        if groups.iter().any(|&id| !node_exists(&tree, id)) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if groups.is_empty() || groups.iter().any(|&id| !is_group_node(&tree, id)) {
            return Err(ObjectEditError::NotAGroup);
        }
        tree.enable_fractional_index(0);
        let mut released = Vec::new();
        for group in groups {
            let mut after = group;
            for child in children_of(&tree, Some(group)) {
                // invariant: the child and the group are live; `after` is a sibling of the
                // group's place.
                #[allow(clippy::unwrap_used)]
                tree.mov_after(child, after).unwrap();
                after = child;
                released.push(node_id(child));
            }
            // invariant: the group is live and now has no child.
            #[allow(clippy::unwrap_used)]
            tree.delete(group).unwrap();
        }
        self.commit_with_label(UNGROUP_COMMIT_LABEL);
        Ok(released)
    }
}

#[cfg(test)]
#[allow(clippy::many_single_char_names, clippy::cast_precision_loss)]
mod tests {
    use crate::{AnchorId, Document, NewAnchor, ObjectEditError, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: crate::Length::from_mm(10.0),
            height: crate::Length::from_mm(10.0),
        })
    }

    fn path(document: &Document, n: u64) -> NodeId {
        document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, n), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(10.0, 0.0)),
            ],
            false,
        )
    }

    fn last_label(document: &Document) -> String {
        let loro = document.loro();
        let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
        let mut newest = (0, String::new());
        loro.travel_change_ancestors(&ids, &mut |m| {
            if m.lamport >= newest.0 {
                newest = (
                    m.lamport,
                    m.message
                        .map(|s| s.split(';').next().unwrap_or_default().to_string())
                        .unwrap_or_default(),
                );
            }
            std::ops::ControlFlow::Continue(())
        })
        .unwrap();
        newest.1
    }

    #[test]
    fn a_group_takes_the_place_of_its_topmost_member() {
        // Criterion 1: A (bottom), B, C (top) with A and C selected becomes B, group(A, C).
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let group = document.group_objects(&[c, a]).unwrap();
        assert_eq!(document.children(None), vec![b, group]);
        assert_eq!(document.children(Some(group)), vec![a, c]);
        assert!(document.is_group(group));
        assert_eq!(document.object_ids(), vec![b, a, c]);
        assert_eq!(document.leaves_of(group), vec![a, c]);
        assert_eq!(document.parent_group(a), Some(group));
        assert_eq!(document.parent_group(b), None);
    }

    #[test]
    fn a_group_is_not_an_object_snapshot() {
        let document = Document::new(1);
        let (a, b) = (rect(&document, 0.0), path(&document, 1));
        let group = document.group_objects(&[a, b]).unwrap();
        assert!(document.object(group).is_none());
        assert!(document.path(group).is_none());
        assert!(document.primitive(group).is_none());
        assert!(document.object(a).is_some());
    }

    #[test]
    fn group_refuses_fewer_than_two_missing_and_unrelated_objects() {
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        assert_eq!(
            document.group_objects(&[a]),
            Err(ObjectEditError::NotEnoughObjects)
        );
        assert_eq!(
            document.group_objects(&[a, a]),
            Err(ObjectEditError::NotEnoughObjects)
        );
        let inner = document.group_objects(&[a, b]).unwrap();
        assert_eq!(
            document.group_objects(&[c, a]),
            Err(ObjectEditError::NotSiblings)
        );
        assert_eq!(
            document.children(None),
            vec![inner, c],
            "nothing was written"
        );
        document.delete_objects(&[c]).unwrap();
        assert_eq!(
            document.group_objects(&[c, inner]),
            Err(ObjectEditError::NoSuchObject)
        );
    }

    #[test]
    fn ungroup_puts_the_children_in_the_groups_place_one_level_only() {
        let document = Document::new(1);
        let (a, b, c, d) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
            rect(&document, 60.0),
        );
        let inner = document.group_objects(&[b, c]).unwrap();
        let outer = document.group_objects(&[a, inner]).unwrap();
        assert_eq!(document.children(None), vec![outer, d]);
        let released = document.ungroup(&[outer]).unwrap();
        assert_eq!(released, vec![a, inner]);
        assert_eq!(document.children(None), vec![a, inner, d]);
        assert!(document.is_group(inner), "inner groups stay groups");
        assert!(!document.contains_node(outer));
        assert_eq!(document.ungroup(&[a]), Err(ObjectEditError::NotAGroup));
    }

    #[test]
    fn group_then_ungroup_restores_the_order_without_objects_in_between() {
        let document = Document::new(1);
        let ids = [
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        ];
        let before = document.export_json().unwrap();
        let group = document.group_objects(&ids).unwrap();
        document.ungroup(&[group]).unwrap();
        assert_eq!(document.children(None), ids.to_vec());
        assert_eq!(document.export_json().unwrap(), before);
    }

    #[test]
    fn each_command_is_one_commit_with_its_label() {
        let document = Document::new(1);
        let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
        let commits = |d: &Document| d.loro().len_changes();
        let before = commits(&document);
        let group = document.group_objects(&[a, b]).unwrap();
        assert_eq!(commits(&document), before + 1);
        assert_eq!(last_label(&document), GROUP_COMMIT_LABEL);
        document.ungroup(&[group]).unwrap();
        assert_eq!(commits(&document), before + 2);
        assert_eq!(last_label(&document), UNGROUP_COMMIT_LABEL);
    }

    #[test]
    fn nesting_is_limited_to_thirty_two_levels() {
        let document = Document::new(1);
        let mut top = rect(&document, 0.0);
        for level in 1..=MAX_GROUP_DEPTH {
            let other = rect(&document, 20.0 * level as f64);
            top = document.group_objects(&[top, other]).unwrap();
        }
        assert_eq!(
            document.ancestor_groups(document.leaves_of(top)[0]).len(),
            MAX_GROUP_DEPTH
        );
        let one_more = rect(&document, -20.0);
        assert_eq!(
            document.group_objects(&[top, one_more]),
            Err(ObjectEditError::TooDeep)
        );
    }

    #[test]
    fn deleting_the_last_child_removes_the_group_in_the_same_commit() {
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let inner = document.group_objects(&[a, b]).unwrap();
        let outer = document.group_objects(&[inner, c]).unwrap();
        let before = document.loro().len_changes();
        document.delete_objects(&[a, b]).unwrap();
        assert_eq!(document.loro().len_changes(), before + 1);
        assert!(
            !document.contains_node(inner),
            "the emptied inner group is gone"
        );
        assert!(document.is_group(outer), "the outer group still holds c");
        document.delete_objects(&[c]).unwrap();
        assert!(!document.contains_node(outer));
        assert_eq!(document.children(None), Vec::new());
    }

    #[test]
    fn new_objects_go_on_top_of_the_draw_parent_while_it_exists() {
        let document = Document::new(1);
        let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
        let group = document.group_objects(&[a, b]).unwrap();
        document.set_draw_parent(Some(group));
        let c = rect(&document, 40.0);
        assert_eq!(document.children(Some(group)), vec![a, b, c]);
        let p = path(&document, 5);
        assert_eq!(document.children(Some(group)), vec![a, b, c, p]);
        document.ungroup(&[group]).unwrap();
        let d = rect(&document, 60.0);
        assert_eq!(
            document.children(None).last(),
            Some(&d),
            "a gone group means the root"
        );
    }
}
