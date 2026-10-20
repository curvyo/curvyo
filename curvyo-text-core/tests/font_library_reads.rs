//! What the font library reads from a face's bytes (`specs/0075-font-library`): the declared
//! class from the OS/2 table (criterion 15) and the faces Add font accepts with their names
//! (criteria 28 and 32).
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use common::{SANS, SERIF, font_bytes};
use curvyo_text_core::{DeclaredClass, declared_class, listable_faces, os2_class_bytes};

/// The offset of table `tag` in a single-face file.
fn table_offset(bytes: &[u8], tag: [u8; 4]) -> usize {
    let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    (0..tables)
        .map(|record| 12 + 16 * record)
        .find(|&at| bytes[at..at + 4] == tag)
        .map(|at| u32::from_be_bytes(bytes[at + 8..at + 12].try_into().unwrap()) as usize)
        .unwrap()
}

/// `bytes` with the OS/2 `sFamilyClass` class byte, PANOSE family kind and serif style set.
fn patched(bytes: &[u8], class: u8, kind: u8, serif: u8) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let os2 = table_offset(&out, *b"OS/2");
    out[os2 + 30..os2 + 42].fill(0);
    out[os2 + 30] = class;
    out[os2 + 32] = kind;
    out[os2 + 33] = serif;
    out
}

/// Criterion 15: fixture faces with each declared value, and one with zeros, read from their
/// bytes.
#[test]
fn ac15_fixture_faces_declare_their_class() {
    let sans = font_bytes(SANS);
    let cases = [
        (1, 0, 0, Some(DeclaredClass::Serif)),
        (5, 0, 0, Some(DeclaredClass::Serif)),
        (7, 0, 0, Some(DeclaredClass::Serif)),
        (8, 0, 0, Some(DeclaredClass::Sans)),
        (10, 0, 0, Some(DeclaredClass::Script)),
        (0, 3, 0, Some(DeclaredClass::Script)),
        (0, 2, 11, Some(DeclaredClass::Sans)),
        (0, 2, 13, Some(DeclaredClass::Sans)),
        (0, 2, 4, Some(DeclaredClass::Serif)),
        (0, 0, 0, None),
    ];
    for (class, kind, serif, expected) in cases {
        let face = patched(&sans, class, kind, serif);
        let os2 = os2_class_bytes(&face, 0).unwrap();
        assert_eq!(declared_class(os2), expected, "{class} {kind} {serif}");
    }
    // Source Serif 4 declares no class but PANOSE serif style 4: Serif (tester re-review).
    let serif = os2_class_bytes(&font_bytes(SERIF), 0).unwrap();
    assert_eq!(declared_class(serif), Some(DeclaredClass::Serif));
}

/// Criteria 28 and 32: a font file gives its face with the listing's names; bytes that are not
/// a font, a cut-off file and a colour font give nothing.
#[test]
fn ac32_add_font_accepts_what_the_curvyo_block_lists() {
    let serif = font_bytes(SERIF);
    let faces = listable_faces(&serif);
    assert_eq!(faces.len(), 1);
    assert_eq!(
        (
            faces[0].index,
            faces[0].family.as_str(),
            faces[0].style.as_str()
        ),
        (0, "Source Serif 4", "Regular")
    );
    assert_eq!(listable_faces(b"%PDF-1.7 not a font"), []);
    assert_eq!(listable_faces(&serif[..serif.len() / 3]), []);
    let mut colour = serif.clone();
    let tables = usize::from(u16::from_be_bytes([colour[4], colour[5]]));
    for record in 0..tables {
        let at = 12 + 16 * record;
        if &colour[at..at + 4] == b"GPOS" {
            colour[at..at + 4].copy_from_slice(b"sbix");
        }
    }
    assert_eq!(listable_faces(&colour), []);
}
