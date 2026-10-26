//! The archive reader against the golden fixtures of `specs/0083-font-info-and-zip` criterion 16
//! (built by `tools/zip-fixtures/make_fixtures.py`) and criteria 8, 9 and 15.
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

use curvyo_text_core::limits::MAX_ARCHIVE_BYTES;
use curvyo_text_core::{
    ArchiveError, EntryClass, EntrySkip, FontArchive, FontFileKind, UnsafeReason, listable_faces,
};

fn fixture(name: &str) -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/zip");
    std::fs::read(dir.join(name)).unwrap()
}

fn open(name: &str) -> FontArchive {
    FontArchive::open(fixture(name)).unwrap()
}

/// `(display name, file name, class)` of every entry.
fn table(archive: &FontArchive) -> Vec<(String, String, EntryClass)> {
    archive
        .entries()
        .iter()
        .map(|e| (e.name.clone(), e.file_name.clone(), e.class))
        .collect()
}

fn classes(archive: &FontArchive) -> Vec<EntryClass> {
    archive.entries().iter().map(|e| e.class).collect()
}

/// Criterion 8: fonts are candidates, documents kept, other files left out; the font reads back
/// as the shipped bytes and is listable.
#[test]
fn ac08_plain_archive() {
    let mut archive = open("plain.zip");
    let svg = EntryClass::Font(FontFileKind::Svg);
    assert_eq!(
        classes(&archive),
        [
            svg,
            EntryClass::Document,
            EntryClass::Document,
            EntryClass::LeftOut
        ]
    );
    let font = archive.read(0).unwrap();
    assert_eq!(listable_faces(&font).len(), 1);
    assert!(
        String::from_utf8(archive.read(1).unwrap())
            .unwrap()
            .contains("Hershey")
    );
    assert_eq!(archive.read(3), Err(EntrySkip::NotKept));
}

/// Criteria 8 and 9: folders nested to any depth are read, only the base names are used.
#[test]
fn ac09_nested_folders_flatten() {
    let archive = open("nested-folders.zip");
    let names: Vec<String> = archive
        .entries()
        .iter()
        .map(|e| e.file_name.clone())
        .collect();
    assert_eq!(names, ["HersheySans1.svg", "NOTICE.txt"]);
}

/// Criterion 8: `__MACOSX`, `._` and dot files are left out.
#[test]
fn ac08_macosx_and_dot_files_left_out() {
    let archive = open("macosx.zip");
    assert_eq!(
        classes(&archive),
        [
            EntryClass::Font(FontFileKind::Svg),
            EntryClass::LeftOut,
            EntryClass::LeftOut,
            EntryClass::LeftOut
        ]
    );
}

/// Criterion 9: two entries with the same base name become `OFL.txt` and `OFL-2.txt`.
#[test]
fn ac09_same_licence_name_twice() {
    let archive = open("same-ofl.zip");
    let names: Vec<String> = archive
        .entries()
        .iter()
        .map(|e| e.file_name.clone())
        .collect();
    assert_eq!(
        names,
        [
            "EMSAllure.svg",
            "OFL.txt",
            "EMSReadability.svg",
            "OFL-2.txt"
        ]
    );
}

/// Criterion 16: `..`, absolute, drive-letter and UNC names are unsafe; the rest is read.
#[test]
fn ac16_unsafe_names() {
    let path = EntryClass::Unsafe(UnsafeReason::Path);
    let svg = EntryClass::Font(FontFileKind::Svg);
    assert_eq!(classes(&open("dotdot.zip")), [path, path, svg]);
    assert_eq!(classes(&open("absolute.zip")), [path, path, path, svg]);
    let mut archive = open("dotdot.zip");
    assert_eq!(archive.read(0), Err(EntrySkip::NotKept));
}

/// Criterion 16: a symlink and another non-file type are unsafe.
#[test]
fn ac16_symlinks_and_special_files() {
    let not_file = EntryClass::Unsafe(UnsafeReason::NotAFile);
    assert_eq!(
        classes(&open("symlink.zip")),
        [not_file, not_file, EntryClass::Document]
    );
}

/// Criterion 16: archives inside the archive are never opened.
#[test]
fn ac16_nested_bomb_is_left_out() {
    let archive = open("nested-bomb.zip");
    assert_eq!(archive.entries().len(), 16);
    assert!(classes(&archive).iter().all(|c| *c == EntryClass::LeftOut));
}

/// Criterion 16: entries sharing their data make the archive unreadable.
#[test]
fn ac16_overlapping_entries() {
    assert_eq!(
        FontArchive::open(fixture("overlap-bomb.zip")).unwrap_err(),
        ArchiveError::Damaged
    );
}

/// Criterion 15: one entry over its own limit is skipped as unsafe, and reading it stops at the
/// limit.
#[test]
fn ac15_ratio_bomb_entry_is_unsafe() {
    let mut archive = open("ratio-bomb.zip");
    assert_eq!(archive.read(0), Err(EntrySkip::Unsafe));
}

/// Criterion 15: entries each under their limit but together over 256 MiB refuse the archive.
#[test]
fn ac15_total_over_limit() {
    let mut archive = open("total-bomb.zip");
    let results: Vec<_> = (0..5).map(|i| archive.read(i).map(|b| b.len())).collect();
    assert!(results[..4].iter().all(Result::is_ok));
    assert_eq!(results[4], Err(EntrySkip::ArchiveOverLimit));
}

/// Criterion 16: a wrong CRC-32 and data longer than declared are unsafe; the next entry is
/// still read (a `zip` update that drops the CRC check fails here, adrs.md decision 3).
#[test]
fn ac16_wrong_crc_and_short_size() {
    for name in ["wrong-crc.zip", "short-size.zip"] {
        let mut archive = open(name);
        assert_eq!(archive.read(0), Err(EntrySkip::Unsafe), "{name}");
        assert!(archive.read(1).is_ok(), "{name}");
    }
}

/// Criterion 16: an encrypted entry is left out.
#[test]
fn ac16_encrypted_entry() {
    assert_eq!(
        classes(&open("encrypted.zip")),
        [EntryClass::LeftOut, EntryClass::Document]
    );
}

/// Criterion 15: 10,001 entries refuse the archive before its table is built.
#[test]
fn ac15_too_many_entries() {
    assert_eq!(
        FontArchive::open(fixture("many-entries.zip")).unwrap_err(),
        ArchiveError::OverLimit
    );
}

/// Criterion 15: a file over 256 MiB is refused whole.
#[test]
fn ac15_archive_over_size() {
    assert_eq!(
        FontArchive::open(vec![0; MAX_ARCHIVE_BYTES + 1]).unwrap_err(),
        ArchiveError::OverLimit
    );
}

/// Criterion 16: a name flagged UTF-8 that is not shows `_`; an unflagged one is CP437.
#[test]
fn ac16_non_utf8_names() {
    let archive = open("non-utf8-name.zip");
    let names: Vec<(String, String, EntryClass)> = table(&archive);
    assert_eq!(names[0].0, "caf_-1.txt");
    assert_eq!(names[1].0, "cafΘ-2.txt");
    assert_eq!(names[1].1, "cafΘ-2.txt");
}

/// Criterion 16: a file that is not a zip "could not be read"; an archive with no font has only
/// a document and a left-out file.
#[test]
fn ac16_not_a_zip_and_no_font() {
    assert_eq!(
        FontArchive::open(fixture("not-a-zip.zip")).unwrap_err(),
        ArchiveError::NotZip
    );
    assert_eq!(
        classes(&open("no-font.zip")),
        [EntryClass::Document, EntryClass::LeftOut]
    );
}
