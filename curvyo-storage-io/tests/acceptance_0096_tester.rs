//! Tester acceptance tests for `specs/0096-own-devices-sync` in `storage-io`, written from the
//! specification and ADR 0040 decision 2 before the implementation was read: `group.json` v1 and
//! its refusals, the device group files (no key in them, criterion 31), the Synced workspaces
//! folder (criterion 2) and arrival folder names (criteria 10 and 15).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::collections::BTreeMap;
use std::path::Path;

use curvyo_storage_io::{
    DEVICES_DIR, DeviceGroupFiles, DevicesError, GROUP_FILE, GroupRecord, SyncedFolderError,
    arrival_folder, prepare_synced_folder, read_device_group, remove_device_group,
    safe_folder_name, write_device_group,
};

fn record() -> GroupRecord {
    let mut folders = BTreeMap::new();
    folders.insert([0x77; 16], "/home/maker/Curvyo/ZQX-WS-7731".to_owned());
    GroupRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [0x6e; 16],
        server_seq: 42,
        synced_version: "v1:abc".to_owned(),
        synced_folder: "/home/maker/Curvyo".to_owned(),
        folders,
    }
}

fn files() -> DeviceGroupFiles {
    DeviceGroupFiles {
        group: record(),
        keyring_log: vec![1, 2, 3, 4],
        list: vec![9, 8, 7],
    }
}

fn json_with(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(&record().to_bytes()).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

/// `group.json` round-trips.
#[test]
fn decision_2_group_record_round_trips() {
    let bytes = record().to_bytes();
    assert_eq!(GroupRecord::from_bytes(&bytes).unwrap(), record());
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["format_version"], 1);
}

/// A newer `group.json` is refused with its version, never read as v1.
#[test]
fn decision_2_newer_group_record_is_refused() {
    let bytes = json_with(|v| v["format_version"] = 2.into());
    assert!(matches!(
        GroupRecord::from_bytes(&bytes),
        Err(DevicesError::TooNew { found: 2 })
    ));
    let bytes = json_with(|v| v["format_version"] = u64::from(u32::MAX).into());
    assert!(matches!(
        GroupRecord::from_bytes(&bytes),
        Err(DevicesError::TooNew { .. })
    ));
}

/// Damaged `group.json` files are refused as damaged, never a panic.
#[test]
fn decision_2_damaged_group_record_is_refused() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"{".to_vec(),
        b"null".to_vec(),
        b"[]".to_vec(),
        json_with(|v| v["room"] = "zz".into()),
        json_with(|v| v["room"] = "6e6e".into()),
        json_with(|v| {
            v.as_object_mut().unwrap().remove("room");
        }),
        json_with(|v| {
            v.as_object_mut().unwrap().remove("format_version");
        }),
        json_with(|v| v["server_seq"] = (-1).into()),
        json_with(|v| v["folders"] = serde_json::json!({"not hex": "/x"})),
        json_with(|v| v["format_version"] = 0.into()),
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let result = GroupRecord::from_bytes(bytes);
        assert!(result.is_err(), "case {i} accepted: {result:?}");
    }
}

/// Criterion 31: the group files hold no key material; files are written under `devices/`,
/// read back, and removed with Leave.
#[test]
fn criterion_31_device_group_files_round_trip_without_keys() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_device_group(dir.path()).unwrap().is_none());
    write_device_group(dir.path(), &files()).unwrap();
    let back = read_device_group(dir.path()).unwrap().unwrap();
    assert_eq!(back, files());
    let group_json = std::fs::read(dir.path().join(DEVICES_DIR).join(GROUP_FILE)).unwrap();
    let text = String::from_utf8(group_json).unwrap();
    for word in ["key", "secret", "private", "entropy"] {
        assert!(
            !text.to_lowercase().contains(word),
            "group.json names {word}: {text}"
        );
    }
    // No temp files left behind.
    for entry in std::fs::read_dir(dir.path().join(DEVICES_DIR)).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        assert!(!name.contains("tmp") && !name.starts_with('.'), "{name}");
    }
    remove_device_group(dir.path()).unwrap();
    assert!(read_device_group(dir.path()).unwrap().is_none());
    // Removing twice is fine.
    remove_device_group(dir.path()).unwrap();
}

/// A damaged or newer `group.json` on disk is reported, never treated as "no group" (which would
/// let the device pair into a second group or forget its pairing).
#[test]
fn decision_2_damaged_group_on_disk_is_not_absent() {
    let dir = tempfile::tempdir().unwrap();
    write_device_group(dir.path(), &files()).unwrap();
    let path = dir.path().join(DEVICES_DIR).join(GROUP_FILE);
    std::fs::write(&path, b"{ damaged").unwrap();
    assert!(read_device_group(dir.path()).is_err());
    std::fs::write(&path, json_with(|v| v["format_version"] = 7.into())).unwrap();
    assert!(matches!(
        read_device_group(dir.path()),
        Err(DevicesError::TooNew { found: 7 })
    ));
}

/// A `devices/` directory with `group.json` but without its other files is reported, not
/// read as a group with nothing in it, and not as absent.
#[test]
fn decision_2_partial_devices_dir_is_handled() {
    let dir = tempfile::tempdir().unwrap();
    write_device_group(dir.path(), &files()).unwrap();
    let devices = dir.path().join(DEVICES_DIR);
    for entry in std::fs::read_dir(&devices).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != GROUP_FILE {
            std::fs::remove_file(entry.path()).unwrap();
        }
    }
    // Either an error or a group: never a panic, and never silently "no group".
    let result = read_device_group(dir.path());
    assert!(
        !matches!(result, Ok(None)),
        "a group.json without its keyring reads as no group"
    );
}

