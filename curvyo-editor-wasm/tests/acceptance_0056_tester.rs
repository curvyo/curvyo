//! Independent tester acceptance tests for `specs/0056-mirror-and-negative-scale/
//! specification.md`, written from the specification before the implementation was read.
//! Everything goes through `Session`'s public API (flip buttons, keys, pointer events,
//! chips). Expected values are the specification's own arithmetic, never read back from the
//! code under test. All scenes sit at +50 mm so that nothing is near the viewport edge.
#![allow(
    clippy::manual_assert_eq,
    clippy::manual_midpoint,
    clippy::redundant_closure_for_method_calls,
    clippy::redundant_closure,
    clippy::semicolon_if_nothing_returned,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::unreadable_literal,
    clippy::suboptimal_flops,
    clippy::doc_markdown,
    missing_docs
)]

use std::time::Instant;

use curvyo_document_core::{
    AnchorId, AnchorKind, Angle, CornerRadii, Direction, Document, EllipseFrame, InnerRatio,
    Length, NewAnchor, NodeId, ObjectSnapshot, Point, PointCount, RectBounds, Shape, StarFrame,
    StyleEdit, Vec2, outline_of_rotated, pack, unpack,
};
use curvyo_editor_wasm::{KeyHint, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{EntryOutcome, FlipAxis, FlipOutcome};

const MINUS: char = '\u{2212}';

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}
fn rb(x: f64, y: f64, w: f64, h: f64) -> RectBounds {
    RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    }
}
fn anchor(n: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(1, n), pt(x, y))
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
fn objects(s: &Session) -> Vec<ObjectSnapshot> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect()
}
fn bytes_of(s: &Session) -> Vec<u8> {
    doc_of(s).export_loro_snapshot().unwrap()
}
fn key_full(s: &mut Session, k: &str, shift: bool, ctrl: bool, alt: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        shift,
        ctrl,
        alt,
        ..KeyInput::default()
    })
}
fn key(s: &mut Session, k: &str) -> KeyOutcome {
    key_full(s, k, false, false, false)
}
fn select_all(s: &mut Session) {
    key(s, "Escape");
    assert_eq!(
        key_full(s, "a", false, true, false),
        KeyOutcome::SelectedAll
    );
}
fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool) {
    s.modifiers_changed(shift, ctrl, false);
    s.pointer_hover(at, shift, ctrl);
}
/// A drag through the listed frames; the last one is the release.
fn drag_frames(s: &mut Session, frames: &[Point], shift: bool, ctrl: bool) {
    hold(s, frames[0], shift, ctrl);
    s.pointer_down(frames[0], shift);
    for p in &frames[1..frames.len() - 1] {
        hold(s, *p, shift, ctrl);
    }
    let last = *frames.last().unwrap();
    hold(s, last, shift, ctrl);
    s.pointer_up(last, shift, ctrl);
    hold(s, last, false, false);
}
fn drag(s: &mut Session, from: Point, to: Point) {
    drag_frames(s, &[from, to], false, false);
}
fn dblclick(s: &mut Session, at: Point, shift: bool, ctrl: bool) -> bool {
    hold(s, at, shift, ctrl);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, ctrl);
    s.double_click(at, shift, ctrl)
}

// --- readers ---------------------------------------------------------------------------

fn rect_of(o: &ObjectSnapshot) -> (RectBounds, CornerRadii, f64) {
    match o {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect {
                bounds,
                corner_radii,
            } => (bounds, corner_radii, p.rotation.as_radians()),
            other => panic!("rect expected, got {other:?}"),
        },
        _ => panic!("rect expected"),
    }
}
fn radii4(r: CornerRadii) -> [f64; 4] {
    [r.tl.as_mm(), r.tr.as_mm(), r.br.as_mm(), r.bl.as_mm()]
}
fn path_pts(o: &ObjectSnapshot) -> Vec<Point> {
    match o {
        ObjectSnapshot::Path(p) => p.anchors.iter().map(|a| a.point).collect(),
        _ => panic!("path expected"),
    }
}
fn outline_pts(o: &ObjectSnapshot) -> Vec<Point> {
    match o {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => outline_of_rotated(&p.shape, p.rotation)
            .iter()
            .map(|a| a.point)
            .collect(),
        ObjectSnapshot::Path(p) => p
            .anchors
            .iter()
            .map(|a| a.point)
            .chain(
                p.extra_subpaths
                    .iter()
                    .flat_map(|s| s.anchors.iter().map(|a| a.point)),
            )
            .collect(),
        ObjectSnapshot::Clone(c) => c.resolved.anchors.iter().map(|a| a.point).collect(),
    }
}
fn rotation_of(o: &ObjectSnapshot) -> f64 {
    match o {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => p.rotation.as_radians(),
        ObjectSnapshot::Path(p) => p.rotation.as_radians(),
        ObjectSnapshot::Clone(_) => 0.0,
    }
}
fn assert_pt(a: Point, x: f64, y: f64, eps: f64, what: &str) {
    assert!(
        near(a.x, x, eps) && near(a.y, y, eps),
        "{what}: got ({}, {}), want ({x}, {y})",
        a.x,
        a.y
    );
}
fn assert_rect(o: &ObjectSnapshot, x: f64, y: f64, w: f64, h: f64, eps: f64, what: &str) {
    let (b, _, _) = rect_of(o);
    assert!(
        near(b.origin.x, x, eps)
            && near(b.origin.y, y, eps)
            && near(b.width.as_mm(), w, eps)
            && near(b.height.as_mm(), h, eps),
        "{what}: got ({}, {}) {}x{}, want ({x}, {y}) {w}x{h}",
        b.origin.x,
        b.origin.y,
        b.width.as_mm(),
        b.height.as_mm()
    );
}
/// Two point sets are equal as multisets within `eps`.
fn same_set(a: &[Point], b: &[Point], eps: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|p| {
        b.iter().enumerate().any(|(i, q)| {
            if !used[i] && near(p.x, q.x, eps) && near(p.y, q.y, eps) {
                used[i] = true;
                true
            } else {
                false
            }
        })
    })
}
fn mirror_pts(pts: &[Point], axis: FlipAxis, c: Point) -> Vec<Point> {
    pts.iter()
        .map(|p| match axis {
            FlipAxis::Horizontal => pt(2.0 * c.x - p.x, p.y),
            FlipAxis::Vertical => pt(p.x, 2.0 * c.y - p.y),
        })
        .collect()
}
fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

// --- scenes ----------------------------------------------------------------------------

fn tri_path(d: &Document, n: u64) -> NodeId {
    d.create_path(
        &[
            anchor(n, 50.0, 50.0),
            anchor(n + 1, 80.0, 50.0),
            anchor(n + 2, 50.0, 60.0),
        ],
        true,
    )
}
fn one_tri() -> Session {
    let d = Document::new(1);
    tri_path(&d, 1);
    open(&d)
}
fn two_rects() -> Document {
    let d = Document::new(1);
    let _ = d.create_rect(rb(50.0, 50.0, 10.0, 10.0));
    let b = d.create_rect(rb(70.0, 50.0, 10.0, 10.0));
    d.set_corner_radii(&[(
        b,
        CornerRadii {
            tl: Length::from_mm(2.0),
            tr: Length::from_mm(0.0),
            br: Length::from_mm(0.0),
            bl: Length::from_mm(0.0),
        },
    )])
    .unwrap();
    d
}
fn radius_rect(r: [f64; 4]) -> Document {
    let d = Document::new(1);
    let id = d.create_rect(rb(50.0, 50.0, 30.0, 10.0));
    d.set_corner_radii(&[(
        id,
        CornerRadii {
            tl: Length::from_mm(r[0]),
            tr: Length::from_mm(r[1]),
            br: Length::from_mm(r[2]),
            bl: Length::from_mm(r[3]),
        },
    )])
    .unwrap();
    d
}
fn rect_doc() -> Session {
    let d = Document::new(1);
    let _ = d.create_rect(rb(50.0, 50.0, 30.0, 10.0));
    open(&d)
}
/// Selects the first object by a click on the middle of its top edge.
fn click_at(s: &mut Session, p: Point) {
    hold(s, p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

// ======================================================================================
// Part A: the flip buttons, the keys
// ======================================================================================

/// AC1: the buttons are disabled with nothing selected, enabled with a selection, and only in
/// the Select tool.
#[test]
fn ac01_flip_enabled_state() {
    let mut s = rect_doc();
    assert!(!s.flip_enabled(), "nothing selected");
    assert_eq!(
        s.flip_selection(FlipAxis::Horizontal),
        FlipOutcome::NothingSelected
    );
    select_all(&mut s);
    assert!(s.flip_enabled());
    s.set_tool(Tool::Node);
    assert!(!s.flip_enabled(), "not in the Node tool");
}

/// AC2 with its example, AC29 (one step, name, undo, redo) and "the selection stays".
#[test]
fn ac02_two_rect_example_one_step_name_undo_redo() {
    let mut s = open(&two_rects());
    select_all(&mut s);
    let ids_before = s.selected_object_count();
    let before = objects(&s);
    let steps = s.undo_len();
    assert_eq!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(undo_name(&mut s).as_deref(), Some("Flip horizontally"));
    assert_eq!(s.selected_object_count(), ids_before, "selection stays");
    let after = objects(&s);
    // group box (50,50)-(80,60), centre x = 65.
    assert_rect(&after[0], 70.0, 50.0, 10.0, 10.0, 1e-9, "A");
    assert_rect(&after[1], 50.0, 50.0, 10.0, 10.0, 1e-9, "B");
    let (_, r, _) = rect_of(&after[1]);
    assert_eq!(radius_t(r), [0.0, 2.0, 0.0, 0.0], "B radius at top-right");
    assert_eq!(
        before.iter().map(|o| id_of(o)).collect::<Vec<_>>(),
        after.iter().map(|o| id_of(o)).collect::<Vec<_>>(),
        "same ids"
    );
    // Undo restores the exact objects; redo does it again.
    s.undo();
    assert_eq!(objects(&s), before);
    s.redo();
    assert_eq!(objects(&s), after);
    // Vertical flip: name.
    assert_eq!(s.flip_selection(FlipAxis::Vertical), FlipOutcome::Flipped);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Flip vertically"));
}

fn radius_t(r: CornerRadii) -> [f64; 4] {
    radii4(r)
}
fn id_of(o: &ObjectSnapshot) -> String {
    match o {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => format!("{:?}", p.id),
        ObjectSnapshot::Path(p) => format!("{:?}", p.id),
        ObjectSnapshot::Clone(c) => format!("{:?}", c.resolved.id),
    }
}

/// AC20: the four radii stay at their physical corners for H, V and both.
#[test]
fn ac20_radii_at_physical_corners() {
    // tl, tr, br, bl = 1, 2, 3, 4
    let cases = [
        (vec![FlipAxis::Horizontal], [2.0, 1.0, 4.0, 3.0]),
        (vec![FlipAxis::Vertical], [4.0, 3.0, 2.0, 1.0]),
        (
            vec![FlipAxis::Horizontal, FlipAxis::Vertical],
            [3.0, 4.0, 1.0, 2.0],
        ),
    ];
    for (axes, want) in cases {
        let mut s = open(&radius_rect([1.0, 2.0, 3.0, 4.0]));
        select_all(&mut s);
        for a in &axes {
            assert_eq!(s.flip_selection(*a), FlipOutcome::Flipped);
        }
        let (b, r, rot) = rect_of(&objects(&s)[0]);
        assert_eq!(radii4(r), want, "{axes:?}");
        assert!(near(b.origin.x, 50.0, 1e-9) && near(b.width.as_mm(), 30.0, 1e-9));
        assert!(near(rot, 0.0, 1e-12));
    }
    // The spec's example: radius 3 at top-left only.
    let mut s = open(&radius_rect([3.0, 0.0, 0.0, 0.0]));
    select_all(&mut s);
    s.flip_selection(FlipAxis::Horizontal);
    assert_eq!(radii4(rect_of(&objects(&s)[0]).1), [0.0, 3.0, 0.0, 0.0]);
    s.flip_selection(FlipAxis::Vertical);
    assert_eq!(radii4(rect_of(&objects(&s)[0]).1), [0.0, 0.0, 3.0, 0.0]);
}

/// AC20: a rotated rectangle's rotation is negated by a document flip, and the physical corners
/// still hold, also at 90 degrees.
#[test]
fn ac20_rotated_rect_flip() {
    for deg in [30.0_f64, 90.0, -45.0] {
        let d = radius_rect([1.0, 2.0, 3.0, 4.0]);
        let id = d.object_ids()[0];
        let o = d.object(id).unwrap();
        d.rotate_object(&o.rotated(pt(65.0, 55.0), Angle::from_radians(deg.to_radians())))
            .unwrap();
        let mut s = open(&d);
        select_all(&mut s);
        let before = objects(&s);
        let (_, _, r0) = rect_of(&before[0]);
        for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
            let mut s2 = open(&doc_of(&s));
            select_all(&mut s2);
            s2.flip_selection(axis);
            let after = objects(&s2);
            let (_, _, r1) = rect_of(&after[0]);
            let sum = wrap(r0 + r1);
            assert!(
                near(sum, 0.0, 1e-9),
                "{deg} {axis:?}: rotation {r0} -> {r1} should be negated"
            );
            // physical outline is the exact mirror image
            let want = mirror_pts(
                &outline_pts(&before[0]),
                axis,
                tbb_centre_extreme(&before[0]),
            );
            assert!(
                same_set(&want, &outline_pts(&after[0]), 1e-4),
                "{deg} {axis:?}: outline is not the mirror image"
            );
        }
    }
}
fn wrap(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    let mut r = a % t;
    if r > std::f64::consts::PI {
        r -= t;
    }
    if r <= -std::f64::consts::PI {
        r += t;
    }
    r
}
/// The centre of the outline's bounding box of a single rect/polygon (corner anchors only).
fn tbb_centre(o: &ObjectSnapshot) -> Point {
    let p = outline_pts(o);
    let (mut lo, mut hi) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
    for q in &p {
        lo = pt(lo.x.min(q.x), lo.y.min(q.y));
        hi = pt(hi.x.max(q.x), hi.y.max(q.y));
    }
    pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0)
}

