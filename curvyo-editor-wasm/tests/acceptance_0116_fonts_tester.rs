//! Tester acceptance tests for `specs/0116-library-sync` slice 2 (fonts: criteria 16 to 20, and
//! criteria 4, 10, 11, 21, 23, 24, 25, 27 as they touch fonts), written from the specification,
//! the UX notes and ADR 0062 before the glue was read. Two or three desktop devices, each with
//! its `DeviceGroupGlue`, around one in-memory relay, driven only through the public glue
//! surface. The page is modelled by the test: each device holds the font files it has written
//! to its fonts folder in `host` and supplies them when the glue asks (`font_wants`). Every byte
//! the server sees (its store writes and the wire) is captured for the scans of criteria 24 and
//! 25.
//!
//! Not covered here, reported as seams: both-order arrival of an entry and its blob (the relay
//! order is not controllable through the public surface), the browser's `IndexedDB` writes
//! (criterion 16 browser half, see `frontend/tests/acceptance_0116_fonts_tester.test.ts`), the
//! server-side removal of a switched-off blob (criterion 27: the store's ops are ciphertext and
//! their pruning is not observable), the notices of the glue (the page's own text).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names
)]

#[path = "cpu_time/mod.rs"]
mod cpu_time;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_crypto_core::{Identity, IdentityKeys};
use curvyo_document_core::{
    AssetHash, CuttableChoice, MAX_SYNCED_FONT_BYTES, MAX_SYNCED_FONTS_BYTES,
};
use curvyo_editor_wasm::{
    DeviceGroupGlue, FontArrival, FontChange, FontInput, SyncSeeds, TransportAction,
};
use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput, StoreOp};
use curvyo_ui_core::library_text as texts;

const SERVER: &str = "ws://relay";
const ROOM: [u8; 16] = [0x6e; 16];
const MIB: usize = 1024 * 1024;

/// F1: the file name carries the family-name marker of criterion 24; the tag is the marker.
const F1_NAME: &str = "ZQX-FONT-7731.ttf";
const F1_FAMILY: &[u8] = b"ZQX-FONT-7731";
const TAG: &str = "ZQX-TAG-7731";

/// A high-entropy file: a xorshift stream, so the server-data scans cannot match protocol bytes
/// by chance and compression cannot hide a repeated chunk.
fn file_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

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

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

fn input(name: &str, folder: Option<&str>, tags: &[&str], favourite: bool) -> FontInput {
    FontInput {
        file_name: name.to_owned(),
        folder: folder.map(str::to_owned),
        tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        favourite,
        cuttable: CuttableChoice::Auto,
        documents: Vec::new(),
    }
}

/// One device: its group glue, the font files its page holds (fonts folder and documents, by
/// hash), the arrivals the page took, the epoch keys, and the thread CPU time its glue work took.
struct Device {
    glue: DeviceGroupGlue,
    connection: Option<ConnectionId>,
    offline: bool,
    host: BTreeMap<AssetHash, Vec<u8>>,
    pending: Vec<AssetHash>,
    arrivals: Vec<FontArrival>,
    keys: Vec<(u32, [u8; 32])>,
    cpu: Duration,
}

