//! Acceptance tests for `specs/0158-pen-display` slice 2b (Pen layout and Follow view), criteria 2
//! to 6 as far as they reach session level, plus the 2a follow-up "the pen window opens with the
//! main window's selection": two `Session`s and a byte shuttle in one process, no window. Written
//! from the specification before the implementation was read.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::type_complexity,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, Direction, Document, NewAnchor, NodeId, PathSnapshot, Point, pack, unpack,
};
use curvyo_editor_wasm::{LinkRole, LinkState, Session, SvgExportResult, Tool};
use curvyo_ui_core::SvgExportOptions;
use curvyo_ui_core::pen_window_place::{
    PenWindowFlags, PenWindowPlace, read_pen_window, read_pen_window_flags, write_pen_window,
};
use curvyo_ui_core::window_bridge::{Control, Message, decode, encode};

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

// ---------------------------------------------------------------- helpers of 2b

const MAIN_SIZE: (f64, f64) = (1600.0, 900.0);
const PEN_SIZE: (f64, f64) = (1200.0, 720.0);

fn sized_main(mut main: Session) -> Session {
    main.resize_viewport(MAIN_SIZE.0, MAIN_SIZE.1);
    main
}

fn centre(s: &Session, size: (f64, f64)) -> Point {
    s.screen_to_document(size.0 / 2.0, size.1 / 2.0)
}

fn mm_per_px(s: &Session) -> f64 {
    s.screen_to_document(1.0, 0.0).x - s.screen_to_document(0.0, 0.0).x
}

fn visible(s: &Session, size: (f64, f64)) -> [f64; 4] {
    let a = s.screen_to_document(0.0, 0.0);
    let b = s.screen_to_document(size.0, size.1);
    [a.x, a.y, b.x, b.y]
}

fn views_equal(p: &Pair, pen_size: (f64, f64)) -> bool {
    let (a, b) = (centre(&p.main, MAIN_SIZE), centre(&p.pen, pen_size));
    let (sa, sb) = (mm_per_px(&p.main), mm_per_px(&p.pen));
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3 && ((sa - sb) / sa).abs() < 1e-6
}

fn pan(s: &mut Session, dx: f64, dy: f64) {
    s.wheel(dx, dy, 100.0, 100.0, false, false);
}

fn zoom(s: &mut Session, delta: f64) {
    s.wheel(0.0, delta, 300.0, 200.0, false, true);
}

/// One frame like `Pair::frame`, counting the View items of both directions: (all, moved).
fn frame_counting(p: &mut Pair, tally: &mut (u32, u32)) {
    let from_main = p.main.link_frame();
    let from_pen = p.pen.link_frame();
    for bytes in [&from_main, &from_pen].into_iter().flatten() {
        if let Ok(Message::Bundle(b)) = decode(bytes) {
            for c in &b.controls {
                if let Control::View(v) = c {
                    tally.0 += 1;
                    tally.1 += u32::from(v.moved);
                }
            }
        }
    }
    if let Some(bytes) = from_main {
        p.pen.link_receive(&bytes);
    }
    if let Some(bytes) = from_pen {
        p.main.link_receive(&bytes);
    }
    if let Some(project) = p.pen.link_take_project() {
        let mut fresh = Session::open(p.next_peer, &project).unwrap();
        p.next_peer += 1;
        fresh.link_continue(&mut p.pen);
        p.pen = fresh;
    }
}

fn frames_counting(p: &mut Pair, n: usize, tally: &mut (u32, u32)) {
    for _ in 0..n {
        frame_counting(p, tally);
    }
}

fn open_pair(main: Session) -> Pair {
    Pair::open(sized_main(main))
}

