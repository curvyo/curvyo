//! A client's keyring and epoch keys: replaying received entries, unwrapping the keys addressed
//! to it, sealing under the current epoch and opening under any epoch it holds.

use std::collections::BTreeMap;

use curvyo_crypto_core::keyring::{
    Entry, EntryHash, EpochWrap, Keyring, Payload, RecipientWrap, Role,
};
use curvyo_crypto_core::{
    CryptoError, DocumentId, EnvelopeHeader, EnvelopeKind, EpochKey, Identity, IdentityKeys,
    InviteKey, KeyId, MemberId, NonceStream, Opened, Sealing, open, seal, unwrap_epoch_key,
    wrap_epoch_key,
};

use super::SessionError;
use crate::entropy::Entropy;

/// The keyring entries a client knows, their replay, and the epoch keys it holds.
pub(super) struct Keys {
    pub(super) identity: IdentityKeys,
    document_id: DocumentId,
    entries: Vec<Entry>,
    hashes: Vec<EntryHash>,
    keyring: Option<Keyring>,
    held: BTreeMap<KeyId, (u32, EpochKey)>,
    nonces: NonceStream,
    peer_id: u64,
}

impl Keys {
    pub(super) fn new(
        identity: IdentityKeys,
        document_id: DocumentId,
        peer_id: u64,
        nonce_seed: [u8; 32],
    ) -> Self {
        Self {
            identity,
            document_id,
            entries: Vec::new(),
            hashes: Vec::new(),
            keyring: None,
            held: BTreeMap::new(),
            nonces: NonceStream::new(nonce_seed),
            peer_id,
        }
    }

    pub(super) fn me(&self) -> MemberId {
        self.identity.identity().member
    }

    pub(super) fn keyring(&self) -> Option<&Keyring> {
        self.keyring.as_ref()
    }

    pub(super) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The local entry with `hash`.
    pub(super) fn entry(&self, hash: &EntryHash) -> Option<&Entry> {
        self.hashes
            .iter()
            .position(|h| h == hash)
            .map(|i| &self.entries[i])
    }

    pub(super) fn held_keys(&self) -> Vec<(u32, EpochKey)> {
        self.held.values().cloned().collect()
    }

    pub(super) fn holds_any_key(&self) -> bool {
        !self.held.is_empty()
    }

    /// Keeps keys from the credential store.
    pub(super) fn add_keys(&mut self, keys: Vec<(u32, EpochKey)>) {
        for (epoch, key) in keys {
            self.held.insert(key.key_id(), (epoch, key));
        }
    }

    /// Adds entries and replays. Returns whether anything new was accepted.
    pub(super) fn add_entries(&mut self, entries: Vec<Entry>) -> bool {
        let mut grew = false;
        for entry in entries {
            let Ok(hash) = entry.try_hash() else {
                continue;
            };
            if !self.hashes.contains(&hash) {
                self.hashes.push(hash);
                self.entries.push(entry);
                grew = true;
            }
        }
        if !grew {
            return false;
        }
        let before = self.keyring.as_ref().map(|k| k.accepted_entries().len());
        self.keyring = Keyring::replay(&self.entries).ok();
        self.unwrap_mine();
        self.keyring.as_ref().map(|k| k.accepted_entries().len()) != before
    }

    /// Removes `dropped` from the local entries and replays; returns whether the accepted
    /// entries changed.
    pub(super) fn drop_entries(&mut self, dropped: &[EntryHash]) -> bool {
        self.retain(|hash| !dropped.contains(hash))
    }

    /// Keeps only the entries in `kept` and replays; returns whether the accepted entries
    /// changed.
    pub(super) fn keep_only(&mut self, kept: &std::collections::BTreeSet<EntryHash>) -> bool {
        self.retain(|hash| kept.contains(hash))
    }

    fn retain(&mut self, keep: impl Fn(&EntryHash) -> bool) -> bool {
        let before: Vec<EntryHash> = self
            .keyring
            .as_ref()
            .map(|k| {
                k.accepted_entries()
                    .iter()
                    .filter_map(|e| e.try_hash().ok())
                    .collect()
            })
            .unwrap_or_default();
        let (hashes, entries): (Vec<EntryHash>, Vec<Entry>) = self
            .hashes
            .drain(..)
            .zip(self.entries.drain(..))
            .filter(|(hash, _)| keep(hash))
            .unzip();
        self.hashes = hashes;
        self.entries = entries;
        self.keyring = Keyring::replay(&self.entries).ok();
        let after: Vec<EntryHash> = self
            .keyring
            .as_ref()
            .map(|k| {
                k.accepted_entries()
                    .iter()
                    .filter_map(|e| e.try_hash().ok())
                    .collect()
            })
            .unwrap_or_default();
        before != after
    }

