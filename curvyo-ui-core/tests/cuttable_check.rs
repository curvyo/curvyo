//! The cuttable check (`specs/0075-font-library` criteria 14 and 17): a connected-script face is
//! Cuttable, faces whose letters stand apart and a face without the letters are not, and the
//! shipped piece counts of the included fonts are what the check gives.
#![allow(clippy::unwrap_used, missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use curvyo_ui_core::{BuiltInTag, cuttable_pieces};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn bytes(path: &str) -> Vec<u8> {
    std::fs::read(fonts_dir().join(path)).unwrap()
}

/// The directory record of table `tag` in a single-face file.
fn record(bytes: &[u8], tag: [u8; 4]) -> usize {
    let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    (0..tables)
        .map(|r| 12 + 16 * r)
        .find(|&at| bytes[at..at + 4] == tag)
        .unwrap()
}

fn table(bytes: &[u8], tag: [u8; 4]) -> usize {
    let at = record(bytes, tag);
    u32::from_be_bytes(bytes[at + 8..at + 12].try_into().unwrap()) as usize
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

/// Great Vibes with every advance at 80 %: its letters overlap, a connected-script fixture face.
fn connected_script() -> Vec<u8> {
    let mut out = bytes("great-vibes/GreatVibes-Regular.ttf");
    let metrics = usize::from(u16_at(&out, table(&out, *b"hhea") + 34));
    let hmtx = table(&out, *b"hmtx");
    for glyph in 0..metrics {
        let at = hmtx + 4 * glyph;
        let advance = u32::from(u16_at(&out, at)) * 80 / 100;
        out[at..at + 2].copy_from_slice(&u16::try_from(advance).unwrap().to_be_bytes());
    }
    out
}

/// Source Sans 3 without its `cmap`: every letter is drawn as `.notdef`.
fn without_letters() -> Vec<u8> {
    let mut out = bytes("source-sans-3/SourceSans3-Regular.otf");
    let at = record(&out, *b"cmap");
    out[at..at + 4].copy_from_slice(b"cmaq");
    out
}

/// Criterion 17: a connected script is one piece; Source Sans 3, Source Serif 4 and a
/// face without lowercase letters do not; bytes that are no font are an error.
#[test]
fn ac17_connected_script_is_cuttable_and_separate_letters_are_not() {
    assert_eq!(cuttable_pieces(&connected_script(), 0).unwrap(), 1);
    assert!(cuttable_pieces(&bytes("source-sans-3/SourceSans3-Regular.otf"), 0).unwrap() > 1);
    assert!(cuttable_pieces(&bytes("source-serif-4/SourceSerif4-Regular.ttf"), 0).unwrap() > 1);
    assert_ne!(cuttable_pieces(&without_letters(), 0).unwrap(), 1);
    assert!(cuttable_pieces(b"not a font", 0).is_err());
}

/// Criteria 14 and 17: every shipped entry names built-in tags, and its piece count is what
/// the check gives for the family's Regular face.
#[test]
fn ac14_ac17_shipped_tags_and_counts_match_the_check() {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"].as_array().unwrap();
    for family in index["families"].as_array().unwrap() {
        for tag in family["tags"].as_array().unwrap() {
            assert!(BuiltInTag::from_id(tag.as_str().unwrap()).is_some());
        }
        let regular = faces
            .iter()
            .find(|f| f["family"] == family["family"] && f["style"] == "Regular")
            .unwrap();
        let pieces = cuttable_pieces(&bytes(regular["file"].as_str().unwrap()), 0).unwrap();
        assert_eq!(
            family["cuttable_pieces"].as_u64().unwrap(),
            pieces as u64,
            "{}",
            family["family"]
        );
    }
}
