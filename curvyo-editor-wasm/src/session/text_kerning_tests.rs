//! Kerning per character in the session (`specs/0079-text-layout-controls` slice 4, criteria 26
//! to 29, 35), with the included fonts handed in as the host hands them.

use curvyo_document_core::{ObjectSnapshot, Point, TextSnapshot, TextWritingMode};
use curvyo_ui_core::{CaretMove, Grid, LayoutField, TextInput};

use super::text_layout::LayoutOutcome;
use super::text_tests::{labels, texts, with_text};
use super::{Session, Tool};

fn only_text(session: &Session) -> TextSnapshot {
    texts(session).into_iter().next().unwrap()
}

/// The block's kerning fields: horizontal, vertical.
fn kern_fields(session: &Session) -> (String, String) {
    let block = session.text_layout_block().unwrap();
    (block[5].text.clone(), block[6].text.clone())
}

/// Edit mode on the only text with the caret before character `before` (a byte, ASCII here).
fn edit_at(session: &mut Session, before: usize) {
    let id = only_text(session).id;
    session.edit_text_at(id, None);
    session.text_input(TextInput::Select {
        anchor: before,
        caret: before,
    });
}

/// Criteria 26 and 28: outside edit mode the two fields follow Character rotation and show the
/// common kern of every character; a change sets every character in one step "Change kerning";
/// the reset clears every kern of the texts in one step "Clear kerning".
#[test]
fn ac26_ac28_the_fields_outside_edit_mode() {
    let mut session = with_text("AV\nTo");
    let block = session.text_layout_block().unwrap();
    assert_eq!(block.len(), 8);
    assert_eq!(kern_fields(&session), ("0".to_owned(), "0".to_owned()));
    // Size 10 mm: the horizontal track is -2.5 to 2.5 mm, the vertical -5 to 5 mm.
    assert!((block[5].track_max - 2.5).abs() < 1e-9 && (block[6].track_max - 5.0).abs() < 1e-9);
    assert_eq!(
        session.set_text_layout_text(LayoutField::HorizontalKerning, "0.4"),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session).last().unwrap(), "change_kerning");
    let text = only_text(&session);
    assert_eq!(
        text.chars.entries().len(),
        4,
        "every character but the break"
    );
    assert_eq!(kern_fields(&session).0, "0.4");
    assert_eq!(
        session.set_text_layout_text(LayoutField::VerticalKerning, "50"),
        LayoutOutcome::Clamped(true, 20.0)
    );
    assert!(session.text_layout_block().unwrap()[5].resettable);
    let steps = labels(&session).len();
    assert_eq!(
        session.reset_text_layout(LayoutField::VerticalKerning),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "clear_kerning");
    assert!(only_text(&session).chars.is_empty(), "both kerns cleared");
    session.undo();
    assert_eq!(only_text(&session).chars.entries().len(), 4);
}

/// Criteria 27 and 12: in edit mode the fields show and set the character after the caret; a
/// range shows Mixed; a drag previews and the release writes one step "Change kerning"; at the
/// end of a line a change does nothing.
#[test]
fn ac27_the_fields_follow_the_caret() {
    let mut session = with_text("AVA");
    edit_at(&mut session, 1);
    let steps = labels(&session).len();
    session.preview_text_layout(LayoutField::HorizontalKerning, 0.3, Grid::Normal);
    assert_eq!(
        labels(&session).len(),
        steps,
        "nothing written while dragging"
    );
    assert_eq!(session.commit_text_layout_preview(), LayoutOutcome::Set);
    assert_eq!(labels(&session)[steps..], ["change_kerning"]);
    let kerned = only_text(&session).chars;
    assert_eq!(kerned.entries().len(), 1);
    assert!(
        kerned.get(1).kern_x < 0.0,
        "the V, left of the middle of the track"
    );
    session.text_input(TextInput::Move {
        to: CaretMove::Right,
        extend: true,
    });
    session.text_input(TextInput::Move {
        to: CaretMove::Left,
        extend: false,
    });
    session.text_input(TextInput::Select {
        anchor: 0,
        caret: 3,
    });
    assert!(session.text_layout_block().unwrap()[5].mixed);
    session.text_input(TextInput::Select {
        anchor: 3,
        caret: 3,
    });
    assert_eq!(kern_fields(&session).0, "0");
    let steps = labels(&session).len();
    assert_eq!(
        session.set_text_layout_text(LayoutField::HorizontalKerning, "1"),
        LayoutOutcome::Unchanged
    );
    assert_eq!(labels(&session).len(), steps, "end of the line: nothing");
}

