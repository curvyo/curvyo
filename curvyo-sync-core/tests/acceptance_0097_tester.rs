//! Tester acceptance tests for `specs/0097-membership-lifecycle` slice 1 in the sync engine,
//! written from the specification and ADR 0041 before the implementation was read: the
//! last-admin rule per device and per person, the notices, `reconcile` with a person leave and in
//! a frozen room, roles and mutual removal through the real relay, the return check's traffic,
//! and the timing of the first check right after an add back.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod support;

use std::collections::BTreeMap;

use curvyo_crypto_core::keyring::{
    Entry, EntryHash, EpochWrap, Keyring, Payload, RecipientWrap, Role,
};
use curvyo_crypto_core::{DocumentId, EpochKey, MemberId, wrap_epoch_key};
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{
    CheckOutcome, ClientFrame, ClientSession, DeviceAction, History, MembershipNotice,
    RefusalReason, SessionError, SessionEvent, SessionStart, Target, WorkspaceInput, admin_remains,
    admin_set, membership_notices, reconcile, reconcile_with_left,
};
use support::sim::{Net, shared};
use support::{ROOM, device, member};

// ---------------------------------------------------------------------------------------------
// Hand-built keyrings
// ---------------------------------------------------------------------------------------------

/// A keyring written by hand: entries, the epoch keys by number, a wrap counter.
struct Log {
    doc: DocumentId,
    entries: Vec<Entry>,
    keys: BTreeMap<u32, EpochKey>,
    salt: u8,
}

impl Log {
    fn new(doc: DocumentId, admins: &[u8]) -> Self {
        let mut log = Self {
            doc,
            entries: Vec::new(),
            keys: BTreeMap::new(),
            salt: 1,
        };
        let key = EpochKey::from_bytes([doc.0[0] ^ 0x11; 32]);
        log.keys.insert(0, key);
        let wraps = admins.iter().map(|&n| log.recipient(0, n)).collect();
        let genesis = Entry::sign(
            Vec::new(),
            &device(admins[0]),
            Payload::Genesis {
                document_id: doc,
                admins: admins.iter().map(|&n| device(n).identity()).collect(),
                epoch0_wraps: wraps,
            },
        )
        .unwrap();
        log.entries.push(genesis);
        log
    }

    fn recipient(&mut self, epoch: u32, n: u8) -> RecipientWrap {
        self.salt = self.salt.wrapping_add(1);
        RecipientWrap {
            member: member(n),
            wrap: wrap_epoch_key(
                &self.keys[&epoch],
                epoch,
                self.doc,
                &device(n).identity(),
                [self.salt; 32],
            )
            .unwrap(),
        }
    }

    fn keyring(&self) -> Keyring {
        Keyring::replay(&self.entries).unwrap()
    }

    fn heads(&self) -> Vec<EntryHash> {
        self.keyring().heads()
    }

    /// Signs `payload` by `author` on `parents` (the current heads when `None`).
    fn write(
        &mut self,
        parents: Option<Vec<EntryHash>>,
        author: u8,
        payload: Payload,
    ) -> EntryHash {
        let parents = parents.unwrap_or_else(|| self.heads());
        let entry = Entry::sign(parents, &device(author), payload).unwrap();
        let hash = entry.try_hash().unwrap();
        self.entries.push(entry);
        hash
    }

    /// `author` adds `who` with `role` and every epoch.
    fn add(
        &mut self,
        parents: Option<Vec<EntryHash>>,
        author: u8,
        who: u8,
        role: Role,
    ) -> EntryHash {
        let numbers: Vec<u32> = self.keys.keys().copied().collect();
        let wraps = numbers
            .into_iter()
            .map(|e| EpochWrap {
                epoch: e,
                wrap: self.recipient(e, who).wrap,
            })
            .collect();
        self.write(
            parents,
            author,
            Payload::AddMember {
                identity: device(who).identity(),
                role,
                wraps,
            },
        )
    }

    fn remove(&mut self, parents: Option<Vec<EntryHash>>, author: u8, who: u8) -> EntryHash {
        self.write(
            parents,
            author,
            Payload::RemoveMember {
                member: member(who),
            },
        )
    }

    fn set_role(
        &mut self,
        parents: Option<Vec<EntryHash>>,
        author: u8,
        who: u8,
        role: Role,
    ) -> EntryHash {
        self.write(
            parents,
            author,
            Payload::SetRole {
                member: member(who),
                role,
            },
        )
    }

    /// `author` rotates to the next epoch, wrapped to `to`.
    fn rotate(&mut self, parents: Option<Vec<EntryHash>>, author: u8, to: &[u8]) -> EntryHash {
        let epoch = self.keyring().highest_epoch_number() + 1;
        let key = EpochKey::from_bytes([self.doc.0[0] ^ u8::try_from(epoch).unwrap(); 32]);
        self.keys.insert(epoch, key);
        let wraps = to.iter().map(|&n| self.recipient(epoch, n)).collect();
        self.write(parents, author, Payload::NewEpoch { epoch, wraps })
    }
}

const GROUP: DocumentId = DocumentId([0x6e; 16]);
const WS: DocumentId = DocumentId([0x77; 16]);
const DOC: DocumentId = DocumentId([0x5d; 16]);

/// Maker A's computers.
const A: u8 = 1;
const A2: u8 = 5;
const A3: u8 = 6;
/// Other makers.
const B: u8 = 2;
const C: u8 = 3;
const D: u8 = 4;

// ---------------------------------------------------------------------------------------------
// Criteria 5, 14, 16, 28: the last-admin rule
// ---------------------------------------------------------------------------------------------

