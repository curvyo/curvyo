//! Tester acceptance tests for `specs/0096-own-devices-sync` in `document-core`, written from the
//! specification and ADR 0040 decisions 1 and 11 before the implementation was read: the devices
//! list (names reach every device, switches converge last writer wins, a newer or damaged list is
//! refused) and reading the share members of `format_version` 34 and 35 files (criterion 35).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::path::PathBuf;

use curvyo_document_core::{DevicesList, DevicesListError, container_share_members, unpack};

const A: [u8; 32] = [0xa1; 32];
const B: [u8; 32] = [0xb2; 32];
const C: [u8; 32] = [0xc3; 32];
const W: [u8; 16] = [0x11; 16];
const W2: [u8; 16] = [0x22; 16];

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn sync(a: &DevicesList, b: &DevicesList) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

/// Criteria 3 and 22: a joined device built from the adder's snapshot writes its own name, and
/// the name reaches the adder (and a third device) by updates.
#[test]
fn criterion_22_joined_device_name_reaches_the_others() {
    let a = DevicesList::new(1);
    a.set_label(A, "ZQX-PC-7731 desktop").unwrap();
    a.set_label(B, "Laptop (from pairing)").unwrap();
    let b = DevicesList::from_snapshot(2, &a.snapshot().unwrap()).unwrap();
    b.set_label(B, "My laptop").unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    a.import_updates(&to_a).unwrap();
    assert_eq!(a.label(&B).as_deref(), Some("My laptop"));
    let c = DevicesList::from_snapshot(3, &a.snapshot().unwrap()).unwrap();
    assert_eq!(c.label(&B).as_deref(), Some("My laptop"));
    assert_eq!(c.label(&A).as_deref(), Some("ZQX-PC-7731 desktop"));
}

/// A device that starts from an empty list (new) and imports the adder's updates converges too.
#[test]
fn criterion_22_fresh_list_on_the_joiner_converges() {
    let a = DevicesList::new(1);
    a.set_label(A, "Desktop").unwrap();
    a.switch_workspace(W, "ZQX-WS-7731", true).unwrap();
    let b = DevicesList::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    b.set_label(B, "Laptop").unwrap();
    sync(&a, &b);
    assert_eq!(a.devices(), b.devices());
    assert_eq!(a.workspaces(), b.workspaces());
    assert_eq!(a.label(&B).as_deref(), Some("Laptop"));
    assert_eq!(b.switched_on(), vec![W]);
}

/// Criterion 13: switching off on any device is seen by every device ("off for all devices"),
/// and on again too; concurrent switches converge to one value on every replica.
#[test]
fn criterion_13_switch_is_shared_by_all_devices() {
    let a = DevicesList::new(1);
    a.switch_workspace(W, "Box job", true).unwrap();
    let b = DevicesList::from_snapshot(2, &a.snapshot().unwrap()).unwrap();
    let c = DevicesList::from_snapshot(3, &a.snapshot().unwrap()).unwrap();
    b.switch_workspace(W, "Box job", false).unwrap();
    sync(&a, &b);
    sync(&b, &c);
    for list in [&a, &b, &c] {
        assert_eq!(list.switched_on().len(), 0);
        assert!(!list.workspace(&W).unwrap().on);
    }
    c.switch_workspace(W, "Box job", true).unwrap();
    sync(&c, &a);
    sync(&a, &b);
    for list in [&a, &b, &c] {
        assert_eq!(list.switched_on(), vec![W]);
    }
    // Concurrent: A off, B on, without seeing each other.
    a.switch_workspace(W, "Box job", false).unwrap();
    b.switch_workspace(W, "Box job", true).unwrap();
    sync(&a, &b);
    sync(&b, &c);
    sync(&a, &c);
    let on = a.workspace(&W).unwrap().on;
    assert_eq!(b.workspace(&W).unwrap().on, on);
    assert_eq!(c.workspace(&W).unwrap().on, on);
}

/// Criterion 12: a workspace never switched on is absent from the list.
#[test]
fn criterion_12_only_switched_workspaces_are_listed() {
    let a = DevicesList::new(1);
    a.switch_workspace(W, "On", true).unwrap();
    assert!(a.workspace(&W2).is_none());
    assert_eq!(a.workspaces().len(), 1);
}

/// Criterion 23: a removed device disappears from every replica.
#[test]
fn criterion_23_removed_device_leaves_the_list() {
    let a = DevicesList::new(1);
    a.set_label(A, "A").unwrap();
    a.set_label(B, "B").unwrap();
    a.set_label(C, "C").unwrap();
    let b = DevicesList::from_snapshot(2, &a.snapshot().unwrap()).unwrap();
    a.remove_device(&B).unwrap();
    sync(&a, &b);
    assert!(b.device(&B).is_none());
    assert_eq!(b.devices().len(), 2);
}

/// A device needs a name: empty or white space is refused.
#[test]
fn criterion_2_empty_label_is_refused() {
    let a = DevicesList::new(1);
    assert_eq!(a.set_label(A, ""), Err(DevicesListError::EmptyLabel));
    assert_eq!(a.set_label(A, " \t "), Err(DevicesListError::EmptyLabel));
    assert!(a.device(&A).is_none());
}

