//! Tester cases for `specs/0083-font-info-and-zip` on the desktop host: adding archives as one
//! font folder (criteria 9, 11, 12, 13, 15, 16), the listing one level deep (criterion 10),
//! documents (criterion 4) and Remove (criterion 14), on temporary Curvyo folders under
//! `target/tmp`. Archives are built byte by byte with the text-core tester helper. Written from
//! the spec before the implementation was read.
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]
#![cfg(unix)]

#[path = "../../curvyo-text-core/tests/tester_0083/zip_raw.rs"]
mod zip_raw;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use curvyo_storage_io::{
    ArchiveRefusal, add_font_archive, font_document_pdf, list_font_folder, read_font_document,
    remove_add_leftovers, remove_font_files,
};
use zip_raw::{RawEntry, build};

const MIB: usize = 1024 * 1024;

fn included(path: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(path),
    )
    .unwrap()
}

fn serif() -> Vec<u8> {
    included("source-serif-4/SourceSerif4-Regular.ttf")
}

fn sans() -> Vec<u8> {
    included("source-sans-3/SourceSans3-Regular.otf")
}

fn vibes() -> Vec<u8> {
    included("great-vibes/GreatVibes-Regular.ttf")
}

/// A scratch root under `target/tmp` (nothing is written outside the checkout).
fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

/// Root layout: `<root>/fonts` is the Curvyo folder, `<root>/in` holds the archives.
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = scratch();
    let dir = root.path().join("fonts");
    let inbox = root.path().join("in");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&inbox).unwrap();
    (root, dir, inbox)
}

fn write_zip(inbox: &Path, name: &str, entries: &[RawEntry]) -> PathBuf {
    let path = inbox.join(name);
    std::fs::write(&path, build(entries)).unwrap();
    path
}

