//! Tester cases for `specs/0083-font-info-and-zip` in the core: the archive reader on archives
//! built byte by byte (criteria 8, 9, 15, 16, 17, 19) and the info fields of faces whose name
//! and OS/2 tables are rewritten in memory (criteria 2, 5). Written from the spec before the
//! implementation was read; the existing golden-fixture tests are not repeated.
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]
#![cfg(not(target_arch = "wasm32"))]

mod common;
#[path = "tester_0083/sfnt_edit.rs"]
mod sfnt_edit;
#[path = "tester_0083/zip_raw.rs"]
mod zip_raw;

use common::{SANS, SERIF, font_bytes};
use curvyo_text_core::limits::{
    MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_FONTS, MAX_DOCUMENT_BYTES, MAX_FACE_BYTES,
};
use curvyo_text_core::{
    ArchiveError, EntryClass, EntrySkip, FaceStore, FontArchive, FontFileKind, FontFormat,
    FontInfo, UnsafeReason, font_info, free_folder_name, is_document_name, sanitised_name,
};
use zip_raw::{RawEntry, build, build_with_count};

const MIB: usize = 1024 * 1024;

fn open(entries: &[RawEntry]) -> FontArchive {
    FontArchive::open(build(entries)).unwrap()
}

fn class_of(archive: &FontArchive, name: &str) -> Option<EntryClass> {
    archive
        .entries()
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.class)
}

// ---------------------------------------------------------------------------------------------
// Criterion 8 and the Terms: classes, nesting, left-out entries
// ---------------------------------------------------------------------------------------------

/// Terms: the document rule, case-insensitive prefixes, the three extensions, nothing else.
#[test]
fn ac08_document_names_follow_the_terms() {
    for yes in [
        "OFL.txt",
        "ofl",
        "LICENSE",
        "License.md",
        "LICENCE.pdf",
        "COPYING",
        "Copyright.txt",
        "README",
        "readme.MD",
        "READ ME.txt",
        "CREDITS",
        "AUTHORS.txt",
        "FONTLOG.txt",
        "EULA.pdf",
        "OFL-FAQ.txt",
        "anything.txt",
        "Specimen.PDF",
        "notes.md",
    ] {
        assert!(is_document_name(yes), "{yes} is a document");
    }
    for no in [
        "LICENSE.html",
        "README.rtf",
        "OFL.doc",
        "specimen.png",
        "Font.ttf",
        "font.woff2",
        "index.html",
        "",
    ] {
        assert!(!is_document_name(no), "{no} is not a document");
    }
}

/// Criterion 8: folders nested to any depth are looked at; every class lands where the Terms
/// put it; `__MACOSX/`, `._` and dot names are not listed as kept or unsafe.
#[test]
fn ac08_every_entry_any_depth_classed() {
    let serif = font_bytes(SERIF);
    let deep = "a/b/c/d/e/f/g/h/i/j/k/Deep.ttf";
    let archive = open(&[
        RawEntry::stored(deep, &serif),
        RawEntry::stored("x/Upper.OTF", &serif),
        RawEntry::stored("Coll.ttc", &serif),
        RawEntry::stored("single/Line.svg", b"<svg/>"),
        RawEntry::stored("docs/deeper/OFL.txt", b"ofl"),
        RawEntry::stored("docs/README", b"readme"),
        RawEntry::stored("pic.png", b"png"),
        RawEntry::stored("web/Font.woff2", b"woff"),
        RawEntry::stored("__MACOSX/a/._Deep.ttf", b"junk"),
        RawEntry::stored("__MACOSX/README.txt", b"junk"),
        RawEntry::stored("a/._Deep.ttf", b"junk"),
        RawEntry::stored(".DS_Store", b"junk"),
        RawEntry::stored("a/.hidden.txt", b"junk"),
    ]);
    assert_eq!(
        class_of(&archive, deep),
        Some(EntryClass::Font(FontFileKind::Ttf))
    );
    assert_eq!(
        class_of(&archive, "x/Upper.OTF"),
        Some(EntryClass::Font(FontFileKind::Otf)),
        "extensions any case"
    );
    assert_eq!(
        class_of(&archive, "Coll.ttc"),
        Some(EntryClass::Font(FontFileKind::Ttc))
    );
    assert_eq!(
        class_of(&archive, "single/Line.svg"),
        Some(EntryClass::Font(FontFileKind::Svg))
    );
    assert_eq!(
        class_of(&archive, "docs/deeper/OFL.txt"),
        Some(EntryClass::Document)
    );
    assert_eq!(
        class_of(&archive, "docs/README"),
        Some(EntryClass::Document)
    );
    for left in ["pic.png", "web/Font.woff2"] {
        assert_eq!(
            class_of(&archive, left),
            Some(EntryClass::LeftOut),
            "{left}"
        );
    }
    for hidden in [
        "__MACOSX/a/._Deep.ttf",
        "__MACOSX/README.txt",
        "a/._Deep.ttf",
        ".DS_Store",
        "a/.hidden.txt",
    ] {
        let class = class_of(&archive, hidden);
        assert!(
            matches!(class, None | Some(EntryClass::LeftOut)),
            "{hidden}: left out, never kept or unsafe ({class:?})"
        );
    }
}

