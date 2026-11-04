//! Passkey backups over real `WebSocket` connections (ADR 0040 decision 13, Frage 145 A): a
//! connection whose first frame is `BackupGet` or `BackupBegin` is a backup connection; a backup
//! is written, read, survives a restart, is refused to another write key and deleted; the data
//! directory and the log hold nothing readable of what it seals.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::Path;

use curvyo_crypto_core::{BackupContent, BackupKeys, DocumentId};
use curvyo_sync_core::{BackupExchange, BackupOutcome, RefusalReason};
use futures_util::{SinkExt, StreamExt};
use support::{Captured, identity, start};
use tokio_tungstenite::tungstenite::Message;

const PRF: [u8; 32] = [0x5a; 32];
const ROOM: DocumentId = DocumentId([0x7c; 16]);
const NAME: &str = "ZQX-PC-7731";
const SERVER: &str = "wss://zqx-server.example.org";

fn content() -> BackupContent {
    BackupContent {
        identity: identity(4).to_entropy(),
        group_room: ROOM,
        server: SERVER.to_owned(),
        label: NAME.to_owned(),
    }
}

/// One exchange over its own socket.
async fn exchange(url: &str, mut exchange: BackupExchange) -> BackupOutcome {
    let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    socket
        .send(Message::Binary(exchange.first_frame().into()))
        .await
        .unwrap();
    while let Some(Ok(message)) = socket.next().await {
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

fn every_file(dir: &Path) -> Vec<u8> {
    let mut all = Vec::new();
    for item in std::fs::read_dir(dir).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            all.extend(every_file(&path));
        } else {
            all.extend(std::fs::read(&path).unwrap());
        }
    }
    all
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backup_is_kept_across_a_restart_and_the_server_reads_nothing_of_it() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());

    assert_eq!(
        exchange(&url, BackupExchange::read(&PRF)).await,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    let write = BackupExchange::write(&PRF, &content(), &[3; 24]).unwrap();
    assert_eq!(exchange(&url, write).await, BackupOutcome::Done);
    assert_eq!(
        exchange(&url, BackupExchange::read(&PRF)).await,
        BackupOutcome::Read(content())
    );
    // Another passkey cannot overwrite it: its id differs, so it writes its own record.
    let other = BackupExchange::write(&[0x5b; 32], &content(), &[4; 24]).unwrap();
    assert_eq!(exchange(&url, other).await, BackupOutcome::Done);

    // What the server keeps: the id and the write key are there, nothing the record seals.
    let stored = every_file(data.path());
    let keys = BackupKeys::derive(&PRF);
    assert!(data.path().join("backups").is_dir());
    let secret = identity(4).to_entropy();
    for needle in [
        &secret[..32],
        &secret[32..],
        &ROOM.0[..],
        NAME.as_bytes(),
        b"zqx-server",
        &PRF[..],
    ] {
        assert!(
            !contains(&stored, needle),
            "the server holds a sealed value"
        );
        assert!(
            !contains(&log.bytes(), needle),
            "the log holds a sealed value"
        );
    }
    assert!(contains(&stored, &keys.write_key().0));
    server.shutdown().await.unwrap();

    // A restart reads the backups back.
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    assert_eq!(
        exchange(&url, BackupExchange::read(&PRF)).await,
        BackupOutcome::Read(content())
    );
    assert_eq!(
        exchange(&url, BackupExchange::delete(&PRF)).await,
        BackupOutcome::Done
    );
    assert_eq!(
        exchange(&url, BackupExchange::read(&PRF)).await,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    assert_eq!(
        exchange(&url, BackupExchange::read(&[0x5b; 32])).await,
        BackupOutcome::Read(content())
    );
    server.shutdown().await.unwrap();
}
