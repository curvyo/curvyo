//! The client session's timing and bookkeeping on a virtual clock (adrs.md decision 8: ping
//! 2 s, offline after 4 s, reconnect 0.5 s doubling to 10 s ±20 %, presence 50 ms / 2 s / 5 s,
//! snapshot after 500 records; `specs/0092` criteria 14, 17, 20).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod support;

use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{ClientFrame, SNAPSHOT_RECORDS, SessionEvent};
use support::ROOM;
use support::sim::{Net, shared};

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

fn pair(seed: u64) -> (Net, usize, usize) {
    let (net, c) = shared(seed, ROOM, Document::new(100), &[2]);
    (net, c[0], c[1])
}

fn frames_of(net: &Net, i: usize, from: usize, tag: u8) -> usize {
    net.clients[i].sent[from..]
        .iter()
        .filter(|m| m.first() == Some(&tag))
        .count()
}

#[test]
fn an_idle_connection_pings_every_2_s() {
    let (mut net, a, _) = pair(1);
    let from = net.clients[a].sent.len();
    net.run(10_000);
    let pings = frames_of(&net, a, from, 0x07);
    assert!((4..=6).contains(&pings), "{pings} pings in 10 s");
    assert!(net.clients[a].session().is_online());
}

#[test]
fn reconnects_back_off_from_half_a_second_to_ten_with_jitter() {
    let (mut net, _, b) = pair(2);
    net.set_link(b, false);
    net.run(70_000);
    let times = &net.clients[b].connect_times;
    assert!(times.len() >= 8, "{times:?}");
    let gaps: Vec<u64> = times.windows(2).map(|w| w[1].since(w[0])).collect();
    // `times[0]` is the join, `times[1]` the first retry 0.5 s after the loss was noticed; each
    // failed attempt then doubles the delay up to 10 s.
    let expected = [1_000u64, 2_000, 4_000, 8_000, 10_000, 10_000];
    for (gap, base) in gaps[1..].iter().zip(expected) {
        let low = base * 4 / 5;
        let high = base * 6 / 5 + 10;
        assert!((low..=high).contains(gap), "gap {gap} for {base}");
    }
    // Back online: one attempt brings it back and the backoff starts again.
    net.set_link(b, true);
    assert!(
        net.run_until(12_500, |n| n.clients[b].session().is_online())
            .is_some()
    );
}

/// Criterion 18 after a long outage: the backoff has grown to 10 s, but "network is back"
/// reconnects at once, so the merge does not wait for the next retry.
#[test]
fn network_back_reconnects_at_once_after_a_long_outage() {
    let (mut net, a, b) = pair(7);
    net.set_link(b, false);
    net.run(40_000);
    net.edit(b, |d| rect(d, 30.0));
    net.set_link(b, true);
    let attempts = net.clients[b].connect_times.len();
    net.network_back(b);
    assert_eq!(net.clients[b].connect_times.len(), attempts + 1);
    let took = net
        .run_until(5_000, |n| {
            n.converged(&[a, b]) && n.clients[b].session().is_online()
        })
        .expect("merged");
    assert!(took < 1_000, "merged {took} ms after the network came back");
}

/// While connected, "network is back" changes nothing: no second connection.
#[test]
fn network_back_while_connected_does_nothing() {
    let (mut net, a, _) = pair(8);
    let attempts = net.clients[a].connect_times.len();
    net.network_back(a);
    net.run(500);
    assert_eq!(net.clients[a].connect_times.len(), attempts);
    assert!(net.clients[a].session().is_online());
}