/// Criterion 29: Alt+arrows kern the character after the caret by 0.1 mm, Shift 1 mm, as part
/// of the edit session, which writes one step "Edit text"; the readout says the kern.
#[test]
fn ac29_alt_arrows_in_the_edit_session() {
    let mut session = with_text("AV");
    edit_at(&mut session, 1);
    assert_eq!(session.text_kern(-1, 0, false), "Kern \u{2212}0.1, 0.0 mm");
    assert_eq!(session.text_kern(-1, 0, false), "Kern \u{2212}0.2, 0.0 mm");
    assert_eq!(session.text_kern(0, 1, true), "Kern \u{2212}0.2, +1.0 mm");
    assert_eq!(
        kern_fields(&session),
        ("\u{2212}0.2".to_owned(), "1".to_owned())
    );
    let steps = labels(&session).len();
    session.escape();
    assert_eq!(labels(&session)[steps..], ["edit_text"]);
    let kern = only_text(&session).chars.get(1);
    assert!((kern.kern_x + 0.02).abs() < 1e-9 && (kern.kern_y - 0.1).abs() < 1e-9);
    // At the end of the text the keys do nothing.
    edit_at(&mut session, 2);
    assert_eq!(session.text_kern(1, 0, false), "");
}

/// Criterion 29 with 24: in vertical text Alt+Down and Alt+Up kern along the column (the
/// horizontal kern, down is +), Alt+Right and Alt+Left across it (the vertical kern, right is +).
#[test]
fn ac29_alt_arrows_in_vertical_text() {
    let mut session = with_text("AV");
    session.set_text_writing_mode(TextWritingMode::VerticalRl);
    edit_at(&mut session, 1);
    // Alt+Down, then Alt+Right with Shift (the keys as the host passes them: right, up).
    assert_eq!(session.text_kern(0, -1, false), "Kern +0.1, 0.0 mm");
    assert_eq!(session.text_kern(1, 0, true), "Kern +0.1, +1.0 mm");
    assert_eq!(session.text_kern(0, 1, false), "Kern 0.0, +1.0 mm");
    assert_eq!(session.text_kern(-1, 0, false), "Kern 0.0, +0.9 mm");
}

/// Criterion 35: Object to path of a kerned text gives the drawn outlines.
#[test]
fn ac35_object_to_path_keeps_the_kerns() {
    let mut session = with_text("AVA");
    edit_at(&mut session, 1);
    session.text_kern(0, 1, true);
    session.text_kern(1, 0, true);
    session.escape();
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

/// Criterion 27b: the per-character rotation acts on the character after the caret in edit mode
/// and on every character outside it, in its own step; its reset and "Clear kerning" each clear
/// only their own values.
#[test]
fn ac27b_per_character_rotation_and_its_own_reset() {
    let mut session = with_text("AVA");
    assert_eq!(session.text_layout_block().unwrap().len(), 8);
    edit_at(&mut session, 1);
    assert_eq!(
        session.set_text_layout_text(LayoutField::CharRotation, "30"),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session).last().unwrap(), "change_char_rotation");
    session.text_kern(1, 0, true);
    session.escape();
    let chars = only_text(&session).chars;
    assert_eq!(chars.entries().len(), 1);
    assert!((chars.get(1).rotation - 30.0).abs() < 1e-9 && chars.get(1).kern_x > 0.0);
    assert_eq!(session.text_layout_block().unwrap()[7].text, "", "Mixed");
    assert_eq!(
        session.reset_text_layout(LayoutField::HorizontalKerning),
        LayoutOutcome::Set
    );
    let chars = only_text(&session).chars;
    assert!((chars.get(1).rotation - 30.0).abs() < 1e-9 && chars.get(1).kern_x.abs() < 1e-12);
    assert_eq!(
        session.set_text_layout_text(LayoutField::CharRotation, "-10"),
        LayoutOutcome::Set
    );
    assert_eq!(
        only_text(&session).chars.entries().len(),
        3,
        "every character"
    );
    session.set_text_layout_text(LayoutField::VerticalKerning, "1");
    assert_eq!(
        session.reset_text_layout(LayoutField::CharRotation),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session).last().unwrap(), "clear_char_rotation");
    let chars = only_text(&session).chars;
    assert!(
        chars
            .entries()
            .iter()
            .all(|(_, v)| v.rotation.abs() < 1e-12 && v.kern_y > 0.0)
    );
}

/// Criterion 27c: "All characters" sets the field's value of the character after the caret on
/// every character of the text, in edit mode, in one step; a vertical kern on every character
/// builds a staircase.
#[test]
fn ac27c_all_characters_in_edit_mode() {
    let mut session = with_text("STEP\nUP");
    edit_at(&mut session, 1);
    session.set_text_layout_text(LayoutField::VerticalKerning, "1");
    let steps = labels(&session).len();
    assert_eq!(
        session.apply_text_layout_to_all(LayoutField::VerticalKerning),
        LayoutOutcome::Set
    );
    assert_eq!(labels(&session)[steps..], ["change_kerning"]);
    session.escape();
    let chars = only_text(&session).chars;
    assert_eq!(chars.entries().len(), 6, "every character but the break");
    assert!(
        chars
            .entries()
            .iter()
            .all(|(_, v)| (v.kern_y - 0.1).abs() < 1e-9)
    );
    // Mixed has no value to apply.
    edit_at(&mut session, 0);
    session.text_input(TextInput::Select {
        anchor: 0,
        caret: 2,
    });
    session.set_text_layout_text(LayoutField::CharRotation, "5");
    session.text_input(TextInput::Select {
        anchor: 1,
        caret: 3,
    });
    assert_eq!(
        session.apply_text_layout_to_all(LayoutField::CharRotation),
        LayoutOutcome::Unchanged
    );
}
