//! What a client sends on its own account: pushes through the outbox, snapshots, presence,
//! invites and removals.

use curvyo_crypto_core::keyring::{Entry, Keyring, Payload, Role};
use curvyo_crypto_core::{EnvelopeKind, Identity, InviteKey, MemberId};

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

    /// Seals the plaintexts of a workspace snapshot's parts (`split_snapshot`) as signed
    /// snapshot envelopes covering every record up to `covers_seq`, and uploads them as
    /// `SnapshotPart` frames now or on the next connection (ADR 0037 decision 7).
    ///
    /// # Errors
    /// `NoKey` without the current key; `TooLarge` for a part over the message limit; `Ended`.
    pub fn upload_snapshot_parts(
        &mut self,
        parts: &[Vec<u8>],
        covers_seq: u64,
    ) -> Result<(), SessionError> {
        if self.state == State::Ended {
            return Err(SessionError::Ended);
        }
        if parts.iter().any(|part| part.len() + 1024 > MAX_MESSAGE_LEN) {
            return Err(SessionError::TooLarge);
        }
        let mut envelopes = Vec::with_capacity(parts.len());
        for part in parts {
            let envelope = self
                .keys
                .seal(EnvelopeKind::Snapshot, part)?
                .ok_or(SessionError::NoKey)?;
            envelopes.push(envelope);
        }
        self.since_snapshot = (0, 0);
        self.snapshot_asked = false;
        self.workspace_snapshot = Some(parts.iter().map(|p| p.len() as u64).sum());
        if self.is_online() {
            self.send_parts(covers_seq, envelopes);
        } else {
            self.pending_parts = Some((covers_seq, envelopes));
        }
        Ok(())
    }

    pub(super) fn send_parts(&mut self, covers_seq: u64, envelopes: Vec<Vec<u8>>) {
        let count = envelopes.len();
        for (index, envelope) in (0_u32..).zip(envelopes) {
            self.send(&ClientFrame::SnapshotPart {
                covers_seq,
                index,
                last: index as usize + 1 == count,
                envelope,
            });
        }
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

    /// Adds `identity` as a member with `role` and every epoch key this device holds (ADR 0040
    /// decisions 1 and 5: a paired device, full history). Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember` (this device), `AlreadyMember`, `Offline`, `Ended`, `Crypto`.
    pub fn add_member(&mut self, identity: Identity, role: Role) -> Result<(), SessionError> {
        self.add(identity, role, false)
    }

    /// Adds a device of the maker's `group` like [`add_member`](Self::add_member), also one a
    /// device that ever was in the group removed before (a switch-off, ADR 0040 decision 6),
    /// never one an admin who never was in the group removed ([`may_readmit`](crate::may_readmit), decision 5). The
    /// caller makes sure it is a current group member ([`reconcile`](crate::reconcile)). Admin
    /// only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember` (this device), `AlreadyMember` (also for a device withheld by
    /// that rule), `Offline`, `Ended`, `Crypto`.
    pub fn admit_device(
        &mut self,
        group: &Keyring,
        identity: Identity,
        role: Role,
    ) -> Result<(), SessionError> {
        let readmit = self
            .keys
            .keyring()
            .is_some_and(|keyring| crate::may_readmit(group, keyring, &identity.member));
        self.add(identity, role, readmit)
    }

    fn add(&mut self, identity: Identity, role: Role, readmit: bool) -> Result<(), SessionError> {
        self.require_admin()?;
        let entry = self
            .keys
            .write_addition(identity, role, readmit, &mut self.entropy)?;
        self.send_entries(&[entry])?;
        self.event(SessionEvent::KeyringChanged);
        Ok(())
    }

    /// Removes `member` with a key rotation, then asks for a snapshot under the new epoch
    /// (criterion 32, ADR 0031 decision 6a). Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto`.
    pub fn remove(&mut self, member: MemberId) -> Result<(), SessionError> {
        self.remove_members(&[member])
    }

    /// Removes every one of `members`, then rotates once and asks for a snapshot under the new
    /// epoch (ADR 0040 decision 5). Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember` (also for an empty list), `Offline`, `Ended`, `Crypto`.
    pub fn remove_members(&mut self, members: &[MemberId]) -> Result<(), SessionError> {
        self.require_admin()?;
        let entries = self.keys.write_removal(members, &mut self.entropy)?;
        self.send_entries(&entries)?;
        self.rotated();
        Ok(())
    }

    /// Writes a `NewEpoch` for the current members (a former member still holds the current
    /// key, ADR 0040 decision 5) and asks for a snapshot under it. Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto`.
    pub fn rotate(&mut self) -> Result<(), SessionError> {
        self.require_admin()?;
        let entry = self.keys.write_rotation(&mut self.entropy)?;
        self.send_entries(&[entry])?;
        self.rotated();
        Ok(())
    }

    /// This device leaves the room by its own `RemoveMember` (criterion 27, ADR 0041 decision
    /// 2); an admin rotates later. Refused while no other admin would remain (decision 1). The
    /// caller then closes the session; the room's keys stay in the credential store.
    ///
    /// # Errors
    /// `NotAMember`, `LastAdmin`, `Offline`, `Ended`, `Crypto`.
    pub fn leave(&mut self) -> Result<(), SessionError> {
        if self.state == State::Ended {
            return Err(SessionError::Ended);
        }
        if !self.is_online() {
            return Err(SessionError::Offline);
        }
        let me = self.keys.me();
        let keyring = self.keys.keyring().ok_or(SessionError::NotAMember)?;
        if keyring.role(&me).is_none() {
            return Err(SessionError::NotAMember);
        }
        if !crate::admin_remains(keyring, &[me]) {
            return Err(SessionError::LastAdmin);
        }
        let entry = self.keys.write(Payload::RemoveMember { member: me })?;
        self.send_entries(&[entry])?;
        Ok(())
    }

    /// Gives `member` the role `role` by `SetRole` (ADR 0041 decision 1): Make admin, Make
    /// member, and Step down as `set_role(me, Member)`. Any admin may change any role; a
    /// demotion is refused while no other admin would remain. Nothing is written when the role
    /// is already `role`. Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember` (this device or `member`), `LastAdmin`, `Offline`, `Ended`,
    /// `Crypto`.
    pub fn set_role(&mut self, member: MemberId, role: Role) -> Result<(), SessionError> {
        self.require_admin()?;
        let keyring = self.keys.keyring().ok_or(SessionError::NotAMember)?;
        let current = keyring.role(&member).ok_or(SessionError::NotAMember)?;
        if current == role {
            return Ok(());
        }
        if role == Role::Member && !crate::admin_remains(keyring, &[member]) {
            return Err(SessionError::LastAdmin);
        }
        let entry = self.keys.write(Payload::SetRole { member, role })?;
        self.send_entries(&[entry])?;
        self.event(SessionEvent::KeyringChanged);
        Ok(())
    }

    /// Adds a former participant back as a member (ADR 0041 decision 5): no invite, the
    /// identity from [`Keyring::former_participants`], every epoch key this device holds
    /// wrapped to it (ADR 0044 decision 4). No rotation, no role but member. Admin only.
    ///
    /// # Errors
    /// `NotAdmin`, `NotAMember` (this device, or `member` never was a participant),
    /// `AlreadyMember`, `Offline`, `Ended`, `Crypto`.
    pub fn add_back(&mut self, member: MemberId) -> Result<(), SessionError> {
        self.require_admin()?;
        let keyring = self.keys.keyring().ok_or(SessionError::NotAMember)?;
        if keyring.role(&member).is_some() {
            return Err(SessionError::AlreadyMember);
        }
        let identity = keyring
            .former_participants()
            .into_iter()
            .find(|identity| identity.member == member)
            .ok_or(SessionError::NotAMember)?;
        let entry = self
            .keys
            .write_addition(identity, Role::Member, true, &mut self.entropy)?;
        self.send_entries(&[entry])?;
        self.event(SessionEvent::KeyringChanged);
        Ok(())
    }

    fn rotated(&mut self) {
        self.event(SessionEvent::KeyringChanged);
        self.snapshot_asked = true;
        let covers_seq = self.cursor();
        self.event(SessionEvent::SnapshotDue { covers_seq });
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

    /// Sends own entries, each confirmed by the `Pong` after them (ADR 0041 decision 11).
    fn send_entries(&mut self, entries: &[Entry]) -> Result<(), SessionError> {
        for entry in entries {
            entry.encode()?;
        }
        self.send_own(entries);
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