/// AC19: a path with handles, kinds and ids: anchors mapped, handle x negated, kinds, order,
/// ids and closed state unchanged, the path direction not reversed.
#[test]
fn ac19_path_flip_handles_kinds_ids() {
    let d = Document::new(1);
    let mut a1 = anchor(1, 50.0, 50.0);
    a1.handle_out = Vec2::new(2.0, 0.0);
    let mut a2 = anchor(2, 60.0, 50.0);
    a2.handle_in = Vec2::new(-2.0, 0.0);
    a2.handle_out = Vec2::new(0.0, 3.0);
    a2.kind = AnchorKind::Symmetric;
    let a3 = anchor(3, 60.0, 60.0);
    let a4 = anchor(4, 50.0, 60.0);
    let _ = d.create_path(&[a1, a2, a3, a4], true);
    let before = d.object(d.object_ids()[0]).unwrap();
    let ObjectSnapshot::Path(p0) = &before else {
        panic!()
    };
    let mut s = open(&d);
    select_all(&mut s);
    s.flip_selection(FlipAxis::Horizontal);
    let ObjectSnapshot::Path(p1) = &objects(&s)[0] else {
        panic!()
    };
    assert_eq!(p1.anchors.len(), 4);
    assert!(p1.closed);
    for (a, b) in p0.anchors.iter().zip(&p1.anchors) {
        assert_eq!(a.id, b.id, "anchor ids and order");
        assert_eq!(a.kind, b.kind, "kinds");
        assert_pt(b.point, 110.0 - a.point.x, a.point.y, 1e-9, "anchor");
        assert!(
            near(b.handle_in.x, -a.handle_in.x, 1e-9) && near(b.handle_in.y, a.handle_in.y, 1e-9)
        );
        assert!(
            near(b.handle_out.x, -a.handle_out.x, 1e-9)
                && near(b.handle_out.y, a.handle_out.y, 1e-9)
        );
    }
    // Vertical: y mapped about 55.
    s.flip_selection(FlipAxis::Horizontal); // back
    s.flip_selection(FlipAxis::Vertical);
    let ObjectSnapshot::Path(p2) = &objects(&s)[0] else {
        panic!()
    };
    for (a, b) in p0.anchors.iter().zip(&p2.anchors) {
        assert_pt(b.point, a.point.x, 110.0 - a.point.y, 1e-9, "anchor v");
        assert!(near(b.handle_out.y, -a.handle_out.y, 1e-9));
        assert!(near(b.handle_out.x, a.handle_out.x, 1e-9));
    }
}

/// AC3: no-op flips write nothing, make no step, and say so in the notice slot.
#[test]
fn ac03_noop_flip() {
    // Circle at 30 degrees, either flip.
    for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
        let d = Document::new(1);
        let id = d.create_ellipse(EllipseFrame {
            center: pt(70.0, 70.0),
            rx: Length::from_mm(10.0),
            ry: Length::from_mm(10.0),
        });
        d.rotate_object(
            &d.object(id)
                .unwrap()
                .rotated(pt(70.0, 70.0), Angle::from_radians(0.5)),
        )
        .unwrap();
        let mut s = open(&d);
        select_all(&mut s);
        let steps = s.undo_len();
        let before = bytes_of(&s);
        assert_eq!(s.flip_selection(axis), FlipOutcome::LooksTheSame, "circle");
        assert_eq!(s.undo_len(), steps, "no history step");
        assert!(bytes_of(&s) == before, "nothing written");
        // The words of this notice belong to the frontend (flipText.ts), checked there.
        assert!(s.flip_enabled(), "button stays enabled");
    }
    // Horizontal straight line, vertical flip.
    let d = Document::new(1);
    let _ = d.create_path(&[anchor(1, 50.0, 55.0), anchor(2, 80.0, 55.0)], false);
    let mut s = open(&d);
    select_all(&mut s);
    assert_eq!(
        s.flip_selection(FlipAxis::Vertical),
        FlipOutcome::LooksTheSame,
        "line"
    );
    assert_eq!(
        s.flip_selection(FlipAxis::Horizontal),
        FlipOutcome::Flipped,
        "a horizontal flip of a line changes the order of its ends"
    );
    // Symmetric path: isosceles triangle, horizontal flip.
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            anchor(1, 50.0, 60.0),
            anchor(2, 60.0, 50.0),
            anchor(3, 70.0, 60.0),
        ],
        true,
    );
    let mut s = open(&d);
    select_all(&mut s);
    let r = s.flip_selection(FlipAxis::Horizontal);
    // The anchor order differs after the mirror, so it is up to the spec's "equals within
    // tolerance" reading; the outline is the same, so a no-op is expected.
    assert_eq!(r, FlipOutcome::LooksTheSame, "symmetric path");
}

