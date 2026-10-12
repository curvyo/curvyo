//! Tester's session-level acceptance tests for `specs/0038-path-offset`: availability and the
//! remembered values (criterion 1), the preview (2), closing without writing (4), Apply as one
//! commit with the results selected (5, 13), vanishing operands (7), open-path inset refusal (8),
//! originals kept, results above their own operands with a copied style (10), groups refused (14),
//! and undo/redo of Apply (`0020`). Written from the specification.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    missing_docs
)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, Color, Document, Length, NewAnchor, NodeId, Point, RectBounds, StyleEdit, pack,
    unpack,
};
use curvyo_editor_wasm::{
    KeyInput, KeyOutcome, OFFSET_COMMIT_LABEL, OffsetOutcome, OffsetPreviewState, Session, Tool,
};
use curvyo_ui_core::{OffsetJoin, OffsetRefusal};

static TIME: AtomicU64 = AtomicU64::new(1_000);
static NEXT: AtomicU64 = AtomicU64::new(1);

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

fn open_line(d: &Document, from: Point, to: Point) -> NodeId {
    let a = |p| NewAnchor::corner(AnchorId::new(55, NEXT.fetch_add(1, Ordering::SeqCst)), p);
    d.create_path(&[a(from), a(to)], false)
}

fn session(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn json(s: &Session) -> Vec<u8> {
    doc_of(s).export_json().unwrap()
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

fn select_all(s: &mut Session) {
    key_with(s, "Escape", false, false);
    assert_eq!(key_with(s, "a", true, false), KeyOutcome::SelectedAll);
}

fn click(s: &mut Session, p: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
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

/// The selected objects, found by a nudge that is taken back again.
fn selected(s: &mut Session) -> Vec<NodeId> {
    let before_json = json(s);
    let before: Vec<(NodeId, String)> = snaps(&doc_of(s));
    if !matches!(
        key_with(s, "ArrowRight", false, false),
        KeyOutcome::Nudged { .. }
    ) {
        return Vec::new();
    }
    let after = snaps(&doc_of(s));
    undo(s);
    assert_eq!(json(s), before_json);
    let mut out: Vec<NodeId> = after
        .iter()
        .filter(|(id, a)| {
            before
                .iter()
                .find(|(b, _)| b == id)
                .is_none_or(|(_, b)| b != a)
        })
        .map(|(id, _)| *id)
        .collect();
    out.sort_by_key(|id| format!("{id:?}"));
    out
}

fn snaps(d: &Document) -> Vec<(NodeId, String)> {
    d.object_ids()
        .into_iter()
        .map(|id| (id, format!("{:?}", d.object(id).unwrap())))
        .collect()
}

fn overlay_triangles(s: &Session) -> usize {
    let l = s.draw_list();
    l.triangles.len() - l.overlay_start()
}

// ------------------------------------------------------------- criterion 1 ---------------------

#[test]
fn ac1_offered_only_with_an_operand_and_values_remembered() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let g_member = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let g_other = rect_at(&d, 140.0, 20.0, 30.0, 30.0);
    let _g = d.group_objects(&[g_member, g_other]).unwrap();
    let mut s = session(&d);
    assert!(!s.offset_availability().offered, "nothing selected");
    assert!(!s.open_offset());
    click(&mut s, pt(35.0, 20.0));
    let av = s.offset_availability();
    assert!(av.offered && !av.has_open);
    assert_eq!(s.offset_settings().distance_mm(), 1.0, "1 mm on first use");
    assert_eq!(
        s.offset_settings().join(),
        OffsetJoin::Round,
        "Round by default"
    );
    assert!(s.open_offset());
    assert!(s.set_offset_distance(3.5));
    s.set_offset_join(OffsetJoin::Bevel);
    assert!(!s.set_offset_distance(1000.5), "typed range is -1000..1000");
    assert!(!s.set_offset_distance(f64::NAN));
    s.close_offset();
    assert!(s.open_offset());
    assert_eq!(
        s.offset_settings().distance_mm(),
        3.5,
        "last value of the session"
    );
    assert_eq!(s.offset_settings().join(), OffsetJoin::Bevel);
    // Another tool: not offered.
    s.set_tool(Tool::Rectangle);
    assert!(!s.offset_open());
    assert!(!s.offset_availability().offered);
    let _ = a;
}

/// Criterion 1 and the UX state table: "Nothing, or only groups: not in the bar".
#[test]
fn ac1_a_group_alone_is_not_offered() {
    let d = Document::new(1);
    let m1 = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let m2 = rect_at(&d, 140.0, 20.0, 30.0, 30.0);
    let _g = d.group_objects(&[m1, m2]).unwrap();
    let mut s = session(&d);
    select_all(&mut s);
    let before = json(&s);
    let offered = s.offset_availability().offered;
    let opened = s.open_offset();
    let outcome = if opened { Some(s.apply_offset()) } else { None };
    assert_eq!(json(&s), before, "nothing written");
    assert!(
        !offered && !opened,
        "a group alone is offered (offered={offered}, opened={opened}, apply={outcome:?})"
    );
}

#[test]
fn ac1_cap_shown_only_with_an_open_path() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let _ = open_line(&d, pt(100.0, 60.0), pt(140.0, 60.0));
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    assert!(!s.offset_availability().has_open);
    select_all(&mut s);
    assert!(s.offset_availability().has_open);
}

// ------------------------------------------------------------- criteria 2, 4 -------------------

#[test]
fn ac2_ac4_preview_shows_and_closing_writes_nothing() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    let before = json(&s);
    let steps = s.undo_len();
    let plain = overlay_triangles(&s);
    assert!(s.open_offset());
    assert!(matches!(
        s.offset_preview_state(),
        OffsetPreviewState::Result { vanished: 0, of: 1 }
    ));
    let with_preview = overlay_triangles(&s);
    assert!(with_preview > plain, "the preview draws an outline");
    assert_eq!(json(&s), before, "the preview is never stored");

    // Escape closes it first.
    key_with(&mut s, "Escape", false, false);
    assert!(!s.offset_open());
    assert_eq!(s.offset_preview_state(), OffsetPreviewState::Closed);
    assert_eq!(overlay_triangles(&s), plain, "the preview disappears");
    // Escape was the first step only: the selection survives.
    assert!(s.offset_availability().offered);

    // Tool change closes.
    assert!(s.open_offset());
    s.set_tool(Tool::Ellipse);
    s.set_tool(Tool::Select);
    assert!(!s.offset_open());

    // A press on empty canvas closes.
    click(&mut s, pt(35.0, 20.0));
    assert!(s.open_offset());
    click(&mut s, pt(600.0, 600.0));
    assert!(!s.offset_open());

    // A selection change closes.
    click(&mut s, pt(35.0, 20.0));
    assert!(s.open_offset());
    select_all(&mut s);
    assert!(!s.offset_open(), "another selection closes the entry");

    assert_eq!(json(&s), before);
    assert_eq!(s.undo_len(), steps);
}