fn svg(s: &Session) -> String {
    match s.export_svg(SvgExportOptions {
        text_as_paths: false,
        strokes_as_paths: false,
        selection_only: false,
        active_page_only: false,
    }) {
        SvgExportResult::Written { text, .. } => text,
        _ => panic!("no svg"),
    }
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

// ---------------------------------------------------------------- criterion 5: Follow view

#[test]
fn c05_follow_is_off_by_default_and_views_stay_independent() {
    let mut p = open_pair(Session::new(0xA1));
    assert!(!p.pen.link_follow() && !p.main.link_follow());
    let (c0m, c0p) = (centre(&p.main, MAIN_SIZE), centre(&p.pen, PEN_SIZE));
    pan(&mut p.main, 120.0, 40.0);
    zoom(&mut p.main, -300.0);
    p.settle();
    let cp = centre(&p.pen, PEN_SIZE);
    assert_eq!((cp.x, cp.y), (c0p.x, c0p.y), "pen view untouched by main");
    pan(&mut p.pen, -80.0, 10.0);
    zoom(&mut p.pen, 200.0);
    let pen_after = centre(&p.pen, PEN_SIZE);
    let main_after = centre(&p.main, MAIN_SIZE);
    p.settle();
    assert_eq!(
        centre(&p.main, MAIN_SIZE),
        main_after,
        "main untouched by pen"
    );
    assert_eq!(centre(&p.pen, PEN_SIZE), pen_after);
    assert_ne!(c0m, main_after);
}

#[test]
fn c05_on_takes_the_main_view_at_once_and_follows_both_ways() {
    let mut p = open_pair(Session::new(0xA1));
    pan(&mut p.main, 200.0, -60.0);
    zoom(&mut p.main, -500.0);
    p.settle();
    p.pen.link_set_follow(true);
    assert!(p.pen.link_follow());
    let mut tally = (0, 0);
    let mut frames = 0;
    while !views_equal(&p, PEN_SIZE) && frames < 6 {
        frame_counting(&mut p, &mut tally);
        frames += 1;
    }
    assert!(views_equal(&p, PEN_SIZE), "pen took main's centre and zoom");
    assert!(frames <= 4, "at once: {frames} frames");
    // Afterwards main follows the pen and the pen follows main.
    pan(&mut p.pen, 90.0, 90.0);
    zoom(&mut p.pen, -250.0);
    p.settle();
    assert!(views_equal(&p, PEN_SIZE), "pen move reached main");
    pan(&mut p.main, -150.0, 20.0);
    zoom(&mut p.main, 400.0);
    p.settle();
    assert!(views_equal(&p, PEN_SIZE), "main move reached pen");
    assert!(p.main.link_follow(), "main knows the pen window's switch");
}

#[test]
fn c05_twenty_alternating_changes_end_equal_and_do_not_echo() {
    let mut p = open_pair(Session::new(0xA1));
    p.pen.link_set_follow(true);
    p.settle();
    let mut tally = (0, 0);
    for i in 0..20 {
        let side = if i % 2 == 0 { &mut p.main } else { &mut p.pen };
        if i % 4 < 2 {
            pan(side, 30.0 + f64::from(i) * 7.0, -20.0 + f64::from(i));
        } else {
            zoom(side, if i % 8 < 4 { -120.0 } else { 90.0 });
        }
        frames_counting(&mut p, 4, &mut tally);
        assert!(views_equal(&p, PEN_SIZE), "equal after change {i}");
    }
    // Each change is applied once: a moved message per change, never more.
    assert!(tally.1 <= 20, "moved messages {} for 20 changes", tally.1);
    assert!(tally.0 <= 2 * 20 + 2, "all view messages {}", tally.0);
    let mut quiet = (0, 0);
    frames_counting(&mut p, 12, &mut quiet);
    assert_eq!(quiet, (0, 0), "no echo loop: silence after the last change");
}

#[test]
fn c05_crossing_changes_settle_without_a_loop() {
    let mut p = open_pair(Session::new(0xA1));
    p.pen.link_set_follow(true);
    p.settle();
    pan(&mut p.main, 300.0, 100.0);
    zoom(&mut p.pen, -300.0);
    let mut tally = (0, 0);
    frames_counting(&mut p, 8, &mut tally);
    assert!(views_equal(&p, PEN_SIZE));
    assert!(tally.0 <= 8, "bounded: {}", tally.0);
    let mut quiet = (0, 0);
    frames_counting(&mut p, 10, &mut quiet);
    assert_eq!(quiet, (0, 0));
}

#[test]
fn c05_off_decouples_and_both_keep_their_views() {
    let mut p = open_pair(Session::new(0xA1));
    p.pen.link_set_follow(true);
    p.settle();
    pan(&mut p.main, 100.0, 100.0);
    p.settle();
    assert!(views_equal(&p, PEN_SIZE));
    p.pen.link_set_follow(false);
    p.settle();
    assert!(!p.main.link_follow());
    let (m, n) = (centre(&p.main, MAIN_SIZE), centre(&p.pen, PEN_SIZE));
    p.settle();
    assert_eq!(
        centre(&p.main, MAIN_SIZE),
        m,
        "off: both keep the views they have"
    );
    assert_eq!(centre(&p.pen, PEN_SIZE), n);
    pan(&mut p.main, 77.0, 5.0);
    p.settle();
    assert_eq!(centre(&p.pen, PEN_SIZE), n, "main no longer moves the pen");
    pan(&mut p.pen, -77.0, 5.0);
    let main_now = centre(&p.main, MAIN_SIZE);
    p.settle();
    assert_eq!(
        centre(&p.main, MAIN_SIZE),
        main_now,
        "pen no longer moves main"
    );
}

#[test]
fn c05_a_much_larger_main_window_keeps_the_same_centre_and_zoom() {
    let mut main = Session::new(0xA1);
    main.resize_viewport(3000.0, 1800.0);
    let mut p = Pair::open(main);
    p.pen.link_set_follow(true);
    p.settle();
    zoom(&mut p.main, -600.0);
    pan(&mut p.main, 400.0, 300.0);
    p.settle();
    let (a, b) = (centre(&p.main, (3000.0, 1800.0)), centre(&p.pen, PEN_SIZE));
    assert!(
        (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3,
        "{a:?} {b:?}"
    );
    assert!(((mm_per_px(&p.main) - mm_per_px(&p.pen)) / mm_per_px(&p.main)).abs() < 1e-6);
    let (vm, vp) = (
        visible(&p.main, (3000.0, 1800.0)),
        visible(&p.pen, PEN_SIZE),
    );
    assert!(
        (vm[2] - vm[0]) > (vp[2] - vp[0]),
        "the pen shows the smaller part"
    );
}

#[test]
fn c05_the_view_is_never_a_step_never_dirty_never_a_document_change() {
    let mut p = open_pair(Session::new(0xA1));
    assert!(!p.main.is_dirty());
    let before = doc_of(&p.main).export_json().unwrap();
    let version = (p.main.undo_len(), p.pen.undo_len());
    p.pen.link_set_follow(true);
    for i in 0..10 {
        let s = if i % 2 == 0 { &mut p.main } else { &mut p.pen };
        pan(s, 20.0, 10.0);
        zoom(s, -50.0);
        p.arrive();
    }
    p.settle();
    assert!(!p.main.is_dirty(), "main stays not dirty");
    assert_eq!((p.main.undo_len(), p.pen.undo_len()), version);
    assert_eq!(doc_of(&p.main).export_json().unwrap(), before);
    assert!(p.equal());
    assert!(undo_name(&mut p.main).is_none() && undo_name(&mut p.pen).is_none());
}

#[test]
fn c05_follow_during_a_hold_and_after_a_reload() {
    let mut p = open_pair(Session::new(0xA1));
    p.pen.link_set_follow(true);
    p.settle();
    // A hold: the pen window ignores the pen; a view change must not break anything.
    let hold = p.main.link_hold(true).unwrap();
    p.pen.link_receive(&hold);
    p.settle();
    assert_eq!(p.pen.link_state(), LinkState::Held);
    pan(&mut p.main, 60.0, 60.0);
    p.settle();
    let release = p.main.link_hold(false).unwrap();
    p.pen.link_receive(&release);
    p.settle();
    assert_eq!(p.pen.link_state(), LinkState::Live);
    pan(&mut p.main, 10.0, 10.0);
    p.settle();
    assert!(
        views_equal(&p, PEN_SIZE),
        "after a hold the views are equal again"
    );
    // A reload keeps the switch and the exchange starts over.
    p.pen.link_receive(&[0xFF, 0xFF, 0xFF]);
    assert_eq!(p.pen.link_state(), LinkState::OutOfSync);
    p.settle();
    assert_eq!(p.pen.link_state(), LinkState::Live);
    assert!(p.pen.link_follow(), "the switch survives a reload");
    pan(&mut p.main, 33.0, -40.0);
    zoom(&mut p.pen, -100.0);
    p.settle();
    assert!(
        views_equal(&p, PEN_SIZE),
        "follow still works after the reload"
    );
    assert!(p.equal());
}

// ---------------------------------------------------------------- criterion 6: the frame

fn frame_matches(p: &Pair, pen_size: (f64, f64)) {
    let want = visible(&p.pen, pen_size);
    let got = p.main.link_pen_visible().expect("a frame");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() <= 0.01, "{got:?} vs {want:?}");
    }
    // The screen rectangle of the frame converts back to the same document rectangle.
    let px = p.main.link_pen_frame().expect("a screen frame");
    let a = p.main.screen_to_document(px[0], px[1]);
    let b = p.main.screen_to_document(px[2], px[3]);
    for (g, w) in [a.x, a.y, b.x, b.y].iter().zip(want) {
        assert!((g - w).abs() <= 0.01, "screen frame {px:?}");
    }
}

