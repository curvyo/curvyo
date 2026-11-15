//! Session-level tests of the library sync (`specs/0116-library-sync` criteria 3 to 15 and 21
//! to 27): two devices' groups and sessions in one in-memory relay, the page's pumps included.

use curvyo_document_core::{LibraryKind, LibraryList};
use curvyo_laser_core::library::{Material, write_record as write_material};
use curvyo_laser_core::settings::Thickness;
use curvyo_laser_core::{MachineProfile, record_file_name, write_record as write_machine};
use curvyo_sync_core::{
    ConnectionId, Millis, RecordKind, Relay, RelayOutput, SessionEvent, encode_group_snapshot,
    encode_library_record, split_snapshot,
};

use super::DeviceGroupGlue;
use crate::session::{Session, SyncSeeds, TransportAction};

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];
const P1: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const M1: &str = "5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875";

fn seeds(n: u8) -> SyncSeeds {
    SyncSeeds {
        identity: [n; 64],
        nonce: [n.wrapping_add(100); 32],
        entropy: [n.wrapping_add(200); 32],
    }
}

fn identity(n: u8) -> curvyo_crypto_core::Identity {
    curvyo_crypto_core::IdentityKeys::from_entropy(&[n; 64]).identity()
}

fn machine_text(id: &str, name: &str, modified: i64) -> String {
    let mut profile = MachineProfile::generic_laser();
    profile.id = id.to_owned();
    profile.values.name = name.to_owned();
    profile.modified = modified;
    write_machine(&profile)
}

fn material_text(id: &str, name: &str, modified: u64) -> String {
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

/// One device: its group glue, its session, and what the page collected of it.
struct Device {
    glue: DeviceGroupGlue,
    session: Session,
    connection: Option<ConnectionId>,
    writes: Vec<serde_json::Value>,
    notices: Vec<String>,
}

impl Device {
    /// The page's slow poll: the host write lists (which tap the local edits), then the library
    /// pump (which feeds the edits to the group's list and applies the arrivals).
    fn page(&mut self) {
        for list in [
            self.session.take_machine_writes_json(),
            self.session.take_material_writes_json(),
        ] {
            let taken: Vec<serde_json::Value> = serde_json::from_str(&list).unwrap_or_default();
            self.writes.extend(taken);
        }
        let notice = self.session.library_sync_pump(&mut self.glue);
        if !notice.is_empty() {
            self.notices.push(notice);
        }
    }

    fn wrote(&self, id: &str) -> Vec<&serde_json::Value> {
        self.writes
            .iter()
            .filter(|write| write["id"] == id)
            .collect()
    }
}

/// The devices in one in-memory relay, each with at most one connection.
struct Net {
    relay: Relay,
    devices: Vec<Device>,
    now: u64,
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            devices: Vec::new(),
            now: 1_800_000_000_000,
        }
    }

    fn add(&mut self, glue: DeviceGroupGlue, session: Session) -> usize {
        self.devices.push(Device {
            glue,
            session,
            connection: None,
            writes: Vec::new(),
            notices: Vec::new(),
        });
        self.devices.len() - 1
    }

    fn step(&mut self) -> bool {
        let now = Millis(self.now);
        let mut moved = false;
        for index in 0..self.devices.len() {
            while let Some(action) = self.devices[index].glue.next_action() {
                moved = true;
                match action {
                    TransportAction::Connect(_) => {
                        let connection = self.relay.connect();
                        self.devices[index].connection = Some(connection);
                        self.devices[index].glue.connected(now);
                    }
                    TransportAction::Send(bytes) => {
                        let Some(connection) = self.devices[index].connection else {
                            continue;
                        };
                        let out = self.relay.receive(connection, &bytes, now, [9; 32]);
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
        }
        moved
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        let now = Millis(self.now);
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    if let Some(device) = self
                        .devices
                        .iter_mut()
                        .find(|d| d.connection == Some(connection))
                    {
                        device.glue.received(&frame.encode(), now);
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
                RelayOutput::Store(_) => {}
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
            for device in &mut self.devices {
                device.page();
            }
            while self.step() {}
        }
    }
}

/// A group of two devices with P1 and M1 synced from A to B (criteria 3, 5): B receives both
/// without input, as host writes with the exact texts, and the notice names both.
#[test]
fn the_first_sync_arrives_on_the_other_device() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    assert!(net.devices[a].glue.is_online());
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    net.devices[a].session.load_materials(
        &[(record_file_name(M1), material_text(M1, "ZQX-MAT-7731", 5))],
        None,
    );
    let device = &mut net.devices[a];
    let written = device.session.library_first_sync(&mut device.glue);
    assert_eq!(written, 2);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);
    assert!(net.devices[b].glue.is_online());

    let machine = net.devices[b]
        .session
        .machine_profiles()
        .find(P1)
        .cloned()
        .unwrap();
    assert_eq!(machine.values.name, "ZQX-MACH-7731");
    assert_eq!(
        net.devices[b].session.materials().find(M1).unwrap().name,
        "ZQX-MAT-7731"
    );
    let machine_write = &net.devices[b].wrote(P1)[0];
    assert_eq!(machine_write["kind"], "write");
    assert_eq!(
        machine_write["text"].as_str().unwrap(),
        machine_text(P1, "ZQX-MACH-7731", 5)
    );
    assert_eq!(net.devices[b].wrote(M1).len(), 1);
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "1 machine profile and 1 material arrived from your devices.")
    );
    let toggles: serde_json::Value =
        serde_json::from_str(&net.devices[b].glue.library_view_json()).unwrap();
    assert_eq!(toggles["machines"][0]["sync"], true);
    assert_eq!(toggles["materials"][0]["sync"], true);
}

