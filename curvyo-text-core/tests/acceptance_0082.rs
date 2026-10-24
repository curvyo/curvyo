//! Independent tester cases for `specs/0082-glyph-table/specification.md` on the text engine:
//! criteria 5 (exactly the mapped characters, glyph 0 excluded), 6 (the Unicode 17 block table),
//! 7 (SVG single-line fonts), 8 (cells in cap-height units, empty outlines), 9 (one check with the
//! missing-glyph marker), 16 and 17 (names decoded from a file written in the generator's format,
//! the shipped file checked against `UnicodeData.txt`), 21 (the 65,535-glyph limit).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read; the hostile and degenerate cases at the end were added after reading the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use common::{font_bytes, sfnt};
use curvyo_text_core::{
    BLOCKS, Face, FaceKind, FaceStore, UNICODE_VERSION, UnicodeNames, block_of, glyph_cell,
};
use skrifa::MetadataProvider;

fn face_of(bytes: Vec<u8>) -> Arc<Face> {
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    Arc::clone(store.get(&key).unwrap())
}

fn listed(face: &Face) -> Vec<char> {
    face.listed_chars().iter().map(|l| l.c).collect()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/svg_fonts")
            .join(name),
    )
    .unwrap()
}

/// The spec's fixture face of criterion 5, plus "B" mapped explicitly to glyph 0 and a tab
/// (Cc) mapped to a drawn glyph; "ü" has a glyph but no code.
fn spec_face() -> Vec<u8> {
    sfnt::build(&sfnt::TestFace {
        glyphs: vec![
            ("space", false),    // 1
            ("A", true),         // 2
            ("adieresis", true), // 3
            ("heart", true),     // 4
            ("uniE000", true),   // 5
            ("udieresis", true), // 6, unmapped
            ("tab", true),       // 7
        ],
        subtables: vec![sfnt::Subtable {
            platform: 3,
            encoding: 1,
            format: 4,
            map: vec![
                (0x09, 7),
                (0x20, 1),
                (0x41, 2),
                (0x42, 0),
                (0xE4, 3),
                (0x2665, 4),
                (0xE000, 5),
            ],
        }],
        post_names: true,
    })
}

/// The y coordinates of an absolute path string of the `path_data` writer (pairs after M, L,
/// Q, C).
fn ys(d: &str) -> Vec<f64> {
    let numbers: Vec<f64> = d
        .split(|c: char| c.is_ascii_alphabetic() || c == ' ' || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| t.parse().unwrap())
        .collect();
    numbers.chunks(2).map(|p| p[1]).collect()
}

// --- Criterion 5 ----------------------------------------------------------------------------

/// Criterion 5: the fixture face lists "A", "ä", "♥", U+E000 and its space, in code point
/// order; not "ü" (unmapped), not "B" (mapped to glyph 0), not the tab (Cc).
#[test]
fn ac05_exactly_the_characters_mapped_to_a_real_glyph() {
    let face = face_of(spec_face());
    assert_eq!(listed(&face), vec![' ', 'A', 'ä', '\u{2665}', '\u{E000}']);
}

/// Criterion 5 on a shipped script font: the list is the face's `cmap` (as `skrifa` reads it)
/// minus glyph 0 and Cc, in code point order, nothing added.
#[test]
fn ac05_great_vibes_lists_its_cmap() {
    let bytes = font_bytes("great-vibes/GreatVibes-Regular.ttf");
    let font = skrifa::FontRef::new(&bytes).unwrap();
    let expected: Vec<char> = font
        .charmap()
        .mappings()
        .filter(|(_, g)| g.to_u32() != 0)
        .filter_map(|(c, _)| char::from_u32(c))
        .filter(|c| !c.is_control())
        .collect();
    let face = face_of(bytes.clone());
    let got = listed(&face);
    assert!(got.windows(2).all(|w| w[0] < w[1]), "code point order");
    assert_eq!(got, expected);
}

// --- Criterion 6 ----------------------------------------------------------------------------

