//! Thread-CPU budgets of the synced font library (`specs/0116-library-sync` criterion 20, ADR
//! 0062 consequences): the receiving device spends at most 2 s of thread CPU time on a first sync
//! of 20 fonts totalling 40 MiB, and at most 2 s on the library the ADR names (200 machines, 200
//! materials, 100 font entries of which two carry real font bytes). CPU time, never wall time
//! (`CLAUDE.md` §5); asserted at the stated figure in an optimised build (measured 0.15 s and
//! 0.12 s), run as smoke tests in a debug one, like `sync_cpu_budget.rs`. The first sync must also finish within
//! 60 s of simulated LAN time. Run:
//! `cargo test --release -p curvyo-editor-wasm --test library_fonts_cpu_budget -- --nocapture`.

#![cfg(unix)]
#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::collections::BTreeSet;
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_crypto_core::IdentityKeys;
use curvyo_document_core::{AssetHash, CuttableChoice};
use curvyo_editor_wasm::{
    DeviceGroupGlue, FontChange, FontInput, Session, SyncSeeds, TransportAction,
};
use curvyo_laser_core::library::{Material, write_record as write_material};
use curvyo_laser_core::settings::Thickness;
use curvyo_laser_core::{MachineProfile, record_file_name, write_record as write_machine};
use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput};

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];
const MIB: usize = 1024 * 1024;
const BUDGET: Duration = Duration::from_secs(2);

fn seeds(n: u8) -> SyncSeeds {
    SyncSeeds {
        identity: [n; 64],
        nonce: [n.wrapping_add(100); 32],
        entropy: [n.wrapping_add(200); 32],
    }
}

/// One device: glue, session, what its host holds, and the thread CPU time its calls cost.
struct Device {
    glue: DeviceGroupGlue,
    session: Session,
    connection: Option<ConnectionId>,
    host: BTreeSet<AssetHash>,
    added: usize,
    cpu: Duration,
}

impl Device {
    fn new(glue: DeviceGroupGlue, peer: u64) -> Self {
        Self {
            glue,
            session: Session::new(peer),
            connection: None,
            host: BTreeSet::new(),
            added: 0,
            cpu: Duration::ZERO,
        }
    }

    /// The page's poll: font changes (the host writes of the page are left out), then the
    /// library pump.
    fn page(&mut self) {
        let (added, cpu) = cpu_time_of(|| {
            self.glue.font_host_hashes(self.host.clone());
            let mut added = 0;
            for arrival in self.glue.take_font_arrivals() {
                if matches!(arrival.change, FontChange::Add(_)) {
                    self.host.insert(arrival.hash);
                    added += 1;
                }
            }
            let _ = self.session.library_sync_pump(&mut self.glue);
            added
        });
        self.added += added;
        self.cpu += cpu;
    }
}

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
            if let RelayOutput::Send { connection, frame } = output
                && let Some(device) = self
                    .devices
                    .iter_mut()
                    .find(|d| d.connection == Some(connection))
            {
                let bytes = frame.encode();
                let ((), cpu) = cpu_time_of(|| device.glue.received(&bytes, now));
                device.cpu += cpu;
            }
        }
    }

    fn run(&mut self, ms: u64) {
        let end = self.now + ms;
        while self.now < end {
            self.now += 50;
            for device in &mut self.devices {
                let now = Millis(self.now);
                let ((), cpu) = cpu_time_of(|| device.glue.tick(now));
                device.cpu += cpu;
                device.page();
            }
            while self.step() {}
        }
    }

    /// Runs until `done` holds or `limit_ms` of simulated time passed; the simulated time taken.
    fn until(&mut self, limit_ms: u64, done: impl Fn(&Net) -> bool) -> u64 {
        let start = self.now;
        while self.now - start < limit_ms && !done(self) {
            self.run(100);
        }
        self.now - start
    }
}

/// A desktop A in a group with a laptop identity; B joins when the test says.
fn group() -> (Net, usize) {
    let mut net = Net::new();
    let glue =
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap();
    net.devices.push(Device::new(glue, 11));
    net.run(500);
    let identity = IdentityKeys::from_entropy(&[2; 64]).identity();
    net.devices[0].glue.add_device(identity, "Laptop");
    net.run(200);
    (net, 0)
}