    fn unwrap_mine(&mut self) {
        let Some(keyring) = &self.keyring else {
            return;
        };
        for (epoch, wrap) in keyring.wraps_for(&self.identity.identity().member) {
            if self.held.contains_key(&wrap.key_id()) {
                continue;
            }
            if let Ok(key) = unwrap_epoch_key(&wrap, epoch.number, self.document_id, &self.identity)
            {
                self.held.insert(key.key_id(), (epoch.number, key));
            }
        }
    }

    /// Signs a new entry on the current heads and replays it in.
    pub(super) fn write(&mut self, payload: Payload) -> Result<Entry, SessionError> {
        let parents = self
            .keyring
            .as_ref()
            .map(Keyring::heads)
            .unwrap_or_default();
        let entry = Entry::sign(parents, &self.identity, payload)?;
        self.add_entries(vec![entry.clone()]);
        Ok(entry)
    }

    /// The genesis of a document shared now, with this device as its only admin.
    pub(super) fn write_genesis(&mut self, entropy: &mut Entropy) -> Result<Entry, SessionError> {
        let key = EpochKey::from_bytes(entropy.secret());
        let me = self.identity.identity();
        let wrap = wrap_epoch_key(&key, 0, self.document_id, &me, entropy.secret())?;
        let entry = Entry::sign(
            Vec::new(),
            &self.identity,
            Payload::Genesis {
                document_id: self.document_id,
                admins: vec![me],
                epoch0_wraps: vec![RecipientWrap {
                    member: me.member,
                    wrap,
                }],
            },
        )?;
        self.add_entries(vec![entry.clone()]);
        Ok(entry)
    }

    /// A `ClaimInvite` for the invite `key`; without parents it is the relay probe.
    pub(super) fn claim(&mut self, key: &InviteKey, probe: bool) -> Result<Entry, SessionError> {
        let me = self.identity.identity();
        let payload = Payload::ClaimInvite {
            invite_id: key.invite_id(),
            identity: me,
            proof: key.claim_proof(self.document_id, &me),
        };
        if probe {
            return Ok(Entry::sign(Vec::new(), &self.identity, payload)?);
        }
        self.write(payload)
    }

    /// `AddMember` of `identity` with `role` and a wrap of every epoch key held. Refuses a
    /// member, and an identity that was removed unless `readmit`: this client re-adds a removed
    /// identity only for a device of the maker's group that a device of the group removed (ADR
    /// 0040 decision 5; ADR 0010 §8 itself allows re-adding).
    pub(super) fn write_addition(
        &mut self,
        identity: Identity,
        role: Role,
        readmit: bool,
        entropy: &mut Entropy,
    ) -> Result<Entry, SessionError> {
        let keyring = self.keyring.as_ref().ok_or(SessionError::NotAMember)?;
        if keyring.role(&identity.member).is_some()
            || (!readmit && keyring.was_removed(&identity.member))
        {
            return Err(SessionError::AlreadyMember);
        }
        let mut wraps = Vec::with_capacity(self.held.len());
        for (epoch, key) in self.held.values() {
            wraps.push(EpochWrap {
                epoch: *epoch,
                wrap: wrap_epoch_key(key, *epoch, self.document_id, &identity, entropy.secret())?,
            });
        }
        self.write(Payload::AddMember {
            identity,
            role,
            wraps,
        })
    }

    /// A `RemoveMember` per member, then one `NewEpoch` (highest + 1) with a fresh key wrapped
    /// to every remaining member (ADR 0031 decision 6a, ADR 0040 decision 5).
    pub(super) fn write_removal(
        &mut self,
        members: &[MemberId],
        entropy: &mut Entropy,
    ) -> Result<Vec<Entry>, SessionError> {
        let keyring = self.keyring.as_ref().ok_or(SessionError::NotAMember)?;
        if keyring.role(&self.me()) != Some(Role::Admin) {
            return Err(SessionError::NotAdmin);
        }
        if members.is_empty()
            || members
                .iter()
                .any(|member| keyring.role(member).is_none() || *member == self.me())
        {
            return Err(SessionError::NotAMember);
        }
        let mut written = Vec::with_capacity(members.len() + 1);
        for member in members {
            written.push(self.write(Payload::RemoveMember { member: *member })?);
        }
        written.push(self.write_rotation(entropy)?);
        Ok(written)
    }

