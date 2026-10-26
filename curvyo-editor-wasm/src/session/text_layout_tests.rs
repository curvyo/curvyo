//! The text layout controls in the session (`specs/0079-text-layout-controls` slice 1: criteria
//! 1, 4 to 7, 10, 12 to 14, 33 to 35), with the included fonts handed in as the host hands them.

use std::borrow::Cow;

use curvyo_document_core::{ObjectSnapshot, Point, TextAlign, TextSnapshot};
use curvyo_ui_core::{Grid, LayoutField, TextInput};

use super::text_layout::LayoutOutcome;
use super::text_tests::{labels, load_wanted, session, texts, with_text};
use super::{Session, Tool};

fn only_text(session: &Session) -> TextSnapshot {
    texts(session).into_iter().next().unwrap()
}

/// The drawn x of the left end of `line`'s extent, in the document.
fn line_start(session: &Session, text: &TextSnapshot, line: usize) -> f64 {
    let face = session.face_of(&text.font).unwrap();
    let laid = curvyo_ui_core::text_layout(text, face).unwrap();
    text.placement
        .apply_point(Point::new(laid.lines[line].extent.start.0, 0.0))
        .x
}

/// Criteria 1 and 4: an alignment press on the selected text is one step "Change text
/// alignment"; the widest line keeps its place, the others take the new alignment; Undo puts it
/// back.
#[test]
fn ac01_ac04_alignment_keeps_the_text_in_place() {
    let mut session = with_text("A\nAAAA\nAA");
    let before = only_text(&session);
    assert_eq!(session.text_alignment(), Some(TextAlign::Left));
    session.set_text_alignment(TextAlign::Centre);
    let after = only_text(&session);
    assert_eq!(after.layout.align, TextAlign::Centre);
    assert_eq!(session.text_alignment(), Some(TextAlign::Centre));
    assert_eq!(labels(&session).last().unwrap(), "change_text_alignment");
    assert!((line_start(&session, &before, 1) - line_start(&session, &after, 1)).abs() < 0.01);
    assert!(line_start(&session, &after, 0) > line_start(&session, &before, 0) + 1.0);
    let steps = labels(&session).len();
    session.set_text_alignment(TextAlign::Centre);
    assert_eq!(
        labels(&session).len(),
        steps,
        "a press on the pressed button writes nothing"
    );
    session.undo();
    assert_eq!(only_text(&session).layout.align, TextAlign::Left);
    assert_eq!(only_text(&session).placement, before.placement);
}

/// Criterion 33: with no text selected and no edit mode, the press sets the next text's
/// alignment without a step, and the next text is created with it.
#[test]
fn ac33_the_next_text_takes_the_alignment() {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.set_text_alignment(TextAlign::Right);
    assert_eq!(labels(&session), Vec::<String>::new());
    assert_eq!(session.text_alignment(), Some(TextAlign::Right));
    assert!(
        session.text_layout_block().is_none(),
        "no subject: no block"
    );
    session.pointer_down(Point::new(50.0, 20.0), false);
    session.pointer_up(Point::new(50.0, 20.0), false, false);
    assert!(
        session.text_layout_block().is_some(),
        "the new text in edit mode"
    );
    session.text_input(TextInput::Insert("Right".to_owned()));
    session.escape();
    assert_eq!(labels(&session), ["add_text"]);
    assert_eq!(only_text(&session).layout.align, TextAlign::Right);
}

