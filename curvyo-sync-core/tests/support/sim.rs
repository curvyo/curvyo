//! A deterministic, seeded network simulator (ADR 0031 decision 14): one in-process [`Relay`],
//! clients driving [`ClientSession`]s with real `Document` replicas, per-connection latency in
//! order (as TCP gives), interleaving across connections, partitions, app close and reopen
//! through `pack`/`unpack`, and relay restarts from the store.

use std::collections::{BTreeMap, VecDeque};

use curvyo_crypto_core::EpochKey;
use curvyo_crypto_core::keyring::{decode_log, encode_log};
use curvyo_document_core::{Document, DocumentVersion, ShareRecord, pack, unpack};
use curvyo_sync_core::{
    ClientOutput, ClientSession, ConnectionId, InviteSecret, Millis, RecordKind, Relay,
    RelayOutput, RoomId, SessionConfig, SessionEvent, SessionStart, StoreOp, StoredRoom,
};

use super::{device, seed};

/// The simulator's step.
pub const STEP_MS: u64 = 10;
const START_MS: u64 = 1_800_000_000_000;

enum Msg {
    Data(Vec<u8>),
    Close,
}

/// One app instance: its session, its replica and its saved file.
pub struct Client {
    pub device: u8,
    pub session: Option<ClientSession>,
    pub doc: Option<Document>,
    /// Whether its network works; a cut link drops everything silently.
    pub link_up: bool,
    /// A frozen app neither ticks nor reads its socket, which stays open.
    pub frozen: bool,
    /// When its session asked to connect.
    pub connect_times: Vec<Millis>,
    conn: Option<ConnectionId>,
    up: VecDeque<(Millis, Msg)>,
    down: VecDeque<(Millis, Msg)>,
    /// Every event its session reported, in order.
    pub events: Vec<SessionEvent>,
    /// The bytes of every message it sent to the relay.
    pub sent: Vec<Vec<u8>>,
    pushed_versions: BTreeMap<u64, DocumentVersion>,
    pub synced_version: DocumentVersion,
    /// The credential store: epoch keys by number.
    pub epoch_keys: Vec<(u32, EpochKey)>,
    /// The saved `.curvyo` file, after `close_app`.
    pub file: Option<Vec<u8>>,
    /// Imports the document refused (criterion 25 path).
    pub import_failures: usize,
    sessions_started: u64,
}

impl Client {
    fn new(device: u8) -> Self {
        Self {
            device,
            session: None,
            doc: None,
            link_up: true,
            frozen: false,
            connect_times: Vec::new(),
            conn: None,
            up: VecDeque::new(),
            down: VecDeque::new(),
            events: Vec::new(),
            sent: Vec::new(),
            pushed_versions: BTreeMap::new(),
            synced_version: DocumentVersion::never(),
            epoch_keys: Vec::new(),
            file: None,
            import_failures: 0,
            sessions_started: 0,
        }
    }

    pub fn doc(&self) -> &Document {
        self.doc.as_ref().expect("a replica")
    }

    pub fn session(&self) -> &ClientSession {
        self.session.as_ref().expect("a session")
    }

    pub fn saw(&self, wanted: &SessionEvent) -> bool {
        self.events.iter().any(|e| e == wanted)
    }

    /// Whether a peer's presence was shown and later reported gone.
    pub fn saw_gone_after_presence(&self) -> bool {
        let shown = self
            .events
            .iter()
            .position(|e| matches!(e, SessionEvent::Presence { .. }));
        shown.is_some_and(|at| {
            self.events[at..]
                .iter()
                .any(|e| matches!(e, SessionEvent::PresenceGone { .. }))
        })
    }

    pub fn count(&self, matches: impl Fn(&SessionEvent) -> bool) -> usize {
        self.events.iter().filter(|e| matches(e)).count()
    }

    fn peer_id(&mut self) -> u64 {
        self.sessions_started += 1;
        (u64::from(self.device) << 32) | self.sessions_started
    }
}