#[test]
fn c06_the_frame_equals_the_pen_windows_visible_rectangle() {
    let mut p = open_pair(Session::new(0xA1));
    p.settle();
    frame_matches(&p, PEN_SIZE);
    pan(&mut p.pen, 140.0, -70.0);
    p.arrive();
    frame_matches(&p, PEN_SIZE);
    zoom(&mut p.pen, -400.0);
    p.arrive();
    frame_matches(&p, PEN_SIZE);
    p.pen.resize_viewport(900.0, 640.0);
    p.arrive();
    frame_matches(&p, (900.0, 640.0));
    pan(&mut p.pen, 5.0, 5.0);
    zoom(&mut p.pen, 300.0);
    p.arrive();
    frame_matches(&p, (900.0, 640.0));
}

#[test]
fn c06_the_frame_moves_with_main_own_pan_and_is_independent_of_follow() {
    let mut p = open_pair(Session::new(0xA1));
    p.settle();
    let before = p.main.link_pen_frame().unwrap();
    pan(&mut p.main, 100.0, 50.0);
    let after = p.main.link_pen_frame().unwrap();
    assert!(
        (after[0] - before[0]).abs() > 1.0,
        "screen frame follows main's view"
    );
    frame_matches(&p, PEN_SIZE);
}

#[test]
fn c06_no_frame_without_a_pen_window_and_after_it_closes() {
    let mut solo = sized_main(Session::new(0xA1));
    assert!(solo.link_pen_frame().is_none() && solo.link_pen_visible().is_none());
    let mut p = open_pair(Session::new(0xA1));
    p.settle();
    assert!(p.main.link_pen_frame().is_some());
    // A minimised pen window sends nothing: the frame stays at the last view.
    pan(&mut p.pen, 30.0, 30.0);
    p.arrive();
    let last = p.main.link_pen_visible();
    for _ in 0..5 {
        let _ = p.main.link_frame();
    }
    assert_eq!(p.main.link_pen_visible(), last);
    // Closed: PEER_GONE.
    p.main
        .link_receive(&curvyo_ui_core::window_bridge::PEER_GONE);
    assert!(p.main.link_pen_frame().is_none() && p.main.link_pen_visible().is_none());
    assert!(!p.main.has_window_link());
    solo.link_start(LinkRole::Main);
    assert!(
        solo.link_pen_frame().is_none(),
        "before the pen's first view: none"
    );
}

