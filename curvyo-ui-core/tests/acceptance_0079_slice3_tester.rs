//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 3 on
//! `curvyo-ui-core`: the caret in a column is a line across the column and the selection covers
//! the column (criterion 24); the arrow keys of `TextEdit` in columns (24).
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Document, FontKind, FontRef, Length, TextGlyphOrientation, TextLayoutValues,
    TextWritingMode,
};
use curvyo_text_core::{FaceStore, GlyphOrientation, LayoutOptions, Mm, WritingMode, layout};
use curvyo_ui_core::{CaretMove, TextEdit, TextInput, edit_geometry, layout_options};

const TOL: f64 = 0.01;

fn sans_bytes() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap()
}

fn column_options() -> LayoutOptions {
    LayoutOptions {
        writing_mode: WritingMode::VerticalRl,
        orientation: GlyphOrientation::Sideways,
        ..LayoutOptions::default()
    }
}

/// Criterion 24: the caret is a line across the column (horizontal on screen, as wide as the
/// column), and a selected range is a quad along the column.
#[test]
fn ac24_caret_across_the_column() {
    let mut store = FaceStore::new();
    let key = store.add(sans_bytes(), 0).unwrap();
    let face = store.get(&key).unwrap();
    let l = layout(face, "abc\ndef", Mm(10.0), &column_options()).unwrap();
    let g = edit_geometry(&l, Affine::IDENTITY, 1, &(1..1), None);
    let [a, b] = g.caret.expect("a caret");
    assert!((a.y - b.y).abs() < TOL, "across the column: {a:?} {b:?}");
    let width = (a.x - b.x).abs();
    assert!(
        (width - (l.ascender.0 + l.descender.0)).abs() < TOL,
        "column wide: {width}"
    );
    let g = edit_geometry(&l, Affine::IDENTITY, 0, &(0..3), None);
    assert_eq!(g.selection.len(), 1);
    let q = g.selection[0];
    let (xs, ys): (Vec<f64>, Vec<f64>) = q.iter().map(|p| (p.x, p.y)).unzip();
    let h =
        ys.iter().copied().fold(f64::MIN, f64::max) - ys.iter().copied().fold(f64::MAX, f64::min);
    let w =
        xs.iter().copied().fold(f64::MIN, f64::max) - xs.iter().copied().fold(f64::MAX, f64::min);
    assert!(h > w, "the selection runs down the column: {q:?}");
}

/// Criterion 24 on `TextEdit`: Up and Down move along a column, Left and Right across in reading
/// order, Home and End to the top and bottom.
#[test]
fn ac24_text_edit_keys_in_a_column() {
    let mut store = FaceStore::new();
    let key = store.add(sans_bytes(), 0).unwrap();
    let face = store.get(&key).unwrap();
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let d = Document::new(1);
    let values = TextLayoutValues {
        writing_mode: TextWritingMode::VerticalRl,
        orientation: TextGlyphOrientation::Sideways,
        ..TextLayoutValues::DEFAULT
    };
    let id = d
        .create_text(
            "abc\ndef",
            &font,
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &values,
        )
        .unwrap();
    let snap = d.text(id).unwrap();
    let opts = layout_options(snap.size, &snap.layout);
    assert_eq!(opts.writing_mode, WritingMode::VerticalRl);
    assert_eq!(opts.orientation, GlyphOrientation::Sideways);
    let l = layout(face, &snap.string, Mm(10.0), &opts).unwrap();
    let mut edit = TextEdit::existing(&snap, 1);
    let mut go = |to| {
        edit.apply(TextInput::Move { to, extend: false }, Some(&l));
        edit.selection().end
    };
    assert_eq!(go(CaretMove::Down), 2);
    assert_eq!(go(CaretMove::Up), 1);
    assert_eq!(go(CaretMove::Left), 5, "RL: Left is the next column");
    assert_eq!(go(CaretMove::Right), 1);
    assert_eq!(go(CaretMove::LineEnd), 3);
    assert_eq!(go(CaretMove::LineStart), 0);
}
