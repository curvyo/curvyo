//! Tester acceptance tests for `specs/0116-library-sync` slice 2 at the sync-core level
//! (criteria 16, 19 and 24 for fonts), written from the specification and ADR 0062 before the
//! blob module was read. The blob layer (ADR 0062 decision 3: a font file's bytes as parts of at
//! most 4 MiB, keyed by SHA-256, announced with a total the receiver checks before it holds
//! anything) is tested as a pure layer; the relay scan pushes the parts as sealed `Asset`
//! records between two client sessions of one group room and captures every byte the server
//! sees. The journey-level font tests are `curvyo-editor-wasm/tests/acceptance_0116_fonts_tester.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use std::collections::VecDeque;

use curvyo_crypto_core::keyring::Role;
use curvyo_sync_core::{
    BlobAssembler, ClientOutput, ClientSession, ConnectionId, MAX_BLOB_BYTES, Millis, RecordKind,
    Relay, RelayOutput, RoomId, SessionConfig, SessionEvent, SessionStart, StoreOp,
    decode_blob_entry, decode_blob_part, encode_blob_entry, encode_blob_parts,
};
use support::{device, seed};

const MIB: usize = 1024 * 1024;
const GROUP_ROOM: RoomId = RoomId([0x6f; 16]);
const PEER_A: u64 = 11;
const PEER_B: u64 = 12;
const START: Millis = Millis(1_800_000_000_000);
const FAMILY: &[u8] = b"ZQX-FONT-7731";
const TAG: &[u8] = b"ZQX-TAG-7731";

/// A high-entropy file, so a scan cannot match by chance.
fn file_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            u8::try_from(x >> 56).unwrap_or(0)
        })
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

const HASH: [u8; 32] = [0x5a; 32];

/// Criterion 16 (layer), as the blob module documents it: "start with the blob's first part", then
/// the rest of the parts in order. The start's part is kept, so the file assembles back exactly.
#[test]
fn c16_a_file_splits_into_parts_of_at_most_4_mib_and_assembles_back() {
    let bytes = file_bytes(1, 9 * MIB);
    let parts = encode_blob_parts(&HASH, &bytes);
    assert_eq!(parts.len(), 3, "9 MiB is three parts of at most 4 MiB");
    for part in &parts {
        assert!(
            part.len() <= 4 * MIB + 128,
            "a part stays within PART_BYTES plus its header"
        );
    }
    let mut assembler = BlobAssembler::start(&parts[0]).unwrap();
    assert_eq!(
        assembler.hash(),
        &HASH,
        "the part names the hash it is keyed by"
    );
    let mut rebuilt = None;
    for part in &parts[1..] {
        rebuilt = assembler.push(part).unwrap();
    }
    assert_eq!(
        rebuilt.as_deref(),
        Some(bytes.as_slice()),
        "the bytes come back exactly"
    );
}

/// Criterion 16 (layer): a blob entry of the group snapshot round-trips to its hash and bytes.
#[test]
fn c16_a_blob_entry_round_trips_to_its_hash_and_bytes() {
    let bytes = file_bytes(2, 70_000);
    let entry = encode_blob_entry(&HASH, &bytes);
    let (hash, decoded) = decode_blob_entry(&entry).expect("a blob entry decodes");
    assert_eq!(hash, HASH);
    assert_eq!(decoded, bytes.as_slice());
}

/// Criterion 19 (layer): a file of 16 MiB + 1 byte is announced with a total over the limit;
/// its first part is refused before any byte is held (ADR 0062 decision 6).
#[test]
fn c19_an_announcement_over_16_mib_is_refused_with_its_first_part() {
    assert_eq!(
        MAX_BLOB_BYTES,
        16 * 1024 * 1024,
        "the per-file budget of ADR 0062"
    );
    let parts = encode_blob_parts(&HASH, &file_bytes(3, 16 * MIB + 1));
    assert!(
        BlobAssembler::start(&parts[0]).is_err(),
        "a 16 MiB + 1 announcement is refused at its first part"
    );
}

/// Criterion 19 edge: exactly 16 MiB is within the limit and assembles (`start` takes part 0,
/// `push` the rest).
#[test]
fn c19_exactly_16_mib_is_accepted_by_the_layer() {
    let bytes = file_bytes(4, 16 * MIB);
    let parts = encode_blob_parts(&HASH, &bytes);
    let mut assembler = BlobAssembler::start(&parts[0]).unwrap();
    let mut rebuilt = None;
    for part in &parts[1..] {
        rebuilt = assembler.push(part).unwrap();
    }
    assert_eq!(rebuilt.as_deref(), Some(bytes.as_slice()));
}

/// A part that is not a blob part at all is refused, never read as bytes.
#[test]
fn c16_a_truncated_part_is_refused() {
    assert!(
        decode_blob_part(&[1, 2, 3]).is_err(),
        "a stub of a header is refused"
    );
}

// ---- criterion 24 (relay): a font's blob parts on the wire and in the store --------------------

struct Device {
    session: ClientSession,
    conn: Option<ConnectionId>,
    assembler: Option<BlobAssembler>,
    rebuilt: Vec<Vec<u8>>,
}