/// Criteria 6, 7 and 12: the block is there with a selected text; a drag previews on the canvas
/// without a step, the release writes one step, Escape writes nothing.
#[test]
fn ac06_ac12_drag_preview_commit_and_cancel() {
    let mut session = with_text("HH");
    let block = session.text_layout_block().unwrap();
    assert_eq!(block.len(), 8);
    assert_eq!(block[0].text, "0");
    let steps = labels(&session).len();
    let drawn = |session: &Session| {
        let objects = session.objects();
        let shown = session.with_text_edit(Cow::Borrowed(&objects[..]));
        match &shown[0] {
            ObjectSnapshot::Text(text) => text.outline.as_ref().unwrap().line_box[1].x,
            _ => unreachable!(),
        }
    };
    let width = drawn(&session);
    session.preview_text_layout(LayoutField::LetterSpacing, 0.5, Grid::Normal);
    assert!(drawn(&session) > width + 1.0, "the canvas follows the drag");
    assert_eq!(
        labels(&session).len(),
        steps,
        "nothing written while dragging"
    );
    session.cancel_text_layout_preview();
    assert!((drawn(&session) - width).abs() < 1e-9);
    assert_eq!(
        session.commit_text_layout_preview(),
        LayoutOutcome::Unchanged
    );
    assert_eq!(labels(&session).len(), steps, "Escape writes nothing");
    session.preview_text_layout(LayoutField::LetterSpacing, 0.5, Grid::Normal);
    assert_eq!(session.commit_text_layout_preview(), LayoutOutcome::Set);
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "change_letter_spacing");
    let text = only_text(&session);
    assert!(text.layout.letter_spacing > 0.0);
    // Size 10 mm: the track is -2.5 to 10 mm, the middle 3.75 on the 0.1 mm grid.
    assert_eq!(session.text_layout_block().unwrap()[0].text, "3.8");
}

/// Criterion 7: a typed value past the range is set to the limit with its outcome; reset
/// returns to 0, or to Auto for Line spacing (criterion 10).
#[test]
fn ac07_ac10_typed_limits_and_reset() {
    let mut session = with_text("Name\nTwo");
    assert_eq!(
        session.set_text_layout_text(LayoutField::LetterSpacing, "40"),
        LayoutOutcome::Clamped(true, 20.0)
    );
    assert_eq!(
        session.set_text_layout_text(LayoutField::LetterSpacing, "wide"),
        LayoutOutcome::Invalid
    );
    let line = &session.text_layout_block().unwrap()[2];
    assert!(
        line.auto && !line.resettable,
        "Auto: the natural spacing, muted"
    );
    assert_eq!(
        session.set_text_layout_text(LayoutField::LineSpacing, "15"),
        LayoutOutcome::Set
    );
    let line = &session.text_layout_block().unwrap()[2];
    assert!(!line.auto && line.resettable && line.text == "15");
    // A size change keeps the factor: 15 mm at 10 mm becomes 30 mm at 20 mm (criterion 10, 34).
    session.set_text_size(20.0);
    assert_eq!(session.text_layout_block().unwrap()[2].text, "30");
    assert_eq!(
        session.reset_text_layout(LayoutField::LineSpacing),
        LayoutOutcome::Set
    );
    assert!(session.text_layout_block().unwrap()[2].auto);
    assert_eq!(
        session.set_text_layout_text(LayoutField::CharacterRotation, "270"),
        LayoutOutcome::Set
    );
    assert_eq!(session.text_layout_block().unwrap()[3].text, "\u{2212}90");
    assert_eq!(
        labels(&session)[1..],
        [
            "change_letter_spacing",
            "change_line_spacing",
            "change_text_size",
            "change_line_spacing",
            "change_character_rotation"
        ]
    );
}

/// Criterion 13: several texts show one value when they agree in mm, else Mixed; a change sets
/// every text in one step, each held to its own Size.
#[test]
fn ac13_several_texts_in_one_step() {
    let mut session = with_text("Small");
    session.set_tool(Tool::Text);
    session.pointer_down(Point::new(10.0, 80.0), false);
    session.pointer_up(Point::new(10.0, 80.0), false, false);
    session.text_input(TextInput::Insert("Large".to_owned()));
    session.escape();
    session.set_text_size(30.0);
    load_wanted(&mut session);
    let ids: Vec<_> = texts(&session).iter().map(|t| t.id).collect();
    session.selection.set(&ids);
    assert_eq!(session.text_bar().size_mm, None, "sizes differ");
    let steps = labels(&session).len();
    assert_eq!(
        session.set_text_layout_text(LayoutField::LetterSpacing, "25"),
        LayoutOutcome::Limited { limited: 1, of: 2 }
    );
    assert_eq!(labels(&session).len(), steps + 1);
    let block = session.text_layout_block().unwrap();
    assert!(block[0].mixed, "20 mm on one, 25 mm on the other");
    assert_eq!(
        session.set_text_layout_text(LayoutField::WordSpacing, "2"),
        LayoutOutcome::Set
    );
    assert_eq!(session.text_layout_block().unwrap()[1].text, "2");
}

