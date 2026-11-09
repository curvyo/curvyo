//! The keyring facts the membership lifecycle reads (`specs/0097` criteria 5, 11, 12, 14, 17,
//! 18, 22, 27 and 28; ADR 0041 decisions 1, 2, 4 and 5; ADR 0044 decision 4): held-back
//! denials, a pending rotation, former participants and the repair of every epoch a member
//! lacks.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

mod support;

use curvyo_crypto_core::IdentityKeys;
use curvyo_crypto_core::keyring::{
    Entry, EpochWrap, HeldBack, HeldBackKind, MissingWrap, Payload, Role,
};
use support::{
    check_exhaustively, claim, device, entry, epoch, epoch_key, genesis, hash, id, identity,
    invite, new_epoch, remove, replay, wraps,
};

fn add(parents: &[&Entry], admin: &IdentityKeys, who: &IdentityKeys, role: Role) -> Entry {
    add_with(parents, admin, who, role, &[0])
}

/// `AddMember` of `who` with wraps of the epochs numbered in `epochs` (`epoch_key(n)` is the
/// key of epoch `n` in these tests).
fn add_with(
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
                    wrap: wraps(&epoch_key(n), u32::from(n), &[who], 0x30 ^ n)[0].wrap,
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

fn held(
    entry: &Entry,
    author: &IdentityKeys,
    subject: &IdentityKeys,
    kind: HeldBackKind,
) -> HeldBack {
    HeldBack {
        entry: hash(entry),
        author: id(author),
        subject: id(subject),
        kind,
    }
}

/// Criteria 5 and 14, ADR 0041 decision 1: the linear case. The only admin removes itself
/// while a member exists; replay drops the removal and names it.
#[test]
fn the_only_admin_removing_itself_is_held_back() {
    let (a, c) = (device(1), device(3));
    let g = genesis(&a);
    let add_c = add(&[&g], &a, &c, Role::Member);
    let leave = remove(&[&add_c], &a, &a);
    let all = [&g, &add_c, &leave];
    let keyring = replay(&all);
    assert!(keyring.is_accepted(&hash(&leave)));
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(
        keyring.held_back(),
        vec![held(&leave, &a, &a, HeldBackKind::Removal)]
    );
    assert!(keyring.admin_set_restored());
    check_exhaustively(&all);
}

/// Criterion 5: the only admin stepping down is held back as a demotion.
#[test]
fn the_only_admin_stepping_down_is_held_back() {
    let (a, c) = (device(1), device(3));
    let g = genesis(&a);
    let add_c = add(&[&g], &a, &c, Role::Member);
    let step_down = set_role(&[&add_c], &a, &a, Role::Member);
    let keyring = replay(&[&g, &add_c, &step_down]);
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(
        keyring.held_back(),
        vec![held(&step_down, &a, &a, HeldBackKind::Demotion)]
    );
}

fn two_admins_and_a_member() -> (Vec<Entry>, [IdentityKeys; 3]) {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let add_b = add(&[&g], &a, &b, Role::Admin);
    let add_c = add(&[&add_b], &a, &c, Role::Member);
    (vec![g, add_b, add_c], [a, b, c])
}

/// Criteria 27 and 28: two admins remove each other; both denials are held back, with authors
/// and subjects crossed, in causal order. A removal that applies is not held back.
#[test]
fn a_mutual_removal_is_two_crossed_held_back_entries() {
    let (base, [a, b, c]) = two_admins_and_a_member();
    let head = &base[2];
    let a_removes_b = remove(&[head], &a, &b);
    let b_removes_a = remove(&[head], &b, &a);
    let all: Vec<&Entry> = base.iter().chain([&a_removes_b, &b_removes_a]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&a)), Some(Role::Admin));
    assert_eq!(keyring.role(&id(&b)), Some(Role::Admin));
    let mut expected = vec![
        held(&a_removes_b, &a, &b, HeldBackKind::Removal),
        held(&b_removes_a, &b, &a, HeldBackKind::Removal),
    ];
    // Causal order breaks the tie between concurrent entries by the lower hash.
    expected.sort_by_key(|h| h.entry);
    assert_eq!(keyring.held_back(), expected);
    check_exhaustively(&all);

    let applied: Vec<&Entry> = base.iter().chain([&a_removes_b]).collect();
    let keyring = replay(&applied);
    assert_eq!(keyring.role(&id(&b)), None);
    assert_eq!(keyring.held_back(), vec![]);
    assert!(!keyring.admin_set_restored());
    assert_eq!(keyring.role(&id(&c)), Some(Role::Member));
}

