//! The pen window's link at session level (`specs/0158-pen-display` slice 2a, ADR 0096 A5, A6,
//! A12): two `Session`s and a byte shuttle in one process, no window.
#![allow(clippy::assert_is_empty)]

use curvyo_document_core::Point;
use curvyo_ui_core::window_bridge::{Bundle, Control, Message, ViewMessage, decode, encode};

use super::align_test_support::{click, draw_rect};
use super::window_link::{LinkRole, LinkState};
use super::{Session, Tool};

const MAIN_PEER: u64 = 0xA1;

/// The main window, the pen window and the hub between them; counts what crossed.
pub(super) struct Pair {
    pub(super) main: Session,
    pub(super) pen: Session,
    next_pen_peer: u64,
    /// Asset items carried main to pen and pen to main.
    pub(super) assets_sent: (usize, usize),
    /// Bundles carried main to pen and pen to main.
    pub(super) bundles: (usize, usize),
    /// Strips the pictures from main's bundles (an update arrives before its asset).
    pub(super) drop_main_assets: bool,
    /// Every view that crossed, with `true` for main to pen (2b).
    pub(super) views: Vec<(bool, ViewMessage)>,
}

fn views_of(bundle: &Bundle) -> impl Iterator<Item = ViewMessage> + '_ {
    bundle.controls.iter().filter_map(|c| match c {
        Control::View(view) => Some(*view),
        _ => None,
    })
}

impl Pair {
    /// A main window holding `main` and a pen window opened on it.
    pub(super) fn open(mut main: Session) -> Self {
        main.link_start(LinkRole::Main);
        let mut pen = Session::new(0xB0);
        pen.resize_viewport(1200.0, 720.0);
        pen.link_start(LinkRole::Pen);
        let mut pair = Self {
            main,
            pen,
            next_pen_peer: 0xB1,
            assets_sent: (0, 0),
            bundles: (0, 0),
            drop_main_assets: false,
            views: Vec::new(),
        };
        pair.settle();
        assert_eq!(pair.pen.link_state(), LinkState::Live);
        pair
    }

    /// One frame in both windows: each exports and imports, then the hub carries the bundles;
    /// a project that arrived opens as a fresh replica (as the pen page does).
    pub(super) fn frame(&mut self) {
        let from_main = self.main.link_frame();
        let from_pen = self.pen.link_frame();
        if let Some(mut bytes) = from_main {
            if let Ok(Message::Bundle(mut bundle)) = decode(&bytes) {
                if self.drop_main_assets {
                    bundle.assets.clear();
                    bytes = encode(&bundle);
                }
                self.assets_sent.0 += bundle.assets.len();
                self.views.extend(views_of(&bundle).map(|v| (true, v)));
            }
            self.bundles.0 += 1;
            self.pen.link_receive(&bytes);
        }
        if let Some(bytes) = from_pen {
            if let Ok(Message::Bundle(bundle)) = decode(&bytes) {
                self.assets_sent.1 += bundle.assets.len();
                self.views.extend(views_of(&bundle).map(|v| (false, v)));
            }
            self.bundles.1 += 1;
            self.main.link_receive(&bytes);
        }
        self.take_project();
    }

    pub(super) fn take_project(&mut self) {
        let Some(project) = self.pen.link_take_project() else {
            return;
        };
        let mut fresh = Session::open(self.next_pen_peer, &project).unwrap();
        self.next_pen_peer += 1;
        fresh.link_continue(&mut self.pen);
        self.pen = fresh;
    }

    /// Two frames: what one window sent is imported by the other in its next frame.
    pub(super) fn arrive(&mut self) {
        self.frame();
        self.frame();
    }

    /// Enough frames for anything in flight to arrive and be answered.
    pub(super) fn settle(&mut self) {
        for _ in 0..4 {
            self.frame();
        }
    }

    pub(super) fn equal(&self) -> bool {
        self.main.document.export_json().unwrap() == self.pen.document.export_json().unwrap()
    }
}