#[test]
fn c06_the_frame_is_not_in_the_svg_not_a_hit_and_not_a_document_change() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    let svg_before = svg(&main);
    let mut p = open_pair(main);
    p.settle();
    assert!(p.main.link_pen_frame().is_some());
    assert_eq!(svg(&p.main), svg_before, "the frame is no SVG content");
    assert_eq!(svg(&p.pen), svg_before);
    // Clicking on the frame's top-left corner (empty) selects nothing; the rect behind it still hits.
    let corner = p.main.link_pen_visible().unwrap();
    click(&mut p.main, (corner[0] + 0.5, corner[1] + 0.5));
    assert_eq!(
        p.main.selected_object_count(),
        0,
        "the frame is not selectable"
    );
    click(&mut p.main, (10.0, 20.0));
    assert_eq!(
        p.main.selected_object_count(),
        1,
        "hit tests ignore the frame"
    );
    assert_eq!(count(&p.main), 1);
    let _ = p.main.is_dirty();
}

#[test]
fn c06_the_frame_alone_does_not_dirty_main() {
    let mut p = open_pair(Session::new(0xA1));
    pan(&mut p.pen, 50.0, 50.0);
    p.settle();
    assert!(!p.main.is_dirty());
    assert_eq!(p.main.undo_len(), 0);
}

// ---------------------------------------------------------------- 2a follow-up: selection on open

#[test]
fn followup_the_pen_window_opens_with_the_mains_selection() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    rect(&mut main, (60.0, 10.0), (90.0, 30.0));
    click(&mut main, (10.0, 20.0));
    assert_eq!(main.selected_object_count(), 1);
    let mut p = open_pair(main);
    p.settle();
    assert_eq!(
        p.pen.selected_object_count(),
        1,
        "selection came with the project"
    );
    assert_eq!(p.main.selected_object_count(), 1);
    // The pen's selection is real: Delete there deletes that one object in both.
    p.pen.delete_selected();
    p.arrive();
    assert_eq!(count(&p.pen), 1);
    assert!(p.equal());
    assert_eq!(p.main.selected_object_count(), 0);
}

