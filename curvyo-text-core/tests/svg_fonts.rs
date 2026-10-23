//! Acceptance tests of the SVG font reader against `specs/0076-single-line-fonts` (criteria 1,
//! 12, 14, 15, 16, 23), with fixtures in `tests/fixtures/svg_fonts/` and the shipped
//! single-line fonts. Golden files hold the laid-out centre lines (CLAUDE.md §5).
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use std::fmt::Write as _;
use std::path::PathBuf;

use common::font_bytes;
use curvyo_text_core::limits::{
    MAX_GLYPH_POINTS, MAX_SVG_DEPTH, MAX_SVG_FONT_BYTES, MAX_SVG_GLYPHS,
};
use curvyo_text_core::{FaceKind, FaceStore, Mm, TextError, layout, listable_faces};
use kurbo::{PathEl, Shape};
use proptest::prelude::*;

const TOLERANCE_MM: f64 = 0.01;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).unwrap()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg_fonts")
}

/// The shipped single-line fonts (criterion 2).
const SHIPPED: [&str; 4] = [
    "hershey-sans-1/HersheySans1.svg",
    "hershey-script-1/HersheyScript1.svg",
    "ems-readability/EMSReadability.svg",
    "ems-allure/EMSAllure.svg",
];

/// The laid-out centre lines of `text` as path data, 3 decimals: the golden form.
fn dump(bytes: Vec<u8>, index: u32, text: &str) -> String {
    let mut store = FaceStore::new();
    let key = store.add(bytes, index).unwrap();
    let face = store.get(&key).unwrap();
    let laid = layout(face, text, Mm(20.0)).unwrap();
    let mut out = String::new();
    for path in laid.glyph_paths(face) {
        for element in path.elements() {
            let points: Vec<kurbo::Point> = match *element {
                PathEl::MoveTo(p) => {
                    out.push('M');
                    vec![p]
                }
                PathEl::LineTo(p) => {
                    out.push('L');
                    vec![p]
                }
                PathEl::QuadTo(a, p) => {
                    out.push('Q');
                    vec![a, p]
                }
                PathEl::CurveTo(a, b, p) => {
                    out.push('C');
                    vec![a, b, p]
                }
                PathEl::ClosePath => {
                    out.push('Z');
                    vec![]
                }
            };
            for p in points {
                let _ = write!(out, " {:.3} {:.3}", p.x + 0.0, p.y + 0.0);
            }
            out.push(' ');
        }
        out.push('\n');
    }
    out
}

/// Compares `actual` with the golden file `name`; `UPDATE_GOLDEN=1` writes it.
fn golden(name: &str, actual: &str) {
    let path = fixtures().join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap();
    assert_eq!(actual, expected, "golden {name}");
}

/// Criterion 16 (golden): every command of the fixture and the shipped fonts' centre lines.
#[test]
fn ac16_golden_centre_lines() {
    golden(
        "kerning.golden.txt",
        &dump(fixture("kerning.svg"), 0, "AVHOS Tfi L\u{E000}"),
    );
    let mut shipped = String::new();
    for path in SHIPPED {
        let _ = writeln!(shipped, "# {path}");
        shipped.push_str(&dump(font_bytes(path), 0, "Hag 7"));
    }
    golden("shipped.golden.txt", &shipped);
}

