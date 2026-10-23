//! Independent tester cases for `specs/0077-font-preview/specification.md` on the text engine:
//! criteria 13 (a sample is one line however long; rewritten for the customer decision of
//! 2026-10-23, no wrapping), 15 (single-line faces draw open centre lines) and 18 (a missing
//! character is drawn as the face draws it).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read; the hostile-input cases at the end were added after reading the diff.

#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_text_core::{Face, FaceKind, FaceStore, preview_path};

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn face_from(bytes: Vec<u8>) -> Arc<Face> {
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    Arc::clone(store.get(&key).unwrap())
}

fn included(file: &str) -> Arc<Face> {
    face_from(std::fs::read(fonts_dir().join(file)).unwrap())
}

fn source_sans() -> Arc<Face> {
    included("source-sans-3/SourceSans3-Regular.otf")
}

fn no_umlaut() -> Arc<Face> {
    face_from(
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/svg_fonts/no-umlaut.svg"),
        )
        .unwrap(),
    )
}

/// Width and height of the line box of `text`.
fn natural(face: &Face, text: &str) -> (f64, f64) {
    let (_, rect) = preview_path(face, text).unwrap();
    (rect.width(), rect.height())
}

// --- Criterion 18 -----------------------------------------------------------------------------

/// Criterion 18: the family is not hidden and the tile draws what the face draws for the missing
/// character (`<missing-glyph>`), so a sample of only missing characters still has ink.
#[test]
fn ac18_a_missing_character_is_drawn_as_the_faces_missing_glyph() {
    let (d, rect) = preview_path(&no_umlaut(), "ü").unwrap();
    assert!(!d.trim().is_empty(), "the <missing-glyph> box is drawn");
    assert!(rect.width() > 0.0);
    let (d, _) = preview_path(&source_sans(), "\u{10FFFD}").unwrap();
    assert!(!d.trim().is_empty(), "Source Sans 3 draws its .notdef");
}

// --- Criterion 15 -----------------------------------------------------------------------------

/// Criterion 15: the included single-line faces are single-line, the outline faces are outlines,
/// so the tile knows whether to stroke or fill.
#[test]
fn ac15_single_line_and_outline_faces_are_told_apart() {
    for file in [
        "hershey-sans-1/HersheySans1.svg",
        "hershey-script-1/HersheyScript1.svg",
        "ems-allure/EMSAllure.svg",
    ] {
        assert_eq!(included(file).kind(), FaceKind::SingleLine, "{file}");
        let (d, _) = preview_path(&included(file), "Anna & Jonas 2026").unwrap();
        assert!(!d.is_empty(), "{file} draws the sample");
    }
    assert_eq!(source_sans().kind(), FaceKind::Outline);
    assert_eq!(
        included("great-vibes/GreatVibes-Regular.ttf").kind(),
        FaceKind::Outline
    );
}

/// Criterion 15: a single-line face's sample is its centre lines, open sub-paths: the "H" of the
/// fixture is three strokes with no close.
#[test]
fn ac15_single_line_sample_is_open_centre_lines() {
    let (d, _) = preview_path(&no_umlaut(), "H").unwrap();
    assert!(
        !d.contains('Z') && !d.contains('z'),
        "no closed sub-path: {d}"
    );
    assert_eq!(d.matches('M').count(), 3, "three strokes: {d}");
}

// --- Criterion 13 -----------------------------------------------------------------------------

/// Criterion 13 (customer decision 2026-10-23): the sample is one line, never wrapped and never
/// ended with an ellipsis. The 200-character sample without spaces (the field's limit) is drawn
/// one line high and as wide as its characters, for the tile to clip at its edge.
#[test]
fn ac13_longest_sample_is_one_line() {
    let face = source_sans();
    let (w1, h1) = natural(&face, "Ä");
    let (w, h) = natural(&face, &"Ä".repeat(200));
    assert!((h - h1).abs() < 1e-9, "one line high: {h} vs {h1}");
    assert!(
        w > 190.0 * w1,
        "the whole line, not cut: {w} vs {w1} per character"
    );
}

// --- White-box: degenerate input --------------------------------------------------------------

/// An empty sample and a sample of only spaces do not panic.
#[test]
fn wb_empty_and_blank_samples() {
    let face = source_sans();
    for text in ["", " ", "     "] {
        let (d, _) = preview_path(&face, text).unwrap();
        assert_eq!(d, "", "{text:?}");
    }
}