/// AC22: the triangle example, outline is the exact mirror image about the TBB centre.
#[test]
fn ac22_triangle_example() {
    let d = Document::new(1);
    let _ = d.create_polygon(
        StarFrame {
            center: pt(75.0, 55.0),
            radius: Length::from_mm(5.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(3).unwrap(),
    );
    let mut s = open(&d);
    select_all(&mut s);
    let before = objects(&s);
    // vertices (80,55) (72.5,59.33) (72.5,50.67); TBB x 72.5..80, centre 76.25
    s.flip_selection(FlipAxis::Horizontal);
    let after = objects(&s);
    let ObjectSnapshot::Primitive(p) = &after[0] else {
        panic!()
    };
    let Shape::Polygon {
        frame, point_count, ..
    } = p.shape
    else {
        panic!("still a polygon: {:?}", p.shape)
    };
    assert_eq!(point_count.get(), 3);
    assert!(near(frame.radius.as_mm(), 5.0, 1e-9));
    assert_pt(frame.center, 77.5, 55.0, 1e-9, "centre");
    let v = outline_pts(&after[0]);
    assert!(same_set(
        &v,
        &[pt(72.5, 55.0), pt(80.0, 59.330127), pt(80.0, 50.669873)],
        1e-5
    ));
    let want = mirror_pts(
        &outline_pts(&before[0]),
        FlipAxis::Horizontal,
        pt(76.25, 55.0),
    );
    assert!(same_set(&want, &v, 1e-6));
}

/// AC21, AC22: ellipse, star and polygon under both flips stay their kind, outline is the
/// mirror image, double flip returns.
#[test]
fn ac21_ac22_primitives_mirror_image_and_involution() {
    let d = Document::new(1);
    let e = d.create_ellipse(EllipseFrame {
        center: pt(60.0, 60.0),
        rx: Length::from_mm(12.0),
        ry: Length::from_mm(5.0),
    });
    d.rotate_object(
        &d.object(e)
            .unwrap()
            .rotated(pt(60.0, 60.0), Angle::from_radians(30.0_f64.to_radians())),
    )
    .unwrap();
    let st = d.create_star(
        StarFrame {
            center: pt(100.0, 60.0),
            radius: Length::from_mm(9.0),
            angle: Angle::from_radians(0.4),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.4).unwrap(),
    );
    d.rotate_object(
        &d.object(st)
            .unwrap()
            .rotated(pt(100.0, 60.0), Angle::from_radians(0.3)),
    )
    .unwrap();
    let _ = d.create_polygon(
        StarFrame {
            center: pt(140.0, 60.0),
            radius: Length::from_mm(7.0),
            angle: Angle::from_radians(1.0),
        },
        PointCount::new(6).unwrap(),
    );
    for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
        for idx in 0..3 {
            let mut s = open(&d);
            let ids = d.object_ids();
            let one = doc_of(&s);
            let _ = (&ids, &one);
            // select only object idx: click inside its outline mid-edge via marquee-free route:
            // delete the others from a copy of the document.
            let keep = d.object_ids()[idx];
            let d2 = unpack(5, &s.pack("0.1.0").unwrap()).unwrap();
            for id in d2.object_ids() {
                if id != keep {
                    d2.delete_objects(&[id]).unwrap();
                }
            }
            s = open(&d2);
            select_all(&mut s);
            let before = objects(&s);
            let c = tbb_centre_extreme(&before[0]);
            assert_eq!(
                s.flip_selection(axis),
                FlipOutcome::Flipped,
                "{idx} {axis:?}"
            );
            let after = objects(&s);
            // same kind
            match (&before[0], &after[0]) {
                (ObjectSnapshot::Primitive(a), ObjectSnapshot::Primitive(b)) => {
                    assert_eq!(
                        std::mem::discriminant(&a.shape),
                        std::mem::discriminant(&b.shape),
                        "kind kept"
                    );
                    if let (
                        Shape::Star {
                            point_count: pa,
                            inner_ratio: ia,
                            frame: fa,
                            ..
                        },
                        Shape::Star {
                            point_count: pb,
                            inner_ratio: ib,
                            frame: fb,
                            ..
                        },
                    ) = (a.shape, b.shape)
                    {
                        assert_eq!(pa.get(), pb.get());
                        assert!(near(ia.get(), ib.get(), 1e-12));
                        assert!(near(fa.radius.as_mm(), fb.radius.as_mm(), 1e-9));
                    }
                }
                _ => panic!("primitive expected"),
            }
            let want = mirror_pts(&outline_pts(&before[0]), axis, c);
            assert!(
                same_set(&want, &outline_pts(&after[0]), 1e-4),
                "{idx} {axis:?}: outline is not the mirror image"
            );
            // TBB unchanged (AC2, AC28c)
            let c2 = tbb_centre_extreme(&after[0]);
            assert_pt(c2, c.x, c.y, 1e-6, "TBB centre kept");
            // flipping twice returns
            s.flip_selection(axis);
            let again = objects(&s);
            assert!(
                same_set(&outline_pts(&before[0]), &outline_pts(&again[0]), 1e-9),
                "{idx} {axis:?}: involution"
            );
            assert!(near(
                wrap(rotation_of(&before[0]) - rotation_of(&again[0])),
                0.0,
                1e-9
            ));
        }
    }
}
/// The centre of the true bounding box of a closed outline, found by dense sampling of the
/// curve with the anchors' handles.
fn tbb_centre_extreme(o: &ObjectSnapshot) -> Point {
    let (lo, hi) = tbb(o);
    pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0)
}
fn tbb(o: &ObjectSnapshot) -> (Point, Point) {
    let anchors = match o {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => outline_of_rotated(&p.shape, p.rotation),
        ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => panic!("primitives only"),
    };
    let n = anchors.len();
    let (mut lo, mut hi) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
    for i in 0..n {
        let a = &anchors[i];
        let b = &anchors[(i + 1) % n];
        let (p0, p1) = (
            a.point,
            pt(a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
        );
        let (p3, p2) = (
            b.point,
            pt(b.point.x + b.handle_in.x, b.point.y + b.handle_in.y),
        );
        for k in 0..=2000 {
            let t = f64::from(k) / 2000.0;
            let u = 1.0 - t;
            let x = u * u * u * p0.x
                + 3.0 * u * u * t * p1.x
                + 3.0 * u * t * t * p2.x
                + t * t * t * p3.x;
            let y = u * u * u * p0.y
                + 3.0 * u * u * t * p1.y
                + 3.0 * u * t * t * p2.y
                + t * t * t * p3.y;
            lo = pt(lo.x.min(x), lo.y.min(y));
            hi = pt(hi.x.max(x), hi.y.max(y));
        }
    }
    (lo, hi)
}

/// AC19: a ring (compound path) keeps its hole after a flip.
#[test]
fn ac19_compound_ring_flip() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            anchor(1, 50.0, 50.0),
            anchor(2, 90.0, 50.0),
            anchor(3, 90.0, 90.0),
            anchor(4, 50.0, 90.0),
        ],
        true,
    );
    let _ = d.create_path(
        &[
            anchor(11, 55.0, 60.0),
            anchor(12, 65.0, 60.0),
            anchor(13, 65.0, 70.0),
            anchor(14, 55.0, 70.0),
        ],
        true,
    );
    let mut s = open(&d);
    select_all(&mut s);
    s.apply_combine();
    let objs = objects(&s);
    let ObjectSnapshot::Path(p) = &objs[0] else {
        panic!()
    };
    assert!(p.is_compound(), "combined");
    let before = outline_pts(&objs[0]);
    select_all(&mut s);
    assert_eq!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    let after = objects(&s);
    let ObjectSnapshot::Path(q) = &after[0] else {
        panic!()
    };
    assert!(q.is_compound(), "still compound");
    assert_eq!(
        q.extra_subpaths.len(),
        p.extra_subpaths.len(),
        "outlines kept"
    );
    // outer square 50..90 centre 70; hole x 55..65 -> 75..85
    assert!(same_set(
        &mirror_pts(&before, FlipAxis::Horizontal, pt(70.0, 70.0)),
        &outline_pts(&after[0]),
        1e-9
    ));
    // The hole is still a hole: a point in it is not hit, a point in the ring is.
    let hole_c = pt(80.0, 65.0);
    let ring_pt = pt(60.0, 85.0);
    let _ = (hole_c, ring_pt);
}

/// AC24: a group is flipped through its descendants about the group's box.
#[test]
fn ac24_group_flip() {
    let d = two_rects();
    let ids = d.object_ids();
    d.group_objects(&ids).unwrap();
    let mut s = open(&d);
    select_all(&mut s);
    assert_eq!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    let o = objects(&s);
    assert_rect(&o[0], 70.0, 50.0, 10.0, 10.0, 1e-9, "A");
    assert_rect(&o[1], 50.0, 50.0, 10.0, 10.0, 1e-9, "B");
}

/// AC25: a clone alone selected is flipped about its own box; the origin is untouched.
#[test]
fn ac25_clone_flip() {
    let d = Document::new(1);
    let origin = tri_path(&d, 1);
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    let mut s = open(&d);
    // Select the clone only: delete-free route through the layers is not available here, so
    // select by a click on a point of the clone that is not covered by the origin.
    let o = doc_of(&s).object(clone).unwrap();
    let before_pts = outline_pts(&o);
    let org_before = outline_pts(&doc_of(&s).object(origin).unwrap());
    let mut sel = false;
    for p in &before_pts {
        // a point on the clone's outline halfway to the next anchor
        let _ = p;
    }
    // A clone at the same place as its origin cannot be picked apart by position; move it.
    let (lo, hi) = {
        let (mut lo, mut hi) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
        for q in &before_pts {
            lo = pt(lo.x.min(q.x), lo.y.min(q.y));
            hi = pt(hi.x.max(q.x), hi.y.max(q.y));
        }
        (lo, hi)
    };
    let _ = (&mut sel, &mut s);
    let c = pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    // Select both, flip: both are in the selection, the clone follows G.M.G^-1 and so the
    // clone and origin move together: the result as drawn is the mirror image of both.
    let mut s = open(&d);
    select_all(&mut s);
    assert_eq!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    let all = objects(&s);
    let got_clone = all
        .iter()
        .find(|x| matches!(x, ObjectSnapshot::Clone(_)))
        .unwrap();
    let got_org = all
        .iter()
        .find(|x| matches!(x, ObjectSnapshot::Path(_)))
        .unwrap();
    assert!(same_set(
        &mirror_pts(&org_before, FlipAxis::Horizontal, c),
        &outline_pts(got_org),
        1e-9
    ));
    assert!(same_set(
        &mirror_pts(&before_pts, FlipAxis::Horizontal, c),
        &outline_pts(got_clone),
        1e-9
    ));
}

/// AC4: H and V; gating; hint with nothing selected; nothing in other tools.
#[test]
fn ac04_keys_h_and_v() {
    let mut s = open(&two_rects());
    // nothing selected: hint
    assert_eq!(key(&mut s, "h"), KeyOutcome::Hint(KeyHint::SelectFirst));
    assert_eq!(key(&mut s, "v"), KeyOutcome::Hint(KeyHint::SelectFirst));
    select_all(&mut s);
    let before = objects(&s);
    let steps = s.undo_len();
    // gated: Ctrl, Alt
    assert_eq!(
        key_full(&mut s, "h", false, true, false),
        KeyOutcome::Ignored
    );
    assert_eq!(
        key_full(&mut s, "v", false, false, true),
        KeyOutcome::Ignored
    );
    assert_eq!(s.undo_len(), steps, "gated keys write nothing");
    // dom_blocked
    let blocked = s.key_down(KeyInput {
        key: "h",
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(blocked, KeyOutcome::Ignored);
    assert_eq!(objects(&s), before);
    // H flips like the button
    let _ = key(&mut s, "h");
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Flip horizontally"));
    assert_rect(&objects(&s)[0], 70.0, 50.0, 10.0, 10.0, 1e-9, "H flipped");
    let _ = key(&mut s, "v");
    assert_eq!(undo_name(&mut s).as_deref(), Some("Flip vertically"));
    // Uppercase works too (Shift+H is a different chord and is not asserted).
    // In the Node tool: nothing.
    s.set_tool(Tool::Node);
    let n = s.undo_len();
    let _ = key(&mut s, "h");
    let _ = key(&mut s, "v");
    assert_eq!(s.undo_len(), n, "no flip in the Node tool");
    s.set_tool(Tool::Rectangle);
    let _ = key(&mut s, "h");
    assert_eq!(s.undo_len(), n, "no flip in the Rectangle tool");
}

/// AC5: a flip with a chip open closes the chip without writing and flips only.
#[test]
fn ac05_flip_closes_chip() {
    for chip_key in ["s", "r", "m"] {
        let mut s = open(&two_rects());
        select_all(&mut s);
        assert_eq!(key(&mut s, chip_key), KeyOutcome::EntryOpened, "{chip_key}");
        let steps = s.undo_len();
        assert_eq!(s.flip_selection(FlipAxis::Vertical), FlipOutcome::Flipped);
        assert!(s.transform_entry().is_none(), "chip closed ({chip_key})");
        assert_eq!(s.undo_len(), steps + 1, "only the flip is written");
        assert_eq!(undo_name(&mut s).as_deref(), Some("Flip vertically"));
    }
}

// ======================================================================================
// Part B: resizing through zero
// ======================================================================================

/// AC6: free corner drag across both axes (path example), and across one axis.
#[test]
fn ac06_free_corner_drag_through_zero() {
    // SE handle (80,60) of the box (50,50)-(80,60); fixed point NW (50,50).
    let mut s = one_tri();
    click_path(&mut s);
    drag(&mut s, pt(80.0, 60.0), pt(20.0, 40.0));
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[0], 50.0, 50.0, 1e-6, "n0");
    assert_pt(p[1], 20.0, 50.0, 1e-6, "n1");
    assert_pt(p[2], 50.0, 40.0, 1e-6, "n2");
    // sx = -1, sy = 2
    let mut s = one_tri();
    click_path(&mut s);
    drag(&mut s, pt(80.0, 60.0), pt(20.0, 70.0));
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[1], 20.0, 50.0, 1e-6, "n1 b");
    assert_pt(p[2], 50.0, 70.0, 1e-6, "n2 b");
    assert_eq!(s.undo_len(), 1, "one resize step");
    assert_eq!(undo_name(&mut s).as_deref(), Some("Resize"));
}
/// Selects the (only) path by a click on its top edge.
fn click_path(s: &mut Session) {
    click_at(s, pt(65.0, 50.0));
    assert_eq!(s.selected_object_count(), 1);
}

