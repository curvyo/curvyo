//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 3 on
//! `curvyo-text-core`: columns right to left and left to right one line spacing apart (criterion
//! 22), Auto from `vhea`, alignment and spacing along the column (22), Auto, Upright and Sideways
//! per UAX #50 with `vert` forms and the fallback advance (23), the caret in columns (24),
//! single-line fonts (25), Object to path equals drawing (35) and degenerate input.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::assert_is_empty,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::time::Instant;

use curvyo_text_core::{
    Align, Face, FaceStore, GlyphOrientation, LayoutOptions, Mm, TextLayout, WritingMode, layout,
};
use kurbo::{BezPath, PathEl, Point, Rect, Shape};

const TOL: f64 = 0.01;

fn font_bytes(file: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap()
}

fn noto_bytes() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vertical/NotoSansJP-Subset.otf"),
    )
    .unwrap()
}

fn with_bytes<R>(bytes: Vec<u8>, f: impl FnOnce(&Face) -> R) -> R {
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

fn noto<R>(f: impl FnOnce(&Face) -> R) -> R {
    with_bytes(noto_bytes(), f)
}

fn sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    with_bytes(font_bytes("source-sans-3/SourceSans3-Regular.otf"), f)
}

/// A single-line SVG font: em 1000, H 700 high, "I" one stroke on the middle of its 800 advance.
fn single_line<R>(f: impl FnOnce(&Face) -> R) -> R {
    let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font horiz-adv-x="800"><font-face font-family="Line" units-per-em="1000" ascent="800" descent="-200" cap-height="700"/>
<glyph unicode="H" d="M100 0V700M100 350H700M700 0V700"/>
<glyph unicode="I" d="M400 0V700"/>
<glyph unicode=" " horiz-adv-x="300"/>
</font></defs></svg>"#
        .to_vec();
    with_bytes(bytes, f)
}

fn vopts(mode: WritingMode, orientation: GlyphOrientation) -> LayoutOptions {
    LayoutOptions {
        writing_mode: mode,
        orientation,
        ..LayoutOptions::default()
    }
}

fn lay(face: &Face, text: &str, size: f64, o: &LayoutOptions) -> TextLayout {
    layout(face, text, Mm(size), o).unwrap()
}

fn boxes(face: &Face, l: &TextLayout) -> Vec<Rect> {
    l.glyph_paths(face)
        .iter()
        .map(Shape::bounding_box)
        .collect()
}

fn points(path: &BezPath) -> Vec<Point> {
    path.elements()
        .iter()
        .flat_map(|e| match *e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => vec![p],
            PathEl::QuadTo(a, p) => vec![a, p],
            PathEl::CurveTo(a, b, p) => vec![a, b, p],
            PathEl::ClosePath => vec![],
        })
        .collect()
}

fn same_paths(a: &[BezPath], b: &[BezPath]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            let (p, q) = (points(p), points(q));
            p.len() == q.len()
                && p.iter()
                    .zip(&q)
                    .all(|(u, v)| (u.x - v.x).abs() < 1e-6 && (u.y - v.y).abs() < 1e-6)
        })
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOL
}

/// Criterion 22: in Vertical, right to left, each next line is a column one line spacing further
/// left, the first the rightmost; Line spacing Auto in a face with `vhea` is its vertical line
/// height (Noto subset: 500 − (−500) + 0 = 1000 units).
#[test]
fn ac22_columns_right_to_left_auto_from_vhea() {
    noto(|face| {
        let l = lay(
            face,
            "日\n日\n日",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Upright),
        );
        let pitch = l.mm_per_em;
        assert!(
            near(l.line_height.0, pitch),
            "{} vs {pitch}",
            l.line_height.0
        );
        let b = boxes(face, &l);
        assert_eq!(b.len(), 3);
        for i in 1..3 {
            let dx = b[i].center().x - b[0].center().x;
            assert!(near(dx, -pitch * i as f64), "column {i}: {dx}");
            assert!(near(b[i].y0, b[0].y0), "columns start at the same top");
        }
    });
}

/// Criterion 22: Vertical, left to right puts each next column to the right; a set Line spacing
/// is the distance between columns.
#[test]
fn ac22_columns_left_to_right_with_set_spacing() {
    noto(|face| {
        let l = lay(
            face,
            "日\n日\n日",
            10.0,
            &LayoutOptions {
                line_spacing: Some(Mm(15.0)),
                ..vopts(WritingMode::VerticalLr, GlyphOrientation::Upright)
            },
        );
        let b = boxes(face, &l);
        for i in 1..3 {
            let dx = b[i].center().x - b[0].center().x;
            assert!(near(dx, 15.0 * i as f64), "column {i}: {dx}");
        }
    });
}

