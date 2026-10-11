//! Independent tester tests for `specs/0023-groups` at the editor session (the public `Session`
//! seam: pointer events, keys and the typed commands the rail buttons call). Written from the
//! specification before the implementation diff was read. Expected values are the specification's
//! own arithmetic and small reference models written here: the transform tests compare a selected
//! group with the same leaves multi-selected (the 0019 behaviour, which criterion 22 says a group
//! must reproduce).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::type_complexity
)]
#![allow(
    clippy::assert_is_empty,
    clippy::manual_midpoint,
    clippy::useless_vec,
    clippy::unnecessary_wraps
)]

use std::f64::consts::SQRT_2;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, Document, Length, LineCap, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds,
    StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{
    BooleanOutcome, BreakApartOutcome, CombineOutcome, EscapeStep, GroupingKind, KeyInput,
    KeyOutcome, Session, Tool,
};
use curvyo_ui_core::{BooleanOp, BooleanRefusal, BreakApartRefusal, CombineRefusal, StyleField};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

static TIME: AtomicU64 = AtomicU64::new(1_000);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect_at(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// `n` 30 x 30 squares at x = 20 + 40 i, y = 20.
fn squares(n: usize) -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let ids = (0..n)
        .map(|i| rect_at(&d, 20.0 + 40.0 * i as f64, 20.0, 30.0, 30.0))
        .collect();
    (d, ids)
}

/// The middle of the top edge of square `i` of [`squares`].
fn top(i: usize) -> Point {
    pt(35.0 + 40.0 * i as f64, 20.0)
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn json(s: &Session) -> Vec<u8> {
    doc_of(s).export_json().unwrap()
}

fn root(s: &Session) -> Vec<NodeId> {
    doc_of(s).children(None)
}

fn labels(s: &Session) -> Vec<String> {
    let l = loro::LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&ids, &mut |c| {
        changes.push((
            c.lamport,
            c.message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, m)| m).collect()
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool, alt: bool) {
    s.modifiers_changed(shift, ctrl, alt);
    s.pointer_hover(at, shift, ctrl);
}

fn click(s: &mut Session, p: Point) {
    hold(s, p, false, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn shift_click(s: &mut Session, p: Point) {
    hold(s, p, true, false, false);
    s.pointer_down(p, true);
    s.pointer_up(p, true, false);
    hold(s, p, false, false, false);
}

fn alt_click(s: &mut Session, p: Point) {
    hold(s, p, false, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    hold(s, p, false, false, false);
}

fn dbl(s: &mut Session, p: Point) -> bool {
    s.double_click(p, false, false)
}

fn drag_mod(s: &mut Session, from: Point, to: Point, shift: bool, ctrl: bool) {
    hold(s, from, shift, ctrl, false);
    s.pointer_down(from, shift);
    let mid = pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y));
    hold(s, mid, shift, ctrl, false);
    hold(s, to, shift, ctrl, false);
    s.pointer_up(to, shift, ctrl);
    hold(s, to, false, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    drag_mod(s, from, to, false, false);
}

fn key_with(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        time_ms: TIME.fetch_add(1_000, Ordering::SeqCst) as f64,
        ..KeyInput::default()
    })
}

fn key(s: &mut Session, k: &str) -> KeyOutcome {
    key_with(s, k, false, false)
}

fn ctrl_g(s: &mut Session) -> KeyOutcome {
    key_with(s, "g", true, false)
}

fn ctrl_shift_g(s: &mut Session) -> KeyOutcome {
    key_with(s, "G", true, true)
}

fn undo(s: &mut Session) {
    match key_with(s, "z", true, false) {
        KeyOutcome::UndoBusy => {
            let _ = s.undo();
        }
        KeyOutcome::Undo => {}
        other => panic!("Ctrl+Z gave {other:?}"),
    }
}

fn redo(s: &mut Session) {
    match key_with(s, "Z", true, true) {
        KeyOutcome::RedoBusy => {
            let _ = s.redo();
        }
        KeyOutcome::Redo => {}
        other => panic!("Ctrl+Shift+Z gave {other:?}"),
    }
}

fn snaps(d: &Document) -> Vec<(NodeId, String)> {
    d.object_ids()
        .into_iter()
        .map(|id| (id, format!("{:?}", d.object(id).unwrap())))
        .collect()
}

/// The leaves the Select tool has selected, found by a nudge that is taken back again.
fn selected(s: &mut Session) -> Vec<NodeId> {
    let before_json = json(s);
    let before = snaps(&doc_of(s));
    let out = key(s, "ArrowRight");
    if !matches!(out, KeyOutcome::Nudged { .. }) {
        return Vec::new();
    }
    let after = snaps(&doc_of(s));
    undo(s);
    assert_eq!(json(s), before_json, "the probe nudge was taken back");
    before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b.1 != a.1)
        .map(|(b, _)| b.0)
        .collect()
}

fn set_of(ids: &[NodeId]) -> Vec<String> {
    let mut v: Vec<String> = ids.iter().map(|i| format!("{i:?}")).collect();
    v.sort();
    v
}

fn same_set(a: &[NodeId], b: &[NodeId]) -> bool {
    set_of(a) == set_of(b)
}

fn art_alpha(s: &Session, p: Point) -> u8 {
    let l = s.draw_list();
    let end = l.overlay_start();
    l.triangles[..end]
        .iter()
        .filter(|v| (v.position.x - p.x).abs() < 1.0 && (v.position.y - p.y).abs() < 1.0)
        .map(|v| v.color.a)
        .max()
        .unwrap_or(0)
}

fn overlay_triangles(s: &Session) -> usize {
    let l = s.draw_list();
    (l.triangles.len() - l.overlay_start()) / 3
}

/// A group of the given objects, made through the document API before the session opens.
fn grouped(d: &Document, ids: &[NodeId]) -> NodeId {
    d.group_objects(ids).unwrap()
}

fn select_everything(s: &mut Session) {
    key(s, "Escape");
    assert_eq!(key_with(s, "a", true, false), KeyOutcome::SelectedAll);
}

// ---------------------------------------------------------------------
// Criteria 1, 2, 3, 4, 6: Group and Ungroup
// ---------------------------------------------------------------------

#[test]
fn ac1_ctrl_g_binds_the_selection_and_selects_only_the_group() {
    let (d, ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(2));
    assert!(same_set(&selected(&mut s), &[ids[0], ids[2]]));
    let steps = s.undo_len();
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    let r = s.take_grouping_result().expect("a result");
    assert_eq!(r.kind, GroupingKind::Grouped);
    assert_eq!(r.notice, "Grouped 2 objects.");
    assert!(s.take_grouping_result().is_none(), "taken once");
    let d2 = doc_of(&s);
    let rt = d2.children(None);
    assert_eq!(rt.len(), 2);
    assert_eq!(rt[0], ids[1], "B stays below");
    let g = rt[1];
    assert!(d2.is_group(g));
    assert_eq!(d2.children(Some(g)), vec![ids[0], ids[2]], "ids and order");
    assert_eq!(s.undo_len(), steps + 1, "one commit");
    assert_eq!(labels(&s).last().unwrap(), "group");
    assert!(same_set(&selected(&mut s), &[ids[0], ids[2]]));
    assert!(
        s.selection_announcement()
            .starts_with("Group of 2 objects selected"),
        "{}",
        s.selection_announcement()
    );
}

#[test]
fn ac1_the_button_and_the_key_do_the_same_and_say_the_same() {
    let (d, ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    shift_click(&mut s, top(2));
    let r = s.group_selection();
    assert_eq!(r.kind, GroupingKind::Grouped);
    assert_eq!(r.notice, "Grouped 3 objects.");
    assert_eq!(r.code(), "grouped");
    let d2 = doc_of(&s);
    assert_eq!(d2.children(None).len(), 1);
    assert_eq!(d2.children(Some(d2.children(None)[0])), ids);
}

#[test]
fn ac1_two_groups_and_a_leaf_group_in_their_existing_relative_order() {
    let (d, ids) = squares(5);
    let g1 = grouped(&d, &[ids[0], ids[1]]); // root: g1 C D E
    let mut s = open(&d);
    click(&mut s, top(1)); // member of g1 -> g1
    shift_click(&mut s, top(4)); // E
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    let d2 = doc_of(&s);
    let rt = d2.children(None);
    assert_eq!(rt.len(), 3);
    assert_eq!(&rt[..2], &[ids[2], ids[3]]);
    assert_eq!(d2.children(Some(rt[2])), vec![g1, ids[4]]);
}

#[test]
fn ac2_availability_follows_the_selection_and_the_tool() {
    let (d, ids) = squares(3);
    grouped(&d, &[ids[1], ids[2]]);
    let mut s = open(&d);
    let a = s.grouping_availability();
    assert!(!a.group && !a.ungroup, "nothing selected");
    click(&mut s, top(0));
    let a = s.grouping_availability();
    assert!(!a.group && !a.ungroup, "one leaf");
    click(&mut s, top(1)); // the group alone
    let a = s.grouping_availability();
    assert!(!a.group, "one group is one object: Group is not available");
    assert!(a.ungroup);
    shift_click(&mut s, top(0));
    let a = s.grouping_availability();
    assert!(a.group && a.ungroup, "a group and a leaf: both");
    s.set_tool(Tool::Node);
    let a = s.grouping_availability();
    assert!(!a.group && !a.ungroup, "another tool: dimmed");
}

#[test]
fn ac2_group_with_fewer_than_two_does_nothing_and_leaves_no_step() {
    let (d, _) = squares(2);
    let mut s = open(&d);
    let before = (json(&s), s.undo_len());
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored, "nothing selected");
    click(&mut s, top(0));
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored, "one object");
    let r = s.group_selection();
    assert_eq!(r.kind, GroupingKind::Ignored);
    assert!(r.notice.is_empty());
    assert_eq!((json(&s), s.undo_len()), before);
}

#[test]
fn ac2_a_compound_path_and_a_primitive_group_like_any_leaf() {
    let d = Document::new(1);
    let square = |n: u64, x: f64| {
        let a =
            |k: u64, px: f64, py: f64| NewAnchor::corner(AnchorId::new(9, n * 10 + k), pt(px, py));
        d.create_path(
            &[
                a(0, x, 20.0),
                a(1, x + 30.0, 20.0),
                a(2, x + 30.0, 50.0),
                a(3, x, 50.0),
            ],
            true,
        )
    };
    let _p1 = square(1, 20.0);
    let _p2 = square(2, 70.0);
    let r = rect_at(&d, 150.0, 20.0, 30.0, 30.0);
    let mut s = open(&d);
    select_everything(&mut s);
    // Select only the two paths with a marquee, combine them.
    key(&mut s, "Escape");
    drag(&mut s, pt(10.0, 10.0), pt(120.0, 60.0));
    let out = s.apply_combine();
    assert!(matches!(out, CombineOutcome::Applied { .. }), "{out:?}");
    // The compound is selected; add the rectangle and group both.
    shift_click(&mut s, pt(165.0, 20.0));
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    let d2 = doc_of(&s);
    assert_eq!(d2.children(None).len(), 1);
    let g = d2.children(None)[0];
    assert!(d2.children(Some(g)).contains(&r));
    assert_eq!(d2.children(Some(g)).len(), 2);
}

