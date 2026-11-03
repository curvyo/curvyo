//! The client's handling of each relay frame.

use curvyo_crypto_core::keyring::Entry;
use curvyo_crypto_core::{EnvelopeHeader, EnvelopeKind};

use super::{
    ClientOutput, ClientSession, SNAPSHOT_BYTES, SNAPSHOT_RECORDS, SessionEvent, Start, State,
};
use crate::frame::{ClientFrame, ConnectionId, RecordKind, RelayFrame};
use crate::relay::relay_proof_message;
use crate::time::Millis;

impl ClientSession {
    /// One message from the relay. A message that is not a frame is ignored.
    pub fn received(&mut self, bytes: &[u8], now: Millis) {
        // Before `connected` no socket of this session is open: what arrives is left over from
        // a closed one.
        if matches!(
            self.state,
            State::Ended | State::Idle { .. } | State::Connecting
        ) {
            return;
        }
        self.link.heard(now);
        let Ok(frame) = RelayFrame::decode(bytes) else {
            return;
        };
        match frame {
            RelayFrame::Challenge { nonce } => self.on_challenge(&nonce),
            RelayFrame::Welcome {
                head_seq,
                snapshot_seq,
                ..
            } => self.on_welcome(head_seq, snapshot_seq, now),
            RelayFrame::Keyring { entry } => self.on_keyring(&entry, now),
            RelayFrame::Record {
                seq,
                kind,
                envelope,
            } => {
                self.head_seq = self.head_seq.max(seq);
                self.pending.push_back((seq, kind, envelope));
                self.deliver_pending();
            }
            RelayFrame::Ack { client_id, seq } => self.on_ack(client_id, seq),
            RelayFrame::Peer {
                connection,
                identity,
            } => self.on_peer(connection, identity),
            RelayFrame::Awareness {
                connection,
                envelope,
            } => self.on_awareness(connection, &envelope, now),
            RelayFrame::Refused { reason } => {
                self.event(SessionEvent::Refused(reason));
                if reason == crate::frame::RefusalReason::Removed {
                    self.event(SessionEvent::Removed);
                }
                self.end();
            }
            RelayFrame::Pong => {
                if self.state == (State::Joined { caught_up: false }) {
                    self.state = State::Joined { caught_up: true };
                    self.on_caught_up(now);
                }
            }
        }
    }

    fn on_welcome(&mut self, head_seq: u64, snapshot_seq: u64, now: Millis) {
        self.state = State::Joined { caught_up: false };
        self.head_seq = self.head_seq.max(head_seq);
        if let Start::Join { .. } = self.start {
            self.start = Start::Member;
        }
        self.link.reset_backoff();
        self.since_snapshot = (head_seq.saturating_sub(snapshot_seq), 0);
        // The relay answers in order: its pong follows the keyring and every record of the
        // admission, so it marks the end of the catch-up (Proposal).
        self.send(&ClientFrame::Ping);
        self.link.pinged(now);
    }

    fn on_ack(&mut self, client_id: u64, seq: u64) {
        if let Some(item) = self.outbox.remove(&client_id) {
            self.count_record(item.plaintext.len());
        }
        if self.pending.is_empty() {
            self.delivered = self.delivered.max(seq);
        }
        self.event(SessionEvent::Acked { client_id, seq });
        self.check_snapshot();
    }

    fn on_awareness(&mut self, connection: ConnectionId, envelope: &[u8], now: Millis) {
        if let Ok(opened) = self.keys.open(envelope, EnvelopeKind::Presence) {
            self.presence.heard(connection, now);
            self.event(SessionEvent::Presence {
                connection,
                member: self.peers.get(&connection).copied(),
                payload: opened.plaintext,
            });
        }
    }

