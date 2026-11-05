//! Real WebSocket clients driving `ClientSession`s with real `Document` replicas against a
//! server started in-process on `127.0.0.1:0` (ADR 0031 decision 14: the server's one
//! real-socket test). Fixed identities and seeds; every byte that leaves a client is sealed.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc
)]

use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use curvyo_crypto_core::IdentityKeys;
use curvyo_document_core::{Document, DocumentVersion, pack, unpack};
use curvyo_sync_core::{
    ClientOutput, ClientSession, InviteSecret, Millis, RecordKind, RoomId, SessionConfig,
    SessionEvent, SessionStart,
};
use curvyo_sync_server::{Config, Log, Server};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A log buffer the test can read back.
#[derive(Clone, Default)]
pub struct Captured(pub Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Captured {
    pub fn bytes(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

/// Starts a server on `listen` over `data`, logging into `log`.
pub async fn start(listen: SocketAddr, data: &Path, log: &Captured) -> Server {
    let config = Config {
        listen,
        data: data.to_owned(),
    };
    Server::start(&config, Log::new(log.clone())).await.unwrap()
}

/// The status code of a plain HTTP `GET path`, and the body.
pub async fn http_get(addr: SocketAddr, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: relay\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let status = response
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap();
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

pub fn now() -> Millis {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    Millis(u64::try_from(since.as_millis()).unwrap())
}

/// Device `n`'s identity keys from fixed entropy.
pub fn identity(n: u8) -> IdentityKeys {
    let mut entropy = [0; 64];
    for (i, byte) in entropy.iter_mut().enumerate() {
        *byte = n.wrapping_mul(31).wrapping_add(u8::try_from(i).unwrap());
    }
    IdentityKeys::from_entropy(&entropy)
}

fn seed(n: u8, salt: u8) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = n
            .wrapping_mul(97)
            .wrapping_add(salt.wrapping_mul(13))
            .wrapping_add(u8::try_from(i).unwrap());
    }
    out
}

/// One app instance: its session, its socket and its replica.
pub struct Device {
    pub session: ClientSession,
    pub doc: Option<Document>,
    pub events: Vec<SessionEvent>,
    /// Every plaintext this device sealed and sent (updates and snapshots).
    pub plaintexts: Vec<Vec<u8>>,
    /// The close codes the server sent this device.
    pub close_codes: Vec<u16>,
    socket: Option<Socket>,
    url: String,
    peer: u64,
}

impl Device {
    fn new(n: u8, url: &str, room: RoomId, start: SessionStart, doc: Option<Document>) -> Self {
        let config = SessionConfig {
            room,
            identity: identity(n),
            peer_id: u64::from(n),
            nonce_seed: seed(n, 1),
            entropy_seed: seed(n, 2),
            since_seq: 0,
            start,
        };
        Self {
            session: ClientSession::new(config, now()).unwrap(),
            doc,
            events: Vec::new(),
            plaintexts: Vec::new(),
            close_codes: Vec::new(),
            socket: None,
            url: url.to_owned(),
            peer: u64::from(n) << 32,
        }
    }

    /// Device `n` shares `doc` in `room` (criterion 1).
    pub fn share(n: u8, url: &str, room: RoomId, doc: Document) -> Self {
        Self::new(n, url, room, SessionStart::Share, Some(doc))
    }

    /// Device `n` starts a session in `room` with `start` (a return check, `specs/0097`).
    pub fn start(n: u8, url: &str, room: RoomId, start: SessionStart) -> Self {
        Self::new(n, url, room, start, None)
    }

    /// Device `n` opens an invite link (criterion 3).
    pub fn join(n: u8, url: &str, secret: InviteSecret) -> Self {
        let room = secret.room;
        Self::new(n, url, room, SessionStart::Join(secret), None)
    }

    pub fn doc(&self) -> &Document {
        self.doc.as_ref().expect("a replica")
    }

    pub fn is_online(&self) -> bool {
        self.session.is_online()
    }

    /// One edit, exported and pushed as one update.
    pub fn edit<R>(&mut self, write: impl FnOnce(&Document) -> R) -> R {
        let doc = self.doc.as_ref().expect("a replica");
        let before = doc.version();
        let result = write(doc);
        let bytes = doc.export_updates(&before).unwrap();
        self.plaintexts.push(bytes.clone());
        self.session.push(RecordKind::Update, bytes).unwrap();
        result
    }

    /// One turn: carry out the session's requests, read what arrived, tick.
    pub async fn step(&mut self) {
        self.carry_out().await;
        while let Some(socket) = self.socket.as_mut() {
            match tokio::time::timeout(Duration::from_millis(2), socket.next()).await {
                Err(_) => break,
                Ok(Some(Ok(Message::Binary(bytes)))) => self.session.received(&bytes, now()),
                Ok(Some(Ok(Message::Close(frame)))) => {
                    if let Some(frame) = frame {
                        self.close_codes.push(frame.code.into());
                    }
                    self.lost();
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(_)) | None) => self.lost(),
            }
            self.carry_out().await;
        }
        self.session.tick(now());
        self.carry_out().await;
    }

    fn lost(&mut self) {
        self.socket = None;
        self.session.disconnected(now());
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
                            self.lost();
                        }
                    }
                    ClientOutput::Close => {
                        if let Some(mut socket) = self.socket.take() {
                            let _ = socket.close(None).await;
                        }
                    }
                    ClientOutput::Event(event) => {
                        self.apply(&event);
                        self.events.push(event);
                    }
                }
            }
        }
    }

    /// What the editor's session does with each event (as the simulator of `sync-core`).
    fn apply(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::Remote { kind, bytes, .. } => match kind {
                RecordKind::Update => self.doc().import_updates(bytes).unwrap(),
                RecordKind::Asset => {
                    self.doc().insert_asset(bytes).unwrap();
                }
                RecordKind::Snapshot => {
                    self.peer += 1;
                    let received = unpack(self.peer, bytes).unwrap();
                    match &self.doc {
                        None => self.doc = Some(received),
                        Some(doc) => {
                            let all = received.export_updates(&DocumentVersion::never()).unwrap();
                            doc.import_updates(&all).unwrap();
                        }
                    }
                }
            },
            SessionEvent::SnapshotDue { covers_seq } => {
                let bytes = pack(self.doc(), "test").unwrap();
                self.plaintexts.push(bytes.clone());
                self.session.upload_snapshot(&bytes, *covers_seq).unwrap();
            }
            _ => {}
        }
    }
}

/// Steps every device until `done` holds; the elapsed time, or `None` after `limit`.
pub async fn run_until(
    devices: &mut [&mut Device],
    limit: Duration,
    done: impl Fn(&[&mut Device]) -> bool,
) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() <= limit {
        if done(devices) {
            return Some(start.elapsed());
        }
        for device in devices.iter_mut() {
            device.step().await;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

/// Whether every device holds a replica with the same version and content.
pub fn converged(devices: &[&mut Device]) -> bool {
    let Some(first) = devices[0].doc.as_ref() else {
        return false;
    };
    let json = first.export_json().unwrap();
    devices.iter().all(|d| {
        d.doc.as_ref().is_some_and(|doc| {
            doc.version() == first.version() && doc.export_json().unwrap() == json
        })
    })
}
