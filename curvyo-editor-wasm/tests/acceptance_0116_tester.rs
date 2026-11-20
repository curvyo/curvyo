//! Tester acceptance tests for `specs/0116-library-sync` slice 1 (criteria 1 to 15 and 21 to
//! 27), written from the specification, the UX notes and ADR 0062 before this diff was read:
//! two or three desktop devices, each with its group glue and its session, around one
//! in-memory relay, driven only through the public session and glue surfaces (the page's
//! pumps stand in for the app). A holds P1 `ZQX-MACH-7731` and M1 `ZQX-MAT-7731` with one
//! judged test, B holds neither; both hold the Included fonts and Generic laser (shipped).
//! Where a criterion is marked [ADR] the accepted ADR 0062 decides the expected behaviour.
//! The frontend parts of criteria 1 to 4 and 13 (the rows, the S key, the greyed reason, the
//! confirm, the browser storage keys) are covered by `frontend/tests/acceptance_0116_tester.test.ts`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::too_many_lines
)]

use curvyo_crypto_core::{Identity, IdentityKeys};
use curvyo_document_core::{
    Document, DocumentSize, Length, LibraryEntry, LibraryKind, NewOutputPage, Timestamp, pack,
};
use curvyo_editor_wasm::{DeviceGroupGlue, Session, SyncSeeds, TransportAction};
use curvyo_laser_core::grids::{RecipeDefaults, TestId, TestRecipe};
use curvyo_laser_core::library::{Material, TestRecord, write_record as write_material};
use curvyo_laser_core::settings::{CellIndex, MachineTie, Thickness};
use curvyo_laser_core::{
    FeedRate, MachineProfile, record_file_name, write_choice, write_record as write_machine,
};
use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput, StoreOp};
use curvyo_ui_core::library_text as texts;
use serde_json::Value;

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];
const P1: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const M1: &str = "5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875";
const BIRCH: &str = "8a7b6c5d-4e3f-4a2b-9c1d-0e9f8a7b6c5d";
/// A browser device's own machine pick (criterion 14): the id its `machine-choice.json` names.
const PICK: &str = "7c8d9e0f-1a2b-4c3d-8e7f-6a5b4c3d2e1f";
const TEST: &str = "ZQX77";
/// The clocks: B's runs after A's, C's after B's, so a B edit is the newer `modified`.
const A_NOW_MS: i64 = 1_762_948_800_000;
const B_NOW_MS: i64 = 1_762_949_000_000;

/// The devices' identity entropy: a high-entropy pattern per device, so the server-data scan
/// of criteria 24 and 25 can look for chunks of it without matching ordinary protocol bytes
/// (runs of small integers occur in the sealed streams' framed headers).
/// High-entropy names for the history scans: LZ4 would hide a repeated name in a snapshot.
const OLD_MARK: &str = "Qk7vX9mZpL2wR8sT4nB6yH1";
const OFF_MARK: &str = "Wz3Nq8Rt5Yh2Lp9Xc4Vb7Mk";

fn identity_entropy(n: u8) -> [u8; 64] {
    let mut entropy = [0_u8; 64];
    for (i, slot) in entropy.iter_mut().enumerate() {
        *slot = n
            .wrapping_mul(0x37)
            .wrapping_add((u8::try_from(i).unwrap_or(0)).wrapping_mul(0x5D))
            .rotate_left(3)
            ^ 0xA5;
    }
    entropy
}

fn seeds(n: u8) -> SyncSeeds {
    SyncSeeds {
        identity: identity_entropy(n),
        nonce: [n.wrapping_add(100); 32],
        entropy: [n.wrapping_add(200); 32],
    }
}

fn identity(n: u8) -> Identity {
    IdentityKeys::from_entropy(&identity_entropy(n)).identity()
}

fn clock_a() -> Timestamp {
    Timestamp::from_secs(1_762_948_800)
}

fn clock_b() -> Timestamp {
    Timestamp::from_secs(1_762_949_000)
}

fn clock_c() -> Timestamp {
    Timestamp::from_secs(1_762_950_000)
}

/// P1's record text: a user profile named `name` carrying its connection fields (criterion 15).
fn machine_text(id: &str, name: &str, modified: i64) -> String {
    let mut profile = MachineProfile::generic_laser();
    id.clone_into(&mut profile.id);
    profile.modified = modified;
    name.clone_into(&mut profile.values.name);
    "192.168.7.31".clone_into(&mut profile.connection.connector_address);
    "/dev/ttyUSB0".clone_into(&mut profile.connection.controller_port);
    write_machine(&profile)
}

fn judged_test() -> TestRecord {
    TestRecord {
        id: TestId::parse(TEST).unwrap(),
        created: 1_800_000_000_000,
        machine: MachineTie {
            id: "generic-laser".to_owned(),
            name: "Generic laser".to_owned(),
        },
        recipe: TestRecipe::new(&RecipeDefaults {
            max_feed: FeedRate::DEFAULT,
            set: None,
        }),
        lasered: Some(1_800_000_000_000),
        best: Some(CellIndex { column: 2, row: 1 }),
        note: "ZQX-NOTE-7731".to_owned(),
        photo: None,
    }
}

/// M1's record text: named `name`, with one judged test and its note.
fn material_text(id: &str, name: &str, modified: u64) -> String {
    write_material(&Material {
        id: id.to_owned(),
        modified,
        name: name.to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        lasers: Vec::new(),
        tests: vec![judged_test()],
        history: Vec::new(),
    })
}

/// A material record without tests (B's own local row of criterion 12).
fn plain_material_text(id: &str, name: &str, modified: u64) -> String {
    write_material(&Material {
        id: id.to_owned(),
        modified,
        name: name.to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        lasers: Vec::new(),
        tests: Vec::new(),
        history: Vec::new(),
    })
}