/// Criterion 14: in edit mode a field change keeps edit mode and the caret, applies to the whole
/// text and is its own step.
#[test]
fn ac14_edit_mode_stays() {
    let mut session = with_text("Name");
    session.pointer_down(Point::new(10.2, 18.0), false);
    session.pointer_up(Point::new(10.2, 18.0), false, false);
    assert!(session.text_editing());
    let caret = session.text_sink_state().unwrap().1;
    let steps = labels(&session).len();
    assert_eq!(
        session.set_text_layout_text(LayoutField::WordSpacing, "1"),
        LayoutOutcome::Set
    );
    assert!(session.text_editing());
    assert_eq!(session.text_sink_state().unwrap().1, caret);
    assert_eq!(labels(&session).len(), steps + 1);
    session.set_text_alignment(TextAlign::Right);
    assert!(session.text_editing());
    session.text_input(TextInput::Insert("!".to_owned()));
    session.escape();
    let text = only_text(&session);
    assert!(text.string.contains('!'));
    assert_eq!(text.layout.align, TextAlign::Right);
    assert!((text.layout.word_spacing - 0.1).abs() < 1e-9);
}

/// Criterion 35: Object to path of a centred, spaced, turned text gives the drawn outlines.
#[test]
fn ac35_object_to_path_matches_the_drawing() {
    let mut session = with_text("Hi there\nAV");
    session.set_text_alignment(TextAlign::Centre);
    session.set_text_layout_text(LayoutField::LetterSpacing, "1");
    session.set_text_layout_text(LayoutField::CharacterRotation, "20");
    session.set_text_layout_text(LayoutField::Taper, "20");
    let drawn = only_text(&session).outline.unwrap().glyphs.clone();
    session.set_tool(Tool::Select);
    let _ = session.object_to_path();
    let mut points: Vec<Point> = session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Path(path) => Some(path.clone()),
            _ => None,
        })
        .flat_map(|path| {
            let mut all: Vec<Point> = path.anchors.iter().map(|a| a.point).collect();
            for sub in &path.extra_subpaths {
                all.extend(sub.anchors.iter().map(|a| a.point));
            }
            all
        })
        .collect();
    let mut expected: Vec<Point> = drawn.anchors.iter().map(|a| a.point).collect();
    for sub in &drawn.extra_subpaths {
        expected.extend(sub.anchors.iter().map(|a| a.point));
    }
    assert_ne!(points.len(), 0);
    let order = |a: &Point, b: &Point| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y));
    points.sort_by(order);
    expected.sort_by(order);
    assert_eq!(points.len(), expected.len());
    for (a, b) in points.iter().zip(&expected) {
        assert!((a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
    }
}

/// Criterion 16: the Taper field reads and writes percent in one step "Change taper"; a value
/// beyond 50 % is set to 50 % and reported; the reset returns to 0 %; the drawing changes.
#[test]
fn ac16_taper_in_one_step() {
    let mut session = with_text("Ring");
    let before = only_text(&session).outline.unwrap().glyphs.clone();
    assert_eq!(session.text_layout_block().unwrap()[4].text, "0");
    assert_eq!(
        session.set_text_layout_text(LayoutField::Taper, "20 %"),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session).last().unwrap(), "change_taper");
    assert!((only_text(&session).layout.taper - 0.2).abs() < 1e-12);
    assert_ne!(only_text(&session).outline.unwrap().glyphs, before);
    assert_eq!(
        session.set_text_layout_text(LayoutField::Taper, "80"),
        LayoutOutcome::Clamped(true, 50.0)
    );
    assert_eq!(session.text_layout_block().unwrap()[4].text, "50");
    session.step_text_layout(LayoutField::Taper, -1, Grid::Coarse);
    assert_eq!(session.commit_text_layout_preview(), LayoutOutcome::Set);
    assert!((only_text(&session).layout.taper - 0.4).abs() < 1e-12);
    assert_eq!(
        session.reset_text_layout(LayoutField::Taper),
        LayoutOutcome::Set
    );
    assert!(only_text(&session).layout.taper.abs() < 1e-12);
}
