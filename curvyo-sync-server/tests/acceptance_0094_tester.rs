//! Tester's acceptance test for `specs/0094-workspace` criterion 34 (and 47) against the
//! in-process server over real `WebSocket`s, written from the specification before reading the
//! implementation: two shared workspaces of the same total size and the same edit pattern, one of
//! 3 documents and one of 30, leave server data (files, sizes, log) of the same structure in
//! which no name, folder, marker or document count can be read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod support;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use curvyo_document_core::{Created, Document, WorkspaceList, pack};
use curvyo_sync_core::{
    ClientOutput, ClientSession, InviteSecret, RecordKind, RoomId, SessionConfig, SessionEvent,
    SessionStart, StreamEntry, encode_inner_record, encode_snapshot_stream, split_snapshot,
};
use curvyo_sync_server::room_hex;
use futures_util::{SinkExt, StreamExt};
use support::{Captured, identity, now, start};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const FEW: RoomId = RoomId([0x31; 16]);
const MANY: RoomId = RoomId([0x32; 16]);

/// One app instance of a workspace room, driven as the editor drives it.
struct Ws {
    session: ClientSession,
    socket: Option<Socket>,
    url: String,
    stream: Vec<u8>,
    events: Vec<SessionEvent>,
}

impl Ws {
    fn new(n: u8, url: &str, room: RoomId, start: SessionStart, stream: Vec<u8>) -> Self {
        let mut seed = [0_u8; 32];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = n.wrapping_mul(53).wrapping_add(u8::try_from(i).unwrap());
        }
        let mut entropy = seed;
        entropy[0] ^= 0xff;
        let mut session = ClientSession::new(
            SessionConfig {
                room,
                identity: identity(n),
                peer_id: 0,
                nonce_seed: seed,
                entropy_seed: entropy,
                since_seq: 0,
                start,
            },
            now(),
        )
        .unwrap();
        session.use_workspace_snapshots();
        Self {
            session,
            socket: None,
            url: url.to_owned(),
            stream,
            events: Vec::new(),
        }
    }

    async fn step(&mut self) {
        self.carry_out().await;
        while let Some(socket) = self.socket.as_mut() {
            match tokio::time::timeout(Duration::from_millis(2), socket.next()).await {
                Err(_) => break,
                Ok(Some(Ok(Message::Binary(bytes)))) => self.session.received(&bytes, now()),
                Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) => {
                    self.socket = None;
                    self.session.disconnected(now());
                }
                Ok(Some(Ok(_))) => {}
            }
            self.carry_out().await;
        }
        self.session.tick(now());
        self.carry_out().await;
    }

    async fn carry_out(&mut self) {
        loop {
            let outputs = self.session.drain();
            if outputs.is_empty() {
                return;
            }
            for output in outputs {
                match output {
                    ClientOutput::Connect => {
                        match tokio_tungstenite::connect_async(&self.url).await {
                            Ok((socket, _)) => {
                                self.socket = Some(socket);
                                self.session.connected(now());
                            }
                            Err(_) => self.session.disconnected(now()),
                        }
                    }
                    ClientOutput::Send(bytes) => {
                        if let Some(socket) = self.socket.as_mut()
                            && socket.send(Message::Binary(bytes.into())).await.is_err()
                        {
                            self.socket = None;
                            self.session.disconnected(now());
                        }
                    }
                    ClientOutput::Close => {
                        if let Some(mut socket) = self.socket.take() {
                            let _ = socket.close(None).await;
                        }
                    }
                    ClientOutput::Event(event) => {
                        if let SessionEvent::SnapshotDue { covers_seq } = &event {
                            let parts = split_snapshot(&self.stream, [0x5e; 16]);
                            self.session
                                .upload_snapshot_parts(&parts, *covers_seq)
                                .unwrap();
                        }
                        self.events.push(event);
                    }
                }
            }
        }
    }
}

