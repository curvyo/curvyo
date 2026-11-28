//! Acceptance tests for `specs/0158-pen-display` slice 2a (Pen window), criteria 1 to 26 as far as
//! they reach session level: two `Session`s and a byte shuttle in one process, no window. Written
//! from the specification before the implementation was read.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::type_complexity
)]

use curvyo_document_core::{
    Direction, Document, NodeId, ObjectSnapshot, PathSnapshot, Point, unpack,
};
use curvyo_editor_wasm::{LinkRole, LinkState, Session, Tool};
use curvyo_ui_core::pen_window_place::{
    DEFAULT_LOGICAL_SIZE, OpeningSize, PenWindowFlags, PenWindowPlace, ScreenRect, opening,
    read_pen_window, write_pen_window,
};
use curvyo_ui_core::window_bridge::{Bundle, Control, Message, PEER_GONE, Updates, decode, encode};

const PNG: &[u8] = include_bytes!("../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

// ---------------------------------------------------------------- rig

struct Pair {
    main: Session,
    pen: Session,
    next_peer: u64,
    assets: (usize, usize),
    strip_main_assets: bool,
}

impl Pair {
    fn open(mut main: Session) -> Self {
        main.link_start(LinkRole::Main);
        let mut pen = Session::new(0xB0);
        pen.resize_viewport(1200.0, 720.0);
        pen.link_start(LinkRole::Pen);
        let mut pair = Self {
            main,
            pen,
            next_peer: 0xB1,
            assets: (0, 0),
            strip_main_assets: false,
        };
        pair.settle();
        assert_eq!(pair.pen.link_state(), LinkState::Live);
        pair
    }
    fn frame(&mut self) {
        let from_main = self.main.link_frame();
        let from_pen = self.pen.link_frame();
        if let Some(mut bytes) = from_main {
            if let Ok(Message::Bundle(mut b)) = decode(&bytes) {
                if self.strip_main_assets && !b.assets.is_empty() {
                    self.strip_main_assets = false;
                    b.assets.clear();
                    bytes = encode(&b);
                }
                self.assets.0 += b.assets.len();
            }
            self.pen.link_receive(&bytes);
        }
        if let Some(bytes) = from_pen {
            if let Ok(Message::Bundle(b)) = decode(&bytes) {
                self.assets.1 += b.assets.len();
            }
            self.main.link_receive(&bytes);
        }
        if let Some(project) = self.pen.link_take_project() {
            let mut fresh = Session::open(self.next_peer, &project).unwrap();
            self.next_peer += 1;
            fresh.link_continue(&mut self.pen);
            self.pen = fresh;
        }
    }
    fn arrive(&mut self) {
        self.frame();
        self.frame();
    }
    fn settle(&mut self) {
        for _ in 0..6 {
            self.frame();
        }
    }
    fn docs(&self) -> (Document, Document) {
        (doc_of(&self.main), doc_of(&self.pen))
    }
    fn equal(&self) -> bool {
        let (a, b) = self.docs();
        a.export_json().unwrap() == b.export_json().unwrap()
    }
}

fn doc_of(s: &Session) -> Document {
    unpack(7, &s.pack("t").unwrap()).unwrap()
}

fn count(s: &Session) -> usize {
    doc_of(s).object_ids().len()
}

fn ids(s: &Session) -> Vec<NodeId> {
    doc_of(s).object_ids()
}

fn rect(s: &mut Session, from: (f64, f64), to: (f64, f64)) -> NodeId {
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(from.0, from.1), false);
    s.pointer_up(Point::new(to.0, to.1), false, false);
    ids(s).last().copied().unwrap_or(NodeId::NONE)
}

fn click(s: &mut Session, at: (f64, f64)) {
    s.set_tool(Tool::Select);
    let p = Point::new(at.0, at.1);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

/// Selects with a click, then drags from the same point by `by`.
fn move_by(s: &mut Session, at: (f64, f64), by: (f64, f64)) {
    click(s, at);
    let p = Point::new(at.0, at.1);
    s.pointer_down(p, false);
    s.pointer_up(Point::new(at.0 + by.0, at.1 + by.1), false, false);
}

fn paths(s: &Session) -> Vec<PathSnapshot> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.path(id))
        .collect()
}

fn widths_of(s: &Session) -> Vec<Vec<(f64, f64)>> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter(|id| d.path(*id).is_some())
        .map(|id| {
            d.width_points(id)
                .unwrap()
                .iter()
                .map(|w| (w.position, w.multiplier))
                .collect()
        })
        .collect()
}

fn images(s: &Session) -> usize {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter(|id| matches!(d.object(*id), Some(ObjectSnapshot::Image(_))))
        .count()
}

fn import_png(s: &mut Session) {
    s.import_begin(None);
    s.import_add(PNG, "/home/maker/photo.png");
    assert_eq!(s.import_finish(false).imported, 1);
}

