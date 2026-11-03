//! Sharing, joining by invite link, refusals and rooms through the simulator with real document
//! replicas (`specs/0092` criteria 1 to 4, 27, 40).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_document_core::{Document, Length, Point, RectBounds};
use curvyo_sync_core::{RefusalReason, SessionEvent};
use support::sim::Net;
use support::{OTHER_ROOM, ROOM, member};

fn rect(doc: &Document, x: f64) {
    let _ = doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
}

/// A shares a document with one rectangle; B joins by invite; both converge.
fn shared_pair(seed: u64) -> (Net, usize, usize) {
    let mut net = Net::new(seed);
    let a = net.add_client(1);
    let b = net.add_client(2);
    let doc = Document::new(100);
    rect(&doc, 0.0);
    net.share(a, ROOM, doc);
    net.run(500);
    let secret = net.invite(a, 24 * 3600);
    net.join(b, secret);
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    (net, a, b)
}

/// Criterion 1: sharing writes a genesis with A as the only admin, uploads the sealed state and
/// goes online.
#[test]
fn sharing_writes_a_genesis_and_uploads_the_state() {
    let mut net = Net::new(1);
    let a = net.add_client(1);
    let doc = Document::new(100);
    rect(&doc, 0.0);
    net.share(a, ROOM, doc);
    net.run(500);
    let client = &net.clients[a];
    assert!(client.saw(&SessionEvent::Online));
    let keyring = client.session().keyring().unwrap();
    assert_eq!(keyring.participants().len(), 1);
    assert_eq!(keyring.participants()[0].identity.member, member(1));
    let room = net.relay.room(&ROOM).expect("the relay holds the room");
    assert!(room.snapshot_seq() == 0 && net.store[&ROOM].snapshot.is_some());
    assert_eq!(room.keyring().participants().len(), 1);
}

/// Criteria 2 and 3: the invite admits B automatically through A's rewrap; B never opens a
/// record before admission; both list two participants.
#[test]
fn an_invite_link_admits_the_joiner_through_an_online_member() {
    for seed in 0..4 {
        let (mut net, a, b) = shared_pair(seed);
        let joiner = &net.clients[b];
        let waiting = joiner
            .events
            .iter()
            .position(|e| *e == SessionEvent::WaitingForMember);
        let admitted = joiner
            .events
            .iter()
            .position(|e| *e == SessionEvent::Admitted)
            .expect("admitted");
        let first_remote = joiner
            .events
            .iter()
            .position(|e| matches!(e, SessionEvent::Remote { .. }))
            .expect("content after admission");
        assert!(admitted < first_remote);
        if let Some(waiting) = waiting {
            assert!(waiting < admitted);
        }
        for i in [a, b] {
            assert_eq!(
                net.clients[i]
                    .session()
                    .keyring()
                    .unwrap()
                    .participants()
                    .len(),
                2
            );
        }
        // And they edit together afterwards.
        net.edit(b, |d| rect(d, 50.0));
        assert!(net.run_until(2_000, |n| n.converged(&[a, b])).is_some());
    }
}

/// Criterion 3 (amended 2026-11-03): with no member online the joiner waits; it joins as soon
/// as one comes online before expiry; past expiry it is told the link expired.
#[test]
fn a_joiner_waits_for_a_member_and_is_told_when_the_link_expires() {
    let mut net = Net::new(7);
    let a = net.add_client(1);
    let b = net.add_client(2);
    let c = net.add_client(3);
    net.share(a, ROOM, Document::new(100));
    net.run(500);
    let soon = net.invite(a, 3_600);
    let short = net.invite(a, 20);
    net.run(500);
    net.close_app(a);
    net.join(b, soon);
    net.join(c, short);
    net.run(5_000);
    assert!(net.clients[b].saw(&SessionEvent::WaitingForMember));
    assert!(net.clients[b].doc.is_none(), "no content before admission");
    // C's link expires while it waits.
    net.run(20_000);
    assert!(net.clients[c].saw(&SessionEvent::InviteExpired));
    assert!(net.clients[c].session().has_ended());
    // A comes back: B (unexpired) is admitted, C stays out.
    net.reopen(a);
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    assert!(net.clients[b].saw(&SessionEvent::Admitted));
    assert!(net.clients[c].doc.is_none());
    let keyring = net.clients[a].session().keyring().unwrap();
    assert!(!keyring.holds(&member(3), keyring.current_epoch()));
}

