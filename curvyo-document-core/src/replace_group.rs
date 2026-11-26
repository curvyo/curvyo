//! The command that replaces objects by a new named group of computed paths in a single commit
//! (Bridge, `specs/0156-fused-lettering/adrs.md` decision 5). It computes no geometry; the caller
//! passes the result.

use std::collections::HashSet;

use loro::{TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{KEY_GROUP, MAX_GROUP_DEPTH, group_parent, groups_above, node_id};
use crate::node_registers::KEY_NAME;
use crate::objects::ObjectEditError;
use crate::operation_codec::member_key;
use crate::path_model::{AnchorId, NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::replace::{PieceOutlines, check_new_anchor_ids, create_path_after, write_path_node};
use crate::style_model::Style;

/// The group [`Document::replace_with_group`] writes: a named group holding a named path above a
/// named group of named paths, every path with `style`.
#[derive(Debug, Clone, Copy)]
pub struct NewGroup<'a> {
    /// The group's name.
    pub name: &'a str,
    /// The style of every path.
    pub style: &'a Style,
    /// The top member: its name and outlines (the first in the ordinary keys).
    pub outline: (&'a str, &'a PieceOutlines),
    /// The operations whose membership the top member gets.
    pub operations: &'a [NodeId],
    /// The name of the bottom member, a group.
    pub parts_name: &'a str,
    /// The bottom member's paths, bottom first: name and outlines each.
    pub parts: &'a [(String, PieceOutlines)],
}

/// The ids [`Document::replace_with_group`] created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGroupIds {
    /// The group.
    pub group: NodeId,
    /// Its top member, the path.
    pub outline: NodeId,
    /// Its bottom member, the group of parts.
    pub parts: NodeId,
    /// The parts, in the order given.
    pub part_paths: Vec<NodeId>,
}

impl Document {
    /// Replaces the leaves `removed` by the group `new`, in **one commit** labelled `label`: the
    /// group sits directly after `place` in `place`'s parent (`place` is one of `removed` or a
    /// group whose every leaf is one; such a group is emptied and removed), holds the parts group
    /// at the bottom and the outline path above it, and the outline is written as a member of
    /// each of `new.operations`. Every removed leaf is deleted, with the groups it empties.
    ///
    /// Every id is resolved and every anchor id checked before the first write, so a refusal
    /// changes nothing.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if a removed leaf or an operation is gone;
    /// [`ObjectEditError::BaseNotAnOperand`] if `place` is neither removed nor a group of removed
    /// leaves only; [`ObjectEditError::NoOutlines`] if the outline or a part has no outline or an
    /// outline no anchor; [`ObjectEditError::AnchorIds`] if anchor ids repeat or belong to an object
    /// that stays; [`ObjectEditError::TooDeep`] beyond [`MAX_GROUP_DEPTH`].
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to a node this call just created or resolved.
    pub fn replace_with_group(
        &self,
        removed: &[NodeId],
        place: NodeId,
        new: &NewGroup<'_>,
        label: &str,
    ) -> Result<NewGroupIds, ObjectEditError> {
        let seen = anchor_ids(new)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut doomed: Vec<TreeID> = Vec::with_capacity(removed.len());
        for &leaf in removed {
            let id = tree_id_of(leaf);
            if !crate::path_codec::leaf_exists(&tree, id) {
                return Err(ObjectEditError::NoSuchObject);
            }
            if !doomed.contains(&id) {
                doomed.push(id);
            }
        }
        check_new_anchor_ids(&tree, &doomed, &seen)?;
        let place_id = tree_id_of(place);
        let is_place = doomed.contains(&place_id)
            || (crate::group_tree::is_group_node(&tree, place_id)
                && crate::group_tree::leaves_under(&tree, Some(place_id))
                    .iter()
                    .all(|leaf| doomed.contains(leaf)));
        if !is_place {
            return Err(ObjectEditError::BaseNotAnOperand);
        }
        let above = group_parent(&tree, place_id).map_or(0, |p| groups_above(&tree, p) + 1);
        if above + 2 > MAX_GROUP_DEPTH {
            return Err(ObjectEditError::TooDeep);
        }
        let operation_metas = new
            .operations
            .iter()
            .map(|&op| {
                self.operation_meta(op)
                    .map_err(|_| ObjectEditError::NoSuchObject)
            })
            .collect::<Result<Vec<_>, _>>()?;

        tree.enable_fractional_index(0);
        let group = new_group_after(&tree, place_id, new.name);
        // invariant: the group was just created and is live.
        #[allow(clippy::unwrap_used)]
        let parts = tree.create(TreeParentId::Node(group)).unwrap();
        write_group_meta(&tree, parts, new.parts_name);
        let mut part_paths = Vec::with_capacity(new.parts.len());
        for (name, outlines) in new.parts {
            // invariant: the parts group was just created and is live.
            #[allow(clippy::unwrap_used)]
            let path = tree.create(TreeParentId::Node(parts)).unwrap();
            write_path_node(&tree, path, new.style, outlines);
            write_name(&tree, path, name);
            part_paths.push(node_id(path));
        }
        let outline = create_path_after(&tree, parts, new.style, new.outline.1);
        write_name(&tree, outline, new.outline.0);
        for meta in &operation_metas {
            // invariant: the operation's meta map was just resolved; a plain value.
            #[allow(clippy::unwrap_used)]
            meta.insert(&member_key(&[outline]), true).unwrap();
            self.member_keys_may_exist.set(true);
        }
        self.repair_clones_of_removed(&tree, &doomed);
        let parents = crate::group_tree::parents_of(&tree, &doomed);
        for &id in &doomed {
            // invariant: every id here was just confirmed present above.
            #[allow(clippy::unwrap_used)]
            tree.delete(id).unwrap();
        }
        crate::groups::remove_emptied_groups(&tree, &parents);
        self.commit_with_label(label);
        Ok(NewGroupIds {
            group: node_id(group),
            outline: node_id(outline),
            parts: node_id(parts),
            part_paths,
        })
    }
}

