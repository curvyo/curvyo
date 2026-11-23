//! The commands of a generated object (ADR 0080 §4, §6): create, change the record and frame in
//! one commit, convert to paths. The generator itself lives in an extension crate the document
//! cannot call; the caller passes in what it generated.

use loro::TreeParentId;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::generated_codec::{
    self, GeneratedRecord, GeneratedSnapshot, frame_is_valid, is_generated_meta,
};
use crate::path_codec::leaf_exists;
use crate::path_model::{NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::shape_codec;

/// The label of [`Document::create_generated`].
pub const ADD_GENERATED_COMMIT_LABEL: &str = "add_generated";
/// The label of a parameter edit, [`Document::change_generated`].
pub const CHANGE_PARAMETER_COMMIT_LABEL: &str = "change_parameter";
/// The label of [`Document::convert_generated`].
pub const CONVERT_GENERATED_COMMIT_LABEL: &str = "convert_generated";
/// The commit field that names the kind a generated object's step is about.
pub const KEY_SUBJECT_KIND: &str = "gk";
/// The commit field that names the parameter a step changed.
pub const KEY_SUBJECT_PARAM: &str = "gp";

/// Why a generated-object command refused to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GeneratedError {
    /// No object with this id exists.
    #[error("no such object")]
    NoSuchObject,
    /// The object is not a generated object.
    #[error("the object is not a generated object")]
    NotGenerated,
    /// The record or frame breaks a rule of the stored shape (ADR 0080 §4).
    #[error("the generated record or frame is not valid")]
    Invalid,
    /// A converted outline takes an anchor id another path holds, or one id twice.
    #[error("an anchor id is used twice")]
    AnchorIds,
}

/// Whether `record` follows every rule the reader enforces.
fn record_is_valid(record: &GeneratedRecord) -> bool {
    let kind_ok = record
        .kind
        .split_once('/')
        .is_some_and(|(extension, name)| !extension.is_empty() && !name.is_empty());
    kind_ok
        && !record.parts.is_empty()
        && record.parts.iter().all(|part| {
            !part.is_empty()
                && part.iter().all(|subpath| {
                    subpath.len() >= 8
                        && (subpath.len() - 2) % 6 == 0
                        && subpath.iter().all(|n| n.is_finite() && n.abs() <= 1e7)
                })
        })
}

