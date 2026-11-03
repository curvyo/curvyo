//! Removal, rotation and the fork rules of the keyring (`specs/0092` criteria 32 to 34;
//! ADR 0010 §5, §8, §9; ADR 0031 decision 6a). Each scenario is also checked exhaustively
//! (`support::check_exhaustively`).

#![allow(clippy::unwrap_used, clippy::too_many_lines)]

mod support;

use curvyo_crypto_core::keyring::{Entry, Payload, Role};
use curvyo_crypto_core::{EpochKey, IdentityKeys, unwrap_epoch_key};
use support::{
    DOC, check_exhaustively, claim, device, entry, epoch, epoch_key, genesis, hash, id, invite,
    new_epoch, remove, replay, rewrap, wraps,
};

/// Criteria 32 to 34: removal plus rotation; the removed member keeps the old key only.
#[test]
fn removal_with_rotation_shuts_the_removed_member_out_of_the_new_epoch() {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let inv_b = invite(&[&g], &a, 1, 100);
    let cl_b = claim(&[&inv_b], &b, 1);
    let rw_b = rewrap(&[&cl_b], &a, 0, &epoch_key(0), &b, 1);
    let inv_c = invite(&[&rw_b], &a, 2, 100);
    let cl_c = claim(&[&inv_c], &c, 2);
    let rw_c = rewrap(&[&cl_c], &a, 0, &epoch_key(0), &c, 2);
    let rm = remove(&[&rw_c], &a, &b);
    let ne = new_epoch(&[&rm], &a, 1, &epoch_key(1), &[&a, &c]);
    let all = [&g, &inv_b, &cl_b, &rw_b, &inv_c, &cl_c, &rw_c, &rm, &ne];
    let keyring = replay(&all);
    assert_eq!(keyring.rejected(), vec![]);
    assert_eq!(keyring.role(&id(&b)), None);
    assert!(keyring.was_removed(&id(&b)));
    let new = epoch(1, &epoch_key(1));
    assert_eq!(keyring.current_epoch(), new);
    assert_eq!(keyring.highest_epoch_number(), 1);
    assert!(!keyring.holds(&id(&b), new));
    assert!(keyring.holds(&id(&c), new));
    // Records under epoch 0 stay readable to everyone who held it, B included (0010 §9).
    assert!(keyring.holds(&id(&b), epoch(0, &epoch_key(0))));
    let c_keys: Vec<EpochKey> = keyring
        .wraps_for(&id(&c))
        .iter()
        .map(|(e, w)| unwrap_epoch_key(w, e.number, DOC, &c).unwrap())
        .collect();
    assert!(c_keys.contains(&epoch_key(1)));
    assert_eq!(keyring.missing_wraps(&id(&a), 0), vec![]);
    assert_eq!(
        keyring.missing_wraps(&id(&b), 0),
        vec![],
        "a removed member gives nothing"
    );

    // After the removal: B's rewrap is refused (0031 6a), and so is anything else B writes.
    let b_rewrap = rewrap(&[&ne], &b, 0, &epoch_key(0), &c, 4);
    let b_invite = invite(&[&ne], &b, 5, 100);
    // Rotations that are not authorized or not complete are refused.
    let wraps_removed = new_epoch(&[&ne], &a, 2, &epoch_key(2), &[&a, &b, &c]);
    let misses_c = new_epoch(&[&ne], &a, 2, &epoch_key(3), &[&a]);
    let by_member = new_epoch(&[&ne], &c, 2, &epoch_key(4), &[&a, &c]);
    let skips_a_number = new_epoch(&[&ne], &a, 3, &epoch_key(5), &[&a, &c]);
    let reused_key = new_epoch(&[&ne], &a, 2, &epoch_key(1), &[&a, &c]);
    let later = replay(&[
        &g,
        &inv_b,
        &cl_b,
        &rw_b,
        &inv_c,
        &cl_c,
        &rw_c,
        &rm,
        &ne,
        &b_rewrap,
        &b_invite,
        &wraps_removed,
        &misses_c,
        &by_member,
        &skips_a_number,
        &reused_key,
    ]);
    for refused in [
        &b_rewrap,
        &b_invite,
        &wraps_removed,
        &misses_c,
        &by_member,
        &skips_a_number,
        &reused_key,
    ] {
        assert!(!later.is_accepted(&hash(refused)));
    }
    assert_eq!(later.current_epoch(), new);
    check_exhaustively(&all);
}