/// Every file and folder under `root` with its bytes (folders as `None`), symlinks not followed.
fn tree(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            let meta = std::fs::symlink_metadata(&p).unwrap();
            if meta.is_dir() {
                out.insert(rel, None);
                stack.push(p);
            } else {
                out.insert(rel, Some(std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    out
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// ---------------------------------------------------------------------------------------------
// Criteria 9, 12, 13: one font folder per archive, flat, whole
// ---------------------------------------------------------------------------------------------

/// Criterion 9: the folder is named after the archive, holds the fonts and documents side by
/// side, flat, with `-2` for a repeated base name; the archive stays where it was, untouched.
#[test]
fn ac09_font_folder_flat_and_named_after_the_archive() {
    let (_root, dir, inbox) = setup();
    let entries = [
        RawEntry::deflated("Lobster-1.0/static/Serif.ttf", &serif()),
        RawEntry::stored("Lobster-1.0/OFL.txt", b"ofl one"),
        RawEntry::stored("Lobster-1.0/extra/OFL.txt", b"ofl two"),
        RawEntry::stored("README.md", b"# readme"),
        RawEntry::stored("Specimen.pdf", b"%PDF-1.4"),
        RawEntry::stored("pic.png", b"png"),
        RawEntry::stored("__MACOSX/Lobster-1.0/._Serif.ttf", b"junk"),
    ];
    let zip = write_zip(&inbox, "Lobster.zip", &entries);
    let before = std::fs::read(&zip).unwrap();
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.refused, None);
    assert_eq!(added.folder.as_deref(), Some("Lobster"));
    assert_eq!(added.added, ["Serif.ttf"]);
    assert_eq!(added.documents, 4);
    assert_eq!(added.unsafe_entries, 0);
    assert_eq!(
        names_in(&dir.join("Lobster")),
        [
            "OFL-2.txt",
            "OFL.txt",
            "README.md",
            "Serif.ttf",
            "Specimen.pdf"
        ]
    );
    assert_eq!(
        std::fs::read(dir.join("Lobster/Serif.ttf")).unwrap(),
        serif()
    );
    assert_eq!(
        std::fs::read(dir.join("Lobster/OFL.txt")).unwrap(),
        b"ofl one"
    );
    assert_eq!(
        std::fs::read(dir.join("Lobster/OFL-2.txt")).unwrap(),
        b"ofl two"
    );
    assert_eq!(
        names_in(&dir),
        ["Lobster"],
        "nothing else in the Curvyo folder"
    );
    assert_eq!(
        std::fs::read(&zip).unwrap(),
        before,
        "the archive is only read"
    );
    assert_eq!(names_in(&inbox), ["Lobster.zip"]);
}

/// Criterion 9: a taken name, ignoring case, gives `-2`, `-3`; an unsafe archive name is
/// made safe.
#[test]
fn ac09_taken_and_unsafe_folder_names() {
    let (_root, dir, inbox) = setup();
    std::fs::create_dir(dir.join("lobster")).unwrap();
    std::fs::write(dir.join("lobster/keep.txt"), b"mine").unwrap();
    let a = write_zip(
        &inbox,
        "Lobster.zip",
        &[RawEntry::stored("A.ttf", &serif())],
    );
    assert_eq!(
        add_font_archive(&dir, &a).folder.as_deref(),
        Some("Lobster-2")
    );
    let b = write_zip(&inbox, "LOBSTER.ZIP", &[RawEntry::stored("B.otf", &sans())]);
    assert_eq!(
        add_font_archive(&dir, &b).folder.as_deref(),
        Some("LOBSTER-3")
    );
    let c = write_zip(&inbox, "CON.zip", &[RawEntry::stored("C.ttf", &vibes())]);
    assert_eq!(add_font_archive(&dir, &c).folder.as_deref(), Some("_CON"));
    assert_eq!(
        std::fs::read(dir.join("lobster/keep.txt")).unwrap(),
        b"mine"
    );
}

/// Criterion 13: the counts of one archive: added, already there, documents, unsafe.
#[test]
fn ac13_counts_of_a_mixed_archive() {
    let (_root, dir, inbox) = setup();
    std::fs::write(dir.join("Vibes.ttf"), vibes()).unwrap();
    let mut bad_crc = RawEntry::stored("Bad.ttf", &sans());
    bad_crc.crc ^= 1;
    let zip = write_zip(
        &inbox,
        "Pack.zip",
        &[
            RawEntry::stored("Serif.ttf", &serif()),
            RawEntry::stored("Sans.otf", &sans()),
            RawEntry::stored("Vibes-again.ttf", &vibes()),
            RawEntry::stored("OFL.txt", b"a"),
            RawEntry::stored("README", b"b"),
            RawEntry::stored("FONTLOG.txt", b"c"),
            RawEntry::stored("../evil.ttf", &serif()),
            bad_crc,
            RawEntry::stored("image.png", b"x"),
        ],
    );
    let added = add_font_archive(&dir, &zip);
    let mut names = added.added.clone();
    names.sort();
    assert_eq!(names, ["Sans.otf", "Serif.ttf"]);
    assert_eq!(added.present, 1);
    assert_eq!(added.documents, 3);
    assert_eq!(added.unsafe_entries, 2);
}

// ---------------------------------------------------------------------------------------------
// Criterion 11: de-duplication
// ---------------------------------------------------------------------------------------------

/// Criterion 11: a font already in a subfolder (or the top level) is "already there"; an
/// archive that adds no font creates no folder and keeps no document.
#[test]
fn ac11_already_there_anywhere_adds_nothing() {
    let (_root, dir, inbox) = setup();
    std::fs::create_dir(dir.join("hand")).unwrap();
    std::fs::write(dir.join("hand/Serif.ttf"), serif()).unwrap();
    std::fs::write(dir.join("Top.otf"), sans()).unwrap();
    let before = tree(&dir);
    let zip = write_zip(
        &inbox,
        "Again.zip",
        &[
            RawEntry::stored("x/Copy.ttf", &serif()),
            RawEntry::deflated("y/Copy.otf", &sans()),
            RawEntry::stored("OFL.txt", b"licence"),
        ],
    );
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.folder, None);
    assert_eq!(added.added, Vec::<String>::new());
    assert_eq!(added.present, 2);
    assert_eq!(
        added.documents, 0,
        "documents of an archive that adds no font are not kept"
    );
    assert_eq!(tree(&dir), before, "nothing written");
}

/// Criterion 11: the same face as OTF and TTF in one archive is added once, the TTF, whatever
/// the archive order; the other counts as already there.
#[test]
fn ac11_same_face_as_otf_and_ttf_added_once_ttf_first() {
    let (_root, dir, inbox) = setup();
    let ttf = serif();
    let mut other_bytes = serif();
    other_bytes.extend_from_slice(&[0, 0, 0, 0]);
    let zip = write_zip(
        &inbox,
        "Twice.zip",
        &[
            RawEntry::stored("otf/Face.otf", &other_bytes),
            RawEntry::stored("ttf/Face.ttf", &ttf),
        ],
    );
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.added, ["Face.ttf"]);
    assert_eq!(added.present, 1);
    assert_eq!(names_in(&dir.join("Twice")), ["Face.ttf"]);
}

// ---------------------------------------------------------------------------------------------
// Criteria 12, 15, 16: refusals write nothing
// ---------------------------------------------------------------------------------------------

fn assert_refused(entries_or_bytes: Vec<u8>, reason: Option<ArchiveRefusal>, what: &str) {
    let (_root, dir, inbox) = setup();
    std::fs::write(dir.join("Existing.ttf"), vibes()).unwrap();
    let before = tree(&dir);
    let zip = inbox.join("Pack.zip");
    std::fs::write(&zip, entries_or_bytes).unwrap();
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.refused, reason, "{what}: reason");
    assert_eq!(added.folder, None, "{what}: no folder");
    assert!(added.added.is_empty(), "{what}: nothing added");
    assert_eq!(added.documents, 0, "{what}: no document kept");
    assert_eq!(tree(&dir), before, "{what}: nothing written");
}