async fn run_until(devices: &mut [&mut Ws], limit: Duration, done: impl Fn(&[&mut Ws]) -> bool) {
    let start = Instant::now();
    while start.elapsed() <= limit {
        if done(devices) {
            return;
        }
        for d in devices.iter_mut() {
            d.step().await;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("not done within {limit:?}");
}

const DOC_NAME: &str = "ZQX-DOC-7731";
const FOLDER_NAME: &str = "ZQX-FOLDER-7731";
const TEXT_MARKER: &str = "ZQX-TEXT-7731";
const COLOUR: &str = "#7a3c19";

/// The snapshot stream of a workspace of `count` documents in one folder, padded to `total`.
fn stream_of(room: RoomId, count: usize, total: usize) -> Vec<u8> {
    let list = WorkspaceList::new(1);
    let folder = list
        .create_folder(None, FOLDER_NAME, Created { at: 1, id: [1; 16] })
        .unwrap();
    let mut entries = Vec::new();
    let mut docs = Vec::new();
    for i in 0..count {
        let id = [u8::try_from(i).unwrap() + 1; 16];
        let name = if i == 0 {
            DOC_NAME.to_owned()
        } else {
            format!("{DOC_NAME} {i}")
        };
        list.create_document(
            Some(folder),
            &name,
            id,
            Created {
                at: 2,
                id: [u8::try_from(i).unwrap(); 16],
            },
        )
        .unwrap();
        let mut bytes = pack(&Document::new(9), "test").unwrap();
        bytes.extend_from_slice(format!("{TEXT_MARKER} {COLOUR}").as_bytes());
        docs.push((id, bytes));
    }
    entries.push(StreamEntry {
        stream: room.0,
        bytes: list.snapshot().unwrap(),
    });
    for (id, bytes) in docs {
        entries.push(StreamEntry { stream: id, bytes });
    }
    let len = encode_snapshot_stream(&entries).len();
    assert!(len <= total, "{len} > {total}");
    entries
        .last_mut()
        .unwrap()
        .bytes
        .extend(std::iter::repeat_n(0x20, total - len));
    let stream = encode_snapshot_stream(&entries);
    assert_eq!(stream.len(), total);
    stream
}

/// Shares `room` from device `a_n`, admits device `b_n`, then both push 20 records each of one
/// size, cycling over the `count` documents.
async fn workspace_session(url: &str, room: RoomId, a_n: u8, b_n: u8, count: usize) {
    let total = 900_000;
    let mut a = Ws::new(
        a_n,
        url,
        room,
        SessionStart::Share,
        stream_of(room, count, total),
    );
    run_until(&mut [&mut a], Duration::from_secs(20), |d| {
        d[0].session.is_online()
    })
    .await;
    // The first snapshot is in: the relay asked for it on share.
    run_until(&mut [&mut a], Duration::from_secs(20), |d| {
        d[0].events
            .iter()
            .any(|e| matches!(e, SessionEvent::SnapshotDue { .. }))
    })
    .await;
    let secret: InviteSecret = a.session.invite(now().unix_seconds() + 3_600).unwrap();
    let mut b = Ws::new(b_n, url, room, SessionStart::Join(secret), Vec::new());
    run_until(&mut [&mut a, &mut b], Duration::from_secs(20), |d| {
        d[1].events.contains(&SessionEvent::Admitted) && d[1].session.is_online()
    })
    .await;
    for i in 0..40_usize {
        let doc = [u8::try_from(i % count).unwrap() + 1; 16];
        let record = encode_inner_record(doc, 0, &[0x42; 300]);
        let device = if i % 2 == 0 { &mut a } else { &mut b };
        device.session.push(RecordKind::Update, record).unwrap();
        device.step().await;
    }
    run_until(&mut [&mut a, &mut b], Duration::from_secs(20), |d| {
        d.iter().all(|w| {
            w.events
                .iter()
                .filter(|e| matches!(e, SessionEvent::Acked { .. }))
                .count()
                >= 20
        })
    })
    .await;
}

/// Every file below `dir`: relative path with `hex` replaced by `ROOM`, and its size.
fn structure(dir: &Path, hex: &str) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let entry = entry.unwrap();
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace(hex, "ROOM");
            if entry.file_type().unwrap().is_dir() {
                out.insert(format!("{rel}/"), 0);
                stack.push(entry.path());
            } else {
                out.insert(rel, entry.metadata().unwrap().len());
            }
        }
    }
    out
}

fn all_bytes(dir: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let entry = entry.unwrap();
            out.extend_from_slice(entry.file_name().to_string_lossy().as_bytes());
            if entry.file_type().unwrap().is_dir() {
                stack.push(entry.path());
            } else {
                out.extend(std::fs::read(entry.path()).unwrap());
            }
        }
    }
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Criteria 34 and 47: the 3-document and the 30-document workspace leave server data of the
/// same structure and sizes; no name, folder, text or colour is stored or logged.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn c34_three_and_thirty_documents_look_the_same_on_the_server() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}/", server.local_addr());
    workspace_session(&url, FEW, 1, 2, 3).await;
    workspace_session(&url, MANY, 3, 4, 30).await;
    server.shutdown().await.unwrap();

    let few_hex = room_hex(FEW);
    let many_hex = room_hex(MANY);
    let entries: Vec<String> = std::fs::read_dir(data.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let few_dir = entries
        .iter()
        .find(|e| e.contains(&few_hex))
        .expect("a room directory per workspace");
    let many_dir = entries
        .iter()
        .find(|e| e.contains(&many_hex))
        .expect("a room directory per workspace");
    let few = structure(&data.path().join(few_dir), &few_hex);
    let many = structure(&data.path().join(many_dir), &many_hex);
    assert_eq!(
        few.keys().collect::<Vec<_>>(),
        many.keys().collect::<Vec<_>>(),
        "same files"
    );
    for (path, size) in &few {
        // The keyring holds the members' signatures and keys; same structure, sizes may differ
        // only by member data, which is the same here.
        assert_eq!(
            Some(size),
            many.get(path),
            "{path}: {size} vs {:?} gives the document count away",
            many.get(path)
        );
    }
    assert!(
        few.keys().any(|k| k.contains("snapshot")) && few.len() >= 3,
        "the rooms hold a snapshot and records: {few:?}"
    );
    let stored = all_bytes(data.path());
    let logged = log.bytes();
    for needle in [DOC_NAME, FOLDER_NAME, TEXT_MARKER, COLOUR] {
        assert!(
            !contains(&stored, needle.as_bytes()),
            "{needle} in the data"
        );
        assert!(!contains(&logged, needle.as_bytes()), "{needle} in the log");
    }
    // The log says the same about both rooms.
    let text = String::from_utf8_lossy(&logged).into_owned();
    let lines = |hex: &str| text.lines().filter(|l| l.contains(hex)).count();
    assert_eq!(lines(&few_hex), lines(&many_hex), "log:\n{text}");
    assert!(!text.contains(" 3 ") || !text.contains(" 30 "), "{text}");
}
