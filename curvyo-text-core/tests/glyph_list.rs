//! The characters a face lists in the glyph table (`specs/0082-glyph-table` criteria 5, 7, 9 and
//! 23; `adrs.md` decision 1): exactly those the missing-glyph check reports as present.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::{SANS, font_bytes, sfnt};
use curvyo_text_core::{Face, FaceStore};

fn face_of(bytes: Vec<u8>) -> Arc<Face> {
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn names(face: &Face) -> Vec<Option<String>> {
    face.listed_chars()
        .iter()
        .map(|l| l.name.as_deref().map(str::to_owned))
        .collect()
}

fn some(names: &[&str]) -> Vec<Option<String>> {
    names.iter().map(|n| Some((*n).to_owned())).collect()
}

fn listed(face: &Face) -> Vec<char> {
    face.listed_chars().iter().map(|l| l.c).collect()
}

/// Every scalar value is listed exactly when the face has a glyph other than 0 for it and it is
/// not a control character; the missing check says the same.
fn assert_agrees(face: &Face) {
    let list = face.listed_chars();
    assert!(list.windows(2).all(|w| w[0].c < w[1].c), "code point order");
    let mut at = 0;
    for c in (0..=0x10_FFFF_u32).filter_map(char::from_u32) {
        let present = face.glyph_for(c) != 0 && !c.is_control();
        let is_listed = list.get(at).is_some_and(|l| l.c == c);
        assert_eq!(is_listed, present, "U+{:04X}", u32::from(c));
        if is_listed {
            assert_eq!(list[at].glyph, face.glyph_for(c));
            at += 1;
        }
        if !c.is_whitespace() && !c.is_control() {
            let missing = face.missing_chars(&c.to_string());
            assert_eq!(
                missing.is_empty(),
                present,
                "missing check, U+{:04X}",
                u32::from(c)
            );
        }
    }
    assert_eq!(at, list.len());
}

#[test]
fn the_four_character_face_lists_exactly_its_mapped_characters() {
    let face = face_of(sfnt::four_char_face());
    assert_eq!(listed(&face), vec![' ', 'A', 'ä', '♥', '\u{E000}']);
    assert_eq!(
        names(&face),
        some(&["space", "A", "adieresis", "heart", "uniE000"])
    );
    assert_eq!(face.glyph_for('ü'), 0);
    assert_eq!(face.missing_chars("Aäü♥"), vec!['ü']);
    assert_agrees(&face);
}

#[test]
fn a_symbol_face_lists_its_codes_and_their_low_aliases() {
    let face = face_of(sfnt::symbol_face());
    assert_eq!(
        listed(&face),
        vec!['A', '©', '\u{F041}', '\u{F0A9}', '\u{F100}'],
        "U+F041 is also reached as A, U+F0A9 as ©; U+F100 has no alias"
    );
    assert!(
        face.listed_chars().iter().all(|l| l.name.is_none()),
        "no post table"
    );
    assert_agrees(&face);
}

#[test]
fn a_character_only_in_the_subtable_the_shaper_does_not_use_is_not_listed() {
    let face = face_of(sfnt::two_subtable_face());
    let shown = listed(&face);
    assert!(shown.contains(&'B'));
    assert_agrees(&face);
}

#[test]
fn source_sans_lists_its_whole_cmap_without_control_characters() {
    let face = face_of(font_bytes(SANS));
    let list = face.listed_chars();
    assert!(list.len() > 1000, "{}", list.len());
    assert!(list.iter().all(|l| !l.c.is_control()));
    assert!(list.iter().any(|l| l.name.as_deref() == Some("adieresis")));
    assert_agrees(&face);
}

#[test]
fn every_included_face_agrees_with_the_missing_check() {
    for path in [
        "great-vibes/GreatVibes-Regular.ttf",
        "source-serif-4/SourceSerif4-Regular.ttf",
        "hershey-script-1/HersheyScript1.svg",
        "ems-readability/EMSReadability.svg",
    ] {
        assert_agrees(&face_of(font_bytes(path)));
    }
}

#[test]
fn an_svg_font_lists_its_single_character_glyphs_with_their_names() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font horiz-adv-x="500">
        <font-face font-family="T" units-per-em="1000"/>
        <missing-glyph d="M0 0L1 1"/>
        <glyph unicode="A" glyph-name="A" d="M0 0L100 700"/>
        <glyph unicode="&#x2665;" glyph-name="heart" d="M0 0L100 100"/>
        <glyph unicode="fi" glyph-name="f_i" d="M0 0L10 10"/>
        <glyph glyph-name="orphan" d="M0 0L10 10"/>
        <glyph unicode=" " glyph-name="space"/>
        <glyph unicode="A" glyph-name="A.alt" d="M0 0L5 5"/>
    </font></defs></svg>"#;
    let face = face_of(svg.to_vec());
    assert_eq!(listed(&face), vec![' ', 'A', '♥']);
    assert_eq!(names(&face), some(&["space", "A", "heart"]));
    assert_agrees(&face);
}

#[test]
fn the_list_is_built_once_per_face() {
    let face = face_of(sfnt::four_char_face());
    assert!(Arc::ptr_eq(&face.listed_chars(), &face.listed_chars()));
}
