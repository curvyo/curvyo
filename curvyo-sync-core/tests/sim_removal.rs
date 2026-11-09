//! Removing a participant with key rotation through the simulator with real document replicas
//! (`specs/0092` criteria 32 to 34, ADR 0031 decision 6a).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

mod support;

use curvyo_crypto_core::EnvelopeHeader;
use curvyo_crypto_core::keyring::{Payload, decode_log};
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{ClientFrame, RefusalReason, SessionError, SessionEvent};
use support::sim::{Net, shared};
use support::{ROOM, member};

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// A (admin, device 1), B (device 2), C (device 3), D (device 4), all converged.
fn four(seed: u64) -> (Net, [usize; 4]) {
    let doc = Document::new(100);
    rect(&doc, 0.0);
    let (net, c) = shared(seed, ROOM, doc, &[2, 3, 4]);
    (net, [c[0], c[1], c[2], c[3]])
}

fn remove(net: &mut Net, admin: usize, who: u8) {
    net.clients[admin]
        .session
        .as_mut()
        .unwrap()
        .remove(member(who))
        .unwrap();
    net.pump(admin);
}

/// Every envelope pushed to the relay after `from` (by message index) of client `i`.
fn pushed_epochs(net: &Net, i: usize, from: usize) -> Vec<u32> {
    net.clients[i].sent[from..]
        .iter()
        .filter_map(|m| match ClientFrame::decode(m).ok()? {
            ClientFrame::Push { envelope, .. } => {
                Some(EnvelopeHeader::parse(&envelope).ok()?.epoch)
            }
            _ => None,
        })
        .collect()
}

/// Criterion 32: Remove writes `RemoveMember`, then `NewEpoch` (highest + 1) wrapped to every
/// remaining participant and not to B, and uploads a signed snapshot under the new epoch; a
/// member who is not an admin cannot remove.
#[test]
fn removal_writes_remove_then_rotation_then_a_snapshot() {
    let (mut net, [a, b, c, _]) = four(1);
    let not_admin = net.clients[c].session.as_mut().unwrap().remove(member(2));
    assert_eq!(not_admin, Err(SessionError::NotAdmin));
    remove(&mut net, a, 2);
    net.run(2_000);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert_eq!(keyring.current_epoch().number, 1);
    assert!(
        keyring
            .participants()
            .iter()
            .all(|p| p.identity.member != member(2))
    );
    assert!(!keyring.holds(&member(2), keyring.current_epoch()));
    assert!(keyring.holds(&member(3), keyring.current_epoch()));
    let stored = &net.store[&ROOM];
    let entries: Vec<_> = stored
        .keyring
        .iter()
        .map(|b| curvyo_crypto_core::keyring::Entry::decode(b).unwrap())
        .collect();
    let n = entries.len();
    assert!(
        matches!(entries[n - 2].payload, Payload::RemoveMember { member: m } if m == member(2))
    );
    assert!(matches!(
        entries[n - 1].payload,
        Payload::NewEpoch { epoch: 1, .. }
    ));
    let (_, snapshot) = stored.snapshot.as_ref().unwrap();
    let header = EnvelopeHeader::parse(snapshot).unwrap();
    assert!(header.signed && header.epoch == 1);
    assert_eq!(header.key_id, keyring.current_epoch().key_id);
    assert!(net.clients[b].saw(&SessionEvent::Removed));
}

/// Criterion 33: the relay closes B with "removed" and refuses B later; every envelope after the
/// removal carries the new key id and B receives none; B keeps its copy and stops syncing.
#[test]
fn a_removed_device_is_closed_and_receives_nothing_new() {
    let (mut net, [a, b, c, _]) = four(2);
    remove(&mut net, a, 2);
    net.run(2_000);
    let removed = &net.clients[b];
    assert!(
        removed.saw(&SessionEvent::Refused(RefusalReason::Removed))
            || removed.saw(&SessionEvent::Removed)
    );
    assert!(removed.session().has_ended());
    let removed_at = removed.events.len();
    let objects_b = removed.doc().object_ids().len();
    let sent_a = net.clients[a].sent.len();
    let sent_c = net.clients[c].sent.len();
    net.edit(a, |d| rect(d, 10.0));
    net.edit(c, |d| rect(d, 20.0));
    net.run(2_000);
    assert!(pushed_epochs(&net, a, sent_a).iter().all(|&e| e == 1));
    assert!(pushed_epochs(&net, c, sent_c).iter().all(|&e| e == 1));
    let removed = &net.clients[b];
    assert_eq!(removed.events.len(), removed_at, "nothing reaches B");
    assert_eq!(
        removed.doc().object_ids().len(),
        objects_b,
        "B keeps its copy"
    );
    // B can push nothing more, and a reopened B is refused.
    assert_eq!(
        net.clients[b]
            .session
            .as_mut()
            .unwrap()
            .push(curvyo_sync_core::RecordKind::Update, vec![1]),
        Err(SessionError::Ended)
    );
    net.close_app(b);
    net.reopen(b);
    net.run(2_000);
    let reopened = &net.clients[b];
    assert!(reopened.events[removed_at..].contains(&SessionEvent::Refused(RefusalReason::Removed)));
    assert!(net.converged(&[a, c]));
    assert_eq!(
        net.clients[a].count(|e| *e == SessionEvent::UpdateRejected),
        0
    );
}