    /// One `NewEpoch` (highest + 1) with a fresh key wrapped to every current member.
    pub(super) fn write_rotation(&mut self, entropy: &mut Entropy) -> Result<Entry, SessionError> {
        let keyring = self.keyring.as_ref().ok_or(SessionError::NotAMember)?;
        if keyring.role(&self.me()) != Some(Role::Admin) {
            return Err(SessionError::NotAdmin);
        }
        let epoch = keyring.highest_epoch_number() + 1;
        let remaining = keyring.participants();
        let key = EpochKey::from_bytes(entropy.secret());
        let mut wraps = Vec::with_capacity(remaining.len());
        for participant in remaining {
            wraps.push(RecipientWrap {
                member: participant.identity.member,
                wrap: wrap_epoch_key(
                    &key,
                    epoch,
                    self.document_id,
                    &participant.identity,
                    entropy.secret(),
                )?,
            });
        }
        self.held.insert(key.key_id(), (epoch, key));
        self.write(Payload::NewEpoch { epoch, wraps })
    }

    /// The `RewrapEpoch` entries this device owes other members (ADR 0031 decision 6a), at
    /// `now_seconds` by this device's clock.
    pub(super) fn write_rewraps(
        &mut self,
        now_seconds: u64,
        entropy: &mut Entropy,
    ) -> Result<Vec<Entry>, SessionError> {
        let mut written = Vec::new();
        loop {
            let Some(keyring) = &self.keyring else {
                return Ok(written);
            };
            let missing = keyring.missing_wraps(&self.me(), now_seconds);
            let Some(next) = missing
                .into_iter()
                .find(|m| self.held.contains_key(&m.epoch.key_id))
            else {
                return Ok(written);
            };
            let Some((_, key)) = self.held.get(&next.epoch.key_id) else {
                return Ok(written);
            };
            let wrap = wrap_epoch_key(
                key,
                next.epoch.number,
                self.document_id,
                &next.recipient,
                entropy.secret(),
            )?;
            written.push(self.write(Payload::RewrapEpoch {
                epoch: next.epoch.number,
                member: next.recipient.member,
                wrap,
            })?);
        }
    }

    /// Seals under the keyring's current epoch; `None` while this device does not hold it.
    pub(super) fn seal(
        &mut self,
        kind: EnvelopeKind,
        plaintext: &[u8],
    ) -> Result<Option<Vec<u8>>, SessionError> {
        let Some(current) = self.keyring.as_ref().map(Keyring::current_epoch) else {
            return Ok(None);
        };
        let Some((epoch, key)) = self.held.get(&current.key_id) else {
            return Ok(None);
        };
        let signer = (kind == EnvelopeKind::Snapshot).then_some(&self.identity);
        let sealing = Sealing {
            kind,
            epoch: *epoch,
            key,
            document_id: self.document_id,
            sender_peer: self.peer_id,
            signer,
        };
        let nonce = self.nonces.next_nonce()?;
        Ok(Some(seal(&sealing, nonce, plaintext)?))
    }

    /// Whether this device holds the key an envelope names.
    pub(super) fn can_open(&self, envelope: &[u8]) -> bool {
        EnvelopeHeader::parse(envelope).is_ok_and(|h| self.held.contains_key(&h.key_id))
    }

    /// Opens an envelope of `kind` under the key its header names. A signed snapshot must be
    /// signed by a current member.
    pub(super) fn open(&self, envelope: &[u8], kind: EnvelopeKind) -> Result<Opened, CryptoError> {
        let header = EnvelopeHeader::parse(envelope)?;
        let (_, key) = self.held.get(&header.key_id).ok_or(CryptoError::WrongKey)?;
        let opened = open(envelope, key, self.document_id, kind)?;
        if let Some(signer) = opened.signer {
            let known = self
                .keyring
                .as_ref()
                .is_some_and(|k| k.role(&signer).is_some());
            if !known {
                return Err(CryptoError::BadSignature);
            }
        }
        Ok(opened)
    }
}
