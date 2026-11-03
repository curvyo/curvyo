//! The server over real `WebSocket` connections on `127.0.0.1:0` with a temporary data directory
//! (`specs/0092-cooperative-editing` criteria 20, 24, 37, 38): two devices share a document
//! holding `ZQX-MARKER-7731` and `#13a7c9`, the server restarts on its data directory, and
//! neither the data directory nor the server's log holds any plaintext.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use curvyo_document_core::{
    Affine, Color, Document, FontKind, FontRef, Length, NodeId, Point, RectBounds, StyleEdit,
    TextLayoutValues, Vec2,
};
use curvyo_sync_core::RoomId;
use curvyo_sync_server::room_hex;
use support::{Captured, Device, converged, http_get, now, run_until, start};

const ROOM: RoomId = RoomId([0x5e; 16]);
const MARKER: &str = "ZQX-MARKER-7731";

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// The marker text with the stroke colour `#13a7c9` (criterion 24).
fn marker_text(doc: &Document) -> NodeId {
    let id = doc
        .create_text(
            MARKER,
            &font(),
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

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 50.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// Every file under `dir` with its bytes.
fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for item in fs::read_dir(dir).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.insert(path.clone(), fs::read(&path).unwrap());
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// The 16-byte windows of `plain` with at least eight distinct byte values (runs of padding
/// could occur anywhere by chance; these cannot).
fn fragments(plain: &[u8]) -> Vec<&[u8]> {
    plain
        .windows(16)
        .filter(|w| {
            let mut seen = [false; 256];
            w.iter().for_each(|&b| seen[usize::from(b)] = true);
            seen.iter().filter(|&&s| s).count() >= 8
        })
        .collect()
}

/// Criterion 24: neither the marker, the colour, an SVG or JSON key, nor any fragment of a
/// plaintext is in the data directory or the log.
fn assert_no_plaintext(data: &Path, log: &Captured, plain: &[&[u8]]) {
    let mut stored: Vec<(String, Vec<u8>)> = files(data)
        .into_iter()
        .map(|(path, bytes)| (path.display().to_string(), bytes))
        .collect();
    stored.push(("log".to_owned(), log.bytes()));
    let needles: [&[u8]; 5] = [
        MARKER.as_bytes(),
        b"13a7c9",
        b"13A7C9",
        b"<svg",
        b"\"children\"",
    ];
    for (name, bytes) in &stored {
        for needle in needles {
            assert!(!contains(bytes, needle), "{name} holds {needle:?}");
        }
        for text in plain {
            for fragment in fragments(text) {
                assert!(
                    !contains(bytes, fragment),
                    "{name} holds a plaintext fragment"
                );
            }
        }
    }
}

/// The log holds lifecycle lines only: no client address, no request body (criterion 36).
fn assert_lifecycle_log_only(log: &Captured, listening: &str) {
    let log_text = String::from_utf8(log.bytes()).unwrap();
    for line in log_text.lines() {
        let message = line.strip_prefix("curvyo-sync-server: ").unwrap();
        assert!(
            message == listening
                || message.starts_with("startup scan done: ")
                || message == "shutting down"
                || message == "stopped",
            "unexpected log line {line:?}"
        );
    }
}

/// Criteria 20, 24, 38 (and 37's reconnect): edits survive a restart on the same data
/// directory, both devices reconnect without user action within 30 s, the stored data only
/// grows by new sealed entries, and no plaintext reaches the disk or the log.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sealed_edits_survive_a_restart_and_leave_no_plaintext() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let addr = server.local_addr();
    let url = format!("ws://{addr}/");

    let doc = Document::new(100);
    marker_text(&doc);
    let json_a = doc.export_json().unwrap();
    assert!(
        contains(&json_a, MARKER.as_bytes()),
        "the plaintext holds the marker, so the search below means something"
    );
    let mut a = Device::share(1, &url, ROOM, doc);
    assert!(
        run_until(&mut [&mut a], Duration::from_secs(10), |d| d[0].is_online())
            .await
            .is_some()
    );
    let expires = now().unix_seconds() + 24 * 3600;
    let secret = a.session.invite(expires).unwrap();
    let mut b = Device::join(2, &url, secret);
    assert!(
        run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
            .await
            .is_some(),
        "B joins and holds A's document"
    );
    b.edit(|d| rect(d, 0.0));
    a.edit(marker_text);
    assert!(
        run_until(&mut [&mut a, &mut b], Duration::from_secs(10), converged)
            .await
            .is_some()
    );
    let before = files(data.path());
    let room_dir = data.path().join(room_hex(ROOM));
    assert!(before.contains_key(&room_dir.join("keyring.log")));
    assert!(before.contains_key(&room_dir.join("log")));
    let keyring = &before[&room_dir.join("keyring.log")];
    assert!(
        curvyo_crypto_core::keyring::decode_log(keyring).is_ok(),
        "the stored keyring.log is the client's keyring.log format"
    );

    // Restart on the same address and directory; meanwhile A edits offline.
    server.shutdown().await.unwrap();
    a.edit(|d| rect(d, 10.0));
    let server = start(addr, data.path(), &log).await;
    let took = run_until(&mut [&mut a, &mut b], Duration::from_secs(30), |d| {
        d.iter().all(|x| x.is_online()) && converged(d)
    })
    .await;
    assert!(took.is_some(), "both reconnect and converge within 30 s");
    assert!(a.close_codes.contains(&1012) && b.close_codes.contains(&1012));
    assert_eq!(b.doc().object_ids().len(), 4, "no edit lost");

    // The stored data only grew: every file is unchanged or extended by new entries.
    let after = files(data.path());
    for (path, old) in &before {
        let new = after.get(path).expect("no stored file disappears");
        assert!(new.starts_with(old), "{} only grows", path.display());
    }
    assert!(after[&room_dir.join("log")].len() > before[&room_dir.join("log")].len());
    server.shutdown().await.unwrap();

    // Criterion 24: no marker, colour, JSON or SVG fragment in the stored data or the log.
    let json_b = b.doc().export_json().unwrap();
    let plain: Vec<&[u8]> = a
        .plaintexts
        .iter()
        .chain(&b.plaintexts)
        .map(Vec::as_slice)
        .chain([json_a.as_slice(), json_b.as_slice()])
        .collect();
    assert_no_plaintext(data.path(), &log, &plain);
    assert_lifecycle_log_only(&log, &format!("listening on {addr}"));
}

/// Criterion 37: `/healthz` answers 200 when ready and 503 while the data directory is not
/// writable, revealing no room data.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn healthz_follows_the_data_directory() {
    use std::os::unix::fs::PermissionsExt;

    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let addr = server.local_addr();
    let (status, body) = http_get(addr, "/healthz").await;
    assert_eq!(status, 200);
    assert_eq!(body, "ready\n");

    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let writable_anyway = fs::write(data.path().join("probe"), b"").is_ok();
    if !writable_anyway {
        let (status, body) = http_get(addr, "/healthz").await;
        assert_eq!(status, 503);
        assert_eq!(body, "not ready\n");
    }
    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_file(data.path().join("probe"));
    assert_eq!(http_get(addr, "/healthz").await.0, 200);
    server.shutdown().await.unwrap();
}