/// Criterion 22: a face without `vhea` spaces its columns by the natural line spacing.
#[test]
fn ac22_auto_without_vhea_is_natural() {
    sans(|face| {
        let l = lay(
            face,
            "H\nH",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Sideways),
        );
        assert!(near(l.line_height.0, l.natural_line_height.0));
        let b = boxes(face, &l);
        assert!(near(
            b[1].center().x - b[0].center().x,
            -l.natural_line_height.0
        ));
    });
}

/// Criterion 22: the anchor is the top of the first column's centre line: the column box starts
/// at y = 0 and is centred on x = 0, and an upright glyph of a symmetric outline is centred on it.
#[test]
fn ac22_anchor_is_top_of_the_first_centre_line() {
    noto(|face| {
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            for orientation in [
                GlyphOrientation::Auto,
                GlyphOrientation::Upright,
                GlyphOrientation::Sideways,
            ] {
                let l = lay(face, "日本AB", 10.0, &vopts(mode, orientation));
                let b = l.line_box();
                assert!(b.y0.abs() < TOL, "{mode:?} {orientation:?}: {b:?}");
                assert!(b.center().x.abs() < TOL, "{mode:?} {orientation:?}: {b:?}");
                assert!(b.height() > b.width(), "a column: {b:?}");
            }
        }
    });
    single_line(|face| {
        let l = lay(
            face,
            "I",
            7.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Upright),
        );
        let b = boxes(face, &l)[0];
        assert!(b.center().x.abs() < TOL, "stroke on the centre line: {b:?}");
        assert!(b.y0 >= -TOL, "below the anchor: {b:?}");
    });
}

/// Criterion 22: Left, Centre and Right act along the column as top, middle and bottom.
#[test]
fn ac22_alignment_along_the_column() {
    noto(|face| {
        let at = |align| {
            boxes(
                face,
                &lay(
                    face,
                    "日\n日日",
                    10.0,
                    &LayoutOptions {
                        align,
                        ..vopts(WritingMode::VerticalRl, GlyphOrientation::Upright)
                    },
                ),
            )
        };
        let top = at(Align::Left);
        assert!(near(top[0].y0, top[1].y0), "top: {top:?}");
        let bottom = at(Align::Right);
        assert!(near(bottom[0].y1, bottom[2].y1), "bottom: {bottom:?}");
        let mid = at(Align::Centre);
        let col2 = f64::midpoint(mid[1].center().y, mid[2].center().y);
        assert!(near(mid[0].center().y, col2), "middle: {mid:?}");
        // The x of each column does not depend on the alignment.
        assert!(near(top[1].center().x, bottom[1].center().x));
    });
}

/// Criterion 22: Letter spacing and Word spacing act along the column.
#[test]
fn ac22_spacing_along_the_column() {
    noto(|face| {
        let at = |letter: f64, word: f64| {
            boxes(
                face,
                &lay(
                    face,
                    "日日 日",
                    10.0,
                    &LayoutOptions {
                        letter_spacing: Mm(letter),
                        word_spacing: Mm(word),
                        ..vopts(WritingMode::VerticalRl, GlyphOrientation::Upright)
                    },
                ),
            )
        };
        let plain = at(0.0, 0.0);
        let spaced = at(2.0, 0.0);
        assert!(
            near(spaced[1].y0 - plain[1].y0, 2.0),
            "{spaced:?} {plain:?}"
        );
        assert!(
            near(spaced[1].center().x, plain[1].center().x),
            "no crosswise move"
        );
        let worded = at(0.0, 3.0);
        assert!(near(worded[2].y0 - plain[2].y0, 3.0), "{worded:?}");
        assert!(near(worded[1].y0, plain[1].y0));
    });
}