/// Criterion 28: making a third admin releases the held-back removals; both old admins go.
#[test]
fn a_third_admin_releases_the_held_back_removals() {
    let (base, [a, b, c]) = two_admins_and_a_member();
    let head = &base[2];
    let a_removes_b = remove(&[head], &a, &b);
    let b_removes_a = remove(&[head], &b, &a);
    let promote_c = set_role(&[&a_removes_b, &b_removes_a], &a, &c, Role::Admin);
    let all: Vec<&Entry> = base
        .iter()
        .chain([&a_removes_b, &b_removes_a, &promote_c])
        .collect();
    let keyring = replay(&all);
    assert!(keyring.is_accepted(&hash(&promote_c)));
    assert_eq!(keyring.role(&id(&a)), None);
    assert_eq!(keyring.role(&id(&b)), None);
    assert_eq!(keyring.role(&id(&c)), Some(Role::Admin));
    assert_eq!(keyring.held_back(), vec![]);
}

/// Criteria 11 and 12, ADR 0041 decision 2: a leaver who still holds the current epoch makes
/// a rotation pending until an admin rotates; a removal with rotation never shows it.
#[test]
fn a_leave_makes_a_rotation_pending_until_an_admin_rotates() {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let add_b = add(&[&g], &a, &b, Role::Member);
    let add_c = add(&[&add_b], &a, &c, Role::Member);
    let b_leaves = remove(&[&add_c], &b, &b);
    let before = replay(&[&g, &add_b, &add_c]);
    assert_eq!(before.rotation_pending(), vec![]);
    let left = replay(&[&g, &add_b, &add_c, &b_leaves]);
    assert_eq!(left.rotation_pending(), vec![id(&b)]);

    let rotation = new_epoch(&[&b_leaves], &a, 1, &epoch_key(1), &[&a, &c]);
    let rotated = replay(&[&g, &add_b, &add_c, &b_leaves, &rotation]);
    assert_eq!(rotated.rotation_pending(), vec![]);

    // Removed by an admin without a rotation: pending; with it: never.
    let a_removes_c = remove(&[&add_c], &a, &c);
    let mut pending = vec![id(&c)];
    assert_eq!(
        replay(&[&g, &add_b, &add_c, &a_removes_c]).rotation_pending(),
        pending
    );
    let both = replay(&[&g, &add_b, &add_c, &a_removes_c, &b_leaves]);
    pending.push(id(&b));
    pending.sort();
    assert_eq!(both.rotation_pending(), pending);
    let with_rotation = new_epoch(&[&a_removes_c], &a, 1, &epoch_key(1), &[&a, &b]);
    assert_eq!(
        replay(&[&g, &add_b, &add_c, &a_removes_c, &with_rotation]).rotation_pending(),
        vec![]
    );
}

/// Criteria 17 and 24, ADR 0041 decision 5: everyone ever added and no longer a participant,
/// with both public keys; a re-added identity is a participant again.
#[test]
fn former_participants_are_the_removed_and_the_left() {
    let (a, b, c, d) = (device(1), device(2), device(3), device(4));
    let g = genesis(&a);
    let add_b = add(&[&g], &a, &b, Role::Member);
    let add_c = add(&[&add_b], &a, &c, Role::Member);
    let b_leaves = remove(&[&add_c], &b, &b);
    let a_removes_c = remove(&[&b_leaves], &a, &c);
    let keyring = replay(&[&g, &add_b, &add_c, &b_leaves, &a_removes_c]);
    let mut expected = vec![identity(&b), identity(&c)];
    expected.sort_by_key(|i| i.member);
    assert_eq!(keyring.former_participants(), expected);
    assert!(!keyring.former_participants().contains(&identity(&d)));

    let back = add(&[&a_removes_c], &a, &b, Role::Member);
    let keyring = replay(&[&g, &add_b, &add_c, &b_leaves, &a_removes_c, &back]);
    assert_eq!(keyring.former_participants(), vec![identity(&c)]);
}

/// A, D admins and C a member; B was a member, was removed, then two rotations (epochs 1, 2)
/// happened while B was away.
struct Away {
    a: IdentityKeys,
    b: IdentityKeys,
    c: IdentityKeys,
    d: IdentityKeys,
    entries: Vec<Entry>,
}

