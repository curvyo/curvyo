//! Causal replay of the keyring for sharing and joining (`specs/0092` criteria 1 to 4, 27;
//! ADR 0010 §4, ADR 0031 decision 8). Each scenario is also checked exhaustively
//! (`support::check_exhaustively`).

#![allow(clippy::unwrap_used, clippy::too_many_lines)]

mod support;

use curvyo_crypto_core::keyring::{Entry, Keyring, Payload, Role};
use curvyo_crypto_core::{CryptoError, unwrap_epoch_key};
use support::{
    DOC, check_exhaustively, claim, device, entry, epoch, epoch_key, genesis, hash, id, invite,
    invite_key, replay, rewrap, wraps,
};

/// Criterion 1 (a): sharing writes a genesis with the sharer as the only admin.
#[test]
fn a_genesis_makes_its_author_the_only_admin_holding_epoch_0() {
    let a = device(1);
    let g = genesis(&a);
    let keyring = replay(&[&g]);
    assert_eq!(keyring.document_id(), DOC);
    let participants = keyring.participants();
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].identity, a.identity());
    assert_eq!(participants[0].role, Role::Admin);
    assert_eq!(keyring.current_epoch(), epoch(0, &epoch_key(0)));
    let (held, wrap) = keyring.wraps_for(&id(&a))[0];
    assert_eq!(
        unwrap_epoch_key(&wrap, held.number, DOC, &a).unwrap(),
        epoch_key(0)
    );
    assert_eq!(keyring.heads(), vec![hash(&g)]);
}

#[test]
fn without_a_valid_genesis_nothing_replays() {
    let (a, b) = (device(1), device(2));
    let g = genesis(&a);
    let mut forged = g.clone();
    forged.signature[0] ^= 1;
    assert!(matches!(
        Keyring::replay(&[forged]),
        Err(CryptoError::NoGenesis)
    ));
    // A genesis whose author is not among its admins.
    let foreign = Entry::sign(
        Vec::new(),
        &b,
        Payload::Genesis {
            document_id: DOC,
            admins: vec![a.identity()],
            epoch0_wraps: wraps(&epoch_key(0), 0, &[&a], 0),
        },
    )
    .unwrap();
    assert!(matches!(
        Keyring::replay(&[foreign]),
        Err(CryptoError::NoGenesis)
    ));
}

/// Criteria 2 and 3: invite, claim, and the online member's `RewrapEpoch` admit the joiner.
#[test]
fn a_claim_completed_by_a_rewrap_admits_the_joiner() {
    let (a, b) = (device(1), device(2));
    let g = genesis(&a);
    let inv = invite(&[&g], &a, 1, 100);
    let cl = claim(&[&inv], &b, 1);
    let waiting = replay(&[&g, &inv, &cl]);
    assert_eq!(waiting.role(&id(&b)), Some(Role::Member));
    assert_eq!(
        waiting
            .invite(&invite_key(1).invite_id())
            .unwrap()
            .claimed_by,
        Some(id(&b))
    );
    assert_eq!(
        waiting.wraps_for(&id(&b)),
        vec![],
        "the joiner holds no key yet"
    );
    let missing = waiting.missing_wraps(&id(&a), 50);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].epoch, epoch(0, &epoch_key(0)));
    assert_eq!(missing[0].recipient, b.identity());
    // The joiner cannot write the rewrap itself, holding nothing.
    assert_eq!(waiting.missing_wraps(&id(&b), 50), vec![]);
    let rw = rewrap(&[&cl], &a, 0, &epoch_key(0), &b, 1);
    let joined = replay(&[&g, &inv, &cl, &rw]);
    assert!(joined.is_accepted(&hash(&rw)));
    let (held, wrap) = joined.wraps_for(&id(&b))[0];
    assert_eq!(
        unwrap_epoch_key(&wrap, held.number, DOC, &b).unwrap(),
        epoch_key(0)
    );
    assert_eq!(joined.missing_wraps(&id(&a), 50), vec![]);
    assert_eq!(joined.participants().len(), 2);
    check_exhaustively(&[&g, &inv, &cl, &rw]);
}

