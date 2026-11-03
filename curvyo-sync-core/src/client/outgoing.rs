//! What a client sends on its own account: pushes through the outbox, snapshots, presence,
//! invites and removals.

use curvyo_crypto_core::keyring::{Entry, Payload, Role};
use curvyo_crypto_core::{EnvelopeKind, InviteKey, MemberId};

use super::{ClientSession, InviteSecret, OutItem, SessionError, SessionEvent, State};
use crate::frame::{ClientFrame, MAX_MESSAGE_LEN, RecordKind};
use crate::time::Millis;

impl ClientSession {
    /// Queues an update (`Document::export_updates`) and returns its number; it is sealed under
    /// the then current epoch when it is sent (ADR 0031 decision 6a).
    ///
    /// # Errors
    /// `TooLarge` over the message limit; `SnapshotPushed`; `Ended`.
    pub fn push(&mut self, kind: RecordKind, plaintext: Vec<u8>) -> Result<u64, SessionError> {
        if self.state == State::Ended {
            return Err(SessionError::Ended);
        }
        if kind == RecordKind::Snapshot {
            return Err(SessionError::SnapshotPushed);
        }
        // Envelope and frame overhead stay far below this margin.
        if plaintext.len() + 1024 > MAX_MESSAGE_LEN {
            return Err(SessionError::TooLarge);
        }
        let client_id = self.next_client_id;
        self.next_client_id += 1;
        self.outbox.insert(
            client_id,
            OutItem {
                kind,
                plaintext,
                sent: false,
            },
        );
        self.flush()?;
        Ok(client_id)
    }

    /// Seals the replica's state (`container::pack`) as a signed snapshot covering every
    /// record up to `covers_seq`, and uploads it now or on the next connection.
    ///
    /// # Errors
    /// `NoKey` without the current key; `TooLarge`; `Ended`.
    pub fn upload_snapshot(
        &mut self,
        plaintext: &[u8],
        covers_seq: u64,
    ) -> Result<(), SessionError> {
        if self.state == State::Ended {
            return Err(SessionError::Ended);
        }
        if plaintext.len() + 1024 > MAX_MESSAGE_LEN {
            return Err(SessionError::TooLarge);
        }
        let envelope = self
            .keys
            .seal(EnvelopeKind::Snapshot, plaintext)?
            .ok_or(SessionError::NoKey)?;
        self.since_snapshot = (0, 0);
        self.snapshot_asked = false;
        if self.is_online() {
            self.send(&ClientFrame::Snapshot {
                covers_seq,
                envelope,
            });
        } else {
            self.pending_snapshot = Some((covers_seq, envelope));
        }
        Ok(())
    }

    /// This device's presence (the `ui-core` payload); sent at most every 50 ms while it
    /// changes and every 2 s otherwise.
    pub fn set_presence(&mut self, payload: Vec<u8>, now: Millis) {
        self.presence.set(payload);
        if self.is_online() {
            self.send_presence(now);
        }
    }

    /// Writes a one-time invite valid until `expires` (seconds since the Unix epoch) and
    /// returns the link's secret (criterion 2). Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto`.
    pub fn invite(&mut self, expires: u64) -> Result<InviteSecret, SessionError> {
        self.require_admin()?;
        let seed = self.entropy.secret();
        let key = InviteKey::from_seed(&seed)?;
        let entry = self.keys.write(Payload::AddPendingInvite {
            invite_id: key.invite_id(),
            role: Role::Member,
            expires,
        })?;
        self.send_entries(&[entry])?;
        self.event(SessionEvent::KeyringChanged);
        Ok(InviteSecret {
            room: self.room,
            seed,
        })
    }

    /// Removes `member` with a key rotation, then asks for a snapshot under the new epoch
    /// (criterion 32, ADR 0031 decision 6a). Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto`.
    pub fn remove(&mut self, member: MemberId) -> Result<(), SessionError> {
        self.require_admin()?;
        let entries = self.keys.write_removal(member, &mut self.entropy)?;
        self.send_entries(&entries)?;
        self.event(SessionEvent::KeyringChanged);
        self.snapshot_asked = true;
        let covers_seq = self.cursor();
        self.event(SessionEvent::SnapshotDue { covers_seq });
        Ok(())
    }

    fn require_admin(&self) -> Result<(), SessionError> {
        if self.state == State::Ended {
            return Err(SessionError::Ended);
        }
        // Keyring entries go out at once, so these actions need the connection (Proposal).
        if !self.is_online() {
            return Err(SessionError::Offline);
        }
        let keyring = self.keys.keyring().ok_or(SessionError::NotAMember)?;
        match keyring.role(&self.keys.me()) {
            Some(Role::Admin) => Ok(()),
            Some(Role::Member) => Err(SessionError::NotAdmin),
            None => Err(SessionError::NotAMember),
        }
    }

    fn send_entries(&mut self, entries: &[Entry]) -> Result<(), SessionError> {
        for entry in entries {
            let bytes = entry.encode()?;
            self.send(&ClientFrame::Keyring { entry: bytes });
        }
        Ok(())
    }

    /// Sends every queued push that has not gone out on this connection, sealed now.
    pub(super) fn flush(&mut self) -> Result<(), SessionError> {
        if !self.is_online() {
            return Ok(());
        }
        let ids: Vec<u64> = self
            .outbox
            .iter()
            .filter(|(_, item)| !item.sent)
            .map(|(id, _)| *id)
            .collect();
        for client_id in ids {
            let Some(item) = self.outbox.get(&client_id) else {
                continue;
            };
            let envelope_kind = match item.kind {
                RecordKind::Asset => EnvelopeKind::Asset,
                RecordKind::Update | RecordKind::Snapshot => EnvelopeKind::Update,
            };
            let kind = item.kind;
            let plaintext = item.plaintext.clone();
            let Some(envelope) = self.keys.seal(envelope_kind, &plaintext)? else {
                return Ok(());
            };
            self.send(&ClientFrame::Push {
                client_id,
                kind,
                envelope,
            });
            if let Some(item) = self.outbox.get_mut(&client_id) {
                item.sent = true;
            }
        }
        Ok(())
    }

    pub(super) fn send_presence(&mut self, now: Millis) {
        let Some(payload) = self.presence.due(now) else {
            return;
        };
        if let Ok(Some(envelope)) = self.keys.seal(EnvelopeKind::Presence, &payload) {
            self.send(&ClientFrame::Awareness { envelope });
        }
    }
}