/// Criterion 8: folder entries are not files.
#[test]
fn ac08_directory_entries_are_not_listed() {
    let archive = open(&[
        RawEntry::stored("fonts/", b"").with_mode(0o040_755),
        RawEntry::stored("fonts/A.ttf", &font_bytes(SERIF)),
    ]);
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.entries()[0].name, "fonts/A.ttf");
}

// ---------------------------------------------------------------------------------------------
// Criterion 9: flat, unique, safe names
// ---------------------------------------------------------------------------------------------

/// Criterion 9: same base names in different folders (also differing only in case) get `-2`,
/// `-3`; the inner folders are not part of the name.
#[test]
fn ac09_same_base_name_gets_a_number() {
    let archive = open(&[
        RawEntry::stored("Family A/OFL.txt", b"a"),
        RawEntry::stored("Family B/OFL.txt", b"b"),
        RawEntry::stored("Family C/ofl.TXT", b"c"),
        RawEntry::stored("Family A/static/Font.ttf", &font_bytes(SERIF)),
    ]);
    let names: Vec<String> = archive
        .entries()
        .iter()
        .map(|e| e.file_name.clone())
        .collect();
    assert_eq!(names[0], "OFL.txt");
    assert_eq!(names[1], "OFL-2.txt");
    assert!(
        !names[2].eq_ignore_ascii_case("OFL.txt") && !names[2].eq_ignore_ascii_case("OFL-2.txt"),
        "a name equal ignoring case is taken too: {}",
        names[2]
    );
    assert_eq!(names[3], "Font.ttf");
    let lower: std::collections::HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();
    assert_eq!(lower.len(), names.len(), "unique ignoring case: {names:?}");
}

/// Criterion 9: the sanitising rules for Windows, macOS and Linux.
#[test]
fn ac09_sanitised_names() {
    assert_eq!(
        sanitised_name("a<b>c:d\"e|f?g*h.txt"),
        "a_b_c_d_e_f_g_h.txt"
    );
    assert_eq!(sanitised_name("tab\there\u{1}.txt"), "tab_here_.txt");
    for reserved in [
        "CON", "con.txt", "PRN.ttf", "AUX", "NUL.md", "COM1.otf", "LPT9",
    ] {
        let safe = sanitised_name(reserved);
        assert_eq!(safe, format!("_{reserved}"), "{reserved}");
    }
    assert_eq!(sanitised_name("notes.txt. . "), "notes.txt");
    assert_eq!(
        sanitised_name("CONSOLE.txt"),
        "CONSOLE.txt",
        "only exact reserved names"
    );
    let long = format!("{}.ttf", "x".repeat(300));
    let cut = sanitised_name(&long);
    assert!(cut.chars().count() <= 120, "{} chars", cut.chars().count());
    for name in [
        sanitised_name(&long),
        sanitised_name("a<>"),
        sanitised_name("..."),
    ] {
        assert!(!name.is_empty(), "a sanitised name is never empty");
        assert!(!name.ends_with('.') && !name.ends_with(' '), "{name:?}");
        assert!(
            !name.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*']),
            "{name:?}"
        );
    }
}

