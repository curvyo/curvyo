//! Tester's acceptance tests for `specs/0092-cooperative-editing` slice 1, through the seeded
//! simulator with real document replicas: what the relay stores, invites under race and
//! restart, removal edge cases, offline merges with concurrent page resizes, and relay restarts.
//! Written from the specification before reading the implementation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

mod support;

use curvyo_crypto_core::keyring::Payload;
use curvyo_crypto_core::{EnvelopeHeader, MemberId};
use curvyo_document_core::{
    Affine, Color, Document, DocumentSize, DocumentVersion, FontKind, FontRef, Length, NodeId,
    Point, RectBounds, StyleEdit, TextLayoutValues, Vec2,
};
use curvyo_sync_core::{
    ClientFrame, ClientSession, InviteSecret, RefusalReason, SessionConfig, SessionError,
    SessionEvent, SessionStart,
};
use support::sim::{Net, shared};
use support::{OTHER_ROOM, ROOM, device, member, seed};

const MARKER: &str = "ZQX-MARKER-7731";

fn rect(doc: &Document, x: f64, y: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

fn marker_text(doc: &Document) -> NodeId {
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let id = doc
        .create_text(
            MARKER,
            &font,
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(10.0, 10.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let colour = Color {
        r: 0x13,
        g: 0xa7,
        b: 0xc9,
    };
    doc.edit_style(&[id], &StyleEdit::StrokeColor(colour))
        .unwrap();
    id
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn remove(net: &mut Net, admin: usize, who: u8) -> Result<(), SessionError> {
    let result = net.clients[admin]
        .session
        .as_mut()
        .unwrap()
        .remove(member(who));
    net.pump(admin);
    result
}

fn participants(net: &Net, i: usize) -> Vec<MemberId> {
    net.clients[i]
        .session()
        .keyring()
        .unwrap()
        .participants()
        .iter()
        .map(|p| p.identity.member)
        .collect()
}

fn epoch(net: &Net, i: usize) -> u32 {
    net.clients[i]
        .session()
        .keyring()
        .unwrap()
        .current_epoch()
        .number
}

/// The frame of the first page as (x, y, width, height).
fn frame(net: &Net, i: usize) -> (f64, f64, f64, f64) {
    let page = net.clients[i].doc().pages()[0].clone();
    (
        page.origin.x,
        page.origin.y,
        page.size.width.as_mm(),
        page.size.height.as_mm(),
    )
}

/// Every byte the relay stores for `room`.
fn stored_bytes(net: &Net, room: curvyo_sync_core::RoomId) -> Vec<Vec<u8>> {
    let stored = &net.store[&room];
    let mut out: Vec<Vec<u8>> = stored.keyring.clone();
    out.extend(stored.records.iter().map(|r| r.envelope.clone()));
    if let Some((_, snapshot)) = &stored.snapshot {
        out.push(snapshot.clone());
    }
    out
}

// ------------------------------------------------------------------ criterion 24, 16, 27

/// Criterion 24 (and 16): after sharing, joining, editing, presence carrying the marker, a
/// removal with its fresh snapshot and a relay restart, neither the relay's stored data nor any
/// byte any client sent holds the marker, the colour, or any 24-byte fragment of the document's
/// JSON or its Loro updates.
#[test]
fn t24_relay_store_and_wire_hold_no_plaintext() {
    let doc = Document::new(100);
    marker_text(&doc);
    let (mut net, c) = shared(240, ROOM, doc, &[2, 3]);
    let (a, b, x) = (c[0], c[1], c[2]);
    net.edit(b, marker_text);
    net.presence(a, format!("{MARKER} pointer #13a7c9").as_bytes());
    net.run(1_000);
    remove(&mut net, a, 3).unwrap();
    net.run(2_000);
    net.edit(a, |d| rect(d, 1.0, 1.0));
    net.restart_relay();
    assert!(
        net.run_until(30_000, |n| n.converged(&[a, b])
            && n.clients[a].session().is_online())
            .is_some()
    );
    let _ = x;
    let json = net.clients[a].doc().export_json().unwrap();
    let updates = net.clients[a]
        .doc()
        .export_updates(&DocumentVersion::never())
        .unwrap();
    assert!(
        contains(&json, MARKER.as_bytes()),
        "the search means something"
    );
    let mut haystacks = stored_bytes(&net, ROOM);
    for client in &net.clients {
        haystacks.extend(client.sent.iter().cloned());
    }
    let needles: Vec<&[u8]> = [
        MARKER.as_bytes(),
        b"13a7c9".as_slice(),
        b"13A7C9".as_slice(),
        b"pointer".as_slice(),
    ]
    .into_iter()
    .chain(json.chunks(24).filter(|c| c.len() == 24))
    .chain(updates.chunks(24).filter(|c| c.len() == 24))
    .collect();
    for hay in &haystacks {
        for needle in &needles {
            assert!(
                !contains(hay, needle),
                "plaintext {:?} reached the relay",
                String::from_utf8_lossy(needle)
            );
        }
    }
}

/// Criterion 16: presence is never stored by the relay, never in the document's history and
/// never in its saved file.
#[test]
fn t16_presence_is_not_stored_and_not_in_the_document() {
    let (mut net, c) = shared(160, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    let before_store = net.store[&ROOM].clone();
    let before_version = net.clients[a].doc().version();
    for n in 0..20u8 {
        net.presence(a, &[b'p', n]);
        net.run(60);
    }
    net.run(500);
    assert!(
        net.clients[b]
            .events
            .iter()
            .any(|e| matches!(e, SessionEvent::Presence { .. })),
        "B saw A's presence"
    );
    assert_eq!(
        net.store[&ROOM], before_store,
        "the relay stores no presence"
    );
    assert_eq!(net.clients[a].doc().version(), before_version);
    assert_eq!(net.clients[b].doc().version(), before_version);
}

/// Criterion 27: a device that was never invited but replays the room's keyring as if it were a
/// member is never admitted, receives no record, and the participant lists do not change.
#[test]
fn t27_a_stranger_replaying_the_keyring_is_refused() {
    let (mut net, c) = shared(270, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    net.edit(a, |d| rect(d, 0.0, 0.0));
    net.run(500);
    let keyring = net.clients[a].session().keyring_entries();
    let stranger = net.add_client(9);
    let config = SessionConfig {
        room: ROOM,
        identity: device(9),
        peer_id: 9 << 32,
        nonce_seed: seed(9, 1),
        entropy_seed: seed(9, 2),
        since_seq: 0,
        start: SessionStart::Member {
            keyring,
            epoch_keys: Vec::new(),
        },
    };
    net.clients[stranger].session = Some(ClientSession::new(config, net.now).unwrap());
    net.pump(stranger);
    net.run(30_000);
    let s = &net.clients[stranger];
    // The spec asks only that it gets nothing it can open; a stranger with a member-shaped
    // start is never admitted and is refused with not-a-member.
    assert!(!s.session().is_online(), "never admitted");
    assert_eq!(
        s.session().epoch_keys().len(),
        0,
        "no key reaches a stranger"
    );
    assert!(
        !s.events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. })),
        "no record reaches a stranger"
    );
    for i in [a, b] {
        assert_eq!(participants(&net, i).len(), 2);
    }
}

// ------------------------------------------------------------------ criteria 3, 4, 38

/// Criterion 4 with 38: an invite used once stays used after the relay restarts on its stored
/// data; a third device is refused with "used" and sees no content.
#[test]
fn t4_a_used_invite_stays_used_after_a_relay_restart() {
    let mut net = Net::new(41);
    let a = net.add_client(1);
    let b = net.add_client(2);
    let c = net.add_client(3);
    net.share(a, ROOM, Document::new(100));
    net.run(500);
    let once = net.invite(a, 3_600);
    net.join(b, once.clone());
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    net.run(1_000);
    net.restart_relay();
    assert!(
        net.run_until(30_000, |n| n.clients[a].session().is_online())
            .is_some()
    );
    let stored = net.store[&ROOM].clone();
    net.join(c, once);
    net.run(3_000);
    assert!(net.clients[c].saw(&SessionEvent::Refused(RefusalReason::Used)));
    assert!(net.clients[c].doc.is_none());
    assert_eq!(net.store[&ROOM], stored, "the attempt stored nothing");
}

/// Criterion 4: two devices race for one invite; exactly one is admitted, the other is refused
/// as "used" and sees no content.
#[test]
fn t4_two_devices_racing_for_one_invite_admit_exactly_one() {
    for s in 0..6 {
        let mut net = Net::new(400 + s);
        let a = net.add_client(1);
        net.share(a, ROOM, Document::new(100));
        net.run(500);
        let once = net.invite(a, 3_600);
        let b = net.add_client(2);
        let c = net.add_client(3);
        net.join(b, once.clone());
        net.join(c, once);
        net.run(8_000);
        let admitted: Vec<usize> = [b, c]
            .into_iter()
            .filter(|&i| net.clients[i].doc.is_some())
            .collect();
        assert_eq!(admitted.len(), 1, "seed {s}: exactly one joins");
        let loser = if admitted[0] == b { c } else { b };
        assert!(
            net.clients[loser].saw(&SessionEvent::Refused(RefusalReason::Used)),
            "seed {s}: {:?}",
            net.clients[loser].events
        );
        assert!(net.converged(&[a, admitted[0]]));
        assert_eq!(participants(&net, a).len(), 2);
    }
}

/// Criterion 3: two joiners with their own invites at the same time are both admitted and the
/// participant lists show three entries everywhere.
#[test]
fn t3_two_joiners_at_once_are_both_admitted() {
    let mut net = Net::new(31);
    let a = net.add_client(1);
    let doc = Document::new(100);
    rect(&doc, 0.0, 0.0);
    net.share(a, ROOM, doc);
    net.run(500);
    let first = net.invite(a, 3_600);
    let second = net.invite(a, 3_600);
    let b = net.add_client(2);
    let c = net.add_client(3);
    net.join(b, first);
    net.join(c, second);
    assert!(net.run_until(10_000, |n| n.converged(&[a, b, c])).is_some());
    net.run(1_000);
    for i in [a, b, c] {
        assert_eq!(participants(&net, i).len(), 3);
    }
}

/// Criterion 3: an invite of one room does not open another room of the same maker
/// (criterion 40): a link whose room was swapped is refused.
#[test]
fn t40_an_invite_does_not_open_another_room() {
    let mut net = Net::new(42);
    let a1 = net.add_client(1);
    let a2 = net.add_client(1);
    net.share(a1, ROOM, Document::new(100));
    net.share(a2, OTHER_ROOM, Document::new(101));
    net.run(500);
    let secret = net.invite(a1, 3_600);
    let swapped = InviteSecret {
        room: OTHER_ROOM,
        seed: secret.seed,
    };
    let b = net.add_client(2);
    net.join(b, swapped);
    net.run(5_000);
    assert!(net.clients[b].doc.is_none());
    assert_eq!(participants(&net, a2).len(), 1);
    assert_eq!(participants(&net, a1).len(), 1);
}

// ------------------------------------------------------------------ criteria 32 to 34

/// A (admin), B, C, D; all converged.
fn four(seed: u64) -> (Net, [usize; 4]) {
    let doc = Document::new(100);
    rect(&doc, 0.0, 0.0);
    let (net, c) = shared(seed, ROOM, doc, &[2, 3, 4]);
    (net, [c[0], c[1], c[2], c[3]])
}

/// Criterion 32: removing a device that is no member, or removing while offline, is refused and
/// writes nothing.
#[test]
fn t32_removing_a_stranger_or_while_offline_is_refused() {
    let (mut net, [a, ..]) = four(320);
    let stored = net.store[&ROOM].clone();
    assert_eq!(remove(&mut net, a, 9), Err(SessionError::NotAMember));
    net.set_link(a, false);
    net.run(6_000);
    assert_eq!(remove(&mut net, a, 2), Err(SessionError::Offline));
    net.set_link(a, true);
    net.run(15_000);
    assert_eq!(net.store[&ROOM], stored, "nothing written");
    assert_eq!(epoch(&net, a), 0);
    assert_eq!(participants(&net, a).len(), 4);
}

/// Criterion 32 with ADR 0010 "admin set never empty": the only admin cannot remove itself.
#[test]
fn t32_the_only_admin_cannot_remove_itself() {
    let (mut net, [a, b, ..]) = four(321);
    let result = remove(&mut net, a, 1);
    assert!(result.is_err(), "removing the last admin is refused");
    net.run(2_000);
    assert!(!net.clients[a].session().has_ended());
    assert_eq!(epoch(&net, a), 0);
    net.edit(a, |d| rect(d, 9.0, 9.0));
    assert!(net.run_until(3_000, |n| n.converged(&[a, b])).is_some());
}

/// Criterion 32 to 34: two removals in a row rotate twice (epoch 2); the remaining members keep
/// editing and converge, each removed device stops at its own last state.
#[test]
fn t32_two_removals_rotate_twice_and_the_rest_converge() {
    let (mut net, [a, b, c, d]) = four(322);
    remove(&mut net, a, 2).unwrap();
    net.run(2_000);
    let kept_b = net.clients[b].doc().object_ids().len();
    net.edit(c, |doc| rect(doc, 10.0, 0.0));
    net.run(1_000);
    remove(&mut net, a, 3).unwrap();
    net.run(2_000);
    net.edit(d, |doc| rect(doc, 20.0, 0.0));
    net.edit(a, |doc| rect(doc, 30.0, 0.0));
    assert!(net.run_until(5_000, |n| n.converged(&[a, d])).is_some());
    assert_eq!(epoch(&net, a), 2);
    assert_eq!(epoch(&net, d), 2);
    assert_eq!(participants(&net, a), participants(&net, d));
    assert_eq!(participants(&net, a).len(), 2);
    assert_eq!(net.clients[b].doc().object_ids().len(), kept_b);
    assert!(net.clients[c].session().has_ended());
    assert_eq!(net.clients[a].doc().object_ids().len(), 4);
    let entries: Vec<_> = net.store[&ROOM]
        .keyring
        .iter()
        .map(|e| curvyo_crypto_core::keyring::Entry::decode(e).unwrap())
        .collect();
    let epochs: Vec<u32> = entries
        .iter()
        .filter_map(|e| match e.payload {
            Payload::NewEpoch { epoch, .. } => Some(epoch),
            _ => None,
        })
        .collect();
    assert_eq!(epochs, [1, 2]);
}

/// Criterion 34 (and 3): an invite written after a rotation admits a newcomer, who receives the
/// whole document including edits made before the rotation and edits with the others.
#[test]
fn t34_a_newcomer_after_a_rotation_gets_the_whole_document() {
    let (mut net, [a, _b, c, _d]) = four(340);
    let early = net.edit(c, |doc| rect(doc, 40.0, 0.0));
    net.run(1_000);
    remove(&mut net, a, 2).unwrap();
    net.run(2_000);
    let secret = net.invite(a, 3_600);
    let e = net.add_client(5);
    net.join(e, secret);
    assert!(net.run_until(10_000, |n| n.converged(&[a, c, e])).is_some());
    assert!(net.clients[e].doc().object(early).is_some());
    let late = net.edit(e, |doc| rect(doc, 50.0, 0.0));
    assert!(net.run_until(3_000, |n| n.converged(&[a, c, e])).is_some());
    assert!(net.clients[c].doc().object(late).is_some());
    assert_eq!(epoch(&net, e), 1);
}

/// Criterion 34: an invite written before a rotation and used after it still admits the
/// newcomer with the current key.
#[test]
fn t34_an_invite_from_before_a_rotation_admits_with_the_new_key() {
    let (mut net, [a, _b, c, _d]) = four(341);
    let secret = net.invite(a, 3_600);
    net.run(500);
    remove(&mut net, a, 2).unwrap();
    net.run(2_000);
    let e = net.add_client(5);
    net.join(e, secret);
    assert!(net.run_until(10_000, |n| n.converged(&[a, c, e])).is_some());
    net.edit(e, |doc| rect(doc, 60.0, 0.0));
    assert!(net.run_until(3_000, |n| n.converged(&[a, c, e])).is_some());
    assert_eq!(epoch(&net, e), 1);
}

/// Criterion 33(c): a removed device whose app was frozen during the removal (socket still
/// open, nothing read) pushes an edit under the old key; it is not merged anywhere and no
/// remaining member shows an error.
#[test]
fn t33_a_push_from_a_frozen_removed_device_is_not_merged() {
    let (mut net, [a, b, c, _d]) = four(330);
    net.clients[b].frozen = true;
    remove(&mut net, a, 2).unwrap();
    net.run(2_000);
    let late = net.edit(b, |doc| rect(doc, 70.0, 0.0));
    net.run(3_000);
    net.clients[b].frozen = false;
    net.run(5_000);
    for i in [a, c] {
        assert!(net.clients[i].doc().object(late).is_none(), "not merged");
        assert_eq!(
            net.clients[i].count(|e| *e == SessionEvent::UpdateRejected),
            0
        );
        assert_eq!(net.clients[i].import_failures, 0);
    }
    // The rotation's snapshot pruned older records: everything stored now is sealed under
    // epoch 1, so B's late push under epoch 0 was not stored.
    for record in &net.store[&ROOM].records {
        assert_eq!(EnvelopeHeader::parse(&record.envelope).unwrap().epoch, 1);
    }
    assert!(
        net.clients[b].session().has_ended(),
        "B learns it was removed once it reads again"
    );
    assert!(
        net.clients[b].saw(&SessionEvent::Removed)
            || net.clients[b].saw(&SessionEvent::Refused(RefusalReason::Removed))
    );
}

/// Criterion 33(b): after a removal no record the relay stores past the removal is sealed under
/// the old epoch, and every push frame of a remaining member carries the new key id.
#[test]
fn t33_records_after_a_removal_carry_the_new_key_id() {
    let (mut net, [a, _b, c, d]) = four(331);
    remove(&mut net, a, 2).unwrap();
    net.run(2_000);
    let key_id = net.clients[a]
        .session()
        .keyring()
        .unwrap()
        .current_epoch()
        .key_id;
    let from: Vec<usize> = [a, c, d]
        .iter()
        .map(|&i| net.clients[i].sent.len())
        .collect();
    for (n, i) in [a, c, d].into_iter().enumerate() {
        net.edit(i, |doc| {
            rect(doc, 80.0 + f64::from(u8::try_from(n).unwrap()), 0.0)
        });
    }
    assert!(net.run_until(3_000, |n| n.converged(&[a, c, d])).is_some());
    for (k, i) in [a, c, d].into_iter().enumerate() {
        let pushes: Vec<_> = net.clients[i].sent[from[k]..]
            .iter()
            .filter_map(|m| match ClientFrame::decode(m).ok()? {
                ClientFrame::Push { envelope, .. } => Some(EnvelopeHeader::parse(&envelope).ok()?),
                _ => None,
            })
            .collect();
        assert_ne!(pushes.len(), 0);
        for header in pushes {
            assert_eq!(header.epoch, 1);
            assert_eq!(header.key_id, key_id);
        }
    }
}

// ------------------------------------------------------------------ criteria 17 to 20, 31

/// Criteria 18 and 31: while B is offline both make 50 edits and both resize the page to
/// different sizes; after the merge both hold all edits and one whole frame, and no object's
/// position read relative to the frame differs between them.
#[test]
fn t18_t31_offline_edits_and_concurrent_resizes_merge() {
    let (mut net, c) = shared(180, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    net.set_link(b, false);
    for n in 0..50u32 {
        net.edit(a, |d| rect(d, f64::from(n) * 6.0, 100.0));
        net.edit(b, |d| rect(d, f64::from(n) * 6.0, 200.0));
        if n == 25 {
            net.edit(a, |d| {
                d.resize(DocumentSize::from_mm(500.0, 300.0)).unwrap()
            });
            net.edit(b, |d| {
                d.resize(DocumentSize::from_mm(250.0, 600.0)).unwrap()
            });
        }
        net.run(50);
    }
    let (fa, fb) = (frame(&net, a), frame(&net, b));
    net.set_link(b, true);
    assert!(
        net.run_until(15_000, |n| n.clients[b].session().is_online())
            .is_some()
    );
    assert!(
        net.run_until(5_000, |n| n.converged(&[a, b])).is_some(),
        "merged within 5 s"
    );
    assert_eq!(net.clients[a].doc().object_ids().len(), 100);
    let merged = frame(&net, a);
    assert_eq!(merged, frame(&net, b));
    assert!(merged == fa || merged == fb, "{merged:?}");
}

/// Criterion 19: B closes the app while offline and reopens it while still offline; when the
/// network comes back the edits merge with no user action.
#[test]
fn t19_reopened_while_still_offline_merges_when_the_network_returns() {
    let (mut net, c) = shared(190, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    net.set_link(b, false);
    let mine = net.edit(b, |d| rect(d, 1.0, 1.0));
    net.close_app(b);
    net.reopen(b);
    let more = net.edit(b, |d| rect(d, 2.0, 2.0));
    net.run(10_000);
    assert!(net.clients[a].doc().object(mine).is_none());
    net.set_link(b, true);
    net.network_back(b);
    assert!(net.run_until(15_000, |n| n.converged(&[a, b])).is_some());
    assert!(net.clients[a].doc().object(mine).is_some());
    assert!(net.clients[a].doc().object(more).is_some());
}

/// Criterion 20: the relay restarts while B is offline holding unsent edits, and once more
/// shortly after; everyone reconnects within 30 s and nothing is lost.
#[test]
fn t20_two_restarts_with_an_offline_member_lose_nothing() {
    let (mut net, c) = shared(200, ROOM, Document::new(100), &[2, 3]);
    let (a, b, x) = (c[0], c[1], c[2]);
    net.set_link(b, false);
    let offline = net.edit(b, |d| rect(d, 3.0, 3.0));
    let online = net.edit(a, |d| rect(d, 4.0, 4.0));
    net.restart_relay();
    net.run(700);
    let between = net.edit(x, |d| rect(d, 5.0, 5.0));
    net.restart_relay();
    net.set_link(b, true);
    let took = net.run_until(30_000, |n| {
        [a, b, x]
            .iter()
            .all(|&i| n.clients[i].session().is_online())
            && n.converged(&[a, b, x])
    });
    assert!(took.is_some(), "reconnected and converged within 30 s");
    for id in [offline, online, between] {
        assert!(net.clients[a].doc().object(id).is_some());
    }
}

/// Criterion 20 with 3 and 38: after a restart a fresh invite still admits a newcomer who gets
/// every edit made before the restart.
#[test]
fn t20_a_join_after_a_restart_gets_everything() {
    let (mut net, c) = shared(201, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    let ids: Vec<_> = (0..5u32)
        .map(|n| net.edit(b, |d| rect(d, f64::from(n), 0.0)))
        .collect();
    net.run(500);
    net.restart_relay();
    assert!(
        net.run_until(30_000, |n| n.clients[a].session().is_online())
            .is_some()
    );
    let secret = net.invite(a, 3_600);
    let e = net.add_client(5);
    net.join(e, secret);
    assert!(net.run_until(10_000, |n| n.converged(&[a, b, e])).is_some());
    for id in ids {
        assert!(net.clients[e].doc().object(id).is_some());
    }
}

/// Criterion 17: B cut off reports Offline within 5 s and its presence disappears at A; B keeps
/// a full replica that still accepts edits and saves.
#[test]
fn t17_offline_reported_within_5_s_and_presence_disappears() {
    let (mut net, c) = shared(170, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    net.presence(b, b"here");
    net.run(300);
    assert!(
        net.clients[a]
            .events
            .iter()
            .any(|e| matches!(e, SessionEvent::Presence { .. }))
    );
    let seen = net.clients[b].events.len();
    net.set_link(b, false);
    let took = net.run_until(5_000, |n| {
        n.clients[b].events[seen..].contains(&SessionEvent::Offline)
    });
    assert!(took.is_some(), "offline within 5 s");
    assert!(
        net.run_until(5_000, |n| n.clients[a].saw_gone_after_presence())
            .is_some()
    );
    let id = net.edit(b, |d| rect(d, 6.0, 6.0));
    assert!(net.clients[b].doc().object(id).is_some());
    net.close_app(b);
    let file = net.clients[b].file.clone().unwrap();
    let opened = curvyo_document_core::unpack(77, &file).unwrap();
    assert!(opened.object(id).is_some(), "saved offline");
    assert!(opened.share().is_some());
}

/// Criterion 21: after a merge, B's undo stack is unchanged by A's edits and A's undo, and A's
/// undo leaves B's concurrent edit in place.
#[test]
fn t21_undo_with_a_peer_editing_the_same_object() {
    let doc = Document::new(100);
    let id = rect(&doc, 0.0, 0.0);
    let (mut net, c) = shared(210, ROOM, doc, &[2]);
    let (a, b) = (c[0], c[1]);
    net.edit(a, |d| {
        d.translate_objects(&[id], Vec2::new(10.0, 0.0)).unwrap();
    });
    assert!(net.run_until(2_000, |n| n.converged(&[a, b])).is_some());
    let other = net.edit(b, |d| rect(d, 30.0, 30.0));
    assert!(net.run_until(2_000, |n| n.converged(&[a, b])).is_some());
    let b_steps = net.clients[b].doc().own_steps();
    net.edit(a, |d| {
        let mine = d.own_steps().last().unwrap().step;
        d.undo_step(mine).unwrap();
    });
    assert!(net.run_until(2_000, |n| n.converged(&[a, b])).is_some());
    assert!(net.clients[a].doc().object(other).is_some());
    assert_eq!(net.clients[b].doc().own_steps(), b_steps);
}

/// Criterion 6 under a lossy, slow simulated LAN: 100 alternating edits from A and B all arrive
/// and both converge.
#[test]
fn t6_hundred_alternating_edits_converge_under_latency() {
    let (mut net, c) = shared(60, ROOM, Document::new(100), &[2]);
    let (a, b) = (c[0], c[1]);
    net.max_latency_ms = 120;
    for n in 0..50u32 {
        net.edit(a, |d| rect(d, f64::from(n), 10.0));
        net.run(10);
        net.edit(b, |d| rect(d, f64::from(n), 20.0));
        net.run(10);
    }
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    assert_eq!(net.clients[a].doc().object_ids().len(), 100);
}