/// Criteria 8 and 9: two offline edits of the same record converge to the newer `modified` on
/// both devices, and the loser's version stays as the conflict copy on the device that lost.
#[test]
fn offline_edits_converge_with_a_conflict_copy() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);
    assert!(net.devices[b].session.machine_profiles().find(P1).is_some());

    let _ = net.devices[a]
        .session
        .rename_machine_profile(P1, "Desktop edit", 5);
    let _ = net.devices[b]
        .session
        .rename_machine_profile(P1, "Laptop edit", 6);
    for device in &mut net.devices {
        device.page();
    }
    net.run(1_500);

    for index in [a, b] {
        assert_eq!(
            net.devices[index]
                .session
                .machine_profiles()
                .find(P1)
                .unwrap()
                .values
                .name,
            "Laptop edit",
            "the newer modified wins on both devices"
        );
    }
    let conflicts: Vec<&serde_json::Value> = net.devices[a]
        .writes
        .iter()
        .filter(|write| {
            write["id"]
                .as_str()
                .is_some_and(|id| id.ends_with(".conflict"))
        })
        .collect();
    assert_eq!(conflicts.len(), 1, "one conflict copy on the loser");
    assert!(
        conflicts[0]["text"]
            .as_str()
            .unwrap()
            .contains("Desktop edit (from your devices)"),
        "the conflict copy is renamed (criterion 9)"
    );
    assert!(
        net.devices[a]
            .notices
            .iter()
            .any(|notice| notice == "Both copies are kept: Desktop edit (from your devices).")
    );
    assert_eq!(net.devices[a].wrote(P1).len(), 2);
}

/// Criterion 10: switching a synced item off on A leaves B's local copy as it is, with the
/// marker entry on every device.
#[test]
fn switching_off_keeps_the_local_copy() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);

    let device = &mut net.devices[a];
    assert!(
        !device
            .session
            .library_switch(&mut device.glue, LibraryKind::Machine, P1, false)
    );
    net.run(1_000);

    let entry = net.devices[b]
        .glue
        .library()
        .entry(LibraryKind::Machine, P1)
        .unwrap();
    assert!(!entry.sync);
    assert_eq!(entry.text, None, "the marker entry carries no text");
    assert!(net.devices[b].session.machine_profiles().find(P1).is_some());
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "Sync off, this copy stays here.")
    );
    assert!(
        net.devices[b]
            .writes
            .iter()
            .all(|write| write["id"] != P1 || write["kind"] == "write")
    );
}

/// Criterion 11: deleting a synced item on A removes it on B through sync, with a host write
/// and a notice; the local delete on A reaches the stream through the write list tap.
#[test]
fn deleting_on_one_device_removes_it_on_the_other() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);

    assert!(net.devices[a].session.delete_machine_profile(P1));
    for device in &mut net.devices {
        device.page();
    }
    net.run(1_000);

    assert!(
        net.devices[b]
            .glue
            .library()
            .entry(LibraryKind::Machine, P1)
            .is_none()
    );
    assert!(net.devices[b].session.machine_profiles().find(P1).is_none());
    let removals: Vec<&serde_json::Value> = net.devices[b]
        .writes
        .iter()
        .filter(|write| write["id"] == P1 && write["kind"] == "remove")
        .collect();
    assert_eq!(removals.len(), 1);
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "ZQX-MACH-7731 was deleted on another device.")
    );
}

