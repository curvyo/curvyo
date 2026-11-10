#![allow(clippy::assert_is_empty)]

//! Session-level tests of `specs/0101-laser-operations` criteria 2 to 10, 14 to 16 and 23 (the
//! commands, the assignment rules and their history labels).

use curvyo_document_core::{Direction, NodeId, OperationSnapshot, OperationSwitch};

use super::super::Session;
use super::OperationsViewMode;
use crate::session::align_test_support::draw_rect;

/// A drawing with two rectangles, and a laser page made active.
fn laser_with_two() -> (Session, NodeId, NodeId) {
    let mut s = Session::new(300_001);
    let a = draw_rect(&mut s, (10.0, 10.0), (30.0, 30.0));
    let b = draw_rect(&mut s, (40.0, 10.0), (60.0, 30.0));
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.selection.set(&[]);
    (s, a, b)
}

fn ops(s: &Session) -> Vec<OperationSnapshot> {
    let manufacturing = s.active_manufacturing().unwrap();
    s.document.operations(manufacturing.id).unwrap()
}

fn names(s: &Session) -> Vec<String> {
    ops(s).into_iter().map(|op| op.name).collect()
}

fn id_named(s: &Session, name: &str) -> NodeId {
    ops(s).into_iter().find(|op| op.name == name).unwrap().id
}

/// Marks `id` as a guide, as a step of the session (so no step is left unrecorded).
fn guide(s: &mut Session, id: NodeId) {
    s.recording_unguarded(|this| this.document.set_guide(&[id], true).unwrap());
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

#[test]
fn criterion_2_a_kind_press_adds_the_first_cut_with_both_switches_on() {
    let (mut s, _, _) = laser_with_two();
    let steps = s.undo_len();
    let id = s.press_operation_kind("cut").unwrap();
    let op = ops(&s).remove(0);
    assert_eq!(op.id, id);
    assert_eq!(op.name, "Cut 1");
    assert!(op.show && op.output);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Add operation");
}

#[test]
fn criterion_2_names_count_per_kind_in_list_order() {
    let (mut s, _, _) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    s.press_operation_kind("cut").unwrap();
    s.press_operation_kind("score").unwrap();
    assert_eq!(names(&s), ["Score 1", "Cut 1", "Cut 2"]);
}

#[test]
fn criterion_5_a_new_operation_lands_at_the_end_of_its_section() {
    let (mut s, _, _) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    s.press_operation_kind("engrave").unwrap();
    s.press_operation_kind("score").unwrap();
    assert_eq!(names(&s), ["Engrave 1", "Score 1", "Cut 1"]);
}

#[test]
fn criterion_8_a_kind_press_with_items_selected_assigns_them_in_one_step() {
    let (mut s, a, _) = laser_with_two();
    s.selection.set(&[a]);
    let id = s.press_operation_kind("cut").unwrap();
    assert_eq!(ops(&s)[0].members, vec![vec![a]]);
    assert_eq!(ops(&s)[0].id, id);
    assert_eq!(undo_label(&mut s), "Assign to operation");
}

#[test]
fn criterion_10_guides_are_not_targets_and_a_notice_names_them() {
    let (mut s, a, b) = laser_with_two();
    guide(&mut s, a);
    s.selection.set(&[a, b]);
    s.press_operation_kind("cut").unwrap();
    assert_eq!(ops(&s)[0].members, vec![vec![b]]);
    assert_eq!(
        s.operations_notice().as_deref(),
        Some("1 guide not assigned. Guides never run.")
    );
}

#[test]
fn criterion_10_a_selection_of_only_guides_adds_nothing_and_writes_no_step() {
    let (mut s, a, _) = laser_with_two();
    guide(&mut s, a);
    s.selection.set(&[a]);
    let steps = s.undo_len();
    assert!(s.press_operation_kind("cut").is_none());
    assert!(ops(&s).is_empty());
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.operations_notice().as_deref(), Some("Guides never run."));
}

#[test]
fn criterion_3_rename_reverts_on_an_empty_name_and_is_one_step() {
    let (mut s, _, _) = laser_with_two();
    let id = s.press_operation_kind("cut").unwrap();
    assert!(!s.rename_operation(id, "   "));
    assert_eq!(names(&s), ["Cut 1"]);
    assert!(s.rename_operation(id, "Outline"));
    assert_eq!(names(&s), ["Outline"]);
    assert_eq!(undo_label(&mut s), "Rename operation");
}