fn record(s: &Session, at: Point, p: f64, eraser: bool, phase: f64, t: f64) -> Vec<f64> {
    let o = s.screen_to_document(0.0, 0.0);
    let u = s.screen_to_document(1.0, 0.0);
    let ppm = 1.0 / (u.x - o.x);
    vec![
        (at.x - o.x) * ppm,
        (at.y - o.y) * ppm,
        p,
        f64::NAN,
        f64::NAN,
        if eraser { 1.0 } else { 0.0 },
        phase,
        0.0,
        t,
    ]
}

/// A pressure stroke of `n` samples; `lift` ends it.
fn stroke(s: &mut Session, from: Point, n: usize, lift: bool) {
    s.set_tool(Tool::Pencil);
    let _ = s.draw_list();
    let mut flat = Vec::new();
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let at = Point::new(from.x + 80.0 * t, from.y + 6.0 * (t * 3.0).sin());
        flat.extend(record(s, at, 0.1 + 0.8 * t, false, 1.0, i as f64 * 4.0));
    }
    if lift {
        flat.extend(record(
            s,
            Point::new(from.x + 81.0, from.y),
            0.0,
            false,
            0.0,
            400.0,
        ));
    }
    s.pen_samples(&flat);
}

fn eraser_across(s: &mut Session, x: f64, y0: f64, y1: f64) {
    s.set_tool(Tool::Pencil);
    let mut flat = Vec::new();
    for i in 0..20 {
        let t = f64::from(i) / 19.0;
        flat.extend(record(
            s,
            Point::new(x, y0 + (y1 - y0) * t),
            0.5,
            true,
            1.0,
            f64::from(i) * 4.0,
        ));
    }
    flat.extend(record(s, Point::new(x, y1), 0.0, true, 0.0, 200.0));
    s.pen_samples(&flat);
}

// ---------------------------------------------------------------- criteria 1, 19, 22

#[test]
fn c01_c22_the_pen_window_opens_on_the_same_project_with_the_pencil_and_main_stays_clean() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    main.pack_for_save().unwrap();
    main.mark_saved();
    let pair = Pair::open(main);
    assert!(pair.equal());
    assert_eq!(pair.pen.tool(), Tool::Pencil);
    assert!(!pair.main.is_dirty());
    assert!(pair.pen.link_is_pen());
    assert_eq!(pair.main.link_state(), LinkState::Live);
}

#[test]
fn c01_the_pen_window_waits_until_its_project_arrives() {
    let mut main = Session::new(0xA1);
    main.link_start(LinkRole::Main);
    let mut pen = Session::new(0xB0);
    pen.link_start(LinkRole::Pen);
    assert_eq!(pen.link_state(), LinkState::Waiting);
    // Drawing while waiting makes nothing.
    stroke(&mut pen, Point::new(20.0, 40.0), 30, true);
    assert_eq!(count(&pen), 0);
}

#[test]
fn c19_without_a_pen_window_there_is_no_link_work() {
    let mut s = Session::new(1);
    assert_eq!(s.link_state(), LinkState::None);
    assert!(!s.has_window_link());
    rect(&mut s, (10.0, 10.0), (20.0, 20.0));
    assert!(s.link_frame().is_none());
    assert!(s.link_hold(true).is_none());
    assert!(s.link_take_project().is_none());
    assert!(!s.link_take_save() && !s.link_take_changed() && !s.link_take_redraw());
    assert!(s.link_take_notice().is_none());
    // Bytes without a link are ignored.
    s.link_receive(&[1, 1, 2, 3]);
    assert!(s.link_frame().is_none());
}

#[test]
fn c19_when_the_pen_window_goes_the_main_window_stops_link_work() {
    let mut pair = Pair::open(Session::new(0xA1));
    rect(&mut pair.main, (10.0, 10.0), (20.0, 20.0));
    assert!(pair.main.link_frame().is_some());
    pair.main.link_receive(&PEER_GONE);
    assert_eq!(pair.main.link_state(), LinkState::None);
    rect(&mut pair.main, (30.0, 30.0), (40.0, 40.0));
    assert!(pair.main.link_frame().is_none());
    assert!(pair.main.link_hold(true).is_none());
    // The main window is unchanged and keeps working.
    assert_eq!(count(&pair.main), 2);
}

#[test]
fn c19_an_idle_link_sends_nothing() {
    let mut pair = Pair::open(Session::new(0xA1));
    for _ in 0..10 {
        assert!(pair.main.link_frame().is_none());
        assert!(pair.pen.link_frame().is_none());
    }
}

// ---------------------------------------------------------------- criterion 2, 8, 14

#[test]
fn c02_steps_arrive_a_frame_later_and_fifty_alternating_steps_converge() {
    let mut pair = Pair::open(Session::new(0xA1));
    rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert_eq!(count(&pair.main), 1);
    for i in 0..50 {
        let f = f64::from(i);
        if i % 2 == 0 {
            rect(&mut pair.main, (f, 100.0), (f + 3.0, 104.0));
        } else {
            rect(&mut pair.pen, (f, 120.0), (f + 3.0, 124.0));
        }
        pair.arrive();
        assert!(pair.equal(), "step {i}");
    }
    assert_eq!(count(&pair.main), 51);
}

