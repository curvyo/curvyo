//! The commands that write paths computed by the caller in a single commit: several objects by one
//! path with one or more outlines (Combine, the Boolean operations,
//! `specs/0016-boolean-operations/adrs.md`), and objects by pieces that take the place and style
//! of the object they come from (Split, Break apart, Cut, `specs/0048-split-compound-path`). They
//! compute no geometry; the caller passes the result.

use std::collections::{HashMap, HashSet};

use loro::TreeParentId;

use crate::document::{Document, OBJECTS_TREE};
use crate::objects::ObjectEditError;
use crate::path_codec::{self, anchor_map_at, insert_anchors_container, leaf_exists, push_anchor};
use crate::path_model::{AnchorId, NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::style_model::Style;
use crate::subpath_codec::{all_anchor_lists, write_extra_subpaths};

impl Document {
    /// Replaces `operands` by one new path made of `outlines`, in **one
    /// commit** labelled `label` (criteria 19 to 22, 27 and 28 of
    /// `specs/0016-boolean-operations`):
    ///
    /// - the new object sits directly after `base` in the stacking order
    ///   (`base` must be one of `operands`), so objects between the operands
    ///   that were not replaced keep their places;
    /// - it takes `base`'s complete style, whichever kind `base` is, and no
    ///   rotation (the register is absent, so it reads as 0);
    /// - it is a guide only when every operand is one (`specs/0046-document-shapes`
    ///   criterion 34);
    /// - the first outline is stored in the ordinary keys and the others as
    ///   [`crate::PathSnapshot::extra_subpaths`], so one outline gives an
    ///   ordinary path;
    /// - every operand is deleted.
    ///
    /// Every id is resolved and every anchor id checked before the first
    /// write, so a refusal changes nothing and the operands are never removed
    /// without the result being added. A peer's concurrent edit of an operand
    /// is lost with the operand, as with a delete. Returns the new object's id.
    ///
    /// Each outline is `(anchors, closed)`; the caller mints the anchor ids,
    /// which must be different over all outlines (this crate never mints one).
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if an operand no longer exists;
    /// [`ObjectEditError::BaseNotAnOperand`] if `base` is not one of
    /// `operands`; [`ObjectEditError::NoOutlines`] if there is no outline or an
    /// outline has no anchor; [`ObjectEditError::AnchorIds`] if two anchors
    /// share an id or an id already belongs to an object that is not replaced.
    ///
    /// # Panics
    /// Does not panic in practice: every write below goes to a tree node and
    /// meta map this call just created, or to one it just resolved.
    pub fn replace_with_path(
        &self,
        operands: &[NodeId],
        base: NodeId,
        outlines: &[(Vec<NewAnchor>, bool)],
        label: &str,
    ) -> Result<NodeId, ObjectEditError> {
        if !operands.contains(&base) {
            return Err(ObjectEditError::BaseNotAnOperand);
        }
        let seen = new_anchor_ids(outlines)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let doomed = doomed_leaves(&tree, operands)?;
        check_new_anchor_ids(&tree, &doomed, &seen)?;
        let base_meta = tree
            .get_meta(tree_id_of(base))
            .map_err(|_| ObjectEditError::NoSuchObject)?;
        let style = crate::clone_write::leaf_style(&tree, base, &base_meta);
        Ok(self.write_replacement(&tree, &doomed, tree_id_of(base), &style, outlines, label))
    }

    /// [`Document::replace_with_path`] with the place and the style given by the caller (Weld,
    /// `specs/0068-laserable-lettering/adrs.md` decision 6): the new path has `style` and sits
    /// directly after `place` in `place`'s parent. `place` is one of `operands` or a group whose
    /// every leaf is one; such a group is emptied and removed, so the result takes its place.
    ///
    /// # Errors
    /// As [`Document::replace_with_path`]; [`ObjectEditError::BaseNotAnOperand`] when `place` is
    /// neither an operand nor a group of operands only.
    ///
    /// # Panics
    /// Does not panic in practice, for the reasons given at [`Document::replace_with_path`].
    pub fn replace_with_path_at(
        &self,
        operands: &[NodeId],
        style: &Style,
        place: NodeId,
        outlines: &[(Vec<NewAnchor>, bool)],
        label: &str,
    ) -> Result<NodeId, ObjectEditError> {
        let seen = new_anchor_ids(outlines)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let doomed = doomed_leaves(&tree, operands)?;
        check_new_anchor_ids(&tree, &doomed, &seen)?;
        let place_tree_id = tree_id_of(place);
        if !place_of_operands(&tree, place_tree_id, &doomed) {
            return Err(ObjectEditError::BaseNotAnOperand);
        }
        Ok(self.write_replacement(&tree, &doomed, place_tree_id, style, outlines, label))
    }

    /// The write of [`Document::replace_with_path`] and [`Document::replace_with_path_at`], after
    /// every check: the new path after `place`, a guide when every operand is one
    /// (`specs/0046-document-shapes` criterion 34), the operands and the groups they empty
    /// removed, one commit.
    fn write_replacement(
        &self,
        tree: &loro::LoroTree,
        doomed: &[loro::TreeID],
        place: loro::TreeID,
        style: &Style,
        outlines: &[(Vec<NewAnchor>, bool)],
        label: &str,
    ) -> NodeId {
        let guide = crate::guides::all_guides(tree, doomed);
        tree.enable_fractional_index(0);
        let new_tree_id = create_path_after(tree, place, style, outlines);
        if guide {
            // invariant: the node was just created.
            #[allow(clippy::unwrap_used)]
            crate::guides::write_guide(&tree.get_meta(new_tree_id).unwrap(), true);
        }
        self.repair_clones_of_removed(tree, doomed);
        let parents = crate::group_tree::parents_of(tree, doomed);
        for &tree_id in doomed {
            // invariant: every id here was just confirmed present above.
            #[allow(clippy::unwrap_used)]
            tree.delete(tree_id).unwrap();
        }
        crate::groups::remove_emptied_groups(tree, &parents);
        self.commit_with_label(label);
        NodeId::from_parts(new_tree_id.peer, new_tree_id.counter)
    }

    /// Replaces the objects `removed` by `pieces`, in **one commit** labelled `label`
    /// (`specs/0048-split-compound-path/adrs.md`: the one write behind Split, Break apart and Cut).
    ///
    /// Each piece is `(source, outlines)`: the object whose place and complete style the piece
    /// takes, and its outlines `(anchors, closed)`, the first stored in the ordinary keys and the
    /// others as extra subpaths. A piece has no rotation. The pieces of one source sit
    /// consecutively directly after it, in the order given. With [`PieceFill::None`] the fill of
    /// every piece is off (its colour and opacity stay stored); with [`PieceFill::Keep`] a piece
    /// has the source's style as it is. A piece of a guide is a guide. Every object of `removed`
    /// is deleted, and a group that loses its last child with it. A removed object may have no
    /// piece (Fracture and Flatten remove covered objects, `specs/0037-fracture-and-flatten`), so
    /// `pieces` may be empty when `removed` is not.
    ///
    /// Every id is resolved and every anchor id checked before the first write, so a refusal
    /// changes nothing. Returns the new objects' ids in the order of `pieces`.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if a source or a removed object no longer exists;
    /// [`ObjectEditError::BaseNotAnOperand`] if a piece's source is not in `removed`;
    /// [`ObjectEditError::NoOutlines`] if there is neither a piece nor a removed object, a piece has
    /// no outline or an outline no anchor; [`ObjectEditError::AnchorIds`] as for [`Document::replace_with_path`].
    ///
    /// # Panics
    /// Does not panic in practice, for the reasons given at [`Document::replace_with_path`].
    pub fn replace_with_pieces(
        &self,
        removed: &[NodeId],
        pieces: &[(NodeId, PieceOutlines)],
        fill: PieceFill,
        label: &str,
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        if pieces.is_empty() && removed.is_empty() {
            return Err(ObjectEditError::NoOutlines);
        }
        if pieces.iter().any(|(source, _)| !removed.contains(source)) {
            return Err(ObjectEditError::BaseNotAnOperand);
        }
        let mut seen: HashSet<AnchorId> = HashSet::new();
        for (_, outlines) in pieces {
            if outlines.is_empty() || outlines.iter().any(|(anchors, _)| anchors.is_empty()) {
                return Err(ObjectEditError::NoOutlines);
            }
            for anchor in outlines.iter().flat_map(|(anchors, _)| anchors) {
                if !seen.insert(anchor.id) {
                    return Err(ObjectEditError::AnchorIds);
                }
            }
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut doomed = Vec::with_capacity(removed.len());
        for &object in removed {
            let tree_id = tree_id_of(object);
            if !leaf_exists(&tree, tree_id) {
                return Err(ObjectEditError::NoSuchObject);
            }
            if !doomed.contains(&tree_id) {
                doomed.push(tree_id);
            }
        }
        let sources = resolve_sources(&tree, pieces, fill)?;
        check_new_anchor_ids(&tree, &doomed, &seen)?;

        tree.enable_fractional_index(0);
        let mut after: HashMap<NodeId, loro::TreeID> = HashMap::new();
        let mut created = Vec::with_capacity(pieces.len());
        for (source, outlines) in pieces {
            // invariant: every source was resolved above.
            #[allow(clippy::unwrap_used)]
            let (place, style) = sources.get(source).unwrap();
            let anchor = *after.get(source).unwrap_or(place);
            let piece = create_path_after(&tree, anchor, style, outlines);
            // A piece of a guide is a guide: its one input was (criterion 34).
            if crate::guides::all_guides(&tree, &[*place]) {
                // invariant: the node was just created.
                #[allow(clippy::unwrap_used)]
                crate::guides::write_guide(&tree.get_meta(piece).unwrap(), true);
            }
            created.push(NodeId::from_parts(piece.peer, piece.counter));
            after.insert(*source, piece);
        }
        self.repair_clones_of_removed(&tree, &doomed);
        let parents = crate::group_tree::parents_of(&tree, &doomed);
        for tree_id in doomed {
            // invariant: every id here was just confirmed present above.
            #[allow(clippy::unwrap_used)]
            tree.delete(tree_id).unwrap();
        }
        crate::groups::remove_emptied_groups(&tree, &parents);
        self.commit_with_label(label);
        Ok(created)
    }
}

/// The anchor ids of `outlines`, checked: at least one outline, none without anchors, no id twice.
fn new_anchor_ids(
    outlines: &[(Vec<NewAnchor>, bool)],
) -> Result<HashSet<AnchorId>, ObjectEditError> {
    if outlines.is_empty() || outlines.iter().any(|(anchors, _)| anchors.is_empty()) {
        return Err(ObjectEditError::NoOutlines);
    }
    let mut seen: HashSet<AnchorId> = HashSet::new();
    if outlines
        .iter()
        .flat_map(|(anchors, _)| anchors)
        .all(|anchor| seen.insert(anchor.id))
    {
        Ok(seen)
    } else {
        Err(ObjectEditError::AnchorIds)
    }
}

/// The tree ids of `operands`, each once, or [`ObjectEditError::NoSuchObject`] if one is not a
/// live leaf.
fn doomed_leaves(
    tree: &loro::LoroTree,
    operands: &[NodeId],
) -> Result<Vec<loro::TreeID>, ObjectEditError> {
    let mut doomed = Vec::with_capacity(operands.len());
    for &operand in operands {
        let tree_id = tree_id_of(operand);
        if !leaf_exists(tree, tree_id) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if !doomed.contains(&tree_id) {
            doomed.push(tree_id);
        }
    }
    Ok(doomed)
}

/// Whether `place` is one of `doomed` or a group whose every leaf is.
fn place_of_operands(tree: &loro::LoroTree, place: loro::TreeID, doomed: &[loro::TreeID]) -> bool {
    doomed.contains(&place)
        || (crate::group_tree::is_group_node(tree, place)
            && crate::group_tree::leaves_under(tree, Some(place))
                .iter()
                .all(|leaf| doomed.contains(leaf)))
}

/// The outlines of one piece: `(anchors, closed)` each, the first stored in the ordinary keys.
pub type PieceOutlines = Vec<(Vec<NewAnchor>, bool)>;

/// Whether the pieces of [`Document::replace_with_pieces`] keep the style of the object they
/// come from as it is, or turn its fill off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceFill {
    /// The piece has its source's complete style (Split, Break apart).
    Keep,
    /// The piece has no fill; the stored fill colour and opacity are kept, so a solid fill
    /// restores them (Cut: the pieces are lines, `specs/0036-cut-at-crossings` criterion 6).
    None,
}

/// The tree id and the style (with the fill turned off for [`PieceFill::None`]) of every source of
/// `pieces`, or [`ObjectEditError::NoSuchObject`] if one is gone.
fn resolve_sources(
    tree: &loro::LoroTree,
    pieces: &[(NodeId, PieceOutlines)],
    fill: PieceFill,
) -> Result<HashMap<NodeId, (loro::TreeID, Style)>, ObjectEditError> {
    let mut sources: HashMap<NodeId, (loro::TreeID, Style)> = HashMap::new();
    for (source, _) in pieces {
        if sources.contains_key(source) {
            continue;
        }
        let tree_id = tree_id_of(*source);
        if !leaf_exists(tree, tree_id) {
            return Err(ObjectEditError::NoSuchObject);
        }
        let meta = tree
            .get_meta(tree_id)
            .map_err(|_| ObjectEditError::NoSuchObject)?;
        let mut style = crate::clone_write::leaf_style(tree, *source, &meta);
        if fill == PieceFill::None {
            style.fill.enabled = false;
        }
        sources.insert(*source, (tree_id, style));
    }
    Ok(sources)
}

/// Creates a new path object directly after `after` in the stacking order, with `style`, no
/// rotation, and `outlines` (the first stored in the ordinary keys, the rest as extra subpaths).
pub(crate) fn create_path_after(
    tree: &loro::LoroTree,
    after: loro::TreeID,
    style: &crate::style_model::Style,
    outlines: &[(Vec<NewAnchor>, bool)],
) -> loro::TreeID {
    // `mov_after` needs the fractional-index feature on for this tree (the caller enabled it).
    // invariant: a root-level create on a live tree cannot fail. `mov_after` below puts the node
    // into the parent of `after`, which the caller resolved live.
    #[allow(clippy::unwrap_used)]
    let new_tree_id = tree.create(TreeParentId::Root).unwrap();
    write_path_node(tree, new_tree_id, style, outlines);
    #[allow(clippy::unwrap_used)]
    tree.mov_after(new_tree_id, after).unwrap();
    new_tree_id
}

/// Writes `outlines` with `style` into the fresh node `node`: the first outline in the ordinary
/// keys, the rest as extra subpaths.
pub(crate) fn write_path_node(
    tree: &loro::LoroTree,
    node: loro::TreeID,
    style: &crate::style_model::Style,
    outlines: &[(Vec<NewAnchor>, bool)],
) {
    // invariant: reading the meta map of a node just created cannot fail, and the caller checked
    // `outlines` to be non-empty.
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(node).unwrap();
    #[allow(clippy::unwrap_used)]
    let (first, rest) = outlines.split_first().unwrap();
    path_codec::write_path_style(&meta, first.1, style);
    let anchor_list = insert_anchors_container(&meta);
    for anchor in &first.0 {
        push_anchor(&anchor_list, anchor);
    }
    write_extra_subpaths(&meta, rest);
}

/// An anchor id names a node across the whole document: none of `seen`, the ids a command is
/// about to write, may belong to an object that stays (`doomed` are the objects the command
/// deletes).
pub(crate) fn check_new_anchor_ids(
    tree: &loro::LoroTree,
    doomed: &[loro::TreeID],
    seen: &HashSet<AnchorId>,
) -> Result<(), ObjectEditError> {
    for leaf in crate::group_tree::leaves_under(tree, None) {
        if doomed.contains(&leaf) {
            continue;
        }
        // invariant: a leaf of the tree has a meta map.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(leaf).unwrap();
        if crate::shape_codec::read_shape_tag(&meta).is_some() {
            continue;
        }
        for list in all_anchor_lists(&meta) {
            for index in 0..list.len() {
                let existing = path_codec::read_anchor_id(&anchor_map_at(&list, index));
                if existing.is_some_and(|id| seen.contains(&id)) {
                    return Err(ObjectEditError::AnchorIds);
                }
            }
        }
    }
    Ok(())
}