/// Criterion 4: a used or expired link is refused with its reason, and the relay stores nothing
/// from the attempt.
#[test]
fn a_used_or_expired_link_is_refused_with_its_reason() {
    let mut net = Net::new(3);
    let a = net.add_client(1);
    let b = net.add_client(2);
    let c = net.add_client(3);
    let d = net.add_client(4);
    net.share(a, ROOM, Document::new(100));
    net.run(500);
    let once = net.invite(a, 3_600);
    let expiring = net.invite(a, 1);
    net.join(b, once.clone());
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    net.run(2_000);
    let stored_before = net.store[&ROOM].clone();
    net.join(c, once);
    net.join(d, expiring);
    net.run(2_000);
    assert!(net.clients[c].saw(&SessionEvent::Refused(RefusalReason::Used)));
    assert!(net.clients[d].saw(&SessionEvent::Refused(RefusalReason::Expired)));
    for i in [c, d] {
        assert!(net.clients[i].doc.is_none());
        assert!(
            !net.clients[i]
                .events
                .iter()
                .any(|e| matches!(e, SessionEvent::Remote { .. }))
        );
    }
    assert_eq!(net.store[&ROOM], stored_before);
}

/// Criterion 27: a device that was never invited, holding only the room id, is refused and
/// receives nothing; the participant lists do not change.
#[test]
fn an_uninvited_device_gets_only_a_refusal() {
    let (mut net, a, b) = shared_pair(11);
    let stranger = net.add_client(9);
    let fake = curvyo_sync_core::InviteSecret {
        room: ROOM,
        seed: [0x99; 32],
    };
    net.join(stranger, fake);
    net.run(2_000);
    let client = &net.clients[stranger];
    assert!(client.saw(&SessionEvent::Refused(RefusalReason::NotAMember)));
    assert!(client.doc.is_none());
    assert!(client.session().keyring().is_none(), "not even the keyring");
    for i in [a, b] {
        assert_eq!(
            net.clients[i]
                .session()
                .keyring()
                .unwrap()
                .participants()
                .len(),
            2
        );
    }
}

/// Criterion 27: a device the keyring does not list, starting as a member with a replayed
/// keyring, is refused with "not a member" once, ends its session and does not reconnect.
#[test]
fn a_stranger_starting_as_a_member_is_refused_and_stops() {
    let (mut net, a, _) = shared_pair(27);
    let keyring = net.clients[a].session().keyring_entries();
    let stranger = net.add_client(9);
    let config = curvyo_sync_core::SessionConfig {
        room: ROOM,
        identity: support::device(9),
        peer_id: 9 << 32,
        nonce_seed: support::seed(9, 1),
        entropy_seed: support::seed(9, 2),
        since_seq: 0,
        start: curvyo_sync_core::SessionStart::Member {
            keyring,
            epoch_keys: Vec::new(),
        },
    };
    net.clients[stranger].session =
        Some(curvyo_sync_core::ClientSession::new(config, net.now).unwrap());
    net.pump(stranger);
    net.run(60_000);
    let client = &net.clients[stranger];
    assert_eq!(
        client.count(|e| *e == SessionEvent::Refused(RefusalReason::NotAMember)),
        1
    );
    assert_eq!(
        client.connect_times.len(),
        1,
        "no reconnect after the refusal"
    );
    assert!(!client.session().is_online());
    assert!(
        !client
            .events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. }))
    );
}

/// Criterion 40: two shared documents of A on one relay stay separate: own invites, own
/// participant lists, no cross-talk.
#[test]
fn two_rooms_of_one_maker_stay_separate() {
    let mut net = Net::new(5);
    let a1 = net.add_client(1);
    let a2 = net.add_client(1);
    let b = net.add_client(2);
    let c = net.add_client(3);
    let first = Document::new(100);
    rect(&first, 0.0);
    let second = Document::new(200);
    rect(&second, 100.0);
    net.share(a1, ROOM, first);
    net.share(a2, OTHER_ROOM, second);
    net.run(500);
    let invite_b = net.invite(a1, 3_600);
    let invite_c = net.invite(a2, 3_600);
    net.join(b, invite_b);
    net.join(c, invite_c);
    assert!(
        net.run_until(5_000, |n| n.converged(&[a1, b]) && n.converged(&[a2, c]))
            .is_some()
    );
    net.edit(b, |d| rect(d, 30.0));
    net.edit(c, |d| rect(d, 130.0));
    assert!(
        net.run_until(5_000, |n| n.converged(&[a1, b]) && n.converged(&[a2, c]))
            .is_some()
    );
    assert!(!net.converged(&[a1, a2]));
    assert_eq!(net.clients[a1].doc().object_ids().len(), 2);
    assert_eq!(net.clients[a2].doc().object_ids().len(), 2);
    let members = |i: usize| -> Vec<_> {
        net.clients[i]
            .session()
            .keyring()
            .unwrap()
            .participants()
            .iter()
            .map(|p| p.identity.member)
            .collect()
    };
    assert_eq!(members(a1), {
        let mut m = vec![member(1), member(2)];
        m.sort();
        m
    });
    assert_eq!(members(a2), {
        let mut m = vec![member(1), member(3)];
        m.sort();
        m
    });
    assert_eq!(net.relay.room_ids(), vec![ROOM, OTHER_ROOM]);
}