fn main_with_rect() -> Session {
    let mut main = Session::new(MAIN_PEER);
    draw_rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    main
}

fn count(session: &Session) -> usize {
    session.objects().len()
}

/// Criterion 1 (core) and 22: the pen window opens on the main window's project with the Pencil
/// and its own peer id; the main window stays clean.
#[test]
fn the_pen_window_opens_on_the_same_project_with_the_pencil() {
    let mut main = main_with_rect();
    main.pack_for_save().unwrap();
    main.mark_saved();
    let pair = Pair::open(main);
    assert!(pair.equal());
    assert_eq!(pair.pen.tool(), Tool::Pencil);
    assert_ne!(pair.pen.peer, pair.main.peer);
    assert!(
        !pair.main.is_dirty(),
        "opening the pen window marks nothing unsaved"
    );
}

/// Criterion 2: a step in either window is in the other a frame later, also over 50 steps
/// alternating between both sides.
#[test]
fn steps_from_both_sides_arrive_a_frame_later_and_converge() {
    let mut pair = Pair::open(main_with_rect());
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert_eq!(
        count(&pair.main),
        2,
        "the main window has the pen's rectangle"
    );
    for i in 0..50 {
        let at = f64::from(i) * 3.0;
        if i % 2 == 0 {
            draw_rect(&mut pair.main, (at, 100.0), (at + 2.0, 102.0));
        } else {
            draw_rect(&mut pair.pen, (at, 120.0), (at + 2.0, 122.0));
        }
        pair.arrive();
    }
    pair.arrive();
    assert!(pair.equal());
    assert_eq!(count(&pair.pen), 52);
}

/// Criteria 2 and 8: a drag in progress in one window shows in the other only once it is
/// finished; a step of the other side made meanwhile arrives and the drag still lands.
#[test]
fn a_drag_in_progress_travels_when_it_ends_and_an_escape_sends_nothing() {
    let mut pair = Pair::open(main_with_rect());
    pair.pen.set_tool(Tool::Rectangle);
    // The tool change empties the selection the pen window opened with; that travels first.
    pair.settle();
    pair.pen.pointer_down(Point::new(70.0, 70.0), false);
    pair.pen.pointer_hover(Point::new(80.0, 80.0), false, false);
    draw_rect(&mut pair.main, (5.0, 90.0), (9.0, 95.0));
    let bundles = pair.bundles.1;
    pair.arrive();
    pair.arrive();
    assert_eq!(pair.bundles.1, bundles, "nothing of the drag went out");
    assert_eq!(count(&pair.main), 2);
    pair.pen.pointer_up(Point::new(80.0, 80.0), false, false);
    pair.arrive();
    assert_eq!(count(&pair.main), 3);
    assert!(pair.equal());
    pair.pen.pointer_down(Point::new(90.0, 90.0), false);
    pair.pen.pointer_hover(Point::new(99.0, 99.0), false, false);
    pair.pen.escape();
    pair.pen.pointer_up(Point::new(99.0, 99.0), false, false);
    pair.settle();
    assert_eq!(count(&pair.main), 3, "an escaped drag reaches nobody");
}

/// Criterion 4: the selection is shared a frame later; ids the other side no longer has are
/// dropped; a Delete empties both, and one undo in the deleting window restores both.
#[test]
fn the_selection_is_shared_and_a_delete_undoes_in_both() {
    let mut pair = Pair::open(main_with_rect());
    click(&mut pair.pen, (10.0, 20.0), false);
    let id = pair.pen.selection.ids().to_vec();
    assert_eq!(id.len(), 1);
    pair.arrive();
    assert_eq!(pair.main.selection.ids(), id.as_slice());
    pair.pen.delete_selected();
    pair.arrive();
    assert_eq!(count(&pair.main), 0);
    assert!(pair.main.selection.ids().is_empty());
    pair.pen.undo();
    pair.settle();
    assert_eq!(count(&pair.main), 1);
    assert_eq!(pair.pen.selection.ids(), id.as_slice());
    assert_eq!(pair.main.selection.ids(), id.as_slice());
    // An unknown id is dropped without an error.
    let mut bogus = pair.main.selection.ids().to_vec();
    let mut elsewhere = Session::new(0x77);
    bogus.push(draw_rect(&mut elsewhere, (1.0, 1.0), (2.0, 2.0)));
    pair.main.selection.set(&bogus);
    pair.settle();
    assert_eq!(pair.pen.selection.ids(), id.as_slice());
}