    fn on_challenge(&mut self, nonce: &[u8; 32]) {
        let signature = self
            .keys
            .identity
            .sign(&relay_proof_message(self.room, nonce));
        self.send(&ClientFrame::Proof { signature });
        let first = match &self.start {
            Start::Share { genesis, uploaded } if !*uploaded => genesis.encode().ok(),
            Start::Join { key, claimed } if !*claimed => self
                .keys
                .claim(key, true)
                .ok()
                .and_then(|e| e.encode().ok()),
            _ => None,
        };
        if let Some(entry) = first {
            self.send(&ClientFrame::Keyring { entry });
        } else if matches!(self.start, Start::Member) {
            // A member is admitted on its proof and the relay answers this with a pong after the
            // admission; a device the room's keyring does not list is refused "not a member"
            // instead of waiting for a genesis or a claim that never comes (criterion 27).
            self.send(&ClientFrame::Ping);
        }
    }

    fn on_keyring(&mut self, bytes: &[u8], now: Millis) {
        let Ok(entry) = Entry::decode(bytes) else {
            return;
        };
        let had_key = self.keys.holds_any_key();
        if !self.keys.add_entries(vec![entry]) {
            return;
        }
        self.event(SessionEvent::KeyringChanged);
        let me = self.keys.me();
        if self.keys.keyring().is_some_and(|k| k.was_removed(&me)) {
            self.event(SessionEvent::Removed);
            self.end();
            return;
        }
        if self.claim_once_invite_is_known() {
            return;
        }
        if !had_key && self.keys.holds_any_key() {
            self.note_admitted();
            if self.state == (State::Joined { caught_up: true }) {
                self.event(SessionEvent::Online);
            }
        }
        self.deliver_pending();
        if self.state == (State::Joined { caught_up: true }) {
            self.after_keyring_change(now);
        }
    }

    /// A joiner that received the keyring after its probe claims the invite on its heads.
    fn claim_once_invite_is_known(&mut self) -> bool {
        let Start::Join { key, claimed } = &mut self.start else {
            return false;
        };
        if *claimed {
            return false;
        }
        let Some(expires) = self
            .keys
            .keyring()
            .and_then(|k| k.invite(&key.invite_id()))
            .map(|i| i.expires)
        else {
            return false;
        };
        *claimed = true;
        self.invite_expires = Some(expires);
        if let Ok(claim) = self.keys.claim(key, false)
            && let Ok(entry) = claim.encode()
        {
            self.send(&ClientFrame::Keyring { entry });
        }
        true
    }

    /// Writes the rewraps this device owes and sends what waited for a key.
    fn after_keyring_change(&mut self, now: Millis) {
        if let Ok(entries) = self
            .keys
            .write_rewraps(now.unix_seconds(), &mut self.entropy)
        {
            for entry in entries {
                if let Ok(bytes) = entry.encode() {
                    self.send(&ClientFrame::Keyring { entry: bytes });
                }
            }
        }
        let _ = self.flush();
        self.send_pending_snapshot();
    }

    fn on_caught_up(&mut self, now: Millis) {
        if self.keys.holds_any_key() {
            self.note_admitted();
            self.event(SessionEvent::Online);
        } else {
            self.event(SessionEvent::WaitingForMember);
        }
        self.presence.resend();
        self.after_keyring_change(now);
        if let Start::Share { uploaded, .. } = &mut self.start {
            // Criterion 1 (b): sharing uploads the sealed state as the first snapshot.
            *uploaded = true;
            self.start = Start::Member;
            self.snapshot_asked = true;
            let covers_seq = self.cursor();
            self.event(SessionEvent::SnapshotDue { covers_seq });
            return;
        }
        self.check_snapshot();
    }

    fn on_peer(
        &mut self,
        connection: ConnectionId,
        identity: Option<curvyo_crypto_core::MemberId>,
    ) {
        if let Some(member) = identity {
            self.peers.insert(connection, member);
            self.event(SessionEvent::Peer { connection, member });
        } else {
            self.peers.remove(&connection);
            self.presence.forget(connection);
            self.event(SessionEvent::PresenceGone { connection });
        }
    }

