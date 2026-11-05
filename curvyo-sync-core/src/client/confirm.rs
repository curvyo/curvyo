//! Which own keyring entries the relay holds (ADR 0041 decision 11): a `Ping` after every sent
//! entry, its `Pong` as the confirmation, the resend of what the relay lacks after each
//! catch-up, and the confirmed view a removed device ends with.

use std::collections::BTreeSet;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{Entry, EntryHash, Keyring, Payload};

use super::{ClientSession, SessionEvent};
use crate::frame::ClientFrame;

/// The own entries sent before one `Ping`, judged when its `Pong` arrives.
struct PingBatch {
    entries: Vec<EntryHash>,
    /// The resend after the catch-up: its `Pong` ends the catch-up.
    ends_catch_up: bool,
}

/// What an own keyring act waiting for the relay's confirmation does to a participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingKind {
    /// Make admin.
    MakeAdmin,
    /// Make member or Step down.
    MakeMember,
    /// Remove.
    Remove,
    /// This device's own leave.
    Leave,
    /// Add back, or the addition of a device.
    Add,
}

/// An own keyring act the relay has not confirmed yet (ADR 0041 decision 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingAct {
    /// The participant it concerns.
    pub member: MemberId,
    /// What it does.
    pub kind: PendingKind,
}

/// The confirmation state of one session; nothing of it is persisted.
#[derive(Default)]
pub(super) struct Confirmation {
    /// Every entry the relay sent and every own entry a `Pong` confirmed; rebuilt on `Welcome`.
    confirmed: BTreeSet<EntryHash>,
    /// Whether a `Welcome` arrived in this session.
    welcomed: bool,
    /// Own entries sent since the last `Ping`.
    unpinged: Vec<EntryHash>,
    /// One batch per `Ping` in flight on this connection, oldest first.
    pings: std::collections::VecDeque<PingBatch>,
    /// Own entries the relay's replay refused: never sent again.
    refused: BTreeSet<EntryHash>,
    /// The catch-up's resend went out on this connection.
    resending: bool,
    /// The relay's keyring as it arrives after a `Welcome`; it becomes the confirmed set at the
    /// catch-up's first `Pong`, so the set is never empty or partial in between.
    staging: Option<BTreeSet<EntryHash>>,
}

impl Confirmation {
    /// Before any `Welcome`, the entries of `keyring.log` count as confirmed except this
    /// device's own acts (its genesis and its claim of an invite were stored, or the file was
    /// never shared): a removed device that never reached the room again keeps them out of its
    /// heads, which only makes them older.
    pub(super) fn from_log(entries: &[Entry], me: MemberId) -> Self {
        let confirmed = entries
            .iter()
            .filter(|e| {
                e.author != me
                    || matches!(
                        e.payload,
                        Payload::Genesis { .. } | Payload::ClaimInvite { .. }
                    )
            })
            .filter_map(|e| e.try_hash().ok())
            .collect();
        Self {
            confirmed,
            ..Self::default()
        }
    }
}

impl ClientSession {
    /// Sends a `Ping`; its `Pong` confirms the own entries sent before it.
    pub(super) fn send_ping(&mut self) {
        self.ping_batch(false);
    }

    fn ping_batch(&mut self, ends_catch_up: bool) {
        let entries = core::mem::take(&mut self.confirmation.unpinged);
        self.confirmation.pings.push_back(PingBatch {
            entries,
            ends_catch_up,
        });
        self.send(&ClientFrame::Ping);
    }

    /// Sends own entries and the `Ping` that confirms them.
    pub(super) fn send_own(&mut self, entries: &[Entry]) {
        if entries.is_empty() {
            return;
        }
        for entry in entries {
            let (Ok(bytes), Ok(hash)) = (entry.encode(), entry.try_hash()) else {
                continue;
            };
            self.confirmation.unpinged.push(hash);
            self.send(&ClientFrame::Keyring { entry: bytes });
        }
        self.send_ping();
    }

    /// A new connection: nothing is in flight on it.
    pub(super) fn reset_pings(&mut self) {
        self.confirmation.unpinged.clear();
        self.confirmation.pings.clear();
        self.confirmation.resending = false;
    }

    /// A `Welcome`: the relay's keyring follows and rebuilds the confirmed set.
    pub(super) fn welcomed(&mut self) {
        self.confirmation.staging = Some(BTreeSet::new());
    }

    /// An entry the relay sent: it holds it.
    pub(super) fn relay_holds(&mut self, hash: EntryHash) {
        match self.confirmation.staging.as_mut() {
            Some(staging) => staging.insert(hash),
            None => self.confirmation.confirmed.insert(hash),
        };
    }

    /// The catch-up's first `Pong`: the relay's whole keyring has arrived and replaces the
    /// confirmed set.
    pub(super) fn adopt_relay_keyring(&mut self) {
        if let Some(staging) = self.confirmation.staging.take() {
            self.confirmation.confirmed = staging;
            self.confirmation.welcomed = true;
        }
    }