// Customer 2026-11-10: no operation colour (the colour test of criterion 3 was removed).

#[test]
fn criterion_4_delete_drops_the_operation_and_undo_brings_it_back_with_its_members() {
    let (mut s, a, _) = laser_with_two();
    s.selection.set(&[a]);
    let id = s.press_operation_kind("cut").unwrap();
    assert!(s.delete_operation(id));
    assert!(ops(&s).is_empty());
    assert_eq!(undo_label(&mut s), "Delete operation");
    s.undo();
    assert_eq!(ops(&s)[0].members, vec![vec![a]]);
}

#[test]
fn criterion_27_deleting_a_target_drops_its_assignment_with_no_notice() {
    let (mut s, a, _) = laser_with_two();
    s.selection.set(&[a]);
    s.press_operation_kind("cut").unwrap();
    s.recording_unguarded(|this| this.document.delete_objects(&[a]).unwrap());
    assert!(ops(&s)[0].members.is_empty());
    assert_eq!(s.operations_notice(), None);
}

#[test]
fn criterion_8_tick_adds_the_selection_and_untick_removes_it() {
    let (mut s, a, _) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    let id = id_named(&s, "Cut 1");
    s.selection.set(&[a]);
    assert!(s.set_selection_in_operation(id, true));
    assert_eq!(undo_label(&mut s), "Assign to operation");
    s.undo();
    assert!(s.set_selection_in_operation(id, true));
    assert!(s.set_selection_in_operation(id, false));
    assert!(ops(&s)[0].members.is_empty());
    assert_eq!(undo_label(&mut s), "Remove from operation");
}

#[test]
fn criterion_8_number_keys_toggle_the_selection_in_operation_n() {
    let (mut s, a, b) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    s.press_operation_kind("score").unwrap();
    s.selection.set(&[a, b]);
    assert!(!s.toggle_selection_by_number(9));
    assert!(s.toggle_selection_by_number(2));
    assert_eq!(ops(&s)[1].members.len(), 2);
    assert!(s.toggle_selection_by_number(2));
    assert!(ops(&s)[1].members.is_empty());
}

#[test]
fn criterion_9_removing_from_one_operation_keeps_the_other_memberships() {
    let (mut s, a, _) = laser_with_two();
    s.selection.set(&[a]);
    s.press_operation_kind("cut").unwrap();
    s.selection.set(&[]);
    s.press_operation_kind("score").unwrap();
    s.selection.set(&[a]);
    let cut = id_named(&s, "Cut 1");
    assert!(s.set_selection_in_operation(id_named(&s, "Score 1"), true));
    assert!(s.set_selection_in_operation(cut, false));
    let all = ops(&s);
    assert!(all[1].members.is_empty());
    assert_eq!(all[0].members, vec![vec![a]]);
}

#[test]
fn criterion_11_a_clone_member_is_assigned_by_its_path_not_by_its_own_id() {
    let (mut s, a, b) = laser_with_two();
    let clone = s.recording_unguarded(|this| {
        let group = this.document.group_objects(&[a, b]).unwrap();
        this.document.linked_clone(&[group], &[]).unwrap()[0]
    });
    let clone_member = s.document.clone_member_path(clone, a).unwrap();
    assert_eq!(clone_member, vec![clone, a]);
    s.press_operation_kind("cut").unwrap();
    let id = id_named(&s, "Cut 1");
    assert!(s.set_clone_member_in_operation(id, clone, a, true));
    assert_eq!(ops(&s)[0].members, vec![vec![clone, a]]);
    assert!(!ops(&s)[0].members.contains(&vec![a]));
}

#[test]
fn criterion_6_cuts_last_off_moves_nothing_and_on_puts_the_cutting_below_the_marking() {
    let (mut s, _, _) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    assert!(s.set_cuts_last(false));
    assert_eq!(undo_label(&mut s), "Cuts last off");
    s.undo();
    assert!(s.set_cuts_last(false));
    s.press_operation_kind("score").unwrap();
    assert_eq!(names(&s), ["Cut 1", "Score 1"]);
    assert!(s.set_cuts_last(true));
    assert_eq!(names(&s), ["Score 1", "Cut 1"]);
    assert_eq!(undo_label(&mut s), "Cuts last on");
}

