//! Tests of the device group glue: two and three devices in one in-memory relay.
use curvyo_sync_core::{ConnectionId, Relay, RelayOutput, SessionEvent};

use super::*;
use crate::session::hex;

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];

fn seeds(n: u8) -> SyncSeeds {
    SyncSeeds {
        identity: [n; 64],
        nonce: [n.wrapping_add(100); 32],
        entropy: [n.wrapping_add(200); 32],
    }
}

fn identity(n: u8) -> Identity {
    IdentityKeys::from_entropy(&[n; 64]).identity()
}

/// Devices in one in-memory relay, each with at most one connection.
struct Net {
    relay: Relay,
    devices: Vec<(DeviceGroupGlue, Option<ConnectionId>)>,
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

    fn add(&mut self, glue: DeviceGroupGlue) -> usize {
        self.devices.push((glue, None));
        self.devices.len() - 1
    }

    fn step(&mut self) -> bool {
        let now = Millis(self.now);
        let mut moved = false;
        for index in 0..self.devices.len() {
            while let Some(action) = self.devices[index].0.next_action() {
                moved = true;
                match action {
                    TransportAction::Connect(_) => {
                        let connection = self.relay.connect();
                        self.devices[index].1 = Some(connection);
                        self.devices[index].0.connected(now);
                    }
                    TransportAction::Send(bytes) => {
                        let Some(connection) = self.devices[index].1 else {
                            continue;
                        };
                        let out = self.relay.receive(connection, &bytes, now, [9; 32]);
                        self.route(out);
                    }
                    TransportAction::Close => {
                        if let Some(connection) = self.devices[index].1.take() {
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
                    if let Some(device) = self.devices.iter_mut().find(|d| d.1 == Some(connection))
                    {
                        device.0.received(&frame.encode(), now);
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(device) = self.devices.iter_mut().find(|d| d.1 == Some(connection))
                    {
                        device.1 = None;
                        device.0.disconnected(now);
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
                device.0.tick(Millis(self.now));
            }
            while self.step() {}
        }
    }
}

fn view(glue: &DeviceGroupGlue) -> serde_json::Value {
    serde_json::from_str(&glue.view_json(0, &[])).unwrap()
}

/// A creates the group, adds B; B joins with its own keys and both list each other by name;
/// a switch on A reaches B; the files and keys are handed out.
#[test]
fn two_devices_share_the_group_and_its_list() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    assert!(net.devices[a].0.is_online());
    net.devices[a].0.add_device(identity(2), "Laptop");
    net.run(200);
    assert!(net.devices[a].0.has_member(&identity(2).member));
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
    );
    net.run(1_000);
    assert!(net.devices[b].0.is_online());
    let rows = view(&net.devices[b].0)["rows"].clone();
    assert_eq!(rows[0]["label"], "Laptop");
    assert_eq!(rows[0]["own"], true);
    assert_eq!(rows[1]["label"], "Desktop");
    assert_eq!(view(&net.devices[a].0)["count"], "2 devices");
    assert_eq!(rows[1]["seen"], "Online");

    net.devices[a]
        .0
        .switch_workspace([0x77; 16], "Box job", true);
    net.run(500);
    assert_eq!(net.devices[b].0.list().switched_on(), vec![[0x77; 16]]);
    assert!(net.devices[b].0.take_files().is_some());
    assert!(net.devices[b].0.take_key_update().is_some());
}

/// A device added under one name that joins under another (a restore with the backed-up name)
/// and is a browser: its own name and its browser mark win over the adder's entry on every
/// device, because it writes them only after reading that entry (ADR 0040 decision 1).
#[test]
fn the_joiners_name_and_browser_mark_reach_the_adder() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    net.devices[a].0.add_device(identity(2), "Chrome on Linux");
    net.run(200);
    let mut joined = DeviceGroupGlue::join(
        SERVER,
        ROOM,
        "Rita's browser",
        &seeds(2),
        12,
        Millis(net.now),
    )
    .unwrap();
    joined.set_browser();
    let b = net.add(joined);
    net.run(1_500);
    let rows = view(&net.devices[a].0)["rows"].clone();
    assert_eq!(rows[1]["label"], "Rita's browser");
    assert_eq!(rows[1]["browser"], true);
    assert!(rows[1]["seen"].as_str().unwrap().starts_with("Browser, "));
    let own = view(&net.devices[b].0)["rows"][0].clone();
    assert_eq!(own["label"], "Rita's browser");
}

/// Criterion 23: A removes B; B learns it and the notice on A names it. Criterion 27: C
/// leaves; A renews the group key so C no longer holds the current one.
#[test]
fn remove_and_leave_end_the_device_and_renew_the_key() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    net.devices[a].0.add_device(identity(2), "Laptop");
    net.devices[a].0.add_device(identity(3), "Pad");
    net.run(200);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
    );
    let c = net
        .add(DeviceGroupGlue::join(SERVER, ROOM, "Pad", &seeds(3), 13, Millis(net.now)).unwrap());
    net.run(1_000);
    while net.devices[a].0.take_notice().is_some() {}