/// The envelope gate through the stream: a damaged or newer-format incoming text is skipped
/// with a notice and the local record is never rewritten (P6, ADR 0004 §9).
#[test]
fn damaged_and_newer_texts_are_skipped_with_a_notice() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);
    assert!(net.devices[b].session.machine_profiles().find(P1).is_some());

    let writes_before = net.devices[b].writes.len();
    let snapshot = net.devices[a].glue.library().snapshot().unwrap();
    let other = LibraryList::from_snapshot(99, &snapshot).unwrap();
    for text in [
        "{ not json".to_owned(),
        machine_text(P1, "ZQX-MACH-7731", 9).replacen(
            "\"format_version\": 4",
            "\"format_version\": 5",
            1,
        ),
    ] {
        let before = other.version();
        other.set(LibraryKind::Machine, P1, &text).unwrap();
        let update = other.export_updates(&before).unwrap();
        net.devices[b].glue.on_event(SessionEvent::Remote {
            seq: 1,
            kind: RecordKind::Update,
            bytes: encode_library_record(99, &update),
        });
    }
    for _ in 0..2 {
        net.devices[b].page();
    }
    assert_eq!(
        net.devices[b]
            .session
            .machine_profiles()
            .find(P1)
            .unwrap()
            .values
            .name,
        "ZQX-MACH-7731",
        "the local record stays as it is"
    );
    assert_eq!(net.devices[b].writes.len(), writes_before);
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "A library record could not be read. It was left as it is.")
    );
    assert!(
        net.devices[b]
            .notices
            .iter()
            .any(|notice| notice == "A library record needs a newer Curvyo. It was left as it is.")
    );
}

/// The framed two-stream room snapshot (ADR 0062 decision 1): a device that reads it in parts
/// takes both lists, and a group whose snapshot is asked for one uploads it in parts.
#[test]
fn the_framed_snapshot_carries_both_lists() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(1_500);

    let devices = net.devices[a].glue.list().snapshot().unwrap();
    let library = net.devices[a].glue.library().snapshot().unwrap();
    let stream = encode_group_snapshot(&devices, &encode_library_record(11, &library));
    let mut late = DeviceGroupGlue::join(SERVER, ROOM, "Pad", &seeds(3), 13, Millis(1)).unwrap();
    for (index, part) in split_snapshot(&stream, [0x77; 16]).into_iter().enumerate() {
        late.on_event(SessionEvent::Remote {
            seq: 10 + index as u64,
            kind: RecordKind::Snapshot,
            bytes: part,
        });
    }
    let entry = late.library().entry(LibraryKind::Machine, P1).unwrap();
    assert!(entry.sync);
    assert_eq!(
        entry.text.as_deref(),
        Some(machine_text(P1, "ZQX-MACH-7731", 5).as_str())
    );

    net.devices[a]
        .glue
        .on_event(SessionEvent::SnapshotDue { covers_seq: 0 });
    net.run(1_000);
    assert_eq!(
        net.devices[b]
            .glue
            .library()
            .entry(LibraryKind::Machine, P1)
            .unwrap()
            .text
            .as_deref(),
        Some(machine_text(P1, "ZQX-MACH-7731", 5).as_str()),
        "the uploaded parts changed nothing for an up-to-date device"
    );
}

/// `library.loro` (criteria 2, 21, 22): the glue hands it out with the group's files, a resume
/// from it holds the entries, and a pre-0116 group without one starts fresh.
#[test]
fn library_loro_round_trips_through_the_group_files() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a]
        .session
        .load_machine_profiles(&[(record_file_name(P1), machine_text(P1, "ZQX-MACH-7731", 5))]);
    let device = &mut net.devices[a];
    let _ = device.session.library_first_sync(&mut device.glue);
    net.run(500);
    let files = net.devices[a].glue.take_files().unwrap();

    let keyring_log =
        curvyo_crypto_core::keyring::encode_log(&net.devices[a].glue.client.keyring_entries())
            .unwrap();
    let resumed = DeviceGroupGlue::resume(
        SERVER,
        ROOM,
        &keyring_log,
        &files.list,
        &files.library,
        files.server_seq,
        Vec::new(),
        &seeds(9),
        19,
        Millis(1),
    )
    .unwrap();
    assert_eq!(
        resumed
            .library()
            .entry(LibraryKind::Machine, P1)
            .unwrap()
            .text
            .as_deref(),
        Some(machine_text(P1, "ZQX-MACH-7731", 5).as_str())
    );

    let fresh = DeviceGroupGlue::resume(
        SERVER,
        ROOM,
        &keyring_log,
        &files.list,
        &[],
        files.server_seq,
        Vec::new(),
        &seeds(9),
        19,
        Millis(1),
    )
    .unwrap();
    assert_eq!(fresh.library().entries(LibraryKind::Machine).len(), 0);
}
