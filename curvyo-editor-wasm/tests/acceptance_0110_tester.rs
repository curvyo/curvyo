//! Tester acceptance for `specs/0110-pen-modes-variable-stroke` criteria 2 to 6 and 8 at the
//! session, plus the two open points of the lead's brief (one width point; Select handles over
//! diamonds). Written from the specification and ADR 0053 before reading the implementation.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::manual_midpoint
)]

use curvyo_document_core::{
    AnchorId, AnchorKind, Color, CurveKind, Document, DocumentSize, Length, NewAnchor,
    NewOperation, NewOutputPage, NodeId, PathSnapshot, Point, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{
    CombineOutcome, OperationsViewMode, Session, StrokeToPathOutcome, SvgExportResult, Tool,
};
use curvyo_render_core::{DrawList, OP_CUT};
use curvyo_ui_core::{PenMode, SvgExportOptions, WidthPreset};

// ---------- helpers ----------

/// The artwork part of a draw list (before the overlay of handles and diamonds).
fn artwork(list: &DrawList) -> &[curvyo_render_core::Vertex] {
    let end = list.layers().last().copied().unwrap_or(0);
    &list.triangles[..end]
}

/// Whether a triangle of the artwork covers `p` (document mm).
fn covers(tris: &[curvyo_render_core::Vertex], p: (f64, f64)) -> bool {
    tris.as_chunks::<3>().0.iter().any(|t| {
        let (a, b, c) = (t[0].position, t[1].position, t[2].position);
        let s1 = (b.x - a.x) * (p.1 - a.y) - (b.y - a.y) * (p.0 - a.x);
        let s2 = (c.x - b.x) * (p.1 - b.y) - (c.y - b.y) * (p.0 - b.x);
        let s3 = (a.x - c.x) * (p.1 - c.y) - (a.y - c.y) * (p.0 - c.x);
        (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
    })
}

/// Whether any artwork triangle covers a point of a 0.1 mm grid in the box (document mm).
fn ink_in(list: &DrawList, x: (f64, f64), y: (f64, f64)) -> bool {
    tris_in(artwork(list), x, y)
}

/// Whether any of `tris` covers a point of a 0.1 mm grid in the box.
fn tris_in(tris: &[curvyo_render_core::Vertex], x: (f64, f64), y: (f64, f64)) -> bool {
    let mut px = x.0;
    while px <= x.1 {
        let mut py = y.0;
        while py <= y.1 {
            if covers(tris, (px, py)) {
                return true;
            }
            py += 0.1;
        }
        px += 0.1;
    }
    false
}

/// How many points of a 0.05 mm grid over the box the artwork covers.
fn coverage(list: &DrawList, x: (f64, f64), y: (f64, f64)) -> usize {
    let tris = artwork(list);
    let mut n = 0;
    let mut px = x.0;
    while px <= x.1 {
        let mut py = y.0;
        while py <= y.1 {
            n += usize::from(covers(tris, (px, py)));
            py += 0.05;
        }
        px += 0.05;
    }
    n
}

fn saved(session: &Session) -> Document {
    unpack(900, &session.pack("test").unwrap()).unwrap()
}

fn snap(session: &Session, id: NodeId) -> PathSnapshot {
    saved(session).path(id).unwrap()
}

fn click(session: &mut Session, p: Point) {
    session.pointer_down(p, false);
    session.pointer_up(p, false, false);
}

/// A drag with several intermediate moves, as a real pointer sends them.
fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_down(from, false);
    for i in 1..=6 {
        let t = f64::from(i) / 6.0;
        session.pointer_hover(
            Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t),
            false,
            false,
        );
        let _ = session.draw_list();
    }
    session.pointer_up(to, false, false);
}

