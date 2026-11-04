//! The device group's files (`specs/0096-own-devices-sync` criteria 2, 10, 15 and 31, ADR 0040
//! decision 2): `group.json` v1 and its golden file, the three files written and read back,
//! removal, the Synced workspaces folder and the folder a synced workspace arrives in.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use curvyo_storage_io::{
    DEVICES_DIR, DeviceGroupFiles, DevicesError, GroupRecord, SyncedFolderError, arrival_folder,
    prepare_synced_folder, read_device_group, remove_device_group, safe_folder_name,
    write_device_group,
};

fn record() -> GroupRecord {
    GroupRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [0x6e; 16],
        server_seq: 42,
        synced_version: "v1:abc".to_owned(),
        synced_folder: "/home/maker/Curvyo".to_owned(),
        folders: BTreeMap::from([([0x77; 16], "/home/maker/Curvyo/Box job".to_owned())]),
    }
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/group_v1.json")
}

/// `group.json` v1 is stable and reads back.
#[test]
fn group_json_v1_matches_its_golden_file() {
    let bytes = record().to_bytes();
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(fixture(), &bytes).unwrap();
    }
    assert_eq!(
        std::fs::read(fixture()).unwrap(),
        bytes,
        "group_v1.json differs"
    );
    assert_eq!(GroupRecord::from_bytes(&bytes).unwrap(), record());
}

/// A newer version is refused as too new; a damaged file as damaged.
#[test]
fn newer_and_damaged_records_are_refused() {
    let newer = String::from_utf8(record().to_bytes())
        .unwrap()
        .replace("\"format_version\": 1", "\"format_version\": 2");
    assert!(matches!(
        GroupRecord::from_bytes(newer.as_bytes()),
        Err(DevicesError::TooNew { found: 2 })
    ));
    let bad_room = String::from_utf8(record().to_bytes())
        .unwrap()
        .replace(&"6e".repeat(16), "zz");
    assert!(matches!(
        GroupRecord::from_bytes(bad_room.as_bytes()),
        Err(DevicesError::Damaged)
    ));
    assert!(matches!(
        GroupRecord::from_bytes(b"{"),
        Err(DevicesError::Damaged)
    ));
}

/// Criterion 31: the files go to `devices/` in the app data directory, read back, and Leave
/// removes them; no group reads as none.
#[test]
fn the_group_files_are_written_read_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_device_group(dir.path()).unwrap(), None);
    let files = DeviceGroupFiles {
        group: record(),
        keyring_log: vec![1, 2, 3],
        list: vec![4, 5],
    };
    write_device_group(dir.path(), &files).unwrap();
    assert_eq!(read_device_group(dir.path()).unwrap(), Some(files.clone()));
    let names: Vec<String> = std::fs::read_dir(dir.path().join(DEVICES_DIR))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 3, "no temporary file stays: {names:?}");
    std::fs::remove_file(dir.path().join(DEVICES_DIR).join("devices.loro")).unwrap();
    assert!(matches!(
        read_device_group(dir.path()),
        Err(DevicesError::Damaged)
    ));
    remove_device_group(dir.path()).unwrap();
    assert_eq!(read_device_group(dir.path()).unwrap(), None);
    remove_device_group(dir.path()).unwrap();
}

/// Criterion 2: the Synced workspaces folder is created when absent and never lies in a
/// workspace.
#[test]
fn the_synced_folder_is_created_and_never_inside_a_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Curvyo");
    prepare_synced_folder(&folder).unwrap();
    assert!(folder.is_dir());
    let workspace = dir.path().join("Work");
    std::fs::create_dir_all(workspace.join(".curvyo-workspace")).unwrap();
    assert_eq!(
        prepare_synced_folder(&workspace.join("inner")),
        Err(SyncedFolderError::InsideWorkspace)
    );
    let file = dir.path().join("file");
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(
        prepare_synced_folder(&file),
        Err(SyncedFolderError::NotWritable)
    );
}

/// Criteria 10 and 15: a taken name gets " 2", then " 3"; names every OS refuses become safe.
#[test]
fn an_arriving_workspace_gets_a_free_safe_folder() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        arrival_folder(dir.path(), "Box job"),
        dir.path().join("Box job")
    );
    std::fs::create_dir(dir.path().join("Box job")).unwrap();
    // An empty folder left by an arrival that did not get in is used again.
    assert_eq!(
        arrival_folder(dir.path(), "Box job"),
        dir.path().join("Box job")
    );
    std::fs::write(dir.path().join("Box job").join("Lid.curvyo"), b"x").unwrap();
    assert_eq!(
        arrival_folder(dir.path(), "Box job"),
        dir.path().join("Box job 2")
    );
    std::fs::create_dir(dir.path().join("Box job 2")).unwrap();
    std::fs::write(dir.path().join("Box job 2").join("x"), b"x").unwrap();
    assert_eq!(
        arrival_folder(dir.path(), "Box job"),
        dir.path().join("Box job 3")
    );
    assert_eq!(safe_folder_name("a:b/c?"), "a-b-c-");
    assert_eq!(safe_folder_name("Lid. "), "Lid");
    assert_eq!(safe_folder_name("CON"), "_CON");
    assert_eq!(safe_folder_name("com1.txt"), "_com1.txt");
    assert_eq!(safe_folder_name("  "), "Workspace");
    // Cut to 244 bytes on a character boundary, then trimmed again.
    assert_eq!(safe_folder_name(&"ä".repeat(200)), "ä".repeat(122));
    let cut_at_a_space = format!("{} tail", "a".repeat(243));
    assert_eq!(safe_folder_name(&cut_at_a_space), "a".repeat(243));
}
