//! Session-level tests of the Align group of `specs/0061-align-and-distribute/`:
//! the presses through the real event path, their references, the key
//! object, history steps and undo.

use curvyo_document_core::Point;
use curvyo_ui_core::{
    AlignAxis, AlignCommand, AlignOutcome, AlignReference, DistributeMode, Modifiers,
};

use super::align::{align_command_from_code, distribute_from_code};
use super::align_test_support::{
    box_of, click, draw_rect, last_label, left_x, near, second_page, select_all, select_nothing,
    steps, two_rects,
};
use super::{Session, Tool};

#[test]
fn the_groups_are_in_the_select_bar_in_every_selection_state() {
    let mut session = Session::new(1);
    session.set_tool(Tool::Select);
    // Criterion 15: the empty page keeps the groups.
    assert!(session.align_group_shown());
    assert_eq!(session.grid_count(), 0);
    two_rects(&mut session);
    assert!(session.align_group_shown());
    assert_eq!(session.grid_count(), 2);
    session.set_tool(Tool::Rectangle);
    assert!(!session.align_group_shown());
    assert_eq!(session.grid_count(), 0);
}

#[test]
fn the_reference_follows_the_selected_count() {
    let mut session = Session::new(1);
    let a = draw_rect(&mut session, (0.0, 0.0), (10.0, 10.0));
    draw_rect(&mut session, (20.0, 0.0), (30.0, 10.0));
    draw_rect(&mut session, (40.0, 0.0), (50.0, 10.0));
    select_nothing(&mut session);
    assert_eq!(session.align_reference(), AlignReference::PageObjects);
    assert_eq!(session.distribute_reference(), AlignReference::PageObjects);
    click(&mut session, (5.0, 0.0), false);
    assert_eq!(session.selection.ids(), &[a]);
    assert_eq!(session.align_reference(), AlignReference::Page);
    assert_eq!(session.distribute_reference(), AlignReference::Page);
    click(&mut session, (25.0, 0.0), true);
    assert_eq!(session.align_reference(), AlignReference::Selection);
    assert_eq!(session.distribute_reference(), AlignReference::Page);
    click(&mut session, (45.0, 0.0), true);
    assert_eq!(session.distribute_reference(), AlignReference::Selection);
}

#[test]
fn an_align_is_one_step_and_moves_onto_the_reference() {
    let mut session = Session::new(1);
    two_rects(&mut session);
    assert_eq!(
        session.align_selection(AlignCommand::Left),
        AlignOutcome::Aligned
    );
    assert_eq!(left_x(&session, 1), 0.0);
    assert_eq!(last_label(&session), "align_left");
    // One step: one undo puts both rectangles back.
    assert!(session.undo().changed);
    assert_eq!(left_x(&session, 1), 50.0);
}

#[test]
fn an_alignment_that_changes_nothing_writes_nothing() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (0.0, 0.0), (30.0, 10.0));
    draw_rect(&mut session, (0.0, 20.0), (30.0, 30.0));
    select_all(&mut session);
    assert_eq!(
        session.align_selection(AlignCommand::Left),
        AlignOutcome::Unchanged
    );
    // Nothing was written: the undo takes back the last Draw rectangle.
    assert!(session.undo().changed);
    assert_eq!(session.objects().len(), 1);
}

