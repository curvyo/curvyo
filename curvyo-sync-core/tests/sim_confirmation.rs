//! Own keyring entries confirmed by the relay and resent until they are (ADR 0041 decision 11,
//! `specs/0097-membership-lifecycle` criteria 10, 13, 20, 26 to 29a), and which concurrent acts
//! freeze a room through one relay (decision 4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::BTreeSet;

use curvyo_crypto_core::keyring::{Entry, EntryHash, Keyring, Payload, Role};
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{
    ClientSession, ConnectionId, MembershipNotice, Millis, PendingAct, PendingKind, RelayFrame,
    SessionConfig, SessionError, SessionEvent, SessionStart, membership_notices,
};
use support::sim::{Net, shared};
use support::{ROOM, device, member, seed};

const A: u8 = 1;
const B: u8 = 2;
const C: u8 = 3;

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// A (admin), B and C, converged.
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

fn stored_entries(net: &Net) -> Vec<Entry> {
    net.store[&ROOM]
        .keyring
        .iter()
        .map(|e| Entry::decode(e).unwrap())
        .collect()
}

fn stored_keyring(net: &Net) -> Keyring {
    Keyring::replay(&stored_entries(net)).unwrap()
}

fn stored_hashes(net: &Net) -> Vec<EntryHash> {
    stored_entries(net)
        .iter()
        .map(|e| e.try_hash().unwrap())
        .collect()
}

/// Cuts `i`'s link without its session noticing, lets `write` act, waits past the loss and
/// brings the link back.
fn into_dead_link(
    net: &mut Net,
    i: usize,
    write: impl FnOnce(&mut ClientSession) -> Result<(), SessionError>,
) {
    net.set_link(i, false);
    act(net, i, write);
    net.run(20_000);
    net.set_link(i, true);
    net.network_back(i);
}

/// Criterion 29a: a role change, an add back and a removal with rotation written into a
/// silently dead link reach the room once the link is back, without any action; the writer's
/// edits sealed under the resent epoch reach the others.
#[test]
fn every_act_written_into_a_dead_link_reaches_the_room() {
    let (mut net, [a, b, c]) = three(201);
    into_dead_link(&mut net, a, |s| s.set_role(member(B), Role::Admin));
    net.run(10_000);
    assert_eq!(stored_keyring(&net).role(&member(B)), Some(Role::Admin));
    assert_eq!(session(&mut net, a).pending_acts(), Vec::new());

    into_dead_link(&mut net, a, |s| s.remove(member(C)));
    net.run(10_000);
    let stored = stored_keyring(&net);
    assert_eq!(stored.role(&member(C)), None);
    assert!(net.clients[c].saw(&SessionEvent::Removed));
    let epoch = net.clients[a].session().keyring().unwrap().current_epoch();
    assert_eq!(stored.current_epoch(), epoch);
    net.edit(a, |d| rect(d, 70.0));
    assert!(net.run_until(20_000, |n| n.converged(&[a, b])).is_some());

    into_dead_link(&mut net, a, |s| s.add_back(member(C)));
    net.run(10_000);
    assert_eq!(stored_keyring(&net).role(&member(C)), Some(Role::Member));
}

/// ADR 0041 decision 11: while the relay's keyring arrives again after a `Welcome`, frame by
/// frame, the confirmed view never empties or shrinks, so no row turns pending and no notice
/// reads an older admin set; the relay's keyring replaces it at the catch-up's `Pong`.
#[test]
fn a_reconnect_keeps_the_confirmed_view_whole() {
    let (mut net, [a, _b, _c]) = three(210);
    act(&mut net, a, |s| s.set_role(member(B), Role::Admin));
    net.run(1_000);
    let entries = net.clients[a].session().keyring_entries();
    let epoch_keys = net.clients[a].session().epoch_keys();
    let mut session = ClientSession::new(
        SessionConfig {
            room: ROOM,
            identity: device(A),
            peer_id: 77,
            nonce_seed: seed(A, 1),
            entropy_seed: seed(A, 2),
            since_seq: 0,
            start: SessionStart::Member {
                keyring: entries,
                epoch_keys,
            },
        },
        Millis(0),
    )
    .unwrap();
    let admins = |s: &ClientSession| {
        s.confirmed_keyring()
            .map(|k| curvyo_sync_core::admin_set(&k))
    };
    let feed = |s: &mut ClientSession, frame: RelayFrame| s.received(&frame.encode(), Millis(60));
    session.connected(Millis(1));
    feed(&mut session, RelayFrame::Challenge { nonce: [5; 32] });
    // A first catch-up: the confirmed view is the relay's keyring.
    let stored = stored_entries(&net);
    feed(
        &mut session,
        RelayFrame::Welcome {
            head_seq: 0,
            snapshot_seq: 0,
            connection: ConnectionId(9),
        },
    );
    for entry in &stored {
        feed(
            &mut session,
            RelayFrame::Keyring {
                entry: entry.encode().unwrap(),
            },
        );
    }
    feed(&mut session, RelayFrame::Pong);
    let whole = admins(&session);
    assert_eq!(whole.as_ref().map(Vec::len), Some(2));
    assert_eq!(session.pending_acts(), Vec::new());
    // A reconnect: the keyring arrives again, one frame at a time.
    session.disconnected(Millis(50));
    session.network_back(Millis(50));
    session.connected(Millis(50));
    feed(&mut session, RelayFrame::Challenge { nonce: [6; 32] });
    feed(
        &mut session,
        RelayFrame::Welcome {
            head_seq: 0,
            snapshot_seq: 0,
            connection: ConnectionId(10),
        },
    );
    assert_eq!(admins(&session), whole);
    assert_eq!(session.pending_acts(), Vec::new());
    for entry in &stored {
        feed(
            &mut session,
            RelayFrame::Keyring {
                entry: entry.encode().unwrap(),
            },
        );
        assert_eq!(admins(&session), whole, "the confirmed view stays whole");
        assert_eq!(session.pending_acts(), Vec::new());
    }
    feed(&mut session, RelayFrame::Pong);
    assert_eq!(admins(&session), whole);
}