/// Criteria 13, 16: a file that is not a zip, and the overlapping-entry bomb, are unreadable.
#[test]
fn ac16_unreadable_archives_write_nothing() {
    assert_refused(
        b"PK\x03\x04 not really".to_vec(),
        Some(ArchiveRefusal::Unreadable),
        "not a zip",
    );
    assert_refused(
        serif(),
        Some(ArchiveRefusal::Unreadable),
        "a font named .zip",
    );
    let a = RawEntry::deflated_zeros("A.txt", MIB);
    let mut b = a.clone();
    b.name = b"B.ttf".to_vec();
    b.central_offset_of = Some(0);
    let mut font = RawEntry::stored("F.ttf", &serif());
    font.central_offset_of = None;
    assert_refused(
        build(&[font, a, b]),
        Some(ArchiveRefusal::Unreadable),
        "overlap bomb",
    );
}

/// Criterion 15: over the entry, font or total limit the archive is refused as over the size
/// limit, and nothing is written although some entries were fine.
#[test]
fn ac15_over_limit_archives_write_nothing() {
    let mut many: Vec<RawEntry> = vec![RawEntry::stored("F.ttf", &serif())];
    many.extend((0..10_000).map(|i| RawEntry::stored(&format!("{i}.png"), b"")));
    assert_refused(
        build(&many),
        Some(ArchiveRefusal::OverLimit),
        "10,001 entries",
    );

    let mut fonts: Vec<RawEntry> = vec![RawEntry::stored("F.ttf", &serif())];
    fonts.extend((0..500).map(|i| RawEntry::stored(&format!("x{i}.ttf"), b"x")));
    assert_refused(build(&fonts), Some(ArchiveRefusal::OverLimit), "501 fonts");

    let zeros = RawEntry::deflated_zeros("D.txt", 16 * MIB);
    let mut total = vec![RawEntry::stored("F.ttf", &serif())];
    total.extend((0..17).map(|i| RawEntry {
        name: format!("D{i}.txt").into_bytes(),
        ..zeros.clone()
    }));
    assert_refused(
        build(&total),
        Some(ArchiveRefusal::OverLimit),
        "272 MiB kept",
    );
}

/// Criterion 15: an archive file over 256 MiB is refused without reading it as an archive.
#[test]
fn ac15_archive_file_over_256_mib() {
    let (_root, dir, inbox) = setup();
    let before = tree(&dir);
    let zip = inbox.join("Huge.zip");
    let file = std::fs::File::create(&zip).unwrap();
    file.set_len(256 * MIB as u64 + 1).unwrap();
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.refused, Some(ArchiveRefusal::OverLimit));
    assert_eq!(tree(&dir), before);
}

/// Criterion 13: an archive with no font adds nothing (no folder, no documents).
#[test]
fn ac13_archive_without_font_adds_nothing() {
    assert_refused(
        build(&[
            RawEntry::stored("OFL.txt", b"licence"),
            RawEntry::stored("Font.woff2", b"woff"),
        ]),
        None,
        "no font",
    );
    let mut not_a_font = RawEntry::stored("Fake.ttf", b"this is not a font");
    not_a_font.flags = 0;
    assert_refused(
        build(&[not_a_font, RawEntry::stored("README", b"x")]),
        None,
        "a .ttf that is no font",
    );
}