#[test]
fn ac3_ungroup_releases_each_selected_group_and_keeps_everything_selected() {
    let (d, ids) = squares(5);
    let g1 = grouped(&d, &[ids[0], ids[1]]);
    let g2 = grouped(&d, &[ids[2], ids[3]]);
    let mut s = open(&d);
    drag(&mut s, pt(-50.0, -50.0), pt(300.0, 100.0));
    assert_eq!(selected(&mut s).len(), 5);
    let steps = s.undo_len();
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ungroup);
    let r = s.take_grouping_result().unwrap();
    assert_eq!(r.kind, GroupingKind::Ungrouped);
    assert_eq!(r.notice, "Ungrouped 2 groups into 4 objects.");
    let d2 = doc_of(&s);
    assert_eq!(d2.children(None), ids, "released at their place, in order");
    assert!(!d2.contains_node(g1) && !d2.contains_node(g2));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(labels(&s).last().unwrap(), "ungroup");
    assert!(
        same_set(&selected(&mut s), &ids),
        "the children and the leaf"
    );
}

#[test]
fn ac3_a_nested_group_stays_a_group_and_ungroup_goes_one_level() {
    let (d, ids) = squares(4);
    let inner = grouped(&d, &[ids[0], ids[1]]);
    let outer = grouped(&d, &[inner, ids[2]]);
    let mut s = open(&d);
    click(&mut s, top(2));
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ungroup);
    let d2 = doc_of(&s);
    assert!(!d2.contains_node(outer));
    assert!(d2.is_group(inner));
    assert_eq!(d2.children(None), vec![inner, ids[2], ids[3]]);
    assert_eq!(
        s.take_grouping_result().unwrap().notice,
        "Ungrouped 1 group into 2 objects."
    );
}

#[test]
fn ac3_ungroup_without_a_group_does_nothing() {
    let (d, _) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    let before = (json(&s), s.undo_len());
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ignored);
    assert_eq!(s.ungroup_selection().kind, GroupingKind::Ignored);
    assert_eq!((json(&s), s.undo_len()), before);
}

#[test]
fn ac4_group_then_ungroup_gives_the_old_document_but_for_the_group_ids() {
    let (d, ids) = squares(4);
    let mut s = open(&d);
    let before = snaps(&doc_of(&s));
    click(&mut s, top(1));
    shift_click(&mut s, top(2));
    ctrl_g(&mut s);
    ctrl_shift_g(&mut s);
    let d2 = doc_of(&s);
    assert_eq!(d2.children(None), ids);
    assert_eq!(snaps(&d2), before);
}

#[test]
fn ac4_with_an_object_between_it_ends_below_the_group_and_stays_there() {
    let (d, ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(2));
    ctrl_g(&mut s);
    ctrl_shift_g(&mut s);
    assert_eq!(root(&s), vec![ids[1], ids[0], ids[2]]);
}

#[test]
fn ac5_a_group_beyond_32_levels_is_refused_with_the_exact_notice_and_nothing_changes() {
    let d = Document::new(1);
    let mut leaves = vec![
        rect_at(&d, 20.0, 20.0, 30.0, 30.0),
        rect_at(&d, 60.0, 20.0, 30.0, 30.0),
    ];
    let mut g = grouped(&d, &leaves);
    for i in 1..32 {
        let x = rect_at(&d, 20.0 + 40.0 * f64::from(i + 1), 80.0, 30.0, 30.0);
        leaves.push(x);
        g = grouped(&d, &[g, x]);
    }
    let _ = g;
    let extra = rect_at(&d, 20.0, 200.0, 30.0, 30.0);
    let _ = extra;
    let mut s = open(&d);
    click(&mut s, pt(35.0, 20.0)); // the whole 32-deep group
    shift_click(&mut s, pt(35.0, 200.0));
    let before = (json(&s), s.undo_len());
    let out = ctrl_g(&mut s);
    assert_ne!(out, KeyOutcome::Ignored, "the key acted: it was refused");
    let r = s.take_grouping_result().unwrap();
    assert_eq!(r.kind, GroupingKind::Refused);
    assert_eq!(
        r.notice,
        "Groups can be nested 32 levels deep. Nothing was changed."
    );
    assert_eq!(r.code(), "refused");
    assert_eq!((json(&s), s.undo_len()), before);
}

#[test]
fn ac6_notices_name_what_happened_and_say_nothing_about_undo() {
    let (d, ids) = squares(4);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    shift_click(&mut s, top(2));
    ctrl_g(&mut s);
    let n = s.take_grouping_result().unwrap().notice;
    assert_eq!(n, "Grouped 3 objects.");
    assert!(!n.contains("undo"));
    ctrl_shift_g(&mut s);
    let n = s.take_grouping_result().unwrap().notice;
    assert_eq!(n, "Ungrouped 1 group into 3 objects.");
    assert!(!n.to_lowercase().contains("undo"));
    let _ = ids;
}

// ---------------------------------------------------------------------
// Criterion 7: shortcuts and their gates
// ---------------------------------------------------------------------

#[test]
fn ac7_the_shortcuts_are_gated_like_every_shortcut() {
    let (d, _) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    let base = (json(&s), s.undo_len());
    // Key repeat.
    let r = s.key_down(KeyInput {
        key: "g",
        ctrl: true,
        repeat: true,
        ..KeyInput::default()
    });
    assert_eq!(r, KeyOutcome::Ignored, "auto-repeat");
    // Focus in a text field.
    let r = s.key_down(KeyInput {
        key: "g",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(r, KeyOutcome::Ignored, "dom blocked");
    // A drag in flight.
    hold(&mut s, top(0), false, false, false);
    s.pointer_down(top(0), false);
    hold(&mut s, pt(40.0, 30.0), false, false, false);
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored, "during a drag");
    s.pointer_cancelled();
    assert_eq!(json(&s), base.0);
    // A chip open.
    assert_eq!(key(&mut s, "m"), KeyOutcome::EntryOpened);
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored, "with a chip open");
    key(&mut s, "Escape");
    // Another tool, the Pen with an unfinished path.
    s.set_tool(Tool::Pen);
    click(&mut s, pt(200.0, 100.0));
    click(&mut s, pt(220.0, 120.0));
    assert_eq!(
        ctrl_g(&mut s),
        KeyOutcome::Ignored,
        "Pen with an unfinished path"
    );
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ignored);
    s.finish_pen();
    s.set_tool(Tool::Select);
    assert_eq!(s.undo_len(), base.1 + 1, "only the pen path was drawn");
}

#[test]
fn ac7_command_key_on_macos_and_both_cases_of_shift_g() {
    let (d, _) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    let r = s.key_down(KeyInput {
        key: "g",
        ctrl: true,
        mac: true,
        ..KeyInput::default()
    });
    assert_eq!(r, KeyOutcome::Group, "Cmd+G on macOS arrives as ctrl");
    for k in ["G", "g"] {
        let (d, _) = squares(3);
        let g = grouped(&d, &[d.children(None)[0], d.children(None)[1]]);
        let _ = g;
        let mut s2 = open(&d);
        click(&mut s2, top(1));
        let r = key_with(&mut s2, k, true, true);
        assert_eq!(r, KeyOutcome::Ungroup, "Ctrl+Shift+{k}");
    }
}

// ---------------------------------------------------------------------
// Criteria 8 to 10: selecting groups
// ---------------------------------------------------------------------

#[test]
fn ac8_a_click_on_a_member_selects_the_group_a_direct_child_selects_itself() {
    let (d, ids) = squares(4);
    let _g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, top(1)); // member B of the group (outline rule)
    assert!(same_set(&selected(&mut s), &[ids[0], ids[1]]));
    click(&mut s, top(3));
    assert!(same_set(&selected(&mut s), &[ids[3]]), "a direct child");
    click(&mut s, top(2));
    assert!(same_set(&selected(&mut s), &[ids[2]]));
}

#[test]
fn ac8_the_filled_interior_of_a_member_hits_the_group_too() {
    let (d, ids) = squares(3);
    for id in &ids {
        d.edit_style(&[*id], &StyleEdit::FillEnabled(true)).unwrap();
    }
    let _g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, pt(65.0, 35.0)); // inside B, far from every outline
    assert!(same_set(&selected(&mut s), &[ids[0], ids[1]]));
    click(&mut s, pt(105.0, 35.0)); // inside C
    assert!(same_set(&selected(&mut s), &[ids[2]]));
}

#[test]
fn ac8_the_top_level_object_is_the_outermost_group_of_the_context() {
    let (d, ids) = squares(4);
    let inner = grouped(&d, &[ids[0], ids[1]]);
    let _outer = grouped(&d, &[inner, ids[2]]);
    let mut s = open(&d);
    click(&mut s, top(0)); // deepest leaf
    assert!(same_set(&selected(&mut s), &[ids[0], ids[1], ids[2]]));
}

#[test]
fn ac9_shift_click_toggles_a_top_level_object_in_and_out() {
    let (d, ids) = squares(4);
    let _g1 = grouped(&d, &[ids[0], ids[1]]);
    let _g2 = grouped(&d, &[ids[2], ids[3]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(3));
    assert!(same_set(&selected(&mut s), &ids), "both groups");
    shift_click(&mut s, top(1)); // a member of g1 toggles g1 out
    assert!(same_set(&selected(&mut s), &[ids[2], ids[3]]));
    shift_click(&mut s, top(2));
    assert!(selected(&mut s).is_empty());
}

#[test]
fn ac9_marquee_chooses_among_top_level_objects_by_the_box_of_the_group() {
    // g = A(20..50) B(60..90) at y 20..50, C at 100..130.
    let (d, ids) = squares(3);
    let _g = grouped(&d, &[ids[0], ids[1]]);
    let both = [ids[0], ids[1]];
    let mut s = open(&d);
    // Rightward (enclose): around A only does not enclose the group box.
    drag(&mut s, pt(10.0, 10.0), pt(55.0, 60.0));
    assert!(
        selected(&mut s).is_empty(),
        "A alone is not a top-level object"
    );
    // Around both: the group, not C.
    drag(&mut s, pt(10.0, 10.0), pt(95.0, 60.0));
    assert!(same_set(&selected(&mut s), &both));
    // Around the group and C.
    drag(&mut s, pt(10.0, 10.0), pt(140.0, 60.0));
    assert!(same_set(&selected(&mut s), &ids));
    // Leftward (touch) touching only B.
    key(&mut s, "Escape");
    drag(&mut s, pt(75.0, 70.0), pt(65.0, 45.0));
    assert!(
        same_set(&selected(&mut s), &both),
        "touching one member selects the group"
    );
    // Leftward touching only the gap inside the group's box, not a leaf.
    key(&mut s, "Escape");
    drag(&mut s, pt(56.0, 70.0), pt(54.0, 40.0));
    assert!(
        same_set(&selected(&mut s), &both),
        "the touch rule applies to the group box (spec 9)"
    );
}

#[test]
fn ac10_the_alt_click_cycle_runs_over_top_level_objects_not_leaves() {
    let d = Document::new(1);
    // A and B coincide at the click point and are one group; C also lies there.
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let b = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let c = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let _g = grouped(&d, &[a, b]);
    let mut s = open(&d);
    let p = pt(35.0, 20.0);
    let mut seen = Vec::new();
    click(&mut s, p);
    seen.push(set_of(&selected(&mut s)));
    for _ in 0..4 {
        alt_click(&mut s, p);
        seen.push(set_of(&selected(&mut s)));
    }
    let unit_g = set_of(&[a, b]);
    let unit_c = set_of(&[c]);
    for x in &seen {
        assert!(
            *x == unit_g || *x == unit_c,
            "only units are candidates: {x:?}"
        );
    }
    assert_ne!(seen[0], seen[1], "alt-click steps on");
    assert_eq!(seen[0], seen[2], "two candidates: wraps after two");
    assert_eq!(seen[1], seen[3]);
    assert_eq!(seen[2], seen[4]);
}

// ---------------------------------------------------------------------
// Criterion 11: subject line, 12: Node tool
// ---------------------------------------------------------------------

#[test]
fn ac11_the_subject_line_counts_leaves_nested_groups_through() {
    let (d, ids) = squares(7);
    let inner = grouped(&d, &[ids[0], ids[1], ids[2]]);
    let _five = grouped(&d, &[inner, ids[3], ids[4]]);
    let _two = grouped(&d, &[ids[5], ids[6]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    assert_eq!(s.style_panel_view().subject, "Group of 5 objects");
    shift_click(&mut s, top(5));
    assert_eq!(s.style_panel_view().subject, "2 groups");
    click(&mut s, pt(300.0, 300.0)); // clear
    click(&mut s, top(5));
    assert_eq!(s.style_panel_view().subject, "Group of 2 objects");
}

#[test]
fn ac11_a_group_and_a_leaf_make_a_mixed_subject() {
    let (d, ids) = squares(4);
    let _g = grouped(&d, &[ids[0], ids[1], ids[2]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(3));
    let subject = s.style_panel_view().subject;
    eprintln!(
        "AMBIGUITY (criterion 11): group of 3 + 1 leaf reads {subject:?}; the spec example is \"4 objects\""
    );
    let n: usize = subject
        .strip_suffix(" objects")
        .unwrap_or_else(|| panic!("a mix reads \"<n> objects\": {subject}"))
        .parse()
        .unwrap();
    assert!(n == 2 || n == 4, "{subject}");
}

#[test]
fn ac11_two_groups_and_two_leaves_read_as_a_mix() {
    let (d, ids) = squares(7);
    let _g1 = grouped(&d, &ids[0..2]);
    let _g2 = grouped(&d, &ids[2..4]);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(3));
    shift_click(&mut s, top(4));
    shift_click(&mut s, top(5));
    let subject = s.style_panel_view().subject;
    eprintln!("two groups + two leaves: {subject:?}");
    assert!(subject.ends_with(" objects"), "{subject}");
}

fn open_path(d: &Document, n: u64, x: f64, y: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(8, n * 10), pt(x, y)),
            NewAnchor::corner(AnchorId::new(8, n * 10 + 1), pt(x + 20.0, y + 10.0)),
            NewAnchor::corner(AnchorId::new(8, n * 10 + 2), pt(x + 40.0, y)),
        ],
        false,
    )
}