    /// Opens and hands out the waiting records in order, as far as this device holds their
    /// keys. A record under a key the keyring does not name at all is rejected, never waited
    /// for, so a forged header cannot stall the queue.
    fn deliver_pending(&mut self) {
        while let Some((seq, kind, envelope)) = self.pending.front() {
            let (seq, kind) = (*seq, *kind);
            let named = EnvelopeHeader::parse(envelope).is_ok_and(|h| {
                self.keys.keyring().is_some_and(|k| {
                    k.epochs()
                        .iter()
                        .any(|e| e.key_id == h.key_id && e.number == h.epoch)
                })
            });
            if named && !self.keys.can_open(envelope) {
                return;
            }
            let Some((_, _, envelope)) = self.pending.pop_front() else {
                return;
            };
            self.delivered = self.delivered.max(seq);
            if kind == RecordKind::Snapshot {
                self.since_snapshot = (0, 0);
            } else {
                self.count_record(envelope.len());
            }
            let envelope_kind = match kind {
                RecordKind::Update => EnvelopeKind::Update,
                RecordKind::Asset => EnvelopeKind::Asset,
                RecordKind::Snapshot => EnvelopeKind::Snapshot,
            };
            // A resync clears the queue; without one the next record is delivered.
            match self.keys.open(&envelope, envelope_kind) {
                Ok(opened) => self.event(SessionEvent::Remote {
                    seq,
                    kind,
                    bytes: opened.plaintext,
                }),
                Err(_) => self.record_rejected(seq),
            }
        }
    }

    /// A joiner received its first key.
    fn note_admitted(&mut self) {
        if self.awaiting_admission {
            self.awaiting_admission = false;
            self.event(SessionEvent::Admitted);
        }
    }

    /// Criterion 25: the record `seq` was refused, here because it did not open or by the
    /// caller because its plaintext did not import. It is never merged, the rejection is shown
    /// once, and the session resyncs from the room's snapshot: it reconnects with cursor 0, so
    /// the snapshot and every record after it arrive again (ADR 0031 decision 3). A record up to
    /// the room's head at the last resync starts none, so a record that stays damaged is
    /// skipped instead of looping.
    pub fn record_rejected(&mut self, seq: u64) {
        if !self.rejected_shown {
            self.rejected_shown = true;
            self.event(SessionEvent::UpdateRejected);
        }
        if self.state == State::Ended || self.resynced_through.is_some_and(|t| seq <= t) {
            return;
        }
        self.resynced_through = Some(seq.max(self.head_seq));
        self.delivered = 0;
        self.pending.clear();
        self.event(SessionEvent::Resyncing);
        if matches!(self.state, State::Handshake | State::Joined { .. }) {
            self.peers.clear();
            for connection in self.presence.forget_all() {
                self.event(SessionEvent::PresenceGone { connection });
            }
            for item in self.outbox.values_mut() {
                item.sent = false;
            }
            self.state = State::Connecting;
            self.out.push(ClientOutput::Close);
            self.out.push(ClientOutput::Connect);
        }
    }

    fn count_record(&mut self, len: usize) {
        self.since_snapshot.0 += 1;
        self.since_snapshot.1 += u64::try_from(len).unwrap_or(u64::MAX);
    }

    /// Asks for a snapshot after 500 records or 4 MiB since the last one (decision 10).
    fn check_snapshot(&mut self) {
        let (records, bytes) = self.since_snapshot;
        if self.is_online()
            && !self.snapshot_asked
            && (records > SNAPSHOT_RECORDS || bytes > SNAPSHOT_BYTES)
        {
            self.snapshot_asked = true;
            let covers_seq = self.cursor();
            self.event(SessionEvent::SnapshotDue { covers_seq });
        }
    }

    fn send_pending_snapshot(&mut self) {
        if !self.is_online() {
            return;
        }
        if let Some((covers_seq, envelope)) = self.pending_snapshot.take() {
            self.out.push(ClientOutput::Send(
                ClientFrame::Snapshot {
                    covers_seq,
                    envelope,
                }
                .encode(),
            ));
        }
    }
}
