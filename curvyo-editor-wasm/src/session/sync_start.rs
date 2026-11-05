//! `Session`'s ways into a room (criteria 1, 3, 5): sharing the open document, joining by an
//! invite link, and resuming a shared document opened from its file.

use curvyo_crypto_core::keyring::decode_log;
use curvyo_crypto_core::{EpochKey, IdentityKeys};
use curvyo_sync_core::{
    ClientSession, InviteLink, Millis, RoomId, SessionConfig, SessionError, SessionStart,
};
use curvyo_ui_core::share_text::JoinRefusal;

use super::Session;
use super::sync::{JoinPhase, SyncLink, SyncSeeds};

fn identity_keys(seeds: &SyncSeeds) -> IdentityKeys {
    IdentityKeys::from_entropy(&seeds.identity)
}

impl Session {
    /// Shares the open document on `server` (criterion 1): the session writes the genesis,
    /// connects, and uploads the sealed document once the relay has admitted it. `room` is 16
    /// fresh random bytes from the host, the room and document id.
    ///
    /// # Errors
    /// `Crypto` if the genesis cannot be made.
    pub fn share_document(
        &mut self,
        server: &str,
        label: &str,
        room: [u8; 16],
        seeds: &SyncSeeds,
        now: Millis,
    ) -> Result<(), SessionError> {
        let config = self.sync_config(RoomId(room), 0, SessionStart::Share, seeds);
        let client = ClientSession::new(config, now)?;
        let mut link = SyncLink::new(client, server.to_owned(), label.to_owned(), now);
        link.exported = self.document.version();
        link.assets = self.referenced_assets();
        self.install_link(Box::new(link));
        self.sync_pump();
        Ok(())
    }

    /// Joins the room of an invite link (criterion 3): this session's document is a blank
    /// placeholder until the room's snapshot replaces it.
    ///
    /// # Errors
    /// `Invalid` for a text that is not an invite link.
    pub fn join_document(
        &mut self,
        link: &str,
        label: &str,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> Result<(), JoinRefusal> {
        let link = InviteLink::parse(link).map_err(|_| JoinRefusal::Invalid)?;
        let server =
            curvyo_sync_core::normalize_server_address(&link.server).ok_or(JoinRefusal::Invalid)?;
        let config = self.sync_config(link.secret.room, 0, SessionStart::Join(link.secret), seeds);
        let client = ClientSession::new(config, now).map_err(|_| JoinRefusal::Invalid)?;
        let mut sync = SyncLink::new(client, server, label.to_owned(), now);
        sync.awaiting_document = true;
        sync.join = Some(JoinPhase::Connecting);
        self.install_link(Box::new(sync));
        self.sync_pump();
        Ok(())
    }

    /// Reconnects a shared document opened from its file (criterion 5, 19): no invite, the
    /// keyring from the file and the epoch keys from the credential store. `false` when the
    /// document is not shared or its keyring cannot be read.
    pub fn resume_shared(
        &mut self,
        label: &str,
        epoch_keys: Vec<(u32, [u8; 32])>,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let Some(record) = self.document.share() else {
            return false;
        };
        let Ok(keyring) = decode_log(&record.keyring_log) else {
            return false;
        };
        let start = SessionStart::Member {
            keyring,
            epoch_keys: epoch_keys
                .into_iter()
                .map(|(n, k)| (n, EpochKey::from_bytes(k)))
                .collect(),
        };
        let config = self.sync_config(RoomId(record.room), record.server_seq, start, seeds);
        let Ok(client) = ClientSession::new(config, now) else {
            return false;
        };
        let mut link = SyncLink::new(client, record.server.clone(), label.to_owned(), now);
        // Pictures added since the last acknowledged version (offline, before this reopen) go
        // out on connect (ADR 0031 decision 9). The file does not say which of them the room
        // already holds, so after unsynced edits every referenced picture is sent again; the
        // room's copies are idempotent (Proposal).
        if self.document.version() == record.synced_version {
            link.assets = self.referenced_assets();
            link.asset_total = self.document.asset_bytes();
        }
        link.exported = record.synced_version.clone();
        link.synced = record.synced_version;
        // Keys came from the store: nothing new to write until the keyring changes.
        link.keys_changed = false;
        self.install_link(Box::new(link));
        self.sync_export();
        self.sync_pump();
        true
    }

    fn sync_config(
        &self,
        room: RoomId,
        since_seq: u64,
        start: SessionStart,
        seeds: &SyncSeeds,
    ) -> SessionConfig {
        SessionConfig {
            room,
            identity: identity_keys(seeds),
            peer_id: self.peer,
            nonce_seed: seeds.nonce,
            entropy_seed: seeds.entropy,
            since_seq,
            start,
        }
    }
}
