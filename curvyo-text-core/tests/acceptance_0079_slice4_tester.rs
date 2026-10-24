//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 4 on
//! `curvyo-text-core` (ADR 0021 §6): a horizontal kern moves its character and the rest of the
//! line (criterion 27), a vertical kern moves its character and the rest of the line, positive up
//! (27, customer 2026-10-24), both act
//! along and across a column in vertical text (22, 27), a justified line keeps its width (3, ADR
//! 0021 §6), and degenerate offsets (past the end, on a line break, on the second character of a
//! cluster, unsorted) are ignored or applied as stated.
//!
//! Written from the specification, ADR 0021 and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use curvyo_text_core::{
    Align, CharOffset, Face, FaceStore, LayoutOptions, Mm, TextLayout, WritingMode, layout,
};
use kurbo::{Rect, Shape};

const TOL: f64 = 0.01;

fn sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

fn kern(index: usize, along: f64, across: f64) -> CharOffset {
    CharOffset {
        index,
        along: Mm(along),
        across: Mm(across),
        rotation_rad: 0.0,
    }
}

fn opts(offsets: Vec<CharOffset>) -> LayoutOptions {
    LayoutOptions {
        char_offsets: offsets,
        ..LayoutOptions::default()
    }
}

fn lay(face: &Face, text: &str, o: &LayoutOptions) -> TextLayout {
    layout(face, text, Mm(10.0), o).unwrap()
}

fn boxes(face: &Face, l: &TextLayout) -> Vec<Rect> {
    l.glyph_paths(face)
        .iter()
        .map(Shape::bounding_box)
        .collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOL
}

/// Criterion 27: a horizontal kern is extra space before its character; it and the rest of the
/// line move right by the kern, the characters before it stay; the line extent grows by it.
#[test]
fn c27_horizontal_kern_moves_the_char_and_the_rest_of_the_line() {
    sans(|face| {
        let plain = lay(face, "HHH", &opts(vec![]));
        let kerned = lay(face, "HHH", &opts(vec![kern(1, 2.0, 0.0)]));
        let (p, k) = (boxes(face, &plain), boxes(face, &kerned));
        assert_eq!(p.len(), 3);
        assert!(near(k[0].x0, p[0].x0), "before the kern: stays");
        for i in 1..3 {
            assert!(
                near(k[i].x0 - p[i].x0, 2.0),
                "glyph {i}: {:?} {:?}",
                p[i],
                k[i]
            );
            assert!(near(k[i].y0, p[i].y0), "no vertical move");
        }
        let w = |l: &TextLayout| l.lines[0].extent.end.0 - l.lines[0].extent.start.0;
        assert!(near(w(&kerned) - w(&plain), 2.0));
        // A negative kern pulls the pair closer (permit: overlap allowed).
        let tight = lay(face, "HHH", &opts(vec![kern(1, -9.0, 0.0)]));
        assert!(near(boxes(face, &tight)[1].x0 - p[1].x0, -9.0));
    });
}

/// Criterion 27 (customer 2026-10-24): a vertical kern moves its character and the following
/// ones up, the characters before it stay on the baseline, and no character moves sideways.
#[test]
fn c27_vertical_kern_moves_its_char_and_the_rest_up() {
    sans(|face| {
        let p = boxes(face, &lay(face, "HHH", &opts(vec![])));
        let k = boxes(face, &lay(face, "HHH", &opts(vec![kern(1, 0.0, 3.0)])));
        // "Up" on screen: the glyph's top gets 3 mm further from the line below it.
        let up = p[1].y0 - k[1].y0;
        assert!(near(up.abs(), 3.0), "moved by 3 mm: {:?} {:?}", p[1], k[1]);
        let line_dir = p[0].y1 - p[0].y0; // positive: y grows downward in text space
        assert!(line_dir > 0.0);
        // Up means away from the next line: the next baseline is lower (larger y).
        let two = lay(face, "H\nH", &opts(vec![]));
        let down_is_positive = two.lines[1].baseline.0 > two.lines[0].baseline.0;
        assert_eq!(
            up > 0.0,
            down_is_positive,
            "positive kern_y moves the character up"
        );
        assert!(
            near(k[0].y0, p[0].y0) && near(k[0].x0, p[0].x0),
            "glyph 0 stays"
        );
        assert!(near(p[2].y0 - k[2].y0, up), "glyph 2 moves with it");
        assert!(near(k[1].x0, p[1].x0) && near(k[2].x0, p[2].x0));
    });
}