#[test]
fn followup_nothing_selected_opens_empty_and_a_new_selection_still_crosses() {
    let mut main = Session::new(0xA1);
    rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    click(&mut main, (200.0, 200.0));
    let mut p = open_pair(main);
    p.settle();
    assert_eq!(p.pen.selected_object_count(), 0);
    click(&mut p.main, (10.0, 20.0));
    p.arrive();
    assert_eq!(p.pen.selected_object_count(), 1);
}

// ---------------------------------------------------------------- criterion 2: Eraser button

const TIP: f64 = 0.0;
const ERASER: f64 = 1.0;
const HOVER: f64 = 0.0;
const CONTACT: f64 = 1.0;

fn record(s: &Session, at: Point, pressure: f64, end: f64, phase: f64, time: f64) -> Vec<f64> {
    let origin = s.screen_to_document(0.0, 0.0);
    let unit = s.screen_to_document(1.0, 0.0);
    let px_per_mm = 1.0 / (unit.x - origin.x);
    vec![
        (at.x - origin.x) * px_per_mm,
        (at.y - origin.y) * px_per_mm,
        pressure,
        f64::NAN,
        f64::NAN,
        end,
        phase,
        0.0,
        time,
    ]
}

/// A vertical pass at `x` from `y0` to `y1` with `end` (TIP or ERASER), then a hover sample.
fn pass(s: &Session, end: f64, x: f64, y0: f64, y1: f64) -> Vec<f64> {
    let mut flat = Vec::new();
    for i in 0..=20 {
        let y = y0 + (y1 - y0) * f64::from(i) / 20.0;
        flat.extend(record(s, Point::new(x, y), 0.5, end, CONTACT, f64::from(i)));
    }
    flat.extend(record(s, Point::new(x, y1), 0.0, end, HOVER, 30.0));
    flat
}

/// Two horizontal open lines: y = 50 and y = 90, both from x = 20 to 80; `lock` and `hide`
/// are applied to the first (locked) and second (hidden layer is not available: second is
/// simply left alone).
fn lines(lock_first: bool) -> (Session, NodeId, NodeId) {
    let doc = Document::new(70);
    let line = |y: f64, tag: u64| {
        doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(70, tag), Point::new(20.0, y)),
                NewAnchor::corner(AnchorId::new(70, tag + 1), Point::new(80.0, y)),
            ],
            false,
        )
    };
    let (a, b) = (line(50.0, 1), line(90.0, 3));
    if lock_first {
        doc.set_node_locked(&[a], true).unwrap();
    }
    let mut s = Session::open(71, &pack(&doc, "t").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Pencil);
    (s, a, b)
}

fn path_ids(s: &Session) -> Vec<NodeId> {
    paths(s).iter().map(|p| p.id).collect()
}

#[test]
fn c02_button_and_eraser_end_give_the_same_result_at_every_distance() {
    // Distances (mm) of the pass's end from the line at y = 50, measured along the pass.
    for stop in [50.0, 49.0, 47.0, 44.0, 40.0, 30.0] {
        let (mut by_end, _, _) = lines(false);
        by_end.pen_samples(&pass(&by_end, ERASER, 50.0, stop - 10.0, stop));
        let (mut by_button, _, _) = lines(false);
        by_button.set_pencil_eraser(true);
        by_button.pen_samples(&pass(&by_button, TIP, 50.0, stop - 10.0, stop));
        assert_eq!(path_ids(&by_button), path_ids(&by_end), "stop {stop}");
        assert_eq!(by_button.undo_len(), by_end.undo_len(), "stop {stop}");
        assert_eq!(
            undo_name(&mut by_button),
            undo_name(&mut by_end),
            "stop {stop}"
        );
        assert!(by_button.pencil_eraser());
    }
}

