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
        self.loro()
            .export(ExportMode::updates(&known))
            .map_err(|_| SyncError::Encode)
    }

    /// Merges updates exported by another replica. The document is checked like an opened file:
    /// updates that would break the shape of the objects are refused.
    ///
    /// # Errors
    /// [`SyncError::Malformed`] for bytes that are not updates, or that leave the objects tree in
    /// a shape this build cannot read; [`SyncError::Pending`] when part of the updates cannot be
    /// applied because the history they continue is missing here.
    pub fn import_updates(&self, bytes: &[u8]) -> Result<(), SyncError> {
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
