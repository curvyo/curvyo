//! What the group room's session reports, applied to the device group (ADR 0040 decisions 5
//! and 8; ADR 0062 decision 1): transport actions out, admission, keyring changes, removal and
//! refusals in, the room's records routed to its two streams.

use curvyo_crypto_core::MemberId;
use curvyo_document_core::DevicesListError;
use curvyo_sync_core::{
    ClientOutput, GroupRecord, RecordKind, RefusalReason, SessionEvent, StreamError,
    group_snapshot_assembler, route_group_record,
};
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
                self.export_library();
                self.flush_font_blobs();
            }
            SessionEvent::Offline => {
                self.online = false;
                self.bump();
            }
            SessionEvent::Remote {
                seq, kind, bytes, ..
            } => {
                if kind == RecordKind::Asset {
                    self.receive_font_part(&bytes);
                    return;
                }
                if kind == RecordKind::Snapshot {
                    self.snapshot_part(seq, &bytes);
                } else {
                    self.update_record(&bytes);
                }
            }
            SessionEvent::Acked { client_id, .. } => {
                self.fonts.ack(client_id);
                if let Some(version) = self.library_pushed.remove(&client_id) {
                    self.library_synced = version;
                    self.files_changed = true;

                    if self.snapshot_after_ack && self.library_pushed.is_empty() {
                        self.snapshot_after_ack = false;
                        if !self.newer_list && !self.newer_library {
                            let covers_seq = self.client.cursor();
                            self.upload_group_snapshot(covers_seq);
                        }
                    }
                }
                if let Some(version) = self.pushed.remove(&client_id) {
                    self.synced = version;
                    self.files_changed = true;
                }
            }
            SessionEvent::KeyringChanged | SessionEvent::Admitted => self.keyring_changed(),
            // A replica that refused a newer list or library does not hold every record it
            // would cover.
            SessionEvent::SnapshotDue { covers_seq } if !self.newer_list && !self.newer_library => {
                self.upload_group_snapshot(covers_seq);
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
            | SessionEvent::Resyncing => {}
        }
    }

    /// One update record of the room, routed to its stream (ADR 0062 decision 1): a bare update
    /// is the devices list's as every pre-0116 build writes it; a record with the inner header
    /// is the library's.
    fn update_record(&mut self, bytes: &[u8]) {
        match route_group_record(bytes) {
            Ok(GroupRecord::Devices(bare)) => self.import_devices(bare),
            Ok(GroupRecord::Library { payload, .. }) => self.import_library(payload),
            Err(_) => {}
        }
    }

    /// One snapshot record of the room: a part of the framed snapshot of both streams (ADR
    /// 0062 decision 1), or a pre-0116 build's plain snapshot of the devices list.
    fn snapshot_part(&mut self, seq: u64, bytes: &[u8]) {
        if self.skipped_snapshot == Some(seq) {
            return;
        }
        let starts_new = self.assembling.as_ref().is_none_or(|(at, _)| *at != seq);
        if starts_new {
            self.assembling = Some((seq, group_snapshot_assembler()));
        }
        let outcome = match self.assembling.as_mut() {
            Some((_, assembler)) => assembler
                .push_part(bytes)
                .map(|entries| (entries, assembler.is_complete(), assembler.pending_len())),
            None => return,
        };
        match outcome {
            Ok((entries, complete, pending)) => {
                for entry in &entries {
                    self.snapshot_stream(entry);
                }
                if complete {
                    self.assembling = None;
                } else if !self.make_room(pending as u64) {
                    // What is still arriving cannot fit the library budget: give it up.
                    self.skip_snapshot(seq);
                }
            }
            Err(StreamError::TooLarge) => self.skip_snapshot(seq),
            Err(_) => {
                self.assembling = None;
                self.import_devices(bytes);
            }
        }
    }

    /// Gives up the snapshot `seq` (an entry over its limit or the budget): the rest of its parts
    /// is ignored, with a notice.
    fn skip_snapshot(&mut self, seq: u64) {
        self.assembling = None;
        self.skipped_snapshot = Some(seq);
        self.notices
            .push_back(curvyo_ui_core::library_text::FONT_SKIPPED_NOTICE.to_owned());
        self.bump();
    }

    /// The bytes of the room snapshot being assembled that no entry consumed yet.
    #[cfg(test)]
    pub(super) fn snapshot_assembly_bytes(&self) -> usize {
        self.assembling
            .as_ref()
            .map_or(0, |(_, assembler)| assembler.pending_len())
    }

    /// A bare update or plain snapshot of the devices list, as every pre-0116 build writes it.
    pub(super) fn import_devices(&mut self, bytes: &[u8]) {
        match self.list.import_updates(bytes) {
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