/// The devices around one in-memory relay; `wire` and `stored` collect everything the server
/// sees (every byte on the wire, every store write).
struct Net {
    relay: Relay,
    devices: Vec<Device>,
    now: u64,
    entropy: u8,
    stored: Vec<u8>,
    wire: Vec<u8>,
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
        }
    }

    fn add(&mut self, glue: DeviceGroupGlue) -> usize {
        self.devices.push(Device {
            glue,
            connection: None,
            offline: false,
            host: BTreeMap::new(),
            pending: Vec::new(),
            arrivals: Vec::new(),
            keys: Vec::new(),
            cpu: Duration::ZERO,
        });
        self.devices.len() - 1
    }

    fn glue(&mut self, index: usize) -> &mut DeviceGroupGlue {
        &mut self.devices[index].glue
    }

    /// The page's work of one poll: it takes the font arrivals, writes what arrived to its
    /// fonts folder (`host`) once the bytes are complete, reports the hashes it holds and
    /// supplies the bytes the glue asks for. No input of the maker.
    fn page(&mut self, index: usize) {
        let started = cpu_time_of(|| self.page_work(index)).1;
        self.devices[index].cpu += started;
    }

    fn page_work(&mut self, index: usize) {
        let device = &mut self.devices[index];
        for arrival in device.glue.take_font_arrivals() {
            match &arrival.change {
                FontChange::Add(details) => {
                    device.pending.push(arrival.hash.clone());
                    for document in &details.documents {
                        device.pending.push(document.hash.clone());
                    }
                }
                FontChange::Removed => {
                    device.host.remove(&arrival.hash);
                }
                FontChange::Meta(_) => {}
            }
            device.arrivals.push(arrival);
        }
        let pending = std::mem::take(&mut device.pending);
        for hash in pending {
            let bytes = device.glue.font_blob(&hash).to_vec();
            if bytes.is_empty() {
                device.pending.push(hash);
            } else {
                device.host.insert(hash, bytes);
            }
        }
        let held: BTreeSet<AssetHash> = device.host.keys().cloned().collect();
        device.glue.font_host_hashes(held);
        for hash in device.glue.font_wants() {
            if let Some(bytes) = device.host.get(&hash).cloned() {
                device.glue.font_supply_blob(&hash, bytes);
            }
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
                    if let Some(index) = self
                        .devices
                        .iter()
                        .position(|d| d.connection == Some(connection))
                    {
                        let glue = &mut self.devices[index].glue;
                        let ((), spent) = cpu_time_of(|| glue.received(&bytes, now));
                        self.devices[index].cpu += spent;
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

    /// Runs virtual time until `arrived` holds, at most `ms` (the LAN budget of the criteria).
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
        self.devices[index].glue.network_back(Millis(self.now));
    }

    /// The font arrivals of `index` for `hash`, in order.
    fn arrivals_of(&self, index: usize, hash: &AssetHash) -> Vec<&FontChange> {
        self.devices[index]
            .arrivals
            .iter()
            .filter(|a| &a.hash == hash)
            .map(|a| &a.change)
            .collect()
    }

    fn has_add(&self, index: usize, hash: &AssetHash) -> bool {
        self.arrivals_of(index, hash)
            .iter()
            .any(|c| matches!(c, FontChange::Add(_)))
    }

    /// The page installs a font on `index`: the glue switches it on, and the page keeps the
    /// file (and its documents) in its fonts folder.
    fn install(&mut self, index: usize, input: FontInput, bytes: Vec<u8>) -> AssetHash {
        let documents = input.documents.clone();
        let hash = self.devices[index]
            .glue
            .font_set(input, bytes.clone())
            .unwrap();
        let host = &mut self.devices[index].host;
        host.insert(hash.clone(), bytes);
        for (_, document) in documents {
            host.insert(AssetHash::of(&document), document);
        }
        hash
    }
}

/// A and B paired as `0096` criteria 1 to 3 built them, both online, no font yet.
fn paired() -> (Net, usize, usize) {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    assert!(net.devices[a].glue.is_online());
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
    );
    net.run(1_500);
    assert!(net.devices[b].glue.is_online(), "B joined the room");
    (net, a, b)
}

/// The hash of F1's bytes, the file the fonts of criteria 16 to 18 are built on.
fn f1_bytes() -> Vec<u8> {
    file_bytes(1, 300_000)
}

// ---- criteria 4 and 16: an on-switched font arrives with its bytes and metadata -----------------