/// Criterion 6: one stated Unicode version, blocks sorted and disjoint, every block name unique.
#[test]
fn ac06_block_table_is_ordered_disjoint_and_versioned() {
    assert_eq!(UNICODE_VERSION, "17.0.0");
    assert!(BLOCKS.len() > 330, "{}", BLOCKS.len());
    for w in BLOCKS.windows(2) {
        assert!(w[0].0 <= w[0].1 && w[0].1 < w[1].0, "{w:?}");
    }
    let mut names: Vec<&str> = BLOCKS.iter().map(|b| b.2).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), BLOCKS.len());
    // The spec's examples, an unassigned code point inside a block, and gaps between blocks.
    assert_eq!(block_of('\u{2665}'), Some("Miscellaneous Symbols"));
    assert_eq!(block_of('\u{2014}'), Some("General Punctuation"));
    assert_eq!(block_of('\u{0378}'), Some("Greek and Coptic"));
    assert_eq!(block_of('\u{2FE0}'), None);
    assert_eq!(block_of('\u{E0080}'), None);
    assert_eq!(
        block_of('\u{F0000}'),
        Some("Supplementary Private Use Area-A")
    );
}

/// Criterion 6 against the source: the table is `Blocks.txt` of Unicode 17.0.0 line for line.
/// Needs the file: `CURVYO_UCD_DIR=<folder> cargo test -p curvyo-text-core --test
/// acceptance_0082 -- --ignored`.
#[test]
#[ignore = "needs CURVYO_UCD_DIR with the Unicode 17.0.0 UCD files"]
fn ac06_blocks_equal_blocks_txt() {
    let dir = PathBuf::from(std::env::var("CURVYO_UCD_DIR").unwrap());
    let text = std::fs::read_to_string(dir.join("Blocks.txt")).unwrap();
    let expected: Vec<(u32, u32, String)> = text
        .lines()
        .map(|l| l.split('#').next().unwrap().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (range, name) = l.split_once(';').unwrap();
            let (a, b) = range.split_once("..").unwrap();
            (
                u32::from_str_radix(a, 16).unwrap(),
                u32::from_str_radix(b, 16).unwrap(),
                name.trim().to_owned(),
            )
        })
        .collect();
    let got: Vec<(u32, u32, String)> = BLOCKS
        .iter()
        .map(|(a, b, n)| (*a, *b, (*n).to_owned()))
        .collect();
    assert_eq!(got, expected);
}

// --- Criterion 7 ----------------------------------------------------------------------------

/// Criterion 7: an SVG font lists its one-scalar glyphs (the PUA one too) and nothing from
/// `<missing-glyph>`, a glyph without `unicode` or a multi-character `unicode`; it is a
/// single-line face, and its cells are open paths.
#[test]
fn ac07_svg_font_lists_single_scalar_glyphs_only() {
    let face = face_of(fixture("glyph-table.svg"));
    assert_eq!(face.kind(), FaceKind::SingleLine);
    assert_eq!(listed(&face), vec![' ', 'H', 'L', '\u{E001}']);
    let list = face.listed_chars();
    let names: Vec<Option<&str>> = list.iter().map(|l| l.name.as_deref()).collect();
    assert_eq!(
        names,
        vec![Some("space"), Some("H"), None, Some("swash.one")]
    );
    let l = glyph_cell(&face, 'L').unwrap();
    assert!(!l.empty);
    assert!(
        !l.d.contains('Z') && !l.d.contains('z'),
        "open centre line: {}",
        l.d
    );
    assert!(glyph_cell(&face, ' ').unwrap().empty);
}

// --- Criterion 8 ----------------------------------------------------------------------------

/// Criterion 8: a cell's path is in cap-height units on a common baseline: "H" of Source Sans
/// spans 0 to 1 cap height; "x" ends below it; the advance is positive.
#[test]
fn ac08_cells_are_in_cap_height_units() {
    let face = face_of(font_bytes(common::SANS));
    let h = glyph_cell(&face, 'H').unwrap();
    let y = ys(&h.d);
    let (lo, hi) = y
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    assert!((hi - lo - 1.0).abs() < 0.02, "H spans {lo}..{hi}");
    assert!(
        lo.abs() < 0.02 || hi.abs() < 0.02,
        "baseline at 0: {lo}..{hi}"
    );
    assert!(h.advance > 0.3 && h.advance < 2.0, "{}", h.advance);
    let x = glyph_cell(&face, 'x').unwrap();
    let span = ys(&x.d).iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    assert!(span < 0.9, "x-height below cap height: {span}");
}