/// Criterion 1: every `<font>` with a drawn glyph is a face with family and style; a file
/// without one, or not well-formed, gives none.
#[test]
fn ac01_faces_of_an_svg_file() {
    let faces = listable_faces(&fixture("two-fonts.svg"));
    let named: Vec<(u32, &str, &str, FaceKind)> = faces
        .iter()
        .map(|f| (f.index, f.family.as_str(), f.style.as_str(), f.kind))
        .collect();
    assert_eq!(
        named,
        [
            (1, "Two One", "Regular", FaceKind::SingleLine),
            (2, "Two Two", "Bold", FaceKind::SingleLine)
        ]
    );
    let kerning = listable_faces(&fixture("kerning.svg"));
    assert_eq!(
        (kerning[0].family.as_str(), kerning[0].style.as_str()),
        ("Curvyo Kerning Test", "Bold Italic")
    );
    assert_eq!(listable_faces(&fixture("not-a-font.svg")), []);
    for path in SHIPPED {
        let faces = listable_faces(&font_bytes(path));
        assert_eq!(faces.len(), 1, "{path}");
        assert_eq!(faces[0].style, "Regular");
    }
    let mut store = FaceStore::new();
    let key = store.add(fixture("two-fonts.svg"), 2).unwrap();
    assert_eq!(store.get(&key).unwrap().kind(), FaceKind::SingleLine);
    assert_eq!(
        store.add(fixture("two-fonts.svg"), 0),
        Err(TextError::Unreadable(0))
    );
}

/// Criterion 12: the H's centre line is Size high (its path box, not a stroke), for every
/// shipped face and the fixture; a face without H uses `cap-height`.
#[test]
fn ac12_size_is_the_centre_line_of_h() {
    let mut sources: Vec<Vec<u8>> = SHIPPED.iter().map(|p| font_bytes(p)).collect();
    sources.push(fixture("kerning.svg"));
    for bytes in sources {
        let mut store = FaceStore::new();
        let key = store.add(bytes, 0).unwrap();
        let face = store.get(&key).unwrap();
        let laid = layout(face, "H", Mm(20.0)).unwrap();
        let top = -laid.glyph_paths(face)[0].bounding_box().y0;
        assert!((top - 20.0).abs() < TOLERANCE_MM, "{top}");
    }
    let mut store = FaceStore::new();
    let key = store.add(fixture("two-fonts.svg"), 1).unwrap();
    let face = store.get(&key).unwrap();
    assert!(
        (face.metrics().cap_height - 0.7).abs() < 1e-12,
        "no H and no cap-height: 0.7"
    );
}

/// Criterion 14: one glyph per character, advances, `hkern` by characters and by glyph names,
/// unicode ranges and multi-character glyphs ignored, the missing glyph drawn with its advance.
#[test]
fn ac14_glyphs_advances_and_kerning() {
    let mut store = FaceStore::new();
    let key = store.add(fixture("kerning.svg"), 0).unwrap();
    let face = store.get(&key).unwrap();
    let size = Mm(20.0);
    let mm_per_unit = layout(face, "H", size).unwrap().mm_per_em / 1000.0;
    let width = |text: &str| layout(face, text, size).unwrap().lines[0].width.0;
    assert!((width("AV") - (1200.0 - 120.0) * mm_per_unit).abs() < TOLERANCE_MM);
    assert!((width("TA") - (1100.0 - 80.0) * mm_per_unit).abs() < TOLERANCE_MM);
    assert!((width("TO") - (1200.0 - 80.0) * mm_per_unit).abs() < TOLERANCE_MM);
    assert!(
        (width("LT") - 1000.0 * mm_per_unit).abs() < TOLERANCE_MM,
        "range ignored"
    );
    let fi = layout(face, "fi", size).unwrap();
    assert_eq!(fi.lines[0].glyphs.len(), 2, "no ligature");
    assert!(fi.lines[0].glyphs.iter().all(|g| g.glyph == 0));
    assert!((fi.lines[0].width.0 - 800.0 * mm_per_unit).abs() < TOLERANCE_MM);
    assert_eq!(fi.glyph_paths(face).len(), 2, "the missing glyph is drawn");
    let plain = {
        let mut store = FaceStore::new();
        let key = store.add(font_bytes(SHIPPED[0]), 0).unwrap();
        let face = store.get(&key).unwrap();
        layout(face, "\u{E000}", size)
            .unwrap()
            .glyph_paths(face)
            .len()
    };
    assert_eq!(plain, 0, "a missing glyph without d draws nothing");
}