#[test]
fn c02_a_step_made_while_the_other_side_is_mid_drag_arrives_and_the_drag_finishes() {
    let mut pair = Pair::open(Session::new(0xA1));
    pair.main.set_tool(Tool::Rectangle);
    pair.main.pointer_down(Point::new(10.0, 10.0), false);
    rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert_eq!(count(&pair.main), 1, "the pen's rectangle arrived mid-drag");
    // Criterion 8: nothing of the main drag is in the pen window yet.
    assert_eq!(count(&pair.pen), 1);
    pair.main.pointer_up(Point::new(30.0, 30.0), false, false);
    pair.arrive();
    assert_eq!(count(&pair.main), 2);
    assert!(pair.equal());
}

#[test]
fn c08_a_stroke_in_progress_and_an_escaped_one_reach_nobody() {
    let mut pair = Pair::open(Session::new(0xA1));
    stroke(&mut pair.pen, Point::new(20.0, 40.0), 40, false);
    pair.arrive();
    assert_eq!(count(&pair.main), 0, "not before the step finishes");
    pair.pen.escape();
    pair.arrive();
    assert_eq!(count(&pair.main), 0);
    assert_eq!(count(&pair.pen), 0);
    assert_eq!(pair.main.undo_len(), 0);
    assert!(pair.equal());
}

#[test]
fn c14_both_move_the_same_object_the_documents_end_equal() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    let mut pair = Pair::open(main);
    move_by(&mut pair.main, (10.0, 20.0), (30.0, 0.0));
    move_by(&mut pair.pen, (10.0, 20.0), (0.0, 40.0));
    pair.settle();
    assert!(pair.equal());
    assert_eq!(count(&pair.main), 1);
}

#[test]
fn c14_a_move_in_one_and_a_delete_in_the_other_ends_deleted_in_both() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    let mut pair = Pair::open(main);
    move_by(&mut pair.main, (10.0, 20.0), (30.0, 0.0));
    click(&mut pair.pen, (10.0, 20.0));
    pair.pen.delete_selected();
    pair.settle();
    assert!(pair.equal());
    assert_eq!(count(&pair.main), 0);
    assert_eq!(count(&pair.pen), 0);
}

#[test]
fn c14_undo_of_a_step_on_an_object_the_other_side_deleted_does_nothing_without_error() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    let mut pair = Pair::open(main);
    move_by(&mut pair.main, (10.0, 20.0), (30.0, 0.0));
    pair.arrive();
    click(&mut pair.pen, (40.0, 20.0));
    pair.pen.delete_selected();
    pair.arrive();
    assert_eq!(count(&pair.main), 0);
    let _ = pair.main.undo();
    let _ = pair.main.undo();
    pair.settle();
    assert!(pair.equal());
    assert_eq!(pair.main.link_state(), LinkState::Live);
    assert_eq!(pair.pen.link_state(), LinkState::Live);
}

#[test]
fn c14_two_concurrent_strokes_both_survive() {
    let mut pair = Pair::open(Session::new(0xA1));
    stroke(&mut pair.main, Point::new(20.0, 40.0), 50, true);
    stroke(&mut pair.pen, Point::new(20.0, 90.0), 50, true);
    pair.settle();
    assert_eq!(count(&pair.main), 2);
    assert!(pair.equal());
}

// ---------------------------------------------------------------- criteria 7, 24

fn same_path(a: &PathSnapshot, b: &PathSnapshot) {
    assert_eq!(a.anchors.len(), b.anchors.len());
    for (x, y) in a.anchors.iter().zip(&b.anchors) {
        assert_eq!(x.point, y.point);
        assert_eq!(x.handle_in, y.handle_in);
        assert_eq!(x.handle_out, y.handle_out);
    }
    assert_eq!(a.closed, b.closed);
}

#[test]
fn c07_c24_a_pen_pressure_stroke_crosses_identically_one_step_selected_main_not_receiving() {
    let mut pair = Pair::open(Session::new(0xA1));
    stroke(&mut pair.pen, Point::new(20.0, 50.0), 80, true);
    assert_eq!(pair.pen.undo_len(), 1);
    assert_eq!(
        pair.pen.history_target_name(Direction::Undo).as_deref(),
        Some("Pencil stroke")
    );
    assert_eq!(pair.pen.selected_object_count(), 1);
    pair.arrive();
    let (pm, pp) = (paths(&pair.main), paths(&pair.pen));
    assert_eq!((pm.len(), pp.len()), (1, 1));
    same_path(&pm[0], &pp[0]);
    let (wm, wp) = (widths_of(&pair.main), widths_of(&pair.pen));
    assert!(!wm[0].is_empty(), "the profile crossed");
    assert_eq!(wm, wp, "identical width profile");
    assert_eq!(
        pair.main.undo_len(),
        0,
        "the main window's Pencil did not receive it"
    );
    assert_eq!(pair.main.tool(), Tool::Select);
    assert!(pair.equal());
}