fn anchor_points(s: &Session, id: NodeId) -> Vec<Point> {
    doc_of(s)
        .path(id)
        .unwrap()
        .anchors
        .iter()
        .map(|a| a.point)
        .collect()
}

#[test]
fn ac12_the_node_tool_edits_the_paths_of_a_group_nested_ones_included() {
    let d = Document::new(1);
    let p = open_path(&d, 1, 20.0, 20.0);
    let q = open_path(&d, 2, 20.0, 80.0);
    let r = open_path(&d, 3, 20.0, 140.0);
    let sq = rect_at(&d, 120.0, 20.0, 20.0, 20.0);
    let inner = grouped(&d, &[q, r]);
    let _outer = grouped(&d, &[p, inner, sq]);
    let mut s = open(&d);
    click(&mut s, pt(40.0, 30.0)); // the p's middle anchor: selects the whole outer group
    assert_eq!(selected(&mut s).len(), 4);
    s.set_tool(Tool::Node);
    let (p0, q0, r0) = (
        anchor_points(&s, p),
        anchor_points(&s, q),
        anchor_points(&s, r),
    );
    let rect_before = format!("{:?}", doc_of(&s).object(sq));
    drag(&mut s, p0[1], pt(p0[1].x + 3.0, p0[1].y + 4.0));
    assert!((anchor_points(&s, p)[1].x - (p0[1].x + 3.0)).abs() < 1e-6);
    drag(&mut s, q0[1], pt(q0[1].x - 2.0, q0[1].y + 5.0));
    assert!(
        (anchor_points(&s, q)[1].y - (q0[1].y + 5.0)).abs() < 1e-6,
        "nested group path"
    );
    drag(&mut s, r0[1], pt(r0[1].x + 1.0, r0[1].y + 1.0));
    assert!((anchor_points(&s, r)[1].x - (r0[1].x + 1.0)).abs() < 1e-6);
    // The rectangle contributes no nodes: dragging at its corner does not move it.
    drag(&mut s, pt(120.0, 20.0), pt(125.0, 25.0));
    assert_eq!(
        format!("{:?}", doc_of(&s).object(sq)),
        rect_before,
        "primitive: no nodes"
    );
}

// ---------------------------------------------------------------------
// Criteria 13, 14: delete and duplicate, empty groups
// ---------------------------------------------------------------------

#[test]
fn ac13_delete_removes_the_whole_group_with_all_descendants_in_one_step() {
    let (d, ids) = squares(4);
    let inner = grouped(&d, &[ids[0], ids[1]]);
    let outer = grouped(&d, &[inner, ids[2]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    let before = json(&s);
    let steps = s.undo_len();
    s.delete_selected();
    let d2 = doc_of(&s);
    assert_eq!(d2.children(None), vec![ids[3]]);
    for id in [inner, outer, ids[0], ids[1], ids[2]] {
        assert!(!d2.contains_node(id), "gone: {id:?}");
    }
    assert_eq!(s.undo_len(), steps + 1);
    undo(&mut s);
    assert_eq!(json(&s), before, "undo is byte-equal");
}

#[test]
fn ac13_ctrl_drag_copies_the_group_with_new_ids_for_everything() {
    let d = Document::new(1);
    let p = open_path(&d, 1, 20.0, 20.0);
    let a = rect_at(&d, 20.0, 60.0, 30.0, 30.0);
    let q = open_path(&d, 2, 20.0, 120.0);
    let inner = grouped(&d, &[a, q]);
    let outer = grouped(&d, &[p, inner]);
    let mut s = open(&d);
    click(&mut s, pt(40.0, 30.0));
    let before = doc_of(&s);
    let steps = s.undo_len();
    drag_mod(&mut s, pt(40.0, 30.0), pt(140.0, 30.0), false, true);
    assert_eq!(s.undo_len(), steps + 1, "one commit");
    let d2 = doc_of(&s);
    let rt = d2.children(None);
    assert_eq!(rt.len(), 2, "original and copy");
    assert_eq!(rt[0], outer, "the original stays");
    let copy = rt[1];
    assert!(d2.is_group(copy));
    let copy_kids = d2.children(Some(copy));
    assert_eq!(copy_kids.len(), 2);
    let copy_inner = copy_kids[1];
    assert!(d2.is_group(copy_inner));
    assert_eq!(d2.children(Some(copy_inner)).len(), 2);
    // Every id of the copy is new.
    let mut originals = vec![outer, inner, p, a, q];
    originals.sort_by_key(|i| format!("{i:?}"));
    for id in [copy, copy_inner].into_iter().chain(d2.leaves_of(copy)) {
        assert!(!originals.contains(&id), "a new node id: {id:?}");
    }
    // Every anchor id is new and unique across the document.
    let mut anchor_ids = Vec::new();
    for id in d2.object_ids() {
        if let Some(path) = d2.path(id) {
            for a in path.anchors {
                anchor_ids.push(format!("{:?}", a.id));
            }
        }
    }
    let total = anchor_ids.len();
    anchor_ids.sort();
    anchor_ids.dedup();
    assert_eq!(anchor_ids.len(), total, "anchor ids are unique");
    assert_eq!(total, 12, "3 anchors x 2 paths x original and copy");
    // The originals did not change.
    for id in before.object_ids() {
        assert_eq!(
            format!("{:?}", d2.object(id)),
            format!("{:?}", before.object(id))
        );
    }
    // The copy sits 100 mm to the right.
    let copy_leaves = d2.leaves_of(copy);
    let moved = copy_leaves.iter().any(|id| match d2.object(*id).unwrap() {
        ObjectSnapshot::Primitive(pr) => format!("{:?}", pr.shape).contains("120"),
        ObjectSnapshot::Path(_) => false,
    });
    assert!(moved, "the copy was moved by the drag");
    assert_eq!(selected(&mut s).len(), 3, "the copy is selected");
}

#[test]
fn ac14_a_group_of_one_is_allowed_and_the_last_delete_inside_removes_it_and_leaves_the_context() {
    let (d, ids) = squares(3);
    let g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::EnteredGroup);
    assert_eq!(s.context_depth(), 1);
    click(&mut s, top(0));
    s.delete_selected();
    let d2 = doc_of(&s);
    assert!(d2.contains_node(g), "a group of one child is allowed");
    assert_eq!(d2.children(Some(g)), vec![ids[1]]);
    assert_eq!(s.context_depth(), 1);
    click(&mut s, top(1));
    s.delete_selected();
    let d3 = doc_of(&s);
    assert!(
        !d3.contains_node(g),
        "the empty group is removed in the same commit"
    );
    assert_eq!(d3.children(None), vec![ids[2]]);
    assert_eq!(s.context_depth(), 0, "the context moves to the parent");
    undo(&mut s);
    assert!(doc_of(&s).contains_node(g), "one undo step brings it back");
    assert_eq!(doc_of(&s).children(Some(g)), vec![ids[1]]);
}

// ---------------------------------------------------------------------
// Criteria 15 to 21: entering and leaving
// ---------------------------------------------------------------------

fn nested_scene() -> (Document, Vec<NodeId>, NodeId, NodeId) {
    // A (outside), G = (B, C, inner(D, E)), F (outside)
    let (d, ids) = squares(6);
    let inner = grouped(&d, &[ids[3], ids[4]]);
    let g = grouped(&d, &[ids[1], ids[2], inner]);
    (d, ids, g, inner)
}

#[test]
fn ac15_double_click_enters_and_the_double_clicked_leaf_is_selected() {
    let (d, ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(1));
    click(&mut s, top(1));
    dbl(&mut s, top(1));
    assert_eq!(s.context_depth(), 1);
    assert!(
        same_set(&selected(&mut s), &[ids[1]]),
        "the leaf that was hit"
    );
    // Double-click on the nested group's member enters nothing deeper by itself and selects the
    // nested group as the top-level object of the context.
    click(&mut s, top(3));
    assert!(same_set(&selected(&mut s), &[ids[3], ids[4]]));
}

#[test]
fn ac15_enter_enters_with_an_empty_selection() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::EnteredGroup);
    assert_eq!(s.context_depth(), 1);
    assert!(selected(&mut s).is_empty());
    assert_eq!(
        s.take_context_announcement(),
        "Inside group, 1 level. Escape leaves."
    );
}

#[test]
fn ac15_enter_needs_exactly_one_group() {
    let (d, ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(0)); // a leaf
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::Ignored);
    shift_click(&mut s, top(1)); // a leaf and a group
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::Ignored);
    key(&mut s, "Escape");
    assert_eq!(
        key(&mut s, "Enter"),
        KeyOutcome::Ignored,
        "nothing selected"
    );
    assert_eq!(s.context_depth(), 0);
    let _ = ids;
}