/// Names with any Unicode survive; the browser mark is per device.
#[test]
fn criterion_33_browser_mark_and_unicode_names() {
    let a = DevicesList::new(1);
    a.set_label(A, "Werkstatt-PC äöü 🛠").unwrap();
    a.set_label(B, "Firefox on Linux").unwrap();
    a.mark_browser(B).unwrap();
    let b = DevicesList::from_snapshot(2, &a.snapshot().unwrap()).unwrap();
    assert!(b.device(&B).unwrap().browser);
    assert!(!b.device(&A).unwrap().browser);
    assert_eq!(b.label(&A).as_deref(), Some("Werkstatt-PC äöü 🛠"));
    // Renaming the browser keeps its mark in this build.
    b.set_label(B, "Firefox at home").unwrap();
    assert!(b.device(&B).unwrap().browser);
}

/// ADR 0040 decision 11: a list written by a newer build is refused, as a snapshot and as an
/// update.
#[test]
fn decision_11_newer_list_is_refused() {
    let newer = fixture("devices_list_v2.loro");
    assert_eq!(
        DevicesList::from_snapshot(1, &newer).err(),
        Some(DevicesListError::FormatTooNew)
    );
    let a = DevicesList::new(1);
    a.set_label(A, "A").unwrap();
    let result = a.import_updates(&newer);
    assert!(
        result.is_err(),
        "a newer list was merged into a v1 list: {result:?}"
    );
}

/// Damaged bytes are refused, never a panic; truncations of a good list too.
#[test]
fn decision_11_damaged_list_is_refused() {
    assert!(DevicesList::from_snapshot(1, &fixture("devices_list_truncated.loro")).is_err());
    assert!(DevicesList::from_snapshot(1, &[]).is_err());
    let good = fixture("devices_list_v1.loro");
    let read = DevicesList::from_snapshot(1, &good).unwrap();
    assert_ne!(read.devices().len(), 0);
    for len in [1, 2, 8, good.len() / 3, good.len() / 2, good.len() - 1] {
        let _ = DevicesList::from_snapshot(1, &good[..len]);
        let fresh = DevicesList::new(5);
        let _ = fresh.import_updates(&good[..len]);
    }
    let mut seed: u32 = 0x1234_5678;
    for _ in 0..200 {
        let mut bytes = good.clone();
        for _ in 0..4 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let i = (seed as usize) % bytes.len();
            bytes[i] ^= u8::try_from(seed >> 24).unwrap() | 1;
        }
        // Must not panic; the result may be either.
        let _ = DevicesList::from_snapshot(1, &bytes);
    }
}

/// Entries of other shapes hide only themselves.
#[test]
fn decision_11_odd_entries_hide_only_themselves() {
    let list = DevicesList::from_snapshot(1, &fixture("devices_list_odd_entries.loro")).unwrap();
    for device in list.devices() {
        assert_ne!(device.label.trim().len(), 0);
    }
}

/// v34/v35 compatibility (criterion 35): the share members of older shared files are read
/// without opening them, and the files still open.
#[test]
fn criterion_35_v34_v35_files_share_members() {
    let shared = fixture("shared_v34.curvyo");
    let members = container_share_members(&shared);
    assert!(members.share_json.is_some(), "{members:?}");
    unpack(1, &shared).unwrap();

    let ws = fixture("workspace_shared_v35.curvyo");
    let members = container_share_members(&ws);
    assert!(members.share_json.is_some(), "{members:?}");
    unpack(1, &ws).unwrap();

    let plain = fixture("laser_workbench_v35.curvyo");
    let members = container_share_members(&plain);
    assert!(members.share_json.is_none());
    unpack(1, &plain).unwrap();

    // Not a container at all.
    assert_eq!(container_share_members(b"not a zip").share_json, None);
    assert_eq!(container_share_members(&[]).share_json, None);
    let truncated = &shared[..shared.len() / 2];
    let _ = container_share_members(truncated);
}

fn created(n: u8) -> curvyo_document_core::Created {
    curvyo_document_core::Created {
        at: 1_800_000_000,
        id: [n; 16],
    }
}

/// Criterion 15 (this build's side): a name some desktop OS refuses is refused at creation,
/// so only names written elsewhere need the safe replacement on arrival.
#[test]
fn criterion_15_refused_names_never_enter_the_list() {
    use curvyo_document_core::{ListError, WorkspaceList};
    let list = WorkspaceList::new(1);
    for (i, name) in ["a:b", "a?b", "CON", "x. ", "tab\there", "a/b", ""]
        .into_iter()
        .enumerate()
    {
        let n = u8::try_from(i).unwrap();
        let result = list.create_document(None, name, [n + 1; 16], created(n));
        assert!(
            matches!(result, Err(ListError::Name(_))),
            "{name:?}: {result:?}"
        );
    }
    assert_eq!(list.entries().len(), 0);
    let node = list
        .create_document(None, "ZQX-DOC-7731", [0x55; 16], created(9))
        .unwrap();
    assert_eq!(list.stored_name(node).as_deref(), Some("ZQX-DOC-7731"));
    assert_eq!(list.find_document([0x55; 16]), Some(node));
}
