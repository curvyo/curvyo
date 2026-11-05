//! What the group room's session reports, applied to the device group (ADR 0040 decisions 5
//! and 8): transport actions out, admission, keyring changes, removal and refusals in.

use curvyo_crypto_core::MemberId;
use curvyo_document_core::DevicesListError;
use curvyo_sync_core::{ClientOutput, RecordKind, RefusalReason, SessionEvent};
use curvyo_ui_core::devices_text as texts;

use super::{DeviceGroupGlue, JOIN_RETRY_MS};
use crate::session::TransportAction;

impl DeviceGroupGlue {
    /// Carries out the session's outputs.
    pub(super) fn pump(&mut self) {
        loop {
            let out = self.client.drain();
            if out.is_empty() {
                return;
            }
            for output in out {
                match output {
                    ClientOutput::Connect => self
                        .actions
                        .push_back(TransportAction::Connect(self.server.clone())),
                    ClientOutput::Send(bytes) => {
                        self.actions.push_back(TransportAction::Send(bytes));
                    }
                    ClientOutput::Close => self.actions.push_back(TransportAction::Close),
                    ClientOutput::Event(event) => self.on_event(event),
                }
            }
        }
    }

    pub(super) fn on_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Online => {
                self.online = true;
                self.retries = None;
                self.bump();
                self.write_pending();
                self.write_own_label();
            }
            SessionEvent::Offline => {
                self.online = false;
                self.bump();
            }
            SessionEvent::Remote { kind, bytes, .. } => {
                if kind == RecordKind::Asset {
                    return;
                }
                match self.list.import_updates(&bytes) {
                    Ok(()) => {
                        self.files_changed = true;
                        self.bump();
                        self.write_own_label();
                    }
                    // ADR 0040 decision 11: the list stays as it is; reported once.
                    Err(DevicesListError::FormatTooNew) => {
                        if !self.newer_list {
                            self.newer_list = true;
                            self.notices.push_back(texts::newer_devices_list_notice());
                            self.bump();
                        }
                    }
                    Err(_) => {}
                }
            }
            SessionEvent::Acked { client_id, .. } => {
                if let Some(version) = self.pushed.remove(&client_id) {
                    self.synced = version;
                    self.files_changed = true;
                }
            }
            SessionEvent::KeyringChanged | SessionEvent::Admitted => self.keyring_changed(),
            // A replica that refused a newer list does not hold every record it would cover.
            SessionEvent::SnapshotDue { covers_seq } if !self.newer_list => {
                if let Ok(bytes) = self.list.snapshot() {
                    let _ = self.client.upload_snapshot(&bytes, covers_seq);
                }
            }
            SessionEvent::Removed => {
                self.removed = true;
                self.online = false;
                self.bump();
            }
            SessionEvent::Refused(reason) => self.refused(reason),
            SessionEvent::Peer { connection, member } => {
                self.peers.insert(connection, member);
                self.bump();
            }
            SessionEvent::PresenceGone { connection } => {
                if let Some(member) = self.peers.remove(&connection) {
                    self.last_seen.insert(member, self.now.unix_seconds());
                    self.bump();
                }
            }
            SessionEvent::WaitingForMember
            | SessionEvent::InviteExpired
            | SessionEvent::Presence { .. }
            | SessionEvent::UpdateRejected
            | SessionEvent::SnapshotDue { .. }
            | SessionEvent::Resyncing
            // The group room never runs a return check.
            | SessionEvent::CheckResult(_) => {}
        }
    }

    fn refused(&mut self, reason: RefusalReason) {
        if reason == RefusalReason::Removed {
            self.removed = true;
        } else if let Some((left, _)) = self.retries
            && left > 0
        {
            self.retries = Some((left, Some(self.now.plus(JOIN_RETRY_MS))));
        }
        self.online = false;
        self.bump();
    }

    fn keyring_changed(&mut self) {
        let before = std::mem::take(&mut self.devices_seen);
        self.files_changed = true;
        self.keys_changed = true;
        self.bump();
        let after = self.device_ids();
        let left: Vec<MemberId> = self.client.keyring().map_or_else(Vec::new, |keyring| {
            keyring
                .accepted_entries()
                .iter()
                .filter(|e| is_removal_of(e, &e.author))
                .map(|e| e.author)
                .collect()
        });
        for member in after.iter().filter(|m| !before.contains(m)) {
            if *member != self.client.me() && !before.is_empty() {
                let label = self.label_of(member);
                self.notices.push_back(texts::device_added_notice(&label));
            }
        }
        for member in before.iter().filter(|m| !after.contains(m)) {
            let label = self.label_of(member);
            self.notices.push_back(if left.contains(member) {
                texts::device_left_notice(&label)
            } else {
                texts::device_removed_notice(&label)
            });
        }
        self.devices_seen = after;
    }
}

fn is_removal_of(entry: &curvyo_crypto_core::keyring::Entry, member: &MemberId) -> bool {
    matches!(
        entry.payload,
        curvyo_crypto_core::keyring::Payload::RemoveMember { member: m } if m == *member
    )
}
