//! The library list of a maker's device group (`specs/0116-library-sync`, ADR 0062 decisions 1,
//! 2 and 4): machine profiles and material records as their exact record texts with the
//! per-item sync toggle; registers, last writer wins, replicas converge.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use curvyo_document_core::{
    LIBRARY_LIST_FORMAT_VERSION, LibraryEntry, LibraryKind, LibraryList, LibraryListError,
};

const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const OTHER_MACHINE: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
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

fn entry(id: &str, sync: bool, text: Option<&str>) -> LibraryEntry {
    LibraryEntry {
        id: id.to_owned(),
        sync,
        text: text.map(std::borrow::ToOwned::to_owned),
    }
}

/// Each replica's updates imported into the other.
fn sync(a: &LibraryList, b: &LibraryList) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Criteria 5 and 23: a machine set on one replica reaches the other with its exact record text
/// and its toggle on; the same for a material.
#[test]
fn entries_of_both_kinds_reach_every_replica() {
    let on_a = LibraryList::new(1);
    on_a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
        .unwrap();
    on_a.set(LibraryKind::Material, MATERIAL, MATERIAL_TEXT)
        .unwrap();
    let on_b = LibraryList::from_snapshot(2, &on_a.snapshot().unwrap()).unwrap();
    sync(&on_a, &on_b);
    assert_eq!(
        on_b.entries(LibraryKind::Machine),
        vec![entry(MACHINE, true, Some(MACHINE_TEXT))]
    );
    assert_eq!(
        on_b.entries(LibraryKind::Material),
        vec![entry(MATERIAL, true, Some(MATERIAL_TEXT))]
    );
    assert_eq!(
        on_a.entry(LibraryKind::Machine, MACHINE),
        Some(entry(MACHINE, true, Some(MACHINE_TEXT)))
    );
    assert_eq!(on_a.entry(LibraryKind::Machine, OTHER_MACHINE), None);
    assert_eq!(
        LIBRARY_LIST_FORMAT_VERSION, 1,
        "a change takes a new version and a refusal of the old"
    );
}

/// Criterion 4: a bad id or a text over the entry's limit is refused; nothing is written.
#[test]
fn a_bad_id_or_an_oversized_text_is_refused() {
    let list = LibraryList::new(1);
    assert_eq!(
        list.set(LibraryKind::Machine, "not-a-uuid", MACHINE_TEXT),
        Err(LibraryListError::BadId)
    );
    let over_machine = format!(
        "{}{}",
        "x",
        " ".repeat(LibraryKind::Machine.max_text_bytes())
    );
    assert_eq!(
        list.set(LibraryKind::Machine, MACHINE, &over_machine),
        Err(LibraryListError::TooLarge)
    );
    let over_material = format!(
        "{}{}",
        "x",
        " ".repeat(LibraryKind::Material.max_text_bytes())
    );
    assert_eq!(
        list.set(LibraryKind::Material, MATERIAL, &over_material),
        Err(LibraryListError::TooLarge)
    );
    assert_eq!(list.entries(LibraryKind::Machine), Vec::new());
    assert_eq!(list.entries(LibraryKind::Material), Vec::new());
}

/// Criterion 10: switching an item off keeps its entry as a marker without text; switching an
/// item off that never synced writes nothing (criterion 4); setting it again turns it on.
#[test]
fn switching_off_keeps_a_marker_without_text() {
    let on_a = LibraryList::new(1);
    on_a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
        .unwrap();
    let on_b = LibraryList::from_snapshot(2, &on_a.snapshot().unwrap()).unwrap();
    on_a.switch_off(LibraryKind::Machine, MACHINE).unwrap();
    on_a.switch_off(LibraryKind::Machine, OTHER_MACHINE)
        .unwrap();
    sync(&on_a, &on_b);
    assert_eq!(
        on_b.entries(LibraryKind::Machine),
        vec![entry(MACHINE, false, None)]
    );
    on_a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
        .unwrap();
    sync(&on_a, &on_b);
    assert_eq!(
        on_b.entries(LibraryKind::Machine),
        vec![entry(MACHINE, true, Some(MACHINE_TEXT))]
    );
}

/// Criterion 11: deleting an item removes its entry; deleting an absent one does nothing.
#[test]
fn deleting_removes_the_entry() {
    let list = LibraryList::new(1);
    list.set(LibraryKind::Material, MATERIAL, MATERIAL_TEXT)
        .unwrap();
    list.remove(LibraryKind::Material, MATERIAL).unwrap();
    list.remove(LibraryKind::Material, MATERIAL).unwrap();
    assert_eq!(list.entries(LibraryKind::Material), Vec::new());
}

/// Concurrent writes of one entry: the replicas converge on one text (last writer wins).
#[test]
fn concurrent_writes_converge() {
    let on_a = LibraryList::new(1);
    let on_b = LibraryList::new(2);
    let other_text = MACHINE_TEXT.replace("ZQX-MACH-7731", "ZQX-MACH-7732");
    on_a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
        .unwrap();
    on_b.set(LibraryKind::Machine, MACHINE, &other_text)
        .unwrap();
    sync(&on_a, &on_b);
    assert_eq!(
        on_a.entries(LibraryKind::Machine),
        on_b.entries(LibraryKind::Machine)
    );
    assert_eq!(on_a.entries(LibraryKind::Machine).len(), 1);
}

