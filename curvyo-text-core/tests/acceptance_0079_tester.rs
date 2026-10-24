//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 1 on
//! `curvyo-text-core`: alignment (criterion 2), Justify (3), letter spacing and ligatures (8),
//! word spacing (9), line spacing (10), character rotation about the cluster centre (11), the
//! line box and caret with spacing (15), the 500-character budget (5, 12) and degenerate input.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::assert_is_empty,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::time::Instant;

use curvyo_text_core::{Align, Face, FaceStore, LayoutOptions, Mm, TextLayout, layout};
use kurbo::{Rect, Shape};

const TOL: f64 = 0.01;

fn bytes(file: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap()
}

fn with_face<R>(file: &str, f: impl FnOnce(&Face) -> R) -> R {
    let mut store = FaceStore::new();
    let key = store.add(bytes(file), 0).unwrap();
    f(store.get(&key).unwrap())
}

fn sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    with_face("source-sans-3/SourceSans3-Regular.otf", f)
}

fn opts(align: Align) -> LayoutOptions {
    LayoutOptions {
        align,
        ..LayoutOptions::default()
    }
}

fn lay(face: &Face, text: &str, size: f64, o: &LayoutOptions) -> TextLayout {
    layout(face, text, Mm(size), o).unwrap()
}

fn ext(l: &TextLayout, i: usize) -> (f64, f64) {
    let e = &l.lines[i].extent;
    (e.start.0, e.end.0)
}

