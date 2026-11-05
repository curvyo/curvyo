//! Tester acceptance tests for `specs/0096-own-devices-sync` against the in-process server,
//! written from the specification and ADR 0040 decisions 3, 10 and 13 before the implementation
//! was read: pairing slots never survive a restart (criteria 4 and 30), a garbage first frame
//! does not stop the server, and passkey backups over real sockets: write needs the signed
//! challenge, nobody else replaces a record, the 4 KiB cap, and nothing readable in the data
//! directory or the log (criteria 34 and 36).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use std::path::Path;
use std::time::Duration;

use curvyo_crypto_core::{BackupContent, BackupKeys, BackupOp, DocumentId};
use curvyo_sync_core::{BackupExchange, BackupOutcome, ClientFrame, RefusalReason, RelayFrame};
use futures_util::{SinkExt, StreamExt};
use support::{Captured, identity, start};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const ROOM: DocumentId = DocumentId([0x3d; 16]);
const LABEL: &str = "ZQX-PC-7731 browser";
const SERVER: &str = "wss://zqx-relay.example.org";

fn content(label: &str) -> BackupContent {
    BackupContent {
        identity: identity(6).to_entropy(),
        group_room: ROOM,
        server: SERVER.to_owned(),
        label: label.to_owned(),
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

fn every_file(dir: &Path) -> Vec<u8> {
    let mut all = Vec::new();
    for item in std::fs::read_dir(dir).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            all.extend(every_file(&path));
        } else {
            all.extend(std::fs::read(&path).unwrap());
            all.extend(path.to_string_lossy().as_bytes());
        }
    }
    all
}

async fn connect(url: &str) -> Socket {
    tokio_tungstenite::connect_async(url).await.unwrap().0
}

async fn send(socket: &mut Socket, frame: &ClientFrame) {
    socket
        .send(Message::Binary(frame.encode().into()))
        .await
        .unwrap();
}

/// The next relay frame, or `None` when the connection ended or nothing came within 3 s.
async fn next_frame(socket: &mut Socket) -> Option<RelayFrame> {
    loop {
        match tokio::time::timeout(Duration::from_secs(3), socket.next()).await {
            Ok(Some(Ok(Message::Binary(bytes)))) => {
                return Some(RelayFrame::decode(&bytes).unwrap());
            }
            Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) | Err(_) => return None,
            Ok(Some(Ok(_))) => {}
        }
    }
}

async fn exchange(url: &str, mut exchange: BackupExchange) -> BackupOutcome {
    let mut socket = connect(url).await;
    socket
        .send(Message::Binary(exchange.first_frame().into()))
        .await
        .unwrap();
    while let Ok(Some(Ok(message))) =
        tokio::time::timeout(Duration::from_secs(3), socket.next()).await
    {
        if let Message::Binary(bytes) = message
            && let Some(answer) = exchange.receive(&bytes)
        {
            socket.send(Message::Binary(answer.into())).await.unwrap();
        }
        if exchange.finished() {
            break;
        }
    }
    exchange.closed();
    exchange.take_outcome().unwrap()
}

/// Begins a write of `keys`' record and puts `record` signed by `signer` with `write_key`.
async fn put_raw(
    url: &str,
    keys: &BackupKeys,
    signer: &BackupKeys,
    record: Vec<u8>,
) -> Option<RelayFrame> {
    let mut socket = connect(url).await;
    send(&mut socket, &ClientFrame::BackupBegin { id: keys.id() }).await;
    let Some(RelayFrame::Challenge { nonce }) = next_frame(&mut socket).await else {
        panic!("no challenge");
    };
    let signature = signer.sign_write(BackupOp::Put, &nonce, &record);
    send(
        &mut socket,
        &ClientFrame::BackupPut {
            write_key: signer.write_key(),
            signature,
            record,
        },
    )
    .await;
    next_frame(&mut socket).await
}

/// Criteria 4 and 30: an open pairing slot lives in memory only: after a restart the code is
/// unknown, and the data directory holds nothing of it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn criterion_4_slots_do_not_survive_a_restart() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    let mut opener = connect(&url).await;
    send(&mut opener, &ClientFrame::PairOpen).await;
    let Some(RelayFrame::PairSlot { slot, expires_in_s }) = next_frame(&mut opener).await else {
        panic!("no slot");
    };
    assert_eq!(expires_in_s, 300);
    server.shutdown().await.unwrap();
    drop(opener);

    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    let mut joiner = connect(&url).await;
    send(&mut joiner, &ClientFrame::PairJoin { slot }).await;
    assert_eq!(
        next_frame(&mut joiner).await,
        Some(RelayFrame::Refused {
            reason: RefusalReason::Unknown
        })
    );
    server.shutdown().await.unwrap();
    assert!(
        every_file(data.path()).is_empty(),
        "a pairing slot left files"
    );
}

