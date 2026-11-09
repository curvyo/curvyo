//! Tester acceptance tests for `specs/0097-membership-lifecycle` slice 1 on the keyring
//! (criteria 3 to 6, 17 to 22, 26 to 28), written from the specification and ADR 0041 before
//! the implementation was read: role races, a demoted admin's concurrent acts, mutual demotion,
//! concurrent adds back with different history choices, and the "added after" rule of the
//! return check against keyrings a relay serves cut off.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod support;

use curvyo_crypto_core::IdentityKeys;
use curvyo_crypto_core::keyring::{Entry, EpochWrap, HeldBackKind, Payload, Role};
use support::{
    check_exhaustively, device, entry, epoch, epoch_key, genesis, hash, id, new_epoch, remove,
    replay, wraps,
};

/// `AddMember` of `who` with the wraps of the epochs in `epochs` (`epoch_key(n)` is epoch n).
fn add(
    parents: &[&Entry],
    admin: &IdentityKeys,
    who: &IdentityKeys,
    role: Role,
    epochs: &[u8],
) -> Entry {
    entry(
        parents,
        admin,
        Payload::AddMember {
            identity: who.identity(),
            role,
            wraps: epochs
                .iter()
                .map(|&n| EpochWrap {
                    epoch: u32::from(n),
                    wrap: wraps(&epoch_key(n), u32::from(n), &[who], 0x61 ^ n)[0].wrap,
                })
                .collect(),
        },
    )
}

fn set_role(parents: &[&Entry], admin: &IdentityKeys, who: &IdentityKeys, role: Role) -> Entry {
    entry(
        parents,
        admin,
        Payload::SetRole {
            member: id(who),
            role,
        },
    )
}

/// A (admin), C (admin), B (member) in epoch 0: genesis, add C, add B.
fn room() -> (Vec<Entry>, [IdentityKeys; 3]) {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let add_c = add(&[&g], &a, &c, Role::Admin, &[0]);
    let add_b = add(&[&add_c], &a, &b, Role::Member, &[0]);
    (vec![g, add_c, add_b], [a, b, c])
}

/// Criterion 6 (first half): A promotes B while C removes B; deny wins, B is out, in every
/// order and every ancestor-closed subset.
#[test]
fn promote_concurrent_with_removal_leaves_b_out() {
    let (base, [a, b, c]) = room();
    let head = base.last().unwrap();
    let promote = set_role(&[head], &a, &b, Role::Admin);
    let removal = remove(&[head], &c, &b);
    let all: Vec<&Entry> = base.iter().chain([&promote, &removal]).collect();
    check_exhaustively(&all);
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&b)), None);
    assert!(keyring.was_removed(&id(&b)));
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(keyring.role(&id(&c)), Some(Role::Admin));
    assert!(keyring.held_back().is_empty(), "nothing to report");
}

/// Criterion 6 (second half): A promotes B while C demotes B; the lower role wins.
#[test]
fn promote_concurrent_with_demotion_leaves_b_a_member() {
    let (base, [a, b, c]) = room();
    let head = base.last().unwrap();
    // B is an admin first, so C's demotion is a real demotion and A's promotion a no-op twin.
    let first = set_role(&[head], &a, &b, Role::Admin);
    let promote_again = set_role(&[&first], &a, &b, Role::Admin);
    let demote = set_role(&[&first], &c, &b, Role::Member);
    let all: Vec<&Entry> = base
        .iter()
        .chain([&first, &promote_again, &demote])
        .collect();
    check_exhaustively(&all);
    assert_eq!(replay(&all).role(&id(&b)), Some(Role::Member));
    // And from member: promote ∥ demote-to-member.
    let promote = set_role(&[head], &a, &b, Role::Admin);
    let keep = set_role(&[head], &c, &b, Role::Member);
    let all: Vec<&Entry> = base.iter().chain([&promote, &keep]).collect();
    check_exhaustively(&all);
    assert_eq!(replay(&all).role(&id(&b)), Some(Role::Member));
}

/// Criterion 4 / Frage 148 with ADR 0010 §5 (validity depends on an entry's ancestors alone):
/// A demotes C while C, still an admin in what it knew, removes B. The merged state is
/// deterministic, C ends as member, and the removal (a denial) applies: security fails closed.
#[test]
fn a_demoted_admins_concurrent_removal_still_applies() {
    let (base, [a, b, c]) = room();
    let head = base.last().unwrap();
    let demote = set_role(&[head], &a, &c, Role::Member);
    let removal = remove(&[head], &c, &b);
    let all: Vec<&Entry> = base.iter().chain([&demote, &removal]).collect();
    check_exhaustively(&all);
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&c)), Some(Role::Member));
    assert_eq!(keyring.role(&id(&b)), None, "deny wins");
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    // A rotation C writes after its own demotion is known to it is rejected.
    let late = new_epoch(&[&demote, &removal], &c, 1, &epoch_key(1), &[&a, &c]);
    let all: Vec<&Entry> = base.iter().chain([&demote, &removal, &late]).collect();
    let keyring = replay(&all);
    assert!(!keyring.is_accepted(&hash(&late)));
    assert_eq!(keyring.current_epoch().number, 0);
}