/// Criteria 5 and 28, ADR 0041 decision 1: after A and C removed each other (both held back),
/// neither may step down or leave: the other admin is one only because replay held back its
/// removal, so it is no successor.
#[test]
fn after_a_mutual_removal_neither_admin_counts_as_a_successor() {
    let mut log = Log::new(DOC, &[A]);
    log.add(None, A, C, Role::Admin);
    log.add(None, A, B, Role::Member);
    let base = log.heads();
    log.remove(Some(base.clone()), A, C);
    log.remove(Some(base), C, A);
    let keyring = log.keyring();
    assert_eq!(keyring.held_back().len(), 2);
    assert!(!admin_remains(&keyring, &[member(A)]));
    assert!(!admin_remains(&keyring, &[member(C)]));
    // Acting on member B only leaves the admins as they are.
    assert!(admin_remains(&keyring, &[member(B)]));
}

/// Criterion 5 for a demotion of the only other admin, and the plain counting.
#[test]
fn admin_remains_counts_per_device() {
    let mut log = Log::new(DOC, &[A]);
    log.add(None, A, B, Role::Member);
    let keyring = log.keyring();
    assert!(!admin_remains(&keyring, &[member(A)]));
    assert!(admin_remains(&keyring, &[member(B)]));
    log.add(None, A, C, Role::Admin);
    let keyring = log.keyring();
    assert!(admin_remains(&keyring, &[member(A)]));
    assert!(admin_remains(&keyring, &[member(C)]));
    assert!(!admin_remains(&keyring, &[member(A), member(C)]));
}

/// Criteria 14 and 16 (per person): A's two computers are the only admins of W; the person
/// leave (both devices acted on) is refused, with another maker's admin it is allowed.
#[test]
fn the_person_leave_needs_an_admin_outside_the_makers_devices() {
    let mut w = Log::new(WS, &[A]);
    w.add(None, A, A2, Role::Admin);
    w.add(None, A, B, Role::Member);
    let keyring = w.keyring();
    assert!(
        admin_remains(&keyring, &[member(A)]),
        "per device: A2 remains"
    );
    assert!(!admin_remains(&keyring, &[member(A), member(A2)]));
    w.add(None, A, D, Role::Admin);
    assert!(admin_remains(&w.keyring(), &[member(A), member(A2)]));
}

// ---------------------------------------------------------------------------------------------
// Criteria 7, 8, 11, 27, 28, 30: the notices
// ---------------------------------------------------------------------------------------------

/// Criterion 30: the first look shows no change line; promotion, demotion, an admin's leave
/// and an admin's removal each give their own line.
#[test]
fn admin_set_changes_give_one_line_each() {
    let mut log = Log::new(DOC, &[A]);
    log.add(None, A, B, Role::Member);
    log.add(None, A, C, Role::Admin);
    log.add(None, A, D, Role::Admin);
    let before = admin_set(&log.keyring());
    let first = membership_notices(None, &log.keyring(), &[member(A)]);
    assert!(
        first.iter().all(|n| !matches!(
            n,
            MembershipNotice::NowAdmin(_)
                | MembershipNotice::NoLongerAdmin(_)
                | MembershipNotice::AdminLeft(_)
                | MembershipNotice::AdminRemoved(_)
        )),
        "{first:?}"
    );
    log.set_role(None, A, B, Role::Admin);
    log.set_role(None, A, C, Role::Member);
    log.remove(None, D, D);
    let notices = membership_notices(Some(&before), &log.keyring(), &[member(A)]);
    assert!(
        notices.contains(&MembershipNotice::NowAdmin(member(B))),
        "{notices:?}"
    );
    assert!(notices.contains(&MembershipNotice::NoLongerAdmin(member(C))));
    assert!(notices.contains(&MembershipNotice::AdminLeft(member(D))));
    let before = admin_set(&log.keyring());
    log.remove(None, A, B);
    let notices = membership_notices(Some(&before), &log.keyring(), &[member(A)]);
    assert!(
        notices.contains(&MembershipNotice::AdminRemoved(member(B))),
        "{notices:?}"
    );
    assert!(
        notices.contains(&MembershipNotice::RotationPending {
            member: member(B),
            left: false
        }),
        "{notices:?}"
    );
    assert!(notices.contains(&MembershipNotice::RotationPending {
        member: member(D),
        left: true
    }));
    // The same keyring again gives no change line (the set was stored).
    let same = membership_notices(
        Some(&admin_set(&log.keyring())),
        &log.keyring(),
        &[member(A)],
    );
    assert!(
        same.iter()
            .all(|n| !matches!(n, MembershipNotice::AdminRemoved(_)))
    );
}

/// Criteria 11 and 12: the rotation notice ends with the rotation.
#[test]
fn the_rotation_notice_ends_with_a_new_epoch() {
    let mut log = Log::new(DOC, &[A]);
    log.add(None, A, B, Role::Member);
    log.add(None, A, C, Role::Member);
    log.remove(None, B, B);
    let pending = |log: &Log| {
        membership_notices(None, &log.keyring(), &[member(A)])
            .into_iter()
            .filter(|n| matches!(n, MembershipNotice::RotationPending { .. }))
            .count()
    };
    assert_eq!(pending(&log), 1);
    log.rotate(None, A, &[A, C]);
    assert_eq!(pending(&log), 0);
}

