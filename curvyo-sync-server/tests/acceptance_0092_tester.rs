//! Tester's acceptance tests for `specs/0092-cooperative-editing` over real `WebSocket`s against
//! the in-process server: invitation state survives a restart (criteria 4, 38), hostile bytes
//! do not disturb a room (criteria 20, 37), `/healthz` reveals no room (criterion 37), and two
//! rooms of one maker stay separate on disk (criterion 40). Written from the specification
//! before reading the implementation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{RefusalReason, RoomId, SessionEvent};
use curvyo_sync_server::room_hex;
use futures_util::SinkExt;
use support::{Captured, Device, converged, http_get, now, run_until, start};
use tokio_tungstenite::tungstenite::Message;

const ROOM: RoomId = RoomId([0x7a; 16]);
const OTHER: RoomId = RoomId([0x7b; 16]);

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

async fn online(device: &mut Device) {
    assert!(
        run_until(&mut [device], Duration::from_secs(10), |d| d[0].is_online())
            .await
            .is_some()
    );
}

/// Criteria 4 and 38: an invite claimed before a restart is refused as "used" after the server
/// restarts on the same data directory; the refused device sees no content.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t4_t38_a_used_invite_stays_used_across_a_server_restart() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let addr = server.local_addr();
    let url = format!("ws://{addr}/");
    let doc = Document::new(100);
    rect(&doc, 0.0);
    let mut a = Device::share(1, &url, ROOM, doc);
    online(&mut a).await;
    let secret = a.session.invite(now().unix_seconds() + 3_600).unwrap();
    let mut b = Device::join(2, &url, secret.clone());
    assert!(
        run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
            .await
            .is_some()
    );
    server.shutdown().await.unwrap();
    let server = start(addr, data.path(), &log).await;
    let mut c = Device::join(3, &url, secret);
    let refused = run_until(&mut [&mut c], Duration::from_secs(10), |d| {
        d[0].events
            .iter()
            .any(|e| matches!(e, SessionEvent::Refused(_)))
    })
    .await;
    assert!(refused.is_some(), "C is refused: {:?}", c.events);
    assert!(
        c.events
            .contains(&SessionEvent::Refused(RefusalReason::Used))
    );
    assert!(c.doc.is_none());
    assert!(
        !c.events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. }))
    );
    server.shutdown().await.unwrap();
}

/// Criteria 20 and 37: garbage, an empty message, text frames and a truncated frame from a
/// hostile socket neither stop the server nor disturb a room; `/healthz` stays 200 and the
/// members keep editing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t37_hostile_bytes_do_not_disturb_a_room() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let addr = server.local_addr();
    let url = format!("ws://{addr}/");
    let mut a = Device::share(1, &url, ROOM, Document::new(100));
    online(&mut a).await;
    let secret = a.session.invite(now().unix_seconds() + 3_600).unwrap();
    let mut b = Device::join(2, &url, secret);
    assert!(
        run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
            .await
            .is_some()
    );
    for payload in [
        Message::Binary(vec![0xff; 64].into()),
        Message::Binary(Vec::new().into()),
        Message::Binary(vec![1, 1].into()),
        Message::Text("hello".into()),
        Message::Binary([&[1u8, 1, 0][..], &ROOM.0[..]].concat().into()),
    ] {
        let (mut hostile, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let _ = hostile.send(payload).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(hostile);
    }
    assert_eq!(http_get(addr, "/healthz").await.0, 200);
    b.edit(|d| rect(d, 10.0));
    a.edit(|d| rect(d, 20.0));
    assert!(
        run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
            .await
            .is_some()
    );
    assert_eq!(a.doc().object_ids().len(), 2);
    server.shutdown().await.unwrap();
}

/// Criteria 37 and 40: `/healthz` names no room, and two shared documents of one maker live in
/// two separate room directories with separate participant lists.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t40_two_rooms_stay_separate_and_healthz_names_none() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let addr = server.local_addr();
    let url = format!("ws://{addr}/");
    let mut first = Device::share(1, &url, ROOM, Document::new(100));
    let mut second = Device::share(1, &url, OTHER, Document::new(101));
    online(&mut first).await;
    online(&mut second).await;
    let secret = first.session.invite(now().unix_seconds() + 3_600).unwrap();
    let mut b = Device::join(2, &url, secret);
    assert!(
        run_until(
            &mut [&mut first, &mut b],
            Duration::from_secs(10),
            converged
        )
        .await
        .is_some()
    );
    first.edit(|d| rect(d, 1.0));
    assert!(
        run_until(
            &mut [&mut first, &mut b, &mut second],
            Duration::from_secs(5),
            |d| converged(&d[..2])
        )
        .await
        .is_some()
    );
    assert!(second.doc().object_ids().is_empty(), "no cross-talk");
    assert_eq!(first.session.keyring().unwrap().participants().len(), 2);
    assert_eq!(second.session.keyring().unwrap().participants().len(), 1);
    assert!(data.path().join(room_hex(ROOM)).is_dir());
    assert!(data.path().join(room_hex(OTHER)).is_dir());
    let (status, body) = http_get(addr, "/healthz").await;
    assert_eq!(status, 200);
    for room in [ROOM, OTHER] {
        assert!(!body.contains(&room_hex(room)));
    }
    server.shutdown().await.unwrap();
}