// ------------------------------------------------------------- criteria 5, 10, 13 --------------

#[test]
fn ac5_ac10_ac13_apply_one_commit_results_above_their_operands_and_selected() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let b = rect_at(&d, 30.0, 30.0, 30.0, 30.0);
    let top = rect_at(&d, 300.0, 300.0, 10.0, 10.0);
    d.edit_style(&[b], &StyleEdit::StrokeWidth(Length::from_mm(2.5)))
        .unwrap();
    d.edit_style(
        &[b],
        &StyleEdit::StrokeColor(Color {
            r: 200,
            g: 10,
            b: 30,
        }),
    )
    .unwrap();
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    // shift-click b
    s.modifiers_changed(true, false, false);
    s.pointer_hover(pt(45.0, 30.0), true, false);
    s.pointer_down(pt(45.0, 30.0), true);
    s.pointer_up(pt(45.0, 30.0), true, false);
    s.modifiers_changed(false, false, false);
    let originals = snaps(&doc_of(&s));
    let steps = s.undo_len();
    assert!(s.open_offset());
    assert!(s.set_offset_distance(5.0));
    let outcome = s.apply_offset();
    assert_eq!(
        outcome,
        OffsetOutcome::Applied {
            added: 2,
            vanished: 0,
            of: 2,
            distance_mm: 5.0
        },
        "overlapping results are not merged"
    );
    assert!(!s.offset_open(), "the entry closes");
    assert_eq!(s.undo_len(), steps + 1, "exactly one commit");
    let after = doc_of(&s);
    for (id, snap) in &originals {
        assert_eq!(
            &format!("{:?}", after.object(*id).unwrap()),
            snap,
            "original kept"
        );
    }
    let order = after.children(None);
    assert_eq!(order.len(), 5);
    let pos = |id: NodeId| order.iter().position(|x| *x == id).unwrap();
    let ra = order[pos(a) + 1];
    let rb = order[pos(b) + 1];
    assert!(pos(a) < pos(b) && pos(b) < pos(top));
    assert_eq!(pos(ra), pos(a) + 1, "result of a directly above a");
    assert_eq!(pos(rb), pos(b) + 1, "result of b directly above b");
    assert_eq!(
        after.object(rb).unwrap().style(),
        after.object(b).unwrap().style(),
        "complete style copied"
    );
    assert_eq!(
        after.object(ra).unwrap().style(),
        after.object(a).unwrap().style()
    );
    let mut want = vec![ra, rb];
    want.sort_by_key(|id| format!("{id:?}"));
    assert_eq!(selected(&mut s), want, "the results are the selection");
    assert_eq!(OFFSET_COMMIT_LABEL, "offset_path");
}