    /// A `Pong`: confirms the batch of its `Ping`. An entry the replay of the confirmed
    /// entries refuses was refused by the relay too: it leaves the local log. Returns whether
    /// this `Pong` ends the catch-up's resend.
    pub(super) fn ponged(&mut self) -> bool {
        let Some(batch) = self.confirmation.pings.pop_front() else {
            return false;
        };
        let mut known = self.confirmation.confirmed.clone();
        let mut dropped = Vec::new();
        for hash in batch.entries {
            // An entry on a parent not confirmed is not judged here: it stays unconfirmed and
            // goes out again after the next catch-up.
            let parents_known = self
                .keys
                .entry(&hash)
                .is_some_and(|e| e.parents.iter().all(|p| known.contains(p)));
            if !parents_known {
                continue;
            }
            let mut with = known.clone();
            with.insert(hash);
            if self.replay_of(&with).is_some_and(|k| k.is_accepted(&hash)) {
                known.insert(hash);
                self.confirmation.confirmed.insert(hash);
            } else {
                self.confirmation.refused.insert(hash);
                dropped.push(hash);
            }
        }
        if !dropped.is_empty() && self.keys.drop_entries(&dropped) {
            self.event(SessionEvent::KeyringChanged);
        }
        batch.ends_catch_up
    }

    /// After the catch-up: sends every local entry the relay lacks, parents first, then one
    /// `Ping`. Returns whether anything was resent (the catch-up then ends at its `Pong`).
    pub(super) fn resend_unconfirmed(&mut self) -> bool {
        let in_flight: BTreeSet<EntryHash> = self
            .confirmation
            .pings
            .iter()
            .flat_map(|b| b.entries.iter().copied())
            .chain(self.confirmation.unpinged.iter().copied())
            .collect();
        let missing: Vec<Entry> = self
            .keys
            .keyring()
            .map(|k| {
                k.accepted_entries()
                    .into_iter()
                    .filter(|e| {
                        !matches!(
                            e.payload,
                            Payload::Genesis { .. } | Payload::ClaimInvite { .. }
                        )
                    })
                    .filter(|e| {
                        e.try_hash().is_ok_and(|h| {
                            !self.confirmation.confirmed.contains(&h)
                                && !self.confirmation.refused.contains(&h)
                                && !in_flight.contains(&h)
                        })
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if missing.is_empty() {
            return false;
        }
        for entry in &missing {
            let (Ok(bytes), Ok(hash)) = (entry.encode(), entry.try_hash()) else {
                continue;
            };
            self.confirmation.unpinged.push(hash);
            self.send(&ClientFrame::Keyring { entry: bytes });
        }
        self.confirmation.resending = true;
        self.ping_batch(true);
        true
    }

    /// Ends the session as removed: the local log keeps only what the relay confirmed, so an
    /// own entry it never stored is no head of the ended record (ADR 0041 decision 11).
    pub(super) fn end_removed(&mut self) {
        let confirmed = self.confirmation.confirmed.clone();
        if self.keys.keep_only(&confirmed) {
            self.event(SessionEvent::KeyringChanged);
        }
        self.event(SessionEvent::Removed);
        self.end();
    }

    /// The replay of the local entries the relay confirmed (plus `extra`).
    fn replay_of(&self, hashes: &BTreeSet<EntryHash>) -> Option<Keyring> {
        let entries: Vec<Entry> = self
            .keys
            .entries()
            .iter()
            .filter(|e| e.try_hash().is_ok_and(|h| hashes.contains(&h)))
            .cloned()
            .collect();
        Keyring::replay(&entries).ok()
    }

    /// The keyring as the relay confirmed it to this device; `None` before any entry is known.
    #[must_use]
    pub fn confirmed_keyring(&self) -> Option<Keyring> {
        self.replay_of(&self.confirmation.confirmed)
    }

    /// The heads of the confirmed keyring, for the ended record's `heads` (ADR 0041 decision
    /// 6): an own entry the relay never stored is never one of them.
    #[must_use]
    pub fn confirmed_heads(&self) -> Vec<EntryHash> {
        self.confirmed_keyring()
            .map(|k| k.heads())
            .unwrap_or_default()
    }

    /// The own keyring acts the relay has not confirmed yet, per participant, in the order
    /// written. Before this session's `Welcome` only a leave counts: older own entries were
    /// most likely confirmed in an earlier session.
    #[must_use]
    pub fn pending_acts(&self) -> Vec<PendingAct> {
        let me = self.keys.me();
        let Some(keyring) = self.keys.keyring() else {
            return Vec::new();
        };
        keyring
            .accepted_entries()
            .into_iter()
            .filter(|e| e.author == me)
            .filter(|e| {
                e.try_hash()
                    .is_ok_and(|h| !self.confirmation.confirmed.contains(&h))
            })
            .filter_map(|e| pending_of(&e.payload, me))
            .filter(|act| self.confirmation.welcomed || act.kind == PendingKind::Leave)
            .collect()
    }

    /// Whether the catch-up's resend is under way on this connection.
    pub(super) fn is_resending(&self) -> bool {
        self.confirmation.resending
    }
}

fn pending_of(payload: &Payload, me: MemberId) -> Option<PendingAct> {
    let (member, kind) = match payload {
        Payload::SetRole { member, role } => (
            *member,
            if *role == curvyo_crypto_core::keyring::Role::Admin {
                PendingKind::MakeAdmin
            } else {
                PendingKind::MakeMember
            },
        ),
        Payload::RemoveMember { member } if *member == me => (*member, PendingKind::Leave),
        Payload::RemoveMember { member } => (*member, PendingKind::Remove),
        Payload::AddMember { identity, .. } => (identity.member, PendingKind::Add),
        _ => return None,
    };
    Some(PendingAct { member, kind })
}