/// A session with a laser page opened, so undo steps are observable (criterion 7).
fn doc_session(peer: u64, clock: fn() -> Timestamp) -> Session {
    let document = Document::new(peer);
    document
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut session = Session::open(peer, &pack(&document, "0116-tester").unwrap()).unwrap();
    session.set_clock(clock);
    let _ = session.wipe_history();
    session
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

/// One write's kind, from either host write list ("kind" for machines, "op" for materials).
fn write_kind(write: &Value) -> &str {
    write["kind"]
        .as_str()
        .or_else(|| write["op"].as_str())
        .unwrap_or("")
}

/// One device: its group glue, its session, and what the page collected of it.
struct Device {
    glue: DeviceGroupGlue,
    session: Session,
    connection: Option<ConnectionId>,
    offline: bool,
    writes: Vec<Value>,
    notices: Vec<String>,
    keys: Vec<(u32, [u8; 32])>,
}

/// The devices around one in-memory relay; `stored` and `wire` collect everything the server
/// sees (its store writes and every byte on the wire), for the scans of criteria 24, 25 and 27.
struct Net {
    relay: Relay,
    devices: Vec<Device>,
    now: u64,
    entropy: u8,
    stored: Vec<u8>,
    wire: Vec<u8>,
    stored_records: usize,
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            devices: Vec::new(),
            now: 1_800_000_000_000,
            entropy: 0,
            stored: Vec::new(),
            wire: Vec::new(),
            stored_records: 0,
        }
    }

    fn add(&mut self, glue: DeviceGroupGlue, session: Session) -> usize {
        self.devices.push(Device {
            glue,
            session,
            connection: None,
            offline: false,
            writes: Vec::new(),
            notices: Vec::new(),
            keys: Vec::new(),
        });
        self.devices.len() - 1
    }

    fn glue(&mut self, index: usize) -> &mut DeviceGroupGlue {
        &mut self.devices[index].glue
    }

    fn session(&mut self, index: usize) -> &mut Session {
        &mut self.devices[index].session
    }

    /// The page's slow poll: the host write lists (which tap the local edits), then the library
    /// pump (which feeds the edits to the group's list and applies the arrivals).
    fn page(&mut self, index: usize) {
        let device = &mut self.devices[index];
        for list in [
            device.session.take_machine_writes_json(),
            device.session.take_material_writes_json(),
        ] {
            if let Ok(taken) = serde_json::from_str::<Vec<Value>>(&list) {
                device.writes.extend(taken);
            }
        }
        let notice = device.session.library_sync_pump(&mut device.glue);
        if !notice.is_empty() {
            device.notices.push(notice);
        }
    }

    fn step(&mut self) -> bool {
        let now = Millis(self.now);
        let mut moved = false;
        for index in 0..self.devices.len() {
            while let Some(action) = self.devices[index].glue.next_action() {
                moved = true;
                match action {
                    TransportAction::Connect(_) => {
                        if self.devices[index].offline {
                            self.devices[index].glue.disconnected(now);
                        } else {
                            let connection = self.relay.connect();
                            self.devices[index].connection = Some(connection);
                            self.devices[index].glue.connected(now);
                        }
                    }
                    TransportAction::Send(bytes) => {
                        let Some(connection) = self.devices[index].connection else {
                            continue;
                        };
                        self.wire.extend_from_slice(&bytes);
                        self.entropy = self.entropy.wrapping_add(1);
                        let out = self
                            .relay
                            .receive(connection, &bytes, now, [self.entropy; 32]);
                        self.route(out);
                    }
                    TransportAction::Close => {
                        if let Some(connection) = self.devices[index].connection.take() {
                            let out = self.relay.disconnect(connection);
                            self.route(out);
                        }
                    }
                }
            }
            if let Some(keys) = self.devices[index].glue.take_key_update() {
                self.devices[index].keys = keys;
            }
        }
        moved
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        let now = Millis(self.now);
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    let bytes = frame.encode();
                    self.wire.extend_from_slice(&bytes);
                    if let Some(device) = self
                        .devices
                        .iter_mut()
                        .find(|d| d.connection == Some(connection))
                    {
                        device.glue.received(&bytes, now);
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(device) = self
                        .devices
                        .iter_mut()
                        .find(|d| d.connection == Some(connection))
                    {
                        device.connection = None;
                        device.glue.disconnected(now);
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
        let end = self.now + ms;
        while self.now < end {
            self.now += 50;
            for device in &mut self.devices {
                device.glue.tick(Millis(self.now));
            }
            for index in 0..self.devices.len() {
                self.page(index);
            }
            let mut guard = 0;
            while self.step() {
                guard += 1;
                assert!(guard < 10_000, "does not settle");
            }
        }
    }

    /// Runs virtual time until `arrived` holds, at most `ms`: a LAN arrival budget.
    fn within(&mut self, ms: u64, mut arrived: impl FnMut(&Net) -> bool) -> bool {
        let end = self.now + ms;
        while self.now < end {
            self.run(250);
            if arrived(&*self) {
                return true;
            }
        }
        arrived(&*self)
    }

    fn go_offline(&mut self, index: usize) {
        self.devices[index].offline = true;
        if let Some(connection) = self.devices[index].connection.take() {
            let out = self.relay.disconnect(connection);
            self.route(out);
            self.devices[index].glue.disconnected(Millis(self.now));
        }
    }

    fn go_online(&mut self, index: usize) {
        self.devices[index].offline = false;
        let now = Millis(self.now);
        self.devices[index].glue.network_back(now);
    }

    fn max_epoch(&self, index: usize) -> Option<u32> {
        self.devices[index].keys.iter().map(|(n, _)| *n).max()
    }

    /// Loads machine records into a device's session (a host read), as `[(id, name)]`.
    fn load_machines(&mut self, index: usize, records: &[(&str, &str)]) {
        let files: Vec<(String, String)> = records
            .iter()
            .map(|(id, name)| (record_file_name(id), machine_text(id, name, 5)))
            .collect();
        self.session(index).load_machine_profiles(&files);
    }

    /// Loads material records into a device's session, as `[(id, name)]`, `judged` for M1's.
    fn load_material(&mut self, index: usize, id: &str, name: &str, judged: bool) {
        let text = if judged {
            material_text(id, name, 5)
        } else {
            plain_material_text(id, name, 5)
        };
        self.session(index)
            .load_materials(&[(record_file_name(id), text)], None);
    }

    fn first_sync(&mut self, index: usize) -> usize {
        let device = &mut self.devices[index];
        device.session.library_first_sync(&mut device.glue)
    }

    fn machine(&self, index: usize, id: &str) -> Option<MachineProfile> {
        self.devices[index]
            .session
            .machine_profiles()
            .find(id)
            .cloned()
    }

    fn material_name(&self, index: usize, id: &str) -> Option<String> {
        self.devices[index]
            .session
            .materials()
            .find(id)
            .map(|material| material.name.clone())
    }

    fn material_note(&self, index: usize, id: &str) -> Option<String> {
        self.devices[index]
            .session
            .materials()
            .find(id)
            .and_then(|material| material.tests.first().map(|t| t.note.clone()))
    }

    /// The host write texts queued for `id` on `index` (whole record texts, criterion 7).
    fn wrote_texts(&self, index: usize, id: &str) -> Vec<&str> {
        self.devices[index]
            .writes
            .iter()
            .filter(|write| write["id"] == id && write_kind(write) == "write")
            .filter_map(|write| write["text"].as_str())
            .collect()
    }

    /// The conflict copy texts the device wrote for `kind` (ids ending in `.conflict`,
    /// criterion 9); machine writes carry `kind`, material writes `op`.
    fn conflict_texts(&self, index: usize, kind: LibraryKind) -> Vec<&str> {
        self.devices[index]
            .writes
            .iter()
            .filter(|write| {
                let id = write["id"].as_str();
                id.is_some_and(|id| id.ends_with(".conflict"))
                    && write_kind(write) == "write"
                    && match kind {
                        LibraryKind::Machine => write.get("kind").is_some(),
                        LibraryKind::Material => write.get("op").is_some(),
                    }
            })
            .filter_map(|write| write["text"].as_str())
            .collect()
    }

    fn entry(&self, index: usize, kind: LibraryKind, id: &str) -> Option<LibraryEntry> {
        self.devices[index].glue.library().entry(kind, id)
    }

    /// The device's user machine profiles (the built-in aside), for counting records.
    fn user_machines(&self, index: usize) -> Vec<&MachineProfile> {
        let profiles = self.devices[index].session.machine_profiles();
        profiles
            .listed()
            .into_iter()
            .filter(|profile| profiles.is_user(&profile.id))
            .collect()
    }
}

/// A and B paired as `0096` criteria 1 to 3 built them: A holds the group with P1 and M1
/// loaded, its first sync wrote both on (criterion 3), B joined and received both without any
/// input of its own (criterion 5). Everything is settled when this returns.
fn paired() -> (Net, usize, usize) {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        doc_session(11, clock_a),
    );
    net.run(500);
    assert!(net.devices[a].glue.is_online());
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    net.load_machines(a, &[(P1, "ZQX-MACH-7731")]);
    net.load_material(a, M1, "ZQX-MAT-7731", true);
    assert_eq!(
        net.first_sync(a),
        2,
        "the first sync writes every local item on"
    );
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        doc_session(12, clock_b),
    );
    net.run(1_500);
    (net, a, b)
}