/// Criterion 16: unsafe entries are skipped, counted and never written, inside or outside the
/// Curvyo folder; only the sanitised base name of a safe entry is ever used.
#[test]
fn ac16_unsafe_entries_never_written() {
    let (root, dir, inbox) = setup();
    let mut bad_crc = RawEntry::stored("crc/BadCrc.otf", &sans());
    bad_crc.crc ^= 1;
    let zip = write_zip(
        &inbox,
        "Mixed.zip",
        &[
            RawEntry::stored("../../Escape.otf", &sans()),
            RawEntry::stored("..\\..\\EscapeWin.otf", &sans()),
            RawEntry::stored("/tmp/Absolute.otf", &sans()),
            RawEntry::stored("C:\\Drive.otf", &sans()),
            RawEntry::stored("Link.txt", b"/etc/passwd").with_mode(0o120_777),
            RawEntry::stored("Fifo.ttf", &sans()).with_mode(0o010_644),
            bad_crc,
            RawEntry::stored("ok/Good.ttf", &serif()),
        ],
    );
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.added, ["Good.ttf"]);
    assert_eq!(added.unsafe_entries, 7);
    assert_eq!(names_in(&dir.join("Mixed")), ["Good.ttf"]);
    assert_eq!(
        names_in(root.path()),
        ["fonts", "in"],
        "nothing beside the Curvyo folder"
    );
    assert_eq!(names_in(&dir), ["Mixed"]);
    let link = std::fs::symlink_metadata(dir.join("Mixed/Good.ttf")).unwrap();
    assert!(link.file_type().is_file());
}

/// Criterion 16: a nested archive is never opened; its font is not added.
#[test]
fn ac16_nested_archive_not_opened() {
    let (_root, dir, inbox) = setup();
    let inner = build(&[RawEntry::stored("Inner.otf", &sans())]);
    let zip = write_zip(
        &inbox,
        "Outer.zip",
        &[
            RawEntry::stored("inner.zip", &inner),
            RawEntry::stored("Outer.ttf", &serif()),
        ],
    );
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.added, ["Outer.ttf"]);
    assert_eq!(added.unsafe_entries, 0, "left out, not unsafe");
    assert_eq!(names_in(&dir.join("Outer")), ["Outer.ttf"]);
}

/// Criterion 12: an add that cannot write (a read-only Curvyo folder) leaves no font folder,
/// no temporary folder and no partial file.
#[test]
fn ac12_failed_write_leaves_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, dir, inbox) = setup();
    let zip = write_zip(
        &inbox,
        "Lobster.zip",
        &[
            RawEntry::stored("Serif.ttf", &serif()),
            RawEntry::stored("OFL.txt", b"x"),
        ],
    );
    let before = tree(&dir);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let added = add_font_archive(&dir, &zip);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(added.refused, Some(ArchiveRefusal::Unreadable));
    assert_eq!(added.folder, None);
    assert_eq!(tree(&dir), before);
}

/// Criterion 12: a leftover `.curvyo-add-*` folder is neither listed nor removed by a listing
/// or an add; only the start-up cleanup removes it, and no other dot-folder.
#[test]
fn ac12_leftovers_removed_only_at_start() {
    let (_root, dir, inbox) = setup();
    for leftover in [".curvyo-add-1", ".curvyo-add-xyz"] {
        std::fs::create_dir(dir.join(leftover)).unwrap();
        std::fs::write(dir.join(leftover).join("Half.ttf"), serif()).unwrap();
    }
    for kept in [".mine", ".curvyo-addition", "curvyo-add-2"] {
        std::fs::create_dir(dir.join(kept)).unwrap();
        std::fs::write(dir.join(kept).join("notes.txt"), b"mine").unwrap();
    }
    let listed = list_font_folder(&dir);
    assert!(listed.faces.is_empty(), "the half folder is not listed");
    assert!(
        dir.join(".curvyo-add-1/Half.ttf").exists(),
        "a listing removes nothing"
    );
    let zip = write_zip(&inbox, "A.zip", &[RawEntry::stored("Sans.otf", &sans())]);
    let added = add_font_archive(&dir, &zip);
    assert_eq!(added.folder.as_deref(), Some("A"));
    assert!(
        dir.join(".curvyo-add-1/Half.ttf").exists(),
        "an add removes no leftover"
    );
    assert!(dir.join(".curvyo-add-xyz").exists());

    remove_add_leftovers(&dir);
    assert!(!dir.join(".curvyo-add-1").exists());
    assert!(!dir.join(".curvyo-add-xyz").exists());
    for kept in [".mine", ".curvyo-addition", "curvyo-add-2", "A"] {
        assert!(dir.join(kept).exists(), "{kept} is kept");
    }
}