#[test]
fn bytes_that_are_not_a_list_are_refused() {
    assert!(matches!(
        LibraryList::from_snapshot(1, b"not loro"),
        Err(LibraryListError::Malformed)
    ));
    let list = LibraryList::new(1);
    assert_eq!(
        list.import_updates(b"not loro"),
        Err(LibraryListError::Malformed)
    );
}

/// A list without a `format_version` (an empty Loro document) is not a library list.
#[test]
fn a_list_without_a_version_is_refused() {
    let doc = loro::LoroDoc::new();
    doc.get_map("machines").insert("x", 1).unwrap();
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    assert!(matches!(
        LibraryList::from_snapshot(1, &bytes),
        Err(LibraryListError::Malformed)
    ));
}

/// The golden `library_list_v1.loro` (ADR 0062 decisions 1, 2 and 4); regenerate with
/// `UPDATE_0116_FIXTURES=1`.
#[test]
fn golden_library_list_v1() {
    let path = fixture("library_list_v1.loro");
    if std::env::var_os("UPDATE_0116_FIXTURES").is_some() {
        let list = LibraryList::new(1);
        list.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
            .unwrap();
        list.set(LibraryKind::Material, MATERIAL, MATERIAL_TEXT)
            .unwrap();
        std::fs::write(&path, list.snapshot().unwrap()).unwrap();
    }
    let list = LibraryList::from_snapshot(5, &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        list.entries(LibraryKind::Machine),
        vec![entry(MACHINE, true, Some(MACHINE_TEXT))]
    );
    assert_eq!(
        list.entries(LibraryKind::Material),
        vec![entry(MATERIAL, true, Some(MATERIAL_TEXT))]
    );
}

/// Damaged variants: a list of a newer format is refused; truncated bytes are refused; entries
/// with a bad key or value shape are skipped, the good ones read.
#[test]
fn golden_damaged_library_lists() {
    let newer = fixture("library_list_v2.loro");
    let truncated = fixture("library_list_truncated.loro");
    let odd = fixture("library_list_odd_entries.loro");
    if std::env::var_os("UPDATE_0116_FIXTURES").is_some() {
        let doc = loro::LoroDoc::new();
        doc.set_peer_id(1).unwrap();
        doc.get_map("library").insert("format_version", 2).unwrap();
        doc.commit();
        std::fs::write(&newer, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();

        let list = LibraryList::new(1);
        list.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT)
            .unwrap();
        let good = list.snapshot().unwrap();
        std::fs::write(&truncated, &good[..good.len() / 2]).unwrap();
        let doc = loro::LoroDoc::new();
        doc.set_peer_id(1).unwrap();
        doc.import(&list.snapshot().unwrap()).unwrap();
        let machines = doc.get_map("machines");
        machines.insert("not a uuid", "x").unwrap();
        machines.insert(OTHER_MACHINE, 7).unwrap();
        let mut bad_sync = HashMap::new();
        bad_sync.insert("sync".to_owned(), loro::LoroValue::from(7));
        machines
            .insert(MACHINE, loro::LoroValue::from(bad_sync))
            .unwrap();
        let mut bad_text = HashMap::new();
        bad_text.insert("sync".to_owned(), loro::LoroValue::from(true));
        bad_text.insert("text".to_owned(), loro::LoroValue::from(7));
        machines
            .insert(OTHER_MACHINE, loro::LoroValue::from(bad_text))
            .unwrap();
        let materials = doc.get_map("materials");
        let mut marker = HashMap::new();
        marker.insert("sync".to_owned(), loro::LoroValue::from(false));
        materials
            .insert(MATERIAL, loro::LoroValue::from(marker))
            .unwrap();
        doc.commit();
        std::fs::write(&odd, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    }
    assert_eq!(
        LibraryList::from_snapshot(1, &std::fs::read(&newer).unwrap()).err(),
        Some(LibraryListError::FormatTooNew)
    );
    assert!(LibraryList::from_snapshot(1, &std::fs::read(&truncated).unwrap()).is_err());
    let list = LibraryList::from_snapshot(1, &std::fs::read(&odd).unwrap()).unwrap();
    assert_eq!(list.entries(LibraryKind::Machine), Vec::new());
    assert_eq!(
        list.entries(LibraryKind::Material),
        vec![entry(MATERIAL, false, None)]
    );
}

/// ADR 0040 decision 11's rule, carried to the library list: an update of a newer format merged
/// into a replica is refused and merges nothing.
#[test]
fn an_update_of_a_newer_format_is_refused_and_merges_nothing() {
    let a = LibraryList::new(1);
    a.set(LibraryKind::Machine, MACHINE, MACHINE_TEXT).unwrap();
    let newer = loro::LoroDoc::new();
    newer.set_peer_id(9).unwrap();
    newer.import(&a.snapshot().unwrap()).unwrap();
    let known = newer.oplog_vv();
    newer
        .get_map("library")
        .insert("format_version", 2)
        .unwrap();
    newer
        .get_map("machines")
        .insert(OTHER_MACHINE, "x")
        .unwrap();
    newer.commit();
    let update = newer.export(loro::ExportMode::updates(&known)).unwrap();
    let before = a.snapshot().unwrap();
    assert_eq!(
        a.import_updates(&update),
        Err(LibraryListError::FormatTooNew)
    );
    assert_eq!(a.snapshot().unwrap(), before);
    assert_eq!(
        a.entries(LibraryKind::Machine),
        vec![entry(MACHINE, true, Some(MACHINE_TEXT))]
    );
}
