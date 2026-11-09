//! Tester acceptance tests for `specs/0105-sharing-concept` on disk, written from the
//! specification before the implementation was read: legacy shared files open without a write
//! (criterion 4, `open_writes_nothing`), the workspace marker's old share files read as before
//! (criterion 6), and Make a workspace for this file names and makes its folder (criterion 8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use curvyo_document_core::{WorkspaceRoomRecord, pack, unpack};
use curvyo_storage_io::{
    MARKER_DIR, make_workspace_around, read_marker_share, read_to_vec, scan_workspace,
    workspace_folder_for, write_atomic,
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../curvyo-document-core/tests/fixtures")
}

fn listing(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = walk(dir)
        .into_iter()
        .map(|p| p.strip_prefix(dir).unwrap().display().to_string())
        .collect();
    out.sort();
    out
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        out.push(path.clone());
        if entry.file_type().unwrap().is_dir() {
            out.extend(walk(&path));
        }
    }
    out
}

/// Criterion 4: each legacy golden opened from disk leaves the file byte-identical and the
/// folder unchanged (no sibling written).
#[test]
fn c4_open_writes_nothing_for_the_legacy_goldens() {
    for name in [
        "shared_v34.curvyo",
        "ended_v36.curvyo",
        "workspace_shared_v35.curvyo",
    ] {
        let original = read_to_vec(&fixtures().join(name)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        write_atomic(&path, &original).unwrap();
        let before = listing(dir.path());
        let document = unpack(2, &read_to_vec(&path).unwrap()).unwrap();
        let _ = document.export_json().unwrap();
        assert_eq!(
            read_to_vec(&path).unwrap(),
            original,
            "{name}: bytes changed"
        );
        assert_eq!(listing(dir.path()), before, "{name}: folder changed");
    }
}

/// Criterion 5 on disk: a legacy file saved over itself holds no legacy member and opens local
/// without the flag.
#[test]
fn c5_a_legacy_file_saved_to_disk_drops_its_members() {
    for name in ["shared_v34.curvyo", "ended_v36.curvyo"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        write_atomic(&path, &read_to_vec(&fixtures().join(name)).unwrap()).unwrap();
        let document = unpack(3, &read_to_vec(&path).unwrap()).unwrap();
        assert!(document.legacy_share(), "{name}");
        write_atomic(&path, &pack(&document, "tester").unwrap()).unwrap();
        let saved = read_to_vec(&path).unwrap();
        let archive = zip::ZipArchive::new(std::io::Cursor::new(&saved[..])).unwrap();
        let names: Vec<String> = archive
            .file_names()
            .map(|n| n.unwrap().into_owned())
            .collect();
        for legacy in ["share.json", "keyring.log", "share.ended.json"] {
            assert!(!names.iter().any(|n| n == legacy), "{name}: {names:?}");
        }
        assert!(!unpack(4, &saved).unwrap().legacy_share());
    }
}

/// Criterion 6: a workspace marker with the old share files (`share.json` v1 shape, keyring,
/// list) and its golden reads as before.
#[test]
fn c6_the_marker_share_files_read_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let marker = root.join(MARKER_DIR);
    std::fs::create_dir_all(&marker).unwrap();
    let share = read_to_vec(&fixtures().join("workspace_marker_share_v1.json")).unwrap();
    std::fs::write(marker.join("share.json"), &share).unwrap();
    std::fs::write(marker.join("keyring.log"), b"keyring").unwrap();
    std::fs::write(marker.join("list.loro"), b"list").unwrap();
    let read = read_marker_share(root).unwrap().expect("shared");
    assert_eq!(read.share_json, share);
    assert_eq!(read.keyring_log, b"keyring");
    let record = WorkspaceRoomRecord::from_bytes(&read.share_json).unwrap();
    assert_eq!(record.workspace, [0x57; 16]);
    // A legacy document lying in the workspace is still scanned as a document.
    std::fs::copy(
        fixtures().join("shared_v34.curvyo"),
        root.join("Old.curvyo"),
    )
    .unwrap();
    assert!(
        scan_workspace(root)
            .unwrap()
            .iter()
            .any(|e| e.path == vec!["Old.curvyo".to_owned()])
    );
}