/// Criterion 4 (UX 9.4): while a value field of the receiving window is busy, the arriving
/// selection waits until it commits.
#[test]
fn an_arriving_selection_waits_for_a_busy_field() {
    let mut pair = Pair::open(main_with_rect());
    pair.main.selection.clear();
    pair.settle();
    assert!(pair.pen.selection.ids().is_empty());
    pair.main.link_set_field_busy(true);
    click(&mut pair.pen, (10.0, 20.0), false);
    pair.settle();
    assert!(
        pair.main.selection.ids().is_empty(),
        "held while the field is busy"
    );
    assert_eq!(pair.pen.selection.ids().len(), 1, "and not echoed back");
    pair.main.link_set_field_busy(false);
    assert_eq!(pair.main.selection.ids(), pair.pen.selection.ids());
}

/// Criterion 5: tool, view and zoom stay per window over 20 steps.
#[test]
fn tool_and_view_stay_per_window() {
    let mut pair = Pair::open(main_with_rect());
    pair.main.set_tool(Tool::Select);
    pair.main.zoom_step(true, 100.0, 100.0);
    let main_view = pair.main.saved_view_text();
    pair.pen.zoom_step(false, 300.0, 300.0);
    let pen_view = pair.pen.saved_view_text();
    for i in 0..20 {
        let at = f64::from(i) * 4.0;
        let side = if i % 2 == 0 {
            &mut pair.main
        } else {
            &mut pair.pen
        };
        draw_rect(side, (at, 150.0), (at + 2.0, 152.0));
        side.set_tool(if i % 2 == 0 {
            Tool::Select
        } else {
            Tool::Pencil
        });
        pair.arrive();
    }
    assert_eq!(pair.main.tool(), Tool::Select);
    assert_eq!(pair.pen.tool(), Tool::Pencil);
    assert_eq!(pair.main.saved_view_text(), main_view);
    assert_eq!(pair.pen.saved_view_text(), pen_view);
    assert_ne!(main_view, pen_view);
}

/// Criterion 6: undo is per window, remapped over the other's later steps; redo likewise; the
/// main window's History names the pen window's steps "Pen window".
#[test]
fn undo_in_a_window_undoes_its_own_step_only() {
    let mut pair = Pair::open(Session::new(MAIN_PEER));
    draw_rect(&mut pair.main, (10.0, 10.0), (20.0, 20.0));
    pair.arrive();
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert!(pair.main.undo().changed);
    pair.settle();
    assert_eq!(count(&pair.pen), 1, "A's rectangle gone, B's kept in both");
    assert!(pair.equal());
    assert!(pair.main.redo().changed);
    pair.settle();
    assert_eq!(count(&pair.pen), 2);
    assert!(pair.pen.undo().changed);
    pair.settle();
    assert_eq!(count(&pair.main), 1);
    assert!(pair.equal());
    let rows: serde_json::Value = serde_json::from_str(&pair.main.history_rows_json(0, 5)).unwrap();
    let pen_rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["peerLabel"] == "Pen window")
        .count();
    assert!(pen_rows >= 1, "{rows}");
}

