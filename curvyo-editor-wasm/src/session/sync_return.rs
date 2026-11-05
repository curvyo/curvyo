//! `Session`'s way back after a removal or a leave (`specs/0097` criteria 20, 21, 25, 26 and
//! 33; ADR 0041 decision 6): "Check whether I was added back", Sync again and Keep as it is.
//!
//! Nothing connects until the maker presses Check. The check is one connection that reads the
//! keyring and sends nothing else; Sync again turns the ended record and the checked keyring
//! back into a share record and resumes; Keep as it is sends one own leave entry.

use curvyo_crypto_core::keyring::{EntryHash, Keyring, encode_log};
use curvyo_crypto_core::{EpochKey, IdentityKeys};
use curvyo_document_core::{EndReason, EndedRecord, ShareRecord, WorkspaceRoomRecord};
use curvyo_sync_core::{CheckOutcome, ClientSession, Millis, RoomId, SessionConfig, SessionStart};

use super::Session;
use super::sync::{SyncEnd, SyncLink, SyncSeeds};

impl Session {
    /// Check whether I was added back, for a removed or left document: one connection to its
    /// room with `epoch_keys` (the credential store's); the answer arrives as the Share tab's
    /// `check`. `false` when the document keeps no ended record.
    pub fn check_added_back(
        &mut self,
        epoch_keys: Vec<(u32, [u8; 32])>,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let Some(record) = self.document.ended_share() else {
            return false;
        };
        self.start_return_check(record, false, keys(epoch_keys), false, seeds, now);
        true
    }

    /// The same for a removed or left workspace, from its marker's `share.ended.json`; `false`
    /// for bytes that are not an ended record.
    pub fn check_workspace_added_back(
        &mut self,
        ended_json: &[u8],
        epoch_keys: Vec<(u32, [u8; 32])>,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let Some(record) = EndedRecord::from_bytes(ended_json) else {
            return false;
        };
        self.start_return_check(record, true, keys(epoch_keys), false, seeds, now);
        true
    }

    /// Keep as it is after "added back" (Frage 162 A): one more connection that writes this
    /// device's own `RemoveMember` once; the record then says "left". `false` without a check
    /// that answered "added back".
    pub fn keep_as_it_is(&mut self, seeds: &SyncSeeds, now: Millis) -> bool {
        let Some((record, workspace, client)) = self.added_back() else {
            return false;
        };
        let epoch_keys = client.epoch_keys();
        let record = record.clone();
        self.start_return_check(record, workspace, epoch_keys, true, seeds, now);
        true
    }

    /// Sync again for a document after "added back" (criterion 21): the share record is rebuilt
    /// from the ended record and the checked keyring, and the document resumes as a reopened
    /// shared file, its edits made while away going out as offline edits. The new keys are
    /// handed to the host by [`Session::take_key_update`]. `false` without such a check.
    pub fn sync_again(&mut self, label: &str, seeds: &SyncSeeds, now: Millis) -> bool {
        let Some((record, false, client)) = self.added_back() else {
            return false;
        };
        let Ok(keyring_log) = encode_log(&client.keyring_entries()) else {
            return false;
        };
        let epoch_keys = key_bytes(client);
        self.document.set_share(Some(ShareRecord {
            server: record.server.clone(),
            room: record.room,
            server_seq: record.server_seq,
            synced_version: record.synced_version.clone(),
            keyring_log,
        }));
        let resumed = self.resume_shared(label, epoch_keys, seeds, now);
        if let Some(link) = self.sync.as_mut() {
            link.keys_changed = resumed;
        }
        resumed
    }

