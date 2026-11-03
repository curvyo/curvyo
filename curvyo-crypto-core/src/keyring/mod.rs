//! The document keyring (ADR 0010): a hash-linked log of signed entries and its causal replay.
//!
//! Replay is a pure function of the entry set: entries are deduplicated by hash, ordered
//! causally (ties by lowest hash), and each is accepted only if its signature verifies, its
//! parents are accepted and the state replayed from its own ancestors authorizes it. The
//! state of all accepted entries applies the fork rules of ADR 0010 §5.

mod entry;
mod graph;
mod log;
mod state;
mod validity;

use std::collections::BTreeSet;

pub use entry::{ENTRY_VERSION, Entry, EntryHash, EpochWrap, Payload, RecipientWrap, Role};
pub use log::{LOG_VERSION, decode_log, encode_log};
pub use state::{EpochRef, InviteState};

use crate::identity::{Identity, MemberId};
use crate::invite::InviteId;
use crate::wrap::Wrap;
use crate::{CryptoError, DocumentId};
use graph::Graph;
use state::{Admission, State};

/// A current participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Participant {
    /// Their public identity.
    pub identity: Identity,
    /// Their role.
    pub role: Role,
}

/// A wrap a member can write as `RewrapEpoch`: `recipient` is a current member without a
/// wrap of `epoch`, which the asking member holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MissingWrap {
    /// The epoch key to wrap.
    pub epoch: EpochRef,
    /// Who lacks it.
    pub recipient: Identity,
}

/// The replayed keyring of one document.
pub struct Keyring {
    graph: Graph,
    accepted: Vec<bool>,
    state: State,
}

impl Keyring {
    /// Replays a set of entries in any order.
    ///
    /// # Errors
    /// `NoGenesis` when no valid genesis entry is among them.
    pub fn replay(entries: &[Entry]) -> Result<Self, CryptoError> {
        let graph = Graph::new(entries);
        let mut accepted = vec![false; graph.entries.len()];
        // One root: the lowest-hash well-formed, well-signed genesis; any other root is refused.
        let genesis = graph.order.iter().copied().find(|&i| {
            let entry = &graph.entries[i];
            validity::genesis_is_valid(entry) && entry.verify_signature().is_ok()
        });
        let Some(genesis) = genesis else {
            return Err(CryptoError::NoGenesis);
        };
        accepted[genesis] = true;
        for &i in &graph.order {
            let entry = &graph.entries[i];
            if i == genesis
                || entry.parents.is_empty()
                || !graph.parents[i].iter().all(|&p| accepted[p])
                || entry.verify_signature().is_err()
            {
                continue;
            }
            let ancestors: BTreeSet<usize> = graph.ancestors[i].clone();
            accepted[i] = validity::is_authorized(&state::compute(&graph, &ancestors), entry);
        }
        let all: BTreeSet<usize> = (0..accepted.len()).filter(|&i| accepted[i]).collect();
        let state = state::compute(&graph, &all);
        Ok(Self {
            graph,
            accepted,
            state,
        })
    }

    /// The document (and room) the keyring belongs to.
    #[must_use]
    pub fn document_id(&self) -> DocumentId {
        // invariant: replay succeeds only with an accepted genesis, which sets the id.
        self.state.document_id.unwrap_or(DocumentId([0; 16]))
    }

    /// Whether the entry with this hash was accepted.
    #[must_use]
    pub fn is_accepted(&self, hash: &EntryHash) -> bool {
        self.graph
            .hashes
            .iter()
            .position(|h| h == hash)
            .is_some_and(|i| self.accepted[i])
    }

    /// The accepted entries in causal order: what a `keyring.log` stores.
    #[must_use]
    pub fn accepted_entries(&self) -> Vec<&Entry> {
        self.graph
            .order
            .iter()
            .filter(|&&i| self.accepted[i])
            .map(|&i| &self.graph.entries[i])
            .collect()
    }

    /// The hashes of the entries that were refused or wait for a missing parent.
    #[must_use]
    pub fn rejected(&self) -> Vec<EntryHash> {
        (0..self.accepted.len())
            .filter(|&i| !self.accepted[i])
            .map(|i| self.graph.hashes[i])
            .collect()
    }

