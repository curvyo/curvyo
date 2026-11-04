//! The devices list of a maker's device group (`specs/0096-own-devices-sync`, ADR 0040 decision
//! 1): which devices belong together under which label, and which workspaces sync with which
//! name; registers, last writer wins, replicas converge.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    DEVICES_LIST_FORMAT_VERSION, DeviceEntry, DevicesList, DevicesListError, SyncedWorkspace,
};

const A: [u8; 32] = [0xa1; 32];
const B: [u8; 32] = [0xb2; 32];
const C: [u8; 32] = [0xc3; 32];
const W1: [u8; 16] = [0x11; 16];
const W2: [u8; 16] = [0x22; 16];
const W3: [u8; 16] = [0x33; 16];

/// Each replica's updates imported into the other.
fn sync(a: &DevicesList, b: &DevicesList) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

fn device(member: [u8; 32], label: &str) -> DeviceEntry {
    DeviceEntry {
        member,
        label: label.to_owned(),
        browser: false,
    }
}

fn workspace(id: [u8; 16], name: &str, on: bool) -> SyncedWorkspace {
    SyncedWorkspace {
        id,
        name: name.to_owned(),
        on,
    }
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Criteria 3, 6 and 22: labels of every paired device, sorted by member; a third device added
/// on one replica shows on the other.
#[test]
fn labels_of_paired_devices_reach_every_replica() {
    let on_a = DevicesList::new(1);
    on_a.set_label(A, "Workshop").unwrap();
    on_a.set_label(B, "Laptop").unwrap();
    let on_b = DevicesList::from_snapshot(2, &on_a.snapshot().unwrap()).unwrap();
    // B pairs C (criterion 6).
    on_b.set_label(C, "Office").unwrap();
    sync(&on_a, &on_b);
    let expected = vec![
        device(A, "Workshop"),
        device(B, "Laptop"),
        device(C, "Office"),
    ];
    assert_eq!(on_a.devices(), expected);
    assert_eq!(on_b.devices(), expected);
    assert_eq!(on_a.label(&C).as_deref(), Some("Office"));
    assert_eq!(on_a.label(&[0; 32]), None);
}

/// Criteria 23 and 27: a removed or left device leaves the list; removing an absent one does
/// nothing.
#[test]
fn a_removed_device_leaves_the_list() {
    let list = DevicesList::new(1);
    list.set_label(A, "Workshop").unwrap();
    list.set_label(B, "Laptop").unwrap();
    list.remove_device(&B).unwrap();
    list.remove_device(&C).unwrap();
    assert_eq!(list.devices(), vec![device(A, "Workshop")]);
}

#[test]
fn an_empty_label_is_refused() {
    let list = DevicesList::new(1);
    assert_eq!(list.set_label(A, ""), Err(DevicesListError::EmptyLabel));
    assert_eq!(list.set_label(A, "  "), Err(DevicesListError::EmptyLabel));
    assert_eq!(list.devices(), Vec::new());
}

/// Criteria 8, 9, 11 and 13: a switched-on workspace reaches every replica with its name; off
/// keeps the entry with `on: false`.
#[test]
fn switches_reach_every_replica() {
    let on_a = DevicesList::new(1);
    let on_b = DevicesList::new(2);
    on_a.switch_workspace(W1, "Boxes", true).unwrap();
    on_a.switch_workspace(W2, "Lamps", true).unwrap();
    sync(&on_a, &on_b);
    assert_eq!(
        on_b.workspaces(),
        vec![workspace(W1, "Boxes", true), workspace(W2, "Lamps", true)]
    );
    on_a.switch_workspace(W2, "Lamps", false).unwrap();
    sync(&on_a, &on_b);
    assert_eq!(on_b.workspace(&W2), Some(workspace(W2, "Lamps", false)));
    assert_eq!(on_b.switched_on(), vec![W1]);
}

/// Criterion 12: a workspace never switched on is not in the list.
#[test]
fn a_workspace_never_switched_on_is_absent() {
    let list = DevicesList::new(1);
    list.switch_workspace(W1, "Boxes", true).unwrap();
    assert_eq!(list.workspace(&W3), None);
    assert!(!list.switched_on().contains(&W3));
}

/// Concurrent writes of one register: the replicas converge on one value (last writer wins).
#[test]
fn concurrent_writes_converge() {
    let on_a = DevicesList::new(1);
    let on_b = DevicesList::new(2);
    on_a.set_label(C, "Office").unwrap();
    on_b.set_label(C, "Office PC").unwrap();
    on_a.switch_workspace(W1, "Boxes", true).unwrap();
    on_b.switch_workspace(W1, "Boxes", false).unwrap();
    // A removal concurrent with a relabel.
    on_a.set_label(B, "Laptop").unwrap();
    sync(&on_a, &on_b);
    on_a.remove_device(&B).unwrap();
    on_b.set_label(B, "Old laptop").unwrap();
    sync(&on_a, &on_b);
    assert_eq!(on_a.devices(), on_b.devices());
    assert_eq!(on_a.workspaces(), on_b.workspaces());
    assert_eq!(on_a.workspaces().len(), 1);
}

#[test]
fn bytes_that_are_not_a_list_are_refused() {
    assert!(matches!(
        DevicesList::from_snapshot(1, b"not loro"),
        Err(DevicesListError::Malformed)
    ));
    let list = DevicesList::new(1);
    assert_eq!(
        list.import_updates(b"not loro"),
        Err(DevicesListError::Malformed)
    );
    assert_eq!(DEVICES_LIST_FORMAT_VERSION, 1);
}

/// A list without a `format_version` (an empty Loro document) is not a devices list.
#[test]
fn a_list_without_a_version_is_refused() {
    let doc = loro::LoroDoc::new();
    doc.get_map("devices").insert("x", 1).unwrap();
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    assert!(matches!(
        DevicesList::from_snapshot(1, &bytes),
        Err(DevicesListError::Malformed)
    ));
}

/// The golden `devices_list_v1.loro` (ADR 0040 decisions 1 and 11); regenerate with
/// `UPDATE_0096_FIXTURES=1`.
#[test]
fn golden_devices_list_v1() {
    let path = fixture("devices_list_v1.loro");
    if std::env::var_os("UPDATE_0096_FIXTURES").is_some() {
        let list = DevicesList::new(1);
        list.set_label(A, "Workshop").unwrap();
        list.set_label(B, "Laptop").unwrap();
        list.switch_workspace(W1, "Boxes", true).unwrap();
        list.switch_workspace(W2, "Lamps", false).unwrap();
        std::fs::write(&path, list.snapshot().unwrap()).unwrap();
    }
    let list = DevicesList::from_snapshot(5, &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        list.devices(),
        vec![device(A, "Workshop"), device(B, "Laptop")]
    );
    assert_eq!(
        list.workspaces(),
        vec![workspace(W1, "Boxes", true), workspace(W2, "Lamps", false)]
    );
}

/// Damaged variants: a list of a newer format is refused; truncated bytes are refused; entries
/// with a bad key or value shape are skipped, the good ones read.
#[test]
fn golden_damaged_devices_lists() {
    let newer = fixture("devices_list_v2.loro");
    let truncated = fixture("devices_list_truncated.loro");
    let odd = fixture("devices_list_odd_entries.loro");
    if std::env::var_os("UPDATE_0096_FIXTURES").is_some() {
        let doc = loro::LoroDoc::new();
        doc.set_peer_id(1).unwrap();
        doc.get_map("group").insert("format_version", 2).unwrap();
        doc.commit();
        std::fs::write(&newer, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();

        let list = DevicesList::new(1);
        list.set_label(A, "Workshop").unwrap();
        list.switch_workspace(W1, "Boxes", true).unwrap();
        let good = list.snapshot().unwrap();
        std::fs::write(&truncated, &good[..good.len() / 2]).unwrap();
        let doc = loro::LoroDoc::new();
        doc.set_peer_id(1).unwrap();
        doc.import(&list.snapshot().unwrap()).unwrap();
        let devices = doc.get_map("devices");
        devices.insert("not hex", "x").unwrap();
        devices.insert(&"ab".repeat(16), "short key").unwrap();
        devices.insert(&"cd".repeat(32), 7).unwrap();
        let workspaces = doc.get_map("workspaces");
        workspaces.insert(&"ee".repeat(16), "no map").unwrap();
        let mut no_flag = std::collections::HashMap::new();
        no_flag.insert("name".to_owned(), loro::LoroValue::from("Half"));
        workspaces
            .insert(&"ff".repeat(16), loro::LoroValue::from(no_flag))
            .unwrap();
        doc.commit();
        std::fs::write(&odd, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    }
    assert!(matches!(
        DevicesList::from_snapshot(1, &std::fs::read(&newer).unwrap()),
        Err(DevicesListError::FormatTooNew)
    ));
    assert!(DevicesList::from_snapshot(1, &std::fs::read(&truncated).unwrap()).is_err());
    let list = DevicesList::from_snapshot(1, &std::fs::read(&odd).unwrap()).unwrap();
    assert_eq!(list.devices(), vec![device(A, "Workshop")]);
    assert_eq!(list.workspaces(), vec![workspace(W1, "Boxes", true)]);
}

/// An update of a newer format merged into a replica is refused.
#[test]
fn an_update_of_a_newer_format_is_refused() {
    let list = DevicesList::new(1);
    let doc = loro::LoroDoc::new();
    doc.set_peer_id(9).unwrap();
    doc.import(&list.snapshot().unwrap()).unwrap();
    doc.get_map("group").insert("format_version", 2).unwrap();
    doc.commit();
    assert_eq!(
        list.import_updates(&doc.export(loro::ExportMode::Snapshot).unwrap()),
        Err(DevicesListError::FormatTooNew)
    );
}

/// UX 11 of the review, ADR 0040 decision 1: a browser marks its own entry; a rename keeps the
/// mark; a field a reader does not know (a newer build's) is ignored, so the v1 golden with an
/// extra field still reads, and a device without the field is no browser.
#[test]
fn the_browser_mark_is_an_optional_field_old_readers_ignore() {
    let list =
        DevicesList::from_snapshot(5, &std::fs::read(fixture("devices_list_v1.loro")).unwrap())
            .unwrap();
    assert!(list.devices().iter().all(|d| !d.browser));
    list.mark_browser(B).unwrap();
    list.mark_browser(C).unwrap();
    list.set_label(B, "Firefox on Linux").unwrap();
    assert_eq!(
        list.device(&B),
        Some(DeviceEntry {
            member: B,
            label: "Firefox on Linux".to_owned(),
            browser: true,
        })
    );
    assert_eq!(list.device(&C), None, "an unlisted device is not added");
    let doc = loro::LoroDoc::new();
    doc.set_peer_id(9).unwrap();
    doc.import(&list.snapshot().unwrap()).unwrap();
    let mut future = std::collections::HashMap::new();
    future.insert("label".to_owned(), loro::LoroValue::from("Workshop"));
    future.insert("colour".to_owned(), loro::LoroValue::from("teal"));
    doc.get_map("devices")
        .insert(&"a1".repeat(32), loro::LoroValue::from(future))
        .unwrap();
    doc.commit();
    let read =
        DevicesList::from_snapshot(1, &doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    assert_eq!(read.device(&A), Some(device(A, "Workshop")));
    assert!(read.device(&B).unwrap().browser);
}

/// ADR 0040 decision 11: an update from a newer build that builds on this replica's history is
/// refused and merges nothing, neither its version nor its other entries.
#[test]
fn an_update_from_a_newer_build_is_refused_and_merges_nothing() {
    let a = DevicesList::new(1);
    a.set_label(A, "Workshop").unwrap();
    let newer = loro::LoroDoc::new();
    newer.set_peer_id(9).unwrap();
    newer.import(&a.snapshot().unwrap()).unwrap();
    let known = newer.oplog_vv();
    newer.get_map("group").insert("format_version", 2).unwrap();
    newer
        .get_map("devices")
        .insert(&"b2".repeat(32), "x")
        .unwrap();
    newer.commit();
    let update = newer.export(loro::ExportMode::updates(&known)).unwrap();
    let before = a.snapshot().unwrap();
    assert_eq!(
        a.import_updates(&update),
        Err(DevicesListError::FormatTooNew)
    );
    assert_eq!(a.snapshot().unwrap(), before);
    assert_eq!(a.devices(), vec![device(A, "Workshop")]);
}