/// ADR 0031 decision 6a: a join concurrent with a removal is repaired by `RewrapEpoch` of the
/// new epoch; the removed member's concurrent rewrap of the old epoch is harmless.
#[test]
fn a_claim_concurrent_with_a_removal_is_repaired_by_a_rewrap() {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let inv_b = invite(&[&g], &a, 1, 100);
    let cl_b = claim(&[&inv_b], &b, 1);
    let rw_b = rewrap(&[&cl_b], &a, 0, &epoch_key(0), &b, 1);
    let inv_c = invite(&[&rw_b], &a, 2, 100);
    // A removes B and rotates, not yet knowing C's claim.
    let rm = remove(&[&inv_c], &a, &b);
    let ne = new_epoch(&[&rm], &a, 1, &epoch_key(1), &[&a]);
    // Concurrently C claims and B (still a member in its view) completes the join.
    let cl_c = claim(&[&inv_c], &c, 2);
    let b_completes = rewrap(&[&cl_c], &b, 0, &epoch_key(0), &c, 2);
    let merged = [
        &g,
        &inv_b,
        &cl_b,
        &rw_b,
        &inv_c,
        &rm,
        &ne,
        &cl_c,
        &b_completes,
    ];
    let keyring = replay(&merged);
    assert_eq!(keyring.rejected(), vec![]);
    assert_eq!(keyring.role(&id(&b)), None);
    assert_eq!(keyring.role(&id(&c)), Some(Role::Member));
    let new = epoch(1, &epoch_key(1));
    assert_eq!(keyring.current_epoch(), new);
    let missing = keyring.missing_wraps(&id(&a), 50);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].epoch, new);
    assert_eq!(missing[0].recipient, c.identity());
    check_exhaustively(&merged);

    let heads: Vec<&Entry> = [&ne, &b_completes].into_iter().collect();
    let repair = rewrap(&heads, &a, 1, &epoch_key(1), &c, 3);
    let repaired = replay(&[
        &g,
        &inv_b,
        &cl_b,
        &rw_b,
        &inv_c,
        &rm,
        &ne,
        &cl_c,
        &b_completes,
        &repair,
    ]);
    assert!(repaired.is_accepted(&hash(&repair)));
    assert!(repaired.holds(&id(&c), new));
    assert_eq!(repaired.missing_wraps(&id(&a), 50), vec![]);
}

/// Two admins, A and B, and a member C.
fn two_admins_and_a_member() -> (Vec<Entry>, [IdentityKeys; 3]) {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let add_b = entry(
        &[&g],
        &a,
        Payload::AddMember {
            identity: b.identity(),
            role: Role::Admin,
            wraps: vec![curvyo_crypto_core::keyring::EpochWrap {
                epoch: 0,
                wrap: wraps(&epoch_key(0), 0, &[&b], 7)[0].wrap,
            }],
        },
    );
    let add_c = entry(
        &[&add_b],
        &b,
        Payload::AddMember {
            identity: c.identity(),
            role: Role::Member,
            wraps: vec![curvyo_crypto_core::keyring::EpochWrap {
                epoch: 0,
                wrap: wraps(&epoch_key(0), 0, &[&c], 8)[0].wrap,
            }],
        },
    );
    (vec![g, add_b, add_c], [a, b, c])
}

/// ADR 0010 §5: two concurrent rotations both create epoch 1 with distinct `key_id`s; the lower
/// entry hash wins; both keys stay readable.
#[test]
fn concurrent_rotations_keep_both_keys_and_pick_one() {
    let (base, [a, b, c]) = two_admins_and_a_member();
    let head = &base[2];
    let by_a = new_epoch(&[head], &a, 1, &epoch_key(1), &[&a, &b, &c]);
    let by_b = new_epoch(&[head], &b, 1, &epoch_key(2), &[&a, &b, &c]);
    let all: Vec<&Entry> = base.iter().chain([&by_a, &by_b]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.rejected(), vec![]);
    let (ka, kb) = (epoch(1, &epoch_key(1)), epoch(1, &epoch_key(2)));
    assert_ne!(ka.key_id, kb.key_id);
    assert!(keyring.epochs().contains(&ka) && keyring.epochs().contains(&kb));
    let expected = if hash(&by_a) < hash(&by_b) { ka } else { kb };
    assert_eq!(keyring.current_epoch(), expected);
    assert!(keyring.holds(&id(&c), ka) && keyring.holds(&id(&c), kb));
    assert_eq!(keyring.missing_wraps(&id(&a), 0), vec![]);
    check_exhaustively(&all);

    // The next rotation, seeing both, is epoch 2.
    let next = new_epoch(&[&by_a, &by_b], &a, 2, &epoch_key(3), &[&a, &b, &c]);
    let mut more = all.clone();
    more.push(&next);
    assert!(replay(&more).is_accepted(&hash(&next)));
    assert_eq!(replay(&more).current_epoch(), epoch(2, &epoch_key(3)));
}