#[test]
fn ac15_outside_is_dimmed_to_35_percent_inside_is_normal_and_nothing_outside_can_be_hit() {
    let (d, ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    let full = art_alpha(&s, pt(20.0, 20.0));
    assert!(full > 200, "stroke is opaque at the root: {full}");
    click(&mut s, top(2));
    key(&mut s, "Enter");
    let outside = art_alpha(&s, pt(20.0, 20.0)); // square A's corner
    let outside_f = art_alpha(&s, pt(220.0, 20.0)); // square F (index 5)
    let inside = art_alpha(&s, pt(60.0, 20.0)); // square B's corner
    let nested = art_alpha(&s, pt(140.0, 20.0)); // square D (inner group, in the context)
    assert_eq!(inside, full, "the entered group is drawn normally");
    assert_eq!(nested, full);
    let want = f64::from(full) * 0.35;
    for o in [outside, outside_f] {
        assert!(
            (f64::from(o) - want).abs() <= 3.0,
            "dimmed to 35 %: {o} vs {want}"
        );
    }
    // Inert: click, marquee, move, Alt-click do not touch the outside.
    let before = json(&s);
    click(&mut s, top(0));
    assert!(
        selected(&mut s).is_empty(),
        "a click on the outside selects nothing"
    );
    drag(&mut s, pt(-50.0, 10.0), pt(60.0, 60.0));
    let sel = selected(&mut s);
    assert!(!sel.contains(&ids[0]), "the marquee ignores the outside");
    drag(&mut s, top(5), pt(top(5).x + 10.0, 80.0));
    assert_eq!(json(&s), before, "nothing outside was moved");
    click(&mut s, pt(100.0, 300.0)); // clears the selection, stays inside
    alt_click(&mut s, top(0));
    assert!(
        selected(&mut s).is_empty(),
        "no Alt-click candidate outside"
    );
    // Ctrl+A in a context selects the top-level objects of the context only.
    assert_eq!(key_with(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    let all = selected(&mut s);
    assert!(same_set(&all, &[ids[1], ids[2], ids[3], ids[4]]), "{all:?}");
}

#[test]
fn ac15_double_click_on_a_direct_child_path_opens_the_node_tool_as_before() {
    let d = Document::new(1);
    let p = open_path(&d, 1, 20.0, 20.0);
    let a = rect_at(&d, 100.0, 100.0, 20.0, 20.0);
    let q = open_path(&d, 2, 20.0, 80.0);
    let _g = grouped(&d, &[a, q]);
    let mut s = open(&d);
    click(&mut s, pt(40.0, 30.0));
    dbl(&mut s, pt(40.0, 30.0));
    assert_eq!(s.tool(), Tool::Node, "a direct child path");
    assert_eq!(s.context_depth(), 0);
    let _ = p;
    s.set_tool(Tool::Select);
    key(&mut s, "Escape");
    // On a primitive that is a direct child nothing changes (no group entered).
    let (d, ids) = squares(2);
    let mut s2 = open(&d);
    click(&mut s2, top(0));
    dbl(&mut s2, top(0));
    assert_eq!(s2.context_depth(), 0);
    assert_eq!(s2.tool(), Tool::Select);
    let _ = ids;
}

#[test]
fn ac16_each_level_of_the_indicator_makes_that_level_the_context() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(3)); // the nested group
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::EnteredGroup);
    assert_eq!(s.context_depth(), 2);
    let _ = s.take_context_announcement();
    assert!(s.leave_to_level(1), "a click on the first crumb");
    assert_eq!(s.context_depth(), 1);
    assert!(s.leave_to_level(0), "the root entry leaves everything");
    assert_eq!(s.context_depth(), 0);
    assert_eq!(s.take_context_announcement(), "Back at the document.");
    // From two levels straight to the root.
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(3));
    key(&mut s, "Enter");
    assert!(s.leave_to_level(0));
    assert_eq!(s.context_depth(), 0);
    // A level that does not exist changes nothing.
    assert!(!s.leave_to_level(5));
    assert_eq!(s.context_depth(), 0);
}

#[test]
fn ac17_escape_clears_first_then_leaves_one_level_selecting_the_group_just_left() {
    let (d, ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(3)); // inner group
    key(&mut s, "Enter");
    click(&mut s, top(3)); // D
    assert_eq!(s.context_depth(), 2);
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClearedState)
    );
    assert_eq!(s.context_depth(), 2, "clearing the selection came first");
    assert!(selected(&mut s).is_empty());
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::LeftGroup)
    );
    assert_eq!(s.context_depth(), 1);
    assert!(
        same_set(&selected(&mut s), &[ids[3], ids[4]]),
        "the group just left is selected"
    );
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClearedState)
    );
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::LeftGroup)
    );
    assert_eq!(s.context_depth(), 0);
    assert!(same_set(
        &selected(&mut s),
        &[ids[1], ids[2], ids[3], ids[4]]
    ));
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClearedState)
    );
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::Nothing)
    );
    assert_eq!(s.context_depth(), 0);
}

#[test]
fn ac18_double_click_on_empty_canvas_leaves_one_level_a_single_click_only_clears() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(3));
    key(&mut s, "Enter");
    click(&mut s, top(3));
    assert_eq!(s.context_depth(), 2);
    let empty = pt(100.0, 300.0);
    click(&mut s, empty);
    assert!(selected(&mut s).is_empty(), "a single click clears");
    assert_eq!(s.context_depth(), 2, "and stays");
    click(&mut s, empty);
    dbl(&mut s, empty);
    assert_eq!(s.context_depth(), 1);
    click(&mut s, empty);
    dbl(&mut s, empty);
    assert_eq!(s.context_depth(), 0);
    click(&mut s, empty);
    dbl(&mut s, empty);
    assert_eq!(s.context_depth(), 0, "nothing above the root");
}

#[test]
fn ac19_new_objects_become_children_of_the_entered_group_on_top() {
    let (d, ids, g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    let before: Vec<NodeId> = doc_of(&s).object_ids();
    s.set_tool(Tool::Rectangle);
    drag(&mut s, pt(150.0, 100.0), pt(190.0, 140.0));
    s.set_tool(Tool::Ellipse);
    drag(&mut s, pt(200.0, 100.0), pt(230.0, 130.0));
    s.set_tool(Tool::Select);
    let d2 = doc_of(&s);
    let new: Vec<NodeId> = d2
        .object_ids()
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert_eq!(new.len(), 2);
    let kids = d2.children(Some(g));
    assert_eq!(
        kids[kids.len() - 2..],
        new[..],
        "on top of its other children, in order"
    );
    for n in &new {
        assert_eq!(d2.parent_group(*n), Some(g));
    }
    assert_eq!(d2.children(None).len(), 3, "A, G, F at the root");
    assert_eq!(s.context_depth(), 1, "a tool change keeps the context");
    let _ = ids;
    // The Pen draws inside as well.
    s.set_tool(Tool::Pen);
    click(&mut s, pt(60.0, 200.0));
    click(&mut s, pt(80.0, 220.0));
    s.finish_pen();
    let d3 = doc_of(&s);
    let newest = *d3
        .object_ids()
        .iter()
        .find(|i| !d2.object_ids().contains(i))
        .unwrap();
    assert_eq!(d3.parent_group(newest), Some(g));
}

#[test]
fn ac19_a_boolean_result_takes_the_place_of_its_base_operand_inside_the_group() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let b = rect_at(&d, 35.0, 20.0, 30.0, 30.0);
    let c = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let outside = rect_at(&d, 200.0, 20.0, 30.0, 30.0);
    let g = grouped(&d, &[a, b, c]);
    let mut s = open(&d);
    click(&mut s, pt(60.0, 20.0));
    key(&mut s, "Enter");
    click(&mut s, pt(25.0, 20.0));
    shift_click(&mut s, pt(60.0, 20.0));
    let out = s.apply_boolean(BooleanOp::Union);
    assert!(matches!(out, BooleanOutcome::Applied { .. }), "{out:?}");
    let d2 = doc_of(&s);
    let kids = d2.children(Some(g));
    assert_eq!(
        kids.len(),
        2,
        "A and B became one result, C stays: {kids:?}"
    );
    assert!(!kids.contains(&a) && !kids.contains(&b));
    assert_eq!(kids[1], c, "the result sits where the base operand was");
    assert_eq!(d2.children(None).last().copied(), Some(outside));
    assert_eq!(s.context_depth(), 1);
}

#[test]
fn ac20_the_context_falls_back_when_the_entered_group_is_gone_and_is_not_saved() {
    let (d, ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    ctrl_g(&mut s);
    key(&mut s, "Enter");
    assert_eq!(s.context_depth(), 1);
    // The step that made the group is taken back from the inside.
    undo(&mut s);
    assert_eq!(s.context_depth(), 0, "the entered group no longer exists");
    redo(&mut s);
    assert_eq!(
        s.context_depth(),
        0,
        "the entered state is not restored by redo"
    );
    // Entered, saved, reopened: starts at the root.
    click(&mut s, top(0));
    key(&mut s, "Enter");
    assert_eq!(s.context_depth(), 1);
    let bytes = s.pack("0.1.0").unwrap();
    let s2 = Session::open(5, &bytes).unwrap();
    assert_eq!(s2.context_depth(), 0);
    let _ = ids;
}

#[test]
fn ac20_nested_fallback_goes_to_the_nearest_ancestor_that_still_exists() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    // Create a group inside, then enter it.
    click(&mut s, top(1));
    shift_click(&mut s, top(2));
    ctrl_g(&mut s);
    key(&mut s, "Enter");
    assert_eq!(s.context_depth(), 2);
    undo(&mut s); // the inner group goes
    assert_eq!(s.context_depth(), 1, "nearest ancestor that still exists");
}

#[test]
fn ac21_pan_zoom_and_tool_changes_keep_the_context_and_the_dimming() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    let dimmed = art_alpha(&s, pt(20.0, 20.0));
    s.begin_pan(100.0, 100.0);
    s.pan_to(160.0, 130.0);
    s.end_pan();
    s.wheel(0.0, -120.0, 300.0, 300.0, false, true);
    s.wheel(0.0, 50.0, 300.0, 300.0, false, false);
    for t in [
        Tool::Node,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Pen,
        Tool::PolygonStar,
        Tool::Select,
    ] {
        s.set_tool(t);
        assert_eq!(s.context_depth(), 1, "{t:?}");
    }
    assert_eq!(art_alpha(&s, pt(20.0, 20.0)), dimmed, "still dimmed");
}

// ---------------------------------------------------------------------
// Criteria 22 to 26: transforms, fit, style
// ---------------------------------------------------------------------

#[derive(Clone, Copy)]
struct GBox {
    min: Point,
    max: Point,
    k: f64,
}