#[test]
fn c24_a_main_stroke_crosses_to_the_pen_and_both_survive_a_reload() {
    let mut pair = Pair::open(Session::new(0xA1));
    stroke(&mut pair.main, Point::new(20.0, 50.0), 80, true);
    stroke(&mut pair.pen, Point::new(20.0, 90.0), 80, true);
    pair.settle();
    assert_eq!(widths_of(&pair.main), widths_of(&pair.pen));
    let before_paths = paths(&pair.main);
    let before_widths = widths_of(&pair.main);
    // A reload by a refused import: the main window's project message.
    pair.pen.link_receive(&[1, 1, 0xff, 0xff, 0xff]);
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    assert_eq!(widths_of(&pair.pen), before_widths);
    for (a, b) in before_paths.iter().zip(paths(&pair.pen).iter()) {
        same_path(a, b);
    }
    // A fresh pen window opened on the project later sees the same.
    let main = std::mem::replace(&mut pair.main, Session::new(1));
    let again = Pair::open(main);
    assert_eq!(widths_of(&again.pen), before_widths);
    for (a, b) in before_paths.iter().zip(paths(&again.pen).iter()) {
        same_path(a, b);
    }
}

#[test]
fn c07_the_pens_eraser_end_in_the_pen_window_reaches_the_main_window() {
    let mut pair = Pair::open(Session::new(0xA1));
    stroke(&mut pair.pen, Point::new(20.0, 50.0), 60, true);
    pair.arrive();
    assert_eq!(count(&pair.main), 1);
    eraser_across(&mut pair.pen, 40.0, 40.0, 62.0);
    pair.arrive();
    assert_eq!(count(&pair.pen), 0, "erased in the pen window");
    assert_eq!(count(&pair.main), 0, "and in the main window");
    assert!(pair.equal());
}

// ---------------------------------------------------------------- criterion 3, pictures

#[test]
fn c03_a_picture_travels_once_and_a_known_one_not_at_all() {
    let mut pair = Pair::open(Session::new(0xA1));
    import_png(&mut pair.main);
    pair.arrive();
    assert_eq!(images(&pair.pen), 1);
    assert_eq!(pair.assets.0, 1, "the asset travelled once");
    import_png(&mut pair.main);
    pair.arrive();
    assert_eq!(images(&pair.pen), 2);
    assert_eq!(pair.assets.0, 1, "the same picture again: no asset message");
    import_png(&mut pair.pen);
    pair.arrive();
    assert_eq!(images(&pair.main), 3);
    assert_eq!(
        pair.assets.1, 0,
        "a picture both sides hold is not sent back"
    );
    assert!(pair.equal());
}

#[test]
fn c03_a_picture_from_the_pen_window_travels_to_main() {
    let mut pair = Pair::open(Session::new(0xA1));
    import_png(&mut pair.pen);
    pair.arrive();
    assert_eq!(images(&pair.main), 1);
    assert_eq!(pair.assets.1, 1);
    assert!(pair.equal());
}

#[test]
fn c03_an_update_waits_for_a_missing_asset_and_the_document_stays_consistent() {
    let mut pair = Pair::open(Session::new(0xA1));
    pair.strip_main_assets = true;
    import_png(&mut pair.main);
    pair.frame();
    pair.frame();
    assert_eq!(
        images(&pair.pen),
        0,
        "no empty picture while the asset is missing"
    );
    assert_eq!(
        pair.pen.link_state(),
        LinkState::Live,
        "waiting is not a refusal"
    );
    pair.settle();
    assert_eq!(images(&pair.pen), 1);
    assert!(pair.equal());
}

#[test]
fn c03_a_later_step_waits_behind_the_waiting_update_in_order() {
    let mut pair = Pair::open(Session::new(0xA1));
    pair.strip_main_assets = true;
    import_png(&mut pair.main);
    pair.frame();
    rect(&mut pair.main, (10.0, 10.0), (20.0, 20.0));
    pair.frame();
    assert_eq!(count(&pair.pen), 0, "nothing applied out of order");
    pair.settle();
    assert_eq!(count(&pair.pen), 2);
    assert!(pair.equal());
}

// ---------------------------------------------------------------- criterion 4, selection

fn two_rects() -> Session {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    rect(&mut main, (60.0, 10.0), (90.0, 30.0));
    main
}

#[test]
fn c04_a_selection_is_the_others_a_frame_later() {
    let mut pair = Pair::open(two_rects());
    click(&mut pair.pen, (10.0, 20.0));
    assert_eq!(pair.pen.selected_object_count(), 1);
    pair.arrive();
    assert_eq!(pair.main.selected_object_count(), 1);
    click(&mut pair.main, (60.0, 20.0));
    pair.arrive();
    assert_eq!(pair.pen.selected_object_count(), 1);
    let sel = |s: &Session| s.selection_ids_for_test();
    let _ = sel;
}