/// Criterion 9: the folder name `-N` rule ignores case.
#[test]
fn ac09_free_folder_name_ignores_case() {
    assert_eq!(free_folder_name("Lobster", &[]), "Lobster");
    assert_eq!(
        free_folder_name("Lobster", &["lobster".to_owned()]),
        "Lobster-2"
    );
    assert_eq!(
        free_folder_name("Lobster", &["LOBSTER".to_owned(), "lobster-2".to_owned()]),
        "Lobster-3"
    );
}

/// Criterion 9: names in an archive pass through the sanitiser: no separator or reserved
/// character survives into a file name.
#[test]
fn ac09_entry_file_names_are_safe() {
    let archive = open(&[
        RawEntry::stored("dir/Bad:Name?.txt", b"x"),
        RawEntry::stored("dir/CON.txt", b"x"),
        RawEntry::stored("dir/trailing. .txt", b"x"),
    ]);
    let names: Vec<&str> = archive
        .entries()
        .iter()
        .map(|e| e.file_name.as_str())
        .collect();
    assert_eq!(names, ["Bad_Name_.txt", "_CON.txt", "trailing. .txt"]);
}

// ---------------------------------------------------------------------------------------------
// Criterion 15: limits on bytes actually read
// ---------------------------------------------------------------------------------------------

fn tiny_files(n: usize) -> Vec<RawEntry> {
    (0..n)
        .map(|i| RawEntry::stored(&format!("f{i}.png"), b""))
        .collect()
}

/// Criterion 15: 10,000 entries are fine, 10,001 are over the size limit.
#[test]
fn ac15_entry_count_boundary() {
    assert_eq!(MAX_ARCHIVE_ENTRIES, 10_000);
    assert!(FontArchive::open(build(&tiny_files(10_000))).is_ok());
    assert_eq!(
        FontArchive::open(build(&tiny_files(10_001))).unwrap_err(),
        ArchiveError::OverLimit
    );
}

/// Criterion 15: an end record that claims 10,001 entries is refused before a table is built.
#[test]
fn ac15_declared_entry_count_over_limit() {
    let bytes = build_with_count(&tiny_files(3), Some(10_001));
    assert!(matches!(
        FontArchive::open(bytes),
        Err(ArchiveError::OverLimit | ArchiveError::Damaged)
    ));
}

/// Criterion 15: 500 font entries are fine, 501 are over the limit.
#[test]
fn ac15_font_count_boundary() {
    assert_eq!(MAX_ARCHIVE_FONTS, 500);
    let fonts = |n: usize| -> Vec<RawEntry> {
        (0..n)
            .map(|i| RawEntry::stored(&format!("F{i}.ttf"), b"x"))
            .collect()
    };
    assert!(FontArchive::open(build(&fonts(500))).is_ok());
    assert_eq!(
        FontArchive::open(build(&fonts(501))).unwrap_err(),
        ArchiveError::OverLimit
    );
}

/// Criterion 15: a document of exactly 16 MiB is read; one byte more is skipped as unsafe (a
/// zip bomb by compression ratio: 16 MiB of zeros in a few KiB).
#[test]
fn ac15_document_entry_limit() {
    assert_eq!(MAX_DOCUMENT_BYTES, 16 * MIB);
    let mut archive = open(&[
        RawEntry::deflated_zeros("at.txt", 16 * MIB),
        RawEntry::deflated_zeros("over.txt", 16 * MIB + 1),
    ]);
    assert_eq!(archive.read(0).unwrap().len(), 16 * MIB);
    assert_eq!(archive.read(1).unwrap_err(), EntrySkip::Unsafe);
}

/// Criterion 15: a font entry over 64 MiB is skipped as unsafe, the archive is not refused.
#[test]
fn ac15_font_entry_limit() {
    assert_eq!(MAX_FACE_BYTES, 64 * MIB);
    let mut archive = open(&[
        RawEntry::deflated_zeros("Bomb.ttf", 64 * MIB + 1),
        RawEntry::stored("Real.ttf", &font_bytes(SERIF)),
    ]);
    assert_eq!(archive.read(0).unwrap_err(), EntrySkip::Unsafe);
    assert_eq!(archive.read(1).unwrap(), font_bytes(SERIF));
}

