//! The sample the font preview follows (`specs/0077-font-preview` criteria 8 and 9, `adrs.md`
//! decision 5): the Text bar state names the one edited or selected text and its first line.

use curvyo_document_core::Point;
use curvyo_ui_core::TextInput;

use super::text_tests::{session, texts, with_text};
use super::{EscapeStep, Tool};

#[test]
fn no_text_gives_no_sample() {
    let bar = session().text_bar();
    assert_eq!(bar.sample_node, "");
    assert_eq!(bar.sample, "");
}

#[test]
fn one_selected_text_gives_its_first_line() {
    let mut session = with_text("Anna\nJonas");
    assert_eq!(session.escape(), EscapeStep::LeftTool);
    let id = texts(&session)[0].id;
    assert_eq!(session.selection.ids(), [id]);
    let bar = session.text_bar();
    assert_eq!(bar.sample_node, id.full_id());
    assert_eq!(bar.sample, "Anna");
}

#[test]
fn the_edited_text_is_followed_while_typing() {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.pointer_down(Point::new(10.0, 20.0), false);
    session.pointer_up(Point::new(10.0, 20.0), false, false);
    session.text_input(TextInput::Insert("Lea".to_owned()));
    let first = session.text_bar();
    assert_eq!(first.sample, "Lea");
    assert_ne!(first.sample_node, "");
    session.text_input(TextInput::Insert("!".to_owned()));
    let second = session.text_bar();
    assert_eq!(second.sample, "Lea!");
    assert_eq!(second.sample_node, first.sample_node);
}

#[test]
fn two_selected_texts_give_no_sample() {
    let mut session = with_text("Anna");
    session.pointer_down(Point::new(60.0, 60.0), false);
    session.pointer_up(Point::new(60.0, 60.0), false, false);
    session.text_input(TextInput::Insert("Tom".to_owned()));
    assert_eq!(session.escape(), EscapeStep::ClearedState);
    session.set_tool(Tool::Select);
    let _ = session.select_all();
    assert_eq!(texts(&session).len(), 2);
    assert_eq!(session.selection.ids().len(), 2);
    let bar = session.text_bar();
    assert_eq!(bar.sample_node, "");
    assert_eq!(bar.sample, "");
}

#[test]
fn a_long_first_line_is_cut_after_200_characters() {
    let mut session = with_text(&"ä".repeat(201));
    session.escape();
    assert_eq!(session.text_bar().sample.chars().count(), 200);
}