trait SelIds {
    fn selection_ids_for_test(&self) -> usize;
}
impl SelIds for Session {
    fn selection_ids_for_test(&self) -> usize {
        self.selected_object_count()
    }
}

#[test]
fn c04_a_selection_of_unknown_ids_is_dropped_without_error() {
    let mut pair = Pair::open(Session::new(0xA1));
    let mut other = Session::new(0xC3);
    let ghost = rect(&mut other, (1.0, 1.0), (5.0, 5.0));
    rect(&mut pair.pen, (3.0, 3.0), (6.0, 6.0));
    let real = pair.pen.link_frame().unwrap();
    let mut genuine = match decode(&real).unwrap() {
        Message::Bundle(bundle) => bundle,
        Message::PeerGone => panic!(),
    };
    genuine.selection = Some(vec![ghost]);
    pair.main.link_receive(&encode(&genuine));
    pair.settle();
    assert_eq!(pair.main.selected_object_count(), 0);
    assert_eq!(pair.main.link_state(), LinkState::Live);
    assert_eq!(count(&pair.main), 1);
}

#[test]
fn c04_delete_in_one_window_clears_the_others_selection_and_one_undo_restores_both() {
    let mut pair = Pair::open(two_rects());
    click(&mut pair.pen, (10.0, 20.0));
    pair.arrive();
    assert_eq!(pair.main.selected_object_count(), 1);
    pair.pen.delete_selected();
    pair.arrive();
    assert_eq!(count(&pair.main), 1);
    assert_eq!(pair.main.selected_object_count(), 0);
    assert_eq!(pair.pen.selected_object_count(), 0);
    let _ = pair.pen.undo();
    pair.arrive();
    assert_eq!(count(&pair.main), 2);
    assert_eq!(count(&pair.pen), 2);
    assert_eq!(pair.pen.selected_object_count(), 1);
    assert_eq!(
        pair.main.selected_object_count(),
        1,
        "the selection is back in both"
    );
    assert!(pair.equal());
}

#[test]
fn c04_an_arriving_selection_waits_for_a_busy_field_and_lands_when_it_commits() {
    let mut pair = Pair::open(two_rects());
    click(&mut pair.main, (10.0, 20.0));
    pair.arrive();
    click(&mut pair.main, (60.0, 20.0));
    pair.arrive();
    assert_eq!(pair.pen.selected_object_count(), 1);
    let (id1, id2) = (ids(&pair.main)[0], ids(&pair.main)[1]);
    pair.pen.link_set_field_busy(true);
    click(&mut pair.main, (10.0, 20.0));
    pair.arrive();
    pair.arrive();
    // The pen window's selection is still the rectangle it had: a Delete there removes that
    // one, never the one the main window selected meanwhile.
    pair.pen.delete_selected();
    assert_eq!(
        ids(&pair.pen),
        vec![id1],
        "the typed value would have hit {id2:?}"
    );
    pair.pen.link_set_field_busy(false);
    pair.settle();
    assert_eq!(
        pair.pen.selected_object_count(),
        1,
        "the waiting selection landed"
    );
    assert!(pair.equal());
}

#[test]
fn c04_a_busy_field_holds_the_selection_back_even_if_it_was_cleared_on_the_other_side() {
    let mut pair = Pair::open(two_rects());
    click(&mut pair.main, (10.0, 20.0));
    pair.arrive();
    assert_eq!(pair.pen.selected_object_count(), 1);
    pair.pen.link_set_field_busy(true);
    pair.main.escape();
    pair.main.escape();
    pair.arrive();
    assert_eq!(pair.pen.selected_object_count(), 1, "waits");
    pair.pen.link_set_field_busy(false);
    assert_eq!(pair.pen.selected_object_count(), 0, "then lands");
}

// ---------------------------------------------------------------- criterion 5, 6

#[test]
fn c05_tool_and_view_are_per_window() {
    let mut pair = Pair::open(two_rects());
    pair.pen.set_tool(Tool::Pencil);
    pair.main.set_tool(Tool::Select);
    let zoom0 = pair.main.zoom_percent();
    for i in 0..20 {
        rect(
            &mut pair.main,
            (f64::from(i), 100.0),
            (f64::from(i) + 2.0, 103.0),
        );
        pair.main.set_tool(Tool::Select);
        pair.pen.zoom_step(true, 300.0, 300.0);
        pair.arrive();
    }
    assert_eq!(pair.pen.tool(), Tool::Pencil);
    assert_eq!(pair.main.tool(), Tool::Select);
    assert_eq!(pair.main.zoom_percent(), zoom0);
    assert!(pair.pen.zoom_percent() > zoom0);
}