/// Criteria 3, 5 and 15: the first sync writes every item on behind the page's confirm (the
/// sentence is checked below), B receives P1 and M1 within 30 s without any input, as whole
/// record texts with their `modified` field and the same values, P1's connection fields
/// included, with one notice of at most 70 characters, and every toggle reads on.
#[test]
fn c3_c5_c15_first_sync_arrives_without_input_with_the_same_values() {
    assert_eq!(
        texts::FIRST_SYNC_CONFIRM,
        "Your machines and materials leave this computer encrypted, to your own computers only.",
        "criterion 3's confirm sentence"
    );
    let (mut net, a, b) = paired();
    assert!(
        net.within(30_000, |net| {
            net.machine(b, P1).is_some() && net.material_name(b, M1).is_some()
        }),
        "P1 and M1 arrive within 30 s (criterion 5)"
    );
    let p1 = net.machine(b, P1).unwrap();
    assert_eq!(p1.values.name, "ZQX-MACH-7731");
    assert_eq!(p1.connection.connector_address, "192.168.7.31");
    assert_eq!(p1.connection.controller_port, "/dev/ttyUSB0");
    assert_eq!(
        p1.connection.baud_rate.get(),
        net.machine(a, P1).unwrap().connection.baud_rate.get(),
        "the connection fields travel with the record (criterion 15)"
    );
    let m1 = write_machine(net.machine(a, P1).as_ref().unwrap());
    assert!(
        net.wrote_texts(b, P1).contains(&m1.as_str()),
        "B's host write is the whole record text"
    );
    assert!(
        m1.contains("\"modified\""),
        "a whole file with a modified field"
    );
    assert!(
        net.wrote_texts(b, M1)
            .iter()
            .any(|text| text.contains("\"modified\"")),
        "the material record is written as a whole file too"
    );
    let arrived = net.devices[b]
        .notices
        .iter()
        .find(|notice| {
            notice.contains("machine") && notice.contains("material") && notice.contains("arrived")
        })
        .expect("an arrival notice");
    assert_eq!(
        *arrived, "1 machine profile and 1 material arrived from your devices.",
        "the batch counts per kind, kinds with none left out (UX notes section 5)"
    );
    assert!(arrived.chars().count() <= 70, "{arrived}");
    let judged = net.devices[b]
        .session
        .materials()
        .find(M1)
        .unwrap()
        .tests
        .first()
        .unwrap();
    assert_eq!(
        judged.best,
        Some(CellIndex { column: 2, row: 1 }),
        "the judged test travelled"
    );
    assert_eq!(judged.note, "ZQX-NOTE-7731");
    for index in [a, b] {
        let view: Value =
            serde_json::from_str(&net.devices[index].glue.library_view_json()).unwrap();
        assert_eq!(
            view["machines"][0]["sync"], true,
            "every toggle is on (criterion 3)"
        );
        assert_eq!(view["materials"][0]["sync"], true);
    }
}