    net.devices[a].0.remove_device(identity(2).member).unwrap();
    net.run(500);
    assert!(net.devices[b].0.removed());
    assert_eq!(
        net.devices[a].0.take_notice().as_deref(),
        Some("Laptop was removed.")
    );

    net.devices[c].0.leave().unwrap();
    net.run(500);
    let keyring = net.devices[a].0.keyring().unwrap();
    assert!(keyring.was_removed(&identity(3).member));
    assert!(!keyring.holds(&identity(3).member, keyring.current_epoch()));
    assert_eq!(
        net.devices[a].0.take_notice().as_deref(),
        Some("Pad left your devices.")
    );
}

/// A joiner that connects before the opener's `AddMember` reached the relay retries.
#[test]
fn a_join_before_the_admission_retries() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
    );
    net.run(300);
    assert!(!net.devices[b].0.is_online());
    net.devices[a].0.add_device(identity(2), "Laptop");
    net.run(3_000);
    assert!(net.devices[b].0.is_online(), "B retried and got in");
}

/// Criteria 22, 26 and 27: the rows' detail lines come from the workspaces' facts; a removed
/// device still in a workspace keeps a row with the removal's count and an alert; a key a
/// departed device holds shows as a pending renewal.
#[test]
fn rows_show_each_workspace_removals_and_renewals() {
    use crate::session::{DeviceFacts, OwnState};
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    net.devices[a].0.add_device(identity(2), "Laptop");
    net.devices[a].0.add_device(identity(3), "Pad");
    net.run(300);
    let b = hex(&identity(2).member.0);
    let c = hex(&identity(3).member.0);
    let me = hex(&identity(1).member.0);
    let facts = vec![
        DeviceFacts {
            id: "77".repeat(16),
            name: "Box job".to_owned(),
            state: OwnState::Syncing { done: 3, total: 14 },
            members: vec![me.clone(), b.clone()],
            present: vec![b.clone()],
            pending_adds: vec![c.clone()],
            ..DeviceFacts::default()
        },
        DeviceFacts {
            id: "78".repeat(16),
            name: "Poster".to_owned(),
            state: OwnState::InSync,
            members: vec![me.clone(), b.clone(), c.clone()],
            rotate_pending: true,
            ..DeviceFacts::default()
        },
    ];
    let shown: serde_json::Value =
        serde_json::from_str(&net.devices[a].0.view_json(0, &facts)).unwrap();
    let rows = shown["rows"].as_array().unwrap();
    assert_eq!(rows[0]["seen"], "Syncing 3 of 14 documents");
    assert_eq!(rows[0]["details"][1]["state"], "In sync");
    let laptop = rows.iter().find(|r| r["member"] == b.as_str()).unwrap();
    assert_eq!(laptop["details"][0]["state"], "In sync");
    let pad = rows.iter().find(|r| r["member"] == c.as_str()).unwrap();
    assert_eq!(pad["details"][0]["state"], "Waiting for first copy");
    assert_eq!(shown["renewal"], "Key renewal pending for 1 workspace");

    net.devices[a].0.remove_device(identity(3).member).unwrap();
    net.run(300);
    let shown: serde_json::Value =
        serde_json::from_str(&net.devices[a].0.view_json(0, &facts)).unwrap();
    let pad = shown["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["member"] == c.as_str())
        .unwrap()
        .clone();
    assert_eq!(pad["removing"], true);
    assert_eq!(pad["seen"], "Removal incomplete, 1 of 2 workspaces");
    assert_eq!(
        shown["alerts"][0],
        "Pad is still in Poster. Ask an admin of Poster to remove it."
    );
}

/// ADR 0040 decision 11: a list update of a newer build is refused, the list stays as it was,
/// the notice shows once, and later updates of this build still arrive.
#[test]
fn a_newer_list_update_is_refused_and_reported_once() {
    let mut glue =
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(1)).unwrap();
    let newer = loro::LoroDoc::new();
    newer.set_peer_id(9).unwrap();
    newer.import(&glue.list().snapshot().unwrap()).unwrap();
    let known = newer.oplog_vv();
    newer.get_map("group").insert("format_version", 2).unwrap();
    newer.commit();
    let update = newer.export(loro::ExportMode::updates(&known)).unwrap();
    let before = glue.list().snapshot().unwrap();
    for seq in [1, 2] {
        glue.on_event(SessionEvent::Remote {
            seq,
            kind: RecordKind::Update,
            bytes: update.clone(),
        });
    }
    assert_eq!(glue.list().snapshot().unwrap(), before);
    assert_eq!(
        glue.take_notice().as_deref(),
        Some("Another device uses a newer Curvyo. Update this one to see changes to your devices.")
    );
    assert_eq!(glue.take_notice(), None);

    let other = DevicesList::from_snapshot(12, &before).unwrap();
    let start = other.version();
    other.switch_workspace([0x77; 16], "Box job", true).unwrap();
    glue.on_event(SessionEvent::Remote {
        seq: 3,
        kind: RecordKind::Update,
        bytes: other.export_updates(&start).unwrap(),
    });
    assert_eq!(glue.list().switched_on(), vec![[0x77; 16]]);
}
