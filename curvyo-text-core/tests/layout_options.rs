//! The layout values of `specs/0079-text-layout-controls` slice 1 in the engine (criteria 2, 3, 5,
//! 8 to 11, 15, 38): alignment, justify, letter, word and line spacing and character rotation,
//! with the included fonts as fixture faces.
#![allow(clippy::unwrap_used)]

mod common;

use common::{SANS, SERIF, face, store_with};
use curvyo_text_core::{Align, LayoutOptions, Mm, TextLayout, layout};
use kurbo::{Affine, Point, Rect, Shape};

const TOLERANCE_MM: f64 = 0.01;

fn options(align: Align) -> LayoutOptions {
    LayoutOptions {
        align,
        ..LayoutOptions::default()
    }
}

fn extent_width(laid: &TextLayout, line: usize) -> f64 {
    let extent = &laid.lines[line].extent;
    extent.end.0 - extent.start.0
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE_MM
}

/// Criterion 2: Left, Centre and Right put each line's extent at, around or before the anchor.
#[test]
fn ac02_left_centre_right_on_the_line_extent() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let text = "A\nAAA\nAA";
    let left = layout(face, text, Mm(10.0), &options(Align::Left)).unwrap();
    let centre = layout(face, text, Mm(10.0), &options(Align::Centre)).unwrap();
    let right = layout(face, text, Mm(10.0), &options(Align::Right)).unwrap();
    for index in 0..3 {
        assert!(near(left.lines[index].extent.start.0, 0.0));
        let middle = f64::midpoint(
            centre.lines[index].extent.start.0,
            centre.lines[index].extent.end.0,
        );
        assert!(near(middle, 0.0), "line {index} centred: {middle}");
        assert!(near(right.lines[index].extent.end.0, 0.0));
        assert!(near(
            extent_width(&centre, index),
            extent_width(&left, index)
        ));
    }
    // Trailing spaces do not count: "AA  " right-aligned ends its last A at the anchor.
    let spaced = layout(face, "AA  \nA", Mm(10.0), &options(Align::Right)).unwrap();
    let last_a = spaced.lines[0].glyphs[1];
    let a_advance = left.lines[0].width.0;
    assert!(near(last_a.origin.x + a_advance, 0.0));
}

/// Criterion 3: Justify spreads every line but the last to the widest line, by its spaces, or by
/// its cluster gaps without spaces; a line of one cluster and the last line stay left.
#[test]
fn ac03_justify() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let justify = options(Align::Justify);
    let laid = layout(face, "A A\nHHHHHH\nAB\nX\nA A", Mm(10.0), &justify).unwrap();
    let widest = extent_width(&laid, 1);
    for line in [0, 2] {
        assert!(near(laid.lines[line].extent.start.0, 0.0));
        assert!(
            near(extent_width(&laid, line), widest),
            "line {line} spread"
        );
    }
    let plain = layout(
        face,
        "A A\nHHHHHH\nAB\nX\nA A",
        Mm(10.0),
        &LayoutOptions::default(),
    )
    .unwrap();
    assert!(
        near(extent_width(&laid, 3), extent_width(&plain, 3)),
        "one cluster"
    );
    assert!(
        near(extent_width(&laid, 4), extent_width(&plain, 4)),
        "last line"
    );
    // With a space the A's keep their own spacing: the extra goes to the space.
    let first = &laid.lines[0].glyphs;
    let plain_first = &plain.lines[0].glyphs;
    assert!(near(first[1].origin.x, plain_first[1].origin.x));
    // Without a space the extra goes between the two clusters.
    let ab = &laid.lines[2].glyphs;
    let plain_ab = &plain.lines[2].glyphs;
    assert!(ab[1].origin.x > plain_ab[1].origin.x + 1.0);
}

/// Criterion 8: letter spacing widens the advance of every cluster except the line's last;
/// ligatures are off with letter spacing, the font's kerning stays.
#[test]
fn ac08_letter_spacing() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let spaced = LayoutOptions {
        letter_spacing: Mm(2.0),
        ..LayoutOptions::default()
    };
    let width = |text: &str, options: &LayoutOptions| {
        extent_width(&layout(face, text, Mm(20.0), options).unwrap(), 0)
    };
    let plain = LayoutOptions::default();
    assert!(near(width("HH", &spaced) - width("HH", &plain), 2.0));
    assert!(near(width("H", &spaced), width("H", &plain)));
    // Kerning stays: AV is still the kerned pair plus one spacing.
    assert!(near(width("AV", &spaced) - width("AV", &plain), 2.0));
    let fi = layout(face, "fi", Mm(20.0), &spaced).unwrap();
    assert_eq!(
        fi.lines[0].glyphs.len(),
        2,
        "no ligature with letter spacing"
    );
    let fi = layout(face, "fi", Mm(20.0), &plain).unwrap();
    assert_eq!(fi.lines[0].glyphs.len(), 1, "the ligature without");
}

/// Criterion 9: word spacing widens every space character; a negative one may overlap.
#[test]
fn ac09_word_spacing() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let width = |text: &str, word: f64| {
        let options = LayoutOptions {
            word_spacing: Mm(word),
            ..LayoutOptions::default()
        };
        extent_width(&layout(face, text, Mm(10.0), &options).unwrap(), 0)
    };
    assert!(near(width("H H", 3.0) - width("H H", 0.0), 3.0));
    assert!(near(width("H\u{a0}H", 3.0) - width("H\u{a0}H", 0.0), 3.0));
    assert!(near(width("HH", 3.0), width("HH", 0.0)));
    assert!(
        width("H H", -20.0) < width("HH", 0.0),
        "permit: the words overlap"
    );
}