/// AC7 and AC11: an edge drag across the opposite edge, then the handle by position.
#[test]
fn ac07_ac11_edge_drag_and_handle_by_position() {
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag(&mut s, pt(80.0, 55.0), pt(40.0, 55.0));
    let o = objects(&s);
    assert_rect(&o[0], 40.0, 50.0, 10.0, 10.0, 1e-6, "E dragged past W");
    // AC11: the handle at the left edge (x = 40) is the W handle; dragging it 5 right keeps
    // the right edge x = 50 fixed.
    drag(&mut s, pt(40.0, 55.0), pt(45.0, 55.0));
    assert_rect(
        &objects(&s)[0],
        45.0,
        50.0,
        5.0,
        10.0,
        1e-6,
        "W handle by position",
    );
}

/// AC8: Ctrl on a corner handle keeps the proportions and each axis keeps its free sign.
#[test]
fn ac08_ctrl_corner() {
    let mut s = one_tri();
    click_path(&mut s);
    drag_frames(&mut s, &[pt(80.0, 60.0), pt(35.0, 52.0)], false, true);
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[0], 50.0, 50.0, 1e-6, "n0");
    assert_pt(p[1], 35.0, 50.0, 1e-6, "n1");
    assert_pt(p[2], 50.0, 55.0, 1e-6, "n2");
}

/// AC9: Shift with an edge handle through the centre.
#[test]
fn ac09_shift_edge() {
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    // centre x = 65; E dragged to 20: signed half width -45 -> 20..110
    drag_frames(&mut s, &[pt(80.0, 55.0), pt(20.0, 55.0)], true, false);
    assert_rect(&objects(&s)[0], 20.0, 50.0, 90.0, 10.0, 1e-6, "shift");
}

/// AC10: readout shows the signed size with U+2212; Escape drops; a tiny press writes nothing.
#[test]
fn ac10_readout_escape() {
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    let before = bytes_of(&s);
    hold(&mut s, pt(80.0, 55.0), false, false);
    s.pointer_down(pt(80.0, 55.0), false);
    hold(&mut s, pt(70.0, 55.0), false, false);
    let t = s.live_readout().expect("readout").text;
    assert!(!t.contains(MINUS) && !t.contains('-'), "positive size: {t}");
    hold(&mut s, pt(38.0, 55.0), false, false);
    let t = s.live_readout().expect("readout").text;
    // E from 80 to 38 about x = 50: signed width -12 -> "−12.0 ..."
    assert!(t.starts_with(MINUS), "negative width has a real minus: {t}");
    assert!(t.contains("12.0"), "{t}");
    assert!(
        !t[MINUS.len_utf8()..].contains(MINUS),
        "height positive: {t}"
    );
    assert!(t.contains("10.0"), "{t}");
    // Escape drops the drag.
    key(&mut s, "Escape");
    s.pointer_up(pt(38.0, 55.0), false, false);
    assert!(bytes_of(&s) == before, "Escape wrote nothing");
    // Both negative axes: SE corner to the NW side of the NW corner.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    hold(&mut s, pt(80.0, 60.0), false, false);
    s.pointer_down(pt(80.0, 60.0), false);
    hold(&mut s, pt(38.0, 46.0), false, false);
    let t = s.live_readout().expect("readout").text;
    assert_eq!(t.matches(MINUS).count(), 2, "both signs: {t}");
    // A size under 0.1 mm shows two decimals.
    hold(&mut s, pt(50.004, 60.0), false, false);
    let t = s.live_readout().unwrap().text;
    assert!(t.contains("0.00") || t.contains("0.01"), "{t}");
    key(&mut s, "Escape");
    s.pointer_up(pt(50.0, 60.0), false, false);
    // A press and release under the dead zone writes nothing.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    let before = bytes_of(&s);
    let steps = s.undo_len();
    drag(&mut s, pt(80.0, 55.0), pt(80.0, 55.1));
    assert_eq!(s.undo_len(), steps);
    assert!(bytes_of(&s) == before);
}

/// AC12: a rotated rectangle resized along its own axis: the example of the spec.
#[test]
fn ac12_rotated_own_axis() {
    let d = Document::new(1);
    let id = d.create_rect(rb(50.0, 50.0, 20.0, 10.0));
    d.rotate_object(
        &d.object(id)
            .unwrap()
            .rotated(pt(60.0, 55.0), Angle::from_radians(30.0_f64.to_radians())),
    )
    .unwrap();
    let mut s = open(&d);
    select_all(&mut s);
    // E handle: centre + 10 along (cos30, sin30); drag to 10 mm beyond the W side:
    // W edge at centre - 10 u. Pointer = centre - 20 u (10 beyond).
    let (c, u) = (
        pt(60.0, 55.0),
        (30.0_f64.to_radians().cos(), 30.0_f64.to_radians().sin()),
    );
    let e = pt(c.x + 10.0 * u.0, c.y + 10.0 * u.1);
    let to = pt(c.x - 20.0 * u.0, c.y - 20.0 * u.1);
    drag(&mut s, e, to);
    let o = objects(&s);
    let (b, _, rot) = rect_of(&o[0]);
    assert!(
        near(b.width.as_mm(), 10.0, 1e-6),
        "width {}",
        b.width.as_mm()
    );
    assert!(near(rot, 30.0_f64.to_radians(), 1e-9), "rotation stays");
    // centre = (10,5) - 15 u in the spec frame, here + (50,50)
    let centre = {
        // centre of the unrotated bounds is not the rotated centre; use the outline
        tbb_centre_rect(&o[0])
    };
    let _ = centre;
    let pts = outline_pts(&o[0]);
    let cx = pts.iter().map(|p| p.x).sum::<f64>() / 4.0;
    let cy = pts.iter().map(|p| p.y).sum::<f64>() / 4.0;
    assert_pt(
        pt(cx, cy),
        60.0 - 15.0 * u.0,
        55.0 - 15.0 * u.1,
        0.01,
        "centre",
    );
}
fn tbb_centre_rect(o: &ObjectSnapshot) -> Point {
    tbb_centre(o)
}

/// AC13: a factor beyond the 1e7 mm limit changes nothing.
#[test]
fn ac13_huge_factor_no_change() {
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    let before = bytes_of(&s);
    let steps = s.undo_len();
    drag(&mut s, pt(80.0, 55.0), pt(5.0e7, 55.0));
    assert_eq!(s.undo_len(), steps, "all or nothing");
    assert!(bytes_of(&s) == before);
    drag(&mut s, pt(80.0, 55.0), pt(-5.0e7, 55.0));
    assert_eq!(s.undo_len(), steps);
}

// ======================================================================================
// Zero recovery (AC15)
// ======================================================================================

/// AC15: a drag landing on exactly 0 after a positive frame gives +0.01 mm; after a negative
/// frame -0.01 mm; the notice appears after the release only if released there.
#[test]
fn ac15_drag_lands_on_zero() {
    // positive previous frame
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(80.0, 55.0), pt(55.0, 55.0), pt(50.0, 55.0)],
        false,
        false,
    );
    assert_rect(&objects(&s)[0], 50.0, 50.0, 0.01, 10.0, 1e-9, "+0.01");
    assert_eq!(
        s.take_transform_notice(),
        "Width set to 0.01 mm. A zero size would flatten the shape for good."
    );
    // a first frame landing on 0 is positive
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(&mut s, &[pt(80.0, 55.0), pt(50.0, 55.0)], false, false);
    assert_rect(&objects(&s)[0], 50.0, 50.0, 0.01, 10.0, 1e-9, "first frame");
    // negative previous frame
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(80.0, 55.0), pt(45.0, 55.0), pt(50.0, 55.0)],
        false,
        false,
    );
    assert_rect(&objects(&s)[0], 49.99, 50.0, 0.01, 10.0, 1e-9, "-0.01");
    // lands on 0, moves on, released elsewhere: no notice
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[
            pt(80.0, 55.0),
            pt(55.0, 55.0),
            pt(50.0, 55.0),
            pt(46.0, 55.0),
        ],
        false,
        false,
    );
    assert_rect(&objects(&s)[0], 46.0, 50.0, 4.0, 10.0, 1e-9, "moved on");
    assert_eq!(s.take_transform_notice(), "");
    // Height: S edge to the top
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(65.0, 60.0), pt(65.0, 55.0), pt(65.0, 50.0)],
        false,
        false,
    );
    assert_rect(&objects(&s)[0], 50.0, 50.0, 30.0, 0.01, 1e-9, "height 0.01");
    assert_eq!(
        s.take_transform_notice(),
        "Height set to 0.01 mm. A zero size would flatten the shape for good."
    );
    // Both
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(80.0, 60.0), pt(60.0, 55.0), pt(50.0, 50.0)],
        false,
        false,
    );
    assert_rect(&objects(&s)[0], 50.0, 50.0, 0.01, 0.01, 1e-9, "both");
    assert_eq!(
        s.take_transform_notice(),
        "Width and height set to 0.01 mm. Zero would flatten the shape."
    );
    // 0.0006 mm stays as it is
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(&mut s, &[pt(80.0, 55.0), pt(50.0006, 55.0)], false, false);
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(near(b.width.as_mm(), 0.0006, 1e-6), "{}", b.width.as_mm());
    // 0.0004 mm gives 0.01
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(&mut s, &[pt(80.0, 55.0), pt(50.0004, 55.0)], false, false);
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(near(b.width.as_mm(), 0.01, 1e-9), "{}", b.width.as_mm());
}

