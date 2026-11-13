//! Tester acceptance for `specs/0110-pen-modes-variable-stroke` after the customer decisions of
//! 2026-11-11: Escape discards an unfinished Pen path and a second Escape returns to Select; a
//! right click finishes the path (with nothing in progress it does nothing); a Spiro never shows
//! a "cannot be solved" notice or a straight-line fallback, never jumps while a point is moved,
//! and is stored and reopened as shown. Written from the brief before reading the implementation.
//! Amended for `specs/0115` (customer 2026-11-12, the curve is a function of its controls): a
//! moved point's curve turns over at once where the point crosses the line behind a neighbour,
//! by at most [`TURNOVER_LIMIT`]; drags across a neighbour are checked for storing and reopening.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use curvyo_document_core::{CurveKind, Document, Point, unpack};
use curvyo_editor_wasm::{EscapeStep, Session, Tool};
use curvyo_render_core::{DrawList, Vertex};
use curvyo_ui_core::PenMode;

// ---------- helpers ----------

fn artwork(list: &DrawList) -> Vec<Vertex> {
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[..end].to_vec()
}

fn saved(session: &Session) -> Document {
    unpack(900, &session.pack("test").unwrap()).unwrap()
}

fn click(session: &mut Session, p: Point) {
    session.pointer_down(p, false);
    session.pointer_up(p, false, false);
}

/// The pointer moves to `p` and the host draws a frame, as a real pointer sends it.
fn hover(session: &mut Session, p: Point) -> DrawList {
    session.pointer_hover(p, false, false);
    session.draw_list()
}

fn pen(mode: PenMode, peer: u64) -> Session {
    let mut s = Session::new(peer);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(mode));
    s
}

