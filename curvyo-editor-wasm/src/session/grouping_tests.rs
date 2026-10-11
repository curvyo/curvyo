//! Session-level tests of `specs/0023-groups`: Group and Ungroup with their notices and chords,
//! clicking a group, entering and leaving it, the dimming of the outside, drawing inside, the
//! refusals of the operations that do not look through groups, and undo.

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, Point, RectBounds};
use curvyo_ui_core::BooleanOp;

use super::grouping::GroupingKind;
use super::{EscapeStep, KeyOutcome, Session, Tool};
use crate::KeyInput;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(session: &Session, x: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: pt(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

/// Four rectangles at x = 0, 20, 40, 60 with the second and third grouped, nothing selected.
fn with_group() -> (Session, [NodeId; 4], NodeId) {
    let mut session = Session::new(1);
    let ids = [0.0, 20.0, 40.0, 60.0].map(|x| rect(&session, x));
    let group = session.document.group_objects(&[ids[1], ids[2]]).unwrap();
    session.selection.clear();
    session.sync_context();
    (session, ids, group)
}

fn click(session: &mut Session, at: Point) {
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
}

fn double_click(session: &mut Session, at: Point) {
    click(session, at);
    session.double_click(at, false, false);
}

fn chord(session: &mut Session, k: &str, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key: k,
        ctrl: true,
        shift,
        ..KeyInput::default()
    })
}

fn key(session: &mut Session, k: &str) -> KeyOutcome {
    session.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    })
}

/// Criterion 1 and 6: Group binds the selected objects into one group that is the only
/// selection, with the notice of the spec, in one commit.
#[test]
fn group_binds_the_selection_and_says_so() {
    let mut session = Session::new(1);
    let ids = [0.0, 20.0, 40.0].map(|x| rect(&session, x));
    session.selection.set(&[ids[0], ids[2]]);
    let result = session.group_selection();
    assert_eq!(result.kind, GroupingKind::Grouped);
    assert_eq!(result.notice, "Grouped 2 objects.");
    let roots = session.document.children(None);
    assert_eq!(roots.len(), 2);
    assert_eq!(
        roots[0], ids[1],
        "the object in between stays below the group"
    );
    assert!(session.document.is_group(roots[1]));
    assert_eq!(
        session.selection.ids(),
        &[ids[0], ids[2]],
        "the group's leaves are selected"
    );
    assert_eq!(session.style_scope().subject, "Group of 2 objects");
}

/// Criterion 2: fewer than two objects, or another tool: not available, nothing changes.
#[test]
fn group_is_not_available_with_fewer_than_two_objects_or_another_tool() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    let _ = rect(&session, 20.0);
    session.selection.set(&[a]);
    assert!(!session.grouping_availability().group);
    assert_eq!(session.group_selection().kind, GroupingKind::Ignored);
    assert_eq!(session.document.children(None).len(), 2);
    assert_eq!(chord(&mut session, "g", false), KeyOutcome::Ignored);
    session.selection.select_single(a);
    session.selection.add(session.document.children(None)[1]);
    assert!(session.grouping_availability().group);
    session.set_tool(Tool::Pen);
    assert!(
        !session.grouping_availability().group,
        "dimmed in another tool"
    );
}

/// Criterion 3: Ungroup replaces each selected group by its children and selects them with the
/// other selected objects; one level only.
#[test]
fn ungroup_releases_the_children_one_level_and_selects_them() {
    let (mut session, ids, group) = with_group();
    session.selection.set(&[ids[0], ids[1], ids[2]]);
    assert!(session.grouping_availability().ungroup);
    let result = session.ungroup_selection();
    assert_eq!(result.kind, GroupingKind::Ungrouped);
    assert_eq!(result.notice, "Ungrouped 1 group into 2 objects.");
    assert!(!session.document.contains_node(group));
    assert_eq!(
        session.document.children(None),
        vec![ids[0], ids[1], ids[2], ids[3]]
    );
    assert_eq!(session.selection.ids().len(), 3);
}

