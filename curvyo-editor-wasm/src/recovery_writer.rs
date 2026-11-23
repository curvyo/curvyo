//! The recovery worker's replica (`specs/0136-autosave-and-restore` criterion 2, ADR 0079
//! decision 4): it holds a copy of the open document, fed the deltas the page exports with
//! `Session::recovery_export`, and packs it as an asset-less recovery container. It runs in a
//! Web Worker with its own wasm instance, so the open document's frames never wait for the
//! packing; the page hands the bytes to the host.

use curvyo_document_core::{Document, SyncError, pack_recovery};

use crate::session::APP_VERSION;

/// Why the replica did not take a delta or could not pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryWriterError {
    /// The delta continues a history the replica lacks, or the history was wiped: the page
    /// reseeds with a full export.
    Reseed,
    /// The bytes are no updates this build reads.
    Rejected,
    /// The replica could not be packed.
    Pack,
}

impl core::fmt::Display for RecoveryWriterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Reseed => "the replica needs a full export",
            Self::Rejected => "the recovery update could not be read",
            Self::Pack => "the recovery copy could not be packed",
        })
    }
}

impl std::error::Error for RecoveryWriterError {}

/// A packed copy and the page count the record shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedCopy {
    /// The container without assets.
    pub bytes: Vec<u8>,
    /// The page count.
    pub pages: u32,
}

/// The replica of one session.
#[derive(Default)]
pub struct RecoveryWriter {
    document: Option<Document>,
}

impl RecoveryWriter {
    /// A writer that holds nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets the replica (a new session key).
    pub fn reset(&mut self) {
        self.document = None;
    }

    /// Whether the replica holds a document.
    #[must_use]
    pub fn is_seeded(&self) -> bool {
        self.document.is_some()
    }

    /// Takes what the page handed: a snapshot (`full`) replaces the replica, updates merge into
    /// it.
    ///
    /// # Errors
    /// [`RecoveryWriterError::Reseed`] when the page has to send a snapshot again.
    pub fn apply(
        &mut self,
        peer: u64,
        full: bool,
        bytes: &[u8],
    ) -> Result<(), RecoveryWriterError> {
        if full {
            self.document = Some(Document::from_snapshot(peer, bytes).map_err(reading)?);
            return Ok(());
        }
        let Some(document) = self.document.as_ref() else {
            return Err(RecoveryWriterError::Reseed);
        };
        document.import_updates(bytes).map_err(|error| {
            let error = reading(error);
            if error == RecoveryWriterError::Reseed {
                self.document = None;
            }
            error
        })
    }

    /// Packs the replica; `None` while it holds nothing.
    ///
    /// # Errors
    /// [`RecoveryWriterError::Pack`].
    pub fn pack(&self) -> Result<Option<PackedCopy>, RecoveryWriterError> {
        let Some(document) = self.document.as_ref() else {
            return Ok(None);
        };
        let bytes = pack_recovery(document, APP_VERSION).map_err(|_| RecoveryWriterError::Pack)?;
        let pages = u32::try_from(document.pages().len()).unwrap_or(u32::MAX);
        Ok(Some(PackedCopy { bytes, pages }))
    }
}

fn reading(error: SyncError) -> RecoveryWriterError {
    match error {
        SyncError::Pending | SyncError::EpochMismatch => RecoveryWriterError::Reseed,
        _ => RecoveryWriterError::Rejected,
    }
}