/// Criterion 15: the kept entries together over 256 MiB refuse the whole archive (seventeen
/// documents of 16 MiB, each under its own limit).
#[test]
fn ac15_total_kept_bytes_refuse_the_archive() {
    let zeros = RawEntry::deflated_zeros("D.txt", 16 * MIB);
    let entries: Vec<RawEntry> = (0..17)
        .map(|i| RawEntry {
            name: format!("D{i}.txt").into_bytes(),
            ..zeros.clone()
        })
        .collect();
    let mut archive = open(&entries);
    let mut outcome = Ok(());
    for i in 0..17 {
        if let Err(skip) = archive.read(i) {
            outcome = Err((i, skip));
            break;
        }
    }
    assert_eq!(outcome, Err((16, EntrySkip::ArchiveOverLimit)));
}

/// Criterion 15: sizes the archive declares are never trusted: declared huge sizes of small
/// entries do not refuse the archive, and data beyond a small declared size is unsafe.
#[test]
fn ac15_declared_sizes_are_not_trusted() {
    let mut lying_big: Vec<RawEntry> = (0..20)
        .map(|i| {
            let mut e = RawEntry::stored(&format!("D{i}.txt"), b"tiny");
            e.plain_size = 20 * MIB as u32;
            e
        })
        .collect();
    lying_big.push(RawEntry::stored("F.ttf", &font_bytes(SERIF)));
    assert!(
        FontArchive::open(build(&lying_big)).is_ok(),
        "400 MiB declared, 80 bytes real: not over the limit"
    );

    let mut small = RawEntry::deflated_zeros("doc.txt", 4 * MIB);
    small.plain_size = 1000;
    let mut archive = open(&[small]);
    assert_eq!(archive.read(0).unwrap_err(), EntrySkip::Unsafe);
}

// ---------------------------------------------------------------------------------------------
// Criterion 16: hostile and damaged archives
// ---------------------------------------------------------------------------------------------

/// Criterion 16: `..` parts, absolute paths, drive letters, backslashes and UNC prefixes are
/// skipped as unsafe; harmless dots are not.
#[test]
fn ac16_unsafe_paths() {
    let unsafe_names = [
        "../evil.ttf",
        "a/../../evil.txt",
        "a/b/..",
        "/etc/evil.ttf",
        "\\evil.ttf",
        "a\\..\\..\\evil.ttf",
        "..\\evil.txt",
        "C:evil.ttf",
        "C:\\Windows\\evil.ttf",
        "c:/evil.txt",
        "\\\\server\\share\\evil.ttf",
        "//server/share/evil.ttf",
    ];
    let mut entries: Vec<RawEntry> = unsafe_names
        .iter()
        .map(|n| RawEntry::stored(n, b"x"))
        .collect();
    entries.push(RawEntry::stored("a..b/Font..Name.ttf", b"x"));
    entries.push(RawEntry::stored("dir/./OFL.txt", b"x"));
    let archive = open(&entries);
    for name in unsafe_names {
        assert_eq!(
            class_of(&archive, name),
            Some(EntryClass::Unsafe(UnsafeReason::Path)),
            "{name}"
        );
    }
    assert_eq!(
        class_of(&archive, "a..b/Font..Name.ttf"),
        Some(EntryClass::Font(FontFileKind::Ttf))
    );
    for e in archive.entries() {
        assert!(
            !e.file_name.contains(['/', '\\']) && e.file_name != ".." && e.file_name != ".",
            "{:?} is never a path",
            e.file_name
        );
    }
}

/// Criterion 16: symlinks and every other non-file Unix type are unsafe; their target is
/// never read.
#[test]
fn ac16_non_file_types_unsafe() {
    let modes = [
        ("link.ttf", 0o120_777),
        ("link.txt", 0o120_777),
        ("fifo.ttf", 0o010_644),
        ("chr.txt", 0o020_644),
        ("blk.otf", 0o060_644),
        ("sock.ttf", 0o140_644),
    ];
    let entries: Vec<RawEntry> = modes
        .iter()
        .map(|(n, m)| RawEntry::stored(n, b"/etc/passwd").with_mode(*m))
        .collect();
    let mut archive = open(&entries);
    for (i, (name, _)) in modes.iter().enumerate() {
        assert_eq!(
            class_of(&archive, name),
            Some(EntryClass::Unsafe(UnsafeReason::NotAFile)),
            "{name}"
        );
        assert!(archive.read(i).is_err(), "{name} is not handed out");
    }
}

