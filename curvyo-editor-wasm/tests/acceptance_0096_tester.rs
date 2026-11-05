//! Tester acceptance tests for `specs/0096-own-devices-sync` through the public
//! `DeviceGroupGlue` and an in-memory relay, written from the specification and ADR 0040
//! decision 8 before the glue was read: a third device paired through the second (criterion 6),
//! the switch seen by every device (criterion 13), removal of an online and an offline device
//! (criteria 24 and 25), a removed device that cannot re-add itself or a puppet, and Leave with
//! the rotation that follows (criterion 27).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use curvyo_crypto_core::{Identity, IdentityKeys};
use curvyo_editor_wasm::{DeviceGroupGlue, SyncSeeds, TransportAction};
use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput};

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];
const W: [u8; 16] = [0x77; 16];

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

struct Device {
    glue: DeviceGroupGlue,
    conn: Option<ConnectionId>,
    offline: bool,
    keys: Vec<(u32, [u8; 32])>,
}

struct Net {
    relay: Relay,
    devices: Vec<Device>,
    now: u64,
    entropy: u8,
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            devices: Vec::new(),
            now: 1_800_000_000_000,
            entropy: 0,
        }
    }

    fn add(&mut self, glue: DeviceGroupGlue) -> usize {
        self.devices.push(Device {
            glue,
            conn: None,
            offline: false,
            keys: Vec::new(),
        });
        self.devices.len() - 1
    }

    fn glue(&mut self, i: usize) -> &mut DeviceGroupGlue {
        &mut self.devices[i].glue
    }

    fn step(&mut self) -> bool {
        let now = Millis(self.now);
        let mut moved = false;
        for i in 0..self.devices.len() {
            while let Some(action) = self.devices[i].glue.next_action() {
                moved = true;
                match action {
                    TransportAction::Connect(_) => {
                        if self.devices[i].offline {
                            self.devices[i].glue.disconnected(now);
                        } else {
                            let c = self.relay.connect();
                            self.devices[i].conn = Some(c);
                            self.devices[i].glue.connected(now);
                        }
                    }
                    TransportAction::Send(bytes) => {
                        let Some(c) = self.devices[i].conn else {
                            continue;
                        };
                        self.entropy = self.entropy.wrapping_add(1);
                        let out = self.relay.receive(c, &bytes, now, [self.entropy; 32]);
                        self.route(out);
                    }
                    TransportAction::Close => {
                        if let Some(c) = self.devices[i].conn.take() {
                            let out = self.relay.disconnect(c);
                            self.route(out);
                        }
                    }
                }
            }
            if let Some(keys) = self.devices[i].glue.take_key_update() {
                self.devices[i].keys = keys;
            }
        }
        moved
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        let now = Millis(self.now);
        for o in out {
            match o {
                RelayOutput::Send { connection, frame } => {
                    if let Some(d) = self.devices.iter_mut().find(|d| d.conn == Some(connection)) {
                        d.glue.received(&frame.encode(), now);
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(d) = self.devices.iter_mut().find(|d| d.conn == Some(connection)) {
                        d.conn = None;
                        d.glue.disconnected(now);
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
            for d in &mut self.devices {
                d.glue.tick(Millis(self.now));
            }
            let mut guard = 0;
            while self.step() {
                guard += 1;
                assert!(guard < 10_000, "does not settle");
            }
        }
    }

    fn go_offline(&mut self, i: usize) {
        self.devices[i].offline = true;
        if let Some(c) = self.devices[i].conn.take() {
            let out = self.relay.disconnect(c);
            self.route(out);
            self.devices[i].glue.disconnected(Millis(self.now));
        }
    }

    fn go_online(&mut self, i: usize) {
        self.devices[i].offline = false;
        let now = Millis(self.now);
        self.devices[i].glue.network_back(now);
    }

    fn max_epoch(&self, i: usize) -> Option<u32> {
        self.devices[i].keys.iter().map(|(n, _)| *n).max()
    }
}

/// A with B and C: A creates, adds B; B joins; B adds C; C joins.
fn three() -> Net {
    let mut net = Net::new();
    let now = Millis(net.now);
    let a = net.add(DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, now).unwrap());
    net.run(500);
    assert!(net.glue(a).is_online());
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    let now = Millis(net.now);
    let b = net.add(DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, now).unwrap());
    net.run(1_000);
    assert!(net.glue(b).is_online());
    net.glue(b).add_device(identity(3), "Workshop");
    net.run(200);
    let now = Millis(net.now);
    let c = net.add(DeviceGroupGlue::join(SERVER, ROOM, "Workshop", &seeds(3), 13, now).unwrap());
    net.run(1_500);
    assert_eq!((a, b, c), (0, 1, 2));
    net
}

/// Criterion 6: C paired through B is a member for A too, and every device lists every name.
#[test]
fn criterion_6_third_device_through_b_is_known_to_a() {
    let mut net = three();
    assert!(net.glue(2).is_online());
    for i in 0..3 {
        for n in [1, 2, 3] {
            assert!(net.glue(i).has_member(&identity(n).member), "{i} lacks {n}");
        }
    }
    assert_eq!(
        net.glue(0).list().label(&identity(3).member.0).as_deref(),
        Some("Workshop")
    );
    assert_eq!(
        net.glue(2).list().label(&identity(1).member.0).as_deref(),
        Some("Desktop")
    );
}

/// Criterion 13: switching off on any device is seen on every device.
#[test]
fn criterion_13_switch_seen_by_all() {
    let mut net = three();
    net.glue(0).switch_workspace(W, "ZQX-WS-7731", true);
    net.run(500);
    for i in 0..3 {
        assert!(net.glue(i).list().workspace(&W).unwrap().on, "{i}");
    }
    net.glue(2).switch_workspace(W, "ZQX-WS-7731", false);
    net.run(500);
    for i in 0..3 {
        assert!(!net.glue(i).list().workspace(&W).unwrap().on, "{i}");
    }
}

/// Criterion 24: B removed while online learns it, gets a notice, and gets no key of the epoch
/// written after; A and C go on.
#[test]
fn criterion_24_online_device_is_removed() {
    let mut net = three();
    let before = net.max_epoch(0).unwrap_or(0);
    net.glue(0).remove_device(identity(2).member).unwrap();
    net.run(1_000);
    assert!(net.glue(1).removed(), "B does not know it was removed");
    assert!(!net.glue(1).is_online());
    let after = net.max_epoch(0).unwrap();
    assert!(after > before, "no rotation on removal");
    assert_eq!(net.max_epoch(2), Some(after), "C did not get the new epoch");
    assert!(
        net.max_epoch(1).unwrap_or(0) < after,
        "B holds the epoch written after its removal"
    );
    assert!(!net.glue(0).has_member(&identity(2).member));
    assert!(!net.glue(2).has_member(&identity(2).member));
    let mut notices = Vec::new();
    while let Some(n) = net.glue(1).take_notice() {
        notices.push(n);
    }
    assert!(!notices.is_empty(), "B shows no notice");
    // A removed device writes no group files any more.
    assert!(net.glue(1).take_files_when(true).is_none());
}

/// Criterion 25: B offline during the removal learns it when it reconnects.
#[test]
fn criterion_25_offline_device_learns_its_removal() {
    let mut net = three();
    net.go_offline(1);
    net.run(200);
    net.glue(0).remove_device(identity(2).member).unwrap();
    net.run(500);
    assert!(!net.glue(1).removed());
    net.go_online(1);
    net.run(5_000);
    assert!(net.glue(1).removed(), "B did not learn its removal");
    assert!(!net.glue(1).is_online());
}

/// Criteria 24 and 25: a removed device that was offline adds a puppet identity before it
/// learns of its removal; the relay refuses it and nobody else ever sees the puppet.
#[test]
fn criterion_24_removed_device_cannot_add_a_puppet() {
    let mut net = three();
    net.go_offline(1);
    net.run(200);
    net.glue(0).remove_device(identity(2).member).unwrap();
    net.run(500);
    net.glue(1).add_device(identity(9), "Puppet");
    net.go_online(1);
    net.run(5_000);
    for i in [0, 2] {
        assert!(
            !net.glue(i).has_member(&identity(9).member),
            "{i} admitted the removed device's puppet"
        );
    }
}

/// Criterion 27: B leaves; A rotates the group without a prompt, so B holds no current key.
#[test]
fn criterion_27_leave_then_rotation() {
    let mut net = three();
    let before = net.max_epoch(0).unwrap_or(0);
    net.glue(1).leave().unwrap();
    net.run(2_000);
    assert!(net.glue(1).removed());
    assert!(!net.glue(0).has_member(&identity(2).member));
    let after = net.max_epoch(0).unwrap();
    assert!(after > before, "no rotation after Leave");
    assert_eq!(net.max_epoch(2), Some(after));
    assert!(net.max_epoch(1).unwrap_or(0) < after);
}

/// Criterion 22: a device renames itself and every device shows the new name.
#[test]
fn criterion_22_rename_reaches_all() {
    let mut net = three();
    net.glue(1).set_own_label("ZQX-PC-7731 laptop");
    net.run(500);
    for i in [0, 2] {
        assert_eq!(
            net.glue(i).list().label(&identity(2).member.0).as_deref(),
            Some("ZQX-PC-7731 laptop")
        );
    }
}