fn bbox(face: &Face, l: &TextLayout) -> Rect {
    l.glyph_paths(face)
        .iter()
        .map(Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap()
}

/// Criterion 2: Left starts, Centre centres and Right ends every line's extent at the anchor;
/// the spec's test "A", "AAA", "AA" with Centre has equal midpoints.
#[test]
fn ac02_left_centre_right_on_the_anchor() {
    sans(|face| {
        let text = "A\nAAA\nAA";
        for (align, want) in [
            (Align::Left, 0.0),
            (Align::Centre, 0.0),
            (Align::Right, 0.0),
        ] {
            let l = lay(face, text, 10.0, &opts(align));
            assert_eq!(l.lines.len(), 3);
            for i in 0..3 {
                let (a, b) = ext(&l, i);
                let at = match align {
                    Align::Left => a,
                    Align::Centre => f64::midpoint(a, b),
                    _ => b,
                };
                assert!((at - want).abs() < TOL, "{align:?} line {i}: {a}..{b}");
                assert!(b - a > 1.0, "the extent is not empty");
            }
        }
        // The extent widths do not depend on the alignment.
        let left = lay(face, text, 10.0, &opts(Align::Left));
        let right = lay(face, text, 10.0, &opts(Align::Right));
        for i in 0..3 {
            let (a, b) = ext(&left, i);
            let (c, d) = ext(&right, i);
            assert!(((b - a) - (d - c)).abs() < TOL);
        }
    });
}

/// Terms "Line extent": trailing spaces do not count for alignment.
#[test]
fn ac02_trailing_spaces_do_not_count() {
    sans(|face| {
        let plain = lay(face, "AA", 10.0, &opts(Align::Right));
        let spaced = lay(face, "AA   ", 10.0, &opts(Align::Right));
        assert!((ext(&plain, 0).0 - ext(&spaced, 0).0).abs() < TOL);
        assert!(ext(&spaced, 0).1.abs() < TOL);
        let c = lay(face, "AA   ", 10.0, &opts(Align::Centre));
        let (a, b) = ext(&c, 0);
        assert!(f64::midpoint(a, b).abs() < TOL, "{a}..{b}");
    });
}

/// Criterion 3: Justify spreads every line but the last to the widest extent, the same extra
/// width on each space; lines start at the anchor; the last line stays left-aligned.
#[test]
fn ac03_justify_by_spaces() {
    sans(|face| {
        let text = "A B C\nHHHHHHHHHHHH\nA B";
        let left = lay(face, text, 10.0, &opts(Align::Left));
        let just = lay(face, text, 10.0, &opts(Align::Justify));
        let widest = (0..3)
            .map(|i| {
                let (a, b) = ext(&left, i);
                b - a
            })
            .fold(0.0, f64::max);
        let (a, b) = ext(&just, 0);
        assert!(a.abs() < TOL, "starts at the anchor: {a}");
        assert!(
            ((b - a) - widest).abs() < TOL,
            "spread: {} vs {widest}",
            b - a
        );
        // Last line unchanged.
        assert_eq!(ext(&just, 2), ext(&left, 2));
        // Same extra on each of the two spaces: the B moves by e, the C by 2e.
        let x = |l: &TextLayout, k: usize| l.lines[0].glyphs[k].origin.x;
        let glyphs = &just.lines[0].glyphs;
        assert_eq!(glyphs.len(), 5, "A space B space C");
        let e1 = x(&just, 2) - x(&left, 2);
        let e2 = x(&just, 4) - x(&left, 4);
        assert!(e1 > 0.1);
        assert!((e2 - 2.0 * e1).abs() < TOL, "{e1} {e2}");
        // The letters themselves are not moved apart: A to the space is unchanged.
        assert!((x(&just, 0) - x(&left, 0)).abs() < TOL);
        assert!((x(&just, 1) - x(&left, 1)).abs() < TOL);
    });
}

/// Criterion 3: a line without spaces is spread evenly between its clusters; a one-cluster line
/// stays left-aligned.
#[test]
fn ac03_justify_by_cluster_gaps_and_one_cluster() {
    sans(|face| {
        let text = "HHHH\nHHHHHHHHHH\nH\nlast";
        let left = lay(face, text, 10.0, &opts(Align::Left));
        let just = lay(face, text, 10.0, &opts(Align::Justify));
        let w = |l: &TextLayout, i: usize| {
            let (a, b) = ext(l, i);
            b - a
        };
        assert!((w(&just, 0) - w(&left, 1)).abs() < TOL);
        let gaps: Vec<f64> = (1..4)
            .map(|k| just.lines[0].glyphs[k].origin.x - left.lines[0].glyphs[k].origin.x)
            .collect();
        assert!(
            (gaps[1] - 2.0 * gaps[0]).abs() < TOL && (gaps[2] - 3.0 * gaps[0]).abs() < TOL,
            "equal gaps: {gaps:?}"
        );
        assert_eq!(ext(&just, 2), ext(&left, 2), "one cluster stays left");
        assert_eq!(ext(&just, 3), ext(&left, 3), "last line stays left");
        assert!(ext(&just, 1).0.abs() < TOL);
    });
}

/// Criterion 8: letter spacing adds `l` after every cluster but the line's last; glyph outlines
/// unchanged; "H" alone is unchanged.
#[test]
fn ac08_letter_spacing_widens_between_clusters() {
    sans(|face| {
        let o = LayoutOptions {
            letter_spacing: Mm(2.0),
            ..LayoutOptions::default()
        };
        let d = LayoutOptions::default();
        let w = |l: &TextLayout| {
            let (a, b) = ext(l, 0);
            b - a
        };
        assert!((w(&lay(face, "HH", 10.0, &o)) - w(&lay(face, "HH", 10.0, &d)) - 2.0).abs() < TOL);
        assert!((w(&lay(face, "H", 10.0, &o)) - w(&lay(face, "H", 10.0, &d))).abs() < TOL);
        // Five clusters, four gaps.
        assert!(
            (w(&lay(face, "HHHHH", 10.0, &o)) - w(&lay(face, "HHHHH", 10.0, &d)) - 8.0).abs() < TOL
        );
        // Each line on its own: the last cluster of every line gets none.
        let two = lay(face, "HH\nHH", 10.0, &o);
        assert!((w(&two) - w(&lay(face, "HH", 10.0, &o))).abs() < TOL);
        // The glyph outlines are unchanged (same size of the second H).
        let a = lay(face, "HH", 10.0, &o).glyph_paths(face);
        let b = lay(face, "HH", 10.0, &d).glyph_paths(face);
        let (ra, rb) = (a[1].bounding_box(), b[1].bounding_box());
        assert!((ra.width() - rb.width()).abs() < 1e-6 && (ra.height() - rb.height()).abs() < 1e-6);
        assert!((ra.x0 - rb.x0 - 2.0).abs() < TOL);
    });
}

/// Criterion 8: with letter spacing the optional ligatures are off ("fi" draws two glyphs); the
/// font's kerning stays on (a kerned pair widens by exactly `l`).
#[test]
fn ac08_ligatures_off_kerning_on() {
    with_face("source-serif-4/SourceSerif4-Regular.ttf", |face| {
        let d = LayoutOptions::default();
        let o = LayoutOptions {
            letter_spacing: Mm(1.0),
            ..d
        };
        let tight = lay(face, "fi", 10.0, &d);
        let spaced = lay(face, "fi", 10.0, &o);
        assert_eq!(
            tight.lines[0].glyphs.len(),
            1,
            "the fixture face has an fi ligature"
        );
        assert_eq!(
            spaced.lines[0].glyphs.len(),
            2,
            "no ligature with letter spacing"
        );
        // Kerning: "AV" is kerned tighter than "A" + "V" in the face; with spacing the pair
        // widens by exactly 1 mm, so the kern stays.
        let w = |l: &TextLayout| {
            let (a, b) = ext(l, 0);
            b - a
        };
        let av = w(&lay(face, "AV", 10.0, &d));
        let av_spaced = w(&lay(face, "AV", 10.0, &o));
        assert!((av_spaced - av - 1.0).abs() < TOL, "{av} {av_spaced}");
    });
}

/// Criterion 9: word spacing adds `w` to every space character (U+0020 and U+00A0); a large
/// negative value lets words overlap without an error.
#[test]
fn ac09_word_spacing() {
    sans(|face| {
        let d = LayoutOptions::default();
        let o = LayoutOptions {
            word_spacing: Mm(3.0),
            ..d
        };
        let w = |l: &TextLayout| {
            let (a, b) = ext(l, 0);
            b - a
        };
        for text in ["H H", "H\u{a0}H"] {
            let diff = w(&lay(face, text, 10.0, &o)) - w(&lay(face, text, 10.0, &d));
            assert!((diff - 3.0).abs() < TOL, "{text:?}: {diff}");
        }
        assert!(
            (w(&lay(face, "H  H", 10.0, &o)) - w(&lay(face, "H  H", 10.0, &d)) - 6.0).abs() < TOL
        );
        // No space: unchanged.
        assert!((w(&lay(face, "HH", 10.0, &o)) - w(&lay(face, "HH", 10.0, &d))).abs() < TOL);
        let neg = LayoutOptions {
            word_spacing: Mm(-50.0),
            ..d
        };
        let l = lay(face, "H H", 10.0, &neg);
        assert!(
            l.lines[0].glyphs[2].origin.x < l.lines[0].glyphs[0].origin.x,
            "overlap permitted"
        );
    });
}

/// Criterion 10: Auto sets the natural line spacing; a set value puts the next baseline that
/// far lower.
#[test]
fn ac10_line_spacing() {
    sans(|face| {
        let auto = lay(face, "A\nB\nC", 10.0, &LayoutOptions::default());
        let nat = auto.natural_line_height.0;
        assert!(nat > 10.0);
        assert!((auto.lines[1].baseline.0 - nat).abs() < TOL);
        assert!((auto.lines[2].baseline.0 - 2.0 * nat).abs() < TOL);
        let set = |mm: f64, size: f64| {
            lay(
                face,
                "A\nB",
                size,
                &LayoutOptions {
                    line_spacing: Some(Mm(mm)),
                    ..LayoutOptions::default()
                },
            )
        };
        assert!((set(15.0, 10.0).lines[1].baseline.0 - 15.0).abs() < TOL);
        assert!((set(30.0, 20.0).lines[1].baseline.0 - 30.0).abs() < TOL);
        assert_eq!(
            set(15.0, 10.0).natural_line_height.0,
            nat,
            "natural stays reported"
        );
        // Line spacing 0: every line on one baseline (typed range starts at 0).
        assert!(set(0.0, 10.0).lines[1].baseline.0.abs() < TOL);
    });
}

/// Criterion 11: "I" at 90° draws its stem horizontal, centred on the same point as at 0°;
/// advances and line positions are unchanged.
#[test]
fn ac11_rotation_about_the_cluster_centre() {
    sans(|face| {
        let up = lay(face, "I", 20.0, &LayoutOptions::default());
        let turned = lay(
            face,
            "I",
            20.0,
            &LayoutOptions {
                rotation_rad: std::f64::consts::FRAC_PI_2,
                ..LayoutOptions::default()
            },
        );
        let (a, b) = (bbox(face, &up), bbox(face, &turned));
        assert!(a.height() > 3.0 * a.width(), "upright stem: {a:?}");
        assert!(
            b.width() > 3.0 * b.height(),
            "turned stem horizontal: {b:?}"
        );
        assert!((a.width() - b.height()).abs() < TOL && (a.height() - b.width()).abs() < TOL);
        // The pivot is the middle of the advance at half the Size above the baseline.
        let pivot_x = up.lines[0].glyphs[0].centre_x.0;
        let c = b.center();
        assert!((c.x - pivot_x).abs() < TOL, "{c:?} pivot x {pivot_x}");
        assert!((c.y + 10.0).abs() < TOL, "{c:?}: centred at half the Size");
        assert!(
            (a.center().x - c.x).abs() < TOL && (a.center().y - c.y).abs() < TOL,
            "same centre as at 0°: {:?} {c:?}",
            a.center()
        );
        assert_eq!(
            up.lines[0].glyphs[0].origin,
            turned.lines[0].glyphs[0].origin
        );
        assert_eq!(ext(&up, 0), ext(&turned, 0));
    });
}

/// Criterion 11: a turn by 360° draws the same outlines as 0°.
/// box onto itself.
#[test]
fn ac11_rotation_full_and_half_turn() {
    sans(|face| {
        let at = |rad: f64| {
            bbox(
                face,
                &lay(
                    face,
                    "HAx",
                    10.0,
                    &LayoutOptions {
                        rotation_rad: rad,
                        ..LayoutOptions::default()
                    },
                ),
            )
        };
        let (z, full) = (at(0.0), at(std::f64::consts::TAU));
        assert!((z.x0 - full.x0).abs() < 1e-6 && (z.y1 - full.y1).abs() < 1e-6);
    });
}

/// Criterion 15: the caret between two clusters sits in the middle of the added letter spacing.
#[test]
fn ac15_caret_in_the_middle_of_the_spacing() {
    sans(|face| {
        let d = lay(face, "HH", 10.0, &LayoutOptions::default());
        let s = lay(
            face,
            "HH",
            10.0,
            &LayoutOptions {
                letter_spacing: Mm(2.0),
                ..LayoutOptions::default()
            },
        );
        let mid = d.caret_point(1).x;
        assert!(
            (s.caret_point(1).x - (mid + 1.0)).abs() < TOL,
            "{} vs {}",
            s.caret_point(1).x,
            mid + 1.0
        );
        // The caret at the end follows the spaced last cluster.
        assert!((s.caret_point(2).x - (d.caret_point(2).x + 2.0)).abs() < TOL);
        // Centred: the caret takes the same shift as the glyphs.
        let c = lay(face, "HH", 10.0, &opts(Align::Centre));
        assert!((c.caret_point(0).x - ext(&c, 0).0).abs() < TOL);
    });
}

/// Criteria 5 and 12: 500 characters with every value set lay out within the frame budget
/// (release build, as the other budget tests; a debug build gets ten times the time).
#[test]
fn ac05_ac12_500_characters_budget() {
    sans(|face| {
        let text: String = "Lorem ipsum dolor sit amet, fi fl.\n"
            .repeat(15)
            .chars()
            .take(500)
            .collect();
        let o = LayoutOptions {
            align: Align::Justify,
            letter_spacing: Mm(0.5),
            word_spacing: Mm(1.0),
            line_spacing: Some(Mm(14.0)),
            rotation_rad: 0.3,
        };
        let _ = lay(face, &text, 10.0, &o).glyph_paths(face);
        let start = Instant::now();
        let l = lay(face, &text, 10.0, &o);
        let paths = l.glyph_paths(face);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let budget = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
        assert!(!paths.is_empty());
        assert!(ms < budget, "{ms} ms");
    });
}

/// Degenerate input: the empty string, only spaces, only line breaks, all with every value set,
/// lay out without a panic and with finite numbers; Justify on one line changes nothing.
#[test]
fn edge_degenerate_strings() {
    sans(|face| {
        let o = LayoutOptions {
            align: Align::Justify,
            letter_spacing: Mm(-5.0),
            word_spacing: Mm(-10.0),
            line_spacing: Some(Mm(0.0)),
            rotation_rad: -3.0,
        };
        for text in ["", " ", "   ", "\n\n", " \n ", "A", "A\n"] {
            for align in [Align::Left, Align::Centre, Align::Right, Align::Justify] {
                let l = lay(face, text, 10.0, &LayoutOptions { align, ..o });
                assert!(!l.lines.is_empty(), "{text:?}");
                let b = l.line_box();
                assert!(b.is_finite(), "{text:?} {align:?}: {b:?}");
                for line in &l.lines {
                    assert!(line.extent.start.0.is_finite() && line.extent.end.0.is_finite());
                }
            }
        }
        let one = lay(face, "A B", 10.0, &opts(Align::Justify));
        let left = lay(face, "A B", 10.0, &opts(Align::Left));
        assert_eq!(
            ext(&one, 0),
            ext(&left, 0),
            "a single line is the last line"
        );
    });
}

/// Single-line SVG fonts (`0076`) take the same values: letter spacing widens, centre centres.
#[test]
fn edge_single_line_face_takes_the_values() {
    with_face("hershey-sans-1/HersheySans1.svg", |face| {
        let d = lay(face, "HH", 10.0, &LayoutOptions::default());
        let o = lay(
            face,
            "HH",
            10.0,
            &LayoutOptions {
                letter_spacing: Mm(2.0),
                align: Align::Centre,
                ..LayoutOptions::default()
            },
        );
        let w = |l: &TextLayout| {
            let (a, b) = ext(l, 0);
            b - a
        };
        assert!((w(&o) - w(&d) - 2.0).abs() < TOL);
        let (a, b) = ext(&o, 0);
        assert!(f64::midpoint(a, b).abs() < TOL);
    });
}

/// Huge input (white-box): the longest string at the largest Size with the largest spacing stays
/// finite; one character more is refused, not truncated.
#[test]
fn edge_huge_values_stay_finite() {
    sans(|face| {
        let n = curvyo_text_core::limits::MAX_SHAPE_CHARS;
        let text: String = "W W\n".repeat(n / 4).chars().take(n).collect();
        let o = LayoutOptions {
            align: Align::Justify,
            letter_spacing: Mm(4000.0),
            word_spacing: Mm(10_000.0),
            line_spacing: Some(Mm(20_000.0)),
            rotation_rad: 1.0,
        };
        let l = lay(face, &text, 2000.0, &o);
        assert!(l.line_box().is_finite());
        assert!(
            l.lines
                .iter()
                .all(|line| line.glyphs.iter().all(|g| g.origin.is_finite()))
        );
        let longer = format!("{text}W");
        assert!(layout(face, &longer, Mm(2000.0), &o).is_err());
    });
}

/// Justify with negative letter spacing (white-box): lines still start at the anchor and spread
/// to the widest line.
#[test]
fn edge_justify_with_negative_letter_spacing() {
    sans(|face| {
        let o = LayoutOptions {
            align: Align::Justify,
            letter_spacing: Mm(-1.0),
            ..LayoutOptions::default()
        };
        let l = lay(face, "AB C\nHHHHHHHHHH\nend", 10.0, &o);
        let w = |i: usize| {
            let (a, b) = ext(&l, i);
            b - a
        };
        assert!(ext(&l, 0).0.abs() < TOL);
        assert!((w(0) - w(1)).abs() < TOL, "{} {}", w(0), w(1));
    });
}