/// A straight path from (10, 10) to (50, 10) with a 1 mm stroke in `color`.
fn line_doc(peer: u64) -> (Document, NodeId) {
    let doc = Document::new(peer);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(peer, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(peer, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    doc.edit_style(
        &[id],
        &StyleEdit::StrokeColor(Color {
            r: 200,
            g: 10,
            b: 0,
        }),
    )
    .unwrap();
    (doc, id)
}

fn open_selected(doc: &Document, at: Point) -> Session {
    let mut s = Session::open(doc_peer(doc), &pack(doc, "test").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    click(&mut s, at);
    assert_eq!(s.selected_object_count(), 1, "setup: the path is selected");
    s
}

fn doc_peer(_doc: &Document) -> u64 {
    777
}

// ---------- criterion 2 ----------

fn draw_curve(mode: PenMode, points: &[Point]) -> Session {
    let mut s = Session::new(31);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(mode));
    for &p in points {
        click(&mut s, p);
    }
    s.finish_pen();
    s
}

const BSPLINE_POINTS: [Point; 4] = [
    Point::new(10.0, 10.0),
    Point::new(30.0, 10.0),
    Point::new(30.0, 30.0),
    Point::new(10.0, 30.0),
];

/// Criterion 2: a B-spline is drawn live while points are placed, off its control polygon;
/// the finished path is drawn, hit, selected and exported as that curve, and after save and
/// reopen (empty caches) its controls are nodes that stay editable.
#[test]
fn c2_bspline_live_finished_hit_reopened_and_editable() {
    // The single cubic span's midpoint is (P0 + 3 P1 + 3 P2 + P3) / 8 = (25, 20), 5 mm from
    // the control polygon: only the curve puts ink there.
    let on_curve = ((24.0, 26.0), (19.0, 21.0));
    let mut s = Session::new(30);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::BSpline));
    for p in &BSPLINE_POINTS[..3] {
        click(&mut s, *p);
    }
    // The pointer is the next control: the preview is the curve the click will commit.
    s.pointer_hover(BSPLINE_POINTS[3], false, false);
    // The unfinished path is drawn in the overlay: look at every triangle.
    assert!(
        tris_in(&s.draw_list().triangles, on_curve.0, on_curve.1),
        "live"
    );
    click(&mut s, BSPLINE_POINTS[3]);
    s.finish_pen();
    assert!(ink_in(&s.draw_list(), on_curve.0, on_curve.1), "finished");
    let before_save = artwork(&s.draw_list()).to_vec();

    let bytes = s.pack("test").unwrap();
    let mut r = Session::open(32, &bytes).unwrap();
    let doc = unpack(33, &bytes).unwrap();
    let id = doc.object_ids()[0];
    let path = doc.path(id).unwrap();
    assert_eq!(path.curve_kind, CurveKind::BSpline);
    assert_eq!(path.anchors.len(), 4, "the controls are stored");
    assert_eq!(
        artwork(&r.draw_list()),
        &before_save[..],
        "reopened drawing equals the drawing before save"
    );
    r.set_tool(Tool::Select);
    click(&mut r, Point::new(30.0, 20.0));
    assert_eq!(
        r.selected_object_count(),
        0,
        "the control polygon is not the hit shape"
    );
    click(&mut r, Point::new(25.0, 20.0));
    assert_eq!(
        r.selected_object_count(),
        1,
        "a click on the curve selects it"
    );
    let SvgExportResult::Written { text, .. } = r.export_svg(SvgExportOptions::default()) else {
        panic!("export")
    };
    assert!(text.contains('C'), "exported as curves: {text}");

    // Node edit after reopen keeps the kind and moves the control.
    click(&mut r, Point::new(25.0, 20.0));
    r.set_tool(Tool::Node);
    drag(&mut r, Point::new(30.0, 10.0), Point::new(36.0, 4.0));
    let edited = snap(&r, id);
    assert_eq!(edited.curve_kind, CurveKind::BSpline);
    assert_eq!(edited.anchors.len(), 4);
    assert_eq!(edited.anchors[1].point, Point::new(36.0, 4.0));
    let again = unpack(34, &r.pack("test").unwrap()).unwrap();
    assert_eq!(again.path(id).unwrap().curve_kind, CurveKind::BSpline);
}