impl GBox {
    fn new(s: &Session, min: Point, max: Point) -> Self {
        Self {
            min,
            max,
            k: s.view().scale(),
        }
    }
    fn centre(&self) -> Point {
        pt(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }
    fn corner(&self, sx: f64, sy: f64) -> Point {
        let c = self.centre();
        pt(
            c.x + sx * (self.max.x - self.min.x) / 2.0,
            c.y + sy * (self.max.y - self.min.y) / 2.0,
        )
    }
    fn mid(&self, nx: f64, ny: f64) -> Point {
        self.corner(nx, ny)
    }
    fn rot_corner(&self, sx: f64, sy: f64) -> Point {
        let d = 32.0 / SQRT_2 / self.k;
        let p = self.corner(sx, sy);
        pt(p.x + sx * d, p.y + sy * d)
    }
    fn skew(&self, nx: f64, ny: f64) -> Point {
        let p = self.mid(nx, ny);
        pt(p.x + nx * 16.0 / self.k, p.y + ny * 16.0 / self.k)
    }
}

/// A scene of mixed leaves and its group box: two rectangles and an ellipse.
fn mixed_scene() -> (Document, Vec<NodeId>, Point, Point) {
    let d = Document::new(1);
    let a = rect_at(&d, 50.0, 50.0, 20.0, 10.0);
    let b = rect_at(&d, 90.0, 60.0, 10.0, 30.0);
    let c = d.create_ellipse(curvyo_document_core::EllipseFrame {
        center: pt(70.0, 100.0),
        rx: Length::from_mm(8.0),
        ry: Length::from_mm(5.0),
    });
    (d, vec![a, b, c], pt(50.0, 50.0), pt(100.0, 105.0))
}

/// Opens the scene twice: once grouped (group selected by a click), once ungrouped with
/// everything multi-selected.
fn pair(
    build: impl Fn() -> (Document, Vec<NodeId>, Point, Point),
    on_group: Point,
) -> (Session, Session, GBox) {
    let (d1, ids1, min, max) = build();
    let _g = grouped(&d1, &ids1);
    let mut s1 = open(&d1);
    click(&mut s1, on_group);
    let (d2, _ids2, _, _) = build();
    let mut s2 = open(&d2);
    drag(&mut s2, pt(-100.0, -100.0), pt(400.0, 400.0));
    let g = GBox::new(&s1, min, max);
    (s1, s2, g)
}

fn same_leaves(a: &Session, b: &Session, what: &str) {
    let (da, db) = (doc_of(a), doc_of(b));
    let (la, lb) = (da.object_ids(), db.object_ids());
    assert_eq!(la.len(), lb.len());
    for (x, y) in la.iter().zip(&lb) {
        assert_eq!(
            format!("{:?}", da.object(*x).unwrap()),
            format!("{:?}", db.object(*y).unwrap()),
            "{what}"
        );
    }
}

#[test]
fn ac22_move_resize_rotate_of_a_group_equal_the_multi_selection_of_its_leaves() {
    for (name, run) in [
        ("move", 0),
        ("corner resize", 1),
        ("edge resize", 2),
        ("rotate", 3),
        ("shift resize", 4),
    ] {
        let (mut s1, mut s2, g) = pair(mixed_scene, pt(60.0, 50.0));
        let steps = s1.undo_len();
        let g2 = GBox {
            k: s2.view().scale(),
            ..g
        };
        for (s, gb) in [(&mut s1, g), (&mut s2, g2)] {
            match run {
                0 => drag(s, pt(60.0, 50.0), pt(72.0, 57.0)),
                1 => drag(s, gb.corner(1.0, 1.0), pt(130.0, 130.0)),
                2 => drag(s, gb.mid(1.0, 0.0), pt(120.0, 78.0)),
                3 => {
                    let from = gb.rot_corner(1.0, 1.0);
                    let c = gb.centre();
                    let a = 30.0_f64.to_radians();
                    let (dx, dy) = (from.x - c.x, from.y - c.y);
                    let to = pt(
                        c.x + dx * a.cos() - dy * a.sin(),
                        c.y + dx * a.sin() + dy * a.cos(),
                    );
                    drag(s, from, to);
                }
                _ => drag_mod(s, gb.corner(1.0, 1.0), pt(120.0, 140.0), true, false),
            }
        }
        same_leaves(&s1, &s2, name);
        assert_eq!(s1.undo_len(), steps + 1, "{name}: one commit");
        assert_ne!(
            json(&s1),
            json(&open(&mixed_scene().0)),
            "{name}: something changed"
        );
    }
}

#[test]
fn ac22_a_group_does_not_remember_a_rotation_the_box_is_axis_aligned_again() {
    let (mut s1, _s2, g) = pair(mixed_scene, pt(60.0, 50.0));
    let from = g.rot_corner(1.0, 1.0);
    let c = g.centre();
    let a = 45.0_f64.to_radians();
    let (dx, dy) = (from.x - c.x, from.y - c.y);
    let to = pt(
        c.x + dx * a.cos() - dy * a.sin(),
        c.y + dx * a.sin() + dy * a.cos(),
    );
    drag(&mut s1, from, to);
    // The group is still selected; typing a move shows the axis-aligned bounds of the leaves.
    assert_eq!(selected(&mut s1).len(), 3);
    let d = doc_of(&s1);
    let rotations: Vec<String> = d
        .object_ids()
        .iter()
        .map(|id| format!("{:?}", d.object(*id).unwrap()))
        .collect();
    assert_eq!(
        rotations.len(),
        3,
        "rotation lives on the leaves; the group node has none"
    );
}

#[test]
fn ac22_skew_of_a_group_of_paths_equals_the_multi_selection() {
    let build = || {
        let d = Document::new(1);
        let p = open_path(&d, 1, 50.0, 50.0);
        let q = open_path(&d, 2, 60.0, 90.0);
        (d, vec![p, q], pt(50.0, 50.0), pt(100.0, 100.0))
    };
    let (mut s1, mut s2, g) = pair(build, pt(60.0, 55.0));
    let g2 = GBox {
        k: s2.view().scale(),
        ..g
    };
    // The bounds of the two open paths: p spans x 50..90, y 50..60; q x 60..100, y 90..100.
    let (min, max) = (pt(50.0, 50.0), pt(100.0, 100.0));
    let gb1 = GBox { min, max, ..g };
    let gb2 = GBox { min, max, ..g2 };
    let h1 = gb1.skew(0.0, -1.0);
    let h2 = gb2.skew(0.0, -1.0);
    drag(&mut s1, h1, pt(h1.x + 10.0, h1.y));
    drag(&mut s2, h2, pt(h2.x + 10.0, h2.y));
    same_leaves(&s1, &s2, "skew");
}

#[test]
fn ac22_a_single_group_shows_one_box_not_one_per_leaf() {
    let mk = |n: usize| {
        let d = Document::new(1);
        let ids: Vec<NodeId> = (0..n)
            .map(|i| {
                rect_at(
                    &d,
                    20.0 + 40.0 * i as f64 / (n - 1) as f64,
                    20.0,
                    15.0,
                    15.0,
                )
            })
            .collect();
        let _g = grouped(&d, &ids);
        let mut s = open(&d);
        click(&mut s, pt(27.5, 20.0));
        overlay_triangles(&s)
    };
    let two = mk(2);
    let thirty = mk(30);
    assert!(two > 0);
    assert_eq!(
        two, thirty,
        "the overlay does not grow with the leaves of a group"
    );
    // A multi-selection of the same two leaves does draw member boxes.
    let (d, _) = {
        let d = Document::new(1);
        let a = rect_at(&d, 20.0, 20.0, 15.0, 15.0);
        let b = rect_at(&d, 40.0, 20.0, 15.0, 15.0);
        (d, vec![a, b])
    };
    let mut s = open(&d);
    click(&mut s, pt(27.5, 20.0));
    shift_click(&mut s, pt(47.5, 20.0));
    assert!(
        overlay_triangles(&s) > two,
        "member boxes for a multi-selection"
    );
}

#[test]
fn ac23_scale_stroke_width_off_leaves_all_widths_as_they_are_on_scales_them_like_a_multi_selection()
{
    for scale_on in [false, true] {
        let (mut s1, mut s2, g) = pair(mixed_scene, pt(60.0, 50.0));
        s1.set_scale_stroke_width(scale_on);
        s2.set_scale_stroke_width(scale_on);
        let g2 = GBox {
            k: s2.view().scale(),
            ..g
        };
        drag(&mut s1, g.corner(1.0, 1.0), pt(150.0, 170.0));
        drag(&mut s2, g2.corner(1.0, 1.0), pt(150.0, 170.0));
        same_leaves(&s1, &s2, "scaled");
        let d = doc_of(&s1);
        for id in d.object_ids() {
            let w = d.object(id).unwrap().style().stroke.width.as_mm();
            if scale_on {
                assert!((w - 0.25).abs() > 1e-6, "the width scaled: {w}");
            } else {
                assert!((w - 0.25).abs() < 1e-9, "widths stay: {w}");
            }
        }
    }
}

#[test]
fn ac24_fit_to_content_sees_through_groups() {
    let (d1, ids, _, _) = mixed_scene();
    let _g = grouped(&d1, &ids);
    let (d2, _, _, _) = mixed_scene();
    let mut s1 = open(&d1);
    let mut s2 = open(&d2);
    let o1 = s1.fit_document();
    let o2 = s2.fit_document();
    assert_eq!(format!("{o1:?}"), format!("{o2:?}"));
    assert_eq!(s1.size_text(), s2.size_text());
    same_leaves(&s1, &s2, "fit");
}

#[test]
fn ac26_a_style_edit_on_a_group_reaches_every_leaf_below_it_in_one_commit() {
    let (d, ids) = squares(5);
    let inner = grouped(&d, &[ids[2], ids[3]]);
    let _outer = grouped(&d, &[ids[0], ids[1], inner]);
    let mut s = open(&d);
    click(&mut s, top(0));
    let steps = s.undo_len();
    s.set_stroke_cap(LineCap::Round);
    s.set_fill_paint(true);
    assert_eq!(s.undo_len(), steps + 2, "one commit per edit");
    let d2 = doc_of(&s);
    for id in &ids[..4] {
        let st = d2.object(*id).unwrap();
        assert_eq!(format!("{:?}", st.style().stroke.cap), "Round", "{id:?}");
        assert!(st.style().fill.enabled);
    }
    let outside = d2.object(ids[4]).unwrap();
    assert!(
        !outside.style().fill.enabled,
        "a leaf outside the group is untouched"
    );
    // No style on the group nodes themselves: the document still re-reads.
    assert_eq!(json(&s), {
        let r = unpack(7, &s.pack("0.1.0").unwrap()).unwrap();
        r.export_json().unwrap()
    });
}

#[test]
fn ac26_differing_values_show_mixed_and_a_typed_value_unifies_them() {
    let (d, ids) = squares(3);
    d.edit_style(&[ids[0]], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    let _g = grouped(&d, &ids);
    let mut s = open(&d);
    click(&mut s, top(1));
    let v = s.style_panel_view();
    assert!(v.stroke_width_mixed, "widths differ: Mixed");
    let ok = s.set_style_text(StyleField::StrokeWidth, "2");
    assert!(ok.is_ok(), "{ok:?}");
    let v = s.style_panel_view();
    assert!(!v.stroke_width_mixed);
    let d2 = doc_of(&s);
    for id in ids {
        let w = d2.object(id).unwrap().style().stroke.width.as_mm();
        assert!((w - 2.0).abs() < 1e-9 || (w - 2.0).abs() < 1e-3, "{w}");
    }
}

// ---------------------------------------------------------------------
// Criteria 28 to 30: other operations
// ---------------------------------------------------------------------

#[test]
fn ac28_boolean_combine_and_break_apart_refuse_a_selection_with_a_group() {
    let (d, ids) = squares(3);
    let g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(2));
    let before = (json(&s), s.undo_len());
    for op in [
        BooleanOp::Union,
        BooleanOp::Difference,
        BooleanOp::Intersection,
        BooleanOp::Exclusion,
    ] {
        match s.apply_boolean(op) {
            BooleanOutcome::Refused(BooleanRefusal::Groups { offenders }) => {
                assert!(!offenders.is_empty());
                assert!(offenders.iter().all(|o| *o == g || ids[..2].contains(o)));
            }
            other => panic!("{op:?}: {other:?}"),
        }
    }
    match s.apply_combine() {
        CombineOutcome::Refused(CombineRefusal::Groups { offenders }) => {
            assert!(!offenders.is_empty());
        }
        other => panic!("{other:?}"),
    }
    match s.apply_break_apart() {
        BreakApartOutcome::Refused(BreakApartRefusal::Groups { offenders }) => {
            assert!(!offenders.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert_eq!((json(&s), s.undo_len()), before, "nothing was changed");
    assert_eq!(
        curvyo_ui_core::groups_refusal_notice("Union"),
        "Union does not work on groups. Ungroup first. Nothing was changed."
    );
    assert_eq!(
        curvyo_ui_core::groups_refusal_notice("Combine"),
        "Combine does not work on groups. Ungroup first. Nothing was changed."
    );
}

#[test]
fn ac28_a_group_alone_is_refused_too() {
    let (d, ids) = squares(2);
    let _g = grouped(&d, &ids);
    let mut s = open(&d);
    click(&mut s, top(0));
    let before = (json(&s), s.undo_len());
    let out = s.apply_combine();
    assert!(
        matches!(out, CombineOutcome::Refused(CombineRefusal::Groups { .. })),
        "{out:?}"
    );
    let out = s.apply_break_apart();
    assert!(
        matches!(
            out,
            BreakApartOutcome::Refused(BreakApartRefusal::Groups { .. })
        ),
        "{out:?}"
    );
    assert_eq!((json(&s), s.undo_len()), before);
}

#[test]
fn ac29_object_to_path_is_not_offered_for_a_group_and_does_nothing() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, top(2));
    assert!(
        s.select_bar_state().object_to_path,
        "a plain rectangle can be converted"
    );
    click(&mut s, top(0));
    assert!(!s.select_bar_state().object_to_path, "a group alone");
    shift_click(&mut s, top(2));
    assert!(
        !s.select_bar_state().object_to_path,
        "a group and a rectangle"
    );
    let before = (json(&s), s.undo_len());
    s.convert_selected_to_paths();
    assert_eq!((json(&s), s.undo_len()), before);
}

fn pen_scene() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let p = open_path(&d, 1, 20.0, 20.0); // ends at (20,20) and (60,20)
    let q = open_path(&d, 2, 20.0, 100.0);
    let a = rect_at(&d, 150.0, 150.0, 20.0, 20.0);
    let _g = grouped(&d, &[p, a]);
    (d, p, q)
}

#[test]
fn ac30_at_the_root_a_root_level_open_path_is_a_pen_target() {
    let (d, _p, q) = pen_scene();
    let mut s = open(&d);
    s.set_tool(Tool::Pen);
    hold(&mut s, pt(20.0, 100.0), false, false, false);
    let t = format!("{:?}", s.pen_target());
    assert!(
        t.contains("Continue") && t.contains(&format!("{q:?}")),
        "{t}"
    );
}

#[test]
fn ac30_at_the_root_an_open_path_inside_a_group_is_not_a_pen_target() {
    let (d, p, _q) = pen_scene();
    let mut s = open(&d);
    s.set_tool(Tool::Pen);
    for at in [pt(20.0, 20.0), pt(60.0, 20.0)] {
        hold(&mut s, at, false, false, false);
        let t = format!("{:?}", s.pen_target());
        assert!(
            !t.contains(&format!("{p:?}")),
            "an end of a path that is not a child of the context is offered: {t}"
        );
    }
}

#[test]
fn ac30_inside_the_group_its_open_path_is_a_target_and_the_outside_is_not() {
    let (d, p, q) = pen_scene();
    let mut s = open(&d);
    click(&mut s, pt(40.0, 30.0));
    key(&mut s, "Enter");
    s.set_tool(Tool::Pen);
    hold(&mut s, pt(20.0, 20.0), false, false, false);
    let t = format!("{:?}", s.pen_target());
    assert!(
        t.contains("Continue") && t.contains(&format!("{p:?}")),
        "{t}"
    );
    hold(&mut s, pt(20.0, 100.0), false, false, false);
    let t = format!("{:?}", s.pen_target());
    assert!(
        !t.contains(&format!("{q:?}")),
        "outside the entered group: {t}"
    );
}

// ---------------------------------------------------------------------
// Undo and redo of every group step, byte-equal
// ---------------------------------------------------------------------

#[test]
fn undo_and_redo_of_every_group_step_are_byte_equal() {
    let (d, ids) = squares(5);
    let mut s = open(&d);
    let mut states = vec![json(&s)];
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    ctrl_g(&mut s); // group
    states.push(json(&s));
    key(&mut s, "ArrowRight"); // move the group
    states.push(json(&s));
    s.set_stroke_cap(LineCap::Round); // style
    states.push(json(&s));
    drag_mod(&mut s, top(0), pt(top(0).x + 5.0, 120.0), false, true); // ctrl-drag copy
    states.push(json(&s));
    click(&mut s, top(2));
    shift_click(&mut s, top(3));
    ctrl_g(&mut s); // a second group
    states.push(json(&s));
    ctrl_shift_g(&mut s); // ungroup it again
    states.push(json(&s));
    click(&mut s, top(0));
    s.delete_selected(); // delete the first group (original)
    states.push(json(&s));
    for k in (0..states.len() - 1).rev() {
        undo(&mut s);
        assert_eq!(json(&s), states[k], "after undoing to state {k}");
    }
    for (k, state) in states.iter().enumerate().skip(1) {
        redo(&mut s);
        assert_eq!(json(&s), *state, "after redoing to state {k}");
    }
    let _ = ids;
}

#[test]
fn undo_of_a_step_taken_inside_a_group_keeps_the_group() {
    let (d, ids) = squares(3);
    let g = grouped(&d, &[ids[0], ids[1]]);
    let mut s = open(&d);
    click(&mut s, top(0));
    key(&mut s, "Enter");
    let before = json(&s);
    s.set_tool(Tool::Rectangle);
    drag(&mut s, pt(150.0, 100.0), pt(190.0, 140.0));
    s.set_tool(Tool::Select);
    assert_eq!(doc_of(&s).children(Some(g)).len(), 3);
    undo(&mut s);
    assert_eq!(json(&s), before);
    assert_eq!(
        s.context_depth(),
        1,
        "undo of an inner step keeps the context"
    );
    redo(&mut s);
    assert_eq!(doc_of(&s).children(Some(g)).len(), 3);
}

// ---------------------------------------------------------------------
// Hostile inputs and large documents
// ---------------------------------------------------------------------

#[test]
fn hostile_group_commands_on_an_empty_document_and_a_stale_selection_do_nothing() {
    let mut s = Session::new(3);
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    let before = (json(&s), s.undo_len());
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored);
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ignored);
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::Ignored);
    assert_eq!(s.group_selection().kind, GroupingKind::Ignored);
    assert_eq!(s.ungroup_selection().kind, GroupingKind::Ignored);
    assert!(!s.leave_to_level(0) || s.context_depth() == 0);
    assert_eq!(key_with(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Ignored, "select all of nothing");
    assert_eq!((json(&s), s.undo_len()), before);
}