    /// The accepted entries no other accepted entry builds on: the parents of the next entry.
    #[must_use]
    pub fn heads(&self) -> Vec<EntryHash> {
        let accepted = |i: usize| self.accepted[i];
        (0..self.accepted.len())
            .filter(|&i| accepted(i))
            .filter(|&i| {
                !(0..self.accepted.len()).any(|j| accepted(j) && self.graph.parents[j].contains(&i))
            })
            .map(|i| self.graph.hashes[i])
            .collect()
    }

    /// The current participants, ordered by member id.
    #[must_use]
    pub fn participants(&self) -> Vec<Participant> {
        self.state
            .members
            .values()
            .map(|m| Participant {
                identity: m.identity,
                role: m.role,
            })
            .collect()
    }

    /// A current participant's role.
    #[must_use]
    pub fn role(&self, member: &MemberId) -> Option<Role> {
        self.state.role(member)
    }

    /// Whether `member` was a participant and is no longer one.
    #[must_use]
    pub fn was_removed(&self, member: &MemberId) -> bool {
        self.state.ever_added.contains(member) && !self.state.members.contains_key(member)
    }

    /// Whether the rule "the admin set never becomes empty" dropped a removal or demotion; the
    /// UI reports it (ADR 0010 §5).
    #[must_use]
    pub fn admin_set_restored(&self) -> bool {
        self.state.admin_set_restored
    }

    /// The epoch new envelopes are sealed under.
    #[must_use]
    pub fn current_epoch(&self) -> EpochRef {
        self.state.current.map_or(
            EpochRef {
                number: 0,
                key_id: crate::KeyId([0; 8]),
            },
            |i| self.state.epochs[i].epoch,
        )
    }

    /// Every epoch key the keyring knows, in replay order.
    #[must_use]
    pub fn epochs(&self) -> Vec<EpochRef> {
        self.state.epochs.iter().map(|e| e.epoch).collect()
    }

    /// The highest epoch number; a rotation writes this plus one.
    #[must_use]
    pub fn highest_epoch_number(&self) -> u32 {
        self.state.highest_number()
    }

    /// Whether `member` holds a wrap of `epoch`.
    #[must_use]
    pub fn holds(&self, member: &MemberId, epoch: EpochRef) -> bool {
        self.state.epoch(epoch).is_some_and(|e| e.holds(member))
    }

    /// Every wrap addressed to `member`, to unwrap the keys it holds.
    #[must_use]
    pub fn wraps_for(&self, member: &MemberId) -> Vec<(EpochRef, Wrap)> {
        self.state
            .epochs
            .iter()
            .flat_map(|e| {
                e.wraps
                    .iter()
                    .filter(move |(m, _)| m == member)
                    .map(move |(_, w)| (e.epoch, *w))
            })
            .collect()
    }

    /// An invite's state.
    #[must_use]
    pub fn invite(&self, invite_id: &InviteId) -> Option<InviteState> {
        self.state.invites.get(invite_id).copied()
    }

    /// The `RewrapEpoch` entries `me` can write (ADR 0031 decision 6a): for each epoch `me`
    /// holds, each current member without a wrap of it. A member admitted by an invite and
    /// holding no key yet is left out once the invite's expiry is not after `now` (seconds
    /// since the Unix epoch, the caller's clock; ADR 0031 decision 8).
    #[must_use]
    pub fn missing_wraps(&self, me: &MemberId, now: u64) -> Vec<MissingWrap> {
        if !self.state.members.contains_key(me) {
            return Vec::new();
        }
        let holds_any = |m: &MemberId| self.state.epochs.iter().any(|e| e.holds(m));
        let mut missing = Vec::new();
        for epoch in self.state.epochs.iter().filter(|e| e.holds(me)) {
            for member in self.state.members.values() {
                if epoch.holds(&member.identity.member) {
                    continue;
                }
                if let Admission::Claimed(invite_id) = member.admission {
                    let expired = self
                        .state
                        .invites
                        .get(&invite_id)
                        .is_none_or(|i| i.expires <= now);
                    if expired && !holds_any(&member.identity.member) {
                        continue;
                    }
                }
                missing.push(MissingWrap {
                    epoch: epoch.epoch,
                    recipient: member.identity,
                });
            }
        }
        missing.sort();
        missing.dedup();
        missing
    }
}
