//! Independent tester cases for `specs/0082-glyph-table/specification.md` on the table's
//! grouping and search: criteria 6 (block groups with counts, code point order, no empty
//! header) and 16 (query forms, exact matches first, no match), with the shipped names file.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read; the "No block" and query-edge cases at the end were added after reading the diff.

#![allow(clippy::unwrap_used, missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use curvyo_text_core::{ListedChar, UnicodeNames};
use curvyo_ui_core::{GlyphTable, NO_BLOCK, glyph_table};

fn names() -> UnicodeNames {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/unicode/names-17.0.0.bin"),
    )
    .unwrap();
    UnicodeNames::parse(&bytes).unwrap()
}

/// A face's list: the characters in code point order, with optional glyph names.
fn list(entries: &[(char, Option<&str>)]) -> Vec<ListedChar> {
    let mut v: Vec<ListedChar> = entries
        .iter()
        .map(|(c, name)| ListedChar {
            c: *c,
            glyph: u32::from(*c) % 60_000 + 1,
            name: name.map(Into::into),
        })
        .collect();
    v.sort_by_key(|l| l.c);
    v
}

fn plain(chars: &str) -> Vec<ListedChar> {
    let entries: Vec<(char, Option<&str>)> = chars.chars().map(|c| (c, None)).collect();
    list(&entries)
}

/// The table as (block, characters) in table order.
fn shown(list: &[ListedChar], table: &GlyphTable) -> Vec<(&'static str, String)> {
    table
        .groups
        .iter()
        .map(|g| (g.name, g.cells.iter().map(|&i| list[i].c).collect()))
        .collect()
}

fn matched(list: &[ListedChar], query: &str, names: &UnicodeNames) -> String {
    let table = glyph_table(list, query, Some(names));
    let chars: String = table
        .groups
        .iter()
        .flat_map(|g| g.cells.iter().map(|&i| list[i].c))
        .collect();
    assert_eq!(table.shown, chars.chars().count(), "shown counts the cells");
    chars
}

const FACE: &str = "Aaäöü€—→⇄♥❤\u{E000}";

// --- Criterion 6 ----------------------------------------------------------------------------

/// Criterion 6: blank query, every character under its block, blocks in code point order, each
/// header's count is the number of its cells, no header without a cell.
#[test]
fn ac06_groups_by_block_in_code_point_order_with_counts() {
    let l = plain(FACE);
    let table = glyph_table(&l, "", None);
    assert_eq!(
        shown(&l, &table),
        vec![
            ("Basic Latin", "Aa".to_owned()),
            ("Latin-1 Supplement", "äöü".to_owned()),
            ("General Punctuation", "—".to_owned()),
            ("Currency Symbols", "€".to_owned()),
            ("Arrows", "→⇄".to_owned()),
            ("Miscellaneous Symbols", "♥".to_owned()),
            ("Dingbats", "❤".to_owned()),
            ("Private Use Area", "\u{E000}".to_owned()),
        ]
    );
    assert_eq!(table.shown, l.len());
    assert!(table.groups.iter().all(|g| !g.cells.is_empty()));
    // Whitespace only is a blank query too.
    assert_eq!(glyph_table(&l, "   ", None), table);
}

// --- Criterion 16 ---------------------------------------------------------------------------