/// Criterion 15: the line height is ascent plus descent; without them, the em.
#[test]
fn ac15_line_height() {
    let mut store = FaceStore::new();
    let key = store.add(fixture("kerning.svg"), 0).unwrap();
    let face = store.get(&key).unwrap();
    let laid = layout(face, "H\nH", Mm(20.0)).unwrap();
    assert!((laid.lines[1].baseline.0 - laid.mm_per_em).abs() < 1e-9);
    let metrics = face.metrics();
    assert!((metrics.ascender - 0.8).abs() < 1e-12 && (metrics.descender + 0.2).abs() < 1e-12);
    let key = store.add(fixture("two-fonts.svg"), 2).unwrap();
    assert!((store.get(&key).unwrap().metrics().line_height() - 1.0).abs() < 1e-12);
}

/// Criterion 16: a sub-path is open unless it has Z, even when its ends meet.
#[test]
fn ac16_open_unless_z() {
    let mut store = FaceStore::new();
    let key = store.add(fixture("kerning.svg"), 0).unwrap();
    let face = store.get(&key).unwrap();
    let closes = |text: &str| {
        let laid = layout(face, text, Mm(20.0)).unwrap();
        laid.glyph_paths(face)[0]
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count()
    };
    assert_eq!(closes("A"), 0);
    assert_eq!(closes("O"), 1);
}

fn svg_font(glyphs: &str) -> Vec<u8> {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font><font-face font-family="X"/>{glyphs}</font></defs></svg>"#
    )
    .into_bytes()
}

/// Adds the bytes and, if they parse, lays out and outlines a text.
fn exercise(bytes: Vec<u8>) -> bool {
    let mut store = FaceStore::new();
    let Ok(key) = store.add(bytes, 0) else {
        return false;
    };
    let face = store.get(&key).unwrap();
    if let Ok(laid) = layout(face, "AVfi Hxg\u{E000}\nÅß0", Mm(5.0)) {
        let _ = laid.glyph_paths(face);
    }
    true
}

/// Criterion 23: hostile files are refused before or while parsing; nothing is resolved.
#[test]
fn ac23_hostile_svg_fonts() {
    for name in [
        "entity.svg",
        "external-entity.svg",
        "not-well-formed.svg",
        "zero-upem.svg",
        "not-finite.svg",
        "not-a-font.svg",
    ] {
        assert!(!exercise(fixture(name)), "{name}");
        assert_eq!(listable_faces(&fixture(name)), [], "{name}");
    }
    let deep = svg_font(&format!(
        "{}{}<glyph unicode='I' d='M0 0V1'/>",
        "<g>".repeat(MAX_SVG_DEPTH + 1),
        "</g>".repeat(MAX_SVG_DEPTH + 1)
    ));
    assert!(!exercise(deep));
    let mut big = svg_font("<glyph unicode='I' d='M0 0V1'/>");
    big.extend(std::iter::repeat_n(b' ', MAX_SVG_FONT_BYTES));
    assert!(!exercise(big));
    let many = svg_font(&"<glyph d='M0 0V1'/>".repeat(MAX_SVG_GLYPHS + 1));
    assert!(!exercise(many));
    let points = svg_font(&format!(
        "<glyph unicode='I' d='M0 0{}'/>",
        " 1 1".repeat(MAX_GLYPH_POINTS + 1)
    ));
    assert!(!exercise(points));
    assert!(exercise(svg_font("<glyph unicode='I' d='M0 0V1'/>")));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, timeout: 20_000, ..ProptestConfig::default() })]

    /// Criterion 23: mutated SVG fonts never panic or hang (time-boxed).
    #[test]
    fn ac23_mutated_svg_fonts_never_panic(
        flips in proptest::collection::vec((0usize..60_000, any::<u8>()), 1..40),
        cut in 0usize..60_000,
    ) {
        let mut bytes = fixture("kerning.svg");
        bytes.extend(font_bytes(SHIPPED[0]).iter().skip(400).take(3000));
        for (at, value) in flips {
            let len = bytes.len();
            bytes[at % len] = value;
        }
        bytes.truncate(cut.max(1).min(bytes.len()));
        let _ = exercise(bytes.clone());
        let _ = listable_faces(&bytes);
    }
}