/// Criteria 4 and 16: F1 installed on A starts on (Frage 214 A), arrives on B within 30 s without
/// input, with the file's bytes (the SHA-256 of what B holds equals the hash, criterion 16's
/// comparison), its tag, star and Cuttable as on A, and one arrival only.
#[test]
fn c4_c16_a_switched_on_font_arrives_with_its_bytes_and_metadata() {
    let (mut net, a, b) = paired();
    let bytes = f1_bytes();
    let hash = net.install(a, input(F1_NAME, None, &[TAG], true), bytes.clone());
    let entry = net.devices[a].glue.library().font_entry(&hash).unwrap();
    assert!(entry.sync, "a new font starts on (criterion 4)");
    assert!(
        net.within(30_000, |n| n.has_add(b, &hash)),
        "B receives F1 within 30 s"
    );
    let arrived = net.arrivals_of(b, &hash);
    assert_eq!(arrived.len(), 1, "one arrival for F1 on B");
    let FontChange::Add(details) = arrived[0] else {
        panic!("first change of a new font is an Add");
    };
    assert_eq!(details.file_name, F1_NAME);
    assert_eq!(details.size, bytes.len() as u64);
    assert_eq!(details.tags, vec![TAG.to_owned()], "the tag travels");
    assert!(details.favourite, "the star travels");
    assert_eq!(details.cuttable, CuttableChoice::Auto);
    assert_eq!(
        AssetHash::of(&net.devices[b].host[&hash]),
        hash,
        "B's file hashes to the entry's hash"
    );
    assert_eq!(
        net.devices[b].glue.font_blob(&hash),
        bytes.as_slice(),
        "the glue holds the same bytes"
    );
    assert!(
        net.devices[b]
            .glue
            .library()
            .font_entry(&hash)
            .unwrap()
            .sync
    );
}

/// Criterion 16 edge: a blob that does not hash to what it claims is dropped; the bytes B
/// already holds stay (the glue's own check against `font_supply_blob`).
#[test]
fn c16_hash_mismatch_is_dropped_and_the_held_bytes_stay() {
    let (mut net, a, b) = paired();
    let bytes = f1_bytes();
    let hash = net.install(a, input(F1_NAME, None, &[], false), bytes.clone());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    let wrong = file_bytes(77, 1_000);
    net.glue(b).font_supply_blob(&hash, wrong.clone());
    assert_eq!(
        net.glue(b).font_blob(&hash),
        bytes.as_slice(),
        "a wrong blob for a hash is dropped"
    );
    let other_hash = AssetHash::of(&wrong);
    net.glue(b).font_supply_blob(&other_hash, wrong);
    assert!(
        net.glue(b).font_blob(&other_hash).is_empty(),
        "a blob for a hash nobody named is dropped"
    );
}

/// Criterion 16: a collection file (one file, several faces) is one entry, one blob and one
/// arrival on the other device, however many times the page installs it.
#[test]
fn c16_a_collection_file_is_one_entry_and_one_arrival() {
    let (mut net, a, b) = paired();
    let bytes = file_bytes(5, 200_000);
    let first = net.install(
        a,
        input("ZQX-COLL-7731.ttc", None, &[], false),
        bytes.clone(),
    );
    let second = net.install(a, input("ZQX-COLL-7731.ttc", None, &[], false), bytes);
    assert_eq!(first, second, "one file, one hash");
    assert_eq!(net.devices[a].glue.library().font_entries().len(), 1);
    assert!(net.within(30_000, |n| n.has_add(b, &first)));
    net.run(2_000);
    assert_eq!(
        net.arrivals_of(b, &first).len(),
        1,
        "the file is transferred once"
    );
    assert_eq!(net.devices[b].glue.library().font_entries().len(), 1);
}

// ---- criterion 17: metadata converges, bytes never conflict --------------------------------------

/// Criterion 17: a tag and star change on A travels to B as a Meta change, and B reads it.
#[test]
fn c17_a_metadata_change_travels_to_the_other_device() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[TAG], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.glue(a).font_meta(
        &hash,
        vec!["ZQX-TAG-2".to_owned()],
        true,
        CuttableChoice::Yes,
    );
    assert!(
        net.within(30_000, |n| n
            .arrivals_of(b, &hash)
            .iter()
            .any(|c| matches!(c, FontChange::Meta(d) if d.favourite))),
        "B takes the Meta change"
    );
    let details = net.devices[b]
        .glue
        .library()
        .font_entry(&hash)
        .unwrap()
        .details
        .unwrap();
    assert_eq!(details.tags, vec!["ZQX-TAG-2".to_owned()]);
    assert!(details.favourite);
    assert_eq!(details.cuttable, CuttableChoice::Yes);
}