/// Criterion 9: a pen step marks the main project unsaved; the pen's Save asks the main window.
#[test]
fn a_pen_step_marks_main_unsaved_and_save_goes_to_main() {
    let mut main = main_with_rect();
    main.pack_for_save().unwrap();
    main.mark_saved();
    let mut pair = Pair::open(main);
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert!(pair.main.is_dirty());
    assert!(!pair.main.link_take_save());
    pair.pen.link_request_save();
    pair.arrive();
    assert!(pair.main.link_take_save());
    assert!(!pair.main.link_take_save(), "once");
}

/// Criterion 12: New in the main window sends the new project; the pen window keeps its tool,
/// starts with an empty selection and undo, and the old project's steps never reach the new one.
#[test]
fn a_new_project_in_main_replaces_the_pen_project() {
    let mut pair = Pair::open(main_with_rect());
    click(&mut pair.main, (10.0, 20.0), false);
    pair.settle();
    // A pen step of the old project, not yet delivered when main opens the new one.
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.pen.set_tool(Tool::Node);
    let stale = pair.pen.link_frame();
    let mut fresh = Session::new(0xA2);
    fresh.link_continue(&mut pair.main);
    pair.main = fresh;
    if let Some(stale) = stale {
        pair.main.link_receive(&stale);
    }
    pair.settle();
    assert_eq!(
        count(&pair.main),
        0,
        "the old step never reaches the new project"
    );
    assert!(pair.equal());
    assert_eq!(pair.pen.tool(), Tool::Node);
    assert!(pair.pen.selection.ids().is_empty());
    assert_eq!(pair.pen.undo_len(), 0);
}

/// Criterion 13: while the main window holds, the pen window cancels its stroke, makes no step
/// and shows the waiting state; Release ends it.
#[test]
fn a_hold_cancels_the_gesture_and_blocks_input() {
    let mut pair = Pair::open(main_with_rect());
    pair.pen.set_tool(Tool::Rectangle);
    // The tool change empties the selection the pen window opened with; that travels first.
    pair.settle();
    pair.pen.pointer_down(Point::new(70.0, 70.0), false);
    pair.pen.pointer_hover(Point::new(80.0, 80.0), false, false);
    let hold = pair.main.link_hold(true).unwrap();
    pair.pen.link_receive(&hold);
    pair.arrive();
    assert_eq!(pair.pen.link_state(), LinkState::Held);
    pair.pen.pointer_up(Point::new(80.0, 80.0), false, false);
    draw_rect(&mut pair.pen, (90.0, 90.0), (95.0, 95.0));
    pair.settle();
    assert_eq!(count(&pair.main), 1, "nothing was made while held");
    let release = pair.main.link_hold(false).unwrap();
    pair.pen.link_receive(&release);
    pair.arrive();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    draw_rect(&mut pair.pen, (90.0, 90.0), (95.0, 95.0));
    pair.arrive();
    assert_eq!(count(&pair.main), 2);
}

/// Criterion 14: both windows change the same object before either import: different
/// properties both apply, a move against a delete ends deleted, both documents equal.
#[test]
fn concurrent_edits_of_one_object_converge() {
    let mut pair = Pair::open(main_with_rect());
    click(&mut pair.main, (10.0, 20.0), false);
    click(&mut pair.pen, (10.0, 20.0), false);
    pair.main.key_down(super::KeyInput {
        key: "ArrowRight",
        ..super::KeyInput::default()
    });
    pair.pen.delete_selected();
    pair.settle();
    assert!(pair.equal());
    assert_eq!(count(&pair.main), 0, "a move and a delete end deleted");
    let undone = pair.main.undo();
    pair.settle();
    assert!(pair.equal(), "{undone:?}");
}