/// Criteria 10 and 13: a leave written into a dead link stays pending ("Leaving…") and the
/// session goes on until the relay has it; then B ends as removed by its own hand and the room
/// lists B as left.
#[test]
fn a_leave_is_pending_until_the_relay_has_it() {
    let (mut net, [a, b, _c]) = three(202);
    net.set_link(b, false);
    act(&mut net, b, ClientSession::leave);
    assert_eq!(
        net.clients[b].session().pending_acts(),
        vec![PendingAct {
            member: member(B),
            kind: PendingKind::Leave
        }]
    );
    net.run(20_000);
    assert!(!net.clients[b].session().has_ended());
    assert_eq!(stored_keyring(&net).role(&member(B)), Some(Role::Member));
    net.set_link(b, true);
    net.network_back(b);
    assert!(
        net.run_until(10_000, |n| n.clients[b].session().has_ended())
            .is_some()
    );
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    assert_eq!(stored_keyring(&net).role(&member(B)), None);
    net.run(2_000);
    assert_eq!(
        net.clients[a].session().keyring().unwrap().role(&member(B)),
        None
    );
    // Its ended heads hold only what the relay stored.
    let stored = stored_hashes(&net);
    let heads = net.clients[b].session().confirmed_heads();
    assert!(!heads.is_empty() && heads.iter().all(|h| stored.contains(h)));
}

/// Criterion 10: a leave written just before the app closed goes out on the next open, before
/// the device ends.
#[test]
fn a_reopened_device_sends_its_pending_leave_first() {
    let (mut net, [_a, b, _c]) = three(203);
    net.set_link(b, false);
    act(&mut net, b, ClientSession::leave);
    net.close_app(b);
    net.set_link(b, true);
    net.reopen(b);
    assert_eq!(
        net.clients[b].session().pending_acts(),
        vec![PendingAct {
            member: member(B),
            kind: PendingKind::Leave
        }]
    );
    assert!(
        net.run_until(10_000, |n| n.clients[b].session().has_ended())
            .is_some()
    );
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    assert_eq!(stored_keyring(&net).role(&member(B)), None);
}

/// Resends are idempotent: an entry resent after several reconnects is stored once.
#[test]
fn a_resent_entry_is_stored_once() {
    let (mut net, [a, _b, _c]) = three(204);
    net.set_link(a, false);
    act(&mut net, a, |s| s.set_role(member(B), Role::Admin));
    net.run(20_000);
    for _ in 0..3 {
        net.set_link(a, true);
        net.network_back(a);
        net.run(3_000);
    }
    let hashes = stored_hashes(&net);
    let unique: BTreeSet<EntryHash> = hashes.iter().copied().collect();
    assert_eq!(hashes.len(), unique.len());
    assert_eq!(stored_keyring(&net).role(&member(B)), Some(Role::Admin));
}

/// Criteria 20 and 26: an add back is pending on the admin's side until the relay holds it, so
/// "Added" on A's screen means B's check finds it.
#[test]
fn an_add_back_is_pending_until_the_relay_holds_it() {
    let (mut net, [a, _b, _c]) = three(205);
    act(&mut net, a, |s| s.remove(member(B)));
    net.run(1_000);
    act(&mut net, a, |s| s.add_back(member(B)));
    let pending = vec![PendingAct {
        member: member(B),
        kind: PendingKind::Add,
    }];
    assert_eq!(net.clients[a].session().pending_acts(), pending);
    let mut confirmed_while_absent = false;
    net.run_until(2_000, |n| {
        let stored = stored_keyring(n).role(&member(B)).is_some();
        if n.clients[a].session().pending_acts().is_empty() && !stored {
            confirmed_while_absent = true;
        }
        stored && n.clients[a].session().pending_acts().is_empty()
    });
    assert!(!confirmed_while_absent);
    assert_eq!(net.clients[a].session().pending_acts(), Vec::new());
}