/// Criterion 2: the same for Spiro: live, finished, reopened identical, node-editable.
#[test]
fn c2_spiro_live_finished_reopened_and_editable() {
    let points = [
        Point::new(10.0, 20.0),
        Point::new(30.0, 10.0),
        Point::new(50.0, 20.0),
        Point::new(70.0, 10.0),
    ];
    let lines = artwork(&draw_curve(PenMode::Lines, &points).draw_list()).to_vec();
    let mut s = Session::new(40);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Spiro));
    for p in points {
        click(&mut s, p);
    }
    s.pointer_hover(points[3], false, false);
    let live = s.draw_list().triangles;
    let lines_live = {
        let mut l = Session::new(39);
        l.set_tool(Tool::Pen);
        assert!(l.set_pen_mode(PenMode::Lines));
        for p in points {
            click(&mut l, p);
        }
        l.pointer_hover(points[3], false, false);
        l.draw_list().triangles
    };
    assert!(!live.is_empty(), "live");
    assert_ne!(live, lines_live, "the live Spiro is not its polygon");
    s.finish_pen();
    let finished = artwork(&s.draw_list()).to_vec();
    assert!(!finished.is_empty());
    assert_ne!(finished, lines, "a Spiro is not its polygon");

    let bytes = s.pack("test").unwrap();
    let mut r = Session::open(41, &bytes).unwrap();
    let doc = unpack(42, &bytes).unwrap();
    let id = doc.object_ids()[0];
    assert_eq!(doc.path(id).unwrap().curve_kind, CurveKind::Spiro);
    assert_eq!(doc.path(id).unwrap().anchors.len(), 4);
    assert_eq!(
        artwork(&r.draw_list()),
        &finished[..],
        "reopen draws the same"
    );

    r.set_tool(Tool::Select);
    click(&mut r, points[1]);
    assert_eq!(r.selected_object_count(), 1);
    r.set_tool(Tool::Node);
    drag(&mut r, points[2], Point::new(50.0, 28.0));
    let edited = snap(&r, id);
    assert_eq!(edited.curve_kind, CurveKind::Spiro);
    assert_eq!(edited.anchors.len(), 4);
    assert_eq!(
        edited.anchors[2].point,
        Point::new(50.0, 28.0),
        "a Node drag moves the Spiro control"
    );
    let shown = r.draw_list();
    assert!(
        ink_in(&shown, (48.0, 52.0), (26.0, 30.0)),
        "the curve follows the moved node"
    );
}

// ---------- criterion 3 ----------

#[test]
fn c3_lines_and_bezier_click_paths_still_commit_plain_paths() {
    for mode in [PenMode::Lines, PenMode::Bezier] {
        let s = draw_curve(mode, &BSPLINE_POINTS);
        let doc = saved(&s);
        let ids = doc.object_ids();
        assert_eq!(ids.len(), 1, "{mode:?}");
        let path = doc.path(ids[0]).unwrap();
        assert_eq!(path.curve_kind, CurveKind::Bezier, "{mode:?}");
        assert_eq!(path.anchors.len(), 4, "{mode:?}");
        assert!(path.width_points.is_empty());
    }
}

#[test]
fn c1_bezier_is_the_default_pen_mode() {
    let mut s = Session::new(50);
    s.set_tool(Tool::Pen);
    assert_eq!(s.pen_mode(), PenMode::Bezier);
}

// ---------- criterion 4 ----------

fn width_session(peer: u64, point: Option<(f64, f64)>) -> (Session, NodeId) {
    let (doc, id) = line_doc(peer);
    if let Some((pos, m)) = point {
        doc.set_width_point(id, AnchorId::new(peer, 9), pos, m)
            .unwrap();
    }
    let mut s = open_selected(&doc, Point::new(30.0, 10.0));
    assert!(s.begin_width_edit());
    assert!(s.width_edit_active());
    (s, id)
}

#[test]
fn c4_click_on_the_line_adds_a_point_in_one_undo_step() {
    let (mut s, id) = width_session(60, None);
    let before = snap(&s, id);
    click(&mut s, Point::new(20.0, 10.0));
    let after = snap(&s, id);
    assert_eq!(after.width_points.len(), 1);
    assert!((after.width_points[0].position - 0.25).abs() < 0.01);
    assert_eq!(after.anchors, before.anchors);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before, "one undo removes it");
    let _ = s.redo();
    assert_eq!(snap(&s, id), after);
}

#[test]
fn c4_sideways_drag_sets_the_width_in_document_units_in_one_undo_step() {
    let (mut s, id) = width_session(61, Some((0.5, 1.0)));
    let before = snap(&s, id);
    // Amended by the implementer for UX review U3 of 2026-11-10 (the drag is relative to the
    // press; the tester re-confirms): 1 mm at the press plus twice the 2 mm moved is 5 mm,
    // multiplier 5 on a 1 mm base.
    drag(&mut s, Point::new(30.0, 10.0), Point::new(30.0, 12.0));
    let after = snap(&s, id);
    assert_eq!(after.width_points.len(), 1);
    assert!(
        (after.width_points[0].multiplier - 5.0).abs() < 1e-6,
        "{:?}",
        after.width_points
    );
    assert!((after.width_points[0].position - 0.5).abs() < 1e-9);
    assert_eq!(after.anchors, before.anchors, "the centre line stays");
    assert!((s.width_point_mm().unwrap() - 5.0).abs() < 1e-6);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before, "one undo undoes the whole drag");
    let _ = s.redo();
    assert_eq!(snap(&s, id), after);
}