/// Criterion 22 with 11: Character rotation turns an upright glyph in its own frame, about the
/// middle of its vertical advance on the column's centre line; the advances do not change.
#[test]
fn ac22_rotation_of_an_upright_glyph() {
    single_line(|face| {
        let o = vopts(WritingMode::VerticalRl, GlyphOrientation::Upright);
        let up = lay(face, "II", 7.0, &o);
        let adv = up.mm_per_em;
        for (rad, turn) in [
            (std::f64::consts::FRAC_PI_2, (0.0, -1.0, 1.0, 0.0)),
            (std::f64::consts::PI, (-1.0, 0.0, 0.0, -1.0)),
        ] {
            let turned = lay(
                face,
                "II",
                7.0,
                &LayoutOptions {
                    rotation_rad: rad,
                    ..o
                },
            );
            let (a, b) = (boxes(face, &up), boxes(face, &turned));
            for i in 0..2 {
                // Pivot: the column's centre line, the middle of the glyph's advance.
                let p = Point::new(0.0, (i as f64 + 0.5) * adv);
                let d = a[i].center() - p;
                let (m00, m01, m10, m11) = turn;
                let want = Point::new(p.x + m00 * d.x + m01 * d.y, p.y + m10 * d.x + m11 * d.y);
                let got = b[i].center();
                assert!(
                    near(got.x, want.x) && near(got.y, want.y),
                    "{rad} glyph {i}: {got:?} want {want:?}"
                );
            }
            assert!(
                b[0].width() > 6.9 || rad > 3.0,
                "turned stroke lies: {:?}",
                b[0]
            );
        }
    });
}

/// Criterion 23: Sideways sets the column as the horizontal line turned 90° clockwise: every
/// outline point is the horizontal one under (x, y) -> (−y, x) plus one constant offset.
#[test]
fn ac23_sideways_is_the_line_turned_clockwise() {
    sans(|face| {
        let h = lay(face, "Hxy AB", 10.0, &LayoutOptions::default()).glyph_paths(face);
        let s = lay(
            face,
            "Hxy AB",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Sideways),
        )
        .glyph_paths(face);
        assert_eq!(h.len(), s.len());
        let first = points(&h[0])[0];
        let turned = points(&s[0])[0];
        let offset = turned - Point::new(-first.y, first.x);
        for (p, q) in h.iter().zip(&s) {
            for (u, v) in points(p).iter().zip(points(q)) {
                let want = Point::new(-u.y, u.x) + offset;
                assert!(
                    (want.x - v.x).abs() < 1e-6 && (want.y - v.y).abs() < 1e-6,
                    "{u:?} -> {v:?}, want {want:?}"
                );
            }
        }
        // Letters lie on their right side: the H's top (most negative y) goes to the right.
        let hb = h[0].bounding_box();
        let sb = s[0].bounding_box();
        assert!(near(sb.width(), hb.height()) && near(sb.height(), hb.width()));
    });
}

/// Criterion 23, the spec's test: in the fixture face "AB" in Auto equals "AB" in Sideways, and a
/// CJK ideograph in Auto equals Upright.
#[test]
fn ac23_auto_follows_uax50_on_the_fixture() {
    noto(|face| {
        let at = |text: &str, orientation| {
            lay(
                face,
                text,
                10.0,
                &vopts(WritingMode::VerticalRl, orientation),
            )
            .glyph_paths(face)
        };
        assert!(same_paths(
            &at("AB", GlyphOrientation::Auto),
            &at("AB", GlyphOrientation::Sideways)
        ));
        assert!(same_paths(
            &at("日本語", GlyphOrientation::Auto),
            &at("日本語", GlyphOrientation::Upright)
        ));
        assert!(!same_paths(
            &at("AB", GlyphOrientation::Upright),
            &at("AB", GlyphOrientation::Sideways)
        ));
    });
}

/// Criterion 23: Auto with Latin and CJK mixed in one column keeps every glyph after the one
/// before it (upright and sideways runs do not overlap along the column).
#[test]
fn ac23_auto_mixed_runs_follow_each_other() {
    noto(|face| {
        let l = lay(
            face,
            "日AB本x語",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Auto),
        );
        let b = boxes(face, &l);
        assert_eq!(b.len(), 6);
        for i in 1..b.len() {
            assert!(b[i].y0 >= b[i - 1].y1 - TOL, "glyph {i} overlaps: {b:?}");
            assert!(
                b[i].center().x.abs() < 0.5 * l.line_height.0,
                "in the column: {b:?}"
            );
        }
        // The A is sideways: wider than high, its upright form is higher than wide.
        let flat = boxes(face, &lay(face, "A", 10.0, &LayoutOptions::default()))[0];
        assert!(near(b[1].width(), flat.height()) && near(b[1].height(), flat.width()));
    });
}