/// Criterion 2: the Synced workspaces folder is created when absent, never inside a workspace.
#[test]
fn criterion_2_synced_folder_rules() {
    let dir = tempfile::tempdir().unwrap();
    let fresh = dir.path().join("Curvyo");
    prepare_synced_folder(&fresh).unwrap();
    assert!(fresh.is_dir());
    // Again: still fine.
    prepare_synced_folder(&fresh).unwrap();

    let ws = dir.path().join("W");
    std::fs::create_dir_all(ws.join(".curvyo-workspace")).unwrap();
    assert_eq!(
        prepare_synced_folder(&ws),
        Err(SyncedFolderError::InsideWorkspace)
    );
    assert_eq!(
        prepare_synced_folder(&ws.join("sub").join("deeper")),
        Err(SyncedFolderError::InsideWorkspace)
    );
    assert!(
        !ws.join("sub").exists(),
        "a refused folder was created inside a workspace"
    );

    let file = dir.path().join("a-file");
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(
        prepare_synced_folder(&file),
        Err(SyncedFolderError::NotWritable)
    );
}

/// Criterion 2: a folder that contains a workspace is fine (it is where workspaces arrive).
#[test]
fn criterion_2_synced_folder_may_hold_workspaces() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("W").join(".curvyo-workspace")).unwrap();
    prepare_synced_folder(dir.path()).unwrap();
}

fn touch_workspace(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(path.join("a.curvyo"), b"x").unwrap();
}

/// Criterion 10: a name that exists gets " 2", then " 3"; an empty folder is reused.
#[test]
fn criterion_10_arrival_name_clashes() {
    let dir = tempfile::tempdir().unwrap();
    let synced = dir.path();
    let first = arrival_folder(synced, "Box job");
    assert_eq!(first, synced.join("Box job"));
    touch_workspace(&first);
    let second = arrival_folder(synced, "Box job");
    assert_eq!(second, synced.join("Box job 2"));
    touch_workspace(&second);
    assert_eq!(arrival_folder(synced, "Box job"), synced.join("Box job 3"));
    // An empty folder left by a try that did not get in is reused.
    std::fs::create_dir_all(synced.join("Empty")).unwrap();
    assert_eq!(arrival_folder(synced, "Empty"), synced.join("Empty"));
    // A plain file of that name is taken.
    std::fs::write(synced.join("File"), b"x").unwrap();
    assert_ne!(arrival_folder(synced, "File"), synced.join("File"));
}

/// Criterion 15: names other systems refuse are made safe and stay inside the synced folder.
#[test]
fn criterion_15_unsafe_names_are_made_safe() {
    let dir = tempfile::tempdir().unwrap();
    let synced = dir.path();
    for name in [
        "a:b",
        "a/b",
        "a\\b",
        "..",
        ".",
        "../escape",
        "/etc",
        "CON",
        "con",
        "LPT1",
        "NUL.txt",
        "trailing. ",
        "q?*<>|\"",
        "tab\tnew\nline",
        "nul\0byte",
        "",
        "   ",
    ] {
        let safe = safe_folder_name(name);
        assert!(!safe.is_empty(), "{name:?} -> empty");
        for c in ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'] {
            assert!(!safe.contains(c), "{name:?} -> {safe:?}");
        }
        assert!(
            safe.chars().all(|c| !c.is_control()),
            "{name:?} -> {safe:?}"
        );
        assert!(
            !safe.ends_with('.') && !safe.ends_with(' '),
            "{name:?} -> {safe:?}"
        );
        assert_ne!(safe, "..");
        assert_ne!(safe, ".");
        let stem = safe.split('.').next().unwrap().to_ascii_uppercase();
        assert!(
            !["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"].contains(&stem.as_str()),
            "{name:?} -> {safe:?}"
        );
        let arrival = arrival_folder(synced, name);
        assert_eq!(
            arrival.parent(),
            Some(synced),
            "{name:?} escapes: {arrival:?}"
        );
    }
    assert_eq!(safe_folder_name("ZQX-WS-7731 Box"), "ZQX-WS-7731 Box");
    assert_eq!(safe_folder_name("Werkstück äöü"), "Werkstück äöü");
    assert_eq!(safe_folder_name("???"), safe_folder_name("???"));
}

/// Two different unsafe names that become the same safe name still arrive in two folders.
#[test]
fn criterion_15_two_names_one_safe_name_do_not_merge() {
    let dir = tempfile::tempdir().unwrap();
    let synced = dir.path();
    let a = arrival_folder(synced, "a:b");
    touch_workspace(&a);
    let b = arrival_folder(synced, "a?b");
    assert_ne!(a, b);
}

/// A name valid on Windows and macOS (255 UTF-16 units) but over Linux's 255 bytes, and a
/// 254-byte name that clashes and gets " 2", still give a folder that can be created.
#[test]
fn criterion_15_long_names_fit() {
    let dir = tempfile::tempdir().unwrap();
    // 200 CJK characters: 200 UTF-16 units (fine on Windows and macOS), 600 bytes in UTF-8.
    let cross_os = "工".repeat(200);
    let arrival = arrival_folder(dir.path(), &cross_os);
    let component = arrival.file_name().unwrap().to_str().unwrap();
    assert!(component.len() <= 255, "{} bytes", component.len());
    std::fs::create_dir_all(&arrival).unwrap();
    // A 254-byte name that exists already gets " 2": 256 bytes.
    let name = "x".repeat(254);
    touch_workspace(&dir.path().join(&name));
    let second = arrival_folder(dir.path(), &name);
    let component = second.file_name().unwrap().to_str().unwrap();
    assert!(component.len() <= 255, "{} bytes", component.len());
    std::fs::create_dir_all(&second).unwrap();
}
