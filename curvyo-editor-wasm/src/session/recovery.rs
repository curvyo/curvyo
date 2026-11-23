//! The session's side of the recovery copy (`specs/0136-autosave-and-restore` criteria 1 and 2,
//! ADR 0079 decision 4): which steps the page has seen, the delta the recovery worker's replica
//! has not been handed, and the assets the host has not been handed. The page decides when
//! (`curvyo-ui-core`'s schedule); this only answers cheaply, on the canvas thread.

use std::collections::HashSet;

use curvyo_document_core::{DocumentVersion, RecoveryAsset, SyncError, recovery_assets};

use super::Session;

/// What the replica is handed.
pub struct RecoveryDelta {
    /// A snapshot that seeds the replica, not updates.
    pub full: bool,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// What the page has been handed so far.
#[derive(Default)]
pub(super) struct Handoff {
    /// The version at the last [`Session::recovery_changed`]; `None` before the first call.
    seen: Option<DocumentVersion>,
    /// The version the worker's replica holds; `None` when it holds nothing.
    handed: Option<DocumentVersion>,
    /// The asset names the host has.
    assets: HashSet<String>,
}

impl Session {
    /// Opens a restored recovery copy: the project differs from every saved file, so it is dirty
    /// and has no path (`specs/0136-autosave-and-restore` criterion 5).
    ///
    /// # Errors
    /// See [`Session::open`].
    pub fn open_recovered(
        peer: u64,
        bytes: &[u8],
    ) -> Result<Self, curvyo_document_core::OpenError> {
        let mut session = Self::open(peer, bytes)?;
        session.saved = DocumentVersion::never();
        Ok(session)
    }

    /// Whether a step was committed since the last call: the page tells the schedule. The first
    /// call only remembers the state (an opened or new project is not a step).
    pub fn recovery_changed(&mut self) -> bool {
        let version = self.document.version();
        let changed = self
            .recovery
            .seen
            .as_ref()
            .is_some_and(|seen| *seen != version);
        self.recovery.seen = Some(version);
        changed
    }

    /// What the replica is handed next: the updates since the last hand-over (cheap for a step),
    /// or, the first time and after [`Session::recovery_reseed`], a snapshot of the document. The
    /// page runs a reseed at an idle moment, never inside a step.
    ///
    /// # Errors
    /// [`SyncError::Encode`] if the bytes cannot be encoded.
    pub fn recovery_export(&mut self) -> Result<RecoveryDelta, SyncError> {
        let version = self.document.version();
        let delta = match self.recovery.handed.clone() {
            Some(since) => RecoveryDelta {
                full: false,
                bytes: self.document.export_updates(&since)?,
            },
            None => RecoveryDelta {
                full: true,
                bytes: self
                    .document
                    .export_loro_snapshot()
                    .map_err(|_| SyncError::Encode)?,
            },
        };
        self.recovery.handed = Some(version);
        Ok(delta)
    }

    /// Whether the next [`Session::recovery_export`] is a full one.
    #[must_use]
    pub fn recovery_is_seeded(&self) -> bool {
        self.recovery.handed.is_some()
    }

    /// The replica is gone or went its own way (a worker restart, a wiped history): the next
    /// export is a full one.
    pub fn recovery_reseed(&mut self) {
        self.recovery.handed = None;
    }

    /// A new session key: nothing has been handed to the replica or the host.
    pub fn recovery_start_over(&mut self) {
        self.recovery.handed = None;
        self.recovery.assets.clear();
    }

    /// The assets the host has not been handed yet.
    #[must_use]
    pub fn recovery_new_assets(&self) -> Vec<RecoveryAsset> {
        recovery_assets(&self.document)
            .into_iter()
            .filter(|asset| !self.recovery.assets.contains(&asset.name))
            .collect()
    }

    /// The host has these assets.
    pub fn recovery_assets_done(&mut self, names: impl IntoIterator<Item = String>) {
        self.recovery.assets.extend(names);
    }

    /// The pages of the document, for the record.
    #[must_use]
    pub fn recovery_pages(&self) -> usize {
        self.document.pages().len()
    }
}