/// Criteria 7 and 8: one admin device gives the second-admin line; two admins that are both
/// the maker's own computers give the own-computers line; another maker's admin ends both.
#[test]
fn the_second_admin_line_counts_devices_and_the_own_group() {
    let mut log = Log::new(WS, &[A]);
    log.add(None, A, B, Role::Member);
    let own = [member(A), member(A2)];
    let notices = membership_notices(None, &log.keyring(), &own);
    assert!(notices.contains(&MembershipNotice::OneAdmin), "{notices:?}");
    log.add(None, A, A2, Role::Admin);
    let notices = membership_notices(None, &log.keyring(), &own);
    assert!(
        notices.contains(&MembershipNotice::OnlyOwnAdmins),
        "{notices:?}"
    );
    assert!(!notices.contains(&MembershipNotice::OneAdmin));
    // Seen from B's computer (its own group is B alone), the line is not "your own computers".
    let notices = membership_notices(None, &log.keyring(), &[member(B)]);
    assert!(
        !notices.contains(&MembershipNotice::OnlyOwnAdmins),
        "{notices:?}"
    );
    log.set_role(None, A, B, Role::Admin);
    let notices = membership_notices(None, &log.keyring(), &own);
    assert!(!notices.contains(&MembershipNotice::OneAdmin));
    assert!(!notices.contains(&MembershipNotice::OnlyOwnAdmins));
}