#[test]
fn c02_button_deletes_in_one_erase_step_and_creates_nothing() {
    let (mut s, a, b) = lines(false);
    s.set_pencil_eraser(true);
    // A pass crossing BOTH lines in one go: both go, one step.
    s.pen_samples(&pass(&s, TIP, 50.0, 40.0, 100.0));
    assert!(path_ids(&s).is_empty(), "{a:?} {b:?} both erased");
    assert_eq!(s.undo_len(), 1);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Erase"));
    s.undo();
    assert_eq!(path_ids(&s).len(), 2, "one Undo brings both back");
    // Nothing touched: nothing written, nothing created.
    let (mut idle, _, _) = lines(false);
    idle.set_pencil_eraser(true);
    idle.pen_samples(&pass(&idle, TIP, 50.0, 60.0, 80.0));
    assert_eq!(path_ids(&idle).len(), 2);
    assert_eq!(idle.undo_len(), 0);
    assert_eq!(count(&idle), 2);
}

#[test]
fn c02_locked_objects_are_untouched_in_both_ways() {
    let (mut by_end, a, b) = lines(true);
    by_end.pen_samples(&pass(&by_end, ERASER, 50.0, 40.0, 100.0));
    let (mut by_button, _, _) = lines(true);
    by_button.set_pencil_eraser(true);
    by_button.pen_samples(&pass(&by_button, TIP, 50.0, 40.0, 100.0));
    assert_eq!(path_ids(&by_end), vec![a]);
    assert_eq!(
        path_ids(&by_button),
        vec![a],
        "locked {a:?} stays, {b:?} goes"
    );
}

#[test]
fn c02_off_or_a_second_tap_draws_again_and_the_real_end_still_erases() {
    let (mut s, _, _) = lines(false);
    s.set_pencil_eraser(true);
    s.set_pencil_eraser(true); // idempotent
    s.set_pencil_eraser(false);
    assert!(!s.pencil_eraser());
    let n = path_ids(&s).len();
    s.pen_samples(&pass(&s, TIP, 10.0, 110.0, 130.0));
    assert_eq!(path_ids(&s).len(), n + 1, "the tip draws");
    assert_eq!(undo_name(&mut s).as_deref(), Some("Pencil stroke"));
    s.set_pencil_eraser(true);
    s.pen_samples(&pass(&s, TIP, 50.0, 40.0, 60.0));
    assert_eq!(
        path_ids(&s).len(),
        n,
        "erases again (only the crossed line goes)"
    );
    // The eraser end in the Pencil mode and in the Eraser mode.
    s.set_pencil_eraser(false);
    s.pen_samples(&pass(&s, ERASER, 50.0, 80.0, 100.0));
    assert_eq!(
        path_ids(&s).len(),
        n - 1,
        "real eraser end works with the button off"
    );
}

#[test]
fn c02_the_button_is_per_window_and_not_remembered() {
    let mut p = open_pair({
        let (s, _, _) = lines(false);
        s
    });
    p.pen.set_tool(Tool::Pencil);
    p.pen.set_pencil_eraser(true);
    p.settle();
    assert!(!p.main.pencil_eraser(), "not shared with the main window");
    p.pen.pen_samples(&pass(&p.pen, TIP, 50.0, 40.0, 60.0));
    p.arrive();
    assert!(p.equal());
    assert_eq!(count(&p.main), 1, "the erase crossed to main");
    assert_eq!(p.pen.undo_len(), 1);
    assert_eq!(p.main.undo_len(), 0, "undo stays per window");
    assert!(
        !Session::new(9).pencil_eraser(),
        "a new window starts with the Pencil"
    );
}

// ---------------------------------------------------------------- criterion 3: layer pick

fn layered() -> (Session, [NodeId; 3]) {
    let doc = Document::new(80);
    let back = doc.new_layer(None);
    let middle = doc.new_layer(Some(back));
    let front = doc.new_layer(Some(middle));
    let mut s = Session::open(81, &pack(&doc, "t").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Pencil);
    (s, [back, middle, front])
}