/// Criterion 16: the spec's query forms: one character, `U+00E4`, `u+e4`, 4 to 6 hex digits,
/// character name words ignoring case, glyph name, block name.
#[test]
fn ac16_each_query_form_finds_its_characters() {
    let n = names();
    let l = plain(FACE);
    assert_eq!(matched(&l, "♥", &n), "♥");
    assert_eq!(matched(&l, "U+00E4", &n), "ä");
    assert_eq!(matched(&l, "u+e4", &n), "ä");
    assert_eq!(matched(&l, "00E4", &n), "ä");
    assert_eq!(matched(&l, "0000e4", &n), "ä");
    assert_eq!(matched(&l, "U+2665", &n), "♥");
    // Names: "heart" finds U+2665 and U+2764; "a diaeresis" finds ä (Ä is not in this face);
    // case does not matter.
    assert_eq!(matched(&l, "heart", &n), "♥❤");
    assert_eq!(matched(&l, "HeArT", &n), "♥❤");
    let with_capital = plain("AaÄäöü");
    let found = matched(&with_capital, "a diaeresis", &n);
    assert!(found.contains('ä') && found.contains('Ä'), "{found}");
    assert!(!found.contains('ö') && !found.contains('ü'), "{found}");
    // Block names: "arrows" lists the Arrows block.
    let arrows = matched(&l, "arrows", &n);
    assert!(arrows.contains('→') && arrows.contains('⇄'), "{arrows}");
    assert!(!arrows.contains('A') && !arrows.contains('♥'), "{arrows}");
    // Glyph names (post, SVG glyph-name), by substring.
    let named = list(&[
        ('ä', Some("adieresis")),
        ('\u{E000}', Some("swash.A")),
        ('A', None),
    ]);
    assert_eq!(matched(&named, "dieres", &n), "ä");
    assert_eq!(matched(&named, "swash", &n), "\u{E000}");
}

/// Criterion 16: exact character and code point matches come first, then the rest in code
/// point order, still under their block headers.
#[test]
fn ac16_exact_matches_come_first() {
    let n = names();
    // "2665" is ♥'s code point and part of the glyph name of "A" and "B".
    let l = list(&[
        ('A', Some("A.uni2665")),
        ('B', Some("B.uni2665")),
        ('\u{2665}', None),
        ('\u{2764}', Some("heart.2665")),
    ]);
    let table = glyph_table(&l, "2665", Some(&n));
    assert_eq!(
        shown(&l, &table),
        vec![
            ("Miscellaneous Symbols", "♥".to_owned()),
            ("Basic Latin", "AB".to_owned()),
            ("Dingbats", "❤".to_owned()),
        ]
    );
}

/// Criterion 16: no match gives an empty table (the frontend's "No character in ... matches").
#[test]
fn ac16_no_match_is_an_empty_table() {
    let n = names();
    let l = plain(FACE);
    let table = glyph_table(&l, "xyz", Some(&n));
    assert_eq!(table.groups, Vec::new());
    assert_eq!(table.shown, 0);
}

/// Criterion 16 / 17: before the names file is loaded, character, code point, glyph name and
/// block name still match; names only after.
#[test]
fn ac16_without_names_the_other_forms_still_match() {
    let l = list(&[('ä', Some("adieresis")), ('♥', None), ('→', None)]);
    let count = |q: &str| glyph_table(&l, q, None).shown;
    assert_eq!(count("ä"), 1);
    assert_eq!(count("U+2665"), 1);
    assert_eq!(count("adier"), 1);
    assert_eq!(count("arrows"), 1);
    assert_eq!(count("heart"), 0);
    assert_eq!(glyph_table(&l, "heart", Some(&names())).shown, 1);
}

// --- After reading the diff -----------------------------------------------------------------

/// `adrs.md` decision 2: a code point in no block goes into one last group "No block". Two such
/// characters on both sides of a block must not give two "No block" headers.
#[test]
fn edge_characters_in_no_block_form_one_last_group() {
    let l = plain("\u{2FE0}\u{3000}\u{E0080}");
    let table = glyph_table(&l, "", None);
    let headers: Vec<&str> = table.groups.iter().map(|g| g.name).collect();
    assert_eq!(headers, vec!["CJK Symbols and Punctuation", NO_BLOCK]);
    assert_eq!(table.groups[1].cells.len(), 2);
}

/// Degenerate input: an empty list, a query of 64 characters and one of 10,000 characters give
/// a table without a panic; a query that is not valid hex after "U+" is not a code point.
#[test]
fn edge_degenerate_queries() {
    let n = names();
    assert_eq!(glyph_table(&[], "heart", Some(&n)), GlyphTable::default());
    let l = plain(FACE);
    let _ = glyph_table(&l, &"x".repeat(64), Some(&n));
    let _ = glyph_table(&l, &"ab ".repeat(3_400), Some(&n));
    assert_eq!(glyph_table(&l, "U+ZZZZ", Some(&n)).shown, 0);
    assert_eq!(glyph_table(&l, "U+110000", Some(&n)).shown, 0);
    assert_eq!(glyph_table(&l, "U+", Some(&n)).shown, 0);
}