#[test]
fn hostile_group_everything_ungroup_everything_repeatedly_stays_consistent() {
    let (d, ids) = squares(6);
    let mut s = open(&d);
    for _ in 0..5 {
        select_everything(&mut s);
        assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
        assert_eq!(root(&s).len(), 1);
        assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ungroup);
        assert_eq!(root(&s), ids);
    }
    // 10 steps, all undone in order, restore the start.
    let start = {
        let f = open(&squares(6).0);
        json(&f)
    };
    for _ in 0..10 {
        undo(&mut s);
    }
    assert_eq!(json(&s).len(), start.len());
    assert_eq!(root(&s), ids);
}

#[test]
fn large_document_5000_leaves_groups_selects_moves_and_ungroups() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5000)
        .map(|i| {
            rect_at(
                &d,
                f64::from(i % 100) * 12.0,
                f64::from(i / 100) * 12.0,
                10.0,
                10.0,
            )
        })
        .collect();
    let mut s = open(&d);
    let t = std::time::Instant::now();
    select_everything(&mut s);
    let t_sel = t.elapsed();
    let t = std::time::Instant::now();
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    let t_group = t.elapsed();
    assert_eq!(
        s.take_grouping_result().unwrap().notice,
        "Grouped 5000 objects."
    );
    let t = std::time::Instant::now();
    click(&mut s, pt(5.0, 0.0));
    let t_click = t.elapsed();
    let t = std::time::Instant::now();
    let _ = s.draw_list();
    let t_draw = t.elapsed();
    let t = std::time::Instant::now();
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ungroup);
    let t_ungroup = t.elapsed();
    assert_eq!(root(&s), ids);
    eprintln!(
        "5000 leaves (debug build): select all {t_sel:?}, group {t_group:?}, click {t_click:?}, \
         draw {t_draw:?}, ungroup {t_ungroup:?}"
    );
    assert!(t_group.as_secs() < 60 && t_ungroup.as_secs() < 60);
}

/// The timing figures against the budget of 0020 criterion 30 (a step of 5,000 objects: 1 s, asserted
/// at four times). Run: `cargo test --release -p curvyo-editor-wasm --test acceptance_0023_tester
/// -- --ignored --nocapture`.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn budget_5000_leaf_group_steps_in_release() {
    let d = Document::new(1);
    for i in 0..5000 {
        rect_at(
            &d,
            f64::from(i % 100) * 12.0,
            f64::from(i / 100) * 12.0,
            10.0,
            10.0,
        );
    }
    let mut s = open(&d);
    select_everything(&mut s);
    // Control on the same machine and load: a plain move of the same 5,000 objects and its undo.
    key(&mut s, "ArrowRight");
    let t = std::time::Instant::now();
    undo(&mut s);
    let control_undo = t.elapsed();
    let t = std::time::Instant::now();
    redo(&mut s);
    let control_redo = t.elapsed();
    undo(&mut s);
    eprintln!("CONTROL move of 5000 objects: undo {control_undo:?}, redo {control_redo:?}");
    let t = std::time::Instant::now();
    ctrl_g(&mut s);
    let group = t.elapsed();
    let t = std::time::Instant::now();
    undo(&mut s);
    let undone = t.elapsed();
    let t = std::time::Instant::now();
    redo(&mut s);
    let redone = t.elapsed();
    click(&mut s, pt(5.0, 0.0));
    let t = std::time::Instant::now();
    for _ in 0..20 {
        let _ = s.draw_list();
    }
    let draw = t.elapsed() / 20;
    let t = std::time::Instant::now();
    ctrl_shift_g(&mut s);
    let ungroup = t.elapsed();
    eprintln!(
        "group {group:?}, undo {undone:?}, redo {redone:?}, draw {draw:?}, ungroup {ungroup:?}"
    );
    assert!(
        undone.as_millis() <= 4_000,
        "undo of a 5,000-object step: {undone:?}"
    );
    assert!(
        redone.as_millis() <= 4_000,
        "redo of a 5,000-object step: {redone:?}"
    );
    assert!(group.as_millis() <= 4_000, "group: {group:?}");
    assert!(ungroup.as_millis() <= 4_000, "ungroup: {ungroup:?}");
}

// ---------------------------------------------------------------------
// More: Group inside a context, Escape order, files, nudge
// ---------------------------------------------------------------------

#[test]
fn ac19_group_and_ungroup_inside_an_entered_group_work_in_that_context() {
    let (d, ids) = squares(5);
    let g = grouped(&d, &ids[1..4]); // root: A g F ; g = (B, C, D)
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(1));
    shift_click(&mut s, top(3));
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    let d2 = doc_of(&s);
    let kids = d2.children(Some(g));
    assert_eq!(kids.len(), 2, "C and the new group");
    assert_eq!(kids[0], ids[2], "C stays below");
    assert!(d2.is_group(kids[1]));
    assert_eq!(d2.children(Some(kids[1])), vec![ids[1], ids[3]]);
    assert_eq!(s.context_depth(), 1);
    assert_eq!(d2.children(None).len(), 3, "the root is untouched");
    assert_eq!(ctrl_shift_g(&mut s), KeyOutcome::Ungroup);
    let d3 = doc_of(&s);
    assert_eq!(d3.children(Some(g)), vec![ids[2], ids[1], ids[3]]);
}

#[test]
fn ac19_a_ctrl_drag_copy_inside_an_entered_group_lands_in_that_group() {
    let (d, ids) = squares(4);
    let g = grouped(&d, &ids[1..3]);
    let mut s = open(&d);
    let grab = pt(60.0, 26.0); // on B's left edge, away from the handles
    click(&mut s, grab);
    key(&mut s, "Enter");
    click(&mut s, grab);
    drag_mod(&mut s, grab, pt(grab.x, 126.0), false, true);
    let d2 = doc_of(&s);
    assert_eq!(
        d2.children(Some(g)).len(),
        3,
        "the copy joined the entered group"
    );
    assert_eq!(d2.children(None).len(), 3);
    assert_eq!(d2.object_ids().len(), 5);
}