/// Criterion 34: B removed while offline learns it on reconnect and its offline edits are not
/// merged; C, offline during the rotation, sends its earlier edits under the new epoch; D,
/// online throughout, keeps editing with A and loses nothing.
#[test]
fn removal_while_offline_and_rotation_across_members() {
    let (mut net, [a, b, c, d]) = four(3);
    net.set_link(b, false);
    net.set_link(c, false);
    let lost = net.edit(b, |doc| rect(doc, 30.0));
    let kept = net.edit(c, |doc| rect(doc, 40.0));
    net.edit(d, |doc| rect(doc, 50.0));
    net.run(1_000);
    remove(&mut net, a, 2);
    let during = net.edit(d, |doc| rect(doc, 60.0));
    net.run(2_000);
    let sent_c = net.clients[c].sent.len();
    net.set_link(b, true);
    net.set_link(c, true);
    assert!(
        net.run_until(30_000, |n| n.converged(&[a, c, d])
            && n.clients[b].session().has_ended())
            .is_some()
    );
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    assert!(net.clients[b].session().has_ended());
    for i in [a, c, d] {
        let doc = net.clients[i].doc();
        assert!(doc.object(lost).is_none(), "B's offline edit is not merged");
        assert!(doc.object(kept).is_some(), "C's offline edit arrives");
        assert!(doc.object(during).is_some(), "D's edit across the rotation");
    }
    assert!(pushed_epochs(&net, c, sent_c).contains(&1));
    assert!(pushed_epochs(&net, c, sent_c).iter().all(|&e| e == 1));
    let keyring = net.clients[c].session().keyring().unwrap();
    assert_eq!(keyring.current_epoch().number, 1);
    // Older records stay readable to members who hold the old key.
    assert!(net.clients[c].epoch_keys.iter().any(|(e, _)| *e == 0));
    for i in [a, c, d] {
        assert_eq!(
            net.clients[i].count(|e| *e == SessionEvent::UpdateRejected),
            0
        );
        assert_eq!(net.clients[i].import_failures, 0);
    }
    // The file B keeps still opens as its last synced state; its keyring shows the removal
    // only once it learned it, and it no longer syncs.
    net.close_app(b);
    let file = net.clients[b].file.clone().unwrap();
    let opened = curvyo_document_core::unpack(9, &file).unwrap();
    let log = decode_log(&opened.share().unwrap().keyring_log).unwrap();
    assert_ne!(log, [] as [curvyo_crypto_core::keyring::Entry; 0]);
}

/// Share tab feedback (customer 2026-11-09): a removed device joins again by a fresh invite and
/// syncs; its old link is still refused as removed.
#[test]
fn a_removed_device_joins_again_by_a_new_invite() {
    let (mut net, [a, b, c, _]) = four(5);
    remove(&mut net, a, 2);
    net.run(2_000);
    assert!(net.clients[b].session().has_ended());
    net.close_app(b);
    net.reopen(b);
    net.run(2_000);
    let refused_at = net.clients[b].events.len();
    assert!(
        net.clients[b].events[..refused_at]
            .contains(&SessionEvent::Refused(RefusalReason::Removed))
    );
    let secret = net.invite(a, 24 * 3600);
    net.set_link(b, false);
    net.set_link(b, true);
    net.join(b, secret);
    let rejoined = |n: &Net| {
        n.clients[a].session().keyring().is_some_and(|k| {
            k.participants()
                .iter()
                .any(|p| p.identity.member == member(2))
        })
    };
    assert!(
        net.run_until(10_000, |n| rejoined(n) && n.converged(&[a, b, c]))
            .is_some()
    );
    assert!(!net.clients[b].session().has_ended());
    net.edit(b, |d| rect(d, 70.0));
    assert!(net.run_until(3_000, |n| n.converged(&[a, b, c])).is_some());
}
