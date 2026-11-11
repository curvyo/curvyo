//! Independently mergeable width points attached to a path.

use loro::LoroMap;

use crate::{AnchorId, Document, NodeId, PathEditError};

pub(crate) const PREFIX: &str = "width_point.";
/// Persistent mark that CRDT history has contained extended path data.
pub(crate) const HISTORY_MARK: &str = "extended_path_history";

pub(crate) fn history_marker_written(doc: &loro::LoroDoc, since: &loro::VersionVector) -> bool {
    let root = loro::ContainerID::new_root(crate::document::ROOT_MAP, loro::ContainerType::Map);
    doc.export_json_updates(since, &doc.oplog_vv())
        .changes.iter().flat_map(|change| &change.ops).any(|op| {
            op.container == root && matches!(&op.content, loro::JsonOpContent::Map(loro::JsonMapOp::Insert { key, .. }) if key == HISTORY_MARK)
        })
}

/// A width point's stable identity, normalised position and width multiplier.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WidthPoint {
    /// Stable identity minted by the editing peer.
    pub id: AnchorId,
    /// Fraction of the centre-line arc length.
    pub position: f64,
    /// Multiplier of the resolved base stroke width.
    pub multiplier: f64,
}

/// A refused width-point edit.
#[derive(Debug, thiserror::Error)]
pub enum WidthPointError {
    /// The target path no longer exists.
    #[error(transparent)]
    Path(#[from] PathEditError),
    /// The position or multiplier is outside its allowed range.
    #[error("width point position or multiplier is invalid")]
    Invalid,
    /// Shared documents cannot yet safely exchange these points with older peers.
    #[error("width profiles are not available in shared documents yet")]
    Shared,
    /// Profiles currently apply only to a path with a single outline.
    #[error("width profiles require a single-outline path")]
    Compound,
}

fn key(id: AnchorId) -> String {
    format!("{PREFIX}{}", id.to_hex())
}

pub(crate) fn write(meta: &LoroMap, points: &[WidthPoint]) {
    for point in points {
        // invariant: inserting a plain value into an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert(&key(point.id), vec![point.position, point.multiplier])
            .unwrap();
    }
}

pub(crate) fn clear(meta: &LoroMap) {
    let keys: Vec<String> = meta
        .keys()
        .filter(|key| key.starts_with(PREFIX))
        .map(|key| key.to_string())
        .collect();
    for key in keys {
        // invariant: deleting a key from an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(&key).unwrap();
    }
}

fn valid(position: f64, multiplier: f64, closed: bool) -> bool {
    position.is_finite()
        && position >= 0.0
        && (position < 1.0 || (!closed && position <= 1.0))
        && multiplier.is_finite()
        && multiplier >= 0.0
}

pub(crate) fn valid_history_marker(loro: &loro::LoroDoc) -> bool {
    let root = loro.get_map(crate::document::ROOT_MAP);
    if let Some(value) = root.get(HISTORY_MARK) {
        return matches!(value.get_deep_value(), loro::LoroValue::Bool(true));
    }
    let tree = loro.get_tree(crate::document::OBJECTS_TREE);
    crate::group_tree::leaves_under(&tree, None)
        .into_iter()
        .all(|id| {
            tree.get_meta(id).is_ok_and(|meta| {
                !meta.keys().any(|name| name.starts_with(PREFIX))
                    && !crate::curve_kind::has_extended(&meta)
            })
        })
}

pub(crate) fn read(meta: &LoroMap, closed: bool) -> Option<Vec<WidthPoint>> {
    let mut points = Vec::new();
    for name in meta.keys().filter(|name| name.starts_with(PREFIX)) {
        let id = AnchorId::from_hex(name.strip_prefix(PREFIX)?)?;
        if name.as_str() != key(id) {
            return None;
        }
        let (position, multiplier) = crate::path_codec::read_xy(meta, &name)?;
        if !valid(position, multiplier, closed) {
            return None;
        }
        points.push(WidthPoint {
            id,
            position: if position == 0.0 { 0.0 } else { position },
            multiplier,
        });
    }
    points.sort_by(|a, b| {
        a.position
            .total_cmp(&b.position)
            .then(a.id.as_u128().cmp(&b.id.as_u128()))
    });
    Some(points)
}

impl Document {
    /// Removes the width points currently observed on a path in one undoable change.
    ///
    /// # Errors
    /// Returns an error if the path does not exist or belongs to a shared document.
    ///
    /// # Panics
    /// Does not panic in practice: deleting keys from an attached map cannot fail.
    pub fn reset_width_profile(&self, path: NodeId) -> Result<(), WidthPointError> {
        if self.extended_edits_blocked() {
            return Err(WidthPointError::Shared);
        }
        let (meta, _) = self.path_parts(path)?;
        let keys: Vec<_> = meta
            .keys()
            .filter(|name| name.starts_with(PREFIX))
            .collect();
        if !keys.is_empty() {
            for key in keys {
                // invariant: deleting a key from an attached map cannot fail.
                #[allow(clippy::unwrap_used)]
                meta.delete(&key).unwrap();
            }
            self.commit_with_label("reset_width_profile");
        }
        Ok(())
    }

