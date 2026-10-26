//! The Style panel on a text in edit mode (customer 2026-10-26, `specs/0074-text-objects`
//! amendment): Fill and Stroke act on the edited text, one step per change, edit mode stays,
//! and a panel drag previews on the text as drawn.

use curvyo_document_core::Point;
use curvyo_ui_core::{Grid, TextInput, ValueField};

use super::text_tests::{labels, session, texts, with_text};
use super::{Session, Tool};

/// A new text "Hi" typed at (10, 20), still in edit mode.
fn typing() -> Session {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.pointer_down(Point::new(10.0, 20.0), false);
    session.pointer_up(Point::new(10.0, 20.0), false, false);
    session.text_input(TextInput::Insert("Hi".to_owned()));
    session
}

/// The edited text as the canvas draws it.
fn drawn(session: &Session) -> curvyo_document_core::TextSnapshot {
    session.laid_out_edit().unwrap()
}

#[test]
fn a_new_text_being_typed_is_the_style_subject() {
    let session = typing();
    assert!(session.text_editing());
    assert_eq!(session.style_panel_view().subject, "Text");
    assert!(session.panel_view().tabs.iter().all(|tab| tab.enabled));
}

#[test]
fn a_fill_switch_on_a_new_text_writes_it_and_keeps_edit_mode() {
    let mut session = typing();
    session.set_fill_paint(true);
    assert!(session.text_editing());
    assert_eq!(labels(&session), ["add_text", "edit_style"]);
    assert!(texts(&session)[0].style.fill.enabled);
    assert!(drawn(&session).style.fill.enabled);
    // The next change is one step; typing goes on in the same text.
    session.set_stroke_paint(false);
    assert_eq!(labels(&session).len(), 3);
    assert!(!drawn(&session).style.stroke.enabled);
    session.text_input(TextInput::Insert("!".to_owned()));
    assert_eq!(drawn(&session).string, "Hi!");
}

#[test]
fn a_fill_switch_on_an_existing_text_in_edit_mode_is_one_step() {
    let mut session = with_text("Name");
    session.pointer_down(Point::new(10.2, 18.0), false);
    session.pointer_up(Point::new(10.2, 18.0), false, false);
    assert!(session.text_editing());
    let steps = labels(&session).len();
    session.set_fill_paint(true);
    assert!(session.text_editing());
    assert_eq!(labels(&session).len(), steps + 1);
    assert!(drawn(&session).style.fill.enabled);
}

#[test]
fn a_width_drag_in_edit_mode_previews_on_the_drawn_text_and_commits_once() {
    let mut session = typing();
    session.set_fill_paint(false);
    let steps = labels(&session).len();
    let before = session.draw_list();
    session.preview_value_field(ValueField::StrokeWidth, 0.9, Grid::Normal);
    assert_eq!(
        labels(&session).len(),
        steps,
        "nothing written during the drag"
    );
    assert_ne!(session.draw_list(), before);
    session.commit_style_preview();
    assert_eq!(labels(&session).len(), steps + 1);
    assert!(session.text_editing());
    assert_eq!(
        drawn(&session).style.stroke.width,
        texts(&session)[0].style.stroke.width
    );
}