fn while_b_was_away() -> Away {
    let (a, b, c, d) = (device(1), device(2), device(3), device(4));
    let g = genesis(&a);
    let add_d = add(&[&g], &a, &d, Role::Admin);
    let add_b = add(&[&add_d], &a, &b, Role::Member);
    let add_c = add(&[&add_b], &a, &c, Role::Member);
    let rm_b = remove(&[&add_c], &a, &b);
    let e1 = new_epoch(&[&rm_b], &a, 1, &epoch_key(1), &[&a, &c, &d]);
    let e2 = new_epoch(&[&e1], &a, 2, &epoch_key(2), &[&a, &c, &d]);
    Away {
        a,
        b,
        c,
        d,
        entries: vec![g, add_d, add_b, add_c, rm_b, e1, e2],
    }
}

fn owed_to(
    keyring: &curvyo_crypto_core::keyring::Keyring,
    by: &IdentityKeys,
    to: &IdentityKeys,
) -> Vec<MissingWrap> {
    keyring
        .missing_wraps(&id(by), 0)
        .into_iter()
        .filter(|m| m.recipient.member == id(to))
        .collect()
}

/// ADR 0044 decision 4: an add back that wraps the current epoch only leaves epoch 1, which
/// began while B was away, to be repaired by every member that holds it; no epoch is withheld.
#[test]
fn an_add_back_with_the_current_epoch_only_is_repaired() {
    let Away {
        a,
        b,
        c,
        d,
        entries,
    } = while_b_was_away();
    let head = entries.last().unwrap();
    let back = add_with(&[head], &a, &b, Role::Member, &[2]);
    let all: Vec<&Entry> = entries.iter().chain([&back]).collect();
    let keyring = replay(&all);
    assert_eq!(keyring.role(&id(&b)), Some(Role::Member));
    assert!(
        keyring.holds(&id(&b), epoch(0, &epoch_key(0))),
        "B keeps what B had"
    );
    assert!(!keyring.holds(&id(&b), epoch(1, &epoch_key(1))));
    for member in [&a, &c, &d] {
        let owed: Vec<u32> = owed_to(&keyring, member, &b)
            .iter()
            .map(|m| m.epoch.number)
            .collect();
        assert_eq!(owed, vec![1], "epoch 1 is repaired");
    }
}

/// A rotation concurrent with the add back is repaired too (ADR 0031 6a).
#[test]
fn a_rotation_concurrent_with_the_add_back_is_still_repaired() {
    let Away {
        a,
        b,
        c,
        d,
        entries,
    } = while_b_was_away();
    let head = entries.last().unwrap();
    let back = add_with(&[head], &a, &b, Role::Member, &[2]);
    let e3 = new_epoch(&[head], &d, 3, &epoch_key(3), &[&a, &c, &d]);
    let all: Vec<&Entry> = entries.iter().chain([&back, &e3]).collect();
    let keyring = replay(&all);
    let owed: Vec<u32> = owed_to(&keyring, &d, &b)
        .iter()
        .map(|m| m.epoch.number)
        .collect();
    assert_eq!(owed, vec![1, 3]);
    assert_eq!(owed_to(&keyring, &c, &b).len(), 2, "C holds both too");
}

/// Criterion 23 and COOP-JOIN A: a member who came back by an invite still gets every epoch;
/// a full-history add back leaves nothing to repair.
#[test]
fn an_invite_claim_or_a_full_history_add_gets_every_epoch() {
    let Away {
        a,
        b,
        c,
        d: _,
        entries,
    } = while_b_was_away();
    let head = entries.last().unwrap();
    let inv = invite(&[head], &a, 1, 100);
    let cl = claim(&[&inv], &b, 1);
    let all: Vec<&Entry> = entries.iter().chain([&inv, &cl]).collect();
    let keyring = replay(&all);
    let owed: Vec<u32> = owed_to(&keyring, &c, &b)
        .iter()
        .map(|m| m.epoch.number)
        .collect();
    assert_eq!(owed, vec![1, 2]);

    let full = add_with(&[head], &a, &b, Role::Member, &[1, 2]);
    let all: Vec<&Entry> = entries.iter().chain([&full]).collect();
    let keyring = replay(&all);
    assert!(keyring.holds(&id(&b), epoch(1, &epoch_key(1))));
    assert_eq!(owed_to(&keyring, &c, &b), vec![]);
}
