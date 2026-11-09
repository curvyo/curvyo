//! Placing linked clones of a selection on a laser page by a drop (`specs/0100-bring-content-in`
//! criterion 1, `adrs.md` note 1): one clone per original, moved by the drop's translation, with
//! own placement, on top of the drawing parent, in one commit.

use loro::TreeID;

use crate::affine::Affine;
use crate::clone_codec::{CloneLink, KEY_OWN_PLACEMENT, write_clone, write_flag};
use crate::clone_model::MAX_CLONE_DEPTH;
use crate::clones::CloneError;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::NodeId;
use crate::units::Vec2;

/// The label of placing linked clones by a drop (`0100` criterion 1 e).
pub const PLACE_LINKED_CLONES_COMMIT_LABEL: &str = "place_linked_clones";

/// The label of Place on… (`0100` criterion 25; the History row names it "Place on page").
pub const PLACE_ON_PAGE_COMMIT_LABEL: &str = "place_on_page";

impl Document {
    /// Places one linked clone of each of `originals` (in that order, a repeated id once), each
    /// with the matrix `translation` and own placement on, on top of the drawing parent (the
    /// active layer outside any entered group), in **one commit** labelled
    /// [`PLACE_LINKED_CLONES_COMMIT_LABEL`]. A primitive original is not converted (ADR 0045
    /// §1). Returns the clones in the order of `originals`.
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] or [`CloneError::NotClonable`] for an original that does not
    /// exist or is not a legal origin (a layer, a dimension, a node of a laser page, an orphan
    /// clone); [`CloneError::TooDeep`] with every original clone [`MAX_CLONE_DEPTH`] deep;
    /// [`CloneError::OutOfRange`] for a translation past the matrix limit. Nothing is written.
    pub fn place_linked_clones(
        &self,
        originals: &[NodeId],
        translation: Vec2,
    ) -> Result<Vec<NodeId>, CloneError> {
        self.place_linked_clones_labelled(originals, translation, PLACE_LINKED_CLONES_COMMIT_LABEL)
    }

    /// [`Document::place_linked_clones`] for Place on… (criterion 25), labelled
    /// [`PLACE_ON_PAGE_COMMIT_LABEL`].
    ///
    /// # Errors
    /// As [`Document::place_linked_clones`].
    pub fn place_linked_clones_on_page(
        &self,
        originals: &[NodeId],
        translation: Vec2,
    ) -> Result<Vec<NodeId>, CloneError> {
        self.place_linked_clones_labelled(originals, translation, PLACE_ON_PAGE_COMMIT_LABEL)
    }

    fn place_linked_clones_labelled(
        &self,
        originals: &[NodeId],
        translation: Vec2,
        label: &str,
    ) -> Result<Vec<NodeId>, CloneError> {
        let mut seen = std::collections::HashSet::new();
        let originals: Vec<NodeId> = originals
            .iter()
            .copied()
            .filter(|id| seen.insert(*id))
            .collect();
        if originals.is_empty() {
            return Ok(Vec::new());
        }
        let matrix = Affine::translation(translation);
        if !matrix.is_within_limit() {
            return Err(CloneError::OutOfRange);
        }
        let depths = self.clone_source_depths(&originals)?;
        let too_deep: Vec<NodeId> = originals
            .iter()
            .zip(&depths)
            .filter(|(_, depth)| **depth >= MAX_CLONE_DEPTH)
            .map(|(id, _)| *id)
            .collect();
        if !too_deep.is_empty() {
            return Err(CloneError::TooDeep(too_deep));
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let parent = self.drawing_parent(&tree);
        let mut created = Vec::with_capacity(originals.len());
        for &origin in &originals {
            let node: TreeID = tree.create(parent).map_err(|_| CloneError::NoSuchObject)?;
            let meta = tree.get_meta(node).map_err(|_| CloneError::NoSuchObject)?;
            write_clone(&meta, CloneLink { origin, matrix });
            write_flag(&meta, KEY_OWN_PLACEMENT, true);
            created.push(NodeId::from_parts(node.peer, node.counter));
        }
        self.commit_with_label(label);
        Ok(created)
    }
}
