//! Vertical writing modes and glyph orientation in the engine (`specs/0079-text-layout-controls`
//! slice 3, criteria 22, 23, 25, 38), with a Noto Sans JP subset (vertical metrics, `vert`) and
//! the included fonts as fixture faces.
#![allow(clippy::unwrap_used)]

mod common;

use std::path::PathBuf;

use common::{SANS, face, font_bytes, store_with};
use curvyo_text_core::{
    Align, FaceStore, GlyphOrientation, LayoutOptions, Mm, TextLayout, WritingMode, layout,
};
use kurbo::{Affine, BezPath, Point, Rect, Shape};

const TOLERANCE_MM: f64 = 0.01;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE_MM
}

fn cjk() -> (FaceStore, curvyo_text_core::FaceKey) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/vertical/NotoSansJP-Subset.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    (store, key)
}

fn vertical(mode: WritingMode, orientation: GlyphOrientation) -> LayoutOptions {
    LayoutOptions {
        writing_mode: mode,
        orientation,
        ..LayoutOptions::default()
    }
}

fn rl(orientation: GlyphOrientation) -> LayoutOptions {
    vertical(WritingMode::VerticalRl, orientation)
}

fn bbox(paths: &[BezPath]) -> Rect {
    paths
        .iter()
        .map(Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO)
}

fn same_paths(a: &[BezPath], b: &[BezPath]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            let (a, b) = (a.elements(), b.elements());
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    let (pa, pb) = (end_of(*a), end_of(*b));
                    pa.distance(pb) < 1e-9
                })
        })
}

fn end_of(el: kurbo::PathEl) -> Point {
    match el {
        kurbo::PathEl::MoveTo(p)
        | kurbo::PathEl::LineTo(p)
        | kurbo::PathEl::QuadTo(_, p)
        | kurbo::PathEl::CurveTo(_, _, p) => p,
        kurbo::PathEl::ClosePath => Point::ZERO,
    }
}

fn lay(face: &curvyo_text_core::Face, text: &str, options: &LayoutOptions) -> TextLayout {
    layout(face, text, Mm(10.0), options).unwrap()
}

/// Criterion 23: Auto sets Latin as Sideways does and a CJK ideograph as Upright does.
#[test]
fn ac23_auto_is_sideways_for_latin_and_upright_for_cjk() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let paths = |text: &str, o: GlyphOrientation| lay(face, text, &rl(o)).glyph_paths(face);
    assert!(same_paths(
        &paths("AB", GlyphOrientation::Auto),
        &paths("AB", GlyphOrientation::Sideways)
    ));
    assert!(same_paths(
        &paths("日本", GlyphOrientation::Auto),
        &paths("日本", GlyphOrientation::Upright)
    ));
    assert!(!same_paths(
        &paths("日本", GlyphOrientation::Auto),
        &paths("日本", GlyphOrientation::Sideways)
    ));
}

/// Criterion 23, Sideways: the column is the horizontal line turned a quarter clockwise, centred
/// on the column; Latin reads top to bottom.
#[test]
fn ac23_sideways_is_the_line_turned_clockwise() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let flat = lay(face, "AB", &LayoutOptions::default());
    let column = lay(face, "AB", &rl(GlyphOrientation::Sideways));
    let metrics = face.metrics();
    let drop = f64::midpoint(metrics.ascender, metrics.descender) * flat.mm_per_em;
    let turn = WritingMode::VerticalRl.frame() * Affine::translate((0.0, drop));
    let expected: Vec<BezPath> = flat
        .glyph_paths(face)
        .iter()
        .map(|p| turn * p.clone())
        .collect();
    assert!(same_paths(&column.glyph_paths(face), &expected));
    let ink = bbox(&column.glyph_paths(face));
    let flat_ink = bbox(&flat.glyph_paths(face));
    assert!(
        near(ink.height(), flat_ink.width()),
        "reads down the column"
    );
    // The A's top lies to the right: the cap height spans from the turned baseline rightwards.
    assert!(ink.x1 > 0.0 && ink.x0 < 0.0);
}

/// Criterion 23, Upright: glyphs stand, centred on the column, advanced by `vmtx`, with the
/// vertical forms of `vert`.
#[test]
fn ac23_upright_uses_vertical_metrics_and_forms() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let column = lay(face, "日本", &rl(GlyphOrientation::Upright));
    let em = column.mm_per_em;
    let line = &column.lines[0];
    // vmtx: 1000 units per glyph, the second cluster one em further down.
    assert!(near(line.boundaries[1].1.0, em), "{:?}", line.boundaries);
    assert!(near(line.width.0, 2.0 * em));
    let first = bbox(&column.glyph_paths(face)[..1]);
    assert!(near(first.center().x, 0.0) || first.center().x.abs() < 0.05 * em);
    assert!(first.y0 >= -TOLERANCE_MM && first.y1 <= em + TOLERANCE_MM);
    // vert: the ideographic comma takes its vertical form.
    let flat = lay(face, "、", &LayoutOptions::default());
    let upright = lay(face, "、", &rl(GlyphOrientation::Upright));
    assert_ne!(
        flat.lines[0].glyphs[0].glyph,
        upright.lines[0].glyphs[0].glyph
    );
}

/// Criterion 23: a face without `vmtx` stands upright glyphs one ascender-to-descender apart.
#[test]
fn ac23_upright_without_vertical_metrics_uses_the_line_height() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let metrics = face.metrics();
    let column = lay(face, "AB", &rl(GlyphOrientation::Upright));
    let step = (metrics.ascender - metrics.descender) * column.mm_per_em;
    assert!(near(column.lines[0].boundaries[1].1.0, step));
}

