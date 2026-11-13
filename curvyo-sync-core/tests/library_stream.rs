//! The library stream of the device group room (`specs/0116-library-sync` criteria 5, 6, 13 and
//! 23, ADR 0062 decision 1): the stream ids of the room's two sealed streams, the routing of a
//! record's plaintext, the framed room snapshot and the golden library records.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{LibraryKind, LibraryList};
use curvyo_sync_core::{
    DEVICES_STREAM_ID, GroupRecord, LIBRARY_STREAM_ID, SnapshotAssembler, StreamError,
    encode_group_snapshot, encode_inner_record, encode_library_record, route_group_record,
    split_snapshot,
};

const PEER: u64 = 0x0102_0304_0506_0708;
const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const MATERIAL: &str = "5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875";

const MACHINE_TEXT: &str = "{
  \"format_version\": 4,
  \"id\": \"6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f\",
  \"kind\": \"laser\",
  \"modified\": 1765000000000,
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
    \"connector_address\": \"192.168.1.40:8080\",
    \"controller_port\": \"/dev/ttyUSB0\",
    \"baud_rate\": 115200
  },
  \"fixtures\": []
}";
const MATERIAL_TEXT: &str = "{
  \"format_version\": 2,
  \"id\": \"5d4e3f2a-1b0c-4a9d-8e7f-2c3b4a596875\",
  \"kind\": \"laser\",
  \"modified\": 1765000000000,
  \"name\": \"ZQX-MAT-7731\",
  \"thickness_mm\": 3.0,
  \"lasers\": [],
  \"tests\": [],
  \"history\": []
}";

/// The payload of a golden record as a routed library update.
fn golden_update(name: &str) -> Vec<u8> {
    let bytes = std::fs::read(fixture(name)).unwrap();
    match route_group_record(&bytes).unwrap() {
        GroupRecord::Library { peer, payload } => {
            assert_eq!(peer, PEER);
            payload.to_vec()
        }
        GroupRecord::Devices(_) => panic!("{name} is not a library record"),
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/library")
        .join(name)
}

fn golden(name: &str, bytes: &[u8]) {
    let path = fixture(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name} differs");
}

/// The golden bytes of the stream records v1 (ADR 0062 consequences), one update per record
/// from one sender's history: a machine and a material arriving, a switch-off marker and a
/// deletion; regenerate with `CURVYO_UPDATE_GOLDEN=1`.
#[test]
fn golden_library_records_v1() {
    let a = LibraryList::new(PEER);
    let before_machine = a.version();
    a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT).unwrap();
    let machine = a.export_updates(&before_machine).unwrap();
    let before_material = a.version();
    a.set(LibraryKind::Material, MATERIAL, MATERIAL_TEXT)
        .unwrap();
    let material = a.export_updates(&before_material).unwrap();
    let before_off = a.version();
    a.switch_off(LibraryKind::Machine, MACHINE).unwrap();
    let off = a.export_updates(&before_off).unwrap();
    let before_deletion = a.version();
    a.remove(LibraryKind::Machine, MACHINE).unwrap();
    let deletion = a.export_updates(&before_deletion).unwrap();

    golden(
        "machine_record_v1.bin",
        &encode_library_record(PEER, &machine),
    );
    golden(
        "material_record_v1.bin",
        &encode_library_record(PEER, &material),
    );
    golden(
        "machine_off_marker_v1.bin",
        &encode_library_record(PEER, &off),
    );
    golden(
        "machine_deletion_v1.bin",
        &encode_library_record(PEER, &deletion),
    );
}

/// The golden records are a real sender's history (criteria 5, 10 and 11): a device that
/// joined by the room's snapshot takes each record in order — the arrivals, the switch-off
/// marker, the deletion.
#[test]
fn the_golden_records_apply_to_a_joined_list() {
    let a = LibraryList::new(PEER);
    a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT).unwrap();
    let b = LibraryList::from_snapshot(0x0a0b_0c0d_0e0f_1011, &a.snapshot().unwrap()).unwrap();
    let machine = b.entry(LibraryKind::Machine, MACHINE).unwrap();
    assert!(machine.sync);
    assert_eq!(machine.text.as_deref(), Some(MACHINE_TEXT));

    b.import_updates(&golden_update("material_record_v1.bin"))
        .unwrap();
    let material = b.entry(LibraryKind::Material, MATERIAL).unwrap();
    assert!(material.sync);
    assert_eq!(material.text.as_deref(), Some(MATERIAL_TEXT));

    b.import_updates(&golden_update("machine_off_marker_v1.bin"))
        .unwrap();
    let off = b.entry(LibraryKind::Machine, MACHINE).unwrap();
    assert!(!off.sync);
    assert_eq!(off.text, None);

    b.import_updates(&golden_update("machine_deletion_v1.bin"))
        .unwrap();
    assert_eq!(b.entry(LibraryKind::Machine, MACHINE), None);
}

/// ADR 0062 decision 1: the library stream's records carry the ADR 0037 inner header, so a
/// session routes them apart from the bare devices list updates a pre-0116 build writes.
#[test]
fn a_record_is_routed_by_its_inner_header() {
    let library = encode_inner_record(LIBRARY_STREAM_ID, PEER, b"loro update");
    assert_eq!(
        route_group_record(&library),
        Ok(GroupRecord::Library {
            peer: PEER,
            payload: b"loro update",
        })
    );
    assert_eq!(
        route_group_record(b"bare devices update"),
        Ok(GroupRecord::Devices(b"bare devices update"))
    );
    assert_eq!(
        route_group_record(&library[..20]),
        Err(StreamError::Malformed)
    );
}

/// The group room's snapshot is a framed snapshot stream of both lists, devices first, cut into
/// parts and assembled by the existing machinery (ADR 0062 decision 1).
#[test]
fn the_room_snapshot_carries_both_streams() {
    let devices: Vec<u8> = vec![0x11, 0x22, 0x33];
    let library = encode_library_record(PEER, b"loro snapshot");
    let stream = encode_group_snapshot(&devices, &library);
    let parts = split_snapshot(&stream, [0x77; 16]);
    assert_eq!(parts.len(), 1);
    let mut assembler = SnapshotAssembler::default();
    let entries = assembler.push_part(&parts[0]).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].stream, DEVICES_STREAM_ID);
    assert_eq!(entries[0].bytes, devices);
    assert_eq!(entries[1].stream, LIBRARY_STREAM_ID);
    assert_eq!(entries[1].bytes, library);
    assert!(assembler.is_complete());
}