/// Criterion 14: one object goes to the **active** page (the second of two,
/// off the document origin) for each of the six buttons; a second press of
/// the same button records nothing.
#[test]
fn c14_one_object_aligns_onto_the_active_page_for_every_button() {
    let mut session = Session::new(1);
    let (page_id, page) = second_page(&mut session);
    let id = draw_rect(&mut session, (40.0, 50.0), (60.0, 94.0));
    click(&mut session, (50.0, 50.0), false);
    assert_eq!(session.selection.ids(), &[id]);
    // The click activated the first page; the Pages row activates the second.
    assert_ne!(session.active_page().id, page_id);
    assert!(session.activate_page(&page_id));
    let cases = [
        (AlignCommand::Left, "align_left_page"),
        (AlignCommand::CentreH, "align_centre_h_page"),
        (AlignCommand::Right, "align_right_page"),
        (AlignCommand::Top, "align_top_page"),
        (AlignCommand::CentreV, "align_centre_v_page"),
        (AlignCommand::Bottom, "align_bottom_page"),
    ];
    for (command, label) in cases {
        assert_eq!(
            session.align_selection(command),
            AlignOutcome::Aligned,
            "{label}"
        );
        assert_eq!(last_label(&session), label);
        let box_ = box_of(&session, id);
        near(command.source(&box_), command.target(&page), label);
        let before = steps(&session);
        assert_eq!(
            session.align_selection(command),
            AlignOutcome::Unchanged,
            "{label} again"
        );
        assert_eq!(steps(&session), before, "{label}: no step when in place");
        assert_eq!(session.selection.ids(), &[id], "the selection stays");
    }
    // Width and height are kept; the object ends in the page's bottom right.
    let box_ = box_of(&session, id);
    near(box_.extent(AlignAxis::Horizontal), 20.0, "width");
    near(box_.max.x, page.max.x, "right");
    near(box_.max.y, page.max.y, "bottom");
}

/// Criterion 15: with nothing selected every visible, unlocked object of the
/// active page goes onto the page; the other page's and the locked object
/// stay; one step, undone as one.
#[test]
fn c15_nothing_selected_aligns_only_the_active_page_s_unlocked_objects() {
    let mut session = Session::new(1);
    let other = draw_rect(&mut session, (40.0, 30.0), (60.0, 50.0));
    let (_, page) = second_page(&mut session);
    let a = draw_rect(&mut session, (270.0, 60.0), (290.0, 80.0));
    let b = draw_rect(&mut session, (330.0, 120.0), (360.0, 150.0));
    let locked = draw_rect(&mut session, (300.0, 200.0), (310.0, 210.0));
    let hidden = draw_rect(&mut session, (320.0, 220.0), (330.0, 230.0));
    session.document.set_node_locked(&[locked], true).unwrap();
    session.document.set_node_visible(&[hidden], false).unwrap();
    session.sync_context();
    select_nothing(&mut session);
    let (other_before, locked_before) = (box_of(&session, other), box_of(&session, locked));
    let cases = [
        (AlignCommand::Left, "align_left_page"),
        (AlignCommand::CentreH, "align_centre_h_page"),
        (AlignCommand::Right, "align_right_page"),
        (AlignCommand::Top, "align_top_page"),
        (AlignCommand::CentreV, "align_centre_v_page"),
        (AlignCommand::Bottom, "align_bottom_page"),
    ];
    for (command, label) in cases {
        assert_eq!(
            session.align_selection(command),
            AlignOutcome::Aligned,
            "{label}"
        );
        assert_eq!(last_label(&session), label);
        for id in [a, b] {
            near(
                command.source(&box_of(&session, id)),
                command.target(&page),
                label,
            );
        }
        assert_eq!(box_of(&session, other), other_before, "{label}: other page");
        assert_eq!(box_of(&session, locked), locked_before, "{label}: locked");
        assert!(
            session.selection.ids().is_empty(),
            "the selection stays empty"
        );
    }
    // One undo takes the last align back as one step.
    let bottom = box_of(&session, a).max.y;
    assert!(session.undo().changed);
    assert!((box_of(&session, a).max.y - bottom).abs() > 1.0);
}

/// Criterion 15: an empty active page, or one with only locked objects,
/// changes nothing and adds no step; the groups stay.
#[test]
fn c15_an_empty_or_locked_page_changes_nothing() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (40.0, 30.0), (60.0, 50.0));
    let (page_id, _) = second_page(&mut session);
    select_nothing(&mut session);
    assert_eq!(session.active_page().id, page_id);
    let before = steps(&session);
    assert_ne!(
        session.align_selection(AlignCommand::Left),
        AlignOutcome::Aligned
    );
    assert_ne!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    assert_eq!(steps(&session), before, "the empty page adds no step");
    let locked = draw_rect(&mut session, (300.0, 200.0), (310.0, 210.0));
    session.document.set_node_locked(&[locked], true).unwrap();
    session.sync_context();
    select_nothing(&mut session);
    let before_locked = steps(&session);
    assert_ne!(
        session.align_selection(AlignCommand::Right),
        AlignOutcome::Aligned
    );
    assert_eq!(steps(&session), before_locked);
    assert!(session.align_group_shown());
}