impl Document {
    /// The generated object `id` as stored, `None` when it is not one or cannot be read.
    #[must_use]
    pub fn generated(&self, id: NodeId) -> Option<GeneratedSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return None;
        }
        let meta = tree.get_meta(tree_id).ok()?;
        if !is_generated_meta(&meta) {
            return None;
        }
        generated_codec::read_snapshot(id, &meta)
    }

    /// Whether `id` is a generated object, read from its shape tag alone (no record decoded): the
    /// Node tool leaves such an object out of its editing set.
    #[must_use]
    pub fn is_generated(&self, id: NodeId) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        leaf_exists(&tree, tree_id)
            && tree
                .get_meta(tree_id)
                .is_ok_and(|meta| is_generated_meta(&meta))
    }

    /// Creates a generated object with `record` and `frame` on top of the active layer, in one
    /// commit labelled [`ADD_GENERATED_COMMIT_LABEL`].
    ///
    /// # Errors
    /// [`GeneratedError::Invalid`] for a record or frame that breaks the stored shape.
    pub fn create_generated(
        &self,
        record: &GeneratedRecord,
        frame: Affine,
    ) -> Result<NodeId, GeneratedError> {
        self.create_generated_with(record, frame, false)
    }

    /// [`Document::create_generated`] with the seed set from the new node's id (ADR 0029 §5,
    /// ADR 0080 §2): the id's peer and counter folded into a `u32`, so a copy keeps it and two
    /// objects differ.
    ///
    /// # Errors
    /// As [`Document::create_generated`].
    pub fn create_generated_seeded(
        &self,
        record: &GeneratedRecord,
        frame: Affine,
    ) -> Result<NodeId, GeneratedError> {
        self.create_generated_with(record, frame, true)
    }

    fn create_generated_with(
        &self,
        record: &GeneratedRecord,
        frame: Affine,
        seeded: bool,
    ) -> Result<NodeId, GeneratedError> {
        if !record_is_valid(record) || !frame_is_valid(frame) {
            return Err(GeneratedError::Invalid);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root or a live group on a fresh tree handle
        // cannot fail.
        #[allow(clippy::unwrap_used)]
        let tree_id = tree.create(self.drawing_parent(&tree)).unwrap();
        // invariant: reading the meta map of a node this call just created cannot fail.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(tree_id).unwrap();
        let mut stored = record.clone();
        if seeded {
            // Folded, never wider than a `u32`: the low bits of the counter mixed with the peer.
            #[allow(clippy::cast_possible_truncation)]
            let folded = tree_id.counter.cast_unsigned() ^ (tree_id.peer as u32);
            stored.seed = folded;
        }
        generated_codec::write_generated(&meta, &stored, frame);
        crate::style_codec::write_creation_style(&meta);
        self.commit_with_label_and(
            ADD_GENERATED_COMMIT_LABEL,
            &[(KEY_SUBJECT_KIND, record.kind.clone())],
        );
        Ok(NodeId::from_parts(tree_id.peer, tree_id.counter))
    }

    /// Writes a new record (and the frame, when it differs) of the generated object `id` in one
    /// commit under `label`, with `subject` as the step's fields. A register that is unchanged is
    /// not rewritten, so a concurrent move survives a parameter edit.
    ///
    /// # Errors
    /// [`GeneratedError::NoSuchObject`], [`GeneratedError::NotGenerated`] or
    /// [`GeneratedError::Invalid`].
    pub fn change_generated(
        &self,
        id: NodeId,
        record: &GeneratedRecord,
        frame: Affine,
        label: &str,
        subject: &[(&str, String)],
    ) -> Result<(), GeneratedError> {
        if !record_is_valid(record) || !frame_is_valid(frame) {
            return Err(GeneratedError::Invalid);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return Err(GeneratedError::NoSuchObject);
        }
        let meta = tree
            .get_meta(tree_id)
            .map_err(|_| GeneratedError::NoSuchObject)?;
        if !is_generated_meta(&meta) {
            return Err(GeneratedError::NotGenerated);
        }
        if generated_codec::read_record(&meta).as_ref() != Some(record) {
            generated_codec::write_record(&meta, record);
        }
        if generated_codec::read_frame(&meta) != Some(frame) {
            generated_codec::write_frame(&meta, frame);
        }
        self.commit_with_label_and(label, subject);
        Ok(())
    }

    /// The affine map that takes the stored outline of the generated object `id` to the outline of
    /// `result`, `None` when `result` is not such an image (a node edit) or `id` is not generated:
    /// what a resize reads its factor from.
    #[must_use]
    pub fn generated_map(&self, id: NodeId, result: &crate::ObjectSnapshot) -> Option<Affine> {
        let snapshot = self.generated(id)?;
        let crate::ObjectSnapshot::Path(path) = result else {
            return None;
        };
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(tree_id_of(id)).ok()?;
        let current =
            crate::generated_outline::outline(&snapshot, crate::style_codec::read_style(&meta));
        crate::outline_map::outline_map(&current, path)
    }

    /// Writes only the frame of the generated object `id` (a move, rotate or flip), in one commit
    /// under `label`; the record is not touched, so it merges with a concurrent parameter edit.
    ///
    /// # Errors
    /// As [`Document::change_generated`].
    pub fn write_generated_frame(
        &self,
        id: NodeId,
        frame: Affine,
        label: &str,
    ) -> Result<(), GeneratedError> {
        if !frame_is_valid(frame) {
            return Err(GeneratedError::Invalid);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return Err(GeneratedError::NoSuchObject);
        }
        let meta = tree
            .get_meta(tree_id)
            .map_err(|_| GeneratedError::NoSuchObject)?;
        if !is_generated_meta(&meta) {
            return Err(GeneratedError::NotGenerated);
        }
        generated_codec::write_frame(&meta, frame);
        self.commit_with_label(label);
        Ok(())
    }

    /// Converts the generated object `id` into plain paths in one commit labelled
    /// [`CONVERT_GENERATED_COMMIT_LABEL`], under the same id: with one part the node becomes that
    /// path (compound when the part has holes); with several it becomes a group holding one path
    /// per part. `parts` are the outlines in document millimetres with their fresh anchor ids.
    /// Its style, name and operation assignments stay on the node.
    ///
    /// # Errors
    /// [`GeneratedError::NoSuchObject`], [`GeneratedError::NotGenerated`],
    /// [`GeneratedError::Invalid`] for no part or an empty outline and
    /// [`GeneratedError::AnchorIds`] for an id used twice or one another path holds.
    ///
    /// # Panics
    /// Does not panic in practice: every write below goes to a node this call just confirmed
    /// live or just created.
    pub fn convert_generated(
        &self,
        id: NodeId,
        parts: &[Vec<(Vec<NewAnchor>, bool)>],
    ) -> Result<(), GeneratedError> {
        if parts.is_empty()
            || parts
                .iter()
                .any(|part| part.is_empty() || part.iter().any(|(anchors, _)| anchors.len() < 2))
        {
            return Err(GeneratedError::Invalid);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return Err(GeneratedError::NoSuchObject);
        }
        let meta = tree
            .get_meta(tree_id)
            .map_err(|_| GeneratedError::NoSuchObject)?;
        if !is_generated_meta(&meta) {
            return Err(GeneratedError::NotGenerated);
        }
        let mut seen = std::collections::HashSet::new();
        let all_distinct = parts
            .iter()
            .flatten()
            .flat_map(|(anchors, _)| anchors.iter())
            .all(|anchor| seen.insert(anchor.id));
        if !all_distinct || crate::replace::check_new_anchor_ids(&tree, &[tree_id], &seen).is_err()
        {
            return Err(GeneratedError::AnchorIds);
        }
        let single = match parts {
            [only] => only.split_first(),
            _ => None,
        };
        generated_codec::strip(&meta);
        if let Some((first, rest)) = single {
            shape_codec::write_converted_path_fields(&meta, &first.0, first.1);
            crate::subpath_codec::append_outlines(&meta, rest);
        } else {
            // invariant: the node is a live leaf; a group marker is a plain bool insert.
            #[allow(clippy::unwrap_used)]
            meta.insert(crate::group_tree::KEY_GROUP, true).unwrap();
            let style = crate::style_codec::read_style(&meta);
            for part in parts {
                // invariant: the node was just confirmed live and is now a group.
                #[allow(clippy::unwrap_used)]
                let child = tree.create(TreeParentId::Node(tree_id)).unwrap();
                crate::replace::write_path_node(&tree, child, &style, part);
            }
        }
        self.commit_with_label(CONVERT_GENERATED_COMMIT_LABEL);
        Ok(())
    }
}