/// Criterion 4: an item created afterwards (a duplicated profile) starts with its toggle on
/// (Frage 214 A) on every paired device, and an off item's text leaves the stream everywhere
/// while the local copies stay (nothing about it reaches the server).
#[test]
fn c4_a_new_item_starts_on_and_an_off_item_publishes_no_text() {
    let (mut net, a, b) = paired();
    let p2 = net
        .session(a)
        .duplicate_machine_profile(P1, [0x7f; 16], A_NOW_MS)
        .unwrap();
    net.page(a);
    assert!(
        net.within(30_000, |net| net.machine(b, &p2).is_some()),
        "the new item reaches B without input"
    );
    for index in [a, b] {
        let entry = net.entry(index, LibraryKind::Machine, &p2).unwrap();
        assert!(entry.sync, "a new item starts on (criterion 4)");
    }
    let device = &mut net.devices[a];
    assert!(
        !device
            .session
            .library_switch(&mut device.glue, LibraryKind::Machine, &p2, false)
    );
    net.run(1_000);
    for index in [a, b] {
        let entry = net.entry(index, LibraryKind::Machine, &p2).unwrap();
        assert!(!entry.sync);
        assert_eq!(
            entry.text, None,
            "the off item's record text is gone from the stream on every device"
        );
    }
    assert!(net.machine(b, &p2).is_some(), "B keeps its local copy");
}

/// Criterion 6 [ADR]: B offline when a new item is switched on, A offline when B returns: the
/// sealed record waits on the server and B receives it without any action on A.
#[test]
fn c6_the_sealed_record_waits_on_the_server_for_a_later_device() {
    let (mut net, a, b) = paired();
    net.go_offline(b);
    net.run(200);
    let p2 = net
        .session(a)
        .duplicate_machine_profile(P1, [0x7e; 16], A_NOW_MS)
        .unwrap();
    net.page(a);
    net.run(500);
    net.go_offline(a);
    net.run(200);
    net.go_online(b);
    assert!(
        net.within(30_000, |net| net.machine(b, &p2).is_some()),
        "the item arrives as in criterion 5"
    );
    assert!(!net.devices[a].glue.is_online(), "A stayed offline");
    assert_eq!(
        net.machine(b, &p2).unwrap().values.name,
        "ZQX-MACH-7731 copy"
    );
}

/// Criterion 7: A's edit of P1 reaches B within 2 s as the whole record text with its
/// `modified` field, and creates no document history step anywhere on B; the same for a
/// material edit (the note of M1's judged test).
#[test]
fn c7_edits_reach_the_other_device_as_whole_records_without_history_steps() {
    let (mut net, a, b) = paired();
    let undo_before = net.devices[b].session.undo_len();
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 K40", A_NOW_MS);
    net.page(a);
    assert!(
        net.within(2_000, |net| net
            .machine(b, P1)
            .is_some_and(|p1| p1.values.name == "ZQX-MACH-7731 K40")),
        "B takes the change within 2 s"
    );
    let expected = write_machine(net.machine(a, P1).as_ref().unwrap());
    assert!(
        net.wrote_texts(b, P1).contains(&expected.as_str()),
        "the whole record text with its modified field"
    );
    assert_eq!(
        net.devices[b].session.undo_len(),
        undo_before,
        "no document history step on B"
    );
    net.session(a)
        .set_test_note(M1, TEST, "ZQX-NOTE-7731 second")
        .unwrap();
    net.page(a);
    assert!(
        net.within(2_000, |net| net.material_note(b, M1).as_deref()
            == Some("ZQX-NOTE-7731 second")),
        "the material edit reaches B within 2 s"
    );
    assert_eq!(
        net.devices[b].session.undo_len(),
        undo_before,
        "no document history step for the material either"
    );
}

/// Criterion 8 [ADR]: A and B edit different fields of P1 while both are offline. ADR 0062
/// decision 5 decides the outcome (the spec's union-of-fields is its rejected future work):
/// the newer record wins on both devices, the losing edit is kept as the conflict copy on the
/// losing device, no edit is lost, and no duplicated record appears. (The criterion is [ADR];
/// the copy stays local per the ADR — the losing device writes it, the winning device, whose
/// register already holds the winner, never sees the loser's text and writes no copy — plan P10.)
#[test]
fn c8_different_field_offline_edits_converge_with_no_edit_lost() {
    let (mut net, a, b) = paired();
    net.go_offline(a);
    net.go_offline(b);
    net.run(200);
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 desktop", A_NOW_MS);
    let _ = net.session(b).set_profile_work_area(
        P1,
        Length::from_mm(500.0),
        Length::from_mm(400.0),
        B_NOW_MS,
    );
    net.page(a);
    net.page(b);
    net.go_online(a);
    net.go_online(b);
    net.run(3_000);
    let on_a = write_machine(net.machine(a, P1).as_ref().unwrap());
    let on_b = write_machine(net.machine(b, P1).as_ref().unwrap());
    assert_eq!(on_a, on_b, "both devices converge to the same record");
    // The newer edit (B's work area) won; A's name edit is kept as the conflict copy on A.
    assert_eq!(
        net.machine(a, P1).unwrap().work_area,
        DocumentSize::from_mm(500.0, 400.0)
    );
    assert!(
        net.conflict_texts(a, LibraryKind::Machine)
            .iter()
            .any(|text| text.contains("ZQX-MACH-7731 desktop (from your devices)")),
        "the loser is renamed and no edit is lost"
    );
    for index in [a, b] {
        assert_eq!(net.user_machines(index).len(), 1, "no duplicated record");
    }
    let conflict_counts: Vec<usize> = [a, b]
        .iter()
        .map(|i| net.conflict_texts(*i, LibraryKind::Machine).len())
        .collect();
    assert_eq!(
        conflict_counts,
        vec![1, 0],
        "the losing device keeps the copy, the winning device (peer order) writes none (P10)"
    );
}

