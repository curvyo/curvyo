//! Tester acceptance tests for `specs/0116-library-sync` slice 1 at the relay level: the
//! server-data scan of criterion 24 (with criterion 25's key scan), written from the
//! specification and ADR 0062 before the session glue was read. As the 0096 criterion-30 scan
//! (`acceptance_0096_tester.rs` `criteria_3_30_nothing_secret_on_the_wire`) models the pairing,
//! this models the group room itself: two client sessions of the device group around one
//! in-memory `Relay`, A pushing the library stream's records (a machine, a material with its
//! judged test's note, an edit, a switch-off marker) while the harness captures every byte on
//! the wire and everything the relay stores. None of the plaintext markers may appear in
//! either; the relay sees only ciphertext rooms, record sizes and connection times.
//!
//! The font markers `ZQX-FONT-7731` (the family name) and `ZQX-TAG-7731` (the tag) of criterion
//! 24 cannot travel yet (fonts are slice 2); they are scanned anyway so slice 2's journey keeps
//! them. The journey-level scan through the whole story is
//! `curvyo-editor-wasm/tests/acceptance_0116_tester.rs` `c24_c25_c26_…`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use std::collections::VecDeque;

use curvyo_crypto_core::keyring::Role;
use curvyo_document_core::{DocumentVersion, LibraryKind, LibraryList};
use curvyo_sync_core::{
    ClientOutput, ClientSession, ConnectionId, GroupRecord, Millis, RecordKind, Relay, RelayOutput,
    RoomId, SessionConfig, SessionEvent, SessionStart, StoreOp, encode_library_record,
    route_group_record,
};
use support::{device, seed};

/// The device group's room, as the 0096 pairing hands it over.
const GROUP_ROOM: RoomId = RoomId([0x6e; 16]);
const PEER_A: u64 = 11;
const PEER_B: u64 = 12;
const START: Millis = Millis(1_800_000_000_000);
const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const MATERIAL: &str = "5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875";

/// P1's record text: a user machine profile with its connection fields (criterion 15's leak
/// candidate).
const MACHINE_TEXT: &str = "{
  \"format_version\": 4,
  \"id\": \"6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f\",
  \"kind\": \"laser\",
  \"modified\": 1762948805000,
  \"name\": \"ZQX-MACH-7731\",
  \"work_area_mm\": {
    \"width\": 600.0,
    \"height\": 400.0
  },
  \"origin\": \"top_left\",
  \"max_feed_mm_per_min\": 12000,
  \"max_power\": 255,
  \"laser_mode\": false,
  \"motorised_z\": false,
  \"connection\": {
    \"connector_address\": \"192.168.7.31:8080\",
    \"controller_port\": \"/dev/ttyUSB0\",
    \"baud_rate\": 115200
  },
  \"fixtures\": []
}";

/// M1's record text: a material with one judged test whose note is the scan's note marker.
const MATERIAL_TEXT: &str = "{
  \"format_version\": 2,
  \"id\": \"5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875\",
  \"kind\": \"laser\",
  \"modified\": 1762948805000,
  \"name\": \"ZQX-MAT-7731\",
  \"thickness_mm\": 3.0,
  \"lasers\": [],
  \"tests\": [
    {
      \"id\": \"7K2QX\",
      \"created\": 1762948800000,
      \"machine\": {
        \"id\": \"generic-laser\",
        \"name\": \"Generic laser\"
      },
      \"recipe\": {
        \"kind\": \"cut\",
        \"columns\": {
          \"param\": \"power\",
          \"min\": 20.0,
          \"max\": 100.0,
          \"count\": 5
        },
        \"rows\": {
          \"param\": \"speed\",
          \"min\": 600,
          \"max\": 6000,
          \"count\": 5
        },
        \"fixed\": {
          \"passes\": 1
        },
        \"cell_mm\": 10.0,
        \"crosshatch\": false
      },
      \"lasered\": 1762948801000,
      \"best\": {
        \"column\": 2,
        \"row\": 1
      },
      \"note\": \"ZQX-NOTE-7731\",
      \"photo\": null
    }
  ],
  \"history\": []
}";

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

/// One device of the group: its session, its library list replica and the version the last
/// push exported from (the glue starts at `never()`, so a fresh replica can apply: a list's own
/// first change travels with its first push).
struct Device {
    session: ClientSession,
    library: LibraryList,
    exported: DocumentVersion,
    conn: Option<ConnectionId>,
    /// Whether the journey's records were received and applied (the control of the scan).
    applied: usize,
}

