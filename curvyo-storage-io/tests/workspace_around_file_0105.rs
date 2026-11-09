//! Make a workspace for this file (`specs/0105-sharing-concept` criterion 8, ADR 0044 decision
//! 6): a fresh folder next to the file, named after it (" 2", " 3" on a clash), the file moved
//! in; nothing else moves.

#![allow(clippy::unwrap_used)]

use curvyo_storage_io::{make_workspace_around, workspace_folder_for};

#[test]
fn the_folder_is_named_after_the_file_and_the_file_moves_in() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path();
    let file = parent.join("Lid.curvyo");
    std::fs::write(&file, b"lid").unwrap();
    std::fs::write(parent.join("Base.curvyo"), b"base").unwrap();
    assert_eq!(workspace_folder_for(&file).unwrap(), parent.join("Lid"));
    let root = make_workspace_around(&file).unwrap();
    assert_eq!(root, parent.join("Lid"));
    assert_eq!(std::fs::read(root.join("Lid.curvyo")).unwrap(), b"lid");
    assert!(!file.exists());
    assert!(parent.join("Base.curvyo").exists(), "its neighbours stay");
}

#[test]
fn a_taken_name_gets_a_number_and_never_the_parent() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path();
    let file = parent.join("Lid.curvyo");
    std::fs::write(&file, b"lid").unwrap();
    std::fs::create_dir(parent.join("Lid")).unwrap();
    std::fs::write(parent.join("Lid 2"), b"a file").unwrap();
    assert_eq!(workspace_folder_for(&file).unwrap(), parent.join("Lid 3"));
    let root = make_workspace_around(&file).unwrap();
    assert_eq!(root, parent.join("Lid 3"));
    assert_ne!(root, parent);
    assert!(root.join("Lid.curvyo").exists());
}

#[test]
fn a_missing_file_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Gone.curvyo");
    assert!(make_workspace_around(&file).is_err());
    assert!(
        !dir.path().join("Gone").exists(),
        "no folder is left behind"
    );
}
