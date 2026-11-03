//! Take-in of a document shared on its own into a shared workspace (`specs/0094-workspace`
//! criteria 42 and 49, ADR 0036 decision 10, WS-SEPARATE B): the membership check, the sealed
//! `Moved` record a short-lived session of the document's own room appends there, and what a
//! member of that room does when it reads `Moved`.
//!
//! The document's state goes into the workspace through `workspace_adopt_document` first; the
//! host then replaces its file (`share.json` v2, no keyring).

use curvyo_crypto_core::Identity;
use curvyo_crypto_core::keyring::{Keyring, decode_log};
use curvyo_document_core::unpack;
use curvyo_sync_core::{Moved, RecordKind, decode_moved, encode_moved};
use curvyo_ui_core::workspace_tree_text::moved_into_workspace_text;

use super::Session;
use super::sync::{SyncEnd, TransportAction};

/// Whether a document file may join the shared workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeIn {
    /// Not shared on its own: it joins with a fresh document id (criterion 41).
    Plain,
    /// Shared on its own with `people` participants, every one a member of the workspace: it is
    /// taken in after a confirm (criterion 42).
    Allowed {
        /// The participants of its own room.
        people: usize,
    },
    /// Shared on its own with someone who is not a member of the workspace (criterion 49).
    Refused,
}

/// Whether every current participant of the document's own keyring is a current participant
/// of the workspace's (ADR 0036 decision 10, **Proposal**).
#[must_use]
pub fn take_in_allowed(document: &Keyring, workspace: &[Identity]) -> bool {
    document
        .participants()
        .iter()
        .all(|p| workspace.contains(&p.identity))
}

/// What the file `bytes` is to a workspace whose current participants are `workspace`.
#[must_use]
pub fn take_in_check(bytes: &[u8], workspace: &[Identity]) -> TakeIn {
    let Ok(document) = unpack(1, bytes) else {
        return TakeIn::Plain;
    };
    let Some(record) = document.share() else {
        return TakeIn::Plain;
    };
    let keyring = decode_log(&record.keyring_log)
        .ok()
        .and_then(|entries| Keyring::replay(&entries).ok());
    match keyring {
        Some(keyring) if take_in_allowed(&keyring, workspace) => TakeIn::Allowed {
            people: keyring.participants().len(),
        },
        _ => TakeIn::Refused,
    }
}

impl Session {
    /// What the file `bytes` is to this session's shared workspace; `Refused` without one.
    #[must_use]
    pub fn workspace_take_in_check(&self, bytes: &[u8]) -> TakeIn {
        let Some(link) = self.sync.as_ref().filter(|l| l.workspace.is_some()) else {
            return TakeIn::Refused;
        };
        let members: Vec<Identity> = link
            .client
            .keyring()
            .map(|k| k.participants().iter().map(|p| p.identity).collect())
            .unwrap_or_default();
        take_in_check(bytes, &members)
    }

    /// Take-in: appends `Moved{workspace, document}` to this document's own room (a session of
    /// that room opened from the file); `false` without a live session.
    pub fn push_moved(&mut self, workspace: [u8; 16], document: [u8; 16]) -> bool {
        let Some(link) = self.sync.as_mut().filter(|l| l.ended.is_none()) else {
            return false;
        };
        let payload = encode_moved(&Moved {
            workspace,
            document,
        });
        match link.client.push(RecordKind::Update, payload) {
            Ok(client_id) => {
                link.moved_push = Some(client_id);
                self.sync_pump();
                true
            }
            Err(_) => false,
        }
    }

    /// Whether this device was removed from the shared workspace it carries (criteria 44, 45):
    /// the host drops the marker's share files and the workspace is a local one.
    #[must_use]
    pub fn workspace_removed(&self) -> bool {
        self.sync
            .as_ref()
            .is_some_and(|l| l.workspace.is_some() && l.ended == Some(SyncEnd::Removed))
    }

    /// Whether the session is connected and caught up.
    #[must_use]
    pub fn sync_online(&self) -> bool {
        self.sync.as_ref().is_some_and(|l| l.online)
    }

    /// Whether the relay stored the `Moved` record [`Session::push_moved`] appended.
    #[must_use]
    pub fn moved_stored(&self) -> bool {
        self.sync.as_ref().is_some_and(|l| l.moved_stored)
    }

    /// An update record of this document's own room that is a `Moved` record (criterion 49):
    /// the room is left for good, nobody is admitted from here any more, the file is a local one
    /// from its next save, and the maker is told. `true` when it was one.
    pub(super) fn moved_away(&mut self, bytes: &[u8]) -> bool {
        if decode_moved(bytes).is_none() {
            return false;
        }
        self.document.set_share(None);
        if let Some(link) = self.sync.as_mut() {
            link.actions.clear();
            link.actions.push_back(TransportAction::Close);
            link.ended = Some(SyncEnd::Moved);
            link.online = false;
            link.peers.clear();
            link.notice(moved_into_workspace_text("This document", ""), true);
            link.revision = link.revision.wrapping_add(1);
        }
        true
    }
}