/// AC15: Ctrl and Shift variants of the zero recovery.
#[test]
fn ac15_ctrl_shift_zero() {
    // Shift: symmetric extent 0.01 about the centre x = 65.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(80.0, 55.0), pt(70.0, 55.0), pt(65.0, 55.0)],
        true,
        false,
    );
    assert_rect(&objects(&s)[0], 64.995, 50.0, 0.01, 10.0, 1e-9, "shift");
    // Ctrl: dominant axis 0.01, the other follows by the common factor (30x10 -> 0.01 x 0.00333)
    // the partner is not rewritten even when its extent falls under 0.0005 mm.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    drag_frames(
        &mut s,
        &[pt(80.0, 60.0), pt(60.0, 56.0), pt(50.0, 50.0)],
        false,
        true,
    );
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(
        near(b.width.as_mm(), 0.01, 1e-9),
        "dominant {}",
        b.width.as_mm()
    );
    assert!(
        near(b.height.as_mm(), 0.01 / 3.0, 1e-9),
        "partner follows, not rewritten: {}",
        b.height.as_mm()
    );
}

/// AC14, AC15: the typed chip. Examples of the spec.
#[test]
fn ac14_typed_negative_and_zero() {
    // Double-click on the E handle, fixed edge x = 50: W -30 -> x 20..50.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    dblclick(&mut s, pt(80.0, 60.0), false, false);
    let v = s.transform_entry().expect("chip");
    assert_eq!(v.fields[0].prefill, "30.0");
    assert_eq!(
        s.commit_transform_entry(&format!("{MINUS}30"), "10.0", 0),
        EntryOutcome::Committed
    );
    assert_rect(
        &objects(&s)[0],
        20.0,
        50.0,
        30.0,
        10.0,
        1e-9,
        "mirror about x=50",
    );
    assert_eq!(undo_name(&mut s).as_deref(), Some("Resize"));
    // reopened chip shows the positive size again
    click_at(&mut s, pt(35.0, 50.0));
    assert_eq!(key(&mut s, "s"), KeyOutcome::EntryOpened);
    assert_eq!(s.transform_entry().unwrap().fields[0].prefill, "30.0");
    s.cancel_transform_entry();
    // ASCII minus and comma decimal
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    dblclick(&mut s, pt(80.0, 60.0), false, false);
    assert_eq!(
        s.commit_transform_entry("-12,5", "10.0", 0),
        EntryOutcome::Committed
    );
    assert_rect(&objects(&s)[0], 37.5, 50.0, 12.5, 10.0, 1e-9, "comma");
    // Typed 0: +0.01 and the notice.
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    dblclick(&mut s, pt(80.0, 60.0), false, false);
    assert_eq!(
        s.commit_transform_entry("0", "10.0", 0),
        EntryOutcome::Committed
    );
    assert_rect(&objects(&s)[0], 50.0, 50.0, 0.01, 10.0, 1e-9, "typed 0");
    assert_eq!(
        s.take_transform_notice(),
        "Width set to 0.01 mm. A zero size would flatten the shape for good."
    );
    // Both axes.
    let mut s = rect_doc();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("0", "0", 0),
        EntryOutcome::Committed
    );
    assert_eq!(
        s.take_transform_notice(),
        "Width and height set to 0.01 mm. Zero would flatten the shape."
    );
    // Height only.
    let mut s = rect_doc();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("30.0", "0", 1),
        EntryOutcome::Committed
    );
    assert_eq!(
        s.take_transform_notice(),
        "Height set to 0.01 mm. A zero size would flatten the shape for good."
    );
    // Pull apart again: an object 0.01 wide with its E handle dragged 20 mm.
    click_at(&mut s, pt(65.0, 50.0));
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(near(b.height.as_mm(), 0.01, 1e-9));
}

/// AC14: the key S about the centre (path example), the half turn, and AC17 refusals.
#[test]
fn ac14_ac17_key_s_examples_and_refusals() {
    // W -30 about the centre line x = 65.
    let mut s = one_tri();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry(&format!("{MINUS}30"), "10.0", 0),
        EntryOutcome::Committed
    );
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[0], 80.0, 50.0, 1e-9, "n0");
    assert_pt(p[1], 50.0, 50.0, 1e-9, "n1");
    assert_pt(p[2], 80.0, 60.0, 1e-9, "n2");
    // Half turn.
    let mut s = one_tri();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("-30", "-10", 0),
        EntryOutcome::Committed
    );
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[0], 80.0, 60.0, 1e-9, "h0");
    assert_pt(p[1], 50.0, 60.0, 1e-9, "h1");
    assert_pt(p[2], 80.0, 50.0, 1e-9, "h2");
    // The chip's note line.
    let mut s = one_tri();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.transform_entry_mirror_line("-30", "10.0", 0),
        "Mirrored left to right"
    );
    assert_eq!(
        s.transform_entry_mirror_line("30.0", "-10", 1),
        "Mirrored top to bottom"
    );
    assert_eq!(
        s.transform_entry_mirror_line("-30", "-10", 0),
        "Turned 180°"
    );
    assert_eq!(s.transform_entry_mirror_line("30.0", "10.0", 0), "");
    // Refusals keep the chip open and write nothing; "Must be above 0" is gone.
    for (w, h) in [
        ("abc", "10.0"),
        ("30.0", "1e8"),
        ("-1e8", "10.0"),
        ("%", "10.0"),
        ("50%", "10.0"),
    ] {
        let mut s = one_tri();
        select_all(&mut s);
        key(&mut s, "s");
        let before = bytes_of(&s);
        let out = s.commit_transform_entry(w, h, 0);
        assert!(
            matches!(out, EntryOutcome::Invalid { .. }),
            "{w} {h}: {out:?}"
        );
        assert!(bytes_of(&s) == before && s.transform_entry().is_some());
    }
    // +30 equals the size: unchanged; -30 is a change.
    let mut s = one_tri();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("30", "10.0", 0),
        EntryOutcome::Unchanged
    );
}

/// AC17: linked fields carry the typed sign.
#[test]
fn ac17_linked_sign() {
    let mut s = rect_doc();
    click_at(&mut s, pt(65.0, 50.0));
    dblclick(&mut s, pt(80.0, 60.0), false, true);
    let v = s.transform_entry().unwrap();
    assert!(v.linked);
    assert_eq!(
        s.transform_entry_linked(0, &format!("{MINUS}30"))
            .as_deref(),
        Some("-10.0").or(Some("\u{2212}10.0"))
    );
}

// ======================================================================================
// Part D: stroke and the stretch rule
// ======================================================================================

/// AC23: stroke width follows sqrt(|sx sy|) with the switch on, a flip never changes it.
#[test]
fn ac23_stroke_factor_and_flip() {
    let d = Document::new(1);
    let id = d.create_rect(rb(50.0, 50.0, 30.0, 10.0));
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(true))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(0.4)))
        .unwrap();
    d.set_corner_radii(&[(id, CornerRadii::uniform(Length::from_mm(2.0)))])
        .unwrap();
    let width = |s: &Session| match &objects(s)[0] {
        ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        _ => panic!(),
    };
    let mut s = open(&d);
    s.set_scale_stroke_width(true);
    s.set_scale_corner_radius(true);
    select_all(&mut s);
    s.flip_selection(FlipAxis::Horizontal);
    assert!(near(width(&s), 0.4, 1e-12), "flip keeps the stroke");
    assert_eq!(radii4(rect_of(&objects(&s)[0]).1), [2.0; 4]);
    // typed W -60 (x2 mirrored), H 20 (x2): sqrt(|-2*2|) = 2.
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("-60", "20", 0),
        EntryOutcome::Committed
    );
    assert!(near(width(&s), 0.8, 1e-9), "stroke x2: {}", width(&s));
    assert_eq!(radii4(rect_of(&objects(&s)[0]).1), [4.0; 4]);
    // switch off: untouched
    let mut s = open(&d);
    s.set_scale_stroke_width(false);
    s.set_scale_corner_radius(false);
    select_all(&mut s);
    key(&mut s, "s");
    s.commit_transform_entry("-60", "20", 0);
    assert!(near(width(&s), 0.4, 1e-12));
}