/// Criterion 22: columns run right to left or left to right, one Line spacing apart; Auto takes
/// the face's `vhea` line, else the natural line spacing.
#[test]
fn ac22_columns_right_to_left_and_left_to_right() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let upright = GlyphOrientation::Upright;
    let column_x = |laid: &TextLayout, line: usize| {
        let glyph = &laid.lines[line].glyphs[0];
        (laid.frame() * Point::new(glyph.centre_x.0, glyph.centre_y.0)).x
    };
    let right_to_left = lay(face, "日\n本", &vertical(WritingMode::VerticalRl, upright));
    let lr = lay(face, "日\n本", &vertical(WritingMode::VerticalLr, upright));
    let pitch = face.metrics().vertical_line_height.unwrap() * right_to_left.mm_per_em;
    assert!(near(right_to_left.line_height.0, pitch));
    assert!(near(column_x(&right_to_left, 0), 0.0) && near(column_x(&right_to_left, 1), -pitch));
    assert!(near(column_x(&lr, 1), pitch));
    let set = LayoutOptions {
        line_spacing: Some(Mm(15.0)),
        ..vertical(WritingMode::VerticalRl, upright)
    };
    assert!(near(column_x(&lay(face, "日\n本", &set), 1), -15.0));
    let (sans_store, sans_key) = store_with(SANS);
    let sans = common::face(&sans_store, &sans_key);
    let natural = lay(sans, "A\nB", &rl(GlyphOrientation::Auto));
    assert!(near(natural.line_height.0, natural.natural_line_height.0));
    assert!(near(
        natural.line_height.0,
        sans.metrics().line_height() * natural.mm_per_em
    ));
}

/// Criterion 22: alignment, letter spacing and the line box act along the column.
#[test]
fn ac22_alignment_and_spacing_along_the_column() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let centre = LayoutOptions {
        align: Align::Centre,
        ..rl(GlyphOrientation::Auto)
    };
    let centred = lay(face, "日本", &centre);
    let extent = &centred.lines[0].extent;
    assert!(near(extent.start.0 + extent.end.0, 0.0));
    let ink = bbox(&centred.glyph_paths(face));
    assert!(ink.y0 < 0.0 && ink.y1 > 0.0, "centred on the anchor's y");
    let spaced = LayoutOptions {
        letter_spacing: Mm(2.0),
        ..rl(GlyphOrientation::Auto)
    };
    let plain = lay(face, "日本", &rl(GlyphOrientation::Auto));
    let wide = lay(face, "日本", &spaced);
    let height = |l: &TextLayout| bbox(&l.glyph_paths(face)).height();
    assert!(near(height(&wide) - height(&plain), 2.0));
    let line_box = wide.line_box();
    let glyphs = bbox(&wide.glyph_paths(face));
    assert!(
        line_box
            .inflate(TOLERANCE_MM, TOLERANCE_MM)
            .contains_rect(glyphs)
    );
    assert!(line_box.height() > line_box.width(), "a column is tall");
}

/// Criterion 22 with criterion 11: an upright glyph turns about its own middle on the column.
#[test]
fn ac22_character_rotation_in_a_column() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let plain = lay(face, "日", &rl(GlyphOrientation::Upright));
    let turned = LayoutOptions {
        rotation_rad: std::f64::consts::FRAC_PI_2,
        ..rl(GlyphOrientation::Upright)
    };
    let turned = lay(face, "日", &turned);
    let pivot = Point::new(0.0, plain.mm_per_em / 2.0);
    let expected = Affine::rotate_about(std::f64::consts::FRAC_PI_2, pivot);
    let want: Vec<BezPath> = plain
        .glyph_paths(face)
        .iter()
        .map(|p| expected * p.clone())
        .collect();
    assert!(same_paths(&turned.glyph_paths(face), &want));
}

/// Criteria 10 and 24: the caret positions of a column are found by a point in text space.
#[test]
fn ac24_a_point_on_the_column_finds_its_caret_position() {
    let (store, key) = cjk();
    let face = face(&store, &key);
    let column = lay(face, "日本語", &rl(GlyphOrientation::Upright));
    let em = column.mm_per_em;
    assert_eq!(column.nearest_boundary(Point::new(0.0, 1.1 * em)), 3);
    assert_eq!(column.nearest_boundary(Point::new(0.2, 2.9 * em)), 9);
    let two = lay(face, "日本\n語", &rl(GlyphOrientation::Upright));
    let pitch = two.line_height.0;
    assert_eq!(two.nearest_boundary(Point::new(-pitch, 0.1)), 7);
}

/// Criterion 25: a single-line font stands upright one em height per glyph, centred on the
/// column.
#[test]
fn ac25_single_line_fonts_in_a_column() {
    let mut store = FaceStore::new();
    let key = store
        .add(font_bytes("ems-readability/EMSReadability.svg"), 0)
        .unwrap();
    let face = face(&store, &key);
    let column = lay(face, "AB", &rl(GlyphOrientation::Upright));
    assert!(near(column.lines[0].boundaries[1].1.0, column.mm_per_em));
    let a = bbox(&column.glyph_paths(face)[..1]);
    assert!(a.center().x.abs() < 0.1 * column.mm_per_em, "{a:?}");
    let sideways = lay(face, "AB", &rl(GlyphOrientation::Auto));
    assert_ne!(sideways.glyph_paths(face).len(), 0);
}