    /// Sync again for a workspace after "added back": the marker's `share.json` and
    /// `keyring.log` rebuilt from the ended record and the checked keyring, then resumed with the
    /// marker's `list` as [`Session::resume_workspace`] does. A workspace marked `left` in the
    /// devices list is cleared by the host first (ADR 0041 decision 3). `false` without such a
    /// check.
    pub fn workspace_sync_again(
        &mut self,
        list: &[u8],
        name: &str,
        label: &str,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let Some((record, true, client)) = self.added_back() else {
            return false;
        };
        let share_json = WorkspaceRoomRecord {
            server: record.server.clone(),
            workspace: record.room,
            server_seq: record.server_seq,
            synced_version: record.synced_version.clone(),
        }
        .to_bytes();
        let (Ok(share_json), Ok(keyring_log)) = (share_json, encode_log(&client.keyring_entries()))
        else {
            return false;
        };
        let epoch_keys = key_bytes(client);
        let resumed = self.resume_workspace(
            &share_json,
            &keyring_log,
            list,
            name,
            label,
            epoch_keys,
            seeds,
            now,
        );
        if let Some(link) = self.sync.as_mut() {
            link.keys_changed = resumed;
        }
        resumed
    }

    /// A return check answered: after Keep as it is, the ended record says "left" (a
    /// document's in its file, a workspace's for the marker).
    pub(super) fn check_answered(&mut self, outcome: CheckOutcome) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.check = Some(outcome);
        link.revision = link.revision.wrapping_add(1);
        if outcome != CheckOutcome::LeftAgain {
            return;
        }
        let heads = link.client.keyring().map(Keyring::heads);
        if let Some((record, workspace)) = link.returning.as_mut() {
            record.ended = EndReason::Left;
            // The heads now hold this leave: the earlier add back is no answer to a later check.
            if let Some(heads) = heads {
                record.heads = heads.into_iter().map(|h| h.0).collect();
            }
            if *workspace {
                // The marker's record, for the host to write.
                link.workspace_ended = Some(record.clone());
            } else {
                self.document.set_ended_share(Some(record.clone()));
            }
        }
    }

    /// The ended record of a return check that answered "added back", whether it is a
    /// workspace's, and its session.
    fn added_back(&self) -> Option<(&EndedRecord, bool, &ClientSession)> {
        let link = self.sync.as_ref()?;
        if link.check != Some(CheckOutcome::AddedBack) {
            return None;
        }
        let (record, workspace) = link.returning.as_ref()?;
        Some((record, *workspace, &link.client))
    }

    fn start_return_check(
        &mut self,
        record: EndedRecord,
        workspace: bool,
        epoch_keys: Vec<(u32, EpochKey)>,
        then_leave: bool,
        seeds: &SyncSeeds,
        now: Millis,
    ) {
        let config = SessionConfig {
            room: RoomId(record.room),
            identity: IdentityKeys::from_entropy(&seeds.identity),
            peer_id: if workspace { 0 } else { self.peer },
            nonce_seed: seeds.nonce,
            entropy_seed: seeds.entropy,
            since_seq: 0,
            start: SessionStart::ReturnCheck {
                epoch_keys,
                then_leave,
                heads: record.heads.iter().copied().map(EntryHash).collect(),
            },
        };
        let Ok(client) = ClientSession::new(config, now) else {
            return;
        };
        let mut link = SyncLink::new(client, record.server.clone(), String::new(), now);
        // The share stays ended while the check runs: nothing is shared or saved as shared.
        link.ended = Some(match record.ended {
            EndReason::Removed => SyncEnd::Removed,
            EndReason::Left => SyncEnd::Left,
        });
        link.returning = Some((record, workspace));
        link.keys_changed = false;
        self.install_link(Box::new(link));
        self.sync_pump();
    }
}

fn keys(epoch_keys: Vec<(u32, [u8; 32])>) -> Vec<(u32, EpochKey)> {
    epoch_keys
        .into_iter()
        .map(|(n, k)| (n, EpochKey::from_bytes(k)))
        .collect()
}

fn key_bytes(client: &ClientSession) -> Vec<(u32, [u8; 32])> {
    client
        .epoch_keys()
        .into_iter()
        .map(|(n, k)| (n, *k.as_bytes()))
        .collect()
}