/// Criterion 16: an archive made on DOS (no Unix mode) holds regular files.
#[test]
fn ac16_dos_made_entries_are_files() {
    let mut e = RawEntry::stored("Font.ttf", &font_bytes(SERIF));
    e.made_by = 20;
    e.external_attr = 0x20;
    let archive = open(&[e]);
    assert_eq!(
        archive.entries()[0].class,
        EntryClass::Font(FontFileKind::Ttf)
    );
}

/// Criterion 16: two central records over the same data (the overlapping-file bomb) make the
/// archive unreadable.
#[test]
fn ac16_overlapping_entries_unreadable() {
    let a = RawEntry::deflated_zeros("A.txt", MIB);
    let mut b = a.clone();
    b.name = b"B.txt".to_vec();
    b.central_offset_of = Some(0);
    assert_eq!(
        FontArchive::open(build(&[a.clone(), b])).unwrap_err(),
        ArchiveError::Damaged
    );

    let mut bombs = vec![a.clone()];
    for i in 1..200 {
        let mut e = a.clone();
        e.name = format!("B{i}.txt").into_bytes();
        e.central_offset_of = Some(0);
        bombs.push(e);
    }
    assert_eq!(
        FontArchive::open(build(&bombs)).unwrap_err(),
        ArchiveError::Damaged
    );
}

/// Criterion 16: a central record pointing into the central directory is damaged.
#[test]
fn ac16_offset_into_central_directory_damaged() {
    let mut bytes = build(&[RawEntry::stored("A.txt", b"hello")]);
    let eocd = bytes.len() - 22;
    let cd_start = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let point = (cd_start as u32).to_le_bytes();
    bytes[cd_start + 42..cd_start + 46].copy_from_slice(&point);
    assert!(matches!(
        FontArchive::open(bytes),
        Err(ArchiveError::Damaged | ArchiveError::NotZip)
    ));
}

/// Criterion 9 and 16: two distinct entries with the same full name (not overlapping) are both
/// kept under unique names; this is not the overlap bomb.
#[test]
#[ignore = "0083 FAIL: two entries with the same full name list as one; the first is dropped silently, neither kept as OFL-2.txt nor counted"]
fn ac16_duplicate_names_without_overlap() {
    let archive = open(&[
        RawEntry::stored("OFL.txt", b"first"),
        RawEntry::stored("OFL.txt", b"second"),
    ]);
    let names: Vec<String> = archive
        .entries()
        .iter()
        .map(|e| e.file_name.clone())
        .collect();
    let mut archive = archive;
    let first = archive
        .read(0)
        .map(|b| String::from_utf8_lossy(&b).into_owned());
    assert_eq!(names, ["OFL.txt", "OFL-2.txt"], "first read: {first:?}");
}

/// Criterion 16: archives inside the archive are left out and never opened.
#[test]
fn ac16_nested_archives_left_out_unopened() {
    let inner = build(&[RawEntry::stored("Inner.ttf", &font_bytes(SERIF))]);
    let archive = open(&[
        RawEntry::stored("inner.zip", &inner),
        RawEntry::stored("x/INNER.ZIP", &inner),
        RawEntry::stored("a.7z", b"7z"),
        RawEntry::stored("a.rar", b"rar"),
        RawEntry::stored("a.tar", b"tar"),
        RawEntry::stored("a.tar.gz", b"gz"),
    ]);
    for e in archive.entries() {
        assert_eq!(e.class, EntryClass::LeftOut, "{}", e.name);
    }
    assert!(
        archive.entries().iter().all(|e| !e.name.contains("Inner")),
        "nothing from inside is listed"
    );
}