fn join_b(net: &mut Net) -> usize {
    let glue =
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap();
    net.devices.push(Device::new(glue, 12));
    net.devices.len() - 1
}

fn font_input(name: &str) -> FontInput {
    FontInput {
        file_name: name.to_owned(),
        folder: None,
        tags: Vec::new(),
        favourite: false,
        cuttable: CuttableChoice::Auto,
        documents: Vec::new(),
    }
}

/// Incompressible-looking deterministic bytes: a font stand-in of `len` bytes.
fn font_bytes(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

fn assert_within(name: &str, took: Duration, budget: Duration) {
    eprintln!(
        "{name}: {took:?} (budget {budget:?}, asserted at the stated figure in an optimised build)"
    );
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{name}: {took:?} over {budget:?}");
    }
}

/// Criterion 20: 20 fonts of 2 MiB (40 MiB) first-sync from A to B on a LAN server within 60 s of
/// simulated time, and cost the receiving device at most 2 s of thread CPU time (the file writes
/// of the host layer are not wasm work and are not counted).
#[test]
fn a_first_sync_of_20_fonts_and_40_mib_costs_the_receiver_at_most_2_s() {
    let (mut net, a) = group();
    for n in 0..20 {
        net.devices[a]
            .glue
            .font_set(font_input(&format!("Font{n}.ttf")), font_bytes(2 * MIB, n))
            .unwrap();
    }
    net.run(500);
    let b = join_b(&mut net);
    let took = net.until(60_000, |net| net.devices[b].added == 20);
    assert_eq!(net.devices[b].added, 20, "all 20 fonts arrived");
    assert!(took <= 60_000, "simulated time {took} ms");
    assert_within("receive 20 fonts, 40 MiB", net.devices[b].cpu, BUDGET);
}

/// ADR 0062 consequences: the receiver applies 200 machines, 200 materials and 100 font entries
/// (two with 2 MiB of bytes, the rest 1 KiB) within the same 2 s of thread CPU time.
#[test]
fn applying_200_machines_200_materials_and_100_font_entries_costs_at_most_2_s() {
    let (mut net, a) = group();
    let mut machines = Vec::new();
    let mut materials = Vec::new();
    for n in 0..200_u32 {
        let id = format!("6f1c2a4e-9b3d-4c1a-8e2f-{n:012x}");
        let mut profile = MachineProfile::generic_laser();
        profile.id.clone_from(&id);
        profile.values.name = format!("Machine {n}");
        profile.modified = 5;
        machines.push((record_file_name(&id), write_machine(&profile)));
        let id = format!("5d4e3f2a-1b0c-4a9d-8e7f-{n:012x}");
        let text = write_material(&Material {
            id: id.clone(),
            modified: 5,
            name: format!("Material {n}"),
            thickness: Thickness::from_mm(3.0).unwrap(),
            lasers: Vec::new(),
            tests: Vec::new(),
            history: Vec::new(),
        });
        materials.push((record_file_name(&id), text));
    }
    net.devices[a].session.load_machine_profiles(&machines);
    net.devices[a].session.load_materials(&materials, None);
    let device = &mut net.devices[a];
    assert_eq!(device.session.library_first_sync(&mut device.glue), 400);
    for n in 0..100_u32 {
        let size = if n < 2 { 2 * MIB } else { 1024 };
        net.devices[a]
            .glue
            .font_set(
                font_input(&format!("Entry{n}.ttf")),
                font_bytes(size, n + 100),
            )
            .unwrap();
    }
    net.run(500);
    let b = join_b(&mut net);
    let took = net.until(60_000, |net| {
        net.devices[b].added == 100
            && net.devices[b].session.machine_profiles().listed().len() > 200
    });
    assert_eq!(
        net.devices[b].added, 100,
        "all 100 fonts arrived after {took} ms"
    );
    assert!(
        net.devices[b]
            .session
            .machine_profiles()
            .find(&machines[199].0[..36])
            .is_some()
    );
    assert_within("receive 200 + 200 + 100", net.devices[b].cpu, BUDGET);
}