/// Criterion 9 [ADR]: A and B rename the same field of P1 (and of M1) differently while
/// offline; on reconnect the newer `modified` wins on both devices, the losing version is kept
/// as a renamed copy on the losing device with a notice of at most 70 characters, and no edit
/// is lost. (The criterion's closing sentence "both devices end with the same two records"
/// holds only when the register order lets the winner see the loser's text — ADR 0062 decision
/// 5 keeps the copy local and plan P10 leaves that reading to the demo; reported as a finding.)
#[test]
fn c9_same_field_conflicts_keep_the_loser_and_converge() {
    let (mut net, a, b) = paired();
    net.go_offline(a);
    net.go_offline(b);
    net.run(200);
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 desktop", A_NOW_MS);
    let _ = net
        .session(b)
        .rename_machine_profile(P1, "ZQX-MACH-7731 laptop", B_NOW_MS);
    net.page(a);
    net.page(b);
    net.go_online(a);
    net.go_online(b);
    net.run(3_000);
    for index in [a, b] {
        assert_eq!(
            net.machine(index, P1).unwrap().values.name,
            "ZQX-MACH-7731 laptop",
            "the newer modified wins on both devices"
        );
    }
    // The losing version is kept as a renamed copy on the loser, with a notice naming it.
    assert!(
        net.conflict_texts(a, LibraryKind::Machine)
            .iter()
            .any(|text| text.contains("ZQX-MACH-7731 desktop (from your devices)")),
        "the losing version is kept as a renamed copy"
    );
    assert!(
        net.devices[a]
            .notices
            .iter()
            .any(|notice| notice.contains("Both copies are kept")
                && notice.contains("desktop")
                && notice.chars().count() <= 70),
        "a notice of at most 70 characters names it"
    );
    assert_eq!(
        write_machine(net.machine(a, P1).as_ref().unwrap()),
        write_machine(net.machine(b, P1).as_ref().unwrap()),
        "both devices converge on the winner"
    );
    // The same for M1.
    net.go_offline(a);
    net.go_offline(b);
    net.run(200);
    net.session(a)
        .rename_material(M1, "ZQX-MAT-7731 desktop")
        .unwrap();
    net.session(b)
        .rename_material(M1, "ZQX-MAT-7731 laptop")
        .unwrap();
    net.page(a);
    net.page(b);
    net.go_online(a);
    net.go_online(b);
    net.run(3_000);
    for index in [a, b] {
        assert_eq!(
            net.material_name(index, M1).as_deref(),
            Some("ZQX-MAT-7731 laptop")
        );
    }
    assert!(
        net.conflict_texts(a, LibraryKind::Material)
            .iter()
            .any(|text| text.contains("ZQX-MAT-7731 desktop (from your devices)")),
        "the material's loser is kept as a renamed copy"
    );
    // The copies stay local to the loser (ADR 0062 decision 5, plan P10): the losing device
    // holds one of each kind, the winning device none.
    let machine_copies: Vec<usize> = [a, b]
        .iter()
        .map(|i| net.conflict_texts(*i, LibraryKind::Machine).len())
        .collect();
    assert_eq!(
        machine_copies,
        vec![1, 0],
        "the machine copy stays on the loser"
    );
    let material_copies: Vec<usize> = [a, b]
        .iter()
        .map(|i| net.conflict_texts(*i, LibraryKind::Material).len())
        .collect();
    assert_eq!(
        material_copies,
        vec![1, 0],
        "the material copy stays on the loser"
    );
}

/// Criterion 10: switching a synced item off on A stops A publishing, keeps B's copy as a
/// local record with the notice, deletes nothing on B; switching it on again merges B's
/// local edits as an offline edit would (criterion 8's rule).
#[test]
fn c10_switching_off_keeps_the_copy_and_resumes_as_an_offline_edit() {
    let (mut net, a, b) = paired();
    let device = &mut net.devices[a];
    assert!(
        !device
            .session
            .library_switch(&mut device.glue, LibraryKind::Machine, P1, false)
    );
    net.run(1_000);
    assert!(net.machine(b, P1).is_some(), "nothing is deleted on B");
    assert!(
        net.devices[b]
            .writes
            .iter()
            .all(|write| write["id"] != P1 || write_kind(write) != "remove"),
        "no removal write on B"
    );
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "Sync off, this copy stays here.")
    );
    // B edits its local copy while the item is off.
    let _ = net
        .session(b)
        .rename_machine_profile(P1, "ZQX-MACH-7731 laptop edit", B_NOW_MS);
    net.page(b);
    // Turning it on again merges B's local edit as an offline edit would.
    let device = &mut net.devices[a];
    assert!(
        device
            .session
            .library_switch(&mut device.glue, LibraryKind::Machine, P1, true)
    );
    net.run(2_000);
    for index in [a, b] {
        assert_eq!(
            net.machine(index, P1).unwrap().values.name,
            "ZQX-MACH-7731 laptop edit",
            "B's local edit reached every device"
        );
    }
    // B edited its copy while the item was off: A's older text, which B never saw changed, is
    // kept there as the conflict copy, no edit lost. A made no edit since its text went out;
    // B's newer record supersedes it there, so A keeps no copy (UX review 2026-11-19: a plain
    // edit of another device is not a conflict).
    assert!(
        net.conflict_texts(b, LibraryKind::Machine)
            .iter()
            .any(|text| text.contains("ZQX-MACH-7731 (from your devices)")),
        "the older version is kept as the conflict copy, no edit lost"
    );
    assert!(
        net.conflict_texts(a, LibraryKind::Machine).is_empty(),
        "A edited nothing: no copy"
    );
}