/// Criterion 16: a wrong CRC-32 and data beyond the declared size are skipped as unsafe, for
/// stored and deflated entries; the other entries still read.
#[test]
fn ac16_crc_and_declared_size() {
    let serif = font_bytes(SERIF);
    let mut bad_crc = RawEntry::stored("Bad.ttf", &serif);
    bad_crc.crc ^= 1;
    let mut bad_crc_deflate = RawEntry::deflated("Bad.txt", b"licence text");
    bad_crc_deflate.crc ^= 0x8000_0000;
    let mut long = RawEntry::deflated("Long.txt", b"0123456789");
    long.plain_size = 4;
    let mut archive = open(&[
        bad_crc,
        bad_crc_deflate,
        long,
        RawEntry::deflated("Good.txt", b"good"),
    ]);
    assert_eq!(archive.read(0).unwrap_err(), EntrySkip::Unsafe);
    assert_eq!(archive.read(1).unwrap_err(), EntrySkip::Unsafe);
    assert_eq!(archive.read(2).unwrap_err(), EntrySkip::Unsafe);
    assert_eq!(archive.read(3).unwrap(), b"good");
}

/// Criterion 16: encrypted entries and methods other than stored and deflate are left out.
#[test]
fn ac16_encrypted_and_other_methods_left_out() {
    let serif = font_bytes(SERIF);
    let mut encrypted = RawEntry::stored("Enc.ttf", &serif);
    encrypted.flags |= 1;
    let mut methods = vec![encrypted];
    for (name, method) in [
        ("Bz.ttf", 12_u16),
        ("Lzma.ttf", 14),
        ("Zstd.txt", 93),
        ("Aes.ttf", 99),
        ("Implode.txt", 6),
    ] {
        let mut e = RawEntry::stored(name, &serif);
        e.method = method;
        methods.push(e);
    }
    let mut archive = open(&methods);
    for i in 0..archive.entries().len() {
        let e = &archive.entries()[i];
        assert_eq!(e.class, EntryClass::LeftOut, "{}", e.name);
        assert_eq!(archive.read(i).unwrap_err(), EntrySkip::NotKept);
    }
}

/// Criterion 16: what is not a zip is "could not be read", and damaged bytes never panic.
#[test]
fn ac16_not_a_zip_and_truncations() {
    assert_eq!(
        FontArchive::open(Vec::new()).unwrap_err(),
        ArchiveError::NotZip
    );
    assert_eq!(
        FontArchive::open(font_bytes(SERIF)).unwrap_err(),
        ArchiveError::NotZip
    );
    let mut eocd_only = b"garbage".to_vec();
    eocd_only.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    eocd_only.extend_from_slice(&[0xFF; 18]);
    assert!(FontArchive::open(eocd_only).is_err());
    let good = build(&[
        RawEntry::deflated("OFL.txt", b"licence"),
        RawEntry::stored("F.ttf", &font_bytes(SERIF)),
    ]);
    for cut in [1, 10, 22, 23, 60, good.len() / 2, good.len() - 30] {
        let bytes = good[..good.len() - cut].to_vec();
        if let Ok(mut archive) = FontArchive::open(bytes) {
            for i in 0..archive.entries().len() {
                let _ = archive.read(i);
            }
        }
    }
}

/// Criterion 16: names in CP437 are decoded for display; a name with the UTF-8 flag and bad
/// bytes shows `_` for them.
#[test]
fn ac16_name_decoding() {
    let cp437 = RawEntry::stored("x", b"x").raw_name(b"M\x81ller.txt");
    let mut utf8 = RawEntry::stored("x", b"x").raw_name("Grüße.txt".as_bytes());
    utf8.flags |= 0x0800;
    let mut bad = RawEntry::stored("x", b"x").raw_name(b"Bad\xFF\xFEName.txt");
    bad.flags |= 0x0800;
    let archive = open(&[cp437, utf8, bad]);
    let names: Vec<&str> = archive.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names[0], "Müller.txt");
    assert_eq!(names[1], "Grüße.txt");
    assert!(
        names[2].starts_with("Bad_") && names[2].ends_with("Name.txt"),
        "{}",
        names[2]
    );
    assert!(!names[2].contains('\u{FFFD}'));
}

// ---------------------------------------------------------------------------------------------
// Criterion 17: an entry's bytes are those of the archive, unchanged
// ---------------------------------------------------------------------------------------------