#[test]
fn ac17_escape_closes_a_chip_first_and_cancels_a_drag_before_leaving_the_group() {
    let (d, _ids, _g, _inner) = nested_scene();
    let mut s = open(&d);
    click(&mut s, top(2));
    key(&mut s, "Enter");
    click(&mut s, top(2));
    assert_eq!(key(&mut s, "m"), KeyOutcome::EntryOpened);
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::ClosedEntry)
    );
    assert_eq!(s.context_depth(), 1);
    // A drag in flight.
    let before = json(&s);
    hold(&mut s, top(2), false, false, false);
    s.pointer_down(top(2), false);
    hold(&mut s, pt(top(2).x + 30.0, 90.0), false, false, false);
    assert_eq!(
        key(&mut s, "Escape"),
        KeyOutcome::Escape(EscapeStep::CancelledDrag)
    );
    assert_eq!(s.context_depth(), 1);
    assert_eq!(json(&s), before);
}

#[test]
fn ac22_nudge_of_a_group_equals_the_nudge_of_its_leaves() {
    let (mut s1, mut s2, _g) = pair(mixed_scene, pt(60.0, 50.0));
    for k in ["ArrowRight", "ArrowDown", "ArrowLeft"] {
        key(&mut s1, k);
        key(&mut s2, k);
    }
    s1.set_tool(Tool::Select);
    let _ = key_with(&mut s1, "ArrowUp", false, true);
    let _ = key_with(&mut s2, "ArrowUp", false, true);
    same_leaves(&s1, &s2, "nudge");
}

#[test]
fn ac27_the_session_opens_a_saved_group_project_and_refuses_damaged_ones_without_a_crash() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    let bytes = s.pack("0.1.0").unwrap();
    let r = Session::open(9, &bytes).unwrap();
    assert_eq!(json(&r), json(&s));
    assert_eq!(r.context_depth(), 0);
    click(&mut s, top(0));
    assert_eq!(s.selected_object_count().min(2), 2);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures");
    for (name, ok) in [
        ("groups_v11.curvyo", true),
        ("groups_too_deep_v11.curvyo", false),
        ("groups_leaf_child_v11.curvyo", false),
        ("future_format_version.curvyo", false),
        ("truncated.curvyo", false),
    ] {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let r = Session::open(9, &bytes);
        assert_eq!(r.is_ok(), ok, "{name}");
    }
}

#[test]
fn ac27_a_saved_project_with_groups_is_byte_equal_after_reopen_and_second_save() {
    let (d, ids) = squares(5);
    let inner = grouped(&d, &ids[..2]);
    let _outer = grouped(&d, &[inner, ids[2]]);
    let s = open(&d);
    let first = s.pack("0.1.0").unwrap();
    let s2 = Session::open(4, &first).unwrap();
    assert_eq!(json(&s2), json(&s));
    assert_eq!(doc_of(&s2).children(None), doc_of(&s).children(None));
}

#[test]
fn hover_over_a_member_outlines_the_whole_group_not_the_member() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    let bare = overlay_triangles(&s);
    let extent = |s: &Session| {
        let l = s.draw_list();
        let ov = &l.triangles[l.overlay_start()..];
        let (mut x0, mut x1) = (f64::MAX, f64::MIN);
        for v in ov {
            x0 = x0.min(v.position.x);
            x1 = x1.max(v.position.x);
        }
        (x0, x1)
    };
    hold(&mut s, top(0), false, false, false);
    assert!(overlay_triangles(&s) > bare, "a hover box is drawn");
    let (x0, x1) = extent(&s);
    assert!(
        x0 < 21.0 && x1 > 89.0,
        "the hover box spans the group (20..90): {x0}..{x1}"
    );
    hold(&mut s, top(2), false, false, false);
    let (x0, x1) = extent(&s);
    assert!(
        x0 > 98.0 && x1 < 132.0,
        "a direct child hovers alone: {x0}..{x1}"
    );
}

#[test]
fn history_names_the_group_steps_group_and_ungroup() {
    assert_eq!(curvyo_ui_core::operation_name("group"), "Group");
    assert_eq!(curvyo_ui_core::operation_name("ungroup"), "Ungroup");
    fn notice(s: &mut Session, redo: bool) -> String {
        let out = if redo {
            key_with(s, "Z", true, true)
        } else {
            key_with(s, "z", true, false)
        };
        match out {
            KeyOutcome::UndoBusy => s.undo().notice,
            KeyOutcome::RedoBusy => s.redo().notice,
            _ => s.take_history_notice().unwrap_or_default(),
        }
    }
    let (d, _ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(1));
    ctrl_g(&mut s);
    let n = notice(&mut s, false);
    eprintln!("undo of Group: {n:?}");
    assert!(n.starts_with("Undid: Group"), "{n}");
    let n = notice(&mut s, true);
    eprintln!("redo of Group: {n:?}");
    assert!(n.starts_with("Redid: Group"), "{n}");
    ctrl_shift_g(&mut s);
    let n = notice(&mut s, false);
    eprintln!("undo of Ungroup: {n:?}");
    assert!(n.starts_with("Undid: Ungroup"), "{n}");
}

#[test]
fn delete_key_removes_a_selected_group_and_one_undo_brings_it_back() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    click(&mut s, top(0));
    let before = json(&s);
    assert_eq!(key(&mut s, "Delete"), KeyOutcome::Deleted);
    assert_eq!(root(&s), vec![ids[2]]);
    undo(&mut s);
    assert_eq!(json(&s), before);
    assert_eq!(key(&mut s, "Backspace"), KeyOutcome::Deleted);
    assert_eq!(root(&s), vec![ids[2]]);
}

// ---------------------------------------------------------------------
// Edge cases found by reading the criteria again
// ---------------------------------------------------------------------

#[test]
fn leaving_a_group_with_a_part_selected_never_leaves_a_half_selected_group() {
    let (d, ids) = squares(4);
    let _g = grouped(&d, &ids[..3]);
    let mut s = open(&d);
    click(&mut s, top(0));
    key(&mut s, "Enter");
    click(&mut s, top(1)); // one member of the entered group
    assert!(same_set(&selected(&mut s), &[ids[1]]));
    assert!(s.leave_to_level(0));
    let sel = selected(&mut s);
    assert!(
        sel.is_empty() || same_set(&sel, &ids[..3]),
        "after leaving, the selection is nothing or the whole group, not a part: {sel:?}"
    );
    // Whatever it is, the commands work and never corrupt the tree.
    shift_click(&mut s, top(3));
    let r = s.group_selection();
    assert_ne!(format!("{:?}", r.kind), "Refused");
    let d2 = doc_of(&s);
    assert_eq!(d2.object_ids().len(), 4);
}

#[test]
fn undo_and_redo_of_group_restore_a_sensible_selection() {
    let (d, ids) = squares(3);
    let mut s = open(&d);
    click(&mut s, top(0));
    shift_click(&mut s, top(2));
    ctrl_g(&mut s);
    undo(&mut s);
    let ann = s.selection_announcement();
    eprintln!("selection after undo of Group: {ann:?}");
    let a = s.grouping_availability();
    assert!(!a.ungroup, "no group exists after the undo");
    assert_eq!(root(&s), ids);
    redo(&mut s);
    let ann = s.selection_announcement();
    eprintln!("selection after redo of Group: {ann:?}");
    let a = s.grouping_availability();
    assert_eq!(root(&s).len(), 2, "the group is back");
    assert_eq!(
        a.ungroup,
        ann.starts_with("Group of"),
        "availability and announcement agree: {ann:?}, {a:?}"
    );
    if ann.is_empty() {
        assert!(!a.group);
    }
}

#[test]
fn a_click_in_the_gap_inside_a_group_box_hits_nothing_and_clears_the_selection() {
    let (d, ids) = squares(3);
    for id in &ids {
        d.edit_style(&[*id], &StyleEdit::FillEnabled(true)).unwrap();
    }
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    // Between A (20..50) and B (60..90), inside the group box: with nothing selected it is empty
    // canvas (a click inside an existing selection box keeps it, as for a multi-selection).
    click(&mut s, pt(55.0, 35.0));
    assert!(selected(&mut s).is_empty(), "the gap is empty canvas");
    click(&mut s, top(0));
    assert_eq!(selected(&mut s).len(), 2);
    click(&mut s, pt(300.0, 300.0));
    assert!(selected(&mut s).is_empty());
}

#[test]
fn ac9_a_lasso_chooses_among_top_level_objects() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    // Alt at the press starts a lasso. Cross B's bottom edge (y = 50) and end inside B.
    hold(&mut s, pt(75.0, 70.0), false, false, true);
    s.pointer_down(pt(75.0, 70.0), false);
    for p in [
        pt(75.0, 60.0),
        pt(75.0, 52.0),
        pt(75.0, 40.0),
        pt(76.0, 38.0),
    ] {
        hold(&mut s, p, false, false, true);
    }
    s.pointer_up(pt(76.0, 38.0), false, false);
    hold(&mut s, pt(76.0, 38.0), false, false, false);
    assert!(
        same_set(&selected(&mut s), &ids[..2]),
        "the group, not C and not B alone"
    );
}

#[test]
fn ac22_the_announcement_gives_the_leaf_count_and_the_size_of_the_group_box() {
    let (d, ids) = squares(3);
    let _g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    click(&mut s, top(0));
    let a = s.selection_announcement();
    eprintln!("announcement: {a:?}");
    assert!(a.starts_with("Group of 2 objects selected"), "{a}");
    assert!(
        a.contains("70.0") && a.contains("30.0"),
        "box 70 x 30 mm, stroke excluded: {a}"
    );
}

#[test]
fn ac22_a_group_of_degenerate_lines_does_not_break_the_transform() {
    let build = || {
        let d = Document::new(1);
        let mk = |n: u64, y: f64| {
            d.create_path(
                &[
                    NewAnchor::corner(AnchorId::new(7, n * 2), pt(60.0, y)),
                    NewAnchor::corner(AnchorId::new(7, n * 2 + 1), pt(60.0, y + 20.0)),
                ],
                false,
            )
        };
        let a = mk(1, 20.0);
        let b = mk(2, 60.0);
        (d, vec![a, b], pt(60.0, 20.0), pt(60.0, 80.0))
    };
    let (mut s1, mut s2, g) = pair(build, pt(60.0, 30.0));
    let g2 = GBox {
        k: s2.view().scale(),
        ..g
    };
    for run in 0..3 {
        let (h1, h2) = match run {
            0 => (g.corner(1.0, 1.0), g2.corner(1.0, 1.0)),
            1 => (g.mid(0.0, 1.0), g2.mid(0.0, 1.0)),
            _ => (g.rot_corner(1.0, 1.0), g2.rot_corner(1.0, 1.0)),
        };
        drag(&mut s1, h1, pt(h1.x + 15.0, h1.y + 25.0));
        drag(&mut s2, h2, pt(h2.x + 15.0, h2.y + 25.0));
    }
    same_leaves(&s1, &s2, "degenerate box");
    let d = doc_of(&s1);
    let text = format!(
        "{:?}",
        d.object_ids()
            .iter()
            .map(|i| d.object(*i))
            .collect::<Vec<_>>()
    );
    assert!(
        !text.contains("NaN") && !text.contains("inf"),
        "finite geometry"
    );
}