/// Criteria 22 and 27: in a vertical column the horizontal kern is extra advance along the column
/// (it and the rest move down), the vertical kern moves its character and the rest of the column
/// across, to the right of the column (ADR 0021 §1, customer 2026-10-24).
#[test]
fn c27_vertical_text_kerns_along_and_across_the_column() {
    sans(|face| {
        let v = |offsets| LayoutOptions {
            writing_mode: WritingMode::VerticalRl,
            char_offsets: offsets,
            ..LayoutOptions::default()
        };
        let p = boxes(face, &lay(face, "HHH", &v(vec![])));
        let along = boxes(face, &lay(face, "HHH", &v(vec![kern(1, 2.0, 0.0)])));
        assert!(near(along[0].y0, p[0].y0) && near(along[0].x0, p[0].x0));
        for i in 1..3 {
            assert!(
                near(along[i].y0 - p[i].y0, 2.0),
                "glyph {i} down the column"
            );
            assert!(near(along[i].x0, p[i].x0), "glyph {i} stays in the column");
        }
        let across = boxes(face, &lay(face, "HHH", &v(vec![kern(1, 0.0, 3.0)])));
        assert!(
            near(across[1].x0 - p[1].x0, 3.0),
            "to the right of the column"
        );
        assert!(near(across[1].y0, p[1].y0));
        assert!(near(across[0].x0, p[0].x0) && near(across[0].y0, p[0].y0));
        assert!(near(across[2].x0 - p[2].x0, 3.0) && near(across[2].y0, p[2].y0));
    });
}

/// ADR 0021 §6 with criterion 3: the kern is added before justify, so a justified line keeps the
/// width of the widest line; the last line stays left-aligned and grows by the kern.
#[test]
fn c27_justify_keeps_the_width_of_a_kerned_line() {
    sans(|face| {
        let text = "A B C\nWWWWWWWW\nx y";
        let o = |offsets| LayoutOptions {
            align: Align::Justify,
            char_offsets: offsets,
            ..LayoutOptions::default()
        };
        let plain = lay(face, text, &o(vec![]));
        // 'B' is character 2 of line 0; 'y' is character 17 (last line).
        let kerned = lay(face, text, &o(vec![kern(2, 1.5, 0.0), kern(17, 1.0, 0.0)]));
        let ext = |l: &TextLayout, i: usize| (l.lines[i].extent.start.0, l.lines[i].extent.end.0);
        assert!(near(ext(&kerned, 0).0, ext(&plain, 0).0));
        assert!(
            near(ext(&kerned, 0).1, ext(&plain, 0).1),
            "justified width kept"
        );
        assert!(
            near(ext(&kerned, 0).1, ext(&plain, 1).1),
            "spread to the widest"
        );
        let last = ext(&kerned, 2).1 - ext(&plain, 2).1;
        assert!(
            near(last, 1.0),
            "the last line is left-aligned and grows: {last}"
        );
    });
}

/// White-box (ADR 0021 §1, §6): an offset past the end or on a line break is ignored, an offset
/// on a later character of a cluster is ignored, offsets in any order apply, and the caret
/// boundary before a kerned character sits in the middle of the gap.
#[test]
fn edge_degenerate_offsets_and_caret_in_the_gap() {
    sans(|face| {
        let plain = lay(face, "AB\nCD", &opts(vec![]));
        let ignored = lay(
            face,
            "AB\nCD",
            &opts(vec![kern(2, 5.0, 5.0), kern(99, 5.0, 5.0)]),
        );
        assert_eq!(
            boxes(face, &plain),
            boxes(face, &ignored),
            "line break and past end"
        );
        // e + combining acute is one cluster; an offset on the mark (character 1) is ignored.
        let base = lay(face, "e\u{301}x", &opts(vec![]));
        let on_mark = lay(face, "e\u{301}x", &opts(vec![kern(1, 4.0, 4.0)]));
        assert_eq!(
            boxes(face, &base),
            boxes(face, &on_mark),
            "second char of a cluster"
        );
        // Unsorted.
        let a = lay(
            face,
            "HHHH",
            &opts(vec![kern(3, 1.0, 0.0), kern(1, 2.0, 0.0)]),
        );
        let b = lay(
            face,
            "HHHH",
            &opts(vec![kern(1, 2.0, 0.0), kern(3, 1.0, 0.0)]),
        );
        assert_eq!(boxes(face, &a), boxes(face, &b));
        let p = boxes(face, &lay(face, "HHHH", &opts(vec![])));
        assert!(near(boxes(face, &a)[3].x0 - p[3].x0, 3.0), "both apply");
        // The caret between H and the kerned H: in the middle of the 2 mm gap.
        let k = lay(face, "HH", &opts(vec![kern(1, 2.0, 0.0)]));
        let u = lay(face, "HH", &opts(vec![]));
        let caret = |l: &TextLayout| l.caret_point(1).x;
        assert!(
            near(caret(&k) - caret(&u), 1.0),
            "{} {}",
            caret(&k),
            caret(&u)
        );
        // Non-finite input is the caller's to clean (ADR 0021 §1 read rules); it must not panic.
        let _ = layout(
            face,
            "HH",
            Mm(10.0),
            &opts(vec![kern(1, f64::NAN, f64::INFINITY)]),
        );
    });
}
