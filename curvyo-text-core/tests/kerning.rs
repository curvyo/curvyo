//! Kerning per character in the engine (`specs/0079-text-layout-controls` slice 4, criteria 26,
//! 27, 38; ADR 0021 §6): an along offset before a cluster moves it and the rest of the line, so
//! does an across offset (cumulative, customer 2026-10-24), in lines and in columns; a rotation
//! turns that cluster alone, on top of the text's rotation.
#![allow(clippy::unwrap_used)]

mod common;

use common::{SANS, face, store_with};
use curvyo_text_core::{Align, CharOffset, LayoutOptions, Mm, TextLayout, WritingMode, layout};
use kurbo::Point;

const TOLERANCE_MM: f64 = 0.01;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE_MM
}

fn kerned(index: usize, along: f64, across: f64) -> LayoutOptions {
    LayoutOptions {
        char_offsets: vec![CharOffset {
            index,
            along: Mm(along),
            across: Mm(across),
            rotation_rad: 0.0,
        }],
        ..LayoutOptions::default()
    }
}

fn origins(laid: &TextLayout) -> Vec<Point> {
    laid.lines
        .iter()
        .flat_map(|line| line.glyphs.iter().map(|g| g.origin))
        .collect()
}

/// Criterion 27: Horizontal kerning is extra space before the character: it and the rest of the
/// line move by it, the characters before stay; the caret boundary sits in the middle of the gap.
#[test]
fn ac27_along_moves_the_character_and_the_rest_of_the_line() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let plain = layout(face, "AVA", Mm(10.0), &LayoutOptions::default()).unwrap();
    let laid = layout(face, "AVA", Mm(10.0), &kerned(1, -1.5, 0.0)).unwrap();
    let (before, after) = (origins(&plain), origins(&laid));
    assert!(near(after[0].x, before[0].x));
    assert!(near(after[1].x, before[1].x - 1.5));
    assert!(near(after[2].x, before[2].x - 1.5));
    let extent = |l: &TextLayout| l.lines[0].extent.end.0 - l.lines[0].extent.start.0;
    assert!(near(extent(&laid), extent(&plain) - 1.5));
    let boundary = |l: &TextLayout| l.lines[0].boundaries[1].1.0;
    assert!(near(boundary(&laid), boundary(&plain) - 0.75));
}

/// Criterion 27: a kern on the first character moves the whole line from the anchor.
#[test]
fn ac27_along_on_the_first_character() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let plain = layout(face, "HH", Mm(10.0), &LayoutOptions::default()).unwrap();
    let laid = layout(face, "HH", Mm(10.0), &kerned(0, 2.0, 0.0)).unwrap();
    assert!(near(origins(&laid)[0].x, origins(&plain)[0].x + 2.0));
    assert!(near(laid.lines[0].extent.start.0, 2.0));
}

/// Criterion 27 (customer 2026-10-24): Vertical kerning moves that character and the rest of the
/// line, positive up, as SVG `dy`; the characters before it stay; the next line does not move.
#[test]
fn ac27_across_moves_the_character_and_the_rest_of_the_line() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let plain = layout(face, "HHH\nH", Mm(10.0), &LayoutOptions::default()).unwrap();
    let laid = layout(face, "HHH\nH", Mm(10.0), &kerned(1, 0.0, 3.0)).unwrap();
    let (before, after) = (origins(&plain), origins(&laid));
    assert!(near(after[0].y, before[0].y));
    for i in [1, 2] {
        assert!(near(after[i].y, before[i].y - 3.0), "glyph {i}");
        assert!(near(after[i].x, before[i].x), "glyph {i}");
    }
    assert!(near(after[3].y, before[3].y), "the next line stays");
    // Its outline moves with it.
    let paths = laid.glyph_paths(face);
    let plain_paths = plain.glyph_paths(face);
    let top = |p: &kurbo::BezPath| kurbo::Shape::bounding_box(p).y0;
    assert!(near(top(&paths[2]), top(&plain_paths[2]) - 3.0));
}