/// Criterion 17: both devices change the metadata offline; on reconnect both show the same
/// values (one side's whole set, no copy of the font), and a notice is raised on a device.
#[test]
fn c17_offline_metadata_conflict_converges_with_one_value_and_a_notice() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[TAG], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.go_offline(a);
    net.go_offline(b);
    net.glue(a).font_meta(
        &hash,
        vec!["ZQX-TAG-A".to_owned()],
        true,
        CuttableChoice::Yes,
    );
    net.glue(b).font_meta(
        &hash,
        vec!["ZQX-TAG-B".to_owned()],
        false,
        CuttableChoice::No,
    );
    net.go_online(a);
    net.go_online(b);
    net.run(5_000);
    let on_a = net.devices[a]
        .glue
        .library()
        .font_entry(&hash)
        .unwrap()
        .details
        .unwrap();
    let on_b = net.devices[b]
        .glue
        .library()
        .font_entry(&hash)
        .unwrap()
        .details
        .unwrap();
    assert_eq!(on_a.tags, on_b.tags, "tags converge");
    assert_eq!(on_a.favourite, on_b.favourite, "star converges");
    assert_eq!(on_a.cuttable, on_b.cuttable, "Cuttable converges");
    // ADR 0062 decision 5: the metadata fields are registers, last writer wins per field, so each
    // converged field is one of the two sides' values (the fields need not come from one side).
    let tags_ok = on_a.tags == ["ZQX-TAG-A".to_owned()] || on_a.tags == ["ZQX-TAG-B".to_owned()];
    assert!(tags_ok, "tags is one side's value: {on_a:?}");
    assert!(
        matches!(on_a.cuttable, CuttableChoice::Yes | CuttableChoice::No),
        "Cuttable is one side's value: {on_a:?}"
    );
    assert_eq!(
        net.devices[a].glue.library().font_entries().len(),
        1,
        "no copy of the font"
    );
    assert_eq!(
        net.devices[b].glue.library().font_entries().len(),
        1,
        "no copy of the font"
    );
    let notices: Vec<String> = [a, b]
        .iter()
        .filter_map(|i| net.devices[*i].glue.take_notice())
        .collect();
    assert!(!notices.is_empty(), "a notice names the conflicting field");
}

/// Criterion 17 (tag rename, Proposal): a tag renamed in the Tags list renames on B for the
/// synced font only; the switched-off font's change stays on A, and no change of it reaches B.
#[test]
fn c17_a_tag_rename_reaches_synced_fonts_and_stays_local_for_switched_off_ones() {
    let (mut net, a, b) = paired();
    let synced = net.install(a, input(F1_NAME, None, &[TAG], false), f1_bytes());
    let off = net.install(
        a,
        input("ZQX-OFF-7731.ttf", None, &[TAG], false),
        file_bytes(9, 50_000),
    );
    assert!(net.within(30_000, |n| n.has_add(b, &synced) && n.has_add(b, &off)));
    net.glue(a).font_switch_off(&off);
    net.run(1_000);
    let renamed = vec!["ZQX-TAG-NEW".to_owned()];
    net.glue(a)
        .font_meta(&synced, renamed.clone(), false, CuttableChoice::Auto);
    assert!(
        net.within(30_000, |n| n
            .arrivals_of(b, &synced)
            .iter()
            .any(|c| matches!(c, FontChange::Meta(d) if d.tags == renamed))),
        "the synced font takes the rename"
    );
    let meta_for_off = net
        .arrivals_of(b, &off)
        .iter()
        .filter(|c| matches!(c, FontChange::Meta(_)))
        .count();
    assert_eq!(
        meta_for_off, 0,
        "the switched-off font's rename never reaches B"
    );
}