/// Criterion 8: space, no-break space and zero-width characters are empty cells; a drawn
/// character is not.
#[test]
fn ac08_empty_outlines_are_flagged() {
    let face = face_of(font_bytes(common::SANS));
    for c in [' ', '\u{A0}'] {
        if face.glyph_for(c) != 0 {
            assert!(glyph_cell(&face, c).unwrap().empty, "{c:?}");
        }
    }
    for c in ['\u{200B}', '\u{200D}'] {
        if let Some(cell) = glyph_cell(&face, c) {
            assert!(cell.empty, "{c:?}");
        }
    }
    assert!(!glyph_cell(&face, 'A').unwrap().empty);
}

// --- Criterion 9 ----------------------------------------------------------------------------

/// Criterion 9: the sample of all listed characters has no missing character; each printable
/// ASCII character not listed is reported as missing, itself (spec fixture and SVG fixture).
#[test]
fn ac09_listed_and_missing_are_one_check() {
    for face in [face_of(spec_face()), face_of(fixture("glyph-table.svg"))] {
        let list = listed(&face);
        let sample: String = list.iter().collect();
        assert!(face.missing_chars(&sample).is_empty(), "{sample:?}");
        for c in (0x20_u8..0x7F)
            .map(char::from)
            .filter(|c| !list.contains(c))
        {
            assert_eq!(face.missing_chars(&c.to_string()), vec![c], "{c:?}");
        }
    }
}

// --- Criteria 16, 17: the names file --------------------------------------------------------