/// Criterion 11: deleting a synced item on A deletes it on B with a notice; an off item's
/// deletion stays local.
#[test]
fn c11_delete_travels_to_every_device_and_stays_local_when_off() {
    let (mut net, a, b) = paired();
    let p2 = net
        .session(a)
        .duplicate_machine_profile(P1, [0x7d; 16], A_NOW_MS)
        .unwrap();
    net.page(a);
    assert!(net.within(30_000, |net| net.machine(b, &p2).is_some()));
    let device = &mut net.devices[a];
    let _ = device
        .session
        .library_switch(&mut device.glue, LibraryKind::Machine, &p2, false);
    net.run(500);
    assert!(net.session(a).delete_machine_profile(&p2));
    net.page(a);
    net.run(1_000);
    assert!(net.machine(a, &p2).is_none());
    assert!(
        net.machine(b, &p2).is_some(),
        "an off item's deletion stays local"
    );
    assert!(net.session(a).delete_machine_profile(P1));
    net.page(a);
    net.run(1_000);
    assert!(net.machine(b, P1).is_none(), "deleted on B");
    let removals = net.devices[b]
        .writes
        .iter()
        .filter(|write| write["id"] == P1 && write_kind(write) == "remove")
        .count();
    assert_eq!(removals, 1);
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice.contains("ZQX-MACH-7731") && notice.contains("deleted")),
        "B is told about the deletion"
    );
    assert!(net.entry(b, LibraryKind::Machine, P1).is_none());
    net.session(a).delete_material(M1).unwrap();
    net.page(a);
    net.run(1_000);
    assert!(
        net.material_name(b, M1).is_none(),
        "the material deletion travels too"
    );
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice.contains("ZQX-MAT-7731") && notice.contains("deleted"))
    );
}

/// Criterion 5's notice forms (UX notes section 5, P14): a single arriving item of one kind is
/// named in the notice, not counted.
#[test]
fn c5_a_single_arriving_item_is_named_in_the_notice() {
    // A machine only.
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        doc_session(11, clock_a),
    );
    net.run(500);
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    net.load_machines(a, &[(P1, "ZQX-MACH-7731")]);
    assert_eq!(net.first_sync(a), 1);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        doc_session(12, clock_b),
    );
    net.run(1_500);
    assert!(net.within(30_000, |net| net.machine(b, P1).is_some()));
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| *notice == "Machine profile ZQX-MACH-7731 arrived from your devices."),
        "the single machine is named"
    );
    // A material only.
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        doc_session(11, clock_a),
    );
    net.run(500);
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    net.load_material(a, M1, "ZQX-MAT-7731", true);
    assert_eq!(net.first_sync(a), 1);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        doc_session(12, clock_b),
    );
    net.run(1_500);
    assert!(net.within(30_000, |net| net.material_name(b, M1).is_some()));
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| *notice == "Material ZQX-MAT-7731 arrived from your devices."),
        "the single material is named"
    );
}

/// Criterion 12: M1 is appended at the end of B's material list. (The spec's order-travel
/// sentence is superseded by ADR 0062 decision 7: `material-order.json` never syncs and a new
/// device orders by name, so the reorder convergence is not testable as written.)
#[test]
fn c12_arriving_materials_are_appended_at_the_end_of_the_receiving_list() {
    let (mut net, a, b) = paired_with_birch();
    assert!(
        net.within(30_000, |net| net.material_name(b, M1).is_some()),
        "M1 arrives"
    );
    let listed: Vec<String> = net.devices[b]
        .session
        .materials()
        .listed()
        .into_iter()
        .map(|read| read.id().to_owned())
        .collect();
    assert_eq!(
        listed,
        vec![BIRCH.to_owned(), M1.to_owned()],
        "M1 is appended at the end of B's list"
    );
    assert_eq!(net.material_name(a, M1).as_deref(), Some("ZQX-MAT-7731"));
}

/// Criterion 13's Rust half: a browser device R of the group (as `0096` slice 1b modelled it)
/// receives the records without input, as the host writes the browser keeps under
/// `curvyo.machines/<id>` and `curvyo.materials/<id>` (the keys are the frontend test's).
/// Criterion 14: no device's machine pick leaves its computer.
#[test]
fn c13_c14_a_browser_device_receives_and_keeps_its_own_machine_pick() {
    let (mut net, a, b) = paired();
    net.glue(a).add_device(identity(3), "Browser");
    net.run(200);
    let mut r_session = Session::new(13);
    r_session.set_clock(clock_c);
    r_session.load_machine_choice(&write_choice(PICK));
    let r = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Browser", &seeds(3), 13, Millis(net.now)).unwrap(),
        r_session,
    );
    net.run(1_500);
    assert!(
        net.within(30_000, |net| {
            net.machine(r, P1).is_some() && net.material_name(r, M1).is_some()
        }),
        "criteria 5 to 12 hold for R unchanged"
    );
    assert!(
        !net.wrote_texts(r, P1).is_empty(),
        "R's records arrive as host writes"
    );
    // Criterion 14: the machine pick stays per computer and never leaves it.
    for index in [b, r] {
        assert!(
            net.devices[index]
                .writes
                .iter()
                .all(|write| write_kind(write) != "choice"),
            "no pick write left this computer"
        );
        assert!(
            net.entry(index, LibraryKind::Machine, PICK).is_none(),
            "the pick never enters the stream"
        );
    }
}