// ---- criterion 18: a font from a zip keeps its folder and licence files --------------------------

/// Criterion 18: a font installed from a zip arrives with its folder and its kept licence and
/// readme files; B holds the documents' bytes under their hashes.
#[test]
fn c18_a_zip_font_arrives_with_its_folder_and_licence_files() {
    let (mut net, a, b) = paired();
    let licence = b"SIL Open Font License, ZQX licence text".to_vec();
    let readme = b"ZQX readme: how to use this pack".to_vec();
    let mut zip_input = input("ZQX-PACK-7731.ttf", Some("ZQX-PACK-7731"), &[TAG], false);
    zip_input.documents = vec![("OFL.txt".to_owned(), licence.clone())];
    net.glue(a)
        .font_stage_document("README.txt", readme.clone());
    let hash = net.install(a, zip_input, f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    let FontChange::Add(details) = net.arrivals_of(b, &hash)[0] else {
        panic!("an Add");
    };
    assert_eq!(details.folder.as_deref(), Some("ZQX-PACK-7731"));
    let mut names: Vec<&str> = details.documents.iter().map(|d| d.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["OFL.txt", "README.txt"],
        "both kept files travel"
    );
    for (name, bytes) in [("OFL.txt", &licence), ("README.txt", &readme)] {
        let document = details.documents.iter().find(|d| d.name == name).unwrap();
        assert_eq!(document.hash, AssetHash::of(bytes), "{name} keeps its hash");
        assert_eq!(
            net.devices[b].glue.font_blob(&document.hash),
            bytes.as_slice(),
            "{name} bytes on B"
        );
    }
}

// ---- criterion 19: budgets refuse with the numbers and write nothing -----------------------------

/// Criterion 19: the ADR 0062 decision 6 budgets as written (16 MiB per font file, 512 MiB per
/// library); the glue refuses the library's larger values in its own constants.
#[test]
fn c19_the_budgets_are_the_adr_0062_values() {
    assert_eq!(MAX_SYNCED_FONT_BYTES, 16 * 1024 * 1024);
    assert_eq!(MAX_SYNCED_FONTS_BYTES, 512 * 1024 * 1024);
}

/// Criterion 19: a font of 16 MiB + 1 byte refuses to switch on with its reason and the numbers
/// (16.1 MiB over 16 MiB), writes no entry, no blob and no part, and no device receives it.
#[test]
fn c19_over_the_file_limit_refuses_with_numbers_and_writes_nothing() {
    let (mut net, a, b) = paired();
    let too_big = file_bytes(3, 16 * MIB + 1);
    let size = too_big.len();
    let refusal = net
        .glue(a)
        .font_set(input("ZQX-BIG-7731.ttf", None, &[], false), too_big)
        .unwrap_err();
    assert!(refusal.contains("16.1 MiB"), "the size is named: {refusal}");
    assert!(refusal.contains("16 MiB"), "the limit is named: {refusal}");
    assert_eq!(size, 16 * MIB + 1);
    assert!(
        net.devices[a].glue.library().font_entries().is_empty(),
        "no entry"
    );
    assert!(net.glue(a).font_wants().is_empty(), "no blob waits");
    net.run(5_000);
    assert!(
        net.devices[b].arrivals.is_empty(),
        "B receives nothing, no partial transfer"
    );
}

/// Criterion 19 edge: a font of exactly 16 MiB is within the budget and switches on.
#[test]
fn c19_an_exact_file_limit_is_accepted() {
    let (mut net, a, b) = paired();
    let bytes = file_bytes(4, 16 * MIB);
    let hash = net.install(a, input("ZQX-EXACT-7731.ttf", None, &[], false), bytes);
    assert!(
        net.devices[a]
            .glue
            .library()
            .font_entry(&hash)
            .unwrap()
            .sync
    );
    assert!(
        net.within(60_000, |n| n.has_add(b, &hash)),
        "B receives the 16 MiB font"
    );
}

// ---- criterion 20: a first sync of 20 fonts (40 MiB) within 60 s and 2 s of CPU ------------------

/// Criterion 20: twenty fonts of 2 MiB each (40 MiB) installed on A before B joins arrive on B
/// within 60 s of virtual time, and B's glue work costs at most 2 s of thread CPU time (the CPU
/// figure is asserted in a release build only; a debug build is a smoke run).
#[test]
fn c20_a_first_sync_of_20_fonts_completes_within_60_s_and_2_s_cpu() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
    );
    net.run(500);
    net.glue(a).add_device(identity(2), "Laptop");
    net.run(200);
    let hashes: Vec<AssetHash> = (0..20_u64)
        .map(|i| {
            net.install(
                a,
                input(&format!("ZQX-BULK-{i:02}.ttf"), None, &[], false),
                file_bytes(100 + i, 2 * MIB),
            )
        })
        .collect();
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
    );
    let all = |n: &Net| hashes.iter().all(|h| n.has_add(b, h));
    assert!(net.within(60_000, all), "all 20 fonts arrive within 60 s");
    let cpu = net.devices[b].cpu;
    eprintln!("B's glue work took {cpu:?} of thread CPU time (asserted in an optimised build)");
    if !cfg!(debug_assertions) {
        assert!(
            cpu <= Duration::from_secs(2),
            "B's glue work took {cpu:?} of thread CPU time"
        );
    }
}