/// Criterion 27 (a): through one relay a mutual removal removes the admin whose removal
/// arrived second; it is told it was removed, and the room is not frozen.
#[test]
fn a_mutual_removal_through_one_relay_removes_the_second_admin() {
    let (mut net, [a, b, c]) = three(206);
    act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
    net.run(1_000);
    // C does not read its socket: it writes its removal of A before it learns of A's.
    net.clients[c].frozen = true;
    act(&mut net, a, |s| s.remove(member(C)));
    net.run(1_000);
    act(&mut net, c, |s| s.remove(member(A)));
    net.run(1_000);
    net.clients[c].frozen = false;
    net.run(10_000);
    let stored = stored_keyring(&net);
    assert_eq!(stored.role(&member(A)), Some(Role::Admin));
    assert_eq!(stored.role(&member(C)), None);
    assert_eq!(stored.held_back(), Vec::new());
    assert!(net.clients[c].saw(&SessionEvent::Removed));
    assert!(net.clients[c].session().has_ended());
    // C's own removal of A is no head of its ended record.
    let hashes = stored_hashes(&net);
    let heads = net.clients[c].session().confirmed_heads();
    assert!(heads.iter().all(|h| hashes.contains(h)));
    for i in [a, b] {
        let notices = membership_notices(None, net.clients[i].session().keyring().unwrap(), &[]);
        assert!(
            !notices
                .iter()
                .any(|n| matches!(n, MembershipNotice::MutualRemoval { .. }))
        );
    }
}

fn mutual_notice(keyring: &Keyring) -> bool {
    membership_notices(None, keyring, &[])
        .iter()
        .any(|n| matches!(n, MembershipNotice::MutualRemoval { admins, .. } if admins.len() == 2))
}

/// Criteria 27 to 29 (b): two admins who demote each other through one relay are both stored
/// and both held back; both stay admins and every list shows the notice.
#[test]
fn crossed_demotions_through_one_relay_freeze_the_room() {
    let (mut net, [a, b, c]) = three(207);
    act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
    net.run(1_000);
    net.clients[c].frozen = true;
    act(&mut net, a, |s| s.set_role(member(C), Role::Member));
    net.run(1_000);
    act(&mut net, c, |s| s.set_role(member(A), Role::Member));
    net.run(1_000);
    net.clients[c].frozen = false;
    net.run(10_000);
    let stored = stored_keyring(&net);
    assert_eq!(stored.role(&member(A)), Some(Role::Admin));
    assert_eq!(stored.role(&member(C)), Some(Role::Admin));
    assert_eq!(stored.held_back().len(), 2);
    assert!(mutual_notice(&stored));
    for i in [a, b, c] {
        let session = net.clients[i].session();
        assert!(session.is_online());
        assert_eq!(session.pending_acts(), Vec::new());
        assert!(mutual_notice(session.keyring().unwrap()));
        assert!(mutual_notice(&session.confirmed_keyring().unwrap()));
    }
}

/// Criterion 27 (b), the second trigger: C demotes A, and A's removal of C, written before A
/// learned of it, reaches the relay after it: a demotion closes no connection, so both are
/// stored and held back.
#[test]
fn a_removal_after_the_removers_demotion_freezes_the_room() {
    let (mut net, [a, _b, c]) = three(208);
    act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
    net.run(1_000);
    net.clients[a].frozen = true;
    act(&mut net, c, |s| s.set_role(member(A), Role::Member));
    net.run(1_000);
    act(&mut net, a, |s| s.remove(member(C)));
    net.run(1_000);
    net.clients[a].frozen = false;
    net.run(10_000);
    let stored = stored_keyring(&net);
    assert_eq!(stored.role(&member(A)), Some(Role::Admin));
    assert_eq!(stored.role(&member(C)), Some(Role::Admin));
    assert_eq!(stored.held_back().len(), 2);
    assert!(mutual_notice(&stored));
    assert!(!net.clients[c].session().has_ended());
}

/// Criteria 27 to 29 (c): two removals written apart and merged outside one relay (two
/// keyrings put together offline) are both held back: the room is frozen and the notice shows.
#[test]
fn removals_merged_offline_freeze_the_room() {
    let (mut net, [a, _b, c]) = three(209);
    act(&mut net, a, |s| s.set_role(member(C), Role::Admin));
    net.run(1_000);
    net.set_link(a, false);
    net.set_link(c, false);
    act(&mut net, a, |s| s.remove(member(C)));
    act(&mut net, c, |s| s.remove(member(A)));
    let mut merged: Vec<Entry> = net.clients[a].session().keyring_entries();
    merged.extend(net.clients[c].session().keyring_entries());
    let keyring = Keyring::replay(&merged).unwrap();
    assert_eq!(keyring.role(&member(A)), Some(Role::Admin));
    assert_eq!(keyring.role(&member(C)), Some(Role::Admin));
    assert_eq!(keyring.held_back().len(), 2);
    assert!(mutual_notice(&keyring));
    let removals = keyring
        .accepted_entries()
        .iter()
        .filter(|e| matches!(e.payload, Payload::RemoveMember { .. }))
        .count();
    assert_eq!(removals, 2);
}
