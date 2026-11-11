//! The seam through which two replicas of a document exchange updates as bytes, until the sync
//! layer exists (`specs/0020-undo-redo/adrs.md` decision 12). No Loro type appears in it.

use loro::{
    ContainerID, ContainerType, ExportMode, JsonMapOp, JsonOpContent, LoroDoc, LoroValue,
    ValueOrContainer, VersionVector,
};

use super::error::SyncError;
use crate::document::{
    CURRENT_FORMAT_VERSION, Document, DocumentVersion, KEY_FORMAT_VERSION, OBJECTS_TREE, ROOT_MAP,
};

pub(crate) fn format_is_readable(doc: &LoroDoc) -> bool {
    let root = doc.get_map(ROOT_MAP);
    match root.get(KEY_FORMAT_VERSION) {
        Some(ValueOrContainer::Value(LoroValue::I64(version))) => {
            version >= 0 && version <= i64::from(CURRENT_FORMAT_VERSION)
        }
        None => {
            // Some legacy files predate the root register. Extended-path history always
            // writes both the version and a persistent marker; without the version it
            // cannot be interpreted safely, even if the marker was later deleted.
            root.get(crate::width_profile::HISTORY_MARK).is_none()
                && !crate::width_profile::history_marker_written(doc, &VersionVector::default())
        }
        Some(_) => false,
    }
}

pub(crate) fn writes_newer_format(doc: &LoroDoc, before: &VersionVector) -> bool {
    let root = ContainerID::new_root(ROOT_MAP, ContainerType::Map);
    doc.export_json_updates(before, &doc.oplog_vv())
        .changes
        .iter()
        .flat_map(|change| &change.ops)
        .any(|op| {
            op.container == root
                && matches!(&op.content,
            JsonOpContent::Map(JsonMapOp::Insert { key, value: LoroValue::I64(version) })
                if key == KEY_FORMAT_VERSION && *version > i64::from(CURRENT_FORMAT_VERSION))
        })
}

impl Document {
    /// Everything this replica has done since `since`, as bytes another replica can import.
    /// A version this replica does not know yields everything.
    ///
    /// # Errors
    /// [`SyncError::Encode`] if the updates cannot be encoded.
    pub fn export_updates(&self, since: &DocumentVersion) -> Result<Vec<u8>, SyncError> {
        self.assert_outside_batch();
        self.loro().commit();
        let known = self.loro().frontiers_to_vv(&since.0).unwrap_or_default();
        let updates = self
            .loro()
            .export(ExportMode::updates(&known))
            .map_err(|_| SyncError::Encode)?;
        Ok(frame(self.history_epoch().as_deref(), &updates))
    }

    /// Merges updates exported by another replica. The document is checked like an opened file:
    /// updates that would break the shape of the objects are refused.
    ///
    /// # Errors
    /// [`SyncError::Malformed`] for bytes that are not updates, or that leave the objects tree in
    /// a shape this build cannot read; [`SyncError::Pending`] when part of the updates cannot be
    /// applied because the history they continue is missing here.
    pub fn import_updates(&self, bytes: &[u8]) -> Result<(), SyncError> {
        self.import_updates_checked(bytes, false)
    }

    /// Imports updates for a shared document, refusing extended path semantics.
    ///
    /// # Errors
    /// Returns the same errors as [`Document::import_updates`], including malformed updates
    /// that contain extended path history.
    pub fn import_shared_updates(&self, bytes: &[u8]) -> Result<(), SyncError> {
        self.import_updates_checked(bytes, true)
    }