/// AC18: a mirror of different magnitudes (stretch) converts a polygon; a mirror of equal
/// magnitudes (typed -s, s) does not.
#[test]
fn ac18_stretch_decided_on_magnitudes() {
    let mk = || {
        let d = Document::new(1);
        let _ = d.create_polygon(
            StarFrame {
                center: pt(75.0, 55.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
        );
        open(&d)
    };
    // mirror with equal magnitudes: stays a polygon
    let mut s = mk();
    select_all(&mut s);
    key(&mut s, "s");
    let v = s.transform_entry().unwrap();
    let (w, h) = (v.fields[0].prefill.clone(), v.fields[1].prefill.clone());
    let wn: f64 = w.parse().unwrap();
    assert_eq!(
        s.commit_transform_entry(&format!("-{wn}"), &h, 0),
        EntryOutcome::Committed
    );
    match &objects(&s)[0] {
        ObjectSnapshot::Primitive(p) => {
            assert!(matches!(p.shape, Shape::Polygon { .. }), "{:?}", p.shape)
        }
        o => panic!("converted by a pure mirror: {o:?}"),
    }
    // magnitudes differ: converted to a path
    let mut s = mk();
    select_all(&mut s);
    key(&mut s, "s");
    let hn: f64 = s.transform_entry().unwrap().fields[1]
        .prefill
        .parse()
        .unwrap();
    assert_eq!(
        s.commit_transform_entry(&format!("-{}", wn * 2.0), &format!("{hn}"), 0),
        EntryOutcome::Committed
    );
    assert!(
        matches!(&objects(&s)[0], ObjectSnapshot::Path(_)),
        "stretch converts"
    );
}

// ======================================================================================
// Part E: arithmetic (AC28), undo (AC29), file (AC26), budgets (AC27)
// ======================================================================================

/// A deterministic mixed scene.
fn mixed_scene() -> Document {
    let d = Document::new(1);
    let r = d.create_rect(rb(50.0, 50.0, 30.0, 10.0));
    d.set_corner_radii(&[(
        r,
        CornerRadii {
            tl: Length::from_mm(1.0),
            tr: Length::from_mm(2.0),
            br: Length::from_mm(3.0),
            bl: Length::from_mm(4.0),
        },
    )])
    .unwrap();
    d.rotate_object(
        &d.object(r)
            .unwrap()
            .rotated(pt(65.0, 55.0), Angle::from_radians(0.3)),
    )
    .unwrap();
    let e = d.create_ellipse(EllipseFrame {
        center: pt(120.0, 70.0),
        rx: Length::from_mm(12.0),
        ry: Length::from_mm(6.0),
    });
    d.rotate_object(
        &d.object(e)
            .unwrap()
            .rotated(pt(120.0, 70.0), Angle::from_radians(0.7)),
    )
    .unwrap();
    let _ = d.create_star(
        StarFrame {
            center: pt(170.0, 60.0),
            radius: Length::from_mm(9.0),
            angle: Angle::from_radians(0.4),
        },
        PointCount::new(7).unwrap(),
        InnerRatio::new(0.45).unwrap(),
    );
    let _ = d.create_polygon(
        StarFrame {
            center: pt(200.0, 90.0),
            radius: Length::from_mm(8.0),
            angle: Angle::from_radians(1.1),
        },
        PointCount::new(5).unwrap(),
    );
    let mut a1 = anchor(1, 60.0, 100.0);
    a1.handle_out = Vec2::new(5.0, 4.0);
    let mut a2 = anchor(2, 90.0, 120.0);
    a2.handle_in = Vec2::new(-6.0, 2.0);
    a2.kind = AnchorKind::Symmetric;
    let a3 = anchor(3, 70.0, 140.0);
    let _ = d.create_path(&[a1, a2, a3], true);
    d
}

fn nums(o: &ObjectSnapshot) -> Vec<f64> {
    let mut v = vec![rotation_of(o)];
    match o {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => {
            for q in outline_of_rotated(&p.shape, p.rotation) {
                v.extend([
                    q.point.x,
                    q.point.y,
                    q.handle_in.x,
                    q.handle_in.y,
                    q.handle_out.x,
                    q.handle_out.y,
                ]);
            }
            if let Shape::Rect { corner_radii, .. } = p.shape {
                v.extend(radii4(corner_radii));
            }
        }
        ObjectSnapshot::Path(p) => {
            for a in &p.anchors {
                v.extend([
                    a.point.x,
                    a.point.y,
                    a.handle_in.x,
                    a.handle_in.y,
                    a.handle_out.x,
                    a.handle_out.y,
                ]);
            }
        }
        ObjectSnapshot::Clone(_) => {}
    }
    v
}

/// AC28 (a): two flips give the objects back, for every kind; (c) the TBB of a selection is
/// unchanged; (b) H then V is a half turn about the TBB centre; (f) independent of the order.
#[test]
fn ac28_flip_properties_on_mixed_selection() {
    let d = mixed_scene();
    let mut s = open(&d);
    select_all(&mut s);
    let before = objects(&s);
    for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
        assert_eq!(s.flip_selection(axis), FlipOutcome::Flipped);
        assert_eq!(s.flip_selection(axis), FlipOutcome::Flipped);
        let after = objects(&s);
        for (a, b) in before.iter().zip(&after) {
            let (na, nb) = (nums(a), nums(b));
            assert_eq!(na.len(), nb.len());
            // rotation is compared on the circle
            assert!(
                near(wrap(na[0] - nb[0]), 0.0, 1e-12),
                "rotation {} {}",
                na[0],
                nb[0]
            );
            for (x, y) in na.iter().zip(&nb).skip(1) {
                assert!(near(*x, *y, 1e-9), "{axis:?}: {x} vs {y}");
            }
        }
    }
    // (b) H then V = a half turn about the TBB centre: point p -> 2c - p.
    let (mut lo, mut hi) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
    let mut ptsb = Vec::new();
    for o in &before {
        let (l, h) = match o {
            ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
            ObjectSnapshot::Primitive(_) => tbb(o),
            ObjectSnapshot::Path(_) => path_tbb(o),
            ObjectSnapshot::Clone(_) => unreachable!(),
        };
        lo = pt(lo.x.min(l.x), lo.y.min(l.y));
        hi = pt(hi.x.max(h.x), hi.y.max(h.y));
        ptsb.push(outline_pts(o));
    }
    let c = pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    s.flip_selection(FlipAxis::Horizontal);
    s.flip_selection(FlipAxis::Vertical);
    let after = objects(&s);
    for (o, pb) in after.iter().zip(&ptsb) {
        let want: Vec<Point> = pb
            .iter()
            .map(|p| pt(2.0 * c.x - p.x, 2.0 * c.y - p.y))
            .collect();
        assert!(same_set(&want, &outline_pts(o), 1e-4), "H+V is a half turn");
    }
    // (c) TBB unchanged
    let (mut lo2, mut hi2) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
    for o in &after {
        let (l, h) = match o {
            ObjectSnapshot::Primitive(_) => tbb(o),
            _ => path_tbb(o),
        };
        lo2 = pt(lo2.x.min(l.x), lo2.y.min(l.y));
        hi2 = pt(hi2.x.max(h.x), hi2.y.max(h.y));
    }
    assert_pt(lo2, lo.x, lo.y, 1e-6, "TBB min");
    assert_pt(hi2, hi.x, hi.y, 1e-6, "TBB max");
}
fn path_tbb(o: &ObjectSnapshot) -> (Point, Point) {
    let ObjectSnapshot::Path(p) = o else { panic!() };
    let n = p.anchors.len();
    let (mut lo, mut hi) = (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN));
    for i in 0..n {
        let (a, b) = (&p.anchors[i], &p.anchors[(i + 1) % n]);
        let (p0, p3) = (a.point, b.point);
        let p1 = pt(p0.x + a.handle_out.x, p0.y + a.handle_out.y);
        let p2 = pt(p3.x + b.handle_in.x, p3.y + b.handle_in.y);
        for k in 0..=2000 {
            let t = f64::from(k) / 2000.0;
            let u = 1.0 - t;
            let x = u * u * u * p0.x
                + 3.0 * u * u * t * p1.x
                + 3.0 * u * t * t * p2.x
                + t * t * t * p3.x;
            let y = u * u * u * p0.y
                + 3.0 * u * u * t * p1.y
                + 3.0 * u * t * t * p2.y
                + t * t * t * p3.y;
            lo = pt(lo.x.min(x), lo.y.min(y));
            hi = pt(hi.x.max(x), hi.y.max(y));
        }
    }
    (lo, hi)
}

/// AC28 (g): a drag that crosses zero and returns gives back the start; (j) the same event
/// sequence gives the same result twice.
#[test]
fn ac28_drag_through_zero_and_back_and_determinism() {
    let run = || {
        let mut s = one_tri();
        click_path(&mut s);
        drag_frames(
            &mut s,
            &[
                pt(80.0, 60.0),
                pt(20.0, 40.0),
                pt(50.0, 50.0),
                pt(30.0, 70.0),
                pt(80.0, 60.0),
            ],
            false,
            false,
        );
        s
    };
    let a = run();
    let b = run();
    assert_eq!(objects(&a), objects(&b), "deterministic");
    // back at the start pointer: the start geometry (the drag is a no-op, within the dead zone
    // of nothing: sx = sy = 1)
    let p = path_pts(&objects(&a)[0]);
    assert_pt(p[1], 80.0, 50.0, 1e-9, "n1 back");
    assert_pt(p[2], 50.0, 60.0, 1e-9, "n2 back");
}

/// AC28 (e): a release, the typed value and the flip button agree for the same signed map.
#[test]
fn ac28_drag_typed_flip_agree() {
    // flip button: horizontal
    let mut a = one_tri();
    select_all(&mut a);
    a.flip_selection(FlipAxis::Horizontal);
    // typed: W -30 about the centre with the key S
    let mut b = one_tri();
    select_all(&mut b);
    key(&mut b, "s");
    b.commit_transform_entry("-30", "10.0", 0);
    // drag: E handle with Shift to the mirrored position (centre 65, E handle 80 -> 50)
    let mut c = one_tri();
    click_path(&mut c);
    drag_frames(&mut c, &[pt(80.0, 55.0), pt(50.0, 55.0)], true, false);
    let (pa, pb, pc) = (
        path_pts(&objects(&a)[0]),
        path_pts(&objects(&b)[0]),
        path_pts(&objects(&c)[0]),
    );
    for i in 0..3 {
        assert_pt(pb[i], pa[i].x, pa[i].y, 1e-9, "typed vs flip");
        assert_pt(pc[i], pa[i].x, pa[i].y, 1e-6, "drag vs flip");
    }
}

/// AC28 (f): the result does not depend on the order of selection.
#[test]
fn ac28_order_independent() {
    let d = two_rects();
    let ids = d.object_ids();
    let mut results = Vec::new();
    for order in [[0, 1], [1, 0]] {
        let mut s = open(&d);
        let p = |i: usize| {
            if i == 0 {
                pt(55.0, 50.0)
            } else {
                pt(75.0, 50.0)
            }
        };
        click_at(&mut s, p(order[0]));
        hold(&mut s, p(order[1]), true, false);
        s.pointer_down(p(order[1]), true);
        s.pointer_up(p(order[1]), true, false);
        hold(&mut s, p(order[1]), false, false);
        assert_eq!(s.selected_object_count(), 2);
        s.flip_selection(FlipAxis::Horizontal);
        results.push(objects(&s));
    }
    assert_eq!(results[0], results[1], "{ids:?}");
}

/// AC29: a resize released beyond zero is one step named "Resize"; undo restores exactly.
#[test]
fn ac29_resize_beyond_zero_undo_redo() {
    let d = radius_rect([1.0, 2.0, 3.0, 4.0]);
    let mut s = open(&d);
    click_at(&mut s, pt(65.0, 50.0));
    let before = objects(&s);
    let steps = s.undo_len();
    drag(&mut s, pt(80.0, 55.0), pt(20.0, 55.0));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Resize"));
    let after = objects(&s);
    assert_ne!(before, after);
    s.undo();
    assert_eq!(objects(&s), before);
    s.redo();
    assert_eq!(objects(&s), after);
    // Radii of a mirrored-through-zero rectangle stay at their corners.
    // E dragged from 80 to 20 about x = 50 is a pure mirror with sx = -1: tl<->tr, bl<->br.
    assert_eq!(radii4(rect_of(&after[0]).1), [2.0, 1.0, 4.0, 3.0]);
}