/// Criterion 17: a font read from an archive is byte-equal to the loose file, so the loose
/// file's reader and limits apply to the same bytes; deflated or stored.
#[test]
fn ac17_entry_bytes_equal_the_loose_file() {
    let sans = font_bytes(SANS);
    let mut archive = open(&[
        RawEntry::deflated("a/Sans.otf", &sans),
        RawEntry::stored("b/Sans-copy.otf", &sans),
    ]);
    assert_eq!(archive.read(0).unwrap(), sans);
    assert_eq!(archive.read(1).unwrap(), sans);
    let mut store = FaceStore::new();
    assert!(store.add(archive.read(0).unwrap(), 0).is_ok());
}

/// Criterion 17: a left-out entry is never handed out.
#[test]
fn ac17_left_out_is_not_read() {
    let mut archive = open(&[RawEntry::stored("pic.png", b"png")]);
    assert_eq!(archive.read(0).unwrap_err(), EntrySkip::NotKept);
    assert_eq!(archive.read(99).unwrap_err(), EntrySkip::NotKept);
}

// ---------------------------------------------------------------------------------------------
// Criteria 2 and 5: info fields
// ---------------------------------------------------------------------------------------------

fn info_of(bytes: Vec<u8>, index: u32) -> FontInfo {
    let mut store = FaceStore::new();
    let key = store.add(bytes, index).unwrap();
    font_info(store.get(&key).unwrap())
}

const EN: u16 = 0x0409;
const DE: u16 = 0x0407;

fn all_names() -> Vec<(u16, u16, &'static str)> {
    vec![
        (0, EN, "id0 copyright"),
        (1, EN, "Tester Family"),
        (2, EN, "Regular"),
        (3, EN, "id3 unique"),
        (4, EN, "id4 full name"),
        (5, EN, "id5 version"),
        (6, EN, "TesterFamily-Regular"),
        (7, EN, "id7 trademark"),
        (8, EN, "id8 vendor"),
        (9, EN, "id9 designer"),
        (13, EN, "id13 licence"),
        (14, EN, "id14 https://example.org/licence"),
        (9, DE, "nicht dieser"),
        (13, DE, "nicht diese"),
    ]
}

/// Criterion 2: each field comes from its own name ID (distinct values catch a swap), the
/// English US record wins over another language.
#[test]
fn ac02_each_field_from_its_name_id() {
    let font = sfnt_edit::with_table(
        &font_bytes(SERIF),
        *b"name",
        Some(sfnt_edit::name_table(&all_names())),
    );
    let info = info_of(font, 0);
    assert_eq!(info.copyright.as_deref(), Some("id0 copyright"));
    assert_eq!(info.full_name.as_deref(), Some("id4 full name"));
    assert_eq!(info.version.as_deref(), Some("id5 version"));
    assert_eq!(info.trademark.as_deref(), Some("id7 trademark"));
    assert_eq!(info.vendor.as_deref(), Some("id8 vendor"));
    assert_eq!(info.designer.as_deref(), Some("id9 designer"));
    assert_eq!(info.licence.as_deref(), Some("id13 licence"));
    assert_eq!(
        info.licence_url.as_deref(),
        Some("id14 https://example.org/licence")
    );
    assert_eq!(info.format, FontFormat::TrueType);
}

/// Criterion 2: Vendor falls back to the OS/2 `achVendID` code when name ID 8 is missing; a
/// face with only family and style states nothing else.
#[test]
fn ac02_vendor_falls_back_to_ach_vend_id() {
    let names: Vec<_> = all_names()
        .into_iter()
        .filter(|(id, _, _)| *id != 8)
        .collect();
    let font = sfnt_edit::with_table(
        &font_bytes(SERIF),
        *b"name",
        Some(sfnt_edit::name_table(&names)),
    );
    let font = sfnt_edit::with_os2(&font, 0x0208, *b"TSTR");
    let info = info_of(font, 0);
    assert_eq!(info.vendor.as_deref(), Some("TSTR"));
    assert_eq!(info.fs_type, Some(0x0208));

    let bare = sfnt_edit::with_table(
        &font_bytes(SERIF),
        *b"name",
        Some(sfnt_edit::name_table(&[
            (1, EN, "Bare"),
            (2, EN, "Regular"),
        ])),
    );
    let bare = sfnt_edit::with_os2(&bare, 0, *b"    ");
    let info = info_of(bare, 0);
    for (field, value) in [
        ("full name", &info.full_name),
        ("version", &info.version),
        ("designer", &info.designer),
        ("copyright", &info.copyright),
        ("trademark", &info.trademark),
        ("licence", &info.licence),
        ("licence url", &info.licence_url),
        ("vendor (blank achVendID)", &info.vendor),
    ] {
        assert_eq!(value, &None, "{field} is not stated");
    }
    assert_eq!(info.fs_type, Some(0), "0 is Installable, not 'not stated'");
}

