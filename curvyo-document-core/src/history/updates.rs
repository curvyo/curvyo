//! The seam through which two replicas of a document exchange updates as bytes, until the sync
//! layer exists (`specs/0020-undo-redo/adrs.md` decision 12). No Loro type appears in it.

use loro::ExportMode;

use super::error::SyncError;
use crate::document::{Document, DocumentVersion, OBJECTS_TREE};

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
        let (epoch, bytes) = unframe(bytes).ok_or(SyncError::Malformed)?;
        // A replica of another epoch still holds steps this one removed, or the other way round:
        // Loro would not say so reliably, so nothing is merged (ADR 0014 §9).
        if epoch.as_deref() != self.history_epoch().as_deref() {
            return Err(SyncError::EpochMismatch);
        }
        let status = self
            .loro()
            .import(bytes)
            .map_err(|_| SyncError::Malformed)?;
        if status.pending.is_some() {
            return Err(SyncError::Pending);
        }
        if !crate::path_codec::validate_path_tree(self.loro(), OBJECTS_TREE) {
            return Err(SyncError::Malformed);
        }
        Ok(())
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
