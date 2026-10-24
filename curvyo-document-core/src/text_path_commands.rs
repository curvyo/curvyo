//! The document commands of a text on path (`specs/0080-text-on-path` criteria 4 to 7, 17 to 19,
//! 23; `adrs.md` decision 9): put a text on a carrier, change its values there, release it. Each
//! is one commit. Positions and placements are computed by the caller (`curvyo-ui-core`), which
//! knows the faces.

use loro::TreeID;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::NodeId;
use crate::text_path_model::{
    CarrierRef, OnPathLabel, OnPathValues, PUT_ON_PATH_LABEL, RELEASE_FROM_PATH_LABEL,
};
use crate::texts::TextEditError;

impl Document {
    /// Puts text `text` on `carrier` with `values` (criteria 4 to 6), in **one commit** labelled
    /// "Put text on path": the link and the values are written, a text on a path already moves to
    /// the new carrier; with `mark_guide` the carrier becomes a guide in the same commit unless it
    /// lies on a locked layer or group (criterion 7) or is one already. The text's placement is
    /// not written: it keeps the straight placement for Undo and Release.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when `text` is not a live text or `carrier` is not a live
    /// node; nothing is written.
    pub fn put_text_on_path(
        &self,
        text: NodeId,
        carrier: CarrierRef,
        values: &OnPathValues,
        mark_guide: bool,
    ) -> Result<(), TextEditError> {
        let meta = self.text_meta(text)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let carrier_tree = TreeID::new(carrier.node.peer, carrier.node.counter);
        if !crate::path_codec::leaf_exists(&tree, carrier_tree) || carrier.node == text {
            return Err(TextEditError::NoSuchText);
        }
        crate::text_path_codec::write_carrier(&meta, carrier);
        crate::text_path_codec::write_values(&meta, values);
        if mark_guide
            && !crate::node_read::locked_through(&tree, carrier_tree)
            && let Ok(carrier_meta) = tree.get_meta(carrier_tree)
            && !crate::node_registers::read_registers(&carrier_meta).guide
        {
            crate::guides::write_guide(&carrier_meta, true);
        }
        self.commit_with_label(PUT_ON_PATH_LABEL);
        Ok(())
    }

    /// Sets the on-path values of each text to the ones paired with it, in **one commit**
    /// labelled by `label` (criteria 15 to 19, 21); only the keys that differ are written, and a
    /// text that is not on a path is left alone. Nothing is committed when nothing changed.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when an id is not a live text; nothing is written.
    pub fn set_text_path(
        &self,
        texts: &[(NodeId, OnPathValues)],
        label: OnPathLabel,
    ) -> Result<(), TextEditError> {
        let metas: Vec<_> = texts
            .iter()
            .map(|(id, _)| self.text_meta(*id))
            .collect::<Result<_, _>>()?;
        let mut wrote = false;
        for (meta, (_, values)) in metas.iter().zip(texts) {
            if crate::text_path_codec::read_carrier(meta).is_some() {
                wrote |= crate::text_path_codec::write_values(meta, values);
            }
        }
        if wrote {
            self.commit_with_label(label.label());
        }
        Ok(())
    }

    /// Releases each text from its path (criterion 23): every on-path key is deleted and the
    /// straight placement paired with it written, in **one commit** labelled "Release from
    /// path". The carrier is not written (criterion 4a).
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] or [`TextEditError::InvalidPlacement`]; nothing is written.
    pub fn release_from_path(&self, texts: &[(NodeId, Affine)]) -> Result<(), TextEditError> {
        for (_, placement) in texts {
            if !placement.is_within_limit()
                || placement.determinant().abs() <= crate::affine::SINGULAR_DETERMINANT
            {
                return Err(TextEditError::InvalidPlacement);
            }
        }
        let metas: Vec<_> = texts
            .iter()
            .map(|(id, _)| self.text_meta(*id))
            .collect::<Result<_, _>>()?;
        let mut wrote = false;
        for (meta, (_, placement)) in metas.iter().zip(texts) {
            if crate::text_path_codec::delete_all(meta) {
                crate::text_codec::write_placement(meta, *placement);
                wrote = true;
            }
        }
        if wrote {
            self.commit_with_label(RELEASE_FROM_PATH_LABEL);
        }
        Ok(())
    }
}