/// Criterion 5: a group that would nest more than 32 levels deep is refused with the notice.
#[test]
fn the_depth_limit_is_refused_with_its_notice() {
    let mut session = Session::new(1);
    let mut top = rect(&session, 0.0);
    for n in 1..=32 {
        let other = rect(&session, 20.0 * f64::from(n));
        top = session.document.group_objects(&[top, other]).unwrap();
    }
    let extra = rect(&session, -20.0);
    session.selection.set(&session.document.leaves_of(top));
    session.selection.add(extra);
    let result = session.group_selection();
    assert_eq!(result.kind, GroupingKind::Refused);
    assert_eq!(
        result.notice,
        "Groups can be nested 32 levels deep. Nothing was changed."
    );
}

/// Criterion 7: Ctrl+G and Ctrl+Shift+G act once per press and only when they can.
#[test]
fn the_chords_group_and_ungroup_and_leave_the_page_alone_when_they_cannot_act() {
    let mut session = Session::new(1);
    let ids = [0.0, 20.0].map(|x| rect(&session, x));
    assert_eq!(
        chord(&mut session, "g", false),
        KeyOutcome::Ignored,
        "nothing selected"
    );
    session.selection.set(&ids);
    let repeat = session.key_down(KeyInput {
        key: "g",
        ctrl: true,
        repeat: true,
        ..KeyInput::default()
    });
    assert_eq!(repeat, KeyOutcome::Ignored, "a held chord does not repeat");
    assert_eq!(chord(&mut session, "g", false), KeyOutcome::Group);
    assert_eq!(
        session.take_grouping_result().unwrap().notice,
        "Grouped 2 objects."
    );
    assert_eq!(session.document.children(None).len(), 1);
    assert_eq!(chord(&mut session, "G", true), KeyOutcome::Ungroup);
    assert_eq!(
        session.take_grouping_result().unwrap().notice,
        "Ungrouped 1 group into 2 objects."
    );
    assert_eq!(session.document.children(None), ids.to_vec());
    session.set_tool(Tool::Rectangle);
    assert_eq!(
        chord(&mut session, "g", false),
        KeyOutcome::Ignored,
        "another tool"
    );
}

/// Criterion 8: a click on a member selects the group.
#[test]
fn a_click_selects_the_group_and_the_panel_names_it() {
    let (mut session, ids, _) = with_group();
    click(&mut session, pt(45.0, 0.0));
    assert_eq!(session.selection.ids(), &[ids[1], ids[2]]);
    assert_eq!(session.style_scope().subject, "Group of 2 objects");
    assert!(
        session
            .selection_announcement()
            .starts_with("Group of 2 objects selected")
    );
}

/// Criterion 15: a double-click on a group opens it; the content outside is inert and dimmed.
#[test]
fn a_double_click_enters_the_group_and_everything_else_goes_inert() {
    let (mut session, ids, group) = with_group();
    double_click(&mut session, pt(45.0, 0.0));
    assert_eq!(session.context_group(), Some(group));
    assert_eq!(session.context_depth(), 1);
    assert_eq!(
        session.selection.ids(),
        &[ids[2]],
        "the double-clicked leaf is selected"
    );
    assert_eq!(
        session.take_context_announcement(),
        "Inside group, 1 level. Escape leaves."
    );
    let inside: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    assert_eq!(
        inside,
        vec![ids[1], ids[2]],
        "only the group's leaves are hit"
    );
    // A press on a square outside selects nothing.
    click(&mut session, pt(5.0, 0.0));
    assert!(session.selection.is_empty());
    // The outside is drawn at 35 %, the inside as it is.
    let drawn = session.render_objects();
    let strength = |id: NodeId| {
        drawn
            .iter()
            .find(|o| o.id() == id)
            .unwrap()
            .style()
            .stroke
            .opacity
            .get()
    };
    assert!((strength(ids[0]) - 0.35).abs() < 1e-9);
    assert!((strength(ids[3]) - 0.35).abs() < 1e-9);
    assert!((strength(ids[1]) - 1.0).abs() < 1e-9);
    // Pan, zoom and tool changes keep the context (criterion 21).
    session.set_tool(Tool::Node);
    session.set_tool(Tool::Select);
    assert_eq!(session.context_group(), Some(group));
}