/// Criterion 8: the folder is `<parent>/<name>/`, " 2", " 3" on a clash with a folder or a file
/// of that name, and never the file's parent, even when the parent has the file's name.
#[test]
fn c8_the_folder_name_and_its_clashes() {
    let dir = tempfile::tempdir().unwrap();
    let lid_dir = dir.path().join("Lid");
    std::fs::create_dir(&lid_dir).unwrap();
    let file = lid_dir.join("Lid.curvyo");
    std::fs::write(&file, b"lid").unwrap();
    let folder = workspace_folder_for(&file).unwrap();
    assert_eq!(folder, lid_dir.join("Lid"));
    assert_ne!(folder, lid_dir, "never the current parent");
    std::fs::create_dir(lid_dir.join("Lid")).unwrap();
    std::fs::create_dir(lid_dir.join("Lid 2")).unwrap();
    assert_eq!(workspace_folder_for(&file).unwrap(), lid_dir.join("Lid 3"));
    let made = make_workspace_around(&file).unwrap();
    assert_eq!(made, lid_dir.join("Lid 3"));
    assert_eq!(std::fs::read(made.join("Lid.curvyo")).unwrap(), b"lid");
    assert!(!file.exists());
    assert!(
        std::fs::read_dir(lid_dir.join("Lid"))
            .unwrap()
            .next()
            .is_none(),
        "existing folders untouched"
    );
}

/// Criterion 8 with names that are not simple: dots, spaces, non-ASCII.
#[test]
fn c8_names_with_dots_spaces_and_umlauts() {
    let dir = tempfile::tempdir().unwrap();
    for (file, folder) in [
        ("Box.v2.curvyo", "Box.v2"),
        ("Grüße an Lena.curvyo", "Grüße an Lena"),
        ("箱.curvyo", "箱"),
    ] {
        let path = dir.path().join(file);
        std::fs::write(&path, file.as_bytes()).unwrap();
        let made = make_workspace_around(&path).unwrap();
        assert_eq!(made, dir.path().join(folder), "{file}");
        assert_eq!(std::fs::read(made.join(file)).unwrap(), file.as_bytes());
    }
}

/// Criterion 8, Cancel or a refusal changes nothing: a missing file, a folder given as file,
/// and a parent that cannot be written leave the disk as it was.
#[test]
fn c8_refusals_change_nothing_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Folder.curvyo")).unwrap();
    let before = listing(dir.path());
    assert!(make_workspace_around(&dir.path().join("Gone.curvyo")).is_err());
    assert!(make_workspace_around(&dir.path().join("Folder.curvyo")).is_err());
    assert_eq!(listing(dir.path()), before);
}

/// Criterion 8: a parent that cannot be written refuses and leaves the file where it was.
#[cfg(unix)]
#[test]
fn c8_a_read_only_parent_leaves_the_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("ro");
    std::fs::create_dir(&parent).unwrap();
    let file = parent.join("Lid.curvyo");
    std::fs::write(&file, b"lid").unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o555)).unwrap();
    let result = make_workspace_around(&file);
    let still = std::fs::read(&file);
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Root may write anyway; only check where the permission holds.
    if result.is_err() {
        assert_eq!(still.unwrap(), b"lid");
        assert!(!parent.join("Lid").exists());
    }
}

/// Criterion 8: after the move the new folder is a workspace with exactly the one document.
#[test]
fn c8_the_new_folder_scans_as_a_workspace_of_one_document() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Lid.curvyo");
    std::fs::write(&file, b"lid").unwrap();
    std::fs::write(dir.path().join("Base.curvyo"), b"base").unwrap();
    let root = make_workspace_around(&file).unwrap();
    let id = curvyo_storage_io::open_workspace(&root, [7; 16]).unwrap();
    assert_eq!(id, [7; 16]);
    let docs: Vec<_> = scan_workspace(&root)
        .unwrap()
        .into_iter()
        .map(|e| e.path)
        .collect();
    assert_eq!(docs, vec![vec!["Lid.curvyo".to_owned()]]);
    assert!(dir.path().join("Base.curvyo").exists(), "neighbours stay");
}