/// Points of `a` farther than `limit` from every point of `b`, as the largest such distance:
/// one direction of the Hausdorff distance over the triangle vertices.
fn reach(a: &[Vertex], b: &[Vertex]) -> f64 {
    a.iter()
        .map(|p| {
            b.iter()
                .map(|q| (p.position.x - q.position.x).hypot(p.position.y - q.position.y))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

/// The Hausdorff distance between two drawings, in mm; 0 when both are empty.
///
/// Amended by the implementer for `specs/0115` (2026-11-12, needs the tester's re-check): the
/// triangle edges are sampled every 1 mm, so a long flat span drawn with few vertices no longer
/// reads as a move of half its length (the G2 Spiro of `0115` draws such spans).
fn distance(a: &[Vertex], b: &[Vertex]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }
    let (a, b) = (dense(a), dense(b));
    reach(&a, &b).max(reach(&b, &a))
}

/// The vertices of `triangles` and points along each edge at most 1 mm apart.
fn dense(triangles: &[Vertex]) -> Vec<Vertex> {
    let mut out = Vec::new();
    for t in triangles.as_chunks::<3>().0 {
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (dx, dy) = (q.position.x - p.position.x, q.position.y - p.position.y);
            let n = dx.hypot(dy).ceil().max(1.0) as u32;
            for i in 0..n {
                let f = f64::from(i) / f64::from(n);
                let mut v = p;
                v.position.x += dx * f;
                v.position.y += dy * f;
                out.push(v);
            }
        }
    }
    out
}

/// Every notice a host takes after a frame is empty: no "cannot be solved" or any other
/// refusal pops up while a Spiro is drawn or dragged.
fn assert_no_notice(s: &mut Session, at: &str) {
    assert_eq!(s.take_unsupported_width_notice(), "", "{at}: width notice");
    assert_eq!(s.take_width_edit_notice(), "", "{at}: width edit notice");
    assert_eq!(s.take_transform_notice(), "", "{at}: transform notice");
    assert_eq!(s.take_pen_entry_notice(), None, "{at}: pen entry notice");
    assert_eq!(s.take_history_notice(), None, "{at}: history notice");
}

/// The overlay triangles of the Lines preview of the same placed points and pointer: what a
/// straight-line fallback would draw.
fn lines_preview(placed: &[Point], pointer: Point) -> Vec<Vertex> {
    let mut l = pen(PenMode::Lines, 99);
    for &p in placed {
        click(&mut l, p);
    }
    hover(&mut l, pointer).triangles
}

// ---------- abort keys ----------

fn place(mode: PenMode, points: &[Point]) -> Session {
    let mut s = pen(mode, 11);
    for &p in points {
        hover(&mut s, p);
        click(&mut s, p);
    }
    hover(&mut s, Point::new(80.0, 40.0));
    s
}

const THREE: [Point; 3] = [
    Point::new(10.0, 10.0),
    Point::new(30.0, 20.0),
    Point::new(50.0, 10.0),
];

#[test]
fn escape_discards_the_unfinished_path_and_a_second_escape_returns_to_select() {
    for mode in [
        PenMode::Bezier,
        PenMode::Spiro,
        PenMode::BSpline,
        PenMode::Lines,
    ] {
        let mut s = place(mode, &THREE);
        assert!(s.pen_in_progress().is_some(), "{mode:?}: setup");
        assert_eq!(
            s.escape(),
            EscapeStep::ClearedState,
            "{mode:?}: first Escape"
        );
        assert!(s.pen_in_progress().is_none(), "{mode:?}: path discarded");
        assert_eq!(s.tool(), Tool::Pen, "{mode:?}: still the Pen");
        assert!(
            saved(&s).object_ids().is_empty(),
            "{mode:?}: nothing was committed"
        );
        assert!(artwork(&s.draw_list()).is_empty(), "{mode:?}: no artwork");
        let _ = s.undo();
        assert!(
            saved(&s).object_ids().is_empty(),
            "{mode:?}: undo has nothing"
        );
        assert_eq!(s.escape(), EscapeStep::LeftTool, "{mode:?}: second Escape");
        assert_eq!(s.tool(), Tool::Select, "{mode:?}: back to Select");
    }
}

#[test]
fn escape_with_nothing_in_progress_returns_to_select_at_once() {
    let mut s = pen(PenMode::Spiro, 12);
    hover(&mut s, Point::new(20.0, 20.0));
    assert_eq!(s.escape(), EscapeStep::LeftTool);
    assert_eq!(s.tool(), Tool::Select);
}

#[test]
fn escape_during_a_bezier_handle_drag_discards_the_whole_path() {
    let mut s = place(PenMode::Bezier, &THREE[..2]);
    s.pointer_down(Point::new(60.0, 30.0), false);
    s.pointer_hover(Point::new(70.0, 40.0), false, false);
    let _ = s.draw_list();
    assert_eq!(s.escape(), EscapeStep::CancelledDrag);
    s.pointer_up(Point::new(70.0, 40.0), false, false);
    assert!(s.pen_in_progress().is_none(), "the unfinished path goes");
    assert!(saved(&s).object_ids().is_empty(), "nothing committed");
    assert_eq!(s.tool(), Tool::Pen);
}

#[test]
fn escape_keeps_paths_finished_earlier() {
    let mut s = place(PenMode::Spiro, &THREE);
    s.finish_pen();
    let first = artwork(&s.draw_list());
    for p in [Point::new(10.0, 50.0), Point::new(40.0, 60.0)] {
        hover(&mut s, p);
        click(&mut s, p);
    }
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(saved(&s).object_ids().len(), 1, "only the second path went");
    assert_eq!(artwork(&s.draw_list()), first);
}

#[test]
fn right_click_finishes_the_path_in_progress_without_a_node_or_tool_change() {
    for mode in [
        PenMode::Bezier,
        PenMode::Spiro,
        PenMode::BSpline,
        PenMode::Lines,
    ] {
        let mut s = place(mode, &THREE);
        let mut enter = place(mode, &THREE);
        enter.finish_pen();
        assert!(s.pen_secondary_press(), "{mode:?}: the press is the Pen's");
        assert!(s.pen_in_progress().is_none(), "{mode:?}: finished");
        assert_eq!(s.tool(), Tool::Pen, "{mode:?}: still the Pen");
        let doc = saved(&s);
        assert_eq!(doc.object_ids().len(), 1, "{mode:?}: one path");
        let path = doc.path(doc.object_ids()[0]).unwrap();
        assert_eq!(
            path.anchors.len(),
            3,
            "{mode:?}: no node at the right click"
        );
        assert!(!path.closed, "{mode:?}: open");
        let by_enter = saved(&enter);
        let enter_path = by_enter.path(by_enter.object_ids()[0]).unwrap();
        assert_eq!(
            path.anchors.iter().map(|a| a.point).collect::<Vec<_>>(),
            enter_path
                .anchors
                .iter()
                .map(|a| a.point)
                .collect::<Vec<_>>(),
            "{mode:?}: as Enter finishes it"
        );
        assert_eq!(path.curve_kind, enter_path.curve_kind, "{mode:?}");
        // One undo step takes the whole path.
        let _ = s.undo();
        assert!(saved(&s).object_ids().is_empty(), "{mode:?}: one undo step");
    }
}

#[test]
fn right_click_with_nothing_in_progress_does_nothing() {
    let mut s = pen(PenMode::Spiro, 13);
    hover(&mut s, Point::new(20.0, 20.0));
    let before = s.draw_list().triangles;
    assert!(s.pen_secondary_press(), "the Pen takes the press");
    assert_eq!(s.tool(), Tool::Pen, "no tool switch");
    assert!(s.pen_in_progress().is_none(), "no node placed");
    assert!(saved(&s).object_ids().is_empty(), "nothing committed");
    assert_eq!(hover(&mut s, Point::new(20.0, 20.0)).triangles, before);
    // A second right click after a finished path does nothing either.
    let mut s = place(PenMode::Lines, &THREE);
    assert!(s.pen_secondary_press());
    assert!(s.pen_secondary_press());
    assert_eq!(saved(&s).object_ids().len(), 1);
    assert_eq!(s.tool(), Tool::Pen);
}

#[test]
fn right_click_belongs_to_the_pen_only() {
    let mut s = Session::new(14);
    for tool in [Tool::Select, Tool::Node, Tool::Rectangle] {
        s.set_tool(tool);
        assert!(
            !s.pen_secondary_press(),
            "{tool:?} keeps its secondary press"
        );
        assert_eq!(s.tool(), tool);
    }
}

// ---------- Spiro stability ----------

/// Largest frame-to-frame move of the drawing while a Spiro point follows the pointer, in mm,
/// with pointer steps of 0.5 mm. A jump to another solution moves the curve by tens of mm.
const JUMP_LIMIT: f64 = 10.0;
/// The largest move of the drawing between two pointer events while the Pen's point circles or
/// doubles back 15 mm from the last node (`specs/0115`): where the point crosses the line behind
/// that node the curve turns over, by less than the point's distance from it.
const TURNOVER_LIMIT: f64 = 15.0;
const STEP: f64 = 0.5;

/// The pointer positions of a walk along `path` in steps of at most `STEP`.
fn walk(path: &[Point]) -> Vec<Point> {
    let mut out = vec![path[0]];
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = ((b.x - a.x).hypot(b.y - a.y) / STEP).ceil().max(1.0) as u32;
        for i in 1..=n {
            let t = f64::from(i) / f64::from(n);
            out.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

fn circle(c: Point, r: f64, from_deg: f64, to_deg: f64) -> Vec<Point> {
    let steps = 360;
    (0..=steps)
        .map(|i| {
            let a = (from_deg + (to_deg - from_deg) * f64::from(i) / f64::from(steps)).to_radians();
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// The Pen's live Spiro point follows the pointer along `path`. Every frame: no notice, a curve
/// on screen that is not the straight-line fallback, and (except where the pointer is over the
/// closing node, where the closed preview is meant to differ) no jump. Returns the worst move.
fn sweep_pen(s: &mut Session, placed: &[Point], path: &[Point]) -> Jump {
    let mut last: Option<Vec<Vertex>> = None;
    let mut worst = Jump {
        moved: 0.0,
        at: path[0],
        boxes: ([0.0; 4], [0.0; 4]),
        over: 0,
        stalls: 0,
        over_spec: 0,
        events: 0,
    };
    let (mut over_spec, mut events) = (0, 0);
    for p in walk(path) {
        let frame = hover(s, p).triangles;
        assert_no_notice(s, &format!("pointer {p:?}"));
        assert!(!frame.is_empty(), "pointer {p:?}: the curve is shown");
        assert_ne!(
            frame,
            lines_preview(placed, p),
            "pointer {p:?}: no straight-line fallback"
        );
        let special = matches!(
            s.pen_target(),
            Some(curvyo_ui_core::PenTarget::Close { .. })
        ) || placed.iter().any(|q| (q.x - p.x).hypot(q.y - p.y) < 1.0);
        if special {
            last = None;
            continue;
        }
        if let Some(before) = &last {
            let moved = distance(before, &frame);
            events += 1;
            over_spec += usize::from(moved > 4.0 * STEP + 2.0);
            if moved > worst.moved {
                worst = Jump {
                    moved,
                    at: p,
                    boxes: (bbox(before), bbox(&frame)),
                    over: 0,
                    stalls: 0,
                    over_spec: 0,
                    events: 0,
                };
            }
        }
        last = Some(frame);
    }
    worst.over_spec = over_spec;
    worst.events = events;
    worst
}

fn spiro_pen_with(placed: &[Point], peer: u64) -> Session {
    let mut s = pen(PenMode::Spiro, peer);
    for &p in placed {
        hover(&mut s, p);
        click(&mut s, p);
    }
    s
}

const WAVE: [Point; 3] = [
    Point::new(10.0, 20.0),
    Point::new(30.0, 10.0),
    Point::new(50.0, 20.0),
];

#[test]
fn spiro_pen_point_circling_the_last_node_never_jumps_or_falls_back() {
    let mut s = spiro_pen_with(&WAVE, 20);
    let jump = sweep_pen(&mut s, &WAVE, &circle(WAVE[2], 15.0, 0.0, 720.0));
    println!(
        "worst {:.1} mm, {} of {} events over 4 mm",
        jump.moved, jump.over_spec, jump.events
    );
    // `specs/0115` criterion 2: a move above 4 x the step + 2 mm at no more than 1 % of events.
    assert!(
        jump.over_spec * 100 <= jump.events,
        "{} of {} events moved more than 4 mm",
        jump.over_spec,
        jump.events
    );
    assert!(
        jump.moved <= TURNOVER_LIMIT,
        "the curve jumped {:.1} mm with the pointer at ({:.1}, {:.1}), box {:?} -> {:?}",
        jump.moved,
        jump.at.x,
        jump.at.y,
        jump.boxes.0,
        jump.boxes.1
    );
}

#[test]
fn spiro_pen_point_doubling_back_past_the_path_never_jumps_or_falls_back() {
    let mut s = spiro_pen_with(&WAVE, 21);
    let jump = sweep_pen(
        &mut s,
        &WAVE,
        &[
            Point::new(70.0, 20.0),
            Point::new(30.0, 12.0),
            Point::new(0.0, 30.0),
            Point::new(70.0, 20.0),
        ],
    );
    println!(
        "worst {:.1} mm, {} of {} events over 4 mm",
        jump.moved, jump.over_spec, jump.events
    );
    // `specs/0115` criterion 2: a move above 4 x the step + 2 mm at no more than 1 % of events.
    assert!(
        jump.over_spec * 100 <= jump.events,
        "{} of {} events moved more than 4 mm",
        jump.over_spec,
        jump.events
    );
    assert!(
        jump.moved <= TURNOVER_LIMIT,
        "the curve jumped {:.1} mm with the pointer at ({:.1}, {:.1}), box {:?} -> {:?}",
        jump.moved,
        jump.at.x,
        jump.at.y,
        jump.boxes.0,
        jump.boxes.1
    );
}

/// The artwork of a session and of the same bytes reopened agree within `tol` mm: a reopen
/// solves from nothing, the session kept the shape it showed.
fn reopen_equals(s: &Session, shown: &[Vertex], tol: f64) -> Result<(), String> {
    let bytes = s.pack("test").unwrap();
    let r = Session::open(901, &bytes).unwrap();
    let d = distance(shown, &artwork(&r.draw_list()));
    if d <= tol {
        Ok(())
    } else {
        Err(format!("reopen differs from the shown curve by {d:.3} mm"))
    }
}

/// The finished Spiro of each track, the preview the Pen showed and a reopen agree. A click
/// beyond any near solution places the point the Pen showed. Track 0 is an easy wave: its
/// figure is the baseline of the measure (overlay hairline against artwork stroke).
#[test]
fn spiro_clicks_in_hard_places_are_stored_and_reopened_as_shown() {
    let mut failures = Vec::new();
    let mut offs = Vec::new();
    for (n, track) in TRACKS.iter().enumerate() {
        let (s, off) = click_track(track, 30 + n as u64, n);
        offs.push(format!("track {n}: {off:.2} mm"));
        if off > 0.5 {
            failures.push(format!(
                "track {n}: the finished curve is {off:.2} mm off the preview before Enter"
            ));
        }
        let finished = artwork(&s.draw_list());
        let doc = saved(&s);
        let path = doc.path(doc.object_ids()[0]).unwrap();
        assert_eq!(path.curve_kind, CurveKind::Spiro, "track {n}: a Spiro");
        if let Err(e) = reopen_equals(&s, &finished, 0.01) {
            failures.push(format!("track {n}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{failures:#?}\nfinish-vs-preview: {offs:?}"
    );
}

/// Every click of the Spiro Pen is a control: a click is never dropped without a trace, even
/// where the curve through it has no near solution (each prefix of the zigzag also solves from
/// nothing in `curvyo-geometry-core`). Where the click lands is the Pen's hold to decide.
#[test]
fn spiro_every_click_places_a_control() {
    let mut failures = Vec::new();
    for (n, track) in TRACKS.iter().enumerate() {
        let (s, _) = click_track(track, 60 + n as u64, n);
        let doc = saved(&s);
        let path = doc.path(doc.object_ids()[0]).unwrap();
        if path.anchors.len() != track.len() {
            failures.push(format!(
                "track {n}: {} clicks gave {} controls {:?}",
                track.len(),
                path.anchors.len(),
                path.anchors
                    .iter()
                    .map(|a| (a.point.x, a.point.y))
                    .collect::<Vec<_>>()
            ));
        }
    }
    let zigzag: Vec<_> = TRACKS[1]
        .iter()
        .map(|&point| curvyo_geometry_core::SpiroControl {
            point,
            corner: false,
        })
        .collect();
    for k in 2..=zigzag.len() {
        assert!(
            curvyo_geometry_core::solve_spiro(&zigzag[..k], false).is_ok(),
            "setup: the first {k} zigzag clicks solve"
        );
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const TRACKS: [&[Point]; 4] = [
    // An easy wave: the baseline.
    &[
        Point::new(10.0, 20.0),
        Point::new(30.0, 10.0),
        Point::new(50.0, 20.0),
        Point::new(70.0, 10.0),
        Point::new(90.0, 20.0),
    ],
    // A tight zigzag.
    &[
        Point::new(10.0, 10.0),
        Point::new(14.0, 30.0),
        Point::new(18.0, 10.0),
        Point::new(22.0, 30.0),
        Point::new(26.0, 10.0),
        Point::new(30.0, 30.0),
    ],
    // An inward spiral.
    &[
        Point::new(50.0, 50.0),
        Point::new(90.0, 50.0),
        Point::new(90.0, 90.0),
        Point::new(55.0, 90.0),
        Point::new(55.0, 58.0),
        Point::new(82.0, 58.0),
        Point::new(82.0, 82.0),
        Point::new(64.0, 82.0),
        Point::new(64.0, 66.0),
    ],
    // A hairpin: the third point almost on the first segment.
    &[
        Point::new(10.0, 10.0),
        Point::new(50.0, 10.0),
        Point::new(20.0, 10.5),
        Point::new(60.0, 30.0),
    ],
];

/// Clicks `track` with the Spiro Pen, the pointer walking between the clicks and no notice in
/// any frame, then Enter. Returns the session and how far (mm) the finished artwork lies off
/// the preview on screen when Enter was pressed (the overlay stroke against the artwork stroke).
fn click_track(track: &[Point], peer: u64, n: usize) -> (Session, f64) {
    let mut s = pen(PenMode::Spiro, peer);
    let mut from = track[0];
    for &p in track {
        for q in walk(&[from, p]) {
            hover(&mut s, q);
            assert_no_notice(&mut s, &format!("track {n} {q:?}"));
        }
        click(&mut s, p);
        from = p;
    }
    let before_enter = hover(&mut s, *track.last().unwrap()).triangles;
    s.finish_pen();
    let finished = artwork(&s.draw_list());
    assert!(!finished.is_empty(), "track {n}: committed");
    let off = reach(&finished, &before_enter);
    (s, off)
}

/// Selects the only path and drags its control at `from` along `path` with the Node tool,
/// checking every frame; returns the session after the release and the worst move.
/// The bounding box of a drawing as (min x, min y, max x, max y), rounded to 0.1 mm.
fn bbox(v: &[Vertex]) -> [f64; 4] {
    let b = v
        .iter()
        .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, q| {
            [
                b[0].min(q.position.x),
                b[1].min(q.position.y),
                b[2].max(q.position.x),
                b[3].max(q.position.y),
            ]
        });
    b.map(|x| (x * 10.0).round() / 10.0)
}

/// The largest frame-to-frame move of a drag: how far, at which pointer, and the bounding boxes
/// of the drawing before and after.
struct Jump {
    moved: f64,
    at: Point,
    boxes: ([f64; 4], [f64; 4]),
    /// Events at which the drawing moved more than [`JUMP_LIMIT`] with the pointer more than
    /// 2 mm from the dragged point's neighbour (Node drags only).
    over: usize,
    /// Events at which the drawing passes no closer than 1 mm to the pointer (edge samples are
    /// 1 mm apart): the dragged point stayed behind (Node drags only, `specs/0115`: never).
    stalls: usize,
    /// Events at which the drawing moved more than 4 x the step + 2 mm, and of all events.
    over_spec: usize,
    events: usize,
}

fn node_drag(
    doc_session: &Session,
    select_at: Point,
    from: Point,
    path: &[Point],
) -> (Session, Jump, Vec<Vertex>) {
    let bytes = doc_session.pack("test").unwrap();
    let mut s = Session::open(50, &bytes).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    click(&mut s, select_at);
    assert_eq!(s.selected_object_count(), 1, "setup: selected");
    s.set_tool(Tool::Node);
    let _ = s.draw_list();
    s.pointer_down(from, false);
    let mut last = artwork(&s.draw_list());
    let mut worst = Jump {
        moved: 0.0,
        at: from,
        boxes: ([0.0; 4], [0.0; 4]),
        over: 0,
        stalls: 0,
        over_spec: 0,
        events: 0,
    };
    let (mut stalls, mut over_spec, mut events) = (0, 0, 0);
    for p in walk(path) {
        let frame = artwork(&hover(&mut s, p));
        assert_no_notice(&mut s, &format!("node drag {p:?}"));
        assert!(!frame.is_empty(), "node drag {p:?}: the curve is shown");
        let moved = distance(&last, &frame);
        // Within 2 mm of the neighbour (`select_at`) the segment between them nearly vanishes
        // and the curve has no direction to keep there.
        let clear = (p.x - select_at.x).hypot(p.y - select_at.y) > 2.0;
        let over = worst.over + usize::from(clear && moved > JUMP_LIMIT);
        let near = dense(&frame)
            .iter()
            .map(|v| (v.position.x - p.x).hypot(v.position.y - p.y))
            .fold(f64::INFINITY, f64::min);
        stalls += usize::from(near > 1.0);
        over_spec += usize::from(moved > 4.0 * STEP + 2.0);
        events += 1;
        if moved > worst.moved {
            worst = Jump {
                moved,
                at: p,
                boxes: (bbox(&last), bbox(&frame)),
                over,
                stalls: 0,
                over_spec: 0,
                events: 0,
            };
        }
        worst.over = over;
        last = frame;
    }
    worst.stalls = stalls;
    worst.over_spec = over_spec;
    worst.events = events;
    let end = *path.last().unwrap();
    s.pointer_up(end, false, false);
    (s, worst, last)
}

fn spiro_four() -> (Session, [Point; 4]) {
    let points = [
        Point::new(10.0, 20.0),
        Point::new(30.0, 10.0),
        Point::new(50.0, 20.0),
        Point::new(70.0, 10.0),
    ];
    let mut s = spiro_pen_with(&points, 40);
    s.finish_pen();
    (s, points)
}

/// A Node drag far out and back never jumps; onto and round its neighbour the curve turns over
/// at no more than two events away from the neighbour (`specs/0115`); every release stores and
/// reopens what was shown.
#[test]
fn spiro_node_drag_across_hard_places_is_stored_and_reopened_as_shown() {
    let (s, points) = spiro_four();
    let tracks: [&[Point]; 3] = [
        // Far out and back across the line through its neighbours.
        &[
            points[2],
            Point::new(50.0, 70.0),
            Point::new(50.0, -30.0),
            Point::new(50.0, 20.0),
        ],
        // Onto its neighbour and past it.
        &[points[2], Point::new(31.0, 10.5), Point::new(5.0, 5.0)],
        // Round its neighbour.
        &circle(points[1], 12.0, 0.0, 360.0),
    ];
    let mut failures = Vec::new();
    for (n, track) in tracks.iter().enumerate() {
        let mut track = track.to_vec();
        if n == 2 {
            track.insert(0, points[2]);
        }
        let (r, jump, shown) = node_drag(&s, points[1], points[2], &track);
        println!(
            "track {n}: worst {:.1} mm at ({:.1}, {:.1}), {} over {JUMP_LIMIT} mm clear of the neighbour, {} of {} events over 4 mm, {} stalls",
            jump.moved, jump.at.x, jump.at.y, jump.over, jump.over_spec, jump.events, jump.stalls
        );
        // `specs/0115` criterion 2: the dragged point is under the pointer at every event.
        if jump.stalls > 0 {
            failures.push(format!("track {n}: {} stalls", jump.stalls));
        }
        // Onto and round the neighbour the curve turns over at once (`specs/0115`): away from
        // the neighbour it may move more than the jump limit at two events at most; the track
        // that stays clear of the neighbour is held to the limit.
        if n > 0 && jump.over > 2 {
            failures.push(format!(
                "track {n}: the curve moved more than {JUMP_LIMIT} mm at {} events",
                jump.over
            ));
        }
        if n == 0 && jump.moved > JUMP_LIMIT {
            failures.push(format!(
                "track {n}: the curve jumped {:.1} mm with the pointer at ({:.1}, {:.1}), box {:?} -> {:?}",
                jump.moved, jump.at.x, jump.at.y, jump.boxes.0, jump.boxes.1
            ));
        }
        let released = artwork(&r.draw_list());
        let d = distance(&shown, &released);
        if d > 0.01 {
            failures.push(format!(
                "track {n}: the release changes the shown curve by {d:.3} mm"
            ));
        }
        let doc = saved(&r);
        let path = doc.path(doc.object_ids()[0]).unwrap();
        assert_eq!(path.curve_kind, CurveKind::Spiro, "track {n}");
        assert_eq!(path.anchors.len(), 4, "track {n}");
        if let Err(e) = reopen_equals(&r, &released, 0.01) {
            failures.push(format!("track {n}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