    fn import_updates_checked(&self, bytes: &[u8], shared: bool) -> Result<(), SyncError> {
        let (epoch, bytes) = unframe(bytes).ok_or(SyncError::Malformed)?;
        // A replica of another epoch still holds steps this one removed, or the other way round:
        // Loro would not say so reliably, so nothing is merged (ADR 0014 §9).
        if epoch.as_deref() != self.history_epoch().as_deref() {
            return Err(SyncError::EpochMismatch);
        }
        // A peer's updates may carry membership keys: the sweep is on from here (ADR 0046 §3).
        self.member_keys_may_exist.set(true);
        // Loro can apply part of a malformed or incomplete update before reporting the
        // problem. Validate a fork first so rejected data cannot mutate the live document.
        let trial = self.loro().fork();
        let before = trial.oplog_vv();
        let status = trial.import(bytes).map_err(|_| SyncError::Malformed)?;
        if status.pending.is_some() {
            return Err(SyncError::Pending);
        }
        let alone = LoroDoc::new();
        alone.import(bytes).map_err(|_| SyncError::Malformed)?;
        if writes_newer_format(&alone, &VersionVector::default())
            || writes_newer_format(&trial, &before)
            || !format_is_readable(&trial)
            || !crate::path_codec::validate_path_tree(&trial, OBJECTS_TREE)
        {
            return Err(SyncError::Malformed);
        }
        if shared && crate::width_profile::history_marker_written(&trial, &before) {
            return Err(SyncError::Malformed);
        }
        self.loro()
            .import(bytes)
            .map_err(|_| SyncError::Malformed)?;
        Ok(())
    }
}

impl Document {
    /// A document built from the updates of a stream that starts at its creation (a document
    /// another member created in a shared workspace, `specs/0095-workspace` criterion 35), with
    /// the checks of opening a file.
    ///
    /// # Errors
    /// [`SyncError::Malformed`] for bytes that are no updates or a state that does not open;
    /// [`SyncError::Pending`] when the updates depend on missing history.
    pub fn from_updates(peer_id: u64, bytes: &[u8]) -> Result<Self, SyncError> {
        let (_, updates) = unframe(bytes).ok_or(SyncError::Malformed)?;
        let trial = LoroDoc::new();
        if trial
            .import(updates)
            .map_err(|_| SyncError::Malformed)?
            .pending
            .is_some()
        {
            return Err(SyncError::Pending);
        }
        Self::from_loro_snapshot(peer_id, updates).map_err(|_| SyncError::Malformed)
    }
}

/// The updates behind the history epoch of the sender: `[epoch length: u32][epoch][updates]`. The
/// epoch is compared before anything is merged.
fn frame(epoch: Option<&str>, updates: &[u8]) -> Vec<u8> {
    let epoch = epoch.unwrap_or_default().as_bytes();
    let length = u32::try_from(epoch.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(4 + epoch.len() + updates.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(epoch);
    bytes.extend_from_slice(updates);
    bytes
}

/// The inverse of [`frame`]; an empty epoch is none.
fn unframe(bytes: &[u8]) -> Option<(Option<String>, &[u8])> {
    let (length, rest) = bytes.split_first_chunk::<4>()?;
    let length = usize::try_from(u32::from_le_bytes(*length)).ok()?;
    let epoch = rest.get(..length)?;
    let updates = rest.get(length..)?;
    let epoch = std::str::from_utf8(epoch).ok()?;
    Some(((!epoch.is_empty()).then(|| epoch.to_owned()), updates))
}

#[cfg(test)]
mod format_readability_tests {
    use super::*;

    #[test]
    fn absent_legacy_register_is_allowed_only_without_extended_history() {
        let doc = LoroDoc::new();
        assert!(format_is_readable(&doc));
        let root = doc.get_map(ROOT_MAP);
        root.insert(crate::width_profile::HISTORY_MARK, true)
            .unwrap();
        doc.commit();
        assert!(!format_is_readable(&doc));
        root.delete(crate::width_profile::HISTORY_MARK).unwrap();
        doc.commit();
        assert!(!format_is_readable(&doc));
    }

    #[test]
    fn present_invalid_and_future_registers_are_rejected() {
        let doc = LoroDoc::new();
        let root = doc.get_map(ROOT_MAP);
        root.insert(KEY_FORMAT_VERSION, "bad").unwrap();
        assert!(!format_is_readable(&doc));
        root.insert(KEY_FORMAT_VERSION, i64::from(CURRENT_FORMAT_VERSION) + 1)
            .unwrap();
        assert!(!format_is_readable(&doc));
        root.insert(KEY_FORMAT_VERSION, -1_i64).unwrap();
        assert!(!format_is_readable(&doc));
        root.insert(KEY_FORMAT_VERSION, i64::from(CURRENT_FORMAT_VERSION))
            .unwrap();
        assert!(format_is_readable(&doc));
    }
}