#[test]
fn c4_width_follows_document_units_when_zoomed() {
    let (mut s, id) = width_session(62, Some((0.5, 1.0)));
    s.resize_viewport(800.0, 600.0);
    assert!(s.restore_view_text("4 0 0"));
    let _ = s.draw_list();
    // Amended for UX review U3 (relative drag): 1 mm + 2 × 1 mm = 3 mm in document units.
    drag(&mut s, Point::new(30.0, 10.0), Point::new(30.0, 11.0));
    let after = snap(&s, id);
    assert!(
        (after.width_points[0].multiplier - 3.0).abs() < 1e-6,
        "{:?}",
        after.width_points
    );
}

#[test]
fn c4_drag_along_moves_the_point_and_keeps_its_width() {
    let (mut s, id) = width_session(63, Some((0.5, 1.0)));
    let before = snap(&s, id);
    drag(&mut s, Point::new(30.0, 10.0), Point::new(40.0, 10.0));
    let after = snap(&s, id);
    assert_eq!(after.width_points.len(), 1);
    assert!((after.width_points[0].position - 0.75).abs() < 0.01);
    assert_eq!(after.width_points[0].multiplier, 1.0);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before);
}

#[test]
fn c4_delete_removes_the_point_escape_and_done_leave_the_mode() {
    let (mut s, id) = width_session(64, Some((0.5, 2.0)));
    click(&mut s, Point::new(30.0, 10.0));
    s.delete_selected();
    let after = snap(&s, id);
    assert!(after.width_points.is_empty(), "Delete removes the point");
    assert_eq!(saved(&s).object_ids().len(), 1, "not the path");
    let _ = s.undo();
    assert_eq!(snap(&s, id).width_points.len(), 1);
    s.escape();
    assert!(!s.width_edit_active(), "Escape leaves");
    assert!(s.begin_width_edit());
    s.end_width_edit();
    assert!(!s.width_edit_active(), "Done leaves");
}

// ---------- criterion 5 ----------

#[test]
fn c5_each_profile_is_one_undo_step_and_keeps_the_centre_line() {
    let (doc, id) = line_doc(70);
    doc.set_width_point(id, AnchorId::new(70, 9), 0.3, 1.7)
        .unwrap();
    let mut s = open_selected(&doc, Point::new(30.0, 10.0));
    for (preset, start, mid, end) in [
        (WidthPreset::TaperStart, 0.0, 0.5, 1.0),
        (WidthPreset::TaperEnd, 1.0, 0.5, 0.0),
        (WidthPreset::TaperBoth, 0.0, 1.0, 0.0),
        (WidthPreset::Uniform, 1.0, 1.0, 1.0),
    ] {
        let before = snap(&s, id);
        s.apply_width_preset(preset).unwrap();
        let after = snap(&s, id);
        assert_eq!(after.anchors, before.anchors, "{preset:?}");
        assert_eq!(after.curve_kind, before.curve_kind);
        let profile = curvyo_geometry_core::WidthProfile::new(
            after
                .width_points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    curvyo_geometry_core::WidthPoint::new(
                        i as u128,
                        p.position,
                        p.multiplier,
                        false,
                    )
                    .unwrap()
                })
                .collect(),
            false,
        )
        .unwrap();
        for (at, want) in [(0.0, start), (1.0, end)] {
            assert!(
                (profile.sample(at).unwrap() - want).abs() < 1e-9,
                "{preset:?} at {at}"
            );
        }
        if preset != WidthPreset::TaperStart && preset != WidthPreset::TaperEnd {
            assert!(
                (profile.sample(0.5).unwrap() - mid).abs() < 1e-9,
                "{preset:?}"
            );
        }
        let _ = s.undo();
        assert_eq!(snap(&s, id), before, "{preset:?}: one undo step");
        let _ = s.redo();
        assert_eq!(snap(&s, id), after);
    }
    assert!(snap(&s, id).width_points.is_empty(), "Uniform clears");
}

#[test]
fn c5_numeric_field_sets_the_selected_point_in_mm_in_one_undo_step() {
    let (mut s, id) = width_session(71, Some((0.5, 1.0)));
    click(&mut s, Point::new(30.0, 10.0));
    assert!((s.width_point_mm().unwrap() - 1.0).abs() < 1e-9);
    let before = snap(&s, id);
    assert!(s.set_width_point_mm(3.0));
    assert!((snap(&s, id).width_points[0].multiplier - 3.0).abs() < 1e-9);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before);
    assert!(!s.set_width_point_mm(-1.0), "a negative width is refused");
    assert!(!s.set_width_point_mm(f64::NAN));
    assert_eq!(snap(&s, id), before);
}