/// Criterion 15: a text under edit survives the other window's drawing; when the other window
/// deletes it, edit mode ends with a notice.
#[test]
fn a_text_edit_survives_imports_and_ends_when_its_text_is_deleted() {
    let main = super::text_tests::with_text("Name");
    let mut pair = Pair::open(main);
    pair.main.set_tool(Tool::Text);
    pair.main.pointer_down(Point::new(11.0, 20.0), false);
    pair.main.pointer_up(Point::new(11.0, 20.0), false, false);
    assert!(pair.main.text_editing());
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.settle();
    assert!(pair.main.text_editing(), "drawing elsewhere keeps the edit");
    pair.pen.set_tool(Tool::Select);
    pair.pen.pointer_down(Point::new(11.0, 20.0), false);
    pair.pen.pointer_up(Point::new(11.0, 20.0), false, false);
    pair.settle();
    assert!(
        pair.main.text_editing(),
        "a selection from the pen changes nothing in the edit"
    );
    pair.pen.delete_selected();
    pair.settle();
    assert!(!pair.main.text_editing());
    assert_eq!(
        pair.main.link_take_notice().as_deref(),
        Some(super::window_link::TEXT_DELETED)
    );
}

/// Criterion 18: an update that fails the checks reloads the pen window from the main window;
/// tool and view stay, selection and undo start empty, one notice; nothing is written.
#[test]
fn a_refused_update_reloads_the_pen_window() {
    let mut main = main_with_rect();
    main.pack_for_save().unwrap();
    main.mark_saved();
    let mut pair = Pair::open(main);
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    pair.pen.set_tool(Tool::Node);
    pair.pen.zoom_step(true, 200.0, 200.0);
    let view = pair.pen.saved_view_text();
    pair.pen.link_receive(b"\x01\x01garbage");
    pair.arrive();
    assert_eq!(pair.pen.link_state(), LinkState::OutOfSync);
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    assert!(pair.equal());
    assert_eq!(pair.pen.tool(), Tool::Node);
    assert_eq!(pair.pen.saved_view_text(), view);
    assert!(pair.pen.selection.ids().is_empty());
    assert_eq!(pair.pen.undo_len(), 0);
    assert_eq!(
        pair.pen.link_take_notice().as_deref(),
        Some(super::window_link::RELOADED)
    );
    draw_rect(&mut pair.pen, (70.0, 70.0), (80.0, 80.0));
    pair.arrive();
    assert_eq!(count(&pair.main), 3, "the reloaded replica goes on linking");
}

/// Criterion 19: without a pen window the main window does no link work; the hub's notice
/// that the pen window is gone ends the link.
#[test]
fn without_a_pen_window_there_is_no_link_work() {
    let mut main = main_with_rect();
    assert_eq!(main.link_frame(), None);
    let mut pair = Pair::open(main);
    pair.main
        .link_receive(&curvyo_ui_core::window_bridge::PEER_GONE);
    assert_eq!(pair.main.link_state(), LinkState::None);
    draw_rect(&mut pair.main, (50.0, 50.0), (60.0, 60.0));
    assert_eq!(pair.main.link_frame(), None);
}

/// Criterion 22 (core): Fit page shows the active page whole; the zoom steps by 1.414.
#[test]
fn fit_page_and_the_zoom_steps() {
    let mut pair = Pair::open(main_with_rect());
    pair.pen.view_fit_page(80.0, 60.0);
    let page = pair.pen.active_page();
    let view = pair.pen.view();
    let (left, top) = view.document_to_screen(page.origin);
    let (right, bottom) = view.document_to_screen(Point::new(
        page.origin.x + page.size.width.as_mm(),
        page.origin.y + page.size.height.as_mm(),
    ));
    assert!(left >= 80.0 - 1e-6 && top >= 60.0 - 1e-6);
    assert!(right <= 1200.0 - 24.0 + 1e-6 && bottom <= 720.0 - 24.0 + 1e-6);
    let touches = [left - 80.0, right - 1176.0, top - 60.0, bottom - 696.0]
        .iter()
        .any(|gap| gap.abs() < 1e-6);
    assert!(touches, "as large as it fits");
    let before = pair.pen.view().scale();
    pair.pen.zoom_step(true, 600.0, 360.0);
    assert!((pair.pen.view().scale() / before - 1.414).abs() < 1e-9);
}