/// Criterion 21 [ADR]: B removed: the key rotation runs as `0096` built it, B keeps every
/// library item that reached it as a local, usable copy, no envelope written afterwards opens
/// with B's keys, and B's offline library edits are not merged.
#[test]
fn c21_removal_keeps_the_local_copies_and_cuts_b_off() {
    let (mut net, a, b) = paired();
    let before = net.max_epoch(a).unwrap_or(0);
    net.glue(a).remove_device(identity(2).member).unwrap();
    net.run(1_000);
    assert!(net.devices[b].glue.removed(), "B learns of its removal");
    assert!(!net.devices[b].glue.is_online());
    let after = net.max_epoch(a).unwrap();
    assert!(after > before, "the key rotation runs");
    assert!(
        net.max_epoch(b).unwrap_or(0) < after,
        "no envelope written afterwards opens with B's keys"
    );
    assert!(
        net.devices[b].glue.take_files_when(true).is_none(),
        "a removed device writes no group files"
    );
    assert!(net.machine(b, P1).is_some() && net.material_name(b, M1).is_some());
    let _ = net
        .session(b)
        .rename_machine_profile(P1, "ZQX-MACH-7731 after removal", B_NOW_MS);
    net.page(b);
    net.go_online(b);
    net.run(3_000);
    assert_eq!(
        net.machine(a, P1).unwrap().values.name,
        "ZQX-MACH-7731",
        "B's offline edit is not merged"
    );
    assert!(net.devices[b].glue.removed());
}

/// Criterion 22 [ADR]: B leaves: the same outcome for the libraries, the remaining devices
/// rotate without a prompt.
#[test]
fn c22_leave_rotates_the_rest_and_keeps_the_local_copies() {
    let (mut net, a, b) = paired();
    let before = net.max_epoch(a).unwrap_or(0);
    net.glue(b).leave().unwrap();
    net.run(2_000);
    assert!(net.devices[b].glue.removed());
    assert!(!net.glue(a).has_member(&identity(2).member));
    let after = net.max_epoch(a).unwrap();
    assert!(after > before, "the remaining devices rotate");
    assert!(
        net.machine(b, P1).is_some() && net.material_name(b, M1).is_some(),
        "B keeps every library item that reached it"
    );
    assert!(net.max_epoch(b).unwrap_or(0) < after);
}

/// Criterion 23 [ADR]: a third device C pairs through B and receives the current state of
/// every on-switched item without input; it receives no off-switched item and no history
/// beyond the current state. A switch-off or delete uploads the room's snapshot once its
/// record is acked, so the server's history no longer replays the item's text to a fresh
/// member.
#[test]
fn c23_a_third_device_pairs_through_b_and_gets_the_current_state_only() {
    let (mut net, a, b) = paired();
    let p2 = net
        .session(a)
        .duplicate_machine_profile(P1, [0x7c; 16], A_NOW_MS)
        .unwrap();
    net.page(a);
    assert!(net.within(30_000, |net| net.machine(b, &p2).is_some()));
    // Two names that cannot hide in the snapshot's compression: P1's older one and the off
    // item's.
    for (step, (id, name)) in [(P1, OLD_MARK), (p2.as_str(), OFF_MARK)]
        .into_iter()
        .enumerate()
    {
        let now = A_NOW_MS + i64::try_from(step).unwrap() + 1;
        let _ = net.session(a).rename_machine_profile(id, name, now);
        net.page(a);
        assert!(net.within(2_000, |net| {
            net.machine(b, id).is_some_and(|m| m.values.name == name)
        }));
    }
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 new", A_NOW_MS + 3);
    net.page(a);
    assert!(net.within(2_000, |net| {
        net.machine(b, P1)
            .is_some_and(|p1| p1.values.name == "ZQX-MACH-7731 new")
    }));
    let device = &mut net.devices[a];
    let _ = device
        .session
        .library_switch(&mut device.glue, LibraryKind::Machine, &p2, false);
    net.run(500);
    net.glue(b).add_device(identity(3), "Pad");
    net.run(200);
    let c = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Pad", &seeds(3), 13, Millis(net.now)).unwrap(),
        Session::new(13),
    );
    net.run(1_500);
    assert!(
        net.within(30_000, |net| {
            net.machine(c, P1).is_some() && net.material_name(c, M1).is_some()
        }),
        "C receives the current state without input"
    );
    assert_eq!(
        net.machine(c, P1).unwrap().values.name,
        "ZQX-MACH-7731 new",
        "the current state, not the older name"
    );
    assert!(
        net.machine(c, &p2).is_none(),
        "C does not receive the off-switched item"
    );
    assert_eq!(
        net.entry(c, LibraryKind::Machine, &p2).unwrap().text,
        None,
        "no text of the off item reaches C"
    );
    // The oplog of C holds no old text either: not the off item's, not P1's name before the
    // rename (architect review 2026-11-19: the room snapshot is state-only).
    let held = net.glue(c).library().snapshot().unwrap();
    assert!(
        !contains(&held, OFF_MARK.as_bytes()),
        "C's library oplog holds the off item's text"
    );
    assert!(
        !contains(&held, OLD_MARK.as_bytes()),
        "C's library oplog holds P1's older name"
    );
    let users = net.user_machines(c);
    assert_eq!(users.len(), 1, "no history beyond the current state");
    assert!(
        net.conflict_texts(c, LibraryKind::Machine).is_empty(),
        "C receives no stale copies"
    );
}