/// A connection whose first frame is garbage is closed; the server keeps serving others.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn criterion_5_garbage_first_frame_does_not_stop_the_server() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    let mut bad = connect(&url).await;
    bad.send(Message::Binary(vec![0xff; 17].into()))
        .await
        .unwrap();
    let answer = next_frame_lenient(&mut bad).await;
    assert!(
        answer.is_none() || matches!(answer, Some(RelayFrame::Refused { .. })),
        "{answer:?}"
    );
    bad.send(Message::Text("hello".into())).await.ok();
    let mut good = connect(&url).await;
    send(&mut good, &ClientFrame::PairOpen).await;
    assert!(matches!(
        next_frame(&mut good).await,
        Some(RelayFrame::PairSlot { .. })
    ));
    server.shutdown().await.unwrap();
}

async fn next_frame_lenient(socket: &mut Socket) -> Option<RelayFrame> {
    match tokio::time::timeout(Duration::from_secs(3), socket.next()).await {
        Ok(Some(Ok(Message::Binary(bytes)))) => RelayFrame::decode(&bytes).ok(),
        _ => None,
    }
}

/// Criteria 34 and 36 over sockets: write, a refused replace by another write key, a refused
/// unsigned put, the 4 KiB cap, a restart, restore, then delete; the data directory and the
/// log never hold the identity, the name, the server, the room or the PRF output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn criterion_36_backup_rules_over_sockets() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    let prf = [0x61; 32];
    let owner = BackupKeys::derive(&prf);
    let other = BackupKeys::derive(&[0x62; 32]);

    assert_eq!(
        exchange(&url, BackupExchange::read(&prf)).await,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    assert_eq!(
        exchange(
            &url,
            BackupExchange::write(&prf, &content(LABEL), &[1; 24]).unwrap()
        )
        .await,
        BackupOutcome::Done
    );

    // Another write key on the owner's id.
    let evil = other.seal(&content("Evil"), &[2; 24]).unwrap();
    assert_eq!(
        put_raw(&url, &owner, &other, evil).await,
        Some(RelayFrame::Refused {
            reason: RefusalReason::Forbidden
        })
    );
    // A put with no Begin.
    let mut socket = connect(&url).await;
    let record = owner.seal(&content("Replayed"), &[3; 24]).unwrap();
    send(
        &mut socket,
        &ClientFrame::BackupPut {
            write_key: owner.write_key(),
            signature: owner.sign_write(BackupOp::Put, &[0; 32], &record),
            record,
        },
    )
    .await;
    assert!(matches!(
        next_frame(&mut socket).await,
        Some(RelayFrame::Refused { .. }) | None
    ));
    // The cap, on a fresh id.
    let big = BackupKeys::derive(&[0x63; 32]);
    assert_eq!(
        put_raw(&url, &big, &big, vec![7; 4097]).await,
        Some(RelayFrame::Refused {
            reason: RefusalReason::TooLarge
        })
    );
    assert_eq!(
        put_raw(&url, &big, &big, vec![7; 4096]).await,
        Some(RelayFrame::BackupDone)
    );

    server.shutdown().await.unwrap();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    assert_eq!(
        exchange(&url, BackupExchange::read(&prf)).await,
        BackupOutcome::Read(content(LABEL))
    );

    let stored = every_file(data.path());
    let logged = log.bytes();
    for (what, needle) in [
        ("label", LABEL.as_bytes().to_vec()),
        ("server", b"zqx-relay".to_vec()),
        ("room", ROOM.0.to_vec()),
        ("prf", prf.to_vec()),
        ("member", identity(6).identity().member.0.to_vec()),
    ] {
        assert!(!contains(&stored, &needle), "data holds the {what}");
        assert!(!contains(&logged, &needle), "log holds the {what}");
    }
    let entropy = identity(6).to_entropy();
    for window in entropy.chunks(16) {
        assert!(!contains(&stored, window), "data holds identity bytes");
    }

    assert_eq!(
        exchange(&url, BackupExchange::delete(&prf)).await,
        BackupOutcome::Done
    );
    assert_eq!(
        exchange(&url, BackupExchange::read(&prf)).await,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    server.shutdown().await.unwrap();
    // Deleted means gone from disk across a restart.
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    assert_eq!(
        exchange(&url, BackupExchange::read(&prf)).await,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    server.shutdown().await.unwrap();
}