fn json_id(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn stroke_layer(s: &mut Session, y: f64) -> Option<NodeId> {
    let before = path_ids(s);
    s.pen_samples(&pass(s, TIP, 30.0, y, y + 20.0));
    let after = path_ids(s);
    let new = after.into_iter().find(|id| !before.contains(id))?;
    doc_of(s).parent_node(new)
}

#[test]
fn c03_the_pick_sets_the_layer_of_the_next_strokes_and_writes_nothing() {
    let (mut s, [back, middle, front]) = layered();
    let json = doc_of(&s).export_json().unwrap();
    let (undo, dirty) = (s.undo_len(), s.is_dirty());
    assert!(s.set_pen_layer(&json_id(middle)));
    assert_eq!(s.active_layer(), Some(middle));
    assert_eq!(
        doc_of(&s).export_json().unwrap(),
        json,
        "no document change"
    );
    assert_eq!((s.undo_len(), s.is_dirty()), (undo, dirty), "no step");
    assert_eq!(stroke_layer(&mut s, 10.0), Some(middle));
    assert!(s.set_pen_layer(&json_id(back)));
    assert_eq!(stroke_layer(&mut s, 50.0), Some(back));
    assert!(s.set_pen_layer(&json_id(front)));
    assert_eq!(stroke_layer(&mut s, 90.0), Some(front));
    // Earlier strokes stay in their layers.
    let d = doc_of(&s);
    let layers: Vec<_> = path_ids(&s).iter().map(|id| d.parent_node(*id)).collect();
    for layer in [middle, back, front] {
        assert_eq!(
            layers.iter().filter(|l| **l == Some(layer)).count(),
            1,
            "one stroke in each layer"
        );
    }
}

#[test]
fn c03_the_pick_leaves_the_selection_alone_and_a_bad_id_changes_nothing() {
    let (mut s, [back, middle, _]) = layered();
    assert!(s.set_pen_layer(&json_id(back)));
    let first = stroke_layer(&mut s, 10.0);
    assert_eq!(first, Some(back));
    assert_eq!(s.selected_object_count(), 1, "the stroke is selected");
    assert!(s.set_pen_layer(&json_id(middle)));
    assert_eq!(
        s.selected_object_count(),
        1,
        "the pick does not touch the selection"
    );
    for bad in ["", "null", "nonsense", "{}", "999999999"] {
        assert!(!s.set_pen_layer(bad), "{bad}");
    }
    assert_eq!(s.active_layer(), Some(middle));
    // A non-layer node id (the stroke) is refused too.
    let stroke = path_ids(&s)[0];
    assert!(!s.set_pen_layer(&json_id(stroke)));
    assert_eq!(s.active_layer(), Some(middle));
}

#[test]
fn c03_a_selection_moves_the_active_layer_as_before() {
    let (mut s, [back, _, front]) = layered();
    assert!(s.set_pen_layer(&json_id(back)));
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(10.0, 120.0), false);
    s.pointer_up(Point::new(40.0, 140.0), false, false);
    let rect_id = *path_ids_or_objects(&s).last().unwrap();
    assert_eq!(doc_of(&s).parent_node(rect_id), Some(back), "precondition");
    assert!(s.set_pen_layer(&json_id(front)));
    click(&mut s, (300.0, 300.0));
    assert_eq!(s.selected_object_count(), 0);
    click(&mut s, (10.0, 130.0));
    assert_eq!(s.selected_object_count(), 1, "the rectangle's edge is hit");
    assert_eq!(
        s.active_layer(),
        Some(back),
        "selecting an object makes its layer active"
    );
    s.set_tool(Tool::Pencil);
    assert_eq!(stroke_layer(&mut s, 10.0), Some(back));
}

fn path_ids_or_objects(s: &Session) -> Vec<NodeId> {
    ids(s)
}

#[test]
fn c03_hidden_and_locked_layers_are_chooseable_with_the_notice() {
    let doc = Document::new(80);
    let back = doc.new_layer(None);
    let front = doc.new_layer(Some(back));
    doc.set_node_locked(&[back], true).unwrap();
    doc.set_node_visible(&[front], false).unwrap();
    let mut s = Session::open(81, &pack(&doc, "t").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Pencil);
    let rows = s.pen_layers();
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![front, back]
    );
    assert!(rows[0].hidden && rows[1].locked);
    for layer in [back, front] {
        assert!(s.set_pen_layer(&json_id(layer)));
        let n = count(&s);
        s.pen_samples(&pass(&s, TIP, 30.0, 10.0, 30.0));
        assert_eq!(count(&s), n, "nothing starts on a hidden or locked layer");
        assert!(!s.take_layer_notice().is_empty(), "the Pen's notice");
    }
}

#[test]
fn c03_a_page_without_layers_lists_none() {
    assert!(Session::new(5).pen_layers().is_empty());
}

#[test]
fn c03_the_pick_is_per_window_and_not_a_document_change_in_a_pair() {
    let (main, [_, middle, _]) = layered();
    let main_active = main.active_layer();
    let mut p = open_pair(main);
    p.pen.set_tool(Tool::Pencil);
    p.settle();
    assert!(p.pen.set_pen_layer(&json_id(middle)));
    p.settle();
    assert_eq!(
        p.main.active_layer(),
        main_active,
        "the main window's layer is not moved"
    );
    assert!(!p.main.is_dirty());
    assert_eq!(p.main.undo_len() + p.pen.undo_len(), 0);
    assert!(p.equal());
    assert_eq!(stroke_layer(&mut p.pen, 10.0), Some(middle));
    p.arrive();
    assert!(p.equal());
    let d = doc_of(&p.main);
    assert_eq!(d.parent_node(path_ids(&p.main)[0]), Some(middle));
}