#[test]
fn c06_undo_is_per_window_and_a_window_never_undoes_the_others_step() {
    let mut pair = Pair::open(Session::new(0xA1));
    rect(&mut pair.main, (10.0, 10.0), (20.0, 20.0));
    pair.arrive();
    rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    assert_eq!(pair.main.undo_len(), 1);
    assert_eq!(pair.pen.undo_len(), 1);
    let _ = pair.main.undo();
    pair.arrive();
    assert_eq!(count(&pair.main), 1, "only A's rectangle went");
    assert_eq!(count(&pair.pen), 1);
    assert!(pair.equal());
    // The pen's own object is the one left.
    let left = ids(&pair.main)[0];
    assert_eq!(left, ids(&pair.pen)[0]);
    let _ = pair.pen.undo();
    pair.arrive();
    assert_eq!(count(&pair.main), 0);
    let _ = pair.main.redo();
    pair.arrive();
    assert_eq!(count(&pair.pen), 1);
    assert!(pair.equal());
}

#[test]
fn c06_undo_remaps_over_the_others_later_steps_and_main_names_pen_steps() {
    let mut pair = Pair::open(Session::new(0xA1));
    rect(&mut pair.main, (10.0, 10.0), (20.0, 20.0));
    pair.arrive();
    stroke(&mut pair.pen, Point::new(20.0, 70.0), 40, true);
    pair.arrive();
    rect(&mut pair.main, (30.0, 10.0), (40.0, 20.0));
    pair.arrive();
    let _ = pair.pen.undo();
    pair.arrive();
    assert_eq!(
        count(&pair.main),
        2,
        "the pen's stroke went, both main rectangles stay"
    );
    assert_eq!(
        paths(&pair.main).len(),
        0,
        "rectangles are shapes, not paths"
    );
    assert!(pair.equal());
    let rows = pair.main.history_rows_json(0, 10);
    assert!(rows.contains("Pen window"), "{rows}");
}

// ---------------------------------------------------------------- criteria 9, 13, 18, 12

#[test]
fn c09_a_pen_step_marks_main_unsaved_and_ctrl_s_asks_main_to_save() {
    let mut main = Session::new(0xA1);
    main.pack_for_save().unwrap();
    main.mark_saved();
    let mut pair = Pair::open(main);
    assert!(!pair.main.is_dirty());
    rect(&mut pair.pen, (10.0, 10.0), (20.0, 20.0));
    pair.arrive();
    assert!(pair.main.is_dirty(), "marked unsaved as for a local step");
    pair.pen.link_request_save();
    pair.arrive();
    assert!(pair.main.link_take_save());
    assert!(!pair.main.link_take_save(), "once");
}

#[test]
fn c13_hold_makes_the_pen_window_ignore_input_and_release_gives_it_back() {
    let mut pair = Pair::open(Session::new(0xA1));
    let held = pair.main.link_hold(true).unwrap();
    pair.pen.link_receive(&held);
    pair.pen.link_frame();
    assert_eq!(pair.pen.link_state(), LinkState::Held);
    rect(&mut pair.pen, (10.0, 10.0), (20.0, 20.0));
    stroke(&mut pair.pen, Point::new(20.0, 50.0), 40, true);
    assert_eq!(count(&pair.pen), 0, "input ignored while held");
    pair.arrive();
    assert_eq!(count(&pair.main), 0);
    let released = pair.main.link_hold(false).unwrap();
    pair.pen.link_receive(&released);
    pair.pen.link_frame();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    stroke(&mut pair.pen, Point::new(20.0, 50.0), 40, true);
    assert_eq!(count(&pair.pen), 1);
    pair.arrive();
    assert_eq!(count(&pair.main), 1);
}

#[test]
fn c13_the_pen_window_cannot_hold_the_main_window() {
    let mut pair = Pair::open(Session::new(0xA1));
    assert!(pair.pen.link_hold(true).is_none());
    // A Hold control arriving at the main window changes nothing.
    let b = Bundle {
        generation: 1,
        controls: vec![Control::Hold],
        ..Bundle::default()
    };
    pair.main.link_receive(&encode(&b));
    pair.frame();
    assert_eq!(pair.main.link_state(), LinkState::Live);
}

#[test]
fn c18_a_refused_update_reloads_the_pen_window_and_main_stays_unchanged() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    main.pack_for_save().unwrap();
    main.mark_saved();
    let mut pair = Pair::open(main);
    pair.pen.set_tool(Tool::Node);
    rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.pen.set_tool(Tool::Node);
    let real = pair.pen.link_frame().unwrap();
    let mut bundle = match decode(&real).unwrap() {
        Message::Bundle(b) => b,
        Message::PeerGone => panic!(),
    };
    assert!(bundle.updates.is_some());
    bundle.updates = Some(Updates {
        bytes: vec![0xde, 0xad, 0xbe, 0xef, 1, 2, 3],
        needs: None,
    });
    let before = doc_of(&pair.main).export_json().unwrap();
    pair.main.link_receive(&encode(&bundle));
    pair.frame();
    assert_eq!(
        doc_of(&pair.main).export_json().unwrap(),
        before,
        "main unchanged"
    );
    assert!(!pair.main.is_dirty(), "nothing written by the refusal");
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    assert!(pair.equal());
    assert_eq!(
        count(&pair.pen),
        1,
        "the refused pen step is gone, main is the truth"
    );
    assert_eq!(pair.pen.tool(), Tool::Node, "tool kept");
    assert_eq!(pair.pen.undo_len(), 0);
    assert_eq!(pair.pen.selected_object_count(), 0);
    let mut notices = Vec::new();
    while let Some(n) = pair.pen.link_take_notice() {
        notices.push(n);
    }
    assert_eq!(
        notices
            .iter()
            .filter(|n| n.as_str() == "Reloaded from the main window.")
            .count(),
        1,
        "{notices:?}"
    );
}