/// Criterion 27 for demotions: two admins demote each other while apart; both stay admins and
/// `held_back()` names both demotions with kind `Demotion`.
#[test]
fn a_mutual_demotion_is_held_back_as_two_demotions() {
    let (base, [a, _b, c]) = room();
    let head = base.last().unwrap();
    let a_demotes_c = set_role(&[head], &a, &c, Role::Member);
    let c_demotes_a = set_role(&[head], &c, &a, Role::Member);
    let all: Vec<&Entry> = base.iter().chain([&a_demotes_c, &c_demotes_a]).collect();
    check_exhaustively(&all);
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(keyring.role(&id(&c)), Some(Role::Admin));
    let held = keyring.held_back();
    assert_eq!(held.len(), 2, "{held:?}");
    assert!(held.iter().all(|h| h.kind == HeldBackKind::Demotion));
    let pairs: Vec<_> = held.iter().map(|h| (h.author, h.subject)).collect();
    assert!(pairs.contains(&(id(&a), id(&c))));
    assert!(pairs.contains(&(id(&c), id(&a))));
}

/// Criterion 27/28 mixed: A removes C while C demotes A. Both are held back (the admin set
/// would be empty), with their own kinds.
#[test]
fn a_removal_crossed_with_a_demotion_is_held_back_with_both_kinds() {
    let (base, [a, _b, c]) = room();
    let head = base.last().unwrap();
    let a_removes_c = remove(&[head], &a, &c);
    let c_demotes_a = set_role(&[head], &c, &a, Role::Member);
    let all: Vec<&Entry> = base.iter().chain([&a_removes_c, &c_demotes_a]).collect();
    check_exhaustively(&all);
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(keyring.role(&id(&c)), Some(Role::Admin));
    let mut kinds: Vec<_> = keyring.held_back().iter().map(|h| h.kind).collect();
    kinds.sort();
    assert_eq!(kinds, vec![HeldBackKind::Removal, HeldBackKind::Demotion]);
}

/// B removed in epoch 0; A rotates to 1 and to 2 while B is away (C holds every epoch).
fn away() -> (Vec<Entry>, [IdentityKeys; 3]) {
    let (mut log, [a, b, c]) = room();
    let removal = remove(&[log.last().unwrap()], &a, &b);
    let e1 = new_epoch(&[&removal], &a, 1, &epoch_key(1), &[&a, &c]);
    let e2 = new_epoch(&[&e1], &a, 2, &epoch_key(2), &[&a, &c]);
    log.extend([removal, e1, e2]);
    (log, [a, b, c])
}

/// Criterion 22: A adds B back with the current epoch only while C adds B back with every
/// epoch. B ends up holding epoch 1 in every order, and no repair is owed.
#[test]
fn a_concurrent_full_history_add_back_wins_over_from_now_on() {
    let (log, [a, b, c]) = away();
    let head = log.last().unwrap();
    let from_now = add(&[head], &a, &b, Role::Member, &[2]);
    let full = add(&[head], &c, &b, Role::Member, &[0, 1, 2]);
    let all: Vec<&Entry> = log.iter().chain([&from_now, &full]).collect();
    check_exhaustively(&all);
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&b)), Some(Role::Member));
    for n in 0..=2u8 {
        assert!(
            keyring.holds(&id(&b), epoch(u32::from(n), &epoch_key(n))),
            "B holds epoch {n}"
        );
    }
    assert_eq!(keyring.missing_wraps(&id(&c), 0), Vec::new());
    assert_eq!(keyring.missing_wraps(&id(&a), 0), Vec::new());
}

/// ADR 0044 decision 4 (no epoch is withheld any more): D, a newer admin, holds only epoch 2
/// and adds B back with it; epoch 1, which only A and C hold, is owed to B by both of them, and
/// once C repairs it B holds every epoch.
#[test]
fn a_returning_member_gets_an_epoch_only_a_third_member_held() {
    let (log, [a, b, c]) = away();
    let d = device(4);
    let add_d = add(&[log.last().unwrap()], &a, &d, Role::Admin, &[2]);
    let back = add(&[&add_d], &d, &b, Role::Member, &[2]);
    let all: Vec<&Entry> = log.iter().chain([&add_d, &back]).collect();
    let keyring = replay(&all);
    assert!(!keyring.holds(&id(&d), epoch(1, &epoch_key(1))));
    for writer in [&a, &c] {
        let owed: Vec<u32> = keyring
            .missing_wraps(&id(writer), 0)
            .iter()
            .filter(|w| w.recipient.member == id(&b))
            .map(|w| w.epoch.number)
            .collect();
        assert_eq!(owed, vec![1], "epoch 1 is repaired for B");
    }
    let repair = support::rewrap(&[&back], &c, 1, &epoch_key(1), &b, 0x44);
    let all: Vec<&Entry> = log.iter().chain([&add_d, &back, &repair]).collect();
    let keyring = replay(&all);
    for n in 0..=2u8 {
        assert!(
            keyring.holds(&id(&b), epoch(u32::from(n), &epoch_key(n))),
            "B holds epoch {n}"
        );
    }
}

/// Criterion 17: former participants are the removed and the left, and leave the list when
/// added back; a member who was never removed is not in it.
#[test]
fn former_participants_follow_removal_and_add_back() {
    let (log, [a, b, c]) = away();
    let all: Vec<&Entry> = log.iter().collect();
    let keyring = replay(&all);
    let former: Vec<_> = keyring
        .former_participants()
        .iter()
        .map(|i| i.member)
        .collect();
    assert_eq!(former, vec![id(&b)]);
    assert_eq!(keyring.former_participants()[0], b.identity());
    let c_leaves = remove(&[log.last().unwrap()], &c, &c);
    let back = add(&[&c_leaves], &a, &b, Role::Member, &[2]);
    let all: Vec<&Entry> = log.iter().chain([&c_leaves, &back]).collect();
    let keyring = replay(&all);
    let former: Vec<_> = keyring
        .former_participants()
        .iter()
        .map(|i| i.member)
        .collect();
    assert_eq!(former, vec![id(&c)]);
    assert_eq!(
        keyring.rotation_pending(),
        vec![id(&c)],
        "C still holds epoch 2"
    );
}