/// Two devices around one relay; `wire` captures every byte either way, `stored` what the
/// relay's store keeps, as the server's data directory and logs would.
struct Net {
    relay: Relay,
    now: Millis,
    devices: Vec<Device>,
    nonce: u8,
    wire: Vec<u8>,
    stored: Vec<u8>,
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            now: START,
            devices: Vec::new(),
            nonce: 0,
            wire: Vec::new(),
            stored: Vec::new(),
        }
    }

    fn add(&mut self, n: u8, peer: u64, start: SessionStart) -> usize {
        let mut session = ClientSession::new(
            SessionConfig {
                room: GROUP_ROOM,
                identity: device(n),
                peer_id: peer,
                nonce_seed: seed(n, 1),
                entropy_seed: seed(n, 2),
                since_seq: 0,
                start,
            },
            self.now,
        )
        .unwrap();
        session.use_workspace_snapshots();
        self.devices.push(Device {
            session,
            conn: None,
            assembler: None,
            rebuilt: Vec::new(),
        });
        self.devices.len() - 1
    }

    fn pump(&mut self) {
        let mut queue: VecDeque<(usize, Vec<u8>)> = VecDeque::new();
        loop {
            let mut moved = false;
            for i in 0..self.devices.len() {
                for output in self.devices[i].session.drain() {
                    moved = true;
                    match output {
                        ClientOutput::Connect => {
                            let conn = self.relay.connect();
                            self.devices[i].conn = Some(conn);
                            self.devices[i].session.connected(self.now);
                        }
                        ClientOutput::Send(bytes) => {
                            self.wire.extend_from_slice(&bytes);
                            let Some(conn) = self.devices[i].conn else {
                                continue;
                            };
                            self.nonce = self.nonce.wrapping_add(1);
                            let out = self.relay.receive(conn, &bytes, self.now, [self.nonce; 32]);
                            self.route(out);
                        }
                        ClientOutput::Close => {
                            if let Some(conn) = self.devices[i].conn.take() {
                                let out = self.relay.disconnect(conn);
                                self.route(out);
                            }
                        }
                        ClientOutput::Event(event) => self.on_event(i, &event),
                    }
                }
            }
            while let Some((to, bytes)) = queue.pop_front() {
                moved = true;
                self.devices[to].session.received(&bytes, self.now);
            }
            if !moved {
                return;
            }
        }
    }

    /// An asset record of the room is one blob part; the receiver starts its assembler with
    /// the first part and pushes the rest (criterion 16 on the wire).
    fn on_event(&mut self, i: usize, event: &SessionEvent) {
        if let SessionEvent::Remote {
            kind: RecordKind::Asset,
            bytes,
            ..
        } = event
        {
            let device = &mut self.devices[i];
            let done = if let Some(assembler) = device.assembler.as_mut() {
                assembler.push(bytes).ok().flatten()
            } else {
                let mut started = BlobAssembler::start(bytes).ok();
                let done = started.as_mut().and_then(BlobAssembler::take);
                device.assembler = started;
                done
            };
            if let Some(done) = done {
                device.rebuilt.push(done);
                device.assembler = None;
            }
        }
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    let bytes = frame.encode();
                    self.wire.extend_from_slice(&bytes);
                    if let Some(i) = self.devices.iter().position(|d| d.conn == Some(connection)) {
                        self.devices[i].session.received(&bytes, self.now);
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(i) = self.devices.iter().position(|d| d.conn == Some(connection)) {
                        self.devices[i].conn = None;
                        self.devices[i].session.disconnected(self.now);
                    }
                }
                RelayOutput::Store(op) => match &op {
                    StoreOp::Keyring { entry, .. } => self.stored.extend_from_slice(entry),
                    StoreOp::Record { record, .. } => {
                        self.stored.extend_from_slice(&record.envelope);
                    }
                    StoreOp::Snapshot { envelope, .. } | StoreOp::SnapshotPart { envelope, .. } => {
                        self.stored.extend_from_slice(envelope);
                    }
                    _ => {}
                },
            }
        }
    }

    fn run(&mut self, ms: u64) {
        let until = self.now.plus(ms);
        while self.now < until {
            self.now = self.now.plus(10);
            for device in &mut self.devices {
                device.session.tick(self.now);
            }
            self.pump();
        }
    }
}

/// Criteria 16 and 24 at the relay: a 3 MiB font's blob parts pushed as sealed asset records by
/// A arrive rebuilt on B, and neither the wire nor the store holds the family name, the tag, a
/// chunk of the file's bytes, or the epoch keys of the room.
#[test]
fn c16_c24_font_blob_parts_arrive_whole_and_the_server_sees_only_ciphertext() {
    let mut net = Net::new();
    let a = net.add(1, PEER_A, SessionStart::Share);
    net.pump();
    net.run(100);
    net.devices[a]
        .session
        .add_member(device(2).identity(), Role::Admin)
        .unwrap();
    net.pump();
    let b = net.add(
        2,
        PEER_B,
        SessionStart::Member {
            keyring: Vec::new(),
            epoch_keys: Vec::new(),
        },
    );
    net.run(500);
    assert!(net.devices[b].session.is_online(), "B joined the room");

    let mut bytes = file_bytes(7, 3 * MIB);
    bytes[..FAMILY.len()].copy_from_slice(FAMILY);
    let parts = encode_blob_parts(&HASH, &bytes);
    for part in &parts {
        net.devices[a]
            .session
            .push(RecordKind::Asset, part.clone())
            .unwrap();
    }
    net.run(2_000);
    assert_eq!(net.devices[b].rebuilt.len(), 1, "B rebuilt the font once");
    assert_eq!(
        net.devices[b].rebuilt[0], bytes,
        "the bytes arrive rebuilt and exact"
    );

    let chunk = &bytes[1_000..1_032];
    for (label, needle) in [
        ("the family name", FAMILY),
        ("the tag", TAG),
        ("a chunk of the file", chunk),
    ] {
        assert!(!contains(&net.wire, needle), "{label} is on the wire");
        assert!(
            !contains(&net.stored, needle),
            "{label} is in the server's store"
        );
    }
    assert!(
        !net.stored.is_empty(),
        "the scan saw the server's store writes"
    );
}