/// ADR 0031 decision 8: a completing member checks the expiry with its own clock.
#[test]
fn an_expired_claim_is_not_completed() {
    let (a, b) = (device(1), device(2));
    let g = genesis(&a);
    let inv = invite(&[&g], &a, 1, 100);
    let cl = claim(&[&inv], &b, 1);
    let keyring = replay(&[&g, &inv, &cl]);
    assert_eq!(
        keyring.invite(&invite_key(1).invite_id()).unwrap().expires,
        100
    );
    assert_eq!(keyring.missing_wraps(&id(&a), 99).len(), 1);
    assert_eq!(keyring.missing_wraps(&id(&a), 100), vec![]);
    assert_eq!(keyring.missing_wraps(&id(&a), 5_000), vec![]);
}

/// Criterion 4: a used link admits nobody else; of two concurrent claims one wins.
#[test]
fn an_invite_is_claimed_once() {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let inv = invite(&[&g], &a, 1, 100);
    let cl_b = claim(&[&inv], &b, 1);
    let late = claim(&[&cl_b], &c, 1);
    let keyring = replay(&[&g, &inv, &cl_b, &late]);
    assert!(!keyring.is_accepted(&hash(&late)));
    assert_eq!(keyring.role(&id(&c)), None);
    check_exhaustively(&[&g, &inv, &cl_b, &late]);

    let cl_c = claim(&[&inv], &c, 1);
    let both = replay(&[&g, &inv, &cl_b, &cl_c]);
    let winner = if hash(&cl_b) < hash(&cl_c) { &b } else { &c };
    let loser = if hash(&cl_b) < hash(&cl_c) { &c } else { &b };
    assert_eq!(both.role(&id(winner)), Some(Role::Member));
    assert_eq!(both.role(&id(loser)), None);
    assert_eq!(
        both.invite(&invite_key(1).invite_id()).unwrap().claimed_by,
        Some(id(winner))
    );
    assert!(
        both.missing_wraps(&id(&a), 0)
            .iter()
            .all(|m| m.recipient == winner.identity())
    );
    check_exhaustively(&[&g, &inv, &cl_b, &cl_c]);
}

/// Criterion 27: an uninvited device can neither claim nor rewrap, and the participant list
/// does not change.
#[test]
fn an_uninvited_device_changes_nothing() {
    let (a, d) = (device(1), device(4));
    let g = genesis(&a);
    let inv = invite(&[&g], &a, 1, 100);
    // A claim with a proof made by another invite key.
    let wrong = invite_key(2);
    let forged_claim = entry(
        &[&inv],
        &d,
        Payload::ClaimInvite {
            invite_id: invite_key(1).invite_id(),
            identity: d.identity(),
            proof: wrong.claim_proof(DOC, &d.identity()),
        },
    );
    // A claim of an invite that was never added.
    let unknown_claim = claim(&[&inv], &d, 2);
    let self_rewrap = rewrap(&[&inv], &d, 0, &epoch_key(0), &d, 3);
    let self_invite = invite(&[&inv], &d, 3, 100);
    let keyring = replay(&[
        &g,
        &inv,
        &forged_claim,
        &unknown_claim,
        &self_rewrap,
        &self_invite,
    ]);
    for refused in [&forged_claim, &unknown_claim, &self_rewrap, &self_invite] {
        assert!(!keyring.is_accepted(&hash(refused)));
    }
    assert_eq!(keyring.participants().len(), 1);
    assert_eq!(keyring.wraps_for(&id(&d)), vec![]);
    check_exhaustively(&[
        &g,
        &inv,
        &forged_claim,
        &unknown_claim,
        &self_rewrap,
        &self_invite,
    ]);
}
