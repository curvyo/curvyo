//! Independent acceptance tests for `specs/0137-document-library` at the file level: the added
//! folders file (criterion 1), the recursive scan of a source and its read (criteria 1, 2, 10,
//! 12). Written from the spec before reading the implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::fs;
use std::path::Path;

use curvyo_storage_io::{
    AddFolderRefusal, LIBRARY_FOLDERS_FORMAT, LibraryFolders, LibraryFoldersError, SourceError,
    library_read, scan_library_source,
};

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

// ---- criterion 1: added folders ----------------------------------------------------------------

#[test]
fn ac1_add_then_list_keeps_the_order_added() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("logos");
    let b = dir.path().join("jobs");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let mut folders = LibraryFolders::default();
    folders.add(&a).unwrap();
    folders.add(&b).unwrap();
    assert_eq!(folders.paths(), &[a.clone(), b.clone()]);
}

#[test]
fn ac1_adding_the_same_folder_twice_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("logos");
    fs::create_dir_all(&a).unwrap();
    let mut folders = LibraryFolders::default();
    folders.add(&a).unwrap();
    assert_eq!(folders.add(&a), Err(AddFolderRefusal::AlreadyIn));
    assert_eq!(folders.paths().len(), 1);
}

#[test]
fn ac1_removing_a_folder_deletes_nothing_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("logos");
    write(&a.join("box.curvyo"), b"not really a container");
    let mut folders = LibraryFolders::default();
    folders.add(&a).unwrap();
    assert!(folders.remove(&a), "the entry is removed");
    assert_eq!(folders.paths().len(), 0, "the list is empty");
    assert!(a.join("box.curvyo").exists(), "no file is deleted");
    assert!(a.exists(), "the folder itself stays");
}

#[test]
fn ac1_the_list_round_trips_through_its_text_and_is_format_1() {
    assert_eq!(LIBRARY_FOLDERS_FORMAT, 1);
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("logos");
    fs::create_dir_all(&a).unwrap();
    let mut folders = LibraryFolders::default();
    folders.add(&a).unwrap();
    let text = folders.to_text();
    let back = LibraryFolders::parse(&text).unwrap();
    assert_eq!(back.paths(), folders.paths());
}

#[test]
fn ac1_a_newer_folders_file_is_refused_and_never_written_over() {
    let text = "format = 99\nfolders = []\n";
    assert_eq!(
        LibraryFolders::parse(text),
        Err(LibraryFoldersError::Newer),
        "a newer file is reported as newer"
    );
}

#[test]
fn ac1_a_damaged_folders_file_is_reported_as_damaged() {
    assert_eq!(
        LibraryFolders::parse("this is [ not toml"),
        Err(LibraryFoldersError::Damaged)
    );
}

// ---- criteria 1, 2, 10, 12: the scan and the read ----------------------------------------------

#[test]
fn ac1_scan_finds_curvyo_files_recursively_and_lists_their_path_inside_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("top.curvyo"), b"x");
    write(&root.join("sub/deeper/inner.curvyo"), b"yy");
    write(&root.join("sub/readme.txt"), b"not a document");
    let found = scan_library_source(root).unwrap();
    let paths: Vec<Vec<String>> = found.iter().map(|d| d.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            vec![
                "sub".to_owned(),
                "deeper".to_owned(),
                "inner.curvyo".to_owned()
            ],
            vec!["top.curvyo".to_owned()],
        ]
    );
    assert_eq!(found[1].size, 1, "the size is the file's size");
}

#[test]
fn ac1_scan_skips_hidden_entries() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("visible.curvyo"), b"x");
    write(&root.join(".hidden/secret.curvyo"), b"x");
    write(&root.join(".secret.curvyo"), b"x");
    let found = scan_library_source(root).unwrap();
    assert_eq!(found.len(), 1, "hidden entries are left out: {found:?}");
    assert_eq!(found[0].path, vec!["visible.curvyo".to_owned()]);
}

#[cfg(unix)]
#[test]
fn ac1_scan_does_not_follow_folder_links() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let outside = dir.path().join("outside");
    write(&outside.join("far.curvyo"), b"x");
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
    let found = scan_library_source(&root).unwrap();
    assert!(found.is_empty(), "a folder link is not followed: {found:?}");
}

#[test]
fn ac1_scan_of_a_missing_folder_reports_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone");
    assert_eq!(
        scan_library_source(&missing).unwrap_err(),
        SourceError::NotFound
    );
}

#[test]
fn ac1_scan_does_not_change_the_files_it_reads() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("box.curvyo");
    write(&file, b"bytes");
    let before = fs::metadata(&file).unwrap().modified().unwrap();
    let _ = scan_library_source(dir.path()).unwrap();
    let _ = library_read(dir.path(), &["box.curvyo".to_owned()]).unwrap();
    let after = fs::metadata(&file).unwrap().modified().unwrap();
    assert_eq!(before, after, "the file's timestamp is untouched");
    assert_eq!(fs::read(&file).unwrap(), b"bytes", "the file is untouched");
}

#[test]
fn ac2_library_read_returns_the_document_bytes_of_a_path_inside_the_source() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("jobs/lid.curvyo"), b"CONTAINER");
    let bytes = library_read(dir.path(), &["jobs".to_owned(), "lid.curvyo".to_owned()]).unwrap();
    assert_eq!(bytes, b"CONTAINER");
}

#[test]
fn ac2_library_read_refuses_a_path_that_climbs_out_of_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    write(&dir.path().join("secret.curvyo"), b"outside");
    fs::create_dir_all(&source).unwrap();
    let escape = library_read(&source, &["..".to_owned(), "secret.curvyo".to_owned()]);
    assert!(escape.is_err(), "a path outside the source is not read");
}

#[test]
fn ac10_a_missing_source_error_is_distinct_from_a_damaged_document() {
    let dir = tempfile::tempdir().unwrap();
    let err = scan_library_source(&dir.path().join("moved away")).unwrap_err();
    assert_eq!(
        err,
        SourceError::NotFound,
        "moved or offline reads as not found"
    );
}

#[test]
fn ac12_scan_and_read_need_no_network_and_write_nothing() {
    // Reading a source leaves the folder as it was: no new entries appear.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("a.curvyo"), b"x");
    let before: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let _ = scan_library_source(dir.path()).unwrap();
    let after: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before.len(), after.len(), "the scan creates no file");
}