/// The relay, its store, the clients and the virtual clock.
pub struct Net {
    pub relay: Relay,
    pub store: BTreeMap<RoomId, StoredRoom>,
    /// The snapshot parts the relay staged per room (`snapshot.part/`).
    staged: BTreeMap<RoomId, Vec<Vec<u8>>>,
    pub now: Millis,
    pub clients: Vec<Client>,
    conns: BTreeMap<ConnectionId, usize>,
    rng: u64,
    /// Upper bound of the one-way latency in milliseconds.
    pub max_latency_ms: u64,
}

impl Net {
    pub fn new(seed: u64) -> Self {
        Self {
            relay: Relay::new(),
            store: BTreeMap::new(),
            staged: BTreeMap::new(),
            now: Millis(START_MS),
            clients: Vec::new(),
            conns: BTreeMap::new(),
            rng: seed ^ 0x9e37_79b9_7f4a_7c15,
            max_latency_ms: 20,
        }
    }

    fn random(&mut self) -> u64 {
        // splitmix64
        self.rng = self.rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn add_client(&mut self, device: u8) -> usize {
        self.clients.push(Client::new(device));
        self.clients.len() - 1
    }

    fn config(
        &mut self,
        i: usize,
        room: RoomId,
        since_seq: u64,
        start: SessionStart,
    ) -> SessionConfig {
        let salt = u8::try_from(self.random() & 0xff).unwrap();
        let client = &mut self.clients[i];
        SessionConfig {
            room,
            identity: device(client.device),
            peer_id: client.peer_id(),
            nonce_seed: seed(client.device, salt),
            entropy_seed: seed(client.device, salt.wrapping_add(1)),
            since_seq,
            start,
        }
    }

    /// Client `i` shares `doc` in `room` (criterion 1).
    pub fn share(&mut self, i: usize, room: RoomId, doc: Document) {
        let config = self.config(i, room, 0, SessionStart::Share);
        self.clients[i].doc = Some(doc);
        self.clients[i].session = Some(ClientSession::new(config, self.now).unwrap());
        self.pump(i);
    }

    /// Client `i` opens an invite link (criterion 3).
    pub fn join(&mut self, i: usize, secret: InviteSecret) {
        let config = self.config(i, secret.room, 0, SessionStart::Join(secret));
        self.clients[i].session = Some(ClientSession::new(config, self.now).unwrap());
        self.pump(i);
    }

    /// Client `i` (admin, online) writes an invite valid for `seconds`.
    pub fn invite(&mut self, i: usize, seconds: u64) -> InviteSecret {
        let expires = self.now.unix_seconds() + seconds;
        let secret = self.clients[i]
            .session
            .as_mut()
            .unwrap()
            .invite(expires)
            .unwrap();
        self.pump(i);
        secret
    }

    /// One edit in client `i`, exported and pushed as one update (one history step).
    pub fn edit<R>(&mut self, i: usize, write: impl FnOnce(&Document) -> R) -> R {
        let client = &mut self.clients[i];
        let doc = client.doc.as_ref().expect("a replica");
        let before = doc.version();
        let result = write(doc);
        let bytes = doc.export_updates(&before).unwrap();
        let after = doc.version();
        let client_id = client
            .session
            .as_mut()
            .unwrap()
            .push(RecordKind::Update, bytes)
            .unwrap();
        client.pushed_versions.insert(client_id, after);
        self.pump(i);
        result
    }

    /// Client `i`'s presence payload changes.
    pub fn presence(&mut self, i: usize, payload: &[u8]) {
        let now = self.now;
        if let Some(session) = self.clients[i].session.as_mut() {
            session.set_presence(payload.to_vec(), now);
        }
        self.pump(i);
    }

    /// Cuts or restores client `i`'s network. A cut drops every message in flight; the relay
    /// notices the socket closing, the client only by its missing pongs.
    pub fn set_link(&mut self, i: usize, up: bool) {
        self.clients[i].link_up = up;
        if !up {
            self.drop_connection(i);
        }
    }

    /// The platform reports client `i`'s network back (browser `online`): the session tries
    /// at once instead of waiting for its backoff.
    pub fn network_back(&mut self, i: usize) {
        let now = self.now;
        if let Some(session) = self.clients[i].session.as_mut() {
            session.network_back(now);
        }
        self.pump(i);
    }

    fn drop_connection(&mut self, i: usize) {
        let client = &mut self.clients[i];
        client.up.clear();
        client.down.clear();
        if let Some(conn) = client.conn.take() {
            self.conns.remove(&conn);
            let out = self.relay.disconnect(conn);
            self.route(out);
        }
    }

    /// The relay process restarts on its stored data; every socket closes (code 1012).
    pub fn restart_relay(&mut self) {
        self.relay = Relay::restore(self.store.values().cloned().collect()).unwrap();
        self.conns.clear();
        for i in 0..self.clients.len() {
            let client = &mut self.clients[i];
            client.up.clear();
            client.down.clear();
            if client.conn.take().is_some()
                && let Some(session) = client.session.as_mut()
            {
                session.disconnected(self.now);
            }
            self.pump(i);
        }
    }

    /// Client `i` saves its document with its share record and closes the app.
    pub fn close_app(&mut self, i: usize) {
        self.drop_connection(i);
        let client = &mut self.clients[i];
        let session = client.session.take().expect("a session");
        let doc = client.doc.take().expect("a replica");
        doc.set_share(Some(ShareRecord {
            server: "ws://relay.test:8787".to_owned(),
            room: session.room().0,
            server_seq: session.cursor(),
            synced_version: client.synced_version.clone(),
            keyring_log: encode_log(&session.keyring_entries()).unwrap(),
        }));
        client.epoch_keys = session.epoch_keys();
        client.file = Some(pack(&doc, "sim").unwrap());
    }

    /// Client `i` reopens its saved file: it reconnects without an invite and pushes what the
    /// relay has not acknowledged (ADR 0031 decision 9).
    pub fn reopen(&mut self, i: usize) {
        let peer = self.clients[i].peer_id();
        let file = self.clients[i].file.clone().expect("a saved file");
        let doc = unpack(peer, &file).unwrap();
        let record = doc.share().expect("a shared file");
        let keyring = decode_log(&record.keyring_log).unwrap();
        let epoch_keys = self.clients[i].epoch_keys.clone();
        let mut config = self.config(
            i,
            RoomId(record.room),
            record.server_seq,
            SessionStart::Member {
                keyring,
                epoch_keys,
            },
        );
        config.peer_id = peer;
        let client = &mut self.clients[i];
        let mut session = ClientSession::new(config, self.now).unwrap();
        let unsynced = doc.export_updates(&record.synced_version).unwrap();
        let after = doc.version();
        let client_id = session.push(RecordKind::Update, unsynced).unwrap();
        client.pushed_versions.insert(client_id, after);
        client.synced_version = record.synced_version;
        client.doc = Some(doc);
        client.session = Some(session);
        self.pump(i);
    }

    /// Advances the clock by `ms`.
    pub fn run(&mut self, ms: u64) {
        let until = self.now.plus(ms);
        while self.now < until {
            self.step();
        }
    }

    /// Runs until `done` holds or `limit_ms` passed; returns the elapsed milliseconds.
    pub fn run_until(&mut self, limit_ms: u64, mut done: impl FnMut(&Self) -> bool) -> Option<u64> {
        let start = self.now;
        while self.now.since(start) <= limit_ms {
            if done(self) {
                return Some(self.now.since(start));
            }
            self.step();
        }
        None
    }

    fn order(&mut self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.clients.len()).collect();
        for k in (1..order.len()).rev() {
            let j = usize::try_from(self.random() % (k as u64 + 1)).unwrap();
            order.swap(k, j);
        }
        order
    }