// ---------- criterion 6 ----------

/// A 1 mm stroke whose middle is 8 mm wide.
fn bulge_doc(peer: u64) -> (Document, NodeId) {
    let (doc, id) = line_doc(peer);
    doc.set_width_point(id, AnchorId::new(peer, 9), 0.0, 1.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(peer, 10), 0.5, 8.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(peer, 11), 1.0, 1.0)
        .unwrap();
    (doc, id)
}

#[test]
fn c6_drawing_and_hit_testing_follow_the_visible_outline() {
    let (doc, _) = bulge_doc(80);
    let mut s = Session::open(81, &pack(&doc, "test").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    let list = s.draw_list();
    assert!(
        ink_in(&list, (29.0, 31.0), (13.0, 13.9)),
        "the wide middle is drawn"
    );
    assert!(
        !ink_in(&list, (29.0, 31.0), (14.3, 20.0)),
        "but not beyond it"
    );
    assert!(
        !ink_in(&list, (10.5, 11.5), (12.0, 20.0)),
        "the thin end is thin"
    );
    click(&mut s, Point::new(30.0, 13.5));
    assert_eq!(
        s.selected_object_count(),
        1,
        "a click in the wide part hits"
    );

    // Control: the same click misses a plain 1 mm stroke.
    let (plain, _) = line_doc(82);
    let mut p = Session::open(83, &pack(&plain, "test").unwrap()).unwrap();
    p.set_tool(Tool::Select);
    let _ = p.draw_list();
    click(&mut p, Point::new(30.0, 13.5));
    assert_eq!(p.selected_object_count(), 0, "control");
}

#[test]
fn c6_svg_export_of_a_tapered_stroke_is_a_filled_outline_in_the_stroke_paint() {
    let (doc, id) = line_doc(84);
    let mut s = open_selected(&doc, Point::new(30.0, 10.0));
    s.apply_width_preset(WidthPreset::TaperBoth).unwrap();
    let before = snap(&s, id);
    let SvgExportResult::Written { text, objects, .. } = s.export_svg(SvgExportOptions::default())
    else {
        panic!("export")
    };
    assert_eq!(objects, 1);
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("fill=\"#c80a00\"") || lower.contains("fill:#c80a00"),
        "the outline is filled in the stroke colour: {text}"
    );
    assert!(
        !lower.contains("stroke=\"#c80a00\"") && !lower.contains("stroke:#c80a00"),
        "no uniform stroke stands in for the taper: {text}"
    );
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    assert_eq!(snap(&s, id), before, "export changes nothing");
}

#[test]
fn c6_stroke_to_path_of_a_taper_makes_the_closed_outline_and_undoes() {
    let (doc, id) = bulge_doc(85);
    let mut s = open_selected(&doc, Point::new(30.0, 10.0));
    let before = saved(&s).read_objects().objects;
    let outcome = s.apply_stroke_to_path();
    assert!(
        matches!(outcome, StrokeToPathOutcome::Applied { .. }),
        "{outcome:?}"
    );
    let after = saved(&s);
    let outline = after
        .object_ids()
        .into_iter()
        .filter_map(|o| after.path(o))
        .find(|p| p.closed)
        .expect("a closed outline");
    assert!(outline.width_points.is_empty());
    assert_eq!(outline.style.fill.color, before_color());
    let (mut min_y, mut max_y, mut min_x, mut max_x) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for a in &outline.anchors {
        min_y = min_y.min(a.point.y);
        max_y = max_y.max(a.point.y);
        min_x = min_x.min(a.point.x);
        max_x = max_x.max(a.point.x);
    }
    assert!(
        (max_y - 14.0).abs() < 0.02 && (min_y - 6.0).abs() < 0.02,
        "{min_y} {max_y}"
    );
    assert!(min_x >= 9.4 && max_x <= 50.6, "{min_x} {max_x}");
    let _ = s.undo();
    assert_eq!(saved(&s).read_objects().objects, before);
    let _ = id;
}

fn before_color() -> Color {
    Color {
        r: 200,
        g: 10,
        b: 0,
    }
}