// ---------------------------------------------------------------- criterion 4: pen_window.json

fn place() -> PenWindowPlace {
    PenWindowPlace {
        width: 1920,
        height: 1080,
        position: Some((2560, 0)),
        maximised: true,
    }
}

const V2_BOTH: &str = "{\n  \"format_version\": 2,\n  \"width\": 1920,\n  \"height\": 1080,\n  \"x\": 2560,\n  \"y\": 0,\n  \"maximised\": true,\n  \"follow_view\": true,\n  \"canvas_only\": true\n}\n";

#[test]
fn c04_format_2_golden_round_trips_both_switches() {
    let flags = PenWindowFlags {
        follow_view: true,
        canvas_only: true,
    };
    assert_eq!(write_pen_window(&place(), flags), V2_BOTH);
    assert_eq!(read_pen_window(V2_BOTH), Some(place()));
    assert_eq!(read_pen_window_flags(V2_BOTH), flags);
    for (f, c) in [(true, false), (false, true), (false, false)] {
        let flags = PenWindowFlags {
            follow_view: f,
            canvas_only: c,
        };
        let text = write_pen_window(&place(), flags);
        assert_eq!(read_pen_window_flags(&text), flags);
        assert_eq!(read_pen_window(&text), Some(place()));
        assert!(
            !text.contains("project") && !text.contains("layout"),
            "{text}"
        );
    }
    let fixture = include_str!("../../curvyo-ui-core/tests/fixtures/pen_window/pen_window_v2.json");
    let flags = read_pen_window_flags(fixture);
    assert_eq!(
        write_pen_window(&read_pen_window(fixture).unwrap(), flags),
        fixture
    );
}

#[test]
fn c04_format_1_reads_with_both_switches_off_and_is_never_needed_for_writing() {
    let v1 = "{\n  \"format_version\": 1,\n  \"width\": 1920,\n  \"height\": 1080,\n  \"x\": 2560,\n  \"y\": 0,\n  \"maximised\": true\n}\n";
    assert_eq!(read_pen_window(v1), Some(place()));
    assert_eq!(read_pen_window_flags(v1), PenWindowFlags::default());
}

#[test]
fn c04_damaged_newer_and_odd_files_read_as_none_with_both_off() {
    let ok = |v: &str, extra: &str| {
        format!(
            "{{\"format_version\": {v}, \"width\": 1200, \"height\": 700, \"x\": 1, \"y\": 2, \"maximised\": false{extra}}}"
        )
    };
    let flags = ", \"follow_view\": true, \"canvas_only\": true";
    assert!(read_pen_window(&ok("2", flags)).is_some());
    for bad in [
        String::new(),
        "{".into(),
        ok("3", flags),
        ok("0", flags),
        ok("2", ", \"follow_view\": 1, \"canvas_only\": true"),
        ok("2", ", \"follow_view\": \"yes\", \"canvas_only\": true"),
        ok("2", &format!("{flags}, \"layout\": \"full\"")),
        ok("2", &format!("{flags}, \"project\": \"x\"")),
        ok("1", flags),
    ] {
        assert!(read_pen_window(&bad).is_none(), "{bad:.100}");
        assert_eq!(
            read_pen_window_flags(&bad),
            PenWindowFlags::default(),
            "{bad:.100}"
        );
    }
    for fixture in [
        include_str!("../../curvyo-ui-core/tests/fixtures/pen_window/pen_window_newer.json"),
        include_str!("../../curvyo-ui-core/tests/fixtures/pen_window/pen_window_damaged.json"),
    ] {
        assert!(read_pen_window(fixture).is_none());
        assert_eq!(read_pen_window_flags(fixture), PenWindowFlags::default());
    }
}

#[test]
fn c04_there_is_no_layout_file_or_setting_in_the_app() {
    let app = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-app/src");
    for entry in std::fs::read_dir(app).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
            for needle in [
                "pen_layout",
                "pen-layout",
                "customise",
                "customize",
                "pen_rail_side",
                "mirror",
            ] {
                assert!(!text.contains(needle), "{path:?} has {needle}");
            }
        }
    }
}