/// Criterion 12: the cleanup never follows a symlinked leftover out of the Curvyo folder.
#[test]
fn ac12_cleanup_does_not_follow_links() {
    let (root, dir, _inbox) = setup();
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("precious.txt"), b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join(".curvyo-add-link")).unwrap();
    remove_add_leftovers(&dir);
    assert_eq!(
        std::fs::read(outside.join("precious.txt")).unwrap(),
        b"keep"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 10: the listing one level deep
// ---------------------------------------------------------------------------------------------

/// Criterion 10: top level and direct subfolders are listed; dot-folders, deeper folders and
/// symlinked folders are not; documents per folder A to Z ignoring case.
#[test]
fn ac10_one_level_no_dot_no_deeper_no_links() {
    let (root, dir, _inbox) = setup();
    std::fs::write(dir.join("Top.ttf"), serif()).unwrap();
    std::fs::create_dir_all(dir.join("Pack/deeper")).unwrap();
    std::fs::write(dir.join("Pack/Sans.otf"), sans()).unwrap();
    std::fs::write(dir.join("Pack/deeper/Vibes.ttf"), vibes()).unwrap();
    for doc in ["b.txt", "A.md", "README", "Licence.pdf"] {
        std::fs::write(dir.join("Pack").join(doc), b"d").unwrap();
    }
    std::fs::write(dir.join("Pack/notes.odt"), b"not a document").unwrap();
    std::fs::create_dir(dir.join(".hidden")).unwrap();
    std::fs::write(dir.join(".hidden/Vibes.ttf"), vibes()).unwrap();
    let outside = root.path().join("elsewhere");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("Vibes.ttf"), vibes()).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("Linked")).unwrap();

    let listed = list_font_folder(&dir);
    let mut faces: Vec<(String, Option<String>)> = listed
        .faces
        .iter()
        .map(|f| {
            (
                f.path.file_name().unwrap().to_string_lossy().into_owned(),
                f.folder.clone(),
            )
        })
        .collect();
    faces.sort();
    assert_eq!(
        faces,
        [
            ("Sans.otf".to_owned(), Some("Pack".to_owned())),
            ("Top.ttf".to_owned(), None),
        ]
    );
    let sizes: Vec<u64> = listed.faces.iter().map(|f| f.size).collect();
    assert!(sizes.contains(&(serif().len() as u64)));
    let pack = listed.folders.iter().find(|f| f.name == "Pack").unwrap();
    let docs: Vec<&str> = pack.documents.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(docs, ["A.md", "b.txt", "Licence.pdf", "README"]);
    assert!(
        listed
            .folders
            .iter()
            .all(|f| f.name != ".hidden" && f.name != "Linked")
    );
}

/// Criterion 10: a symlinked font file inside a subfolder does not lead out of the Curvyo
/// folder (no loop, no reach outside).
#[test]
fn ac10_symlink_loop_does_not_hang() {
    let (_root, dir, _inbox) = setup();
    std::fs::create_dir(dir.join("Loop")).unwrap();
    std::os::unix::fs::symlink(&dir, dir.join("Loop/back")).unwrap();
    std::os::unix::fs::symlink(&dir, dir.join("self")).unwrap();
    std::fs::write(dir.join("Loop/Sans.otf"), sans()).unwrap();
    let listed = list_font_folder(&dir);
    assert_eq!(listed.faces.len(), 1);
}

// ---------------------------------------------------------------------------------------------
// Criterion 4: documents
// ---------------------------------------------------------------------------------------------

