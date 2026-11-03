//! Offline work, reopening, relay restarts and rejected records through the simulator with real
//! document replicas (`specs/0092` criteria 17 to 20, 25).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::EnvelopeHeader;
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{RecordKind, SessionEvent};
use support::ROOM;
use support::sim::{Net, shared};

fn rect(doc: &Document, x: f64, y: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// Whether the room's snapshot arrived after the client's resync started.
fn snapshot_after_resync(client: &support::sim::Client) -> bool {
    client
        .events
        .iter()
        .skip_while(|e| **e != SessionEvent::Resyncing)
        .any(|e| {
            matches!(
                e,
                SessionEvent::Remote {
                    kind: RecordKind::Snapshot,
                    ..
                }
            )
        })
}

fn pair(seed: u64) -> (Net, usize, usize) {
    let doc = Document::new(100);
    rect(&doc, 0.0, 0.0);
    let (net, c) = shared(seed, ROOM, doc, &[2]);
    (net, c[0], c[1])
}

/// Criterion 17: a cut connection shows "Offline" within 5 s, editing goes on, and the other
/// side sees the presence disappear.
#[test]
fn a_cut_connection_goes_offline_within_5_s_and_keeps_editing() {
    let (mut net, a, b) = pair(1);
    net.presence(b, b"pointer");
    net.run(200);
    assert!(net.clients[a].count(|e| matches!(e, SessionEvent::Presence { .. })) > 0);
    net.set_link(b, false);
    let took = net.run_until(5_000, |n| n.clients[b].saw(&SessionEvent::Offline));
    assert!(took.is_some(), "offline within 5 s");
    assert!(net.clients[a].saw_gone_after_presence());
    net.edit(b, |d| rect(d, 10.0, 0.0));
    assert_eq!(net.clients[b].doc().object_ids().len(), 2);
}

/// Criterion 18: 50 edits on each side while B is offline; once B is connected again both hold
/// all 100 within 5 s, and both are at one version with identical content (the SVG of a version
/// is a function of it; Proposal: compared as the document JSON).
#[test]
fn fifty_plus_fifty_offline_edits_merge() {
    let (mut net, a, b) = pair(2);
    net.set_link(b, false);
    for n in 0..50 {
        net.edit(a, |d| rect(d, f64::from(n) * 6.0, 100.0));
        net.edit(b, |d| rect(d, f64::from(n) * 6.0, 200.0));
        net.run(100);
    }
    net.set_link(b, true);
    assert!(
        net.run_until(15_000, |n| n.clients[b].session().is_online())
            .is_some()
    );
    let took = net.run_until(5_000, |n| n.converged(&[a, b]));
    assert!(took.is_some(), "merged within 5 s of reconnecting");
    assert_eq!(net.clients[a].doc().object_ids().len(), 101);
    assert_eq!(
        net.clients[a].doc().version().to_text(),
        net.clients[b].doc().version().to_text()
    );
}

/// Criterion 19: B edits offline, saves and closes; reopened with the network back, the edits
/// merge with no user action.
#[test]
fn edits_saved_offline_merge_after_reopening() {
    let (mut net, a, b) = pair(3);
    net.set_link(b, false);
    for n in 0..5 {
        net.edit(b, |d| rect(d, f64::from(n) * 6.0, 50.0));
    }
    net.edit(a, |d| rect(d, 0.0, 80.0));
    net.close_app(b);
    net.run(3_000);
    net.set_link(b, true);
    net.reopen(b);
    assert!(net.run_until(10_000, |n| n.converged(&[a, b])).is_some());
    assert_eq!(net.clients[a].doc().object_ids().len(), 7);
}

/// Criterion 20: the relay restarts while both are connected; both reconnect within 30 s, no
/// edit is lost, and the stored data only gained new sealed entries.
#[test]
fn a_relay_restart_loses_nothing() {
    let (mut net, a, b) = pair(4);
    net.edit(a, |d| rect(d, 10.0, 0.0));
    net.run(500);
    let before = net.store[&ROOM].clone();
    net.edit(a, |d| rect(d, 20.0, 0.0));
    net.edit(b, |d| rect(d, 30.0, 0.0));
    net.restart_relay();
    let took = net.run_until(30_000, |n| {
        n.clients[a].session().is_online() && n.clients[b].session().is_online()
    });
    assert!(took.is_some(), "reconnected within 30 s");
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    assert_eq!(net.clients[a].doc().object_ids().len(), 4);
    let after = &net.store[&ROOM];
    assert_eq!(after.keyring, before.keyring);
    assert_eq!(after.snapshot, before.snapshot);
    assert_eq!(&after.records[..before.records.len()], &before.records[..]);
}

/// Criterion 25: a stored record altered by one byte, or moved from another room or epoch, is
/// refused, not merged, and reported once; the session resyncs from the room's snapshot once
/// and does not loop on the records that stay damaged.
#[test]
fn altered_or_moved_records_are_refused_once() {
    let (mut net, a, b) = pair(5);
    net.set_link(b, false);
    let mut ids = Vec::new();
    for n in 0..3 {
        ids.push(net.edit(a, |d| rect(d, f64::from(n) * 6.0, 30.0)));
    }
    net.run(500);
    let records = &mut net.store.get_mut(&ROOM).unwrap().records;
    let count = records.len();
    // One flipped ciphertext byte, a header naming another epoch, another room's document id.
    let last = records[count - 1].envelope.len() - 1;
    records[count - 1].envelope[last] ^= 1;
    records[count - 2].envelope[3] ^= 1;
    records[count - 3].envelope[20] ^= 1;
    assert!(EnvelopeHeader::parse(&records[count - 2].envelope).is_ok());
    net.restart_relay();
    net.set_link(b, true);
    net.run(15_000);
    let client = &net.clients[b];
    assert_eq!(client.count(|e| *e == SessionEvent::UpdateRejected), 1);
    assert_eq!(client.count(|e| *e == SessionEvent::Resyncing), 1);
    assert!(snapshot_after_resync(client), "the snapshot arrived again");
    for id in ids {
        assert!(client.doc().object(id).is_none(), "nothing merged");
    }
    assert!(client.session().is_online(), "no crash, still connected");
    assert_eq!(client.import_failures, 0);
}

/// Criterion 25 on the caller's side: a record that opened but did not import is reported by
/// the caller; the session shows it once and reconnects from cursor 0, so the room's snapshot
/// arrives again and repairs the replica.
#[test]
fn a_record_the_caller_could_not_import_resyncs_from_the_snapshot() {
    let (mut net, a, b) = pair(6);
    net.edit(a, |d| rect(d, 20.0, 0.0));
    net.run(500);
    let connects = net.clients[b].connect_times.len();
    let seq = net.clients[b].session().cursor();
    net.clients[b]
        .session
        .as_mut()
        .unwrap()
        .record_rejected(seq);
    net.pump(b);
    net.run(3_000);
    let client = &net.clients[b];
    assert_eq!(client.count(|e| *e == SessionEvent::UpdateRejected), 1);
    assert_eq!(client.count(|e| *e == SessionEvent::Resyncing), 1);
    assert_eq!(client.connect_times.len(), connects + 1, "reconnected once");
    assert!(snapshot_after_resync(client));
    assert!(client.session().is_online());
    assert!(net.converged(&[a, b]));
    // The same record again starts no second resync.
    net.clients[b]
        .session
        .as_mut()
        .unwrap()
        .record_rejected(seq);
    net.pump(b);
    net.run(1_000);
    assert_eq!(net.clients[b].count(|e| *e == SessionEvent::Resyncing), 1);
}