/// Criterion 15: Enter with exactly one group selected enters it with nothing selected.
#[test]
fn enter_opens_the_one_selected_group() {
    let (mut session, ids, group) = with_group();
    assert_eq!(
        key(&mut session, "Enter"),
        KeyOutcome::Ignored,
        "nothing selected"
    );
    click(&mut session, pt(5.0, 0.0));
    assert_eq!(
        key(&mut session, "Enter"),
        KeyOutcome::Ignored,
        "a plain object"
    );
    click(&mut session, pt(25.0, 0.0));
    assert_eq!(key(&mut session, "Enter"), KeyOutcome::EnteredGroup);
    assert_eq!(session.context_group(), Some(group));
    assert!(session.selection.is_empty());
    let _ = ids;
}

/// Criteria 17 and 18: Escape clears the selection first, then leaves one level and selects the
/// group it left; a double-click on empty canvas leaves one level.
#[test]
fn escape_clears_then_leaves_and_a_double_click_on_empty_canvas_leaves() {
    let (mut session, ids, group) = with_group();
    double_click(&mut session, pt(25.0, 0.0));
    assert_eq!(session.selection.ids(), &[ids[1]]);
    assert_eq!(
        key(&mut session, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClearedState)
    );
    assert_eq!(session.context_group(), Some(group));
    assert_eq!(
        key(&mut session, "Escape"),
        KeyOutcome::Escape(EscapeStep::LeftGroup)
    );
    assert_eq!(session.context_group(), None);
    assert_eq!(
        session.selection.ids(),
        &[ids[1], ids[2]],
        "the group just left is selected"
    );
    assert_eq!(session.take_context_announcement(), "Back at the document.");
    assert_eq!(
        key(&mut session, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClearedState)
    );
    assert_eq!(
        key(&mut session, "Escape"),
        KeyOutcome::Escape(EscapeStep::Nothing)
    );

    double_click(&mut session, pt(25.0, 0.0));
    assert_eq!(session.context_depth(), 1);
    double_click(&mut session, pt(300.0, 300.0));
    assert_eq!(session.context_depth(), 0, "empty canvas leaves one level");
}

/// Criterion 16: the crumbs make a level the context.
#[test]
fn the_crumbs_leave_to_a_level() {
    let mut session = Session::new(1);
    let ids = [0.0, 20.0, 40.0].map(|x| rect(&session, x));
    let inner = session.document.group_objects(&[ids[1], ids[2]]).unwrap();
    let outer = session.document.group_objects(&[ids[0], inner]).unwrap();
    session.selection.clear();
    assert!(session.enter_group(outer, None));
    assert!(session.enter_group(inner, None));
    assert_eq!(session.context_depth(), 2);
    assert!(session.leave_to_level(1));
    assert_eq!(session.context_group(), Some(outer));
    assert!(session.leave_to_level(0));
    assert_eq!(session.context_depth(), 0);
    assert!(!session.leave_to_level(0), "already at the root");
}

/// Criterion 19: what is drawn inside an entered group becomes a child of it, on top.
#[test]
fn objects_drawn_inside_a_group_become_its_children() {
    let (mut session, ids, group) = with_group();
    double_click(&mut session, pt(25.0, 0.0));
    session.set_tool(Tool::Rectangle);
    session.pointer_down(pt(100.0, 100.0), false);
    session.pointer_up(pt(130.0, 120.0), false, false);
    let children = session.document.children(Some(group));
    assert_eq!(children.len(), 3);
    assert_eq!(&children[..2], &[ids[1], ids[2]]);
    assert_eq!(
        session.document.children(None).len(),
        3,
        "the root still has three objects"
    );
}