/// Criterion 4: a text document is shown up to 1 MiB: exactly 1 MiB is whole, one byte more
/// is cut at 1 MiB and says so.
#[test]
fn ac04_text_document_one_mib_boundary() {
    let (_root, dir, _inbox) = setup();
    std::fs::create_dir(dir.join("Pack")).unwrap();
    std::fs::write(dir.join("Pack/Sans.otf"), sans()).unwrap();
    std::fs::write(dir.join("Pack/at.txt"), vec![b'a'; MIB]).unwrap();
    std::fs::write(dir.join("Pack/over.txt"), vec![b'b'; MIB + 1]).unwrap();
    std::fs::write(dir.join("Pack/empty.txt"), b"").unwrap();
    let (at, cut) = read_font_document(&dir, "Pack", "at.txt").unwrap();
    assert_eq!((at.len(), cut), (MIB, false));
    let (over, cut) = read_font_document(&dir, "Pack", "over.txt").unwrap();
    assert_eq!((over.len(), cut), (MIB, true));
    let (empty, cut) = read_font_document(&dir, "Pack", "empty.txt").unwrap();
    assert_eq!((empty.len(), cut), (0, false));
}

/// Criterion 4: only a document directly in a listed subfolder is read; paths, dot-folders,
/// links out and non-documents are refused.
#[test]
fn ac04_document_reads_stay_in_font_folders() {
    let (root, dir, _inbox) = setup();
    std::fs::create_dir(dir.join("Pack")).unwrap();
    std::fs::write(dir.join("Pack/Sans.otf"), sans()).unwrap();
    std::fs::write(dir.join("Pack/notes.odt"), b"x").unwrap();
    std::fs::write(dir.join("Top.txt"), b"x").unwrap();
    std::fs::create_dir(dir.join(".hidden")).unwrap();
    std::fs::write(dir.join(".hidden/OFL.txt"), b"x").unwrap();
    std::fs::write(root.path().join("secret.txt"), b"secret").unwrap();
    std::os::unix::fs::symlink(root.path().join("secret.txt"), dir.join("Pack/link.txt")).unwrap();
    std::os::unix::fs::symlink(root.path(), dir.join("Out")).unwrap();
    for (folder, name) in [
        ("Pack", "../../secret.txt"),
        ("..", "secret.txt"),
        ("Pack/..", "Top.txt"),
        ("", "Top.txt"),
        (".", "Top.txt"),
        ("Pack", "link.txt"),
        ("Out", "secret.txt"),
        (".hidden", "OFL.txt"),
        ("Pack", "notes.odt"),
        ("Pack", "Sans.otf"),
        ("Pack", "missing.txt"),
        ("/", "etc/passwd"),
    ] {
        assert_eq!(
            read_font_document(&dir, folder, name),
            None,
            "{folder}/{name} is not read"
        );
    }
}

/// Criterion 4: only a regular `.pdf` inside a font folder is handed to the viewer.
#[test]
fn ac04_pdf_opener_path_checks() {
    let (root, dir, _inbox) = setup();
    std::fs::create_dir(dir.join("Pack")).unwrap();
    std::fs::write(dir.join("Pack/Sans.otf"), sans()).unwrap();
    std::fs::write(dir.join("Pack/Licence.pdf"), b"%PDF").unwrap();
    std::fs::write(dir.join("Pack/OFL.txt"), b"x").unwrap();
    std::fs::create_dir(dir.join("Pack/Folder.pdf")).unwrap();
    std::fs::write(root.path().join("evil.pdf"), b"%PDF").unwrap();
    std::os::unix::fs::symlink(root.path().join("evil.pdf"), dir.join("Pack/Link.pdf")).unwrap();
    std::fs::write(root.path().join("run.sh"), b"#!/bin/sh").unwrap();
    std::os::unix::fs::symlink(root.path().join("run.sh"), dir.join("Pack/Run.pdf")).unwrap();

    let path = font_document_pdf(&dir, "Pack", "Licence.pdf").unwrap();
    assert!(
        path.canonicalize()
            .unwrap()
            .starts_with(dir.canonicalize().unwrap())
    );
    for name in [
        "OFL.txt",
        "Folder.pdf",
        "Link.pdf",
        "Run.pdf",
        "../../evil.pdf",
        "Sans.otf",
    ] {
        assert_eq!(font_document_pdf(&dir, "Pack", name), None, "{name}");
    }
    assert_eq!(font_document_pdf(&dir, "..", "evil.pdf"), None);
}

// ---------------------------------------------------------------------------------------------
// Criterion 14: Remove from Curvyo fonts
// ---------------------------------------------------------------------------------------------

