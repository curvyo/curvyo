//! The one write of Convert pattern to paths (`specs/0071-pattern-fill` criteria 24, 25; ADR 0029
//! §9): in **one commit** each shape gets a group directly above it holding one path per piece,
//! and its fill is turned off. It computes no geometry; the caller passes the pieces.

use loro::{LoroTree, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::MAX_GROUP_DEPTH;
use crate::group_tree::{KEY_GROUP, group_parent, groups_above};
use crate::objects::ObjectEditError;
use crate::offset_write::write_outlines;
use crate::path_codec::{leaf_exists, write_path_style};
use crate::path_model::{AnchorId, NodeId};
use crate::paths::tree_id_of;
use crate::replace::{PieceOutlines, check_new_anchor_ids};
use crate::style_codec;
use crate::style_model::Style;

/// The commit label of Convert pattern to paths (ADR 0002 §9).
pub const CONVERT_PATTERN_COMMIT_LABEL: &str = "convert_pattern";

/// One shape and the pieces its pattern becomes.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternConversion {
    /// The shape.
    pub id: NodeId,
    /// One path per piece, in order: its outlines and its style.
    pub pieces: Vec<(PieceOutlines, Style)>,
}

impl Document {
    /// Converts the patterns of `items` to paths in one commit labelled
    /// [`CONVERT_PATTERN_COMMIT_LABEL`]: a group directly above each shape with its pieces, the
    /// shape's fill off. Returns the new groups, to select.
    ///
    /// # Errors
    /// [`ObjectEditError::NoOutlines`] for nothing to convert or a piece without anchors;
    /// [`ObjectEditError::NoSuchObject`] for a shape that is gone;
    /// [`ObjectEditError::TooDeep`] when a group would go deeper than [`MAX_GROUP_DEPTH`];
    /// [`ObjectEditError::AnchorIds`] when anchor ids repeat or are in use. Nothing is written.
    pub fn convert_patterns(
        &self,
        items: &[PatternConversion],
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        validate(&tree, items)?;
        Ok(self.batch(CONVERT_PATTERN_COMMIT_LABEL, |_| {
            tree.enable_fractional_index(0);
            items.iter().map(|item| write_one(&tree, item)).collect()
        }))
    }
}

fn write_one(tree: &LoroTree, item: &PatternConversion) -> NodeId {
    let shape = tree_id_of(item.id);
    let parent = group_parent(tree, shape).map_or(TreeParentId::Root, TreeParentId::Node);
    // invariant: the shape was checked live, so its parent is; reading the new node's meta map
    // and inserting plain values cannot fail.
    #[allow(clippy::unwrap_used)]
    let group = tree.create(parent).unwrap();
    #[allow(clippy::unwrap_used)]
    tree.mov_after(group, shape).unwrap();
    #[allow(clippy::unwrap_used)]
    tree.get_meta(group)
        .unwrap()
        .insert(KEY_GROUP, true)
        .unwrap();
    for (outlines, style) in &item.pieces {
        #[allow(clippy::unwrap_used)]
        let node = tree.create(TreeParentId::Node(group)).unwrap();
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(node).unwrap();
        write_path_style(&meta, outlines[0].1, style);
        write_outlines(&meta, outlines);
    }
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(shape).unwrap();
    let old = style_codec::read_style(&meta);
    let mut new = old.clone();
    new.fill.enabled = false;
    style_codec::write_changes(&meta, &old, &new);
    NodeId::from_parts(group.peer, group.counter)
}

fn validate(tree: &LoroTree, items: &[PatternConversion]) -> Result<(), ObjectEditError> {
    let empty = |outlines: &PieceOutlines| {
        outlines.is_empty() || outlines.iter().any(|(anchors, _)| anchors.is_empty())
    };
    if items.is_empty()
        || items
            .iter()
            .any(|item| item.pieces.is_empty() || item.pieces.iter().any(|(o, _)| empty(o)))
    {
        return Err(ObjectEditError::NoOutlines);
    }
    for item in items {
        let shape = tree_id_of(item.id);
        if !leaf_exists(tree, shape) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if groups_above(tree, shape) + 1 > MAX_GROUP_DEPTH {
            return Err(ObjectEditError::TooDeep);
        }
    }
    let mut seen = std::collections::HashSet::<AnchorId>::new();
    for anchor in items
        .iter()
        .flat_map(|item| &item.pieces)
        .flat_map(|(outlines, _)| outlines)
        .flat_map(|(anchors, _)| anchors)
    {
        if !seen.insert(anchor.id) {
            return Err(ObjectEditError::AnchorIds);
        }
    }
    check_new_anchor_ids(tree, &[], &seen)
}