/// Criterion 6 (OUTPUT A): machine output keeps the centre line. There is no job writer on the
/// branch; the laser Operations look is what the session derives from operation members, so it
/// is checked as a proxy: the cut line stays at the centre while the Drawn view shows the width.
#[test]
fn c6_operations_look_keeps_the_centre_line_proxy_for_machine_output() {
    let d = Document::new(86);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let o = laser.origin;
    let id = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(86, 1), Point::new(o.x + 10.0, o.y + 50.0)),
            NewAnchor::corner(AnchorId::new(86, 2), Point::new(o.x + 90.0, o.y + 50.0)),
        ],
        false,
    );
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    d.set_width_point(id, AnchorId::new(86, 3), 0.5, 20.0)
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    d.add_operation(
        m,
        &NewOperation {
            kind: "cut",
            name: "Op 1",
        },
        0,
        &[vec![id]],
    )
    .unwrap();
    let bytes = pack(&d, "t").unwrap();
    let open = |mode| {
        let mut s = Session::open(87, &bytes).unwrap();
        s.resize_viewport(1600.0, 1000.0);
        s.set_tool(Tool::Select);
        assert!(s.activate_page(&laser.id));
        s.set_operations_view_mode(mode);
        s.draw_list()
    };
    let far = (o.y + 56.0, o.y + 59.5);
    let x = (o.x + 45.0, o.x + 55.0);
    let looked = open(OperationsViewMode::Operations);
    let cut: Vec<_> = artwork(&looked)
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| t[0].color == OP_CUT)
        .flatten()
        .copied()
        .collect();
    let cut_far = tris_in(&cut, x, far);
    let cut_centre = tris_in(&cut, x, (o.y + 49.6, o.y + 50.4));
    assert!(cut_centre, "the cut line is on the centre line");
    assert!(!cut_far, "the cut look does not follow the 20 mm width");
    let drawn = open(OperationsViewMode::Drawn);
    assert!(
        ink_in(&drawn, x, far),
        "control: the drawn width reaches there"
    );
}

/// Criterion 6: operations that cannot keep curve data or a profile refuse and change nothing.
#[test]
fn c6_combine_and_union_refuse_for_a_profile_or_curve_and_change_nothing() {
    for extended in ["profile", "spiro", "bspline"] {
        let doc = Document::new(88);
        let plain = doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(88, 1), Point::new(10.0, 40.0)),
                NewAnchor::corner(AnchorId::new(88, 2), Point::new(40.0, 40.0)),
                NewAnchor::corner(AnchorId::new(88, 3), Point::new(40.0, 60.0)),
            ],
            true,
        );
        let _ = plain;
        let mut controls: Vec<_> = [(20.0, 45.0), (35.0, 55.0), (50.0, 45.0), (30.0, 35.0)]
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let mut a = NewAnchor::corner(AnchorId::new(88, 10 + i as u64), Point::new(x, y));
                a.kind = AnchorKind::Symmetric;
                a
            })
            .collect();
        match extended {
            "profile" => {
                for a in &mut controls {
                    a.kind = AnchorKind::Corner;
                }
                let id = doc.create_path(&controls, true);
                doc.set_width_point(id, AnchorId::new(88, 30), 0.2, 3.0)
                    .unwrap();
            }
            "spiro" => {
                doc.create_editable_curve(&controls, true, CurveKind::Spiro)
                    .unwrap();
            }
            _ => {
                doc.create_editable_curve(&controls, true, CurveKind::BSpline)
                    .unwrap();
            }
        }
        let bytes = pack(&doc, "t").unwrap();
        let mut s = Session::open(89, &bytes).unwrap();
        s.set_tool(Tool::Select);
        let _ = s.draw_list();
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "a",
            ctrl: true,
            ..Default::default()
        });
        assert_eq!(s.selected_object_count(), 2);
        let before = saved(&s).read_objects().objects;
        let combine = s.apply_combine();
        assert!(
            !matches!(combine, CombineOutcome::Applied { .. }),
            "{extended}: combine {combine:?}"
        );
        assert_eq!(saved(&s).read_objects().objects, before, "{extended}");
        let union = s.apply_boolean(curvyo_ui_core::BooleanOp::Union);
        assert!(
            !matches!(union, curvyo_editor_wasm::BooleanOutcome::Applied { .. }),
            "{extended}: union {union:?}"
        );
        assert_eq!(saved(&s).read_objects().objects, before, "{extended}");
        let joined = s.join_selected();
        assert!(
            joined.is_none() || joined == Some(0),
            "{extended}: join {joined:?}"
        );
        assert_eq!(saved(&s).read_objects().objects, before, "{extended}");
    }
}

// ---------- criterion 8 ----------