#[test]
fn criterion_5_with_cuts_last_on_a_move_inside_a_section_only() {
    let (mut s, _, _) = laser_with_two();
    s.press_operation_kind("score").unwrap();
    s.press_operation_kind("cut").unwrap();
    s.press_operation_kind("cut").unwrap();
    let cut2 = id_named(&s, "Cut 2");
    let score = id_named(&s, "Score 1");
    assert!(!s.move_operation(cut2, 0));
    assert!(!s.move_operation(score, 1));
    assert_eq!(names(&s), ["Score 1", "Cut 1", "Cut 2"]);
    assert!(s.move_operation(cut2, 1));
    assert_eq!(names(&s), ["Score 1", "Cut 2", "Cut 1"]);
    assert_eq!(undo_label(&mut s), "Move operation");
}

#[test]
fn criterion_6_with_cuts_last_off_any_operation_moves_anywhere() {
    let (mut s, _, _) = laser_with_two();
    s.press_operation_kind("cut").unwrap();
    assert!(s.set_cuts_last(false));
    s.press_operation_kind("score").unwrap();
    let score = id_named(&s, "Score 1");
    assert!(s.move_operation(score, 0));
    assert_eq!(names(&s), ["Score 1", "Cut 1"]);
}

#[test]
fn criterion_15_and_16_show_and_output_are_stored_with_their_own_labels() {
    let (mut s, _, _) = laser_with_two();
    let id = s.press_operation_kind("cut").unwrap();
    assert!(s.set_operation_switch(id, OperationSwitch::Show, false));
    assert_eq!(undo_label(&mut s), "Show or hide operation");
    s.undo();
    assert!(s.set_operation_switch(id, OperationSwitch::Output, false));
    assert!(!ops(&s)[0].output);
    assert_eq!(undo_label(&mut s), "Output on or off");
}

#[test]
fn criterion_23_the_margin_is_zero_or_positive_and_one_step() {
    let (mut s, _, _) = laser_with_two();
    let id = s.press_operation_kind("cut_out").unwrap();
    assert!(!s.set_operation_margin(id, -1.0));
    assert!(s.set_operation_margin(id, 2.5));
    assert!((ops(&s)[0].margin.as_mm() - 2.5).abs() < 1e-9);
    assert_eq!(undo_label(&mut s), "Set margin");
}

#[test]
fn criterion_9_an_operation_of_another_laser_page_is_never_touched() {
    let (mut s, _, _) = laser_with_two();
    let first = s.press_operation_kind("cut").unwrap();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(ops(&s).is_empty());
    assert!(!s.delete_operation(first));
}

#[test]
fn criterion_1_the_commands_need_an_active_laser_page() {
    let (mut s, _, _) = laser_with_two();
    s.activate_page(curvyo_document_core::FIRST_PAGE_ID);
    assert!(s.press_operation_kind("cut").is_none());
    assert!(!s.set_cuts_last(false));
}

#[test]
fn an_unknown_kind_text_adds_nothing() {
    let (mut s, _, _) = laser_with_two();
    assert!(s.press_operation_kind("plasma").is_none());
    assert!(ops(&s).is_empty());
}

/// A laser page made active, with two squares drawn on its output page; returns them.
fn laser_with_two_on_the_page() -> (Session, NodeId, NodeId) {
    use curvyo_document_core::{Point, RectBounds};
    let mut s = Session::new(300_002);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let o = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .origin;
    let square = |x: f64| {
        RectBounds::from_corners(
            Point::new(o.x + x, o.y + 10.0),
            Point::new(o.x + x + 10.0, o.y + 20.0),
        )
    };
    let (a, b) = s.recording_unguarded(|this| {
        (
            this.document.create_rect(square(10.0)),
            this.document.create_rect(square(30.0)),
        )
    });
    s.selection.set(&[]);
    (s, a, b)
}