/// Criterion 23: Upright advances by `vmtx` (Noto: 1000 units) and applies `vert`: the ideographic
/// comma and the corner bracket take other glyphs than in horizontal text.
#[test]
fn ac23_upright_vmtx_advance_and_vert_forms() {
    noto(|face| {
        let o = vopts(WritingMode::VerticalRl, GlyphOrientation::Upright);
        let l = lay(face, "日日", 10.0, &o);
        let b = boxes(face, &l);
        assert!(near(b[1].y0 - b[0].y0, l.mm_per_em), "{b:?}");
        for c in ["、", "「", "ー", "（"] {
            let h = lay(face, c, 10.0, &LayoutOptions::default());
            let v = lay(face, c, 10.0, &o);
            assert_ne!(
                h.lines[0].glyphs[0].glyph, v.lines[0].glyphs[0].glyph,
                "{c}: no vertical form"
            );
            assert!(v.lines[0].glyphs[0].upright, "{c}");
        }
    });
}

/// Criterion 23: a face without `vmtx` advances Upright glyphs by its ascender minus descender
/// scaled to Size.
#[test]
fn ac23_upright_fallback_advance_without_vmtx() {
    sans(|face| {
        let l = lay(
            face,
            "HH",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Upright),
        );
        let b = boxes(face, &l);
        let m = face.metrics();
        let want = (m.ascender - m.descender) * l.mm_per_em;
        let step = b[1].y0 - b[0].y0;
        assert!(step > 10.0, "a vertical advance, not zero: {step}");
        assert!(near(step, want), "{step} vs ascender − descender {want}");
        assert!(near(b[0].center().x, b[1].center().x));
        assert!(b[0].height() > 9.9, "H stands upright: {:?}", b[0]);
    });
}

/// Criterion 21 (core side): a horizontal text ignores the orientation.
#[test]
fn ac21_horizontal_ignores_orientation() {
    noto(|face| {
        let at = |orientation| {
            lay(
                face,
                "日A、",
                10.0,
                &LayoutOptions {
                    orientation,
                    ..LayoutOptions::default()
                },
            )
            .glyph_paths(face)
        };
        let auto = at(GlyphOrientation::Auto);
        assert!(same_paths(&auto, &at(GlyphOrientation::Upright)));
        assert!(same_paths(&auto, &at(GlyphOrientation::Sideways)));
    });
}

/// Criterion 24 (core side): a text-space point in a column finds the caret position along that
/// column, and the caret moves down the column as the byte grows.
#[test]
fn ac24_caret_positions_in_columns() {
    noto(|face| {
        let l = lay(
            face,
            "日本\n日本",
            10.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Upright),
        );
        let adv = l.mm_per_em;
        let pitch = l.line_height.0;
        // Bytes: 日 0..3, 本 3..6, break 6, 日 7..10, 本 10..13.
        assert_eq!(l.nearest_boundary(Point::new(0.0, 0.4 * adv)), 0);
        assert_eq!(l.nearest_boundary(Point::new(0.0, 0.9 * adv)), 3);
        assert_eq!(l.nearest_boundary(Point::new(-pitch, 0.1 * adv)), 7);
        assert_eq!(l.nearest_boundary(Point::new(-pitch, 1.8 * adv)), 13);
        let f = l.frame();
        let c0 = f * l.caret_point(7);
        let c1 = f * l.caret_point(10);
        assert!(near(c1.y - c0.y, adv), "{c0:?} {c1:?}");
        assert!(near(c0.x, c1.x), "same column");
        let first = f * l.caret_point(0);
        assert!(
            near(c0.x - first.x, -pitch),
            "next column left: {first:?} {c0:?}"
        );
        // Left to right: the second column lies right, and a point there finds it.
        let lr = lay(
            face,
            "日本\n日本",
            10.0,
            &vopts(WritingMode::VerticalLr, GlyphOrientation::Upright),
        );
        assert_eq!(lr.nearest_boundary(Point::new(pitch, 0.1 * adv)), 7);
        assert_eq!(lr.nearest_boundary(Point::new(pitch, 1.8 * adv)), 13);
        assert_eq!(lr.nearest_boundary(Point::new(0.0, 1.8 * adv)), 6);
    });
}

/// Criterion 25: a single-line font works in columns: Upright advances by the em height (it has
/// no vertical metrics), Sideways turns the strokes, and the outlines stay open paths.
#[test]
fn ac25_single_line_font_in_columns() {
    single_line(|face| {
        let up = lay(
            face,
            "II",
            7.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Upright),
        );
        let b = boxes(face, &up);
        assert!(near(b[1].y0 - b[0].y0, up.mm_per_em), "{b:?}");
        let side = lay(
            face,
            "II",
            7.0,
            &vopts(WritingMode::VerticalLr, GlyphOrientation::Sideways),
        );
        let s = boxes(face, &side);
        assert!(
            s[0].width() > 6.9 && s[0].height() < TOL,
            "stroke lies: {:?}",
            s[0]
        );
        assert!(near(s[1].y0 - s[0].y0, 0.8 * side.mm_per_em), "{s:?}");
        for l in [&up, &side] {
            for p in l.glyph_paths(face) {
                assert!(
                    !p.elements().iter().any(|e| matches!(e, PathEl::ClosePath)),
                    "open paths"
                );
            }
        }
    });
}