    fn step(&mut self) {
        self.now = self.now.plus(STEP_MS);
        for i in self.order() {
            self.deliver_up(i);
        }
        for i in self.order() {
            if !self.clients[i].frozen {
                self.deliver_down(i);
            }
        }
        for i in 0..self.clients.len() {
            if self.clients[i].frozen {
                continue;
            }
            if let Some(session) = self.clients[i].session.as_mut() {
                session.tick(self.now);
            }
            self.pump(i);
        }
    }

    fn deliver_up(&mut self, i: usize) {
        while let Some((due, _)) = self.clients[i].up.front() {
            if *due > self.now {
                return;
            }
            let Some((_, msg)) = self.clients[i].up.pop_front() else {
                return;
            };
            let Some(conn) = self.clients[i].conn else {
                return;
            };
            if let Msg::Data(bytes) = msg {
                let mut nonce = [0; 32];
                for chunk in nonce.chunks_mut(8) {
                    chunk.copy_from_slice(&self.random().to_le_bytes());
                }
                let out = self.relay.receive(conn, &bytes, self.now, nonce);
                self.route(out);
            }
        }
    }

    fn deliver_down(&mut self, i: usize) {
        while let Some((due, _)) = self.clients[i].down.front() {
            if *due > self.now {
                break;
            }
            let Some((_, msg)) = self.clients[i].down.pop_front() else {
                break;
            };
            let now = self.now;
            let client = &mut self.clients[i];
            let Some(session) = client.session.as_mut() else {
                break;
            };
            match msg {
                Msg::Data(bytes) => session.received(&bytes, now),
                Msg::Close => {
                    client.conn = None;
                    client.up.clear();
                    session.disconnected(now);
                }
            }
            self.pump(i);
        }
    }