    /// Whether the document's history carries path data older peers cannot interpret.
    #[must_use]
    pub fn has_extended_path_history(&self) -> bool {
        let doc = self.loro();
        if doc
            .get_map(crate::document::ROOT_MAP)
            .get(HISTORY_MARK)
            .is_some()
            || self.object_ids().into_iter().any(|id| {
                self.path_parts(id).is_ok_and(|(meta, _)| {
                    meta.keys().any(|name| name.starts_with(PREFIX))
                        || crate::curve_kind::has_extended(&meta)
                })
            })
        {
            return true;
        }
        history_marker_written(doc, &loro::VersionVector::default())
    }

    /// Removes one width point, if present, in one commit.
    ///
    /// # Errors
    /// Returns an error if the path does not exist.
    ///
    /// # Panics
    /// Does not panic in practice: deleting a key from an attached map cannot fail.
    pub fn remove_width_point(&self, path: NodeId, id: AnchorId) -> Result<(), WidthPointError> {
        if self.extended_edits_blocked() {
            return Err(WidthPointError::Shared);
        }
        let (meta, _) = self.path_parts(path)?;
        let name = key(id);
        if meta.get(&name).is_some() {
            // invariant: deleting a key from an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(&name).unwrap();
            self.commit_with_label("remove_width_point");
        }
        Ok(())
    }

    /// Reads the path's independently mergeable width points.
    ///
    /// # Errors
    /// Returns an error for a missing path or invalid stored data.
    pub fn width_points(&self, path: NodeId) -> Result<Vec<WidthPoint>, WidthPointError> {
        let (meta, _) = self.path_parts(path)?;
        read(&meta, crate::path_codec::read_closed(&meta)).ok_or(WidthPointError::Invalid)
    }

    /// Adds or updates one width point in a single commit.
    ///
    /// # Errors
    /// Returns an error for a missing path or invalid values.
    ///
    /// # Panics
    /// Does not panic in practice: writing a value to an attached map cannot fail.
    pub fn set_width_point(
        &self,
        path: NodeId,
        id: AnchorId,
        position: f64,
        multiplier: f64,
    ) -> Result<(), WidthPointError> {
        if self.extended_edits_blocked() {
            return Err(WidthPointError::Shared);
        }
        let (meta, _) = self.path_parts(path)?;
        if self.path(path).is_none_or(|path| path.is_compound()) {
            return Err(WidthPointError::Compound);
        }
        if !valid(position, multiplier, crate::path_codec::read_closed(&meta)) {
            return Err(WidthPointError::Invalid);
        }
        let root = self.loro().get_map(crate::document::ROOT_MAP);
        if root.get(HISTORY_MARK).is_none() {
            // invariant: writing a plain value to an attached root map cannot fail.
            #[allow(clippy::unwrap_used)]
            root.insert(HISTORY_MARK, true).unwrap();
        }
        // invariant: inserting a plain value into an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert(&key(id), vec![position, multiplier]).unwrap();
        self.commit_with_label("set_width_point");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewAnchor, Point};

    #[test]
    fn ordinary_wiped_history_remains_shareable() {
        let mut document = Document::new(17);
        let _ = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(17, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(17, 2), Point::new(10.0, 0.0)),
            ],
            false,
        );
        document.wipe_history().unwrap();
        assert!(!document.has_extended_path_history());
    }

    #[test]
    fn deleted_marker_and_deleted_path_remain_unsafe_to_share() {
        let document = Document::new(17);
        let controls = [
            NewAnchor::corner(AnchorId::new(17, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(17, 2), Point::new(10.0, 0.0)),
        ];
        let path = document
            .create_editable_curve(&controls, false, crate::CurveKind::BSpline)
            .unwrap();
        document.delete_objects(&[path]).unwrap();
        document
            .loro()
            .get_map(crate::document::ROOT_MAP)
            .delete(HISTORY_MARK)
            .unwrap();
        document.loro().commit();
        assert!(document.has_extended_path_history());
    }

    #[test]
    fn missing_history_marker_refuses_live_profile_on_open() {
        let document = Document::new(1);
        let path = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
            ],
            false,
        );
        document
            .set_width_point(path, AnchorId::new(1, 3), 0.5, 2.0)
            .unwrap();
        document
            .loro()
            .get_map(crate::document::ROOT_MAP)
            .delete(HISTORY_MARK)
            .unwrap();
        document.loro().commit();
        let bytes = document.export_loro_snapshot().unwrap();
        assert!(Document::from_loro_snapshot(2, &bytes).is_err());
    }
}