// ---- criteria 10 and 11: switching off keeps the copies, delete travels ---------------------------

/// Criterion 10: switching F1 off on A leaves B's copy in place (no removal arrives, the
/// page's file stays) and its entry reads off on B.
#[test]
fn c10_switching_off_keeps_the_copy_on_the_other_device() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.glue(a).font_switch_off(&hash);
    assert!(
        net.within(30_000, |n| !net_entry_on(n, b, &hash)),
        "B reads the entry off"
    );
    net.run(3_000);
    assert!(
        !net.arrivals_of(b, &hash)
            .iter()
            .any(|c| matches!(c, FontChange::Removed)),
        "switching off removes nothing on B"
    );
    assert!(
        net.devices[b].host.contains_key(&hash),
        "B's file stays where the page wrote it"
    );
}

fn net_entry_on(net: &Net, index: usize, hash: &AssetHash) -> bool {
    net.devices[index]
        .glue
        .library()
        .font_entry(hash)
        .is_some_and(|e| e.sync)
}

/// Criterion 11: deleting a synced font on A removes it on B (a Removed arrival, B's entry is
/// gone); the copy on B is the page's to delete.
#[test]
fn c11_delete_of_a_synced_font_travels_to_the_other_device() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.glue(a).font_remove(&hash);
    assert!(
        net.within(30_000, |n| n
            .arrivals_of(b, &hash)
            .iter()
            .any(|c| matches!(c, FontChange::Removed))),
        "B receives the removal"
    );
    assert!(
        net.devices[b].glue.library().font_entry(&hash).is_none(),
        "B's entry is gone"
    );
    assert!(
        !net.devices[b].host.contains_key(&hash),
        "the page deleted B's file"
    );
}

/// Criterion 11: with the toggle off, a delete on A stays local; B keeps the font.
#[test]
fn c11_delete_with_the_toggle_off_stays_local() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.glue(a).font_switch_off(&hash);
    net.run(2_000);
    net.glue(a).font_remove(&hash);
    net.run(5_000);
    assert!(
        !net.arrivals_of(b, &hash)
            .iter()
            .any(|c| matches!(c, FontChange::Removed)),
        "a delete of an off font never reaches B"
    );
    assert!(net.devices[b].host.contains_key(&hash), "B keeps its copy");
}

// ---- criteria 21 and 23: a removed device keeps copies, a later device gets the on-items ----------

