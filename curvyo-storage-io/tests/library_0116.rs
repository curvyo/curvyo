//! The library list's file beside the device group's (`specs/0116-library-sync` criteria 2, 21
//! and 22, ADR 0062 decision 1): `library.loro` read and written on its own, and forgotten with
//! `devices/`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use curvyo_storage_io::{
    DEVICES_DIR, DeviceGroupFiles, GroupRecord, LIBRARY_LIST_FILE, read_library_list,
    remove_device_group, write_device_group, write_library_list,
};

fn record() -> GroupRecord {
    GroupRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [0x6e; 16],
        server_seq: 42,
        synced_version: "v1:abc".to_owned(),
        synced_folder: "/home/maker/Curvyo".to_owned(),
        folders: BTreeMap::new(),
    }
}

/// Criterion 2: the file sits in `devices/` beside the devices list, reads back, and a group
/// without one (pre-0116) reads as `None`.
#[test]
fn the_library_file_is_written_and_read_beside_the_group() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_library_list(dir.path()).unwrap(), None);
    write_library_list(dir.path(), &[1, 2, 3]).unwrap();
    assert!(
        dir.path()
            .join(DEVICES_DIR)
            .join(LIBRARY_LIST_FILE)
            .is_file()
    );
    assert_eq!(read_library_list(dir.path()).unwrap(), Some(vec![1, 2, 3]));
    write_device_group(
        dir.path(),
        &DeviceGroupFiles {
            group: record(),
            keyring_log: vec![1],
            list: vec![2],
        },
    )
    .unwrap();
    assert_eq!(read_library_list(dir.path()).unwrap(), Some(vec![1, 2, 3]));
}

/// Criteria 21 and 22: removing the device group forgets `library.loro` with `devices/`.
#[test]
fn removing_the_group_forgets_the_library_file() {
    let dir = tempfile::tempdir().unwrap();
    write_library_list(dir.path(), &[9]).unwrap();
    remove_device_group(dir.path()).unwrap();
    assert_eq!(read_library_list(dir.path()).unwrap(), None);
    assert!(!dir.path().join(DEVICES_DIR).exists());
    remove_device_group(dir.path()).unwrap();
}

/// The library file alone does not make a group: `group.json` stays the commit point (plan P5).
#[test]
fn a_library_file_without_a_group_is_no_group() {
    let dir = tempfile::tempdir().unwrap();
    write_library_list(dir.path(), &[7]).unwrap();
    assert_eq!(
        curvyo_storage_io::read_device_group(dir.path()).unwrap(),
        None
    );
}