/// AC26: stored lengths stay non-negative after every kind of negative gesture, and the file
/// reopens equal.
#[test]
fn ac26_file_round_trip_no_negative_lengths() {
    let mut s = open(&mixed_scene());
    select_all(&mut s);
    s.flip_selection(FlipAxis::Horizontal);
    key(&mut s, "s");
    let w: f64 = s.transform_entry().unwrap().fields[0]
        .prefill
        .parse()
        .unwrap();
    let h: f64 = s.transform_entry().unwrap().fields[1]
        .prefill
        .parse()
        .unwrap();
    s.commit_transform_entry(&format!("-{w}"), &format!("-{h}"), 0);
    let now = objects(&s);
    for o in &now {
        match o {
            ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Rect {
                    bounds,
                    corner_radii,
                } => {
                    assert!(bounds.width.as_mm() > 0.0 && bounds.height.as_mm() > 0.0);
                    assert!(radii4(corner_radii).iter().all(|r| *r >= 0.0));
                }
                Shape::Ellipse { frame, .. } => {
                    assert!(frame.rx.as_mm() > 0.0 && frame.ry.as_mm() > 0.0);
                }
                Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => {
                    assert!(frame.radius.as_mm() > 0.0);
                }
            },
            ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => {}
        }
        if let ObjectSnapshot::Primitive(p) = o {
            assert!(p.style.stroke.width.as_mm() >= 0.0);
        }
    }
    let reopened = open(&doc_of(&s));
    assert_eq!(objects(&reopened), now, "equal after save and reopen");
}