#[test]
fn ac5_zero_distance_closes_and_writes_nothing() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    let before = json(&s);
    assert!(s.open_offset());
    assert!(s.set_offset_distance(0.0));
    assert_eq!(s.apply_offset(), OffsetOutcome::ClosedAtZero);
    assert!(!s.offset_open());
    assert_eq!(json(&s), before);
}

// ------------------------------------------------------------- criteria 7, 8 -------------------

#[test]
fn ac7_some_vanish_others_survive_and_all_vanishing_is_refused() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 4.0, 4.0);
    let _ = rect_at(&d, 100.0, 20.0, 5.0, 5.0);
    let _ = rect_at(&d, 200.0, 20.0, 60.0, 60.0);
    let mut s = session(&d);
    select_all(&mut s);
    let before = json(&s);
    assert!(s.open_offset());
    assert!(s.set_offset_distance(-10.0));
    assert_eq!(
        s.apply_offset(),
        OffsetOutcome::Applied {
            added: 1,
            vanished: 2,
            of: 3,
            distance_mm: -10.0
        }
    );
    undo(&mut s);
    assert_eq!(json(&s), before);
    select_all(&mut s);
    assert!(s.open_offset());
    assert!(s.set_offset_distance(-40.0));
    assert!(matches!(
        s.offset_preview_state(),
        OffsetPreviewState::Refused(OffsetRefusal::Empty)
    ));
    assert_eq!(
        s.apply_offset(),
        OffsetOutcome::Refused(OffsetRefusal::Empty)
    );
    assert!(s.offset_open(), "a refusal keeps the entry open");
    assert_eq!(json(&s), before);
}

#[test]
fn ac8_open_path_inset_refused_and_outset_closes_it() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let line = open_line(&d, pt(100.0, 60.0), pt(140.0, 60.0));
    let mut s = session(&d);
    select_all(&mut s);
    let before = json(&s);
    assert!(s.open_offset());
    assert!(s.set_offset_distance(-1.0));
    assert_eq!(
        s.apply_offset(),
        OffsetOutcome::Refused(OffsetRefusal::InsetOfOpenPaths {
            offenders: vec![line],
            of: 2
        })
    );
    assert_eq!(json(&s), before);
    assert!(s.set_offset_distance(3.0));
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::Applied { added: 2, .. }
    ));
}

// ------------------------------------------------------------- criterion 14 --------------------

#[test]
fn ac14_group_with_other_objects_is_refused_and_nothing_written() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let m1 = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let m2 = rect_at(&d, 140.0, 20.0, 30.0, 30.0);
    let _ = d.group_objects(&[m1, m2]).unwrap();
    let mut s = session(&d);
    select_all(&mut s);
    assert!(
        s.offset_availability().offered,
        "shown for a group with other objects"
    );
    let before = json(&s);
    let steps = s.undo_len();
    assert!(s.open_offset());
    assert_eq!(s.apply_offset(), OffsetOutcome::RefusedGroups { leaves: 2 });
    assert_eq!(json(&s), before);
    assert_eq!(s.undo_len(), steps);
}

/// Criterion 2: the preview shows "the result of Apply". With a group in the selection Apply is
/// refused (criterion 14), so no result outline may be previewed.
#[test]
fn ac2_ac14_no_result_preview_when_apply_would_refuse_a_group() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let m1 = rect_at(&d, 100.0, 20.0, 30.0, 30.0);
    let m2 = rect_at(&d, 140.0, 20.0, 30.0, 30.0);
    let _ = d.group_objects(&[m1, m2]).unwrap();
    let mut s = session(&d);
    select_all(&mut s);
    let plain = overlay_triangles(&s);
    assert!(s.open_offset());
    let state = s.offset_preview_state();
    let drawn = overlay_triangles(&s);
    assert!(
        !matches!(state, OffsetPreviewState::Result { .. }) && drawn == plain,
        "preview shows a result Apply will refuse: {state:?}, overlay {plain} -> {drawn}"
    );
}

// ------------------------------------------------------------- undo of Apply (0020) ------------

#[test]
fn undo_and_redo_of_apply() {
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    let before = json(&s);
    assert!(s.open_offset());
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::Applied { added: 1, .. }
    ));
    let applied = json(&s);
    assert_ne!(applied, before);
    undo(&mut s);
    assert_eq!(json(&s), before, "Ctrl+Z takes the whole step back");
    redo(&mut s);
    assert_eq!(json(&s), applied, "redo brings the result back");
}

#[test]
fn preview_equals_apply_for_a_circle_like_operand() {
    // Criterion 3: what Apply writes is what the last preview showed (same node count here).
    let d = Document::new(1);
    let _ = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let mut s = session(&d);
    click(&mut s, pt(35.0, 20.0));
    assert!(s.open_offset());
    assert!(s.set_offset_distance(4.0));
    let OffsetPreviewState::Result { of, vanished } = s.offset_preview_state() else {
        panic!("preview")
    };
    assert_eq!((of, vanished), (1, 0));
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::Applied {
            added: 1,
            vanished: 0,
            of: 1,
            ..
        }
    ));
}