/// Criteria 27 and 28: a mutual removal gives one notice naming both admins; a later removal
/// between the same two is "not applied", not a second mutual removal.
#[test]
fn a_mutual_removal_is_one_notice_and_a_later_one_is_not_applied() {
    let mut log = Log::new(DOC, &[A]);
    log.add(None, A, C, Role::Admin);
    log.add(None, A, B, Role::Member);
    let base = log.heads();
    let x = log.remove(Some(base.clone()), A, C);
    let y = log.remove(Some(base), C, A);
    let notices = membership_notices(None, &log.keyring(), &[member(A)]);
    let mutual: Vec<_> = notices
        .iter()
        .filter_map(|n| match n {
            MembershipNotice::MutualRemoval { admins, entries } => Some((admins, entries)),
            _ => None,
        })
        .collect();
    assert_eq!(mutual.len(), 1, "{notices:?}");
    let mut want = vec![member(A), member(C)];
    want.sort();
    assert_eq!(mutual[0].0, &want);
    assert!(mutual[0].1.contains(&x) && mutual[0].1.contains(&y));
    // A later demotion of C by A (after both removals) is held back too: "not applied".
    log.set_role(None, A, C, Role::Member);
    let keyring = log.keyring();
    assert_eq!(keyring.role(&member(C)), Some(Role::Admin));
    let notices = membership_notices(None, &keyring, &[member(A)]);
    assert_eq!(
        notices
            .iter()
            .filter(|n| matches!(n, MembershipNotice::MutualRemoval { .. }))
            .count(),
        1,
        "{notices:?}"
    );
    assert!(
        notices
            .iter()
            .any(|n| matches!(n, MembershipNotice::NotApplied(h) if h.subject == member(C))),
        "{notices:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 16 and 28: reconcile with a person leave and in a frozen room
// ---------------------------------------------------------------------------------------------

/// Maker A's group: A and A2 admins (plus `extra`).
fn group(extra: &[u8]) -> Log {
    let mut g = Log::new(GROUP, &[A]);
    g.add(None, A, A2, Role::Admin);
    for &n in extra {
        g.add(None, A, n, Role::Admin);
    }
    g
}

/// W with A (admin), A2 (`a2_role`), D (another maker, `d_role`) and member B.
fn workspace(a2_role: Role, d_role: Role) -> Log {
    let mut w = Log::new(WS, &[A]);
    w.add(None, A, A2, a2_role);
    w.add(None, A, D, d_role);
    w.add(None, A, B, Role::Member);
    w
}

fn plan_for(
    group: &Keyring,
    w: &Keyring,
    on: bool,
    left: bool,
    me: u8,
) -> Option<curvyo_sync_core::Plan> {
    let left = if left { vec![WS] } else { Vec::new() };
    reconcile_with_left(
        group,
        &[WorkspaceInput { on, keyring: w }],
        &left,
        member(me),
    )
    .into_iter()
    .find(|p| p.target == Target::Workspace(WS))
}

fn removals(plan: &curvyo_sync_core::Plan) -> Vec<MemberId> {
    plan.actions
        .iter()
        .flat_map(|a| match a {
            DeviceAction::Remove { members } => members.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn adds(plan: &curvyo_sync_core::Plan) -> Vec<(MemberId, Role)> {
    plan.actions
        .iter()
        .filter_map(|a| match a {
            DeviceAction::Add { identity, role } => Some((identity.member, *role)),
            _ => None,
        })
        .collect()
}

/// Criterion 16: each admin device of the leaving maker removes its siblings and then itself;
/// with another maker's admin, all those concurrent removals apply and none of A's devices is
/// left in W, B stays.
#[test]
fn a_person_leave_removes_every_own_device_and_all_removals_apply() {
    let g = group(&[]);
    let w = workspace(Role::Admin, Role::Admin);
    let gk = g.keyring();
    let wk = w.keyring();
    for (me, sibling) in [(A, A2), (A2, A)] {
        let plan = plan_for(&gk, &wk, false, true, me).expect("a plan");
        assert!(plan.leave, "{plan:?}");
        assert_eq!(removals(&plan), vec![member(sibling)], "{plan:?}");
        assert_eq!(adds(&plan), Vec::new());
    }
    // Both carry their plans out concurrently: sibling removal, rotation, own leave.
    let mut w = w;
    let base = w.heads();
    let r1 = w.remove(Some(base.clone()), A, A2);
    let e1 = w.rotate(Some(vec![r1]), A, &[A, D, B]);
    w.remove(Some(vec![e1]), A, A);
    let r2 = w.remove(Some(base), A2, A);
    let e2 = w.rotate(Some(vec![r2]), A2, &[A2, D, B]);
    w.remove(Some(vec![e2]), A2, A2);
    let keyring = w.keyring();
    assert_eq!(keyring.role(&member(A)), None);
    assert_eq!(keyring.role(&member(A2)), None);
    assert_eq!(keyring.role(&member(D)), Some(Role::Admin));
    assert_eq!(keyring.role(&member(B)), Some(Role::Member));
    assert_eq!(keyring.held_back(), Vec::new());
}

/// Criterion 16: a device of the leaving maker that is only a member of W removes itself only.
#[test]
fn a_member_device_of_the_leaving_maker_only_leaves_itself() {
    let g = group(&[]);
    let w = workspace(Role::Member, Role::Admin);
    let plan = plan_for(&g.keyring(), &w.keyring(), false, true, A2).expect("a plan");
    assert!(plan.leave, "{plan:?}");
    assert!(removals(&plan).is_empty(), "{plan:?}");
    assert_eq!(adds(&plan), Vec::new());
}

/// Criterion 16, the re-add race: A left W; A2 had not read the mark and re-added A (A's last
/// removal was its own leave). Once A2 reads the mark, its plan removes A again and leaves.
#[test]
fn a_sibling_that_re_added_the_leaver_removes_it_again_once_it_reads_the_mark() {
    let g = group(&[]);
    let mut w = workspace(Role::Admin, Role::Admin);
    w.remove(None, A, A);
    let gk = g.keyring();
    // Without the mark, A2's reconcile (W on) would add A back: this is the race.
    let before = plan_for(&gk, &w.keyring(), true, false, A2);
    let raced = before.is_some_and(|p| adds(&p).iter().any(|(m, _)| *m == member(A)));
    assert!(
        raced,
        "a sibling without the mark re-adds a device that left by itself"
    );
    w.add(None, A2, A, Role::Admin);
    let after = plan_for(&gk, &w.keyring(), false, true, A2).expect("a plan");
    assert!(after.leave);
    assert_eq!(adds(&after), Vec::new(), "no add under the mark: {after:?}");
    assert_eq!(removals(&after), vec![member(A)], "{after:?}");
}

/// Criterion 28, ADR 0041 decision 4: in W with held-back denials (A and D removed each other),
/// `reconcile` adds the maker's newly paired computer A3 as a member, never as an admin; without
/// held-back denials it gets the writer's role.
#[test]
fn a_frozen_room_takes_new_own_devices_as_members_only() {
    let g = group(&[A3]);
    let mut w = Log::new(WS, &[A]);
    w.add(None, A, A2, Role::Admin);
    w.add(None, A, D, Role::Admin);
    let plain = plan_for(&g.keyring(), &w.keyring(), true, false, A).expect("a plan");
    assert!(
        adds(&plain).contains(&(member(A3), Role::Admin)),
        "{plain:?}"
    );
    // A frozen room: A2 is gone (the only own sibling), A and D remove each other.
    let mut w = Log::new(WS, &[A]);
    w.add(None, A, D, Role::Admin);
    w.add(None, A, B, Role::Member);
    let base = w.heads();
    w.remove(Some(base.clone()), A, D);
    w.remove(Some(base), D, A);
    let wk = w.keyring();
    assert_ne!(wk.held_back(), Vec::new());
    let plan = plan_for(&g.keyring(), &wk, true, false, A).expect("a plan");
    let added = adds(&plan);
    assert_ne!(added, Vec::new(), "{plan:?}");
    assert!(
        added.iter().all(|(_, role)| *role == Role::Member),
        "{plan:?}"
    );
    // And the plain `reconcile` agrees.
    let plans = reconcile(
        &g.keyring(),
        &[WorkspaceInput {
            on: true,
            keyring: &wk,
        }],
        member(A),
    );
    for plan in plans {
        assert!(adds(&plan).iter().all(|(_, role)| *role == Role::Member));
    }
}

// ---------------------------------------------------------------------------------------------
// Through the simulator: roles, mutual removal, the return check
// ---------------------------------------------------------------------------------------------

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// A (device 1, admin), B (2), C (3) members, converged.
fn three(seed: u64) -> (Net, [usize; 3]) {
    let doc = Document::new(100);
    rect(&doc, 0.0);
    let (net, c) = shared(seed, ROOM, doc, &[B, C]);
    (net, [c[0], c[1], c[2]])
}

fn session(net: &mut Net, i: usize) -> &mut ClientSession {
    net.clients[i].session.as_mut().unwrap()
}

fn act(net: &mut Net, i: usize, f: impl FnOnce(&mut ClientSession) -> Result<(), SessionError>) {
    f(session(net, i)).unwrap();
    net.pump(i);
}

fn role_seen_by(net: &Net, i: usize, who: u8) -> Option<Role> {
    net.clients[i]
        .session()
        .keyring()
        .unwrap()
        .role(&member(who))
}

fn stored_keyring(net: &Net) -> Keyring {
    let entries: Vec<Entry> = net.store[&ROOM]
        .keyring
        .iter()
        .map(|e| Entry::decode(e).unwrap())
        .collect();
    Keyring::replay(&entries).unwrap()
}

fn stored_has(net: &Net, wanted: impl Fn(&Payload, &MemberId) -> bool) -> bool {
    net.store.get(&ROOM).is_some_and(|room| {
        room.keyring.iter().any(|e| {
            let entry = Entry::decode(e).unwrap();
            wanted(&entry.payload, &entry.author)
        })
    })
}

/// Criterion 3: the promoted B removes and rotates, and its connection and C's go on.
#[test]
fn a_promoted_member_removes_and_rotates() {
    let (mut net, [a, b, c]) = three(31);
    act(&mut net, a, |s| s.set_role(member(B), Role::Admin));
    net.run(1_000);
    assert_eq!(role_seen_by(&net, c, B), Some(Role::Admin));
    let connects = net.clients[b].connect_times.len();
    act(&mut net, b, |s| s.remove(member(C)));
    net.run(1_000);
    assert!(net.clients[c].saw(&SessionEvent::Removed));
    assert_eq!(role_seen_by(&net, a, C), None);
    act(&mut net, b, ClientSession::rotate);
    net.run(1_000);
    assert_eq!(
        net.clients[b].connect_times.len(),
        connects,
        "B's connection went on"
    );
    net.edit(a, |d| rect(d, 30.0));
    assert!(net.run_until(3_000, |n| n.converged(&[a, b])).is_some());
}

/// Criterion 4 + ADR 0010 §5 through the relay: A demotes admin B in the same moment B removes
/// C. Whatever the order, B ends a member and C is out (deny wins, fail closed), nobody errs.
#[test]
fn a_demotion_racing_the_demoted_admins_removal() {
    for seed in 40..46 {
        let (mut net, [a, b, c]) = three(seed);
        act(&mut net, a, |s| s.set_role(member(B), Role::Admin));
        net.run(1_000);
        session(&mut net, a)
            .set_role(member(B), Role::Member)
            .unwrap();
        session(&mut net, b).remove(member(C)).unwrap();
        net.pump(a);
        net.pump(b);
        net.run(3_000);
        for i in [a, b] {
            assert_eq!(role_seen_by(&net, i, B), Some(Role::Member), "seed {seed}");
        }
        let stored = stored_keyring(&net);
        assert_eq!(stored.role(&member(B)), Some(Role::Member), "seed {seed}");
        let c_out = stored.role(&member(C)).is_none();
        // Report which way it went; the relay may refuse B's entry once B is demoted.
        eprintln!("seed {seed}: C removed = {c_out}");
        if c_out {
            assert!(net.clients[c].saw(&SessionEvent::Removed));
        } else {
            assert!(!net.clients[c].session().has_ended());
        }
        assert!(net.clients[a].session().is_online());
    }
}

fn removal_by(net: &Net, author: u8, subject: u8) -> bool {
    stored_has(net, |p, a| {
        *a == member(author)
            && matches!(p, Payload::RemoveMember { member: m } if *m == member(subject))
    })
}

/// Criterion 27 through the real relay, setup (a): A and C (admins, both online) write their
/// removals of each other in the same step, both entries in flight. Observed and pinned: the
/// relay stores the first removal to arrive, refuses and closes the removed admin's connection
/// and drops its removal, which never reaches the keyring; nobody's view diverges. So through one
/// relay with both online the notice of criterion 27 never arises; one admin is removed.
#[test]
fn mutual_removal_through_one_relay_in_the_same_step() {
    let mut outcomes = Vec::new();
    for seed in 50..60 {
        let (mut net, [a, b, c]) = three(seed);
        act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
        net.run(1_000);
        session(&mut net, a).remove(member(C)).unwrap();
        session(&mut net, c).remove(member(A)).unwrap();
        net.pump(a);
        net.pump(c);
        net.run(5_000);
        let stored = stored_keyring(&net);
        let (a_wins, c_wins) = (removal_by(&net, A, C), removal_by(&net, C, A));
        outcomes.push(format!(
            "seed {seed}: stored A->C {a_wins}, C->A {c_wins}; relay roles A {:?} C {:?}; \
             held_back {}; C refused {:?}; A refused {:?}",
            stored.role(&member(A)),
            stored.role(&member(C)),
            stored.held_back().len(),
            refusals(&net, c),
            refusals(&net, a),
        ));
        assert!(a_wins ^ c_wins, "exactly one removal lands: {outcomes:?}");
        let (winner, loser, w, l) = if a_wins { (a, c, A, C) } else { (c, a, C, A) };
        assert_eq!(stored.role(&member(w)), Some(Role::Admin));
        assert_eq!(stored.role(&member(l)), None);
        assert_eq!(stored.held_back(), Vec::new());
        assert!(
            net.clients[loser].saw(&SessionEvent::Removed),
            "{outcomes:?}"
        );
        assert!(net.clients[winner].session().is_online());
        for i in [winner, b] {
            assert_eq!(role_seen_by(&net, i, l), None);
            assert_eq!(role_seen_by(&net, i, w), Some(Role::Admin));
        }
    }
    eprintln!("{}", outcomes.join("\n"));
}

fn refusals(net: &Net, i: usize) -> Vec<RefusalReason> {
    net.clients[i]
        .events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Refused(r) => Some(*r),
            _ => None,
        })
        .collect()
}

/// Criterion 27 through the real relay, setup (b), and every keyring act generally: A and C
/// lose their network before their sessions notice, write their removals of each other
/// (accepted locally, `Ok`), and come back one after the other. Since ADR 0041 decision 11 the
/// first one back resends its removal and the relay stores it, so the other is removed when it
/// comes back (expectation changed with the architect's amendment: through one relay a mutual
/// removal removes one admin). Every confirmed view, and the removed admin's ended heads,
/// agree with the relay.
#[test]
fn mutual_removal_through_one_relay_after_both_were_cut_off() {
    for first_back in [0usize, 1] {
        let (mut net, [a, b, c]) = three(61 + first_back as u64);
        act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
        net.run(1_000);
        net.set_link(a, false);
        net.set_link(c, false);
        assert_eq!(session(&mut net, a).remove(member(C)), Ok(()));
        assert_eq!(session(&mut net, c).remove(member(A)), Ok(()));
        net.pump(a);
        net.pump(c);
        net.run(20_000);
        let (x, y) = if first_back == 0 { (a, c) } else { (c, a) };
        net.set_link(x, true);
        net.network_back(x);
        net.run(5_000);
        net.set_link(y, true);
        net.network_back(y);
        net.run(60_000);
        let stored = stored_keyring(&net);
        let confirmed = |i: usize| net.clients[i].session().confirmed_keyring().unwrap();
        let report = format!(
            "first back {}: relay A {:?} C {:?} held_back {}; A's view C {:?}; C's view A {:?}; \
             B's view A {:?} C {:?}",
            if first_back == 0 { "A" } else { "C" },
            stored.role(&member(A)),
            stored.role(&member(C)),
            stored.held_back().len(),
            confirmed(a).role(&member(C)),
            confirmed(c).role(&member(A)),
            role_seen_by(&net, b, A),
            role_seen_by(&net, b, C),
        );
        eprintln!("{report}");
        let (winner, loser, w, l) = if first_back == 0 {
            (a, c, A, C)
        } else {
            (c, a, C, A)
        };
        // The first one back wins, the other is removed and told so.
        assert_eq!(stored.role(&member(w)), Some(Role::Admin), "{report}");
        assert_eq!(stored.role(&member(l)), None, "{report}");
        assert_eq!(stored.held_back(), Vec::new(), "{report}");
        assert!(net.clients[loser].saw(&SessionEvent::Removed), "{report}");
        assert!(net.clients[winner].session().is_online(), "{report}");
        for i in [winner, b] {
            assert_eq!(role_seen_by(&net, i, l), None, "{report}");
            assert_eq!(confirmed(i).role(&member(l)), None, "{report}");
        }
        // The loser's own removal never reached the relay: its confirmed view and its ended
        // heads hold only what the relay stored.
        assert_eq!(
            confirmed(loser).role(&member(w)),
            Some(Role::Admin),
            "{report}"
        );
        let stored_hashes: Vec<EntryHash> = net.store[&ROOM]
            .keyring
            .iter()
            .map(|e| Entry::decode(e).unwrap().try_hash().unwrap())
            .collect();
        let heads = net.clients[loser].session().confirmed_heads();
        assert_ne!(heads, Vec::new(), "{report}");
        assert!(heads.iter().all(|h| stored_hashes.contains(h)), "{report}");
    }
}

/// The same loss for a leave (criteria 10 and 11): B leaves while its link is silently down.
/// B believes it left (its session ends, its file goes local) but the room still lists B as a
/// participant holding the current key; no admin sees "B left".
#[test]
fn a_leave_written_into_a_dead_link_reaches_the_room() {
    let (mut net, [a, b, c]) = three(65);
    net.set_link(b, false);
    let wrote = session(&mut net, b).leave();
    net.pump(b);
    net.run(20_000);
    net.set_link(b, true);
    net.network_back(b);
    net.run(30_000);
    let b_thinks_left =
        net.clients[b].saw(&SessionEvent::Removed) || net.clients[b].session().has_ended();
    let report = format!(
        "leave {wrote:?}; B ended {}; B saw Removed {}; relay lists B {:?}; A sees B {:?}; C sees B {:?}",
        net.clients[b].session().has_ended(),
        net.clients[b].saw(&SessionEvent::Removed),
        stored_keyring(&net).role(&member(B)),
        role_seen_by(&net, a, B),
        role_seen_by(&net, c, B),
    );
    eprintln!("{report}");
    if wrote.is_ok() && b_thinks_left {
        assert_eq!(stored_keyring(&net).role(&member(B)), None, "{report}");
        assert_eq!(role_seen_by(&net, a, B), None, "{report}");
    }
}

/// B's return check with heads and keys as the ended record and credential store keep them.
fn check(
    net: &mut Net,
    b: usize,
    then_leave: bool,
    heads: &[EntryHash],
) -> (Vec<ClientFrame>, Option<CheckOutcome>, u64) {
    let from = net.clients[b].sent.len();
    let epoch_keys = net.clients[b].epoch_keys.clone();
    net.start(
        b,
        ROOM,
        0,
        SessionStart::ReturnCheck {
            epoch_keys,
            then_leave,
            heads: heads.to_vec(),
        },
    );
    let start = net.now;
    let mut outcome = None;
    net.run_until(10_000, |n| {
        outcome = n.clients[b].events.iter().rev().find_map(|e| match e {
            SessionEvent::CheckResult(o) => Some(*o),
            _ => None,
        });
        outcome.is_some() && n.clients[b].session().has_ended()
    });
    let took = net.now.since(start);
    let frames = net.clients[b].sent[from..]
        .iter()
        .map(|m| ClientFrame::decode(m).unwrap())
        .collect();
    // Only the outcome of this check (events of earlier checks are older).
    (frames, outcome, took)
}

fn only_check_frames(frames: &[ClientFrame]) -> bool {
    frames.iter().all(|f| {
        matches!(
            f,
            ClientFrame::Hello {
                since_seq: u64::MAX,
                ..
            } | ClientFrame::Proof { .. }
                | ClientFrame::Ping
        )
    })
}

/// Criteria 20, 21 and 33: "Keep as it is" chosen before any add back sends nothing beyond
/// Hello, Proof and Ping, writes no leave and answers "not yet".
#[test]
fn keep_as_it_is_while_still_removed_sends_nothing_else() {
    let (mut net, [a, b, _c]) = three(71);
    act(&mut net, a, |s| s.remove(member(B)));
    net.run(1_000);
    let heads = net.clients[b].session().keyring().unwrap().heads();
    let entries_before = net.store[&ROOM].keyring.len();
    let (frames, outcome, _) = check(&mut net, b, true, &heads);
    assert_eq!(outcome, Some(CheckOutcome::NotYet));
    assert!(only_check_frames(&frames), "{frames:?}");
    assert_eq!(net.store[&ROOM].keyring.len(), entries_before);
}

/// Criteria 13 and 33: after a leave or a removal the device makes no connection attempt by
/// itself for a minute (no reconnect loop), and the room refuses it if it tried.
#[test]
fn a_leaver_and_a_removed_device_never_reconnect_by_themselves() {
    let (mut net, [a, b, c]) = three(72);
    act(&mut net, b, ClientSession::leave);
    act(&mut net, a, |s| s.remove(member(C)));
    net.run(1_000);
    let (bn, cn) = (
        net.clients[b].connect_times.len(),
        net.clients[c].connect_times.len(),
    );
    net.run(60_000);
    assert_eq!(net.clients[b].connect_times.len(), bn);
    assert_eq!(net.clients[c].connect_times.len(), cn);
    // A network-back signal does not wake them either.
    net.network_back(b);
    net.network_back(c);
    net.run(5_000);
    assert_eq!(net.clients[b].connect_times.len(), bn);
    assert_eq!(net.clients[c].connect_times.len(), cn);
}

/// Criterion 11 and 12: a member sees the rotation pending but may not rotate; with two admins,
/// the first admin's rotation clears it for everybody, C keeps editing across it.
#[test]
fn the_first_admins_rotation_clears_the_pending_rotation_for_all() {
    let (mut net, [a, b, c]) = three(73);
    // D joins as a second admin.
    let secret = net.invite(a, 3600);
    let d = net.add_client(D);
    net.join(d, secret);
    assert!(
        net.run_until(5_000, |n| n.converged(&[a, b, c, d]))
            .is_some()
    );
    act(&mut net, a, |s| s.set_role(member(D), Role::Admin));
    net.run(1_000);
    act(&mut net, b, ClientSession::leave);
    net.run(1_000);
    for i in [a, c, d] {
        assert_eq!(
            net.clients[i]
                .session()
                .keyring()
                .unwrap()
                .rotation_pending(),
            vec![member(B)]
        );
    }
    assert_eq!(session(&mut net, c).rotate(), Err(SessionError::NotAdmin));
    act(&mut net, d, ClientSession::rotate);
    net.edit(c, |doc| rect(doc, 40.0));
    net.run(2_000);
    for i in [a, c, d] {
        assert_eq!(
            net.clients[i]
                .session()
                .keyring()
                .unwrap()
                .rotation_pending(),
            Vec::new()
        );
    }
    assert!(net.run_until(3_000, |n| n.converged(&[a, c, d])).is_some());
}

/// Criterion 26 and the flaky first check of the browser journey: when does the relay hold A's
/// add back, measured from the call, for both history choices; and does a check started at any
/// moment after A's own keyring lists B answer "added back"?
#[test]
fn the_first_check_right_after_an_add_back() {
    for history in [History::Full, History::FromNow] {
        for seed in 80..86 {
            let (mut net, [a, b, _c]) = three(seed);
            act(&mut net, a, |s| s.remove(member(B)));
            net.run(1_000);
            let heads = net.clients[b].session().keyring().unwrap().heads();
            act(&mut net, a, |s| s.add_back(member(B), history));
            let listed_at_a = net.clients[a]
                .session()
                .keyring()
                .unwrap()
                .role(&member(B))
                .is_some();
            let start = net.now;
            let stored_after = net.run_until(5_000, |n| {
                stored_has(n, |p, _| matches!(p, Payload::AddMember { identity, .. } if identity.member == member(B)))
            });
            eprintln!(
                "{history:?} seed {seed}: A lists B at once = {listed_at_a}, relay stores the add \
                 after {stored_after:?} ms"
            );
            assert!(stored_after.is_some());
            assert!(
                net.now.since(start) <= 1_000,
                "{history:?}: the add reaches the relay in 1 s"
            );
            let (frames, outcome, took) = check(&mut net, b, false, &heads);
            assert_eq!(
                outcome,
                Some(CheckOutcome::AddedBack),
                "{history:?} seed {seed}"
            );
            assert!(only_check_frames(&frames));
            assert!(took <= 5_000, "criterion 26: {took} ms");
        }
    }
}

/// Criterion 26, the race itself: B's check starts in the same step as A's add back (the
/// maker says "check now" while A clicks). The answer may be "not yet" while the add is in
/// flight, but a second check a moment later must say "added back"; measured per seed.
#[test]
fn a_check_racing_the_add_back_answers_not_yet_then_added_back() {
    let mut first_not_yet = 0;
    for seed in 90..110 {
        let (mut net, [a, b, _c]) = three(seed);
        act(&mut net, a, |s| s.remove(member(B)));
        net.run(1_000);
        let heads = net.clients[b].session().keyring().unwrap().heads();
        session(&mut net, a)
            .add_back(member(B), History::Full)
            .unwrap();
        let epoch_keys = net.clients[b].epoch_keys.clone();
        net.start(
            b,
            ROOM,
            0,
            SessionStart::ReturnCheck {
                epoch_keys,
                then_leave: false,
                heads: heads.clone(),
            },
        );
        net.pump(a);
        net.run(2_000);
        let first = net.clients[b].events.iter().rev().find_map(|e| match e {
            SessionEvent::CheckResult(o) => Some(*o),
            _ => None,
        });
        if first == Some(CheckOutcome::NotYet) {
            first_not_yet += 1;
        }
        let (_, second, _) = check(&mut net, b, false, &heads);
        assert_eq!(second, Some(CheckOutcome::AddedBack), "seed {seed}");
    }
    eprintln!("a check started with the add back said 'not yet' in {first_not_yet} of 20 runs");
}

/// Criterion 25: an admin cannot put a leaver back to work silently: after the add back the
/// leaver's file (no session running) stays out until B checks and chooses; the relay sees no
/// connection of B.
#[test]
fn an_add_back_does_not_wake_the_leaver() {
    let (mut net, [a, b, _c]) = three(74);
    act(&mut net, b, ClientSession::leave);
    net.run(1_000);
    let connects = net.clients[b].connect_times.len();
    let sent = net.clients[b].sent.len();
    act(&mut net, a, |s| s.add_back(member(B), History::Full));
    net.run(10_000);
    assert_eq!(net.clients[b].connect_times.len(), connects);
    assert_eq!(net.clients[b].sent.len(), sent);
}

/// Criterion 20 / ADR 0041 decision 6: a relay that serves only part of the keyring (cut
/// before A's add back) gets "not yet", never "added back"; refusal is a denial, not a fake.
#[test]
fn a_relay_refusing_the_returning_device_gives_not_yet() {
    let (mut net, [a, b, _c]) = three(75);
    act(&mut net, a, |s| s.remove(member(B)));
    net.run(1_000);
    let heads = net.clients[b].session().keyring().unwrap().heads();
    // B's check is answered "removed" by the relay (no add back happened).
    let (_, outcome, _) = check(&mut net, b, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::NotYet));
    assert!(
        net.clients[b]
            .events
            .iter()
            .any(|e| matches!(e, SessionEvent::CheckResult(CheckOutcome::NotYet)))
    );
}

/// The consequence of the lost entry for content: A removes C (removal, rotation, snapshot)
/// while its link is silently down, then comes back and edits. A's edits must still reach B
/// (they are sealed under the epoch the lost `NewEpoch` made, if it stays lost).
#[test]
fn edits_after_a_removal_into_a_dead_link_still_reach_the_others() {
    let (mut net, [a, b, c]) = three(66);
    net.set_link(a, false);
    let wrote = session(&mut net, a).remove(member(C));
    net.pump(a);
    net.run(20_000);
    net.set_link(a, true);
    net.network_back(a);
    net.run(10_000);
    net.edit(a, |d| rect(d, 70.0));
    let converged = net.run_until(60_000, |n| n.converged(&[a, b])).is_some();
    let report = format!(
        "remove {wrote:?}; A's epoch {}, relay epoch {}; relay lists C {:?}; A sees C {:?}; C ended {}; A and B converged {converged}",
        net.clients[a]
            .session()
            .keyring()
            .unwrap()
            .current_epoch()
            .number,
        stored_keyring(&net).current_epoch().number,
        stored_keyring(&net).role(&member(C)),
        role_seen_by(&net, a, C),
        net.clients[c].session().has_ended(),
    );
    eprintln!("{report}");
    assert!(converged, "{report}");
    assert_eq!(
        role_seen_by(&net, a, C),
        stored_keyring(&net).role(&member(C)),
        "{report}"
    );
}

/// Criteria 20 and 26 for the admin the relay cut off in a same-step mutual removal: its own
/// keyring (and so its ended record's heads) holds its removal of the winner and the rotation,
/// which the relay never stored. When the winner adds it back, its check must say "added back".
#[test]
fn the_cut_off_admin_of_a_mutual_removal_can_be_added_back() {
    let (mut net, [a, _b, c]) = three(50);
    act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
    net.run(1_000);
    session(&mut net, a).remove(member(C)).unwrap();
    session(&mut net, c).remove(member(A)).unwrap();
    net.pump(a);
    net.pump(c);
    net.run(5_000);
    let (winner, loser, l) = if removal_by(&net, A, C) {
        (a, c, C)
    } else {
        (c, a, A)
    };
    assert!(net.clients[loser].saw(&SessionEvent::Removed));
    let heads = net.clients[loser].session().keyring().unwrap().heads();
    act(&mut net, winner, |s| s.add_back(member(l), History::Full));
    net.run(1_000);
    let (_, outcome, _) = check(&mut net, loser, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::AddedBack));
}

/// The same for the leaver whose leave went into a dead link. Since ADR 0041 decision 11 the
/// leave is resent once the link is back, so the room lists B as left and A's removal of B is
/// refused "not a member" (expectation changed with the architect's amendment); B's ended heads
/// are its confirmed heads. After A adds B back, B's check must say "added back".
#[test]
fn a_leaver_whose_leave_was_lost_can_be_added_back() {
    let (mut net, [a, b, _c]) = three(65);
    net.set_link(b, false);
    session(&mut net, b).leave().unwrap();
    net.pump(b);
    net.run(20_000);
    net.set_link(b, true);
    net.network_back(b);
    assert!(
        net.run_until(30_000, |n| n.clients[b].session().has_ended())
            .is_some()
    );
    assert_eq!(stored_keyring(&net).role(&member(B)), None);
    let heads = net.clients[b].session().confirmed_heads();
    assert_eq!(heads, net.clients[b].session().keyring().unwrap().heads());
    assert_eq!(
        session(&mut net, a).remove(member(B)),
        Err(SessionError::NotAMember)
    );
    act(&mut net, a, |s| s.add_back(member(B), History::Full));
    net.run(1_000);
    let (_, outcome, _) = check(&mut net, b, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::AddedBack));
}