/// Criterion 10: a set line spacing puts each next baseline that distance lower; Auto keeps the
/// face's, which the layout reports either way.
#[test]
fn ac10_line_spacing() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let auto = layout(face, "H\nH", Mm(10.0), &LayoutOptions::default()).unwrap();
    let set = LayoutOptions {
        line_spacing: Some(Mm(15.0)),
        ..LayoutOptions::default()
    };
    let laid = layout(face, "H\nH", Mm(10.0), &set).unwrap();
    assert!(near(
        laid.lines[1].baseline.0 - laid.lines[0].baseline.0,
        15.0
    ));
    assert!(near(laid.line_height.0, 15.0));
    assert!(near(laid.natural_line_height.0, auto.line_height.0));
    assert!(near(auto.natural_line_height.0, auto.lines[1].baseline.0));
}

/// Criterion 11: every glyph turns about the middle of its advance, half the Size above the
/// baseline, clockwise on screen for a positive angle (the Select tool's sign); advances stay.
#[test]
fn ac11_character_rotation_about_the_cluster_centre() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let size = 20.0;
    let plain = layout(face, "I", Mm(size), &LayoutOptions::default()).unwrap();
    let turned_options = LayoutOptions {
        rotation_rad: 90.0_f64.to_radians(),
        ..LayoutOptions::default()
    };
    let turned = layout(face, "I", Mm(size), &turned_options).unwrap();
    assert!(near(turned.lines[0].width.0, plain.lines[0].width.0));
    let glyph = plain.lines[0].glyphs[0];
    assert!(near(glyph.centre_x.0, plain.lines[0].width.0 / 2.0));
    let before = plain.glyph_paths(face)[0].bounding_box();
    let after = turned.glyph_paths(face)[0].bounding_box();
    assert!(before.height() > before.width(), "the I stands");
    assert!(after.width() > after.height(), "its stem lies horizontal");
    assert!(near(before.center().x, after.center().x));
    assert!(near(before.center().y, after.center().y));
    // Clockwise on screen (y down): the pivot is (centre_x, -size/2), a point above the pivot
    // goes to its right.
    let pivot = Point::new(glyph.centre_x.0, -size / 2.0);
    let expected = Affine::rotate_about(90.0_f64.to_radians(), pivot).transform_rect_bbox(before);
    assert_rect_near(after, expected);
    let top = Point::new(pivot.x, pivot.y - 5.0);
    let moved = Affine::rotate_about(90.0_f64.to_radians(), pivot) * top;
    assert!(moved.x > pivot.x + 4.9);
}

fn assert_rect_near(a: Rect, b: Rect) {
    for (x, y) in [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)] {
        assert!(near(x, y), "{a:?} against {b:?}");
    }
}

/// Criteria 5 and 15: the caret between two clusters sits in the middle of the letter spacing,
/// and every boundary moves with its line; the line box covers the shifted lines.
#[test]
fn ac05_ac15_caret_boundaries_and_line_box_follow_the_layout() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let plain = layout(face, "HH", Mm(10.0), &LayoutOptions::default()).unwrap();
    let h = plain.lines[0].boundaries[1].1.0;
    let spaced = LayoutOptions {
        letter_spacing: Mm(2.0),
        ..LayoutOptions::default()
    };
    let laid = layout(face, "HH", Mm(10.0), &spaced).unwrap();
    assert!(near(laid.lines[0].boundaries[1].1.0, h + 1.0));
    assert!(near(laid.lines[0].boundaries[2].1.0, 2.0 * h + 2.0));

    let centred = layout(face, "H\nHHH", Mm(10.0), &options(Align::Centre)).unwrap();
    let line = &centred.lines[0];
    assert!(near(line.boundaries[0].1.0, line.extent.start.0));
    assert!(near(centred.caret_point(0).x, line.extent.start.0));
    let line_box = centred.line_box();
    assert!(near(line_box.x0, centred.lines[1].extent.start.0));
    assert!(near(line_box.x1, centred.lines[1].extent.end.0));
    // A press at the left end of the long line finds its first boundary.
    let at = centred.nearest_boundary(Point::new(line_box.x0, centred.lines[1].baseline.0));
    assert_eq!(at, centred.lines[1].range.start);
}

/// Criterion 38: the default options lay out as before this spec.
#[test]
fn ac38_default_options_are_todays_layout() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let laid = layout(face, "AVfi Hxg\nH", Mm(10.0), &LayoutOptions::default()).unwrap();
    assert!(laid.lines.iter().all(|line| near(line.extent.start.0, 0.0)));
    assert!(near(laid.line_box().x0, 0.0));
    assert!(near(laid.line_height.0, laid.natural_line_height.0));
}

/// Criteria 5 and 12: every value set on 500 characters lays out and draws within the frame
/// budget (asserted in release builds; a debug build prints the time).
#[test]
fn ac05_ac12_budget_with_every_value_set() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let line = "AVATAR Toffee: the office waffle flavour fits five fine fjords. ";
    let mut text: String = line.chars().cycle().take(500).collect();
    text.replace_range(100..101, "\n");
    text.replace_range(300..301, "\n");
    let options = LayoutOptions {
        align: Align::Justify,
        letter_spacing: Mm(0.5),
        word_spacing: Mm(1.0),
        line_spacing: Some(Mm(14.0)),
        rotation_rad: 0.3,
        ..LayoutOptions::default()
    };
    let started = std::time::Instant::now();
    let laid = layout(face, &text, Mm(10.0), &options).unwrap();
    let paths = laid.glyph_paths(face);
    let elapsed = started.elapsed();
    println!("500 characters with every value: {elapsed:?}");
    assert!(paths.len() > 300);
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() <= 50);
    }
}
