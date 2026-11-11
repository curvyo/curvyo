//! Stores one atomic affine placement for local Spiro controls.

use crate::{Affine, CurveKind, CurveKindError, Document, NodeId};
use loro::{LoroMap, LoroValue};

pub(crate) const KEY: &str = "spiro_transform";

#[allow(clippy::option_option)] // Distinguish absent identity from malformed data.
pub(crate) fn read(meta: &LoroMap) -> Option<Option<Affine>> {
    let Some(value) = meta.get(KEY) else {
        return Some(None);
    };
    let LoroValue::List(values) = value.get_deep_value() else {
        return None;
    };
    if values.len() != 6 || crate::curve_kind::read(meta)? != CurveKind::Spiro {
        return None;
    }
    let mut entries = [0.0; 6];
    for (entry, value) in entries.iter_mut().zip(values.iter()) {
        *entry = crate::path_codec::as_f64(value)?;
    }
    Some(Some(Affine::try_from(entries).ok()?))
}

pub(crate) fn write(meta: &LoroMap, matrix: Affine) {
    let values: Vec<_> = matrix
        .to_array()
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    // invariant: the attached path metadata accepts a plain list value.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY, LoroValue::from(values)).unwrap();
}

impl Document {
    pub(crate) fn ensure_spiro_format(&self) {
        self.commit_migration(|document| {
            let root = document.loro().get_map(crate::document::ROOT_MAP);
            let current = root.get(crate::document::KEY_FORMAT_VERSION).map(|value| value.get_deep_value());
            if !matches!(current, Some(LoroValue::I64(value)) if value >= i64::from(crate::CURRENT_FORMAT_VERSION)) {
                // invariant: the attached root map accepts an integer format version.
                #[allow(clippy::unwrap_used)]
                root.insert(crate::document::KEY_FORMAT_VERSION, crate::CURRENT_FORMAT_VERSION).unwrap();
            }
        });
    }

    /// Replaces a Spiro path's local-to-document placement in one history step.
    ///
    /// # Errors
    /// Rejects shared documents, missing paths, non-Spiro paths and invalid placements.
    pub fn set_spiro_transform(&self, id: NodeId, matrix: Affine) -> Result<(), CurveKindError> {
        if self.extended_edits_blocked() {
            return Err(CurveKindError::Shared);
        }
        let (meta, _) = self.path_parts(id)?;
        if crate::curve_kind::read(&meta) != Some(CurveKind::Spiro)
            || !matrix.is_finite()
            || matrix.inverse_representable().is_none()
        {
            return Err(CurveKindError::NeedsConversion);
        }
        self.ensure_spiro_format();
        write(&meta, matrix);
        self.commit_with_label("set_spiro_transform");
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{AnchorId, NewAnchor, Point, Vec2};

    #[test]
    fn legacy_format_is_migrated_before_translation_and_survives_undo() {
        let doc = Document::new(410);
        let path = doc
            .create_editable_curve(
                &[
                    NewAnchor::corner(AnchorId::new(410, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(410, 2), Point::new(1.0, 0.0)),
                ],
                false,
                CurveKind::Spiro,
            )
            .unwrap();
        let root = doc.loro().get_map(crate::document::ROOT_MAP);
        root.insert(
            crate::document::KEY_FORMAT_VERSION,
            crate::CURRENT_FORMAT_VERSION - 1,
        )
        .unwrap();
        doc.commit_migration(|_| {});
        doc.translate_objects(&[path], Vec2::new(4.0, 5.0)).unwrap();
        assert_eq!(
            root.get(crate::document::KEY_FORMAT_VERSION)
                .unwrap()
                .get_deep_value(),
            LoroValue::I64(i64::from(crate::CURRENT_FORMAT_VERSION))
        );
        let step = doc.own_steps().last().unwrap().step;
        doc.undo_step(step).unwrap();
        assert_eq!(
            root.get(crate::document::KEY_FORMAT_VERSION)
                .unwrap()
                .get_deep_value(),
            LoroValue::I64(i64::from(crate::CURRENT_FORMAT_VERSION))
        );
        assert_eq!(doc.path(path).unwrap().spiro_transform, None);
    }
    #[test]
    fn unlink_of_legacy_spiro_clone_migrates_before_writing_placement() {
        let doc = Document::new(411);
        let path = doc
            .create_editable_curve(
                &[
                    NewAnchor::corner(AnchorId::new(411, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(411, 2), Point::new(1.0, 0.0)),
                ],
                false,
                CurveKind::Spiro,
            )
            .unwrap();
        let clone = doc.linked_clone(&[path], &[]).unwrap()[0];
        let root = doc.loro().get_map(crate::document::ROOT_MAP);
        root.insert(
            crate::document::KEY_FORMAT_VERSION,
            crate::CURRENT_FORMAT_VERSION - 1,
        )
        .unwrap();
        doc.commit_migration(|_| {});
        doc.unlink_clones(&[crate::CopySource {
            id: clone,
            anchor_ids: vec![AnchorId::new(411, 3), AnchorId::new(411, 4)],
        }])
        .unwrap();
        assert_eq!(
            root.get(crate::document::KEY_FORMAT_VERSION)
                .unwrap()
                .get_deep_value(),
            LoroValue::I64(i64::from(crate::CURRENT_FORMAT_VERSION))
        );
        assert!(doc.path(clone).unwrap().spiro_transform.is_some());
        let step = doc.own_steps().last().unwrap().step;
        doc.undo_step(step).unwrap();
        assert_eq!(
            root.get(crate::document::KEY_FORMAT_VERSION)
                .unwrap()
                .get_deep_value(),
            LoroValue::I64(i64::from(crate::CURRENT_FORMAT_VERSION))
        );
    }
}