/// Two devices around one relay; `wire` captures every byte in either direction, `stored`
/// everything the relay's store keeps (keyring entries, record and snapshot envelopes), as the
/// server's database files and logs would.
struct Net {
    relay: Relay,
    now: Millis,
    devices: Vec<Device>,
    nonce: u8,
    wire: Vec<u8>,
    stored: Vec<u8>,
    stored_records: usize,
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
            stored_records: 0,
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
            library: LibraryList::new(peer),
            exported: DocumentVersion::never(),
            conn: None,
            applied: 0,
        });
        self.devices.len() - 1
    }

    /// Sets one entry of device `i`'s library and pushes it as a library record of the room.
    fn set(&mut self, i: usize, kind: LibraryKind, id: &str, text: &str) {
        let device = &mut self.devices[i];
        device.library.set(kind, id, text).unwrap();
        let bytes = device.library.export_updates(&device.exported).unwrap();
        let record = encode_library_record(PEER_A, &bytes);
        device.session.push(RecordKind::Update, record).unwrap();
        device.exported = device.library.version();
    }

    /// Switches one entry off and pushes the marker (ADR 0062 decision 4).
    fn switch_off(&mut self, i: usize, kind: LibraryKind, id: &str) {
        let device = &mut self.devices[i];
        device.library.switch_off(kind, id).unwrap();
        let bytes = device.library.export_updates(&device.exported).unwrap();
        let record = encode_library_record(PEER_A, &bytes);
        device.session.push(RecordKind::Update, record).unwrap();
        device.exported = device.library.version();
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

    fn on_event(&mut self, i: usize, event: &SessionEvent) {
        // The session-glue's routing (ADR 0062 decision 1): an update record with the inner
        // header is the library stream's.
        if let SessionEvent::Remote {
            kind: RecordKind::Update,
            bytes,
            ..
        } = event
            && let Ok(GroupRecord::Library { payload, .. }) = route_group_record(bytes)
            && self.devices[i].library.import_updates(payload).is_ok()
        {
            self.devices[i].applied += 1;
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
                RelayOutput::Store(op) => {
                    if matches!(op, StoreOp::Record { .. }) {
                        self.stored_records += 1;
                    }
                    match &op {
                        StoreOp::Keyring { entry, .. } => self.stored.extend_from_slice(entry),
                        StoreOp::Record { record, .. } => {
                            self.stored.extend_from_slice(&record.envelope);
                        }
                        StoreOp::Snapshot { envelope, .. }
                        | StoreOp::SnapshotPart { envelope, .. } => {
                            self.stored.extend_from_slice(envelope);
                        }
                        _ => {}
                    }
                }
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

/// Criteria 24 and 25: after a library journey through the relay (P1 and M1 with the judged
/// test's note arrive on B, P1 takes an edit, then the switch-off marker — criteria 5, 7 and
/// 10 at the relay level), the server's stored data and every byte on the wire hold none of
/// the plaintext markers and no key material: only ciphertext.
#[test]
fn criteria_24_25_the_relay_store_holds_only_ciphertext() {
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
    assert!(net.devices[b].conn.is_some(), "B joined the room");

    // The journey: P1 and M1 (with the note) arrive, P1 takes an edit, then the switch-off.
    net.set(a, LibraryKind::Machine, MACHINE, MACHINE_TEXT);
    net.set(a, LibraryKind::Material, MATERIAL, MATERIAL_TEXT);
    net.run(200);
    let machine = net.devices[b]
        .library
        .entry(LibraryKind::Machine, MACHINE)
        .unwrap();
    assert_eq!(machine.text.as_deref(), Some(MACHINE_TEXT));
    let material = net.devices[b]
        .library
        .entry(LibraryKind::Material, MATERIAL)
        .unwrap();
    assert_eq!(material.text.as_deref(), Some(MATERIAL_TEXT));
    net.set(
        a,
        LibraryKind::Machine,
        MACHINE,
        &MACHINE_TEXT.replacen("ZQX-MACH-7731", "ZQX-MACH-7731 K40", 1),
    );
    net.run(200);
    net.switch_off(a, LibraryKind::Machine, MACHINE);
    net.run(500);

    // The journey stored library records and B applied them; the scan is not vacuous.
    assert!(
        net.stored_records > 0,
        "the journey stored records at the relay"
    );
    assert!(net.devices[b].applied >= 3, "B applied the records");
    let off = net.devices[b]
        .library
        .entry(LibraryKind::Machine, MACHINE)
        .unwrap();
    assert!(!off.sync && off.text.is_none(), "the marker entry");

    // Criterion 24: none of the plaintext markers in the server's stored data or on the wire.
    for bytes in [&net.stored, &net.wire] {
        for marker in [
            &b"ZQX-FONT-7731"[..],
            b"ZQX-MACH-7731",
            b"ZQX-MAT-7731",
            b"ZQX-TAG-7731",
            b"ZQX-NOTE-7731",
            b"192.168.7.31",
        ] {
            assert!(
                !contains(bytes, marker),
                "the server's data holds the plaintext marker {}",
                String::from_utf8_lossy(marker)
            );
        }
    }

    // Criterion 25: no device key and no epoch key anywhere on the server.
    for bytes in [&net.stored, &net.wire] {
        for n in [1, 2] {
            let entropy = device(n).to_entropy();
            for window in entropy.chunks(16) {
                assert!(
                    !contains(bytes, window),
                    "device {n}'s identity entropy is on the server"
                );
            }
        }
        for device in &net.devices {
            for (_, key) in device.session.epoch_keys() {
                assert!(
                    !contains(bytes, key.as_bytes()),
                    "an epoch key is on the server"
                );
            }
        }
    }
}