#[test]
fn criterion_12_and_17_clone_members_are_the_leaves_of_a_cloned_group() {
    let (mut s, a, b) = laser_with_two_on_the_page();
    let clone = s.recording_unguarded(|this| {
        let group = this.document.group_objects(&[a, b]).unwrap();
        this.document.linked_clone(&[group], &[]).unwrap()[0]
    });
    s.selection.set(&[]);
    s.press_operation_kind("cut").unwrap();
    let id = id_named(&s, "Cut 1");
    // The clone as a whole: its two members run (criterion 12, nearest rule), the two originals
    // of the group are own items with no assignment.
    s.selection.set(&[clone]);
    assert!(s.set_selection_in_operation(id, true));
    let summary = s.operations_view().unwrap().summary;
    assert_eq!(
        (summary.running, summary.not_assigned),
        (2, 2),
        "{summary:?}"
    );
    // One member by its path: the other member and the clone's originals stay out.
    s.selection.set(&[clone]);
    assert!(s.set_selection_in_operation(id, false));
    s.selection.set(&[]);
    assert!(s.set_clone_member_in_operation(id, clone, a, true));
    let summary = s.operations_view().unwrap().summary;
    assert_eq!(
        (summary.running, summary.not_assigned),
        (1, 3),
        "{summary:?}"
    );
}

#[test]
fn criterion_17_each_count_lists_its_items_and_a_count_button_selects_them() {
    let (mut s, a, b) = laser_with_two_on_the_page();
    s.selection.set(&[a]);
    s.press_operation_kind("cut").unwrap();
    s.selection.set(&[]);
    let view = s.operations_view().unwrap();
    assert_eq!(view.items.running, vec![a]);
    assert_eq!(view.items.not_assigned, vec![b]);
    assert!(view.items.runs_while_hidden.is_empty());
    assert_eq!(view.items.running.len(), view.summary.running);
    assert_eq!(view.items.not_assigned.len(), view.summary.not_assigned);
    assert!(s.select_operation_items("not_assigned"));
    assert_eq!(s.selection.ids(), &[b]);
    assert!(s.select_operation_items("running"));
    assert_eq!(s.selection.ids(), &[a]);
    assert!(!s.select_operation_items("nothing"));
    assert_eq!(s.selection.ids(), &[a], "an unknown count changes nothing");
}

#[test]
fn criterion_11_clone_members_are_rows_that_show_the_operations_they_are_in() {
    let (mut s, a, b) = laser_with_two_on_the_page();
    let clone = s.recording_unguarded(|this| {
        let group = this.document.group_objects(&[a, b]).unwrap();
        this.document.linked_clone(&[group], &[]).unwrap()[0]
    });
    s.selection.set(&[]);
    let id = s.press_operation_kind("cut").unwrap();
    assert!(s.set_clone_member_in_operation(id, clone, a, true));
    let view = s.operations_view().unwrap();
    assert_eq!(
        view.clone_members.len(),
        2,
        "one row per member of the clone"
    );
    let in_cut = view.clone_members.iter().find(|m| m.member == a).unwrap();
    assert_eq!(in_cut.clone, clone);
    assert_eq!(in_cut.operations, vec![id]);
    let other = view.clone_members.iter().find(|m| m.member == b).unwrap();
    assert!(other.operations.is_empty());
    assert_eq!(
        in_cut.clone_name, other.clone_name,
        "members of one clone share its name"
    );
}

#[test]
fn the_view_switch_is_session_state_and_writes_no_step() {
    let (mut s, _, _) = laser_with_two_on_the_page();
    let steps = s.undo_len();
    assert_eq!(
        s.operations_view().unwrap().view_mode,
        OperationsViewMode::Operations
    );
    assert!(s.set_operations_view_mode(OperationsViewMode::Drawn));
    assert_eq!(
        s.operations_view().unwrap().view_mode,
        OperationsViewMode::Drawn
    );
    assert_eq!(s.undo_len(), steps, "a view switch is not a step");
}

#[test]
fn criterion_1_the_operations_tab_is_after_document_on_a_laser_page_only() {
    let (mut s, _, _) = laser_with_two_on_the_page();
    let names: Vec<_> = s.panel_view().tabs.iter().map(|t| t.name).collect();
    let at = names.iter().position(|n| *n == "operations").unwrap();
    assert_eq!(names[at - 1], "document");
    assert_eq!(s.panel_view().tabs[at].label, "Operations");
    assert!(s.press_panel_tab("operations"));
    assert_eq!(
        s.panel_view().content,
        curvyo_ui_core::PanelContent::Operations
    );
    s.add_page_of_kind("drawing", None).unwrap();
    let names: Vec<_> = s.panel_view().tabs.iter().map(|t| t.name).collect();
    assert!(!names.contains(&"operations"), "{names:?}");
    assert!(!s.press_panel_tab("operations"));
}
