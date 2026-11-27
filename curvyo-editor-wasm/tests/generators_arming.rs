//! Slice 1 of `specs/0151-generator-ui-polish`: entering the Generators tool arms nothing, a
//! press on a segment arms it, a second press releases it, Escape releases first and leaves
//! second, and an armed generator stays armed after a placement (customer decision 2026-11-27,
//! **(Proposal)**, over criterion 5's "disarmed after one placement").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, pack};
use curvyo_editor_wasm::{EscapeStep, Session, Tool};

const BOX: &str = "curvyo.generators/box";
const GEAR: &str = "curvyo.generators/gear";

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session.set_tool(Tool::Generators);
    session
}

/// The generated objects in the document, read from a saved copy.
fn objects(session: &Session) -> usize {
    let saved = session.pack("0.1.0").unwrap();
    curvyo_document_core::unpack(3, &saved)
        .unwrap()
        .read_objects()
        .objects
        .len()
}

#[test]
fn entering_the_tool_arms_nothing_and_a_click_places_nothing() {
    let mut s = session();
    assert_eq!(s.generator_kind(), "", "no segment is armed");
    assert!(!s.place_generated(100.0, 100.0));
    assert_eq!(objects(&s), 0);
    assert_eq!(s.tool(), Tool::Generators);
    // Armed, left and entered again: nothing is kept.
    s.toggle_generator_kind(BOX);
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Generators);
    assert_eq!(s.generator_kind(), "");
}

#[test]
fn a_press_arms_a_second_press_releases_and_another_segment_moves_the_arming() {
    let mut s = session();
    s.toggle_generator_kind(BOX);
    assert_eq!(s.generator_kind(), BOX);
    s.toggle_generator_kind(GEAR);
    assert_eq!(s.generator_kind(), GEAR, "exactly one is armed");
    s.toggle_generator_kind(GEAR);
    assert_eq!(
        s.generator_kind(),
        "",
        "the armed segment pressed again releases it"
    );
    assert!(!s.place_generated(100.0, 100.0));
    assert_eq!(s.tool(), Tool::Generators, "the tool stays");
}

#[test]
fn an_armed_generator_stays_armed_after_a_placement_so_several_can_be_placed() {
    let mut s = session();
    s.toggle_generator_kind(GEAR);
    assert!(s.place_generated(100.0, 100.0));
    assert!(s.place_generated(200.0, 100.0));
    assert_eq!(objects(&s), 2);
    assert_eq!(s.generator_kind(), GEAR);
    assert_eq!(s.tool(), Tool::Generators, "the tool stays active");
    assert_eq!(s.selected_object_count(), 1, "the new object is selected");
}

#[test]
fn escape_releases_the_armed_generator_first_and_the_second_leaves_the_tool() {
    let mut s = session();
    s.toggle_generator_kind(BOX);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.generator_kind(), "");
    assert_eq!(s.tool(), Tool::Generators);
    assert_eq!(s.escape(), EscapeStep::LeftTool);
    assert_eq!(s.tool(), Tool::Select);
}