fn varint(mut v: u32, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn text(s: &str, out: &mut Vec<u8>) {
    varint(s.len() as u32, out);
    out.extend_from_slice(s.as_bytes());
}

/// A names file written by hand in the format of `tools/unicode-data/generate.py`
/// (`names_file`): magic, format 1, version, words, NR2 ranges, names as code point deltas and
/// word indices, abbreviations as deltas and text.
fn small_names_file() -> Vec<u8> {
    let words = [
        "LETTER",
        "LATIN",
        "SMALL",
        "A",
        "WITH",
        "DIAERESIS",
        "BLACK",
        "HEART",
        "SUIT",
    ];
    let mut out = b"CVYN".to_vec();
    out.extend_from_slice(&[1, 17, 0, 0]);
    varint(words.len() as u32, &mut out);
    for w in words {
        text(w, &mut out);
    }
    varint(1, &mut out); // one range
    varint(0x4E00, &mut out);
    varint(0x9FFF - 0x4E00, &mut out);
    text("CJK UNIFIED IDEOGRAPH-", &mut out);
    let names: [(u32, &[u32]); 3] = [
        (0x61, &[1, 2, 0, 3]),
        (0xE4, &[1, 2, 0, 3, 4, 5]),
        (0x2665, &[6, 7, 8]),
    ];
    varint(names.len() as u32, &mut out);
    let mut last = 0;
    for (code, parts) in names {
        varint(code - last, &mut out);
        varint(parts.len() as u32, &mut out);
        for p in parts {
            varint(*p, &mut out);
        }
        last = code;
    }
    varint(2, &mut out);
    varint(0x20, &mut out);
    text("SP", &mut out);
    varint(0xA0 - 0x20, &mut out);
    text("NBSP", &mut out);
    out
}

/// Criterion 16 / 17: a file in the generator's format decodes to the names, the computed
/// ranges (NR1 Hangul by rule, NR2 by prefix) and the abbreviations; a character without a
/// stored or computed name has none.
#[test]
fn ac17_a_file_in_the_generator_format_decodes() {
    let names = UnicodeNames::parse(&small_names_file()).unwrap();
    assert_eq!(names.version(), (17, 0, 0));
    assert_eq!(names.name_of('a').as_deref(), Some("LATIN SMALL LETTER A"));
    assert_eq!(
        names.name_of('ä').as_deref(),
        Some("LATIN SMALL LETTER A WITH DIAERESIS")
    );
    assert_eq!(
        names.name_of('\u{2665}').as_deref(),
        Some("BLACK HEART SUIT")
    );
    assert_eq!(
        names.name_of('\u{4E00}').as_deref(),
        Some("CJK UNIFIED IDEOGRAPH-4E00")
    );
    assert_eq!(
        names.name_of('\u{9FFF}').as_deref(),
        Some("CJK UNIFIED IDEOGRAPH-9FFF")
    );
    assert_eq!(names.name_of('\u{A000}'), None);
    assert_eq!(
        names.name_of('\u{AC00}').as_deref(),
        Some("HANGUL SYLLABLE GA")
    );
    assert_eq!(
        names.name_of('\u{D7A3}').as_deref(),
        Some("HANGUL SYLLABLE HIH")
    );
    assert_eq!(names.name_of('b'), None);
    assert_eq!(names.abbreviation_of(' '), Some("SP"));
    assert_eq!(names.abbreviation_of('\u{A0}'), Some("NBSP"));
    assert_eq!(names.abbreviation_of('a'), None);
}

/// Criterion 17 (hostile input): every prefix of a valid file and every one-byte change of it
/// gives a result, never a panic; every strict prefix is an error.
#[test]
fn ac17_damaged_small_files_never_panic() {
    let bytes = small_names_file();
    for cut in 0..bytes.len() {
        assert!(
            UnicodeNames::parse(&bytes[..cut]).is_err(),
            "prefix of {cut} bytes"
        );
    }
    for at in 0..bytes.len() {
        for value in [0x00, 0x7F, 0x80, 0xFF] {
            let mut changed = bytes.clone();
            changed[at] = value;
            let _ = UnicodeNames::parse(&changed).map(|n| {
                let _ = n.name_of('ä');
                let _ = n.name_of('\u{4E00}');
                let _ = n.abbreviation_of(' ');
            });
        }
    }
}

/// Criterion 16 / 17 against the source: every code point's name in the shipped file is the
/// name `UnicodeData.txt` of Unicode 17.0.0 gives (explicit and `<..., First>` ranges, Hangul
/// by rule NR1), and no other code point has one. Needs the file:
/// `CURVYO_UCD_DIR=<folder> cargo test -p curvyo-text-core --test acceptance_0082 -- --ignored`.
#[test]
#[ignore = "needs CURVYO_UCD_DIR with the Unicode 17.0.0 UCD files"]
fn ac17_shipped_names_equal_unicode_data() {
    let dir = PathBuf::from(std::env::var("CURVYO_UCD_DIR").unwrap());
    let data = std::fs::read_to_string(dir.join("UnicodeData.txt")).unwrap();
    let mut expected: BTreeMap<u32, String> = BTreeMap::new();
    let mut first: Option<(u32, String)> = None;
    for line in data.lines() {
        let f: Vec<&str> = line.split(';').collect();
        let code = u32::from_str_radix(f[0], 16).unwrap();
        let name = f[1];
        if let Some(label) = name.strip_suffix(", First>") {
            first = Some((code, label[1..].to_owned()));
        } else if name.ends_with(", Last>") {
            let (start, label) = first.take().unwrap();
            for c in start..=code {
                let n = if label.starts_with("CJK Ideograph") {
                    Some(format!("CJK UNIFIED IDEOGRAPH-{c:04X}"))
                } else if label.starts_with("Tangut Ideograph") {
                    Some(format!("TANGUT IDEOGRAPH-{c:04X}"))
                } else {
                    None // Hangul syllables are checked by rule NR1 below
                };
                if let Some(n) = n {
                    expected.insert(c, n);
                }
            }
        } else if !name.starts_with('<') {
            expected.insert(code, name.to_owned());
        }
    }
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/unicode/names-17.0.0.bin"),
    )
    .unwrap();
    let names = UnicodeNames::parse(&bytes).unwrap();
    let mut wrong = Vec::new();
    for code in 0..=0x10_FFFF_u32 {
        let Some(c) = char::from_u32(code) else {
            continue;
        };
        let got = names.name_of(c);
        if (0xAC00..=0xD7A3).contains(&code) {
            assert!(
                got.as_deref().unwrap().starts_with("HANGUL SYLLABLE "),
                "{code:X}"
            );
            continue;
        }
        if got.as_deref() != expected.get(&code).map(String::as_str) {
            wrong.push((code, got, expected.get(&code).cloned()));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong, first {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
    assert_eq!(
        names.name_of('\u{AC01}').as_deref(),
        Some("HANGUL SYLLABLE GAG")
    );
    assert_eq!(
        names.name_of('\u{D4DB}').as_deref(),
        Some("HANGUL SYLLABLE PWILH")
    );
}

// --- Criterion 21: the 65,535-glyph limit ---------------------------------------------------

/// Criterion 21: a face at the glyph limit (65,535 glyphs, 65,534 mapped characters across the
/// BMP and plane 2) lists every mapped character once, in order; release budget for listing a
/// CJK-size face: 150 ms (`adrs.md` decision 3 states it for 20,000; the limit gets the same).
#[test]
fn ac21_a_face_at_the_glyph_limit_lists_all_its_characters() {
    let count: u32 = 65_534;
    let codes: Vec<u32> = (0..count)
        .map(|i| if i < 30_000 { 0x4E00 + i } else { 0x2_0000 + i })
        .collect();
    let codes: Vec<u32> = codes
        .into_iter()
        .filter(|c| char::from_u32(*c).is_some())
        .collect();
    let bytes = sfnt::build(&sfnt::TestFace {
        glyphs: codes.iter().map(|_| ("g", true)).collect(),
        subtables: vec![sfnt::Subtable {
            platform: 3,
            encoding: 10,
            format: 12,
            map: codes
                .iter()
                .enumerate()
                .map(|(i, c)| (*c, u16::try_from(i + 1).unwrap()))
                .collect(),
        }],
        post_names: false,
    });
    let face = face_of(bytes);
    let started = Instant::now();
    let list = face.listed_chars();
    let took = started.elapsed();
    println!("listing {} characters: {took:?}", list.len());
    let got: Vec<u32> = list.iter().map(|l| u32::from(l.c)).collect();
    assert_eq!(got, codes);
    if !cfg!(debug_assertions) && std::env::var_os("CI").is_none() {
        assert!(took.as_millis() < 150, "{took:?}");
    }
}

// --- After reading the diff: degenerate input -----------------------------------------------

/// Hostile input: an empty or garbage names file is an error; a file with zero words, ranges,
/// names and abbreviations is valid and names nothing but Hangul (computed by rule).
#[test]
fn edge_empty_names_file() {
    assert!(UnicodeNames::parse(&[]).is_err());
    assert!(UnicodeNames::parse(b"not a names file").is_err());
    let names = UnicodeNames::parse(b"CVYN\x01\x11\x00\x00\x00\x00\x00\x00").unwrap();
    assert_eq!(names.name_of('a'), None);
    assert_eq!(names.abbreviation_of(' '), None);
    assert_eq!(
        names.name_of('\u{AC00}').as_deref(),
        Some("HANGUL SYLLABLE GA")
    );
}

/// Degenerate face: a face whose only mappings are "A" to glyph 0 and a drawn newline (Cc) lists
/// nothing (the "no characters to insert" state of criterion 4).
#[test]
fn edge_a_face_with_nothing_to_insert_lists_nothing() {
    let face = face_of(sfnt::build(&sfnt::TestFace {
        glyphs: vec![("A", true)],
        subtables: vec![sfnt::Subtable {
            platform: 3,
            encoding: 1,
            format: 4,
            map: vec![(0x41, 0), (0x0A, 1)],
        }],
        post_names: false,
    }));
    assert!(face.listed_chars().is_empty());
}