#[test]
fn c18_a_garbage_message_to_the_pen_window_reloads_it() {
    let mut pair = Pair::open(two_rects());
    pair.pen.link_receive(&[9, 9, 9]);
    pair.frame();
    assert_eq!(pair.pen.link_state(), LinkState::OutOfSync);
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    assert!(pair.equal());
}

#[test]
fn c12_new_or_open_in_main_replaces_the_pen_project() {
    let mut pair = Pair::open(two_rects());
    click(&mut pair.pen, (10.0, 20.0));
    pair.pen.set_tool(Tool::Select);
    pair.arrive();
    let mut fresh = Session::new(0xA7);
    rect(&mut fresh, (5.0, 5.0), (9.0, 9.0));
    fresh.link_continue(&mut pair.main);
    pair.main = fresh;
    pair.settle();
    assert_eq!(count(&pair.pen), 1, "the new project");
    assert_eq!(pair.pen.selected_object_count(), 0);
    assert_eq!(pair.pen.undo_len(), 0);
    assert_eq!(pair.pen.tool(), Tool::Select, "tool kept");
    assert!(pair.equal());
    // Later steps of the new project cross as before.
    rect(&mut pair.pen, (20.0, 20.0), (30.0, 30.0));
    pair.arrive();
    assert!(pair.equal());
}

// ---------------------------------------------------------------- criterion 16

fn place(x: Option<(i32, i32)>) -> PenWindowPlace {
    PenWindowPlace {
        width: 1800,
        height: 1000,
        position: x,
        maximised: false,
    }
}

#[test]
fn c16_golden_file() {
    // 2b criterion 4 writes format 2 with both switches; format 1 still reads.
    let text = write_pen_window(&place(Some((2600, 20))), PenWindowFlags::default());
    let golden = "{\n  \"format_version\": 2,\n  \"width\": 1800,\n  \"height\": 1000,\n  \"x\": 2600,\n  \"y\": 20,\n  \"maximised\": false,\n  \"follow_view\": false,\n  \"canvas_only\": false\n}\n";
    assert_eq!(text, golden);
    assert_eq!(read_pen_window(golden), Some(place(Some((2600, 20)))));
    let golden_v1 = "{\n  \"format_version\": 1,\n  \"width\": 1800,\n  \"height\": 1000,\n  \"x\": 2600,\n  \"y\": 20,\n  \"maximised\": false\n}\n";
    assert_eq!(read_pen_window(golden_v1), Some(place(Some((2600, 20)))));
    let no_pos = write_pen_window(&place(None), PenWindowFlags::default());
    assert!(
        no_pos.contains("\"x\": null") && no_pos.contains("\"y\": null"),
        "{no_pos}"
    );
    assert_eq!(read_pen_window(&no_pos), Some(place(None)));
    assert!(!text.contains("project") && !text.contains("path"));
}

#[test]
fn c16_missing_damaged_newer_and_odd_files_read_as_none() {
    let ok = |v: &str, extra: &str| {
        format!(
            "{{\"format_version\": {v}, \"width\": 1200, \"height\": 700, \"x\": 1, \"y\": 2, \"maximised\": false{extra}}}"
        )
    };
    assert!(read_pen_window(&ok("1", "")).is_some());
    for bad in [
        String::new(),
        "not json".into(),
        "{".into(),
        ok("2", ""),
        ok("0", ""),
        ok("1", ", \"follow_view\": true"),
        ok("1", ", \"project\": \"x\""),
        "{\"format_version\":1,\"width\":50,\"height\":700,\"x\":1,\"y\":2,\"maximised\":false}".into(),
        "{\"format_version\":1,\"width\":1200,\"height\":99999,\"x\":1,\"y\":2,\"maximised\":false}".into(),
        "{\"format_version\":1,\"width\":-5,\"height\":700,\"x\":1,\"y\":2,\"maximised\":false}".into(),
        "{\"format_version\":1,\"width\":1200,\"height\":700,\"x\":1,\"maximised\":false}".into(),
        "{\"format_version\":1,\"width\":1200,\"height\":700,\"x\":1,\"y\":null,\"maximised\":false}".into(),
        "{\"format_version\":1,\"width\":1200,\"height\":700,\"x\":1,\"y\":2}".into(),
        "[]".into(),
        format!("{}{}", ok("1", ""), " ".repeat(5000)),
    ] {
        assert!(read_pen_window(&bad).is_none(), "{bad:.80}");
    }
}

