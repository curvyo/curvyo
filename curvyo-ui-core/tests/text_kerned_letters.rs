//! Box, caret and hit of a straight text with kerned and turned letters
//! (`specs/0079-text-layout-controls` criteria 26-29, customer report 2026-10-24): they follow
//! each letter as it is drawn, raised by the vertical kern and turned about its cluster's centre.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    Affine, Angle, CharValues, FontKind, FontRef, Length, Point, Style, TextChars,
    TextLayoutValues, TextSnapshot, TextWritingMode, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{edit_geometry, lay_out, line_box_contains, nearest_boundary, text_layout};

const TOLERANCE: f64 = 1e-6;

fn sans() -> std::sync::Arc<curvyo_text_core::Face> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

/// "HHHH" at 10 mm, every letter raised by `kern_y` of the size (a staircase) and turned by
/// `rotation` degrees, in `mode`, placed at (30, 40).
fn stairs(mode: TextWritingMode, kern_y: f64, rotation: f64) -> TextSnapshot {
    let mut text = TextSnapshot {
        id: curvyo_document_core::UNCREATED_TEXT_ID,
        string: "HHHH".to_owned(),
        font: FontRef {
            family: "Source Sans 3".to_owned(),
            style: "Regular".to_owned(),
            face_hash: "0".repeat(64),
            face_index: 0,
            kind: FontKind::Outline,
        },
        size: Length::from_mm(10.0),
        layout: TextLayoutValues {
            writing_mode: mode,
            ..TextLayoutValues::DEFAULT
        },
        chars: TextChars::new((0..4).map(|index| {
            (
                index,
                CharValues {
                    kern_x: 0.0,
                    kern_y,
                    rotation,
                },
            )
        })),
        placement: Affine::translation(Vec2::new(30.0, 40.0)),
        style: Style::default(),
        outline: None,
        on_path: None,
    };
    text.outline = lay_out(&text, &sans(), false).map(std::sync::Arc::new);
    text
}

/// The drawn points of each glyph, in the document.
fn glyph_points(text: &TextSnapshot) -> Vec<Vec<Point>> {
    let outline = text.outline.as_ref().unwrap();
    let mut contours = std::iter::once(outline.glyphs.anchors.clone()).chain(
        outline
            .glyphs
            .extra_subpaths
            .iter()
            .map(|s| s.anchors.clone()),
    );
    outline
        .glyph_outlines
        .iter()
        .map(|count| {
            (0..*count)
                .flat_map(|_| contours.next().unwrap())
                .map(|a| a.point)
                .collect()
        })
        .collect()
}

fn bounds(points: &[Point]) -> (Point, Point) {
    points.iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

fn contains(corners: &[Point; 4], points: &[Point]) -> bool {
    let (lo, hi) = bounds(corners);
    points.iter().all(|p| {
        p.x >= lo.x - TOLERANCE
            && p.x <= hi.x + TOLERANCE
            && p.y >= lo.y - TOLERANCE
            && p.y <= hi.y + TOLERANCE
    })
}

const MODES: [TextWritingMode; 3] = [
    TextWritingMode::Horizontal,
    TextWritingMode::VerticalRl,
    TextWritingMode::VerticalLr,
];

/// The text's box (selection, transform, fit) and its edit frame hold every drawn point of a
/// staircase, plain and turned, in every writing mode.
#[test]
fn the_box_of_a_staircase_holds_every_letter() {
    for mode in MODES {
        for rotation in [0.0, 30.0] {
            let text = stairs(mode, 0.4, rotation);
            let drawn: Vec<Point> = glyph_points(&text).concat();
            let outline = text.outline.as_ref().unwrap();
            assert!(contains(&outline.line_box, &drawn), "{mode:?} {rotation}");
            let laid = text_layout(&text, &sans()).unwrap();
            let geometry = edit_geometry(&laid, text.placement, 0, &(0..0), None);
            assert!(contains(&geometry.line_box, &drawn), "{mode:?} {rotation}");
        }
    }
}

/// A raised staircase's box does not keep the unraised line below the first letter.
#[test]
fn the_box_of_a_staircase_follows_the_letters() {
    let text = stairs(TextWritingMode::Horizontal, 0.4, 0.0);
    let plain = stairs(TextWritingMode::Horizontal, 0.0, 0.0);
    let (_, raised) = bounds(&text.outline.as_ref().unwrap().line_box);
    let (_, flat) = bounds(&plain.outline.as_ref().unwrap().line_box);
    // The first letter is raised by 4 mm: the box's bottom with it.
    assert!((flat.y - raised.y - 4.0).abs() < 1e-6);
}

/// The caret before a raised letter spans that letter, from above its top to below its foot.
#[test]
fn the_caret_before_a_raised_letter_spans_it() {
    for mode in MODES {
        let text = stairs(mode, 0.4, 0.0);
        let letters = glyph_points(&text);
        let laid = text_layout(&text, &sans()).unwrap();
        let geometry = edit_geometry(&laid, text.placement, 3, &(3..3), None);
        let [a, b] = geometry.caret.unwrap();
        let (lo, hi) = bounds(&letters[3]);
        if mode == TextWritingMode::Horizontal {
            assert!(a.y.min(b.y) <= lo.y && a.y.max(b.y) >= hi.y, "{mode:?}");
        } else {
            assert!(a.x.min(b.x) <= lo.x && a.x.max(b.x) >= hi.x, "{mode:?}");
        }
    }
}

/// The caret before a turned letter turns with it (its own rotation and Character rotation).
#[test]
fn the_caret_before_a_turned_letter_is_turned() {
    let mut own = stairs(TextWritingMode::Horizontal, 0.0, 30.0);
    let mut whole = stairs(TextWritingMode::Horizontal, 0.0, 0.0);
    whole.layout.rotation = Angle::from_radians(30_f64.to_radians());
    for text in [&mut own, &mut whole] {
        text.outline = lay_out(text, &sans(), false).map(std::sync::Arc::new);
        let laid = text_layout(text, &sans()).unwrap();
        let geometry = edit_geometry(&laid, text.placement, 2, &(2..2), None);
        let [a, b] = geometry.caret.unwrap();
        let turn = (b.x - a.x).atan2(b.y - a.y).to_degrees().abs();
        assert!((turn - 30.0).abs() < 1e-6, "{turn}");
    }
}

/// A point inside a raised letter hits the text and puts the caret next to that letter.
#[test]
fn a_point_in_a_raised_letter_hits_it() {
    for mode in MODES {
        let text = stairs(mode, 0.4, 0.0);
        let letters = glyph_points(&text);
        let (lo, hi) = bounds(&letters[3]);
        let inside = Point::new(f64::midpoint(lo.x, hi.x), f64::midpoint(lo.y, hi.y));
        assert!(line_box_contains(&text, inside, 0.0), "{mode:?}");
        let laid = text_layout(&text, &sans()).unwrap();
        let local = text.placement.inverse().unwrap().apply_point(inside);
        let caret = nearest_boundary(&laid, local);
        assert!(caret == 3 || caret == 4, "{mode:?} {caret}");
    }
}

/// The selection highlight of a raised letter covers it.
#[test]
fn the_selection_of_a_raised_letter_covers_it() {
    let text = stairs(TextWritingMode::Horizontal, 0.4, 0.0);
    let letters = glyph_points(&text);
    let laid = text_layout(&text, &sans()).unwrap();
    let geometry = edit_geometry(&laid, text.placement, 3, &(3..4), None);
    assert!(
        geometry
            .selection
            .iter()
            .any(|quad| contains(quad, &letters[3]))
    );
}