/// Criterion 23: a third device paired through B receives F1 (on) without input and never F2
/// (switched off before the third device joined).
#[test]
fn c23_a_third_device_gets_the_on_switched_fonts_only() {
    let (mut net, a, b) = paired();
    let on = net.install(a, input(F1_NAME, None, &[], false), f1_bytes());
    let off = net.install(
        a,
        input("ZQX-HIDE-7731.ttf", None, &[], false),
        file_bytes(12, 40_000),
    );
    net.glue(a).font_switch_off(&off);
    assert!(net.within(30_000, |n| n.has_add(b, &on)));
    net.glue(b).add_device(identity(3), "Phone");
    net.run(300);
    let c = net
        .add(DeviceGroupGlue::join(SERVER, ROOM, "Phone", &seeds(3), 13, Millis(net.now)).unwrap());
    assert!(net.within(30_000, |n| n.has_add(c, &on)), "C receives F1");
    net.run(3_000);
    assert!(
        !net.has_add(c, &off),
        "C never receives the switched-off font"
    );
}

/// Criterion 21: after A removes B, B keeps every font it received as a local usable copy, and
/// a font A installs afterwards does not reach B.
#[test]
fn c21_removal_keeps_the_local_font_copies_and_cuts_b_off() {
    let (mut net, a, b) = paired();
    let hash = net.install(a, input(F1_NAME, None, &[], false), f1_bytes());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    let member = net.devices[b].glue.me();
    net.glue(a).remove_device(member).unwrap();
    net.run(3_000);
    assert!(net.devices[b].glue.removed(), "B learns it is removed");
    assert!(
        net.devices[b].host.contains_key(&hash),
        "B keeps its usable copy of F1"
    );
    let later = net.install(
        a,
        input("ZQX-LATER-7731.ttf", None, &[], false),
        file_bytes(21, 30_000),
    );
    net.run(10_000);
    assert!(
        !net.has_add(b, &later),
        "a font written after the removal never opens on B"
    );
}

// ---- criterion 24, 25 and 27: the server's data holds no font name, bytes, tag or key -------------

/// Criteria 24 and 25 for fonts: after F1 (family name, tag, bytes) synced to B and was switched
/// off on A, the wire and every store write hold none of the plaintext markers, the bytes of F1
/// or any epoch key of the group.
#[test]
fn c24_c25_c27_the_server_store_holds_no_font_name_tag_bytes_or_key() {
    let (mut net, a, b) = paired();
    let bytes = f1_bytes();
    let hash = net.install(a, input(F1_NAME, None, &[TAG], true), bytes.clone());
    assert!(net.within(30_000, |n| n.has_add(b, &hash)));
    net.run(2_000);
    net.glue(a).font_switch_off(&hash);
    net.run(10_000);
    let chunk = &bytes[1_000..1_032];
    for (label, needle) in [
        ("the family name", F1_FAMILY),
        ("the tag", TAG.as_bytes()),
        ("the file name", F1_NAME.as_bytes()),
        ("a chunk of F1's bytes", chunk),
    ] {
        assert!(!contains(&net.wire, needle), "{label} is on the wire");
        assert!(
            !contains(&net.stored, needle),
            "{label} is in the server's store"
        );
    }
    for device in &net.devices {
        for (_, key) in &device.keys {
            assert!(
                !contains(&net.wire, &key[..16]),
                "an epoch key is on the wire"
            );
            assert!(
                !contains(&net.stored, &key[..16]),
                "an epoch key is in the server's store"
            );
        }
    }
    assert!(
        !net.stored.is_empty(),
        "the scan saw the server's store writes"
    );
}

// ---- criterion 5 (font notice) ---------------------------------------------------------------------

/// Criterion 5 for fonts: the arrival notice names the font, the name cut to 24 characters, at
/// most 70 characters in all (UX notes section 5).
#[test]
fn c5_the_font_arrival_notice_is_at_most_70_characters_and_names_the_font() {
    let short = texts::font_arrived_notice("ZQX-FONT-7731");
    assert!(short.contains("ZQX-FONT-7731"));
    assert!(short.chars().count() <= 70);
    let long = texts::font_arrived_notice(&"ZQX-LONG-NAME-".repeat(8));
    assert!(
        long.chars().count() <= 70,
        "a long name is cut: {}",
        long.chars().count()
    );
}