#[test]
fn ac5_the_32nd_level_can_be_entered_and_a_group_made_there_is_refused() {
    let d = Document::new(1);
    let mut g = grouped(
        &d,
        &[
            rect_at(&d, 20.0, 20.0, 30.0, 30.0),
            rect_at(&d, 60.0, 20.0, 30.0, 30.0),
        ],
    );
    for i in 1..32 {
        let x = rect_at(&d, 20.0 + 40.0 * f64::from(i + 1), 80.0, 30.0, 30.0);
        g = grouped(&d, &[g, x]);
    }
    let mut s = open(&d);
    for level in 1..=32 {
        click(&mut s, pt(35.0, 20.0)); // the deepest leaf: its unit in this context
        assert_eq!(
            key(&mut s, "Enter"),
            KeyOutcome::EnteredGroup,
            "level {level}"
        );
        assert_eq!(s.context_depth(), level);
    }
    let a = s.take_context_announcement();
    eprintln!("announcement at 32: {a:?}");
    // Inside the innermost group: its two leaves. Grouping them would be level 33.
    click(&mut s, pt(35.0, 20.0));
    shift_click(&mut s, pt(75.0, 20.0));
    let before = (json(&s), s.undo_len());
    ctrl_g(&mut s);
    let r = s.take_grouping_result().unwrap();
    assert_eq!(r.kind, GroupingKind::Refused);
    assert_eq!(
        r.notice,
        "Groups can be nested 32 levels deep. Nothing was changed."
    );
    assert_eq!((json(&s), s.undo_len()), before);
    // And everything can be left again in one step.
    assert!(s.leave_to_level(0));
    assert_eq!(s.context_depth(), 0);
}

fn empty_group_file() -> Vec<u8> {
    use loro::{ExportMode, LoroDoc, TreeParentId};
    use std::io::{Cursor, Write};
    let (d, _ids) = squares(3);
    let bytes = pack(&d, "t").unwrap();
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let t = l.get_tree("paths");
    t.enable_fractional_index(0);
    let gid = t.create(TreeParentId::Root).unwrap();
    t.get_meta(gid).unwrap().insert("group", true).unwrap();
    l.commit();
    let mut zr = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut read = |n: &str| {
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut zr.by_name(n).unwrap(), &mut v).unwrap();
        v
    };
    let manifest = read("manifest.json");
    let jsn = read("document.json");
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    w.start_file("manifest.json", o).unwrap();
    w.write_all(&manifest).unwrap();
    w.start_file("document.loro", o).unwrap();
    w.write_all(&l.export(ExportMode::Snapshot).unwrap())
        .unwrap();
    w.start_file("document.json", o).unwrap();
    w.write_all(&jsn).unwrap();
    w.finish().unwrap().into_inner()
}

#[test]
fn ac27_a_file_with_an_empty_group_opens_and_every_command_survives_it() {
    let file = empty_group_file();
    let mut s = Session::open(3, &file).expect("an empty group is valid on read");
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    let _ = s.draw_list();
    select_everything(&mut s);
    assert_eq!(selected(&mut s).len(), 3);
    assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
    ctrl_shift_g(&mut s);
    click(&mut s, pt(300.0, 300.0)); // clear the selection
    click(&mut s, top(0));
    s.delete_selected();
    let d2 = doc_of(&s);
    eprintln!(
        "root after the delete: {:?}",
        d2.children(None)
            .iter()
            .map(|c| (d2.is_group(*c), d2.children(Some(*c)).len()))
            .collect::<Vec<_>>()
    );
    assert_eq!(d2.object_ids().len(), 2);
    let _ = d2;
    let _ = s.pack("0.1.0").unwrap();
}

/// `adrs.md` (feature-local decision on concurrency): "the next local command that touches its
/// parent removes it" -- an empty group that came in from a file or a peer.
#[test]
fn adr_an_empty_group_from_a_file_is_removed_by_the_next_command_that_touches_its_parent() {
    let file = empty_group_file();
    let mut s = Session::open(3, &file).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    click(&mut s, top(0));
    s.delete_selected();
    let d2 = doc_of(&s);
    let empty_left = d2
        .children(None)
        .iter()
        .any(|c| d2.is_group(*c) && d2.children(Some(*c)).is_empty());
    assert!(
        !empty_left,
        "an empty group is left in the document after a delete at its parent"
    );
}

#[test]
fn ac14_select_all_and_delete_inside_a_group_removes_it_and_leaves_the_context() {
    let (d, ids) = squares(4);
    let g = grouped(&d, &ids[1..3]);
    let mut s = open(&d);
    click(&mut s, top(1));
    key(&mut s, "Enter");
    let _ = s.take_context_announcement();
    assert_eq!(key_with(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(key(&mut s, "Delete"), KeyOutcome::Deleted);
    let d2 = doc_of(&s);
    assert!(!d2.contains_node(g));
    assert_eq!(d2.children(None), vec![ids[0], ids[3]]);
    assert_eq!(s.context_depth(), 0);
    // Not a criterion: a forced fall-back is a leave, and a screen-reader user hears nothing.
    eprintln!(
        "OBSERVATION: announcement after a forced leave: {:?}",
        s.take_context_announcement()
    );
}

#[test]
fn ac19_combine_inside_an_entered_group_keeps_the_result_in_the_group() {
    let d = Document::new(1);
    let square = |n: u64, x: f64| {
        let a =
            |k: u64, px: f64, py: f64| NewAnchor::corner(AnchorId::new(9, n * 10 + k), pt(px, py));
        d.create_path(
            &[
                a(0, x, 20.0),
                a(1, x + 30.0, 20.0),
                a(2, x + 30.0, 50.0),
                a(3, x, 50.0),
            ],
            true,
        )
    };
    let p1 = square(1, 20.0);
    let p2 = square(2, 70.0);
    let keep = rect_at(&d, 200.0, 20.0, 30.0, 30.0);
    let g = grouped(&d, &[p1, p2, keep]);
    let mut s = open(&d);
    click(&mut s, pt(35.0, 20.0));
    key(&mut s, "Enter");
    drag(&mut s, pt(10.0, 10.0), pt(120.0, 60.0));
    let out = s.apply_combine();
    assert!(matches!(out, CombineOutcome::Applied { .. }), "{out:?}");
    let d2 = doc_of(&s);
    let kids = d2.children(Some(g));
    assert_eq!(kids.len(), 2, "the compound and the rectangle: {kids:?}");
    assert_eq!(kids[1], keep);
    assert_eq!(d2.children(None), vec![g]);
    assert_eq!(s.context_depth(), 1);
    // Break apart inside the group, too.
    click(&mut s, pt(300.0, 300.0));
    click(&mut s, pt(35.0, 20.0));
    let out = s.apply_break_apart();
    assert!(matches!(out, BreakApartOutcome::Applied { .. }), "{out:?}");
    let d3 = doc_of(&s);
    assert_eq!(d3.children(Some(g)).len(), 3);
    assert_eq!(d3.children(None), vec![g]);
}

#[test]
fn ac15_object_to_path_works_on_primitives_inside_an_entered_group() {
    let (d, ids) = squares(3);
    let g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    click(&mut s, top(0));
    key(&mut s, "Enter");
    click(&mut s, top(0));
    assert!(s.select_bar_state().object_to_path);
    s.convert_selected_to_paths();
    let d2 = doc_of(&s);
    let kids = d2.children(Some(g));
    assert_eq!(kids.len(), 2);
    assert!(
        d2.path(kids[0]).is_some() || d2.path(kids[1]).is_some(),
        "converted in place"
    );
    assert_eq!(d2.children(None).len(), 2);
}

#[test]
fn ac26_a_marker_on_a_group_does_what_the_same_marker_on_the_multi_selection_does() {
    use curvyo_document_core::MarkerShape;
    use curvyo_ui_core::MarkerSlot;
    let run = |group: bool| {
        let d = Document::new(1);
        let square = |n: u64, x: f64| {
            let a = |k: u64, px: f64, py: f64| {
                NewAnchor::corner(AnchorId::new(9, n * 10 + k), pt(px, py))
            };
            d.create_path(
                &[
                    a(0, x, 20.0),
                    a(1, x + 30.0, 20.0),
                    a(2, x + 30.0, 50.0),
                    a(3, x, 50.0),
                ],
                true,
            )
        };
        let _p1 = square(1, 20.0);
        let _p2 = square(2, 70.0);
        let _q = open_path(&d, 3, 200.0, 20.0);
        let mut s = open(&d);
        drag(&mut s, pt(10.0, 10.0), pt(120.0, 60.0));
        let out = s.apply_combine();
        assert!(matches!(out, CombineOutcome::Applied { .. }), "{out:?}");
        shift_click(&mut s, pt(220.0, 30.0));
        if group {
            assert_eq!(ctrl_g(&mut s), KeyOutcome::Group);
        }
        let steps = s.undo_len();
        s.set_marker_shape(MarkerSlot::End, MarkerShape::Arrow);
        assert_eq!(s.undo_len(), steps + 1, "one commit");
        let d2 = doc_of(&s);
        d2.object_ids()
            .iter()
            .map(|i| format!("{:?}", d2.object(*i).unwrap().style()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        run(true),
        run(false),
        "styles of the leaves, group against multi-selection"
    );
}

#[test]
fn ac15_inside_a_group_a_member_moves_alone_and_the_group_box_follows_it() {
    let (d, ids) = squares(3);
    let g = grouped(&d, &ids[..2]);
    let mut s = open(&d);
    click(&mut s, top(0));
    key(&mut s, "Enter");
    let grab = pt(60.0, 26.0); // B's left edge, away from the handles
    click(&mut s, grab);
    let a_before = format!("{:?}", doc_of(&s).object(ids[0]));
    drag(&mut s, grab, pt(grab.x + 5.0, grab.y + 40.0));
    let d2 = doc_of(&s);
    assert_eq!(
        format!("{:?}", d2.object(ids[0])),
        a_before,
        "A did not move"
    );
    assert_ne!(format!("{:?}", d2.object(ids[1])), a_before);
    assert_eq!(d2.children(Some(g)), vec![ids[0], ids[1]], "still members");
    // Leave: the group box is the union of the leaves as they are now.
    assert!(s.leave_to_level(0));
    click(&mut s, top(0));
    let a = s.selection_announcement();
    assert!(a.starts_with("Group of 2 objects selected"), "{a}");
    assert!(
        a.contains("70.0") && a.contains("70.0 mm") || a.contains("by 70.0"),
        "box height grew by 40 and 30 high: {a}"
    );
}

#[test]
fn ac14_a_boolean_that_leaves_nothing_removes_the_emptied_group_in_the_same_commit() {
    // B (on top) covers A completely: Difference (bottom minus the others) is empty.
    let d = Document::new(1);
    let a = rect_at(&d, 30.0, 30.0, 10.0, 10.0);
    let b = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let other = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let g = grouped(&d, &[a, b]);
    let mut s = open(&d);
    click(&mut s, pt(35.0, 30.0));
    key(&mut s, "Enter");
    key_with(&mut s, "a", true, false);
    let steps = s.undo_len();
    let out = s.apply_boolean(BooleanOp::Difference);
    let d2 = doc_of(&s);
    eprintln!(
        "difference to nothing: {out:?}, steps +{}, g alive {}",
        s.undo_len() - steps,
        d2.contains_node(g)
    );
    let empty_left = d2
        .children(None)
        .iter()
        .chain(d2.children(Some(g)).iter())
        .any(|c| d2.is_group(*c) && d2.children(Some(*c)).is_empty());
    assert!(!empty_left, "no empty group is left behind");
    assert!(d2.object(other).is_some());
    if matches!(out, BooleanOutcome::Applied { .. }) && d2.object_ids().len() == 1 {
        assert!(!d2.contains_node(g), "the group lost its last child");
        assert_eq!(s.context_depth(), 0, "and the context moved to the parent");
    }
}
