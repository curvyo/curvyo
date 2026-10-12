//! The writes that change the group structure: Group, Ungroup, the removal of emptied groups and
//! the parent of new objects (`specs/0023-groups`). The reads and the validity rules are in
//! `crate::group_tree`.

use std::collections::HashSet;

use loro::{LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{
    KEY_GROUP, MAX_GROUP_DEPTH, children_of, group_parent, groups_above, groups_in, is_group_node,
    is_layer_node, node_id,
};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of the Group command's commit.
pub const GROUP_COMMIT_LABEL: &str = "group";
/// The label of the Ungroup command's commit.
pub const UNGROUP_COMMIT_LABEL: &str = "ungroup";

/// Removes every group that has no child left, starting from `parents` (the groups the caller took
/// children out of, before the caller's own writes) and going up while a removal empties the next
/// group (`specs/0023-groups` criterion 14). Called in the same commit as the delete or replace
/// that emptied them. A parent that is `None`, deleted or not a group is ignored.
///
/// Empty groups that were already there (a file written by a peer's concurrent edit, which the
/// reader tolerates) go too: every group without a child that is a child of one of `parents`
/// (`None` is the document root) is removed before the walk upwards, so the next local command
/// that touches a parent cleans up what lies in it.
///
/// A layer is never removed: a new layer is empty, and a maker empties a layer on purpose
/// (`specs/0050-layers-and-objects-panel/adrs.md` decision 2).
pub(crate) fn remove_emptied_groups(tree: &LoroTree, parents: &[Option<TreeID>]) {
    for &start in parents {
        for child in children_of(tree, start) {
            if tree.children_num(child).unwrap_or(0) == 0
                && is_group_node(tree, child)
                && !is_layer_node(tree, child)
            {
                let _ = tree.delete(child);
            }
        }
        let mut current = start;
        while let Some(group) = current {
            let alive = tree.contains(group) && matches!(tree.is_node_deleted(&group), Ok(false));
            if !alive
                || !is_group_node(tree, group)
                || is_layer_node(tree, group)
                || tree.children_num(group).unwrap_or(0) != 0
            {
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

impl Document {
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
    /// one chain; [`ObjectEditError::IsALayer`] if an id is a layer.
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
        if members.iter().any(|&id| is_layer_node(&tree, id)) {
            return Err(ObjectEditError::IsALayer);
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
        remove_emptied_groups(&tree, &[parent]);
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
    /// an id is a leaf; [`ObjectEditError::IsALayer`] if an id is a layer (nothing is written).
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
        if groups.iter().any(|&id| is_layer_node(&tree, id)) {
            return Err(ObjectEditError::IsALayer);
        }
        if groups.is_empty() || groups.iter().any(|&id| !is_group_node(&tree, id)) {
            return Err(ObjectEditError::NotAGroup);
        }
        tree.enable_fractional_index(0);
        let mut released = Vec::new();
        let parents: Vec<Option<TreeID>> = groups.iter().map(|&g| group_parent(&tree, g)).collect();
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
        remove_emptied_groups(&tree, &parents);
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
    fn a_command_that_touches_a_parent_removes_the_empty_groups_lying_in_it() {
        // An empty group from a peer's concurrent edit survives on open; the next local command
        // that touches its parent removes it, a group holding only such a group included.
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let outer = document.group_objects(&[a, b]).unwrap();
        let tree = document.loro().get_tree(OBJECTS_TREE);
        let empty = tree.create(TreeParentId::Node(tree_id_of(outer))).unwrap();
        tree.get_meta(empty)
            .unwrap()
            .insert(KEY_GROUP, true)
            .unwrap();
        let loose = tree.create(TreeParentId::Root).unwrap();
        tree.get_meta(loose)
            .unwrap()
            .insert(KEY_GROUP, true)
            .unwrap();
        document.loro().commit();
        assert_eq!(
            document.children(None).len(),
            3,
            "the group, {c:?} and an empty group"
        );
        document.delete_objects(&[c]).unwrap();
        assert_eq!(
            document.children(None),
            vec![outer],
            "the empty group at the root is gone"
        );
        assert_eq!(
            document.children(Some(outer)).len(),
            3,
            "the one inside is not touched yet"
        );
        document.delete_objects(&[a]).unwrap();
        assert_eq!(
            document.children(Some(outer)),
            vec![b],
            "a command on its parent removes it"
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