/// Criterion 35 (core side): the outlines of a vertical text are one path per drawn glyph, and
/// every glyph has a path (no glyph is dropped in mixed runs).
#[test]
fn ac35_one_path_per_glyph_in_columns() {
    noto(|face| {
        for orientation in [
            GlyphOrientation::Auto,
            GlyphOrientation::Upright,
            GlyphOrientation::Sideways,
        ] {
            let l = lay(
                face,
                "日本語、AB\nxy「ー」",
                10.0,
                &vopts(WritingMode::VerticalLr, orientation),
            );
            let glyphs: usize = l.lines.iter().map(|line| line.glyphs.len()).sum();
            assert_eq!(l.glyph_paths(face).len(), glyphs, "{orientation:?}");
            assert_eq!(glyphs, 11, "{orientation:?}");
        }
    });
}

/// Degenerate input in columns: empty string, only breaks, only spaces, with every value set,
/// lay out without a panic and with finite boxes.
#[test]
fn edge_degenerate_vertical_strings() {
    noto(|face| {
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            for orientation in [
                GlyphOrientation::Auto,
                GlyphOrientation::Upright,
                GlyphOrientation::Sideways,
            ] {
                for align in [Align::Left, Align::Centre, Align::Right, Align::Justify] {
                    let o = LayoutOptions {
                        align,
                        letter_spacing: Mm(-5.0),
                        word_spacing: Mm(-10.0),
                        line_spacing: Some(Mm(0.0)),
                        rotation_rad: -3.0,
                        writing_mode: mode,
                        orientation,
                    };
                    for text in ["", " ", "\n\n", " \n ", "日", "日\n", "A 日 B\n日日"] {
                        let l = lay(face, text, 10.0, &o);
                        assert!(!l.lines.is_empty());
                        assert!(l.line_box().is_finite(), "{text:?} {o:?}");
                        let _ = l.caret_point(text.len());
                        let _ = l.nearest_boundary(Point::new(-1e6, 1e6));
                    }
                }
            }
        }
        // The largest Size stays finite.
        let big = lay(
            face,
            "日A",
            2000.0,
            &vopts(WritingMode::VerticalRl, GlyphOrientation::Auto),
        );
        assert!(big.line_box().is_finite() && big.line_box().height() > 1000.0);
    });
}

/// Budget (`0074` typing budget): 500 characters of mixed CJK and Latin in a column lay out
/// within 50 ms (release; a debug build gets ten times the time).
#[test]
fn edge_vertical_500_characters_budget() {
    noto(|face| {
        let text: String = "日本語ABxy、。「ー」\n"
            .repeat(50)
            .chars()
            .take(500)
            .collect();
        let o = LayoutOptions {
            align: Align::Justify,
            letter_spacing: Mm(0.5),
            rotation_rad: 0.3,
            ..vopts(WritingMode::VerticalRl, GlyphOrientation::Auto)
        };
        let _ = lay(face, &text, 10.0, &o).glyph_paths(face);
        let start = Instant::now();
        let paths = lay(face, &text, 10.0, &o).glyph_paths(face);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let budget = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
        assert!(!paths.is_empty());
        assert!(ms < budget, "{ms} ms");
    });
}

/// White box (runs split by orientation): a combining mark after an upright ideograph in Auto is
/// not split into its own cluster; the caret has no stop between base and mark (`0074` decision
/// 3), as in Upright.
#[test]
fn edge_auto_does_not_split_a_base_from_its_mark() {
    noto(|face| {
        let mut split = Vec::new();
        for text in ["日\u{0301}本", "日\u{FE00}本", "A\u{3099}B"] {
            for orientation in [GlyphOrientation::Upright, GlyphOrientation::Auto] {
                let l = lay(
                    face,
                    text,
                    10.0,
                    &vopts(WritingMode::VerticalRl, orientation),
                );
                let stops: Vec<usize> = l.lines[0].boundaries.iter().map(|(b, _)| *b).collect();
                let second = text.char_indices().nth(1).unwrap().0;
                if stops.contains(&second) {
                    split.push(format!("{text:?} {orientation:?}: {stops:?}"));
                }
            }
        }
        assert!(split.is_empty(), "a stop between base and mark: {split:?}");
    });
}