/// Criterion 8: an ordinary stroke is unchanged; a profile of uniform 1 draws exactly as it.
#[test]
fn c8_uniform_one_profile_draws_exactly_as_the_ordinary_stroke() {
    let (plain, _) = line_doc(90);
    let (with, id) = line_doc(90);
    with.set_width_point(id, AnchorId::new(90, 9), 0.4, 1.0)
        .unwrap();
    let a = Session::open(91, &pack(&plain, "t").unwrap()).unwrap();
    let b = Session::open(91, &pack(&with, "t").unwrap()).unwrap();
    let (ca, cb) = (
        coverage(&a.draw_list(), (8.0, 52.0), (8.0, 12.0)),
        coverage(&b.draw_list(), (8.0, 52.0), (8.0, 12.0)),
    );
    assert!(ca > 0);
    // Within the 0.01 mm kernel tolerance: at most one grid row of 0.05 mm differs.
    assert!(ca.abs_diff(cb) <= 900, "{ca} vs {cb}");
}

/// Open point: one width point makes the whole stroke uniform at its multiplier (ADR 0053).
/// Recorded as behaviour; whether the first click should do this is a UX question.
#[test]
fn open_point_one_width_point_makes_the_whole_stroke_uniform() {
    let (one, id) = line_doc(92);
    one.set_width_point(id, AnchorId::new(92, 9), 0.25, 3.0)
        .unwrap();
    let (thick, tid) = line_doc(92);
    thick
        .edit_style(&[tid], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let a = Session::open(93, &pack(&one, "t").unwrap()).unwrap();
    let b = Session::open(93, &pack(&thick, "t").unwrap()).unwrap();
    let la = a.draw_list();
    assert!(
        ink_in(&la, (45.0, 49.0), (11.3, 11.5)),
        "far from the point it is 3 mm too"
    );
    let (ca, cb) = (
        coverage(&la, (8.0, 52.0), (7.0, 13.0)),
        coverage(&b.draw_list(), (8.0, 52.0), (7.0, 13.0)),
    );
    assert!(ca.abs_diff(cb) <= 900, "{ca} vs {cb}");
}

// ---------- open point: Select handles over diamonds ----------

/// A diamond at an endpoint coincides with the selection box's handle; in width edit a
/// sideways drag there changes the width and never scales or moves the path.
#[test]
fn open_point_diamond_wins_over_a_coinciding_select_handle() {
    for (end, sideways) in [
        // Horizontal line: the middle-left handle sits on the start point.
        (Point::new(10.0, 10.0), Point::new(10.0, 12.0)),
        // Diagonal line: the top-left corner handle sits on the start point.
        (Point::new(10.0, 10.0), Point::new(11.414, 8.586)),
    ] {
        let doc = Document::new(94);
        let far = if sideways.x > 11.0 {
            Point::new(50.0, 50.0)
        } else {
            Point::new(50.0, 10.0)
        };
        let id = doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(94, 1), end),
                NewAnchor::corner(AnchorId::new(94, 2), far),
            ],
            false,
        );
        doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
            .unwrap();
        doc.set_width_point(id, AnchorId::new(94, 3), 0.0, 1.0)
            .unwrap();
        let mid = Point::new((end.x + far.x) / 2.0, (end.y + far.y) / 2.0);
        let mut s = Session::open(95, &pack(&doc, "t").unwrap()).unwrap();
        s.set_tool(Tool::Select);
        let _ = s.draw_list();
        click(&mut s, mid);
        assert_eq!(s.selected_object_count(), 1);
        assert!(s.begin_width_edit());
        let before = snap(&s, id);
        drag(&mut s, end, sideways);
        let after = snap(&s, id);
        assert_eq!(
            after.anchors, before.anchors,
            "{far:?}: the path is not scaled"
        );
        // Amended for UX review U3 (relative drag): 1 mm + 2 × 2 mm = 5 mm.
        assert!(
            (after.width_points[0].multiplier - 5.0).abs() < 1e-3,
            "{far:?}: the diamond took the drag: {:?}",
            after.width_points
        );
    }
}

/// In width edit a press on a Select handle away from the line does not scale the path.
#[test]
fn open_point_select_handles_do_not_scale_in_width_edit() {
    let doc = Document::new(96);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(96, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(96, 2), Point::new(50.0, 50.0)),
        ],
        false,
    );
    let mut s = open_selected(&doc, Point::new(30.0, 30.0));
    assert!(s.begin_width_edit());
    let before = snap(&s, id);
    // Bottom-left corner of the box (10, 50) and top-right (50, 10) are off the line.
    drag(&mut s, Point::new(50.0, 10.0), Point::new(60.0, 0.0));
    drag(&mut s, Point::new(10.0, 50.0), Point::new(0.0, 60.0));
    assert_eq!(snap(&s, id).anchors, before.anchors);
}