/// ADR 0010 §5, deny wins: a removal concurrent with a promotion leaves the member out.
#[test]
fn a_removal_beats_a_concurrent_promotion() {
    let (base, [a, b, c]) = two_admins_and_a_member();
    let head = &base[2];
    let removal = remove(&[head], &a, &c);
    let promotion = entry(
        &[head],
        &b,
        Payload::SetRole {
            member: id(&c),
            role: Role::Admin,
        },
    );
    let all: Vec<&Entry> = base.iter().chain([&removal, &promotion]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.rejected(), vec![]);
    assert_eq!(keyring.role(&id(&c)), None);
    check_exhaustively(&all);

    // Lower role wins: a concurrent demotion and promotion leave a member.
    let demote = entry(
        &[head],
        &a,
        Payload::SetRole {
            member: id(&b),
            role: Role::Member,
        },
    );
    let keep = entry(
        &[head],
        &b,
        Payload::SetRole {
            member: id(&b),
            role: Role::Admin,
        },
    );
    let roles: Vec<&Entry> = base.iter().chain([&demote, &keep]).collect();
    assert_eq!(replay(&roles).role(&id(&b)), Some(Role::Member));
    check_exhaustively(&roles);
}

/// ADR 0010 §5: two admins removing each other concurrently both stay admins.
#[test]
fn the_admin_set_never_becomes_empty() {
    let (base, [a, b, _c]) = two_admins_and_a_member();
    let head = &base[2];
    let a_removes_b = remove(&[head], &a, &b);
    let b_removes_a = remove(&[head], &b, &a);
    let all: Vec<&Entry> = base.iter().chain([&a_removes_b, &b_removes_a]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(keyring.role(&id(&b)), Some(Role::Admin));
    assert!(keyring.admin_set_restored());
    assert!(!replay(&base.iter().collect::<Vec<_>>()).admin_set_restored());
    check_exhaustively(&all);
}

/// (Proposal) Deny wins over the epoch tiebreak: a rotation concurrent with a removal that
/// still wraps the removed member never becomes the current epoch.
#[test]
fn a_rotation_wrapping_a_concurrently_removed_member_does_not_win() {
    let (base, [a, b, c]) = two_admins_and_a_member();
    let head = &base[2];
    let rm = remove(&[head], &a, &c);
    let clean = new_epoch(&[&rm], &a, 1, &epoch_key(1), &[&a, &b]);
    let leaky = new_epoch(&[head], &b, 1, &epoch_key(2), &[&a, &b, &c]);
    let leakier = new_epoch(&[&leaky], &b, 2, &epoch_key(3), &[&a, &b, &c]);
    let all: Vec<&Entry> = base.iter().chain([&rm, &clean, &leaky, &leakier]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.rejected(), vec![]);
    assert_eq!(keyring.current_epoch(), epoch(1, &epoch_key(1)));
    assert!(!keyring.holds(&id(&c), keyring.current_epoch()));
    check_exhaustively(&all);
}

#[test]
fn entries_with_a_bad_signature_or_a_missing_parent_are_not_accepted() {
    let (a, b) = (device(1), device(2));
    let g = genesis(&a);
    let mut forged = invite(&[&g], &a, 1, 100);
    forged.signature[5] ^= 0x10;
    let orphan = Entry::sign(
        vec![curvyo_crypto_core::keyring::EntryHash([7; 32])],
        &a,
        Payload::RemoveMember { member: id(&b) },
    )
    .unwrap();
    let child_of_forged = claim(&[&forged], &b, 1);
    let second_root = Entry::sign(
        Vec::new(),
        &b,
        Payload::Genesis {
            document_id: DOC,
            admins: vec![b.identity()],
            epoch0_wraps: wraps(&epoch_key(9), 0, &[&b], 0),
        },
    )
    .unwrap();
    let all = [&g, &forged, &orphan, &child_of_forged, &second_root];
    let keyring = replay(&all);
    let winner = if hash(&g) < hash(&second_root) {
        &g
    } else {
        &second_root
    };
    for e in all {
        assert_eq!(keyring.is_accepted(&hash(e)), std::ptr::eq(e, winner));
    }
    assert_eq!(keyring.rejected().len(), 4);
}