/// The canvas cue: while on, the key mark is the active page's frame.
#[test]
fn the_page_cue_marks_the_active_page() {
    let mut session = Session::new(1);
    let (_, page) = second_page(&mut session);
    session.set_tool(Tool::Select);
    assert!(session.select_decoration_input().key.is_none());
    session.set_page_cue(true);
    let key = session
        .select_decoration_input()
        .key
        .expect("the page is marked");
    assert_eq!(key[0], page.min);
    assert_eq!(key[2], page.max);
    session.set_page_cue(false);
    assert!(session.select_decoration_input().key.is_none());
}

#[test]
fn a_ctrl_click_marks_the_key_and_the_key_is_the_reference() {
    let mut session = Session::new(1);
    two_rects(&mut session);
    // A Ctrl-press on the second marks it as the key (criterion 5).
    session.held = Modifiers::new(false, true);
    session.pointer_down(Point::new(50.0, 5.0), false);
    session.pointer_up(Point::new(50.0, 5.0), false, true);
    session.held = Modifiers::NONE;
    assert!(session.key_object().is_some());
    // Align left: the key keeps its place, the other lands on it.
    assert_eq!(
        session.align_selection(AlignCommand::Left),
        AlignOutcome::Aligned
    );
    assert_eq!(left_x(&session, 0), 50.0);
    assert_eq!(left_x(&session, 1), 50.0);
}

#[test]
fn escape_clears_the_key_before_the_selection() {
    let mut session = Session::new(1);
    two_rects(&mut session);
    session.held = Modifiers::new(false, true);
    session.pointer_down(Point::new(50.0, 5.0), false);
    session.pointer_up(Point::new(50.0, 5.0), false, true);
    session.held = Modifiers::NONE;
    assert!(session.key_object().is_some());
    session.escape();
    assert!(session.key_object().is_none());
    assert!(!session.selection.is_empty());
    session.escape();
    assert!(session.selection.is_empty());
}

#[test]
fn a_clone_moves_through_its_own_matrix_and_the_original_stays() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (0.0, 0.0), (25.0, 40.0));
    draw_rect(&mut session, (50.0, 0.0), (60.0, 40.0));
    // The clone of the second rectangle, at its original's place.
    session.selection.clear();
    session.selection.add(session.objects()[1].id());
    session.linked_clone();
    select_all(&mut session);
    // Ctrl-press on the first rectangle's outline (away from the group
    // box's own handle hit zones) marks it as the key (criterion 5).
    session.held = Modifiers::new(false, true);
    session.pointer_down(Point::new(15.0, 0.0), false);
    session.pointer_up(Point::new(15.0, 0.0), false, true);
    session.held = Modifiers::NONE;
    assert!(session.key_object().is_some());
    // Align right: the key keeps its place, the second rectangle and its
    // clone land on it, the clone through its own placement matrix
    // (criterion 11).
    let outcome = session.align_selection(AlignCommand::Right);
    assert_eq!(outcome, AlignOutcome::Aligned);
    assert_eq!(left_x(&session, 0), 0.0);
    assert_eq!(left_x(&session, 1), 15.0);
    assert_eq!(left_x(&session, 2), 15.0);
}

/// Review change 6: a misspelled command code maps to no command, so the
/// wasm press runs nothing and writes no step.
#[test]
fn an_unknown_command_code_maps_to_no_command() {
    assert_eq!(align_command_from_code("lefft"), None);
    assert_eq!(
        align_command_from_code("bottom"),
        Some(AlignCommand::Bottom)
    );
    assert_eq!(distribute_from_code("x"), None);
    assert_eq!(
        distribute_from_code("v-centres"),
        Some((AlignAxis::Vertical, DistributeMode::Centres))
    );
}