/// Criterion 14: the family's files, the folder's documents go; the maker's other file keeps
/// the folder alive.
#[test]
fn ac14_remove_keeps_other_files_of_the_maker() {
    let (_root, dir, _inbox) = setup();
    let pack = dir.join("Pack");
    std::fs::create_dir(&pack).unwrap();
    std::fs::write(pack.join("Sans.otf"), sans()).unwrap();
    std::fs::write(pack.join("OFL.txt"), b"x").unwrap();
    std::fs::write(pack.join("README.md"), b"x").unwrap();
    std::fs::write(pack.join("Specimen.pdf"), b"x").unwrap();
    std::fs::write(pack.join("sketch.png"), b"mine").unwrap();
    let removed = remove_font_files(&dir, &[(pack.join("Sans.otf"), 0)]);
    assert_eq!(removed.removed.len(), 1);
    assert_eq!(removed.documents, 3);
    assert_eq!(names_in(&pack), ["sketch.png"]);
}

/// Criterion 14: a folder left with only documents is deleted with them.
#[test]
fn ac14_remove_deletes_the_emptied_font_folder() {
    let (_root, dir, _inbox) = setup();
    let pack = dir.join("Pack");
    std::fs::create_dir(&pack).unwrap();
    std::fs::write(pack.join("Sans.otf"), sans()).unwrap();
    std::fs::write(pack.join("OFL.txt"), b"x").unwrap();
    let removed = remove_font_files(&dir, &[(pack.join("Sans.otf"), 0)]);
    assert_eq!(removed.documents, 1);
    assert!(!pack.exists());
    assert!(dir.exists(), "the Curvyo folder stays");
}

/// Criterion 14: a font folder still holding another family keeps its documents.
#[test]
fn ac14_remove_keeps_documents_while_another_family_stays() {
    let (_root, dir, _inbox) = setup();
    let pack = dir.join("Pack");
    std::fs::create_dir(&pack).unwrap();
    std::fs::write(pack.join("Sans.otf"), sans()).unwrap();
    std::fs::write(pack.join("Serif.ttf"), serif()).unwrap();
    std::fs::write(pack.join("OFL.txt"), b"x").unwrap();
    let removed = remove_font_files(&dir, &[(pack.join("Sans.otf"), 0)]);
    assert_eq!(removed.documents, 0);
    assert_eq!(names_in(&pack), ["OFL.txt", "Serif.ttf"]);
}

/// Criterion 14 with criterion 10: files in a dot-folder or through a symlinked folder are
/// not Curvyo fonts and are never deleted.
#[test]
fn ac14_remove_refuses_dot_and_linked_folders() {
    let (root, dir, _inbox) = setup();
    std::fs::create_dir(dir.join(".hidden")).unwrap();
    std::fs::write(dir.join(".hidden/Sans.otf"), sans()).unwrap();
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("Serif.ttf"), serif()).unwrap();
    std::fs::write(outside.join("OFL.txt"), b"x").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("Linked")).unwrap();
    let removed = remove_font_files(
        &dir,
        &[
            (dir.join(".hidden/Sans.otf"), 0),
            (dir.join("Linked/Serif.ttf"), 0),
        ],
    );
    assert!(removed.removed.is_empty(), "{:?}", removed.removed);
    assert!(dir.join(".hidden/Sans.otf").exists());
    assert_eq!(names_in(&outside), ["OFL.txt", "Serif.ttf"]);
}

/// Criteria 9, 10, 12: an archive whose name starts with a dot (or is only dots and spaces)
/// still gives a font folder the listing reads; it never becomes a dot-folder that the listing
/// skips or the start-up cleanup deletes.
#[test]
#[ignore = "0083 FAIL: `.Lobster.zip` gives the dot-folder `.Lobster` (never listed) and `.curvyo-add-7.zip` gives `.curvyo-add-7` (deleted at the next start)"]
fn ac09_dot_archive_names_give_a_listed_folder() {
    let mut wrong = Vec::new();
    for (i, name) in [".Lobster.zip", ".curvyo-add-7.zip", "...zip", " .zip"]
        .into_iter()
        .enumerate()
    {
        let (_root, dir, inbox) = setup();
        let font = [serif(), sans(), vibes(), serif()][i].clone();
        let zip = write_zip(&inbox, name, &[RawEntry::stored("Font.ttf", &font)]);
        let added = add_font_archive(&dir, &zip);
        let folder = added.folder.clone().unwrap_or_default();
        remove_add_leftovers(&dir);
        let listed = list_font_folder(&dir).faces.len();
        if added.added.len() != 1 || folder.starts_with('.') || folder.is_empty() || listed != 1 {
            wrong.push(format!("{name:?} -> folder {folder:?}, listed {listed}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}