/// Criteria 27 and 27c: the same vertical kern on every character builds a staircase, each letter 1 mm
/// above the one before it.
#[test]
fn ac27c_vertical_kern_on_every_character_builds_stairs() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let offsets = (0..4)
        .map(|index| CharOffset {
            index,
            along: Mm(0.0),
            across: Mm(1.0),
            rotation_rad: 0.0,
        })
        .collect();
    let options = LayoutOptions {
        char_offsets: offsets,
        ..LayoutOptions::default()
    };
    let laid = origins(&layout(face, "HHHH", Mm(10.0), &options).unwrap());
    for pair in laid.windows(2) {
        assert!(near(pair[0].y - pair[1].y, 1.0), "{laid:?}");
    }
}

/// Criterion 27b: a character's own rotation turns it alone about its cluster's centre, added to
/// the text's Character rotation; its neighbours keep the text's rotation.
#[test]
fn ac27b_rotation_of_one_character_adds_to_the_text_rotation() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let quarter = std::f64::consts::FRAC_PI_2;
    let turned = |text_rad: f64, char_rad: f64| {
        let options = LayoutOptions {
            rotation_rad: text_rad,
            char_offsets: vec![CharOffset {
                index: 1,
                along: Mm(0.0),
                across: Mm(0.0),
                rotation_rad: char_rad,
            }],
            ..LayoutOptions::default()
        };
        let laid = layout(face, "HIH", Mm(10.0), &options).unwrap();
        laid.glyph_paths(face)
            .iter()
            .map(kurbo::Shape::bounding_box)
            .collect::<Vec<_>>()
    };
    let plain = turned(0.0, 0.0);
    let own = turned(0.0, quarter);
    let both = turned(quarter / 2.0, quarter / 2.0);
    assert_eq!(own[0], plain[0]);
    assert_eq!(own[2], plain[2]);
    // The narrow I lies on its side: wider than high, about the same centre.
    assert!(own[1].width() > own[1].height());
    assert!((own[1].center().x - plain[1].center().x).abs() < 0.3);
    // Half the turn from the text and half from the character: the same as the whole turn.
    assert!(near(both[1].x0, own[1].x0) && near(both[1].y0, own[1].y0));
    assert!(near(both[1].x1, own[1].x1) && near(both[1].y1, own[1].y1));
}

/// Criterion 27 in a column: along runs down the column, across to its right; both move the rest
/// of the column.
#[test]
fn ac27_offsets_in_a_column() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let column = |offsets: LayoutOptions| LayoutOptions {
        writing_mode: WritingMode::VerticalRl,
        ..offsets
    };
    let plain = layout(face, "HHH", Mm(10.0), &column(LayoutOptions::default())).unwrap();
    let laid = layout(face, "HHH", Mm(10.0), &column(kerned(1, 2.0, 1.0))).unwrap();
    let to_text = |l: &TextLayout, p: Point| l.frame() * p;
    let (before, after) = (origins(&plain), origins(&laid));
    let moved = to_text(&laid, after[1]) - to_text(&plain, before[1]);
    assert!(near(moved.y, 2.0), "down the column: {moved:?}");
    assert!(near(moved.x, 1.0), "to the right: {moved:?}");
    let next = to_text(&laid, after[2]) - to_text(&plain, before[2]);
    assert!(
        near(next.y, 2.0) && near(next.x, 1.0),
        "the rest of the column moves too"
    );
}

/// ADR 0021 §6: under Justify a kerned line still spreads to the widest line.
#[test]
fn justify_keeps_the_width_of_a_kerned_line() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let options = LayoutOptions {
        align: Align::Justify,
        ..kerned(1, -1.0, 0.0)
    };
    let laid = layout(face, "A B\nAAAAA\nx", Mm(10.0), &options).unwrap();
    let width = |line: usize| laid.lines[line].extent.end.0 - laid.lines[line].extent.start.0;
    assert!(near(width(0), width(1)));
}

/// ADR 0021 §1: a value on a later character of a cluster or on a line break is ignored.
#[test]
fn a_value_inside_a_cluster_or_on_a_break_is_ignored() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let text = "Ae\u{301}\nB";
    let plain = layout(face, text, Mm(10.0), &LayoutOptions::default()).unwrap();
    for index in [2, 3] {
        let laid = layout(face, text, Mm(10.0), &kerned(index, 2.0, 2.0)).unwrap();
        assert_eq!(origins(&laid), origins(&plain), "index {index}");
    }
}