#[test]
fn c16_opening_rules() {
    let laptop = ScreenRect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1600,
    };
    let kamvas = ScreenRect {
        x: 2560,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let o = opening(None, &[laptop, kamvas], Some(laptop));
    assert_eq!(
        o.size,
        OpeningSize::Logical(DEFAULT_LOGICAL_SIZE.0, DEFAULT_LOGICAL_SIZE.1)
    );
    assert_eq!(DEFAULT_LOGICAL_SIZE, (1200, 720));
    assert!(o.position.is_none() && !o.maximised);
    let saved = PenWindowPlace {
        width: 1800,
        height: 1000,
        position: Some((2600, 20)),
        maximised: true,
    };
    let o = opening(Some(&saved), &[laptop, kamvas], Some(laptop));
    assert_eq!(o.position, Some((2600, 20)));
    assert!(o.maximised);
    assert_eq!(o.size, OpeningSize::Physical(1800, 1000));
    // The screen is gone: centred on the main window's screen, size and maximised kept.
    let o = opening(Some(&saved), &[laptop], Some(laptop));
    assert_eq!(o.position, Some((380, 300)));
    assert!(o.maximised);
    // Wayland: no saved position, none applied.
    let wayland = PenWindowPlace {
        position: None,
        ..saved
    };
    assert!(
        opening(Some(&wayland), &[laptop], Some(laptop))
            .position
            .is_none()
    );
    // No screens known at all: no position and no panic.
    let _ = opening(Some(&saved), &[], None);
    assert!(opening(Some(&saved), &[], None).position.is_none());
    // A window larger than the main screen: no negative offset.
    let huge = PenWindowPlace {
        width: 9000,
        height: 9000,
        position: Some((-9000, -9000)),
        maximised: false,
    };
    let o = opening(Some(&huge), &[laptop], Some(laptop));
    assert_eq!(o.position, Some((0, 0)));
}

// ---------------------------------------------------------------- criterion 21

fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if let Some(name) = t
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix("\",").or_else(|| rest.strip_suffix('"')))
        {
            out.push(name.to_owned());
        }
    }
    out
}

fn rust_allow_list() -> Vec<String> {
    let src = include_str!("../../curvyo-app/src/window_commands.rs");
    let start = src.find("PEN_ALLOWED: &[&str] = &[").unwrap();
    let end = start + src[start..].find("];").unwrap();
    quoted(&src[start..end])
}

fn ts_allow_list() -> Vec<String> {
    let src = include_str!("../../frontend/src/lib/windowRole.ts");
    let start = src.find("PEN_ALLOWED").unwrap();
    let end = start + src[start..].find("]);").unwrap();
    quoted(&src[start..end])
}

#[test]
fn c21_the_rust_and_the_page_allow_lists_are_equal() {
    let (mut a, mut b) = (rust_allow_list(), ts_allow_list());
    assert!(a.len() >= 10, "{a:?}");
    a.sort();
    b.sort();
    assert_eq!(a, b);
    let mut dedup = a.clone();
    dedup.dedup();
    assert_eq!(dedup, a, "no duplicates");
}

#[test]
fn c21_nothing_that_changes_the_main_windows_project_state_is_on_the_list() {
    let handler = include_str!("../../curvyo-app/src/main.rs");
    let start = handler.find("generate_handler![").unwrap();
    let end = start + handler[start..].find(']').unwrap();
    let registered: Vec<String> = handler[start..end]
        .lines()
        .skip(1)
        .filter_map(|l| l.trim().strip_suffix(','))
        .filter_map(|p| p.rsplit("::").next())
        .map(str::to_owned)
        .collect();
    for command in rust_allow_list() {
        assert!(registered.contains(&command), "{command} is registered");
        for forbidden in [
            "save",
            "open",
            "pick",
            "dirty",
            "recovery",
            "quit",
            "reset",
            "export",
            "import",
            "connector",
            "project",
            "write",
            "delete",
            "set_",
            "close",
            "title",
            "sync",
            "invite",
            "online",
            "workspace",
            "library",
            "send",
            "update",
        ] {
            assert!(
                !command.contains(forbidden)
                    || command == "bridge_send"
                    || command == "read_formats_file"
                    || command == "read_extensions_file"
                    || command == "read_machine_choice"
                    || command == "read_test_grid_choice"
                    || command == "read_material_order"
                    || command == "read_retired_test_ids",
                "{command} contains {forbidden}"
            );
        }
        assert!(
            command.starts_with("pen_")
                || command.starts_with("bridge_")
                || command.starts_with("read_")
                || command.starts_with("list_")
                || command.ends_with("_list")
                || command == "font_bytes",
            "{command} is neither a reader nor the pen reader nor the bridge"
        );
    }
}

#[test]
fn c21_the_pen_window_has_no_capability_file_entry() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-app/capabilities");
    for entry in std::fs::read_dir(dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("\"pen\""), "{text}");
        assert!(!text.contains("\"*\""), "{text}");
    }
}