// ---------- criterion 9: the profile under transforms (ADR 0053 implementation notes) ----------

#[test]
fn c9_reflection_keeps_positions_multipliers_and_traversal_in_one_undo_step() {
    let doc = Document::new(97);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(97, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(97, 2), Point::new(50.0, 30.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(97, 3), 0.2, 3.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(97, 4), 0.9, 0.5)
        .unwrap();
    let mut s = open_selected(&doc, Point::new(30.0, 20.0));
    let before = snap(&s, id);
    let outcome = s.flip_selection(curvyo_ui_core::FlipAxis::Horizontal);
    let after = snap(&s, id);
    assert!(
        matches!(outcome, curvyo_ui_core::FlipOutcome::Flipped),
        "{outcome:?}"
    );
    assert_eq!(after.width_points, before.width_points);
    // Flipped about the visible box (its wide start shifts the centre): the start is still the
    // first anchor, now on the right.
    assert!(
        after.anchors[0].point.x > after.anchors[1].point.x,
        "start stays first"
    );
    assert!(((after.anchors[0].point.x - after.anchors[1].point.x) - 40.0).abs() < 1e-9);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before);
}

/// A nonuniform scale by the SE handle: ADR 0053 asks to preserve the visible result or refuse.
/// Recorded: the multipliers never change, and nothing half-applies.
#[test]
fn c9_nonuniform_scale_keeps_multipliers_or_refuses() {
    let doc = Document::new(98);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(98, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(98, 2), Point::new(50.0, 10.0)),
            NewAnchor::corner(AnchorId::new(98, 3), Point::new(30.0, 40.0)),
        ],
        true,
    );
    doc.set_width_point(id, AnchorId::new(98, 4), 0.1, 3.0)
        .unwrap();
    let mut s = open_selected(&doc, Point::new(30.0, 10.0));
    let before = snap(&s, id);
    s.pointer_hover(Point::new(50.0, 40.0), false, false);
    drag(&mut s, Point::new(50.0, 40.0), Point::new(70.0, 40.0));
    let after = snap(&s, id);
    let notice = s.take_transform_notice();
    eprintln!(
        "nonuniform scale: anchors changed {}, notice {notice:?}",
        after.anchors != before.anchors
    );
    assert_eq!(after.width_points, before.width_points);
    // ADR 0053: until a nonuniform scale keeps the whole outline, it is refused and changes
    // nothing (tightened on re-verification, 2026-11-11).
    assert_eq!(after, before, "the refused resize changes nothing");
    assert!(!notice.is_empty(), "a refusal says why");
}

/// Re-verification of the relative width drag (UX review U3): moving back past the press
/// shrinks the width, never below zero, and the whole drag is one undo step.
#[test]
fn c4_relative_width_drag_shrinks_and_stops_at_zero() {
    let (mut s, id) = width_session(62, Some((0.5, 2.0)));
    let before = snap(&s, id);
    // 2 mm at the press; first sideways movement outward (+y), then back 0.5 mm past the press:
    // 2 + 2 × (-0.5) = 1 mm.
    s.pointer_down(Point::new(30.0, 10.0), false);
    for y in [10.5, 11.0, 10.5, 10.0, 9.5] {
        s.pointer_hover(Point::new(30.0, y), false, false);
        let _ = s.draw_list();
    }
    s.pointer_up(Point::new(30.0, 9.5), false, false);
    let after = snap(&s, id);
    assert!(
        (after.width_points[0].multiplier - 1.0).abs() < 1e-6,
        "{:?}",
        after.width_points
    );
    assert_eq!(after.anchors, before.anchors);
    let _ = s.undo();
    assert_eq!(snap(&s, id), before, "one undo undoes the whole drag");

    // Outward first (+y), then far back past the press: clamped at zero, never negative.
    s.pointer_down(Point::new(30.0, 10.0), false);
    for y in [11.0, 8.0, 4.0] {
        s.pointer_hover(Point::new(30.0, y), false, false);
        let _ = s.draw_list();
    }
    s.pointer_up(Point::new(30.0, 4.0), false, false);
    let clamped = snap(&s, id);
    assert!(
        clamped.width_points[0].multiplier >= 0.0 && clamped.width_points[0].multiplier < 1e-9,
        "{:?}",
        clamped.width_points
    );
}