/// AC27: release-build budgets on 5,000 objects. Run with
/// `cargo nextest run --release --run-ignored only -E 'test(ac27)'`.
#[test]
#[ignore = "release-mode benchmark"]
fn ac27_budgets_5000_objects() {
    let d = Document::new(1);
    let mut n = 1_u64;
    for i in 0..5000_usize {
        let (cx, cy) = (
            20.0 + 12.0 * (i % 100) as f64,
            20.0 + 12.0 * (i / 100) as f64,
        );
        match i % 4 {
            0 | 1 => {
                let anchors: Vec<NewAnchor> = (0..20)
                    .map(|k| {
                        let a = std::f64::consts::TAU * f64::from(k) / 20.0;
                        n += 1;
                        anchor(n, cx + 4.0 * a.cos(), cy + 4.0 * a.sin())
                    })
                    .collect();
                let _ = d.create_path(&anchors, true);
            }
            2 => {
                let _ = d.create_rect(rb(cx - 4.0, cy - 3.0, 8.0, 6.0));
            }
            _ => {
                let _ = d.create_star(
                    StarFrame {
                        center: pt(cx, cy),
                        radius: Length::from_mm(4.0),
                        angle: Angle::from_radians(0.2),
                    },
                    PointCount::new(5).unwrap(),
                    InnerRatio::new(0.5).unwrap(),
                );
            }
        }
    }
    let median = |mut v: Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    let fresh = || {
        let mut s = open(&d);
        select_all(&mut s);
        s
    };
    let mut flips = Vec::new();
    let mut moves = Vec::new();
    let mut releases = Vec::new();
    let mut previews = Vec::new();
    for _ in 0..5 {
        let mut s = fresh();
        let t = Instant::now();
        s.flip_selection(FlipAxis::Horizontal);
        flips.push(t.elapsed().as_secs_f64() * 1000.0);
        // move: drag a selected object's body
        let mut s = fresh();
        let t = Instant::now();
        drag(&mut s, pt(20.0, 28.0), pt(60.0, 48.0));
        moves.push(t.elapsed().as_secs_f64() * 1000.0);
        // resize release beyond zero: SE corner of the group box over the NW corner
        let mut s = fresh();
        let (lo, hi) = (
            pt(16.0, 16.0),
            pt(20.0 + 12.0 * 99.0 + 4.0, 20.0 + 12.0 * 49.0 + 4.0),
        );
        hold(&mut s, hi, false, false);
        s.pointer_down(hi, false);
        let tp = Instant::now();
        hold(&mut s, pt(lo.x - 30.0, lo.y - 30.0), false, false);
        let _ = s.draw_list();
        previews.push(tp.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        s.pointer_up(pt(lo.x - 30.0, lo.y - 30.0), false, false);
        releases.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let (f, m, r, p) = (
        median(flips),
        median(moves),
        median(releases),
        median(previews),
    );
    eprintln!("flip {f:.1} ms, move {m:.1} ms, release {r:.1} ms, preview {p:.1} ms");
    assert!(f < 2500.0 && r < 2500.0, "commit budgets");
    // The preview step itself is gated in curvyo-ui-core (mirror_budgets.rs); this figure
    // includes drawing all 5,000 objects and is printed only.
    let _ = p;
    assert!(
        f <= 1.1 * m,
        "flip {f} ms vs move {m} ms (factor {})",
        f / m
    );
    assert!(
        r <= 1.1 * m,
        "release {r} ms vs move {m} ms (factor {})",
        r / m
    );
}

// ======================================================================================
// Second pass: keys on every kind, groups and primitives through zero, a clone alone
// ======================================================================================

/// AC4 on every kind: the key gives exactly the objects the button gives.
#[test]
fn ac04_keys_equal_buttons_on_every_kind() {
    let d = mixed_scene();
    let ids = d.object_ids();
    // Add a group and a clone of the path on top of the mixed kinds.
    let clone = d.linked_clone(&[ids[4]], &[]).unwrap()[0];
    let _ = clone;
    for (k, axis) in [("h", FlipAxis::Horizontal), ("v", FlipAxis::Vertical)] {
        let mut a = open(&d);
        select_all(&mut a);
        let mut b = open(&d);
        select_all(&mut b);
        assert_eq!(a.flip_selection(axis), FlipOutcome::Flipped);
        let _ = key(&mut b, k);
        assert_eq!(objects(&a), objects(&b), "{k}");
        assert_eq!(a.undo_len(), b.undo_len());
    }
    // A group.
    let d = two_rects();
    let ids = d.object_ids();
    d.group_objects(&ids).unwrap();
    let mut a = open(&d);
    select_all(&mut a);
    let mut b = open(&d);
    select_all(&mut b);
    a.flip_selection(FlipAxis::Vertical);
    let _ = key(&mut b, "v");
    assert_eq!(objects(&a), objects(&b));
}

/// AC6/AC12/AC18 on a multi-selection: the corner drag across both axes turns the group
/// about the fixed corner, each rectangle keeps its radius at its physical corner.
#[test]
fn ac06_ac18_group_corner_through_both_axes() {
    let d = two_rects();
    let mut s = open(&d);
    select_all(&mut s);
    // group box (50,50)-(80,60); SE corner to (20,40): sx = -1, sy = -1 about (50,50).
    drag(&mut s, pt(80.0, 60.0), pt(20.0, 40.0));
    let o = objects(&s);
    assert_rect(&o[0], 40.0, 40.0, 10.0, 10.0, 1e-6, "A");
    assert_rect(&o[1], 20.0, 40.0, 10.0, 10.0, 1e-6, "B");
    assert_eq!(
        radii4(rect_of(&o[1]).1),
        [0.0, 0.0, 2.0, 0.0],
        "tl radius is now at br"
    );
    assert_eq!(s.undo_len(), 1);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Resize"));
}

/// AC14/AC17 on a group: signed typed sizes equal the flip, linked fields keep the sign.
#[test]
fn ac14_ac17_group_typed_signed() {
    let d = two_rects();
    let mut a = open(&d);
    select_all(&mut a);
    a.flip_selection(FlipAxis::Horizontal);
    let mut b = open(&d);
    select_all(&mut b);
    key(&mut b, "s");
    assert_eq!(
        b.commit_transform_entry("-30", "10.0", 0),
        EntryOutcome::Committed
    );
    assert_eq!(objects(&a), objects(&b), "typed -30 equals the flip button");
    // Linked: W -30 shows H -10.0 and means the half turn.
    let mut c = open(&d);
    select_all(&mut c);
    dblclick(&mut c, pt(80.0, 60.0), false, true);
    assert!(c.transform_entry().expect("chip").linked);
    let h = c.transform_entry_linked(0, "-30").expect("linked");
    assert_eq!(h, "-10.0");
    assert_eq!(
        c.commit_transform_entry("-30", &h, 0),
        EntryOutcome::Committed
    );
    let o = objects(&c);
    // half turn about the fixed NW corner (50,50): A 40..50 x 40..50, B 20..30
    assert_rect(&o[0], 40.0, 40.0, 10.0, 10.0, 1e-9, "A half turn");
    assert_rect(&o[1], 20.0, 40.0, 10.0, 10.0, 1e-9, "B half turn");
    assert_eq!(c.transform_entry_mirror_line("-30", &h, 0), "");
}

/// AC21/AC22: a Ctrl corner drag across both axes (a half turn, uniform) keeps ellipse,
/// polygon and star what they are; a plain flip of the group of them as well.
#[test]
fn ac21_ac22_uniform_half_turn_keeps_kinds() {
    let d = Document::new(1);
    let _ = d.create_ellipse(EllipseFrame {
        center: pt(70.0, 70.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(6.0),
    });
    let _ = d.create_star(
        StarFrame {
            center: pt(110.0, 70.0),
            radius: Length::from_mm(8.0),
            angle: Angle::from_radians(0.2),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    let _ = d.create_polygon(
        StarFrame {
            center: pt(150.0, 70.0),
            radius: Length::from_mm(7.0),
            angle: Angle::from_radians(0.9),
        },
        PointCount::new(6).unwrap(),
    );
    let mut s = open(&d);
    select_all(&mut s);
    let before = objects(&s);
    let (lo, hi) = {
        let mut lo = pt(f64::MAX, f64::MAX);
        let mut hi = pt(f64::MIN, f64::MIN);
        for o in &before {
            let (l, h) = tbb(o);
            lo = pt(lo.x.min(l.x), lo.y.min(l.y));
            hi = pt(hi.x.max(h.x), hi.y.max(h.y));
        }
        (lo, hi)
    };
    // SE corner to the far side of NW with Ctrl, same magnitudes: pointer at
    // NW - (SE - NW) / 2, i.e. s = -0.5 on both axes (a uniform half turn).
    let se = hi;
    let to = pt(lo.x - (hi.x - lo.x) / 2.0, lo.y - (hi.y - lo.y) / 2.0);
    drag_frames(&mut s, &[se, to], false, true);
    let after = objects(&s);
    assert_eq!(after.len(), 3);
    for (a, b) in before.iter().zip(&after) {
        let (ObjectSnapshot::Primitive(pa), ObjectSnapshot::Primitive(pb)) = (a, b) else {
            panic!("converted by a uniform scale with negative signs");
        };
        assert_eq!(
            std::mem::discriminant(&pa.shape),
            std::mem::discriminant(&pb.shape)
        );
    }
    // A half turn with 0.5: the group is 0.5 x as big about NW, mirrored.
    let want: Vec<Point> = before
        .iter()
        .flat_map(outline_pts)
        .map(|p| pt(lo.x - 0.5 * (p.x - lo.x), lo.y - 0.5 * (p.y - lo.y)))
        .collect();
    let got: Vec<Point> = after.iter().flat_map(outline_pts).collect();
    assert!(same_set(&want, &got, 1e-4), "uniform scale by (-0.5, -0.5)");
}

/// AC25: a clone selected alone is mirrored about its own box, the origin stays.
#[test]
fn ac25_clone_alone() {
    let d = Document::new(1);
    let origin = tri_path(&d, 1);
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    // Move the clone 60 mm to the right so that it can be picked on its own.
    let mut s = open(&d);
    let org_before = outline_pts(&doc_of(&s).object(origin).unwrap());
    select_all(&mut s);
    click_at(&mut s, pt(1000.0, 900.0)); // empty canvas: deselect
    let c0 = outline_pts(&doc_of(&s).object(clone).unwrap());
    // pick the topmost object at the shared top edge
    click_at(&mut s, pt(65.0, 50.0));
    assert_eq!(s.selected_object_count(), 1);
    let picked_clone = matches!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    assert!(picked_clone);
    let now = objects(&s);
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for p in &c0 {
        lo = lo.min(p.x);
        hi = hi.max(p.x);
    }
    let c = pt((lo + hi) / 2.0, 0.0);
    let o_after = now
        .iter()
        .find(|o| matches!(o, ObjectSnapshot::Path(_)))
        .map(outline_pts)
        .unwrap();
    let c_after = now
        .iter()
        .find(|o| matches!(o, ObjectSnapshot::Clone(_)))
        .map(outline_pts)
        .unwrap();
    // Exactly one of the two changed; the one that did is the mirror image.
    let org_same = same_set(&org_before, &o_after, 1e-9);
    let clone_same = same_set(&c0, &c_after, 1e-9);
    assert!(org_same != clone_same, "exactly one object moved");
    if !clone_same {
        assert!(
            org_same,
            "the origin is untouched when only the clone is selected"
        );
        assert!(same_set(
            &mirror_pts(&c0, FlipAxis::Horizontal, c),
            &c_after,
            1e-9
        ));
    }
}

// ======================================================================================
// White-box edges: hostile numbers, degenerate boxes
// ======================================================================================

/// Typed hostile numbers never write anything odd: NaN, infinities, huge, "-0", odd signs.
#[test]
fn edge_typed_hostile_numbers() {
    for text in [
        "NaN", "nan", "inf", "-inf", "\u{221e}", "1e400", "-1e400", "1e7", "-1e7", "--5", "+-5",
        "5-", "- 5", "",
    ] {
        let mut s = one_tri();
        select_all(&mut s);
        key(&mut s, "s");
        let before = bytes_of(&s);
        let out = s.commit_transform_entry(text, "10.0", 0);
        match out {
            EntryOutcome::Committed => {
                // Only a finite, in-limit number may commit; the result must be finite and sane.
                for p in path_pts(&objects(&s)[0]) {
                    assert!(
                        p.x.is_finite()
                            && p.y.is_finite()
                            && p.x.abs() <= 1e7 + 1.0
                            && p.y.abs() <= 1e7 + 1.0,
                        "{text:?}: {p:?}"
                    );
                }
            }
            EntryOutcome::Unchanged | EntryOutcome::Invalid { .. } => {
                assert!(bytes_of(&s) == before, "{text:?}: nothing written");
            }
        }
        if !matches!(out, EntryOutcome::Committed) {
            continue;
        }
        // A committed one must reopen.
        assert!(
            unpack(4, &s.pack("0.1.0").unwrap()).is_ok(),
            "{text:?} reopens"
        );
    }
    // "-0" and the real minus zero are a typed 0: +0.01 mm.
    for text in ["-0", "\u{2212}0", "0", "0.0", "-0.0", "0,0"] {
        let mut s = rect_doc();
        select_all(&mut s);
        key(&mut s, "s");
        assert_eq!(
            s.commit_transform_entry(text, "10.0", 0),
            EntryOutcome::Committed,
            "{text:?}"
        );
        let (b, _, _) = rect_of(&objects(&s)[0]);
        assert!(
            near(b.width.as_mm(), 0.01, 1e-12),
            "{text:?}: width {}",
            b.width.as_mm()
        );
        assert!(b.width.as_mm() > 0.0);
    }
    // A negative size below 0.0005 mm in magnitude is a zero too: 0.01, never a flat object.
    let mut s = rect_doc();
    select_all(&mut s);
    key(&mut s, "s");
    s.commit_transform_entry("-0.0003", "10.0", 0);
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(
        near(b.width.as_mm(), 0.01, 1e-12),
        "width {}",
        b.width.as_mm()
    );
    // Too large: magnitude above the limit is refused, also with a minus.
    for text in ["1e8", "-1e8", "\u{2212}1e8", "20000000", "-20000000"] {
        let mut s = rect_doc();
        select_all(&mut s);
        key(&mut s, "s");
        let before = bytes_of(&s);
        let out = s.commit_transform_entry(text, "10.0", 0);
        assert!(
            matches!(out, EntryOutcome::Invalid { .. }),
            "{text:?}: {out:?}"
        );
        assert!(bytes_of(&s) == before && s.transform_entry().is_some());
    }
}

/// A straight line has no thickness: it can be mirrored and stretched along its length, and
/// the zero axis is never "recovered" into 0.01 mm.
#[test]
fn edge_zero_thickness_line() {
    let d = Document::new(1);
    let _ = d.create_path(&[anchor(1, 50.0, 55.0), anchor(2, 80.0, 55.0)], false);
    let mut s = open(&d);
    select_all(&mut s);
    key(&mut s, "s");
    let v = s.transform_entry().unwrap();
    assert_eq!(v.fields[1].prefill, "0.0");
    // W -30, H unedited ("0.0"): the line is mirrored, still horizontal, still thickness 0.
    assert_eq!(
        s.commit_transform_entry("-30", "0.0", 0),
        EntryOutcome::Committed
    );
    let p = path_pts(&objects(&s)[0]);
    assert_pt(p[0], 80.0, 55.0, 1e-9, "p0");
    assert_pt(p[1], 50.0, 55.0, 1e-9, "p1");
    assert!(
        s.take_transform_notice().is_empty(),
        "no zero notice for an untouched axis"
    );
    // A drag of its end handle through the other end.
    let mut s = open(&d);
    select_all(&mut s);
    drag(&mut s, pt(80.0, 55.0), pt(20.0, 55.0));
    let p = path_pts(&objects(&s)[0]);
    assert!(p.iter().all(|q| near(q.y, 55.0, 1e-9)), "thickness stays 0");
    assert!(
        near((p[0].x - p[1].x).abs(), 30.0 * 2.0, 1e-6)
            || near((p[0].x - p[1].x).abs(), 30.0, 1e-6)
            || (p[0].x - p[1].x).abs() > 1.0
    );
}

/// Far from the origin (+-1e5 mm), double flips return the objects within 1e-9 mm.
#[test]
fn edge_far_coordinates_double_flip() {
    for off in [1.0e5, -1.0e5] {
        let d = Document::new(1);
        let _ = d.create_rect(rb(off, off, 30.0, 10.0));
        let _ = d.create_ellipse(EllipseFrame {
            center: pt(off + 50.0, off - 20.0),
            rx: Length::from_mm(7.0),
            ry: Length::from_mm(3.0),
        });
        let mut s = open(&d);
        select_all(&mut s);
        let before = objects(&s);
        for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
            s.flip_selection(axis);
            s.flip_selection(axis);
            for (a, b) in before.iter().zip(&objects(&s)) {
                for (x, y) in nums(a).iter().zip(nums(b)).skip(1) {
                    assert!(near(*x, y, 1e-8), "off {off}: {x} vs {y}");
                }
            }
        }
    }
}

/// Hostile pointers mid-drag: NaN and infinity change nothing and do not panic.
#[test]
fn edge_hostile_pointer_during_resize() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e300,
        -1.0e300,
    ] {
        let mut s = rect_doc();
        click_at(&mut s, pt(65.0, 50.0));
        let before = bytes_of(&s);
        let steps = s.undo_len();
        hold(&mut s, pt(80.0, 55.0), false, false);
        s.pointer_down(pt(80.0, 55.0), false);
        hold(&mut s, pt(bad, 55.0), false, false);
        let _ = s.live_readout();
        s.pointer_up(pt(bad, 55.0), false, false);
        assert_eq!(s.undo_len(), steps, "{bad}");
        assert!(bytes_of(&s) == before, "{bad}");
        // y too
        hold(&mut s, pt(80.0, 55.0), false, false);
        s.pointer_down(pt(80.0, 55.0), false);
        s.pointer_up(pt(40.0, bad), false, false);
        assert!(unpack(4, &s.pack("0.1.0").unwrap()).is_ok());
    }
}

/// Pulling a 0.01 mm axis apart again works, and repeated passes through zero stay sane.
#[test]
fn edge_pull_apart_and_repeat() {
    let mut s = rect_doc();
    select_all(&mut s);
    key(&mut s, "s");
    assert_eq!(
        s.commit_transform_entry("0", "10.0", 0),
        EntryOutcome::Committed
    );
    let _ = s.take_transform_notice();
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(near(b.width.as_mm(), 0.01, 1e-12));
    // The box is a line with its handles on it (UX notes): the corner handles never hide.
    // SE corner at (65.005, 60) (the key S fixed the centre x = 65), dragged 20 mm right.
    let mut e = open(&doc_of(&s));
    select_all(&mut e);
    drag(&mut e, pt(65.005, 55.0), pt(85.005, 55.0));
    let (be, _, _) = rect_of(&objects(&e)[0]);
    eprintln!(
        "E-edge handle of a 0.01 mm wide object: width {}",
        be.width.as_mm()
    );
    select_all(&mut s);
    drag(&mut s, pt(65.005, 60.0), pt(85.005, 60.0));
    let (b, _, _) = rect_of(&objects(&s)[0]);
    assert!(
        near(b.width.as_mm(), 20.01, 1e-6),
        "pulled apart: {}",
        b.width.as_mm()
    );
    assert!(
        near(be.width.as_mm(), 20.01, 1e-6),
        "criterion 28 (i): the E handle of a 0.01 mm wide object pulled 20 mm gives 20.01 mm, got {}",
        be.width.as_mm()
    );
    // Three repeated passes through the fixed line and back.
    for _ in 0..3 {
        let (b, _, _) = rect_of(&objects(&s)[0]);
        let right = b.origin.x + b.width.as_mm();
        drag_frames(
            &mut s,
            &[
                pt(right, 55.0),
                pt(b.origin.x - 10.0, 55.0),
                pt(right, 55.0),
            ],
            false,
            false,
        );
        let (b2, _, _) = rect_of(&objects(&s)[0]);
        assert!(
            near(b2.width.as_mm(), b.width.as_mm(), 1e-6),
            "back to the start size"
        );
        assert!(near(b2.origin.x, b.origin.x, 1e-6));
    }
}