/// Criterion 27: after a switch-off the server holds no record of the item; the snapshot the
/// switch-off uploads (and a later leave's) replaces the history that carried its text.
#[test]
fn c27_after_a_switch_off_the_server_store_holds_no_text_of_the_item() {
    let (mut net, a, b) = paired();
    let p2 = net
        .session(a)
        .duplicate_machine_profile(P1, [0x7b; 16], A_NOW_MS)
        .unwrap();
    net.page(a);
    assert!(net.within(30_000, |net| net.machine(b, &p2).is_some()));
    let _ = net
        .session(a)
        .rename_machine_profile(&p2, OFF_MARK, A_NOW_MS + 1);
    net.page(a);
    assert!(net.within(2_000, |net| {
        net.machine(b, &p2)
            .is_some_and(|m| m.values.name == OFF_MARK)
    }));
    let device = &mut net.devices[a];
    let _ = device
        .session
        .library_switch(&mut device.glue, LibraryKind::Machine, &p2, false);
    net.run(500);
    for index in [a, b] {
        assert_eq!(
            net.entry(index, LibraryKind::Machine, &p2).unwrap().text,
            None,
            "the text is gone from the stream on every device"
        );
    }
    // A leave rotates the room and uploads the snapshot, so the server's store holds the
    // current state only.
    net.glue(b).leave().unwrap();
    net.run(2_000);
    net.glue(a).add_device(identity(4), "Pad");
    net.run(200);
    let d = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Pad", &seeds(4), 14, Millis(net.now)).unwrap(),
        Session::new(14),
    );
    net.run(1_500);
    assert!(
        net.within(30_000, |net| net.machine(d, P1).is_some()),
        "the on item is there (the control)"
    );
    assert_eq!(
        net.entry(d, LibraryKind::Machine, &p2).unwrap().text,
        None,
        "the server's store holds no record text of the off item"
    );
    assert!(
        net.machine(d, &p2).is_none(),
        "D receives no record of the off item"
    );
    let held = net.glue(d).library().snapshot().unwrap();
    assert!(
        !contains(&held, OFF_MARK.as_bytes()),
        "the room snapshot D started from carries no history of the off item"
    );
    assert!(
        net.machine(b, &p2).is_some(),
        "B keeps its local copy of the switched-off item"
    );
}

/// Criteria 24, 25 and 26: after a journey through criteria 5 to 23 (first sync with P1's
/// connection fields, M1's judged test and note, an edit, a same-field conflict, a deletion, a
/// switch-off, a browser device with its own machine pick, B's removal), the server's stored
/// data and every byte on the wire hold none of the plaintext markers (the family name, the
/// record names, the tag, the note text, the builtin's name, the pick), and no key material:
/// the device identity entropy and no epoch key.
#[test]
fn c24_c25_c26_the_server_store_holds_only_ciphertext_and_no_keys() {
    let (mut net, a, b) = paired();
    // A browser device with its own machine pick (criterion 14's marker).
    net.glue(a).add_device(identity(3), "Browser");
    net.run(200);
    let mut r_session = Session::new(13);
    r_session.set_clock(clock_c);
    r_session.load_machine_choice(&write_choice(PICK));
    net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Browser", &seeds(3), 13, Millis(net.now)).unwrap(),
        r_session,
    );
    net.run(1_500);
    // An edit, a same-field conflict, a deletion, a switch-off and a removal.
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 K40", A_NOW_MS);
    net.page(a);
    net.run(500);
    net.go_offline(a);
    net.go_offline(b);
    net.run(200);
    let _ = net
        .session(a)
        .rename_machine_profile(P1, "ZQX-MACH-7731 desktop", A_NOW_MS);
    let _ = net
        .session(b)
        .rename_machine_profile(P1, "ZQX-MACH-7731 laptop", B_NOW_MS);
    net.page(a);
    net.page(b);
    net.go_online(a);
    net.go_online(b);
    net.run(3_000);
    net.session(a).delete_material(M1).unwrap();
    net.page(a);
    net.run(500);
    let device = &mut net.devices[a];
    let _ = device
        .session
        .library_switch(&mut device.glue, LibraryKind::Machine, P1, false);
    net.run(500);
    net.glue(a).remove_device(identity(2).member).unwrap();
    net.run(1_000);

    assert!(
        net.stored_records > 0,
        "the journey stored records; the scan is not vacuous"
    );
    for bytes in [&net.stored, &net.wire] {
        for marker in [
            &b"ZQX-FONT-7731"[..],
            b"ZQX-MACH-7731",
            b"ZQX-MAT-7731",
            b"ZQX-TAG-7731",
            b"ZQX-NOTE-7731",
            b"Generic laser",
            PICK.as_bytes(),
        ] {
            assert!(
                !contains(bytes, marker),
                "the server's data holds the plaintext marker {}",
                String::from_utf8_lossy(marker)
            );
        }
        for n in 1_u8..=3 {
            let entropy = identity_entropy(n);
            for window in entropy.chunks(16) {
                assert!(
                    !contains(bytes, window),
                    "device {n}'s identity entropy is on the server"
                );
            }
        }
        for (index, device) in net.devices.iter().enumerate() {
            for (_, key) in &device.keys {
                assert!(
                    !contains(bytes, key),
                    "an epoch key of device {index} is on the server"
                );
            }
        }
    }
}

/// The pairing of `paired` with B holding one material of its own (criterion 12).
fn paired_with_birch() -> (Net, usize, usize) {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        doc_session(11, clock_a),
    );
    net.run(500);
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    net.load_machines(a, &[(P1, "ZQX-MACH-7731")]);
    net.load_material(a, M1, "ZQX-MAT-7731", true);
    assert_eq!(net.first_sync(a), 2);
    net.run(500);
    let mut b_session = doc_session(12, clock_b);
    b_session.load_materials(
        &[(
            record_file_name(BIRCH),
            plain_material_text(BIRCH, "Birch 3 mm", 5),
        )],
        None,
    );
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        b_session,
    );
    net.run(1_500);
    (net, a, b)
}
