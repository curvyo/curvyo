//! The relay's admission rule (ADR 0031 decision 5): a connection is admitted if the room's
//! keyring lists its proven identity as a current member, if its first keyring entry is a valid
//! genesis of an unknown room, or a valid, unexpired claim of an open invite.

use std::collections::BTreeMap;

use curvyo_crypto_core::keyring::{Entry, Keyring, Payload};
use curvyo_crypto_core::{MemberId, verify, verify_claim_proof};

use super::{RelayOutput, RoomLog, Stage};
use crate::frame::{
    ClientFrame, ConnectionId, OLDEST_PROTOCOL_VERSION, PROTOCOL_VERSION, RefusalReason,
    RelayFrame, RoomId,
};
use crate::time::Millis;

const PROOF_TAG: &[u8] = b"curvyo-relay-v1";

/// What a client signs to prove its identity: `"curvyo-relay-v1" ‖ room ‖ nonce`.
pub(crate) fn relay_proof_message(room: RoomId, nonce: &[u8; 32]) -> Vec<u8> {
    let mut message = Vec::with_capacity(PROOF_TAG.len() + 16 + 32);
    message.extend_from_slice(PROOF_TAG);
    message.extend_from_slice(&room.0);
    message.extend_from_slice(nonce);
    message
}

/// Who is asking to be admitted.
pub(super) struct Claimant {
    pub(super) room: RoomId,
    pub(super) member: MemberId,
    pub(super) since_seq: u64,
}

type Verdict = Result<Option<Stage>, RefusalReason>;

/// `Hello` opens the handshake with a `Challenge`.
pub(super) fn hello(
    frame: &ClientFrame,
    nonce: [u8; 32],
    connection: ConnectionId,
    out: &mut Vec<RelayOutput>,
) -> Verdict {
    let ClientFrame::Hello {
        protocol,
        room,
        since_seq,
        identity,
    } = *frame
    else {
        return Err(RefusalReason::Protocol);
    };
    if !(OLDEST_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&protocol) {
        return Err(RefusalReason::Protocol);
    }
    out.push(RelayOutput::Send {
        connection,
        frame: RelayFrame::Challenge { nonce },
    });
    Ok(Some(Stage::Challenged {
        room,
        member: identity,
        since_seq,
        nonce,
    }))
}

/// `Proof` admits a current member at once; anyone else must offer a genesis or a claim next.
pub(super) fn proof(
    frame: &ClientFrame,
    claimant: &Claimant,
    nonce: &[u8; 32],
    rooms: &mut BTreeMap<RoomId, RoomLog>,
    connection: ConnectionId,
    out: &mut Vec<RelayOutput>,
) -> Verdict {
    let ClientFrame::Proof { signature } = frame else {
        return Err(RefusalReason::Protocol);
    };
    verify(
        &claimant.member,
        &relay_proof_message(claimant.room, nonce),
        signature,
    )
    .map_err(|_| RefusalReason::NotAMember)?;
    let proven = Stage::Proven {
        room: claimant.room,
        member: claimant.member,
        since_seq: claimant.since_seq,
    };
    let Some(log) = rooms.get_mut(&claimant.room) else {
        return Ok(Some(proven));
    };
    if log.keyring().role(&claimant.member).is_some() {
        log.admit(connection, claimant.member, claimant.since_seq, out);
        return Ok(Some(Stage::Admitted {
            room: claimant.room,
        }));
    }
    // A removed identity that knows nothing of the room (`since_seq` 0) may offer a claim of a
    // new invite and join again (customer 2026-11-09); anything else it sends is refused as
    // removed, and so is a reconnect from a cursor at once.
    if log.keyring().was_removed(&claimant.member) && claimant.since_seq != 0 {
        return Err(RefusalReason::Removed);
    }
    Ok(Some(proven))
}

/// A proven non-member's first keyring entry: a genesis creates the room, a claim joins it.
/// A claim without parents is a probe: the relay checks its invite proof and answers with the
/// keyring, so the joiner can build its real claim on the keyring's heads (Proposal). Nothing
/// reaches a connection that proved neither membership nor an invite.
pub(super) fn genesis_or_claim(
    frame: &ClientFrame,
    claimant: &Claimant,
    now: Millis,
    rooms: &mut BTreeMap<RoomId, RoomLog>,
    connection: ConnectionId,
    out: &mut Vec<RelayOutput>,
) -> Verdict {
    let removed = rooms
        .get(&claimant.room)
        .is_some_and(|log| log.keyring().was_removed(&claimant.member));
    let not_member = if removed {
        RefusalReason::Removed
    } else {
        RefusalReason::NotAMember
    };
    let ClientFrame::Keyring { entry: bytes } = frame else {
        return Err(not_member);
    };
    let entry = Entry::decode(bytes).map_err(|_| RefusalReason::Protocol)?;
    if entry.author != claimant.member {
        return Err(RefusalReason::NotAMember);
    }
    let admitted = Ok(Some(Stage::Admitted {
        room: claimant.room,
    }));
    let Some(log) = rooms.get_mut(&claimant.room) else {
        let mut log = genesis(&entry, bytes, claimant.room)?;
        out.push(RelayOutput::Store(super::StoreOp::Keyring {
            room: claimant.room,
            entry: bytes.clone(),
        }));
        log.admit(connection, claimant.member, claimant.since_seq, out);
        rooms.insert(claimant.room, log);
        return admitted;
    };
    let Payload::ClaimInvite {
        invite_id,
        identity,
        proof,
    } = &entry.payload
    else {
        return Err(not_member);
    };
    if identity.member != claimant.member {
        return Err(RefusalReason::NotAMember);
    }
    verify_claim_proof(*invite_id, claimant.room.document_id(), identity, proof)
        .map_err(|_| RefusalReason::NotAMember)?;
    let invite = log
        .keyring()
        .invite(invite_id)
        .ok_or(RefusalReason::NotAMember)?;
    if invite.claimed_by.is_some_and(|m| m != claimant.member) {
        return Err(RefusalReason::Used);
    }
    if invite.expires <= now.unix_seconds() {
        return Err(RefusalReason::Expired);
    }
    if entry.parents.is_empty() {
        log.send_keyring(connection, out);
        return Ok(None);
    }
    if !log.append_entry(entry, bytes, Some(connection), out) {
        return Err(RefusalReason::Protocol);
    }
    log.admit(connection, claimant.member, claimant.since_seq, out);
    admitted
}

/// A new room from a valid genesis naming this room's document.
fn genesis(entry: &Entry, bytes: &[u8], room: RoomId) -> Result<RoomLog, RefusalReason> {
    if !matches!(entry.payload, Payload::Genesis { .. }) {
        return Err(RefusalReason::NotAMember);
    }
    let keyring =
        Keyring::replay(core::slice::from_ref(entry)).map_err(|_| RefusalReason::Protocol)?;
    if keyring.document_id() != room.document_id() {
        return Err(RefusalReason::Protocol);
    }
    Ok(RoomLog::new(room, entry.clone(), bytes.to_vec(), keyring))
}
