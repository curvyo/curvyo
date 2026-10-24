//! The Writing mode and Glyph orientation groups in the session (`specs/0079-text-layout-controls`
//! slice 3: criteria 20, 21, 24, 33, 35), with the included fonts handed in as the host hands
//! them.

use curvyo_document_core::{
    ObjectSnapshot, Point, TextGlyphOrientation, TextSnapshot, TextWritingMode,
};
use curvyo_ui_core::{CaretMove, TextInput};

use super::text_tests::{labels, session, texts, with_text};
use super::{Session, Tool};

fn only_text(session: &Session) -> TextSnapshot {
    texts(session).into_iter().next().unwrap()
}

/// The document point where the first line (or column) starts.
fn first_start(session: &Session, text: &TextSnapshot) -> Point {
    let face = session.face_of(&text.font).unwrap();
    let laid = curvyo_ui_core::text_layout(text, face).unwrap();
    let line = &laid.lines[0];
    let p = laid.frame() * kurbo::Point::new(line.extent.start.0, line.baseline.0);
    text.placement.apply_point(Point::new(p.x, p.y))
}

/// Criterion 20: a writing mode press is one step "Change writing mode" for the selected text,
/// which stays in place (the first column starts where the first line started); Undo puts it
/// back; a press on the pressed button writes nothing.
#[test]
fn ac20_writing_mode_keeps_the_text_in_place() {
    let mut session = with_text("AB\nC");
    let before = only_text(&session);
    assert_eq!(
        session.text_writing_mode(),
        Some(TextWritingMode::Horizontal)
    );
    session.set_text_writing_mode(TextWritingMode::VerticalRl);
    let after = only_text(&session);
    assert_eq!(after.layout.writing_mode, TextWritingMode::VerticalRl);
    assert_eq!(labels(&session).last().unwrap(), "change_writing_mode");
    let (a, b) = (
        first_start(&session, &before),
        first_start(&session, &after),
    );
    assert!(a.vector_to(b).length() < 0.01, "{a:?} {b:?}");
    let steps = labels(&session).len();
    session.set_text_writing_mode(TextWritingMode::VerticalRl);
    assert_eq!(labels(&session).len(), steps);
    session.undo();
    assert_eq!(
        only_text(&session).layout.writing_mode,
        TextWritingMode::Horizontal
    );
    assert_eq!(only_text(&session).placement, before.placement);
}

/// Criteria 20 and 21: the orientation is its own step, kept on a horizontal text and used once
/// it is set vertical; the drawing (Object to path's source) follows it.
#[test]
fn ac21_orientation_is_kept_and_takes_effect_when_vertical() {
    let mut session = with_text("AB");
    session.set_text_glyph_orientation(TextGlyphOrientation::Upright);
    assert_eq!(labels(&session).last().unwrap(), "change_glyph_orientation");
    let flat = only_text(&session);
    assert_eq!(flat.layout.orientation, TextGlyphOrientation::Upright);
    session.set_text_writing_mode(TextWritingMode::VerticalLr);
    let upright = only_text(&session);
    session.set_text_glyph_orientation(TextGlyphOrientation::Sideways);
    let sideways = only_text(&session);
    let height = |text: &TextSnapshot| {
        let face = session.face_of(&text.font).unwrap();
        let outline = curvyo_ui_core::lay_out(text, face, false).unwrap();
        let ys: Vec<f64> = outline
            .glyphs
            .anchors
            .iter()
            .chain(
                outline
                    .glyphs
                    .extra_subpaths
                    .iter()
                    .flat_map(|s| &s.anchors),
            )
            .map(|a| a.point.y)
            .collect();
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - ys.iter().copied().fold(f64::INFINITY, f64::min)
    };
    // Upright stacks A over B one line height apart; sideways they lie along the column.
    assert!(height(&upright) > height(&sideways) + 1.0);
    session.set_text_writing_mode(TextWritingMode::Horizontal);
    assert_eq!(
        only_text(&session).layout.orientation,
        TextGlyphOrientation::Sideways
    );
}

/// Criterion 33: with no subject, the presses set the next text's writing mode and orientation
/// without a step.
#[test]
fn ac33_the_next_text_takes_the_writing_mode() {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.set_text_writing_mode(TextWritingMode::VerticalRl);
    session.set_text_glyph_orientation(TextGlyphOrientation::Upright);
    assert_eq!(labels(&session), Vec::<String>::new());
    assert_eq!(
        session.text_writing_mode(),
        Some(TextWritingMode::VerticalRl)
    );
    session.pointer_down(Point::new(50.0, 20.0), false);
    session.pointer_up(Point::new(50.0, 20.0), false, false);
    session.text_input(TextInput::Insert("Up".to_owned()));
    session.escape();
    let text = only_text(&session);
    assert_eq!(text.layout.writing_mode, TextWritingMode::VerticalRl);
    assert_eq!(text.layout.orientation, TextGlyphOrientation::Upright);
    assert!(matches!(session.objects()[0], ObjectSnapshot::Text(_)));
}

/// Criterion 24: in a right-to-left column Up and Down move along it, Left goes to the next
/// column and Right back; Home and End go to its top and bottom.
#[test]
fn ac24_caret_keys_in_a_column() {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.set_text_writing_mode(TextWritingMode::VerticalRl);
    session.pointer_down(Point::new(50.0, 20.0), false);
    session.pointer_up(Point::new(50.0, 20.0), false, false);
    session.text_input(TextInput::Insert("AB\nCD".to_owned()));
    let caret = |session: &Session| session.text_sink_state().unwrap().1;
    let go = |session: &mut Session, to: CaretMove| {
        session.text_input(TextInput::Move { to, extend: false });
    };
    go(&mut session, CaretMove::Up);
    assert_eq!(caret(&session), 4, "Up: one character up the column");
    go(&mut session, CaretMove::Right);
    assert_eq!(caret(&session), 1, "Right: the previous column");
    go(&mut session, CaretMove::Down);
    assert_eq!(caret(&session), 2);
    go(&mut session, CaretMove::Left);
    assert_eq!(caret(&session), 5, "Left: the next column");
    go(&mut session, CaretMove::LineStart);
    assert_eq!(caret(&session), 3);
}
