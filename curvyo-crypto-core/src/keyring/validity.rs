//! The authorization rule of ADR 0010 §4: whether an entry is allowed by the state of its own
//! ancestors.

use std::collections::BTreeSet;

use crate::identity::MemberId;
use crate::invite::verify_claim_proof;
use crate::keyring::state::{EpochRef, State};
use crate::keyring::{Entry, Payload, RecipientWrap};

/// Whether a well-signed genesis entry is well-formed: its author is one of its admins, and
/// epoch 0's key (one `key_id`) is wrapped to exactly the admins.
pub(crate) fn genesis_is_valid(entry: &Entry) -> bool {
    let Payload::Genesis {
        admins,
        epoch0_wraps,
        ..
    } = &entry.payload
    else {
        return false;
    };
    let admin_ids: BTreeSet<MemberId> = admins.iter().map(|a| a.member).collect();
    entry.parents.is_empty()
        && admin_ids.len() == admins.len()
        && admin_ids.contains(&entry.author)
        && wraps_cover_exactly(epoch0_wraps, &admin_ids)
}

/// Whether a non-genesis entry is authorized by `state`, the replay of its ancestors.
pub(crate) fn is_authorized(state: &State, entry: &Entry) -> bool {
    let author = &entry.author;
    match &entry.payload {
        Payload::Genesis { .. } => false,
        Payload::AddMember {
            identity, wraps, ..
        } => {
            let mut epochs = BTreeSet::new();
            state.is_admin(author)
                && !state.members.contains_key(&identity.member)
                && wraps.iter().all(|w| {
                    let epoch = EpochRef {
                        number: w.epoch,
                        key_id: w.wrap.key_id(),
                    };
                    state.epoch(epoch).is_some() && epochs.insert(epoch)
                })
        }
        Payload::RemoveMember { member } => {
            state.members.contains_key(member) && (state.is_admin(author) || author == member)
        }
        Payload::SetRole { member, .. } => {
            state.is_admin(author) && state.members.contains_key(member)
        }
        Payload::NewEpoch { epoch, wraps } => {
            let members: BTreeSet<MemberId> = state.members.keys().copied().collect();
            state.is_admin(author)
                && state.highest_number().checked_add(1) == Some(*epoch)
                && wraps_cover_exactly(wraps, &members)
                && wraps.first().is_some_and(|w| {
                    state
                        .epochs
                        .iter()
                        .all(|e| e.epoch.key_id != w.wrap.key_id())
                })
        }
        Payload::AddPendingInvite { invite_id, .. } => {
            state.is_admin(author) && !state.invites.contains_key(invite_id)
        }
        Payload::ClaimInvite {
            invite_id,
            identity,
            proof,
        } => {
            let Some(document_id) = state.document_id else {
                return false;
            };
            state
                .invites
                .get(invite_id)
                .is_some_and(|invite| invite.claimed_by.is_none())
                && *author == identity.member
                && !state.members.contains_key(&identity.member)
                && verify_claim_proof(*invite_id, document_id, identity, proof).is_ok()
        }
        Payload::RewrapEpoch {
            epoch,
            member,
            wrap,
        } => {
            let epoch = EpochRef {
                number: *epoch,
                key_id: wrap.key_id(),
            };
            state.members.contains_key(author)
                && state.members.contains_key(member)
                && state.epoch(epoch).is_some_and(|e| e.holds(author))
        }
    }
}

/// One `key_id` for all wraps, one wrap per recipient, and the recipients are exactly `members`.
fn wraps_cover_exactly(wraps: &[RecipientWrap], members: &BTreeSet<MemberId>) -> bool {
    let Some(first) = wraps.first() else {
        return false;
    };
    let recipients: BTreeSet<MemberId> = wraps.iter().map(|w| w.member).collect();
    recipients.len() == wraps.len()
        && recipients == *members
        && wraps.iter().all(|w| w.wrap.key_id() == first.wrap.key_id())
}
