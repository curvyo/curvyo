//! The commands that write a sketch page as a whole (ADR 0043 §4 to §6): Convert to paths with
//! the kind change, Delete with its sketch, and placing a linked clone of the sketch.

use loro::TreeID;

use crate::affine::Affine;
use crate::clone_codec::{CloneLink, KEY_OWN_PLACEMENT, write_clone, write_flag};
use crate::document::{OBJECTS_TREE, ROOT_MAP};
use crate::page_kind::{DRAWING, KIND_FIELD};
use crate::paths::tree_id_of;
use crate::{AnchorId, Document, NodeId, PageError, PageKind, SketchEditError, Vec2};

/// The label of Convert to paths on a sketch page (`specs/0099-sketch-pages` criterion 15).
pub const CONVERT_SKETCH_PAGE_COMMIT_LABEL: &str = "convert_sketch_page";
/// The label of deleting a sketch page with its sketch (criterion 17).
pub const DELETE_SKETCH_PAGE_COMMIT_LABEL: &str = "delete_sketch_page";
/// The label of placing a linked clone of a sketch (criteria 8 and 11).
pub const PLACE_SKETCH_CLONE_COMMIT_LABEL: &str = "place_sketch_clone";

impl Document {
    /// Converts the owning sketch of sketch page `page` to paths and makes the page a drawing
    /// page, in one commit labelled [`CONVERT_SKETCH_PAGE_COMMIT_LABEL`] (ADR 0043 §4): the
    /// conversion of [`Document::convert_sketch_to_path`] (same node id, so every clone follows
    /// the paths) plus `kind = "drawing"`. `ids` are the fresh anchor ids, one per anchor.
    ///
    /// # Errors
    /// Refuses a page that is not a sketch page with a live owner, or ids that do not fit;
    /// nothing is written.
    pub fn convert_sketch_page(&self, page: &str, ids: &[AnchorId]) -> Result<(), SketchEditError> {
        let owner = self
            .page_sketch(page)
            .ok_or_else(|| SketchEditError("This page has no sketch".into()))?;
        self.batch(CONVERT_SKETCH_PAGE_COMMIT_LABEL, |doc| {
            doc.convert_sketch_uncommitted(owner, ids)?;
            crate::pages::write(&doc.loro().get_map(ROOT_MAP), page, KIND_FIELD, DRAWING);
            Ok(())
        })
    }

    /// Marks sketch page `page` deleted and removes its live owning sketch through the ordinary
    /// object removal, so every clone of it is repaired (`0049` criterion 31), in one commit
    /// labelled [`DELETE_SKETCH_PAGE_COMMIT_LABEL`] (ADR 0043 §5). Called by
    /// [`Document::delete_page`] for a sketch page with a sketch, whose closing moves `close`
    /// writes first, in the same step (ADR 0051 §3a), so a clone that moves is repaired where it
    /// lands.
    pub(crate) fn delete_sketch_page(
        &self,
        page: &str,
        owner: NodeId,
        close: impl FnOnce(&Self) -> Result<(), crate::ObjectEditError>,
    ) -> Result<(), PageError> {
        self.batch(DELETE_SKETCH_PAGE_COMMIT_LABEL, |doc| {
            close(doc)?;
            crate::pages::write(&doc.loro().get_map(ROOT_MAP), page, "deleted", true);
            doc.delete_objects(&[owner]).map_err(PageError::from)
        })
    }

    /// Places a linked clone of the sketch `original`, moved by `translation`, with own placement
    /// on, on top of the creation parent (the active layer), in one commit labelled
    /// [`PLACE_SKETCH_CLONE_COMMIT_LABEL`] (ADR 0043 §6). Returns the clone.
    ///
    /// # Errors
    /// Refuses an `original` that is not the owning sketch of a live sketch page, or a
    /// translation past the clone matrix limit (or not finite); nothing is written.
    pub fn place_linked_clone(
        &self,
        original: NodeId,
        translation: Vec2,
    ) -> Result<NodeId, SketchEditError> {
        if !self.is_fixed_sketch(original) {
            return Err(SketchEditError(
                "Only a page's sketch is placed this way".into(),
            ));
        }
        let matrix = Affine::translation(translation);
        if !matrix.is_within_limit() {
            return Err(SketchEditError("The place is too far out".into()));
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let parent = self.drawing_parent(&tree);
        let failed = |error: loro::LoroError| SketchEditError(error.to_string());
        let created: TreeID = tree.create(parent).map_err(failed)?;
        let meta = tree.get_meta(created).map_err(failed)?;
        write_clone(
            &meta,
            CloneLink {
                origin: original,
                matrix,
            },
        );
        write_flag(&meta, KEY_OWN_PLACEMENT, true);
        self.commit_with_label(PLACE_SKETCH_CLONE_COMMIT_LABEL);
        Ok(NodeId::from_parts(created.peer, created.counter))
    }

    /// The owning sketch of `page` when it is a sketch page; the branch [`Document::delete_page`]
    /// takes for it.
    pub(crate) fn sketch_to_delete_with(&self, page: &str, kind: &PageKind) -> Option<NodeId> {
        (*kind == PageKind::Sketch)
            .then(|| self.page_sketch(page))
            .flatten()
            .filter(|&owner| {
                crate::path_codec::node_exists(
                    &self.loro().get_tree(OBJECTS_TREE),
                    tree_id_of(owner),
                )
            })
    }
}