    fn due(&mut self, last: Option<Millis>) -> Millis {
        let latency = 1 + self.random() % self.max_latency_ms.max(1);
        let at = self.now.plus(latency);
        last.map_or(at, |l| l.max(at))
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    if let Some(&i) = self.conns.get(&connection) {
                        let due = self.due(self.clients[i].down.back().map(|(t, _)| *t));
                        self.clients[i]
                            .down
                            .push_back((due, Msg::Data(frame.encode())));
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(i) = self.conns.remove(&connection) {
                        let due = self.due(self.clients[i].down.back().map(|(t, _)| *t));
                        self.clients[i].down.push_back((due, Msg::Close));
                    }
                }
                RelayOutput::Store(op) => self.apply_store(op),
            }
        }
    }

    fn apply_store(&mut self, op: StoreOp) {
        let room_of = |room: RoomId| StoredRoom {
            room,
            keyring: Vec::new(),
            snapshot: None,
            snapshot_rest: Vec::new(),
            records: Vec::new(),
        };
        match op {
            StoreOp::Keyring { room, entry } => self
                .store
                .entry(room)
                .or_insert_with(|| room_of(room))
                .keyring
                .push(entry),
            StoreOp::Record { room, record } => self
                .store
                .entry(room)
                .or_insert_with(|| room_of(room))
                .records
                .push(record),
            StoreOp::Snapshot {
                room,
                covers_seq,
                envelope,
            } => {
                let stored = self.store.entry(room).or_insert_with(|| room_of(room));
                stored.records.retain(|r| r.seq > covers_seq);
                stored.snapshot = Some((covers_seq, envelope));
                stored.snapshot_rest.clear();
            }
            StoreOp::SnapshotPart {
                room,
                index,
                envelope,
                ..
            } => {
                let staged = self.staged.entry(room).or_default();
                if index == 0 {
                    staged.clear();
                }
                staged.push(envelope);
            }
            StoreOp::SnapshotParts {
                room, covers_seq, ..
            } => {
                let mut parts = self.staged.remove(&room).unwrap_or_default().into_iter();
                let stored = self.store.entry(room).or_insert_with(|| room_of(room));
                stored.records.retain(|r| r.seq > covers_seq);
                stored.snapshot = parts.next().map(|first| (covers_seq, first));
                stored.snapshot_rest = parts.collect();
            }
            StoreOp::DropStagedParts { room } => {
                self.staged.remove(&room);
            }
        }
    }

    /// Carries out everything client `i`'s session asked for, until it asks for nothing.
    pub fn pump(&mut self, i: usize) {
        loop {
            let Some(session) = self.clients[i].session.as_mut() else {
                return;
            };
            let outputs = session.drain();
            if outputs.is_empty() {
                return;
            }
            for output in outputs {
                self.carry_out(i, output);
            }
        }
    }

    fn carry_out(&mut self, i: usize, output: ClientOutput) {
        let now = self.now;
        match output {
            ClientOutput::Connect => {
                self.clients[i].connect_times.push(now);
                if self.clients[i].link_up {
                    let conn = self.relay.connect();
                    self.conns.insert(conn, i);
                    self.clients[i].conn = Some(conn);
                    if let Some(s) = self.clients[i].session.as_mut() {
                        s.connected(now);
                    }
                } else if let Some(s) = self.clients[i].session.as_mut() {
                    s.disconnected(now);
                }
            }
            ClientOutput::Send(bytes) => {
                if self.clients[i].link_up && self.clients[i].conn.is_some() {
                    self.clients[i].sent.push(bytes.clone());
                    let due = self.due(self.clients[i].up.back().map(|(t, _)| *t));
                    self.clients[i].up.push_back((due, Msg::Data(bytes)));
                }
            }
            ClientOutput::Close => self.drop_connection(i),
            ClientOutput::Event(event) => {
                self.apply(i, &event);
                self.clients[i].events.push(event);
            }
        }
    }

    /// What the editor's session does with each event (ADR 0031 decisions 3, 9, 10).
    fn apply(&mut self, i: usize, event: &SessionEvent) {
        let client = &mut self.clients[i];
        match event {
            SessionEvent::Remote { kind, bytes, .. } => match kind {
                RecordKind::Update => {
                    let doc = client.doc.as_ref().expect("an update before any snapshot");
                    if doc.import_updates(bytes).is_err() {
                        client.import_failures += 1;
                    }
                }
                RecordKind::Asset => {
                    client.doc().insert_asset(bytes).unwrap();
                }
                RecordKind::Snapshot => {
                    let peer = client.peer_id();
                    let received = unpack(peer, bytes).unwrap();
                    match &client.doc {
                        None => client.doc = Some(received),
                        Some(doc) => {
                            let all = received.export_updates(&DocumentVersion::never()).unwrap();
                            if doc.import_updates(&all).is_err() {
                                client.import_failures += 1;
                            }
                        }
                    }
                }
            },
            SessionEvent::Acked { client_id, .. } => {
                if let Some(version) = client.pushed_versions.remove(client_id) {
                    client.synced_version = version;
                }
            }
            SessionEvent::SnapshotDue { covers_seq } => {
                let bytes = pack(client.doc(), "sim").unwrap();
                let covers = *covers_seq;
                client
                    .session
                    .as_mut()
                    .unwrap()
                    .upload_snapshot(&bytes, covers)
                    .unwrap();
            }
            SessionEvent::KeyringChanged => {
                client.epoch_keys = client.session().epoch_keys();
            }
            _ => {}
        }
    }

    /// Whether every listed client holds a replica at the same version with the same content.
    pub fn converged(&self, clients: &[usize]) -> bool {
        let Some(first) = self.clients[clients[0]].doc.as_ref() else {
            return false;
        };
        let json = first.export_json().unwrap();
        clients.iter().all(|&i| {
            self.clients[i]
                .doc
                .as_ref()
                .is_some_and(|d| d.version() == first.version() && d.export_json().unwrap() == json)
        })
    }
}

/// Device 1 shares `doc` in `room`; each further device joins by its own invite. Returns the
/// simulator and the clients (sharer first), converged.
pub fn shared(seed: u64, room: RoomId, doc: Document, joiners: &[u8]) -> (Net, Vec<usize>) {
    let mut net = Net::new(seed);
    let a = net.add_client(1);
    net.share(a, room, doc);
    net.run(500);
    let mut clients = vec![a];
    for &device in joiners {
        let secret = net.invite(a, 24 * 3600);
        let i = net.add_client(device);
        net.join(i, secret);
        clients.push(i);
        let all = clients.clone();
        assert!(net.run_until(5_000, |n| n.converged(&all)).is_some());
    }
    net.run(500);
    (net, clients)
}