#[test]
fn presence_is_throttled_to_50_ms_and_beats_every_2_s() {
    let (mut net, a, b) = pair(3);
    let from = net.clients[a].sent.len();
    for n in 0u32..100 {
        net.presence(a, &n.to_le_bytes());
        net.run(10);
    }
    let changing = frames_of(&net, a, from, 0x06);
    assert!(
        (19..=22).contains(&changing),
        "{changing} presence frames in 1 s"
    );
    let from = net.clients[a].sent.len();
    net.run(10_000);
    let beats = frames_of(&net, a, from, 0x06);
    assert!((4..=6).contains(&beats), "{beats} heartbeats in 10 s");
    let seen = net.clients[b].count(|e| matches!(e, SessionEvent::Presence { .. }));
    assert!(seen >= changing + beats - 2);
}

/// Criterion 14: a peer that falls silent disappears within 5 s (here: after 5 s of silence).
#[test]
fn a_silent_peer_disappears_after_5_s() {
    let (mut net, a, b) = pair(4);
    net.presence(b, b"here");
    net.run(100);
    let shown = net.clients[a].count(|e| matches!(e, SessionEvent::Presence { .. }));
    assert!(shown > 0);
    net.clients[b].frozen = true;
    let took = net
        .run_until(8_000, |n| {
            n.clients[a]
                .events
                .iter()
                .any(|e| matches!(e, SessionEvent::PresenceGone { .. }))
        })
        .unwrap();
    assert!((4_800..=5_200).contains(&took), "gone after {took} ms");
}

/// Decision 10: past 500 records since the snapshot a client uploads a new one and the relay
/// prunes the log up to it.
#[test]
fn a_snapshot_follows_500_records_and_prunes_the_log() {
    let (mut net, a, b) = pair(5);
    for n in 0..=SNAPSHOT_RECORDS {
        net.edit(a, |d| rect(d, (n % 40) as f64 * 6.0));
        if n % 50 == 0 {
            net.run(100);
        }
    }
    net.run(2_000);
    let due = net.clients[a].count(|e| matches!(e, SessionEvent::SnapshotDue { .. }))
        + net.clients[b].count(|e| matches!(e, SessionEvent::SnapshotDue { .. }));
    assert!(due >= 2, "the share's snapshot and one after 500 records");
    let stored = &net.store[&ROOM];
    let covers = stored.snapshot.as_ref().unwrap().0;
    assert!(covers > SNAPSHOT_RECORDS);
    assert!(stored.records.iter().all(|r| r.seq > covers));
    assert!(net.converged(&[a, b]));
    // A newcomer starts from that snapshot.
    let secret = net.invite(a, 3_600);
    let c = net.add_client(3);
    net.join(c, secret);
    assert!(net.run_until(5_000, |n| n.converged(&[a, b, c])).is_some());
}

/// Pushes carry increasing client ids; each is acknowledged once with its relay sequence, and
/// edits queued offline go out after the reconnect.
#[test]
fn pushes_are_acknowledged_and_queued_ones_resent() {
    let (mut net, a, b) = pair(6);
    net.set_link(a, false);
    for n in 0..3 {
        net.edit(a, |d| rect(d, f64::from(n) * 6.0));
    }
    net.run(1_000);
    net.set_link(a, true);
    assert!(net.run_until(15_000, |n| n.converged(&[a, b])).is_some());
    net.run(500);
    let acks: Vec<(u64, u64)> = net.clients[a]
        .events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Acked { client_id, seq } => Some((*client_id, *seq)),
            _ => None,
        })
        .collect();
    assert_eq!(acks.iter().map(|a| a.0).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert!(
        acks.windows(2).all(|w| w[1].1 == w[0].1 + 1),
        "gapless {acks:?}"
    );
    let pushes: Vec<u64> = net.clients[a]
        .sent
        .iter()
        .filter_map(|m| match ClientFrame::decode(m).ok()? {
            ClientFrame::Push { client_id, .. } => Some(client_id),
            _ => None,
        })
        .collect();
    assert_eq!(pushes, vec![1, 2, 3]);
    assert_eq!(
        net.clients[a].session().cursor(),
        net.relay.room(&ROOM).unwrap().head_seq()
    );
}