/// Criterion 2: a face without an OS/2 table states no embedding permission and no vendor
/// code (it is "Not stated", not "Installable").
#[test]
fn ac02_no_os2_table() {
    let names: Vec<_> = all_names()
        .into_iter()
        .filter(|(id, _, _)| *id != 8)
        .collect();
    let font = sfnt_edit::with_table(
        &font_bytes(SERIF),
        *b"name",
        Some(sfnt_edit::name_table(&names)),
    );
    let font = sfnt_edit::with_table(&font, *b"OS/2", None);
    let mut store = FaceStore::new();
    if let Ok(key) = store.add(font, 0) {
        let info = font_info(store.get(&key).unwrap());
        assert_eq!(info.fs_type, None);
        assert_eq!(info.vendor, None);
    }
}

/// Criterion 2: Glyphs is the `maxp` count; Format of CFF faces and of a collection's second
/// face of three ("face 2 of 3").
#[test]
fn ac02_glyphs_and_formats() {
    let serif = font_bytes(SERIF);
    let sans = font_bytes(SANS);
    assert_eq!(
        info_of(serif.clone(), 0).glyphs,
        sfnt_edit::maxp_glyphs(&serif)
    );
    let sans_info = info_of(sans.clone(), 0);
    assert_eq!(sans_info.format, FontFormat::OpenTypeCff);
    assert_eq!(sans_info.glyphs, sfnt_edit::maxp_glyphs(&sans));

    let ttc = sfnt_edit::collection(&[serif.clone(), sans.clone(), serif.clone()]);
    let second = info_of(ttc.clone(), 1);
    assert_eq!(second.format, FontFormat::Collection { face: 2, faces: 3 });
    assert_eq!(second.glyphs, sfnt_edit::maxp_glyphs(&sans));
    assert_eq!(
        info_of(ttc, 2).format,
        FontFormat::Collection { face: 3, faces: 3 }
    );
}

/// Criterion 2: an SVG font is "SVG font, single-line", its glyph count the number of
/// `<glyph>` elements, and no permission or name field.
#[test]
fn ac02_svg_font_values() {
    let path = common::fonts_dir().join("hershey-script-1/HersheyScript1.svg");
    let svg = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&svg);
    let glyphs = text.matches("<glyph ").count() + text.matches("<glyph>").count();
    let info = info_of(svg, 0);
    assert_eq!(info.format, FontFormat::SvgFont);
    assert_eq!(info.glyphs as usize, glyphs);
    assert_eq!(info.fs_type, None);
    assert_eq!(info.licence, None);
}

/// Criterion 5: the fields are a pure read of the held face: the same face gives the same
/// fields every time (no second file read can change them).
#[test]
fn ac05_fields_read_from_the_held_face() {
    let mut store = FaceStore::new();
    let key = store.add(font_bytes(SERIF), 0).unwrap();
    let face = store.get(&key).unwrap();
    let first = font_info(face);
    for _ in 0..100 {
        assert_eq!(font_info(face), first);
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 19: the core reads no file
// ---------------------------------------------------------------------------------------------

/// Criterion 19: the archive reader and the info fields use no file, clock or thread API.
#[test]
fn ac19_core_modules_read_no_file() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for module in [
        "font_archive.rs",
        "archive_names.rs",
        "font_info.rs",
        "limits.rs",
    ] {
        let whole = std::fs::read_to_string(src.join(module)).unwrap();
        let text = whole.split("#[cfg(test)]").next().unwrap();
        for banned in [
            "std::fs",
            "std::net",
            "std::thread",
            "std::time",
            "std::env",
            "File::",
            "Path::new",
        ] {
            assert!(!text.contains(banned), "{module} uses {banned}");
        }
    }
}