/// Every anchor id of the group's paths, checked: each path has outlines with anchors, no id
/// twice.
fn anchor_ids(new: &NewGroup<'_>) -> Result<HashSet<AnchorId>, ObjectEditError> {
    let mut seen = HashSet::new();
    let paths = std::iter::once(new.outline.1).chain(new.parts.iter().map(|(_, o)| o));
    for outlines in paths {
        if outlines.is_empty() || outlines.iter().any(|(anchors, _)| anchors.is_empty()) {
            return Err(ObjectEditError::NoOutlines);
        }
        let anchors = outlines.iter().flat_map(|(anchors, _)| anchors);
        if !anchors.map(|a: &NewAnchor| a.id).all(|id| seen.insert(id)) {
            return Err(ObjectEditError::AnchorIds);
        }
    }
    Ok(seen)
}

/// A new named group directly after `after` in its parent.
fn new_group_after(tree: &loro::LoroTree, after: TreeID, name: &str) -> TreeID {
    // invariant: a root-level create on a live tree cannot fail; `after` was resolved live and
    // the fractional index is on.
    #[allow(clippy::unwrap_used)]
    let group = tree.create(TreeParentId::Root).unwrap();
    write_group_meta(tree, group, name);
    #[allow(clippy::unwrap_used)]
    tree.mov_after(group, after).unwrap();
    group
}

/// Marks the fresh node `node` as a group named `name`.
fn write_group_meta(tree: &loro::LoroTree, node: TreeID, name: &str) {
    // invariant: the node was just created, so its meta map exists; plain values only.
    #[allow(clippy::unwrap_used)]
    tree.get_meta(node)
        .unwrap()
        .insert(KEY_GROUP, true)
        .unwrap();
    write_name(tree, node, name);
}

/// Writes the name register of the fresh node `node`.
fn write_name(tree: &loro::LoroTree, node: TreeID, name: &str) {
    // invariant: the node was just created, so its meta map exists; a plain value.
    #[allow(clippy::unwrap_used)]
    tree.get_meta(node).unwrap().insert(KEY_NAME, name).unwrap();
}

#[cfg(test)]
#[path = "replace_group_tests.rs"]
mod tests;
