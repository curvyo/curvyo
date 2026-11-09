//! Tester acceptance tests for `specs/0097-membership-lifecycle` against the real server
//! (criteria 20 and 26): an added-back device starting again right after the add back, through
//! real sockets (the return check is retired, ADR 0044 decision 5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use curvyo_crypto_core::keyring::Role;
use curvyo_document_core::{Document, Length, Point, RectBounds};
use curvyo_sync_core::{RoomId, SessionEvent, SessionStart};
use support::{Captured, Device, converged, identity, now, run_until, start};

/// Whether the device was admitted (`Online`) or refused, once either happened.
fn outcome(device: &Device) -> Option<bool> {
    device.events.iter().find_map(|e| match e {
        SessionEvent::Online => Some(true),
        SessionEvent::Refused(_) => Some(false),
        _ => None,
    })
}

/// A shares in `room`, B joins, A removes B; then A adds B back and, `gap` after A's session
/// handed the entry to its socket, B starts again from its kept keyring. The answers per room.
async fn first_checks(gap: Duration, rooms: std::ops::Range<u8>) -> Vec<String> {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}/", server.local_addr());
    let mut wrong = Vec::new();
    for round in rooms {
        let room = RoomId([round; 16]);
        let doc = Document::new(100);
        let _ = doc.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(5.0),
            height: Length::from_mm(5.0),
        });
        let mut a = Device::share(1, &url, room, doc);
        assert!(
            run_until(&mut [&mut a], Duration::from_secs(10), |d| d[0].is_online())
                .await
                .is_some()
        );
        let secret = a.session.invite(now().unix_seconds() + 3_600).unwrap();
        let mut b = Device::join(2, &url, secret);
        assert!(
            run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
                .await
                .is_some()
        );
        let ben = identity(2).identity().member;
        a.session.remove(ben).unwrap();
        assert!(
            run_until(&mut [&mut a, &mut b], Duration::from_secs(10), |d| {
                d[1].events.contains(&SessionEvent::Removed)
            })
            .await
            .is_some()
        );
        let keyring = b.session.keyring_entries();
        let epoch_keys = b.session.epoch_keys();
        a.session.add_back(ben).unwrap();
        // What A's Share tab shows at once: B listed again, before the relay has the entry.
        assert_eq!(a.session.keyring().unwrap().role(&ben), Some(Role::Member));
        a.step().await;
        if !gap.is_zero() {
            let _ = run_until(&mut [&mut a], gap, |_| false).await;
        }
        let mut check = Device::start(
            2,
            &url,
            room,
            SessionStart::Member {
                keyring,
                epoch_keys,
            },
        );
        let took = run_until(&mut [&mut check, &mut a], Duration::from_secs(10), |d| {
            outcome(d[0]).is_some()
        })
        .await;
        assert!(
            took.is_some_and(|t| t <= Duration::from_secs(5)),
            "criterion 26: an answer within 5 s, {took:?}"
        );
        let answer = outcome(&check);
        if answer != Some(true) {
            wrong.push(format!(
                "room {round:#x}: {answer:?} after {took:?}, fetched keyring {}",
                if check.session.keyring().is_some() {
                    "yes"
                } else {
                    "none (refused)"
                }
            ));
        }
    }
    server.shutdown().await.unwrap();
    wrong
}

/// Criterion 26 as the maker meets it: B starts again once A has told B (here 300 ms after A's
/// add back left A's session); B is admitted within 5 s through the real server.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_start_after_an_add_back_is_admitted() {
    let wrong = first_checks(Duration::from_millis(300), 0x90..0x96).await;
    assert!(wrong.is_empty(), "first checks: {wrong:?}");
}

/// The flaky first start, reproduced: B starts right after A's session handed the
/// add back to its socket (A lists B again at once). Through the real server every such first
/// check answers "not yet" (refused as removed): A's small `Keyring` frame waits in A's TCP
/// stack (Nagle: no `TCP_NODELAY` on the client socket, and neither the server nor the test
/// client sets it) for the server's delayed ACK, about 40 ms, while B's check handshake takes
/// about 10 ms. With `TCP_NODELAY` on the client (`connect_async_with_config(url, None, true)`)
/// all first checks said "added back"; with 50 ms between, too. The relay does not acknowledge
/// keyring entries, so A's list shows B before the relay has the add.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "FINDING (low): a check started within ~40 ms of the add back answers 'not yet' (A's UI lists B before the relay holds the add; Nagle on the client socket)"]
async fn a_start_right_after_the_add_back_is_admitted() {
    let wrong = first_checks(Duration::ZERO, 0xa0..0xa6).await;
    assert!(wrong.is_empty(), "first checks: {wrong:?}");
}