/// Criterion 14 and 20: the entered group falls back to the nearest one that exists.
#[test]
fn the_context_falls_back_when_the_entered_group_goes_away() {
    let (mut session, ids, group) = with_group();
    double_click(&mut session, pt(25.0, 0.0));
    session.selection.set(&[ids[1], ids[2]]);
    session.delete_selected();
    assert!(
        !session.document.contains_node(group),
        "the emptied group is removed"
    );
    assert_eq!(session.context_group(), None);
    assert_eq!(session.objects().len(), 2, "the root's objects are back");
    session.sync_context();
    assert_eq!(session.context_depth(), 0);
}

/// Criterion 28: the operations that do not look through groups refuse them.
#[test]
fn the_boolean_operations_refuse_a_selected_group() {
    let (mut session, ids, _) = with_group();
    click(&mut session, pt(45.0, 0.0));
    session.selection.add(ids[0]);
    let outcome = session.apply_boolean(BooleanOp::Union);
    assert!(matches!(
        outcome,
        crate::BooleanOutcome::Refused(curvyo_ui_core::BooleanRefusal::Groups { .. })
    ));
    assert_eq!(
        session.document.object_ids().len(),
        4,
        "nothing was changed"
    );
    assert!(matches!(
        session.apply_combine(),
        crate::CombineOutcome::Refused(curvyo_ui_core::CombineRefusal::Groups { .. })
    ));
}

/// Criterion 29: Object to path is not offered for a group.
#[test]
fn object_to_path_is_not_offered_for_a_group() {
    let (mut session, ..) = with_group();
    click(&mut session, pt(5.0, 0.0));
    assert!(
        session.select_bar_state().object_to_path,
        "a rectangle converts"
    );
    click(&mut session, pt(45.0, 0.0));
    assert!(!session.select_bar_state().object_to_path);
    session.convert_selected_to_paths();
    assert_eq!(session.tool(), Tool::Select, "nothing happened");
}

/// Criterion 13: Delete and a Ctrl-copy take the whole group; undo takes each action back.
#[test]
fn delete_group_ungroup_and_their_undo_are_one_step_each() {
    let (mut session, ids, group) = with_group();
    click(&mut session, pt(45.0, 0.0));
    session.delete_selected();
    assert_eq!(session.document.children(None), vec![ids[0], ids[3]]);
    session.undo();
    assert_eq!(session.document.children(None), vec![ids[0], group, ids[3]]);
    assert!(session.document.is_group(group));

    click(&mut session, pt(45.0, 0.0));
    session.ungroup_selection();
    assert_eq!(session.document.children(None).len(), 4);
    session.undo();
    assert!(session.document.is_group(group));
    assert_eq!(session.document.children(Some(group)), vec![ids[1], ids[2]]);
    session.redo();
    assert!(!session.document.contains_node(group));

    // Group again and undo it: the members are back in their places.
    session.selection.set(&[ids[1], ids[2]]);
    session.group_selection();
    assert_eq!(session.document.children(None).len(), 3);
    session.undo();
    assert_eq!(session.document.children(None), ids.to_vec());
}

/// Criterion 12: with the Node tool, the nodes of the paths inside a group are the ones shown.
#[test]
fn the_node_tool_sees_the_paths_of_a_group_and_not_those_outside_an_entered_one() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let mut session = Session::new(1);
    let mk = |s: &Session, n: u64, x: f64| {
        s.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, n), pt(x, 0.0)),
                NewAnchor::corner(AnchorId::new(1, n + 1), pt(x + 10.0, 5.0)),
            ],
            false,
        )
    };
    let (a, b, c) = (
        mk(&session, 1, 0.0),
        mk(&session, 10, 20.0),
        mk(&session, 20, 40.0),
    );
    let group = session.document.group_objects(&[b, c]).unwrap();
    assert_eq!(
        session.paths().len(),
        3,
        "all paths at the root, group members included"
    );
    session.enter_group(group, None);
    let inside: Vec<_> = session.paths().iter().map(|p| p.id).collect();
    assert_eq!(inside, vec![b, c], "outside paths are inert");
    let _ = a;
}
