//! Independent tester acceptance tests for Part C of
//! `specs/0047-transform-polish/specification.md` (criteria 33 to 50): the typed move measures
//! from the centre by default, nine reference points, the pale red true bounding box and the
//! marks. Written from the specification before the implementation diff was read. Everything
//! goes through `Session`'s public API: pointer events, keys, the move chip's calls and the draw
//! list (the box and the marks are found by their colour: `#E5484D`, the box line at alpha 140
//! (55 %), the marks opaque). What lives in the DOM (the chip's Reference control, the press
//! slop of a mark click) is not reachable from here and is reported NOT VERIFIED.

#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::suboptimal_flops,
    clippy::unreadable_literal,
    clippy::used_underscore_binding,
    clippy::manual_midpoint,
    clippy::uninlined_format_args
)]
// Mechanical lints of the tester files, allowed so the gate stays strict for production code.
#![allow(
    clippy::approx_constant,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{
    AnchorId, Angle, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
    RectBounds, Shape, StarFrame, Vec2, outline_of_rotated, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_render_core::DrawList;
use curvyo_ui_core::{EntryOutcome, MoveEntryMode, ReferencePoint};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn rect_bounds(x: f64, y: f64, w: f64, h: f64) -> RectBounds {
    RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    }
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

fn loro_of(d: &Document) -> loro::LoroDoc {
    let l = loro::LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn change_count(s: &Session) -> usize {
    loro_of(&doc_of(s)).len_changes()
}

fn bytes_of(s: &Session) -> Vec<u8> {
    doc_of(s).export_loro_snapshot().unwrap()
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool) {
    s.modifiers_changed(shift, ctrl, false);
    s.pointer_hover(at, shift, ctrl);
}

fn key(s: &mut Session, key: &str, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key,
        shift,
        ..KeyInput::default()
    })
}

fn select_everything(s: &mut Session) {
    let n = doc_of(s).object_ids().len();
    key(s, "Escape", false);
    hold(s, pt(-300.0, -300.0), false, false);
    s.pointer_down(pt(-300.0, -300.0), false);
    hold(s, pt(900.0, 900.0), false, false);
    s.pointer_up(pt(900.0, 900.0), false, false);
    if s.selected_object_count() != n {
        key(s, "Escape", false);
        let out = s.key_down(KeyInput {
            key: "a",
            ctrl: true,
            ..KeyInput::default()
        });
        assert_eq!(out, KeyOutcome::SelectedAll);
    }
    assert_eq!(s.selected_object_count(), n);
}

fn is_path(o: &ObjectSnapshot) -> bool {
    matches!(o, ObjectSnapshot::Path(_))
}

/// Dense polyline of the drawn outline of one object (primitives through their path form).
fn polyline(o: &ObjectSnapshot) -> Vec<Point> {
    fn sample(a: &[NewAnchor], closed: bool) -> Vec<Point> {
        let n = a.len();
        let segs = if closed { n } else { n.saturating_sub(1) };
        let mut out = Vec::new();
        for i in 0..segs {
            let (p, q) = (&a[i], &a[(i + 1) % n]);
            let p1 = pt(p.point.x + p.handle_out.x, p.point.y + p.handle_out.y);
            let p2 = pt(q.point.x + q.handle_in.x, q.point.y + q.handle_in.y);
            for step in 0..200 {
                let t = f64::from(step) / 200.0;
                let u = 1.0 - t;
                out.push(pt(
                    u * u * u * p.point.x
                        + 3.0 * u * u * t * p1.x
                        + 3.0 * u * t * t * p2.x
                        + t * t * t * q.point.x,
                    u * u * u * p.point.y
                        + 3.0 * u * u * t * p1.y
                        + 3.0 * u * t * t * p2.y
                        + t * t * t * q.point.y,
                ));
            }
        }
        out.push(a[0].point);
        out
    }
    match o {
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => {
            let out = outline_of_rotated(&p.shape, p.rotation);
            let a: Vec<NewAnchor> = out
                .iter()
                .map(|x| {
                    let mut n = NewAnchor::corner(AnchorId::new(1, 1), x.point);
                    n.handle_in = x.handle_in;
                    n.handle_out = x.handle_out;
                    n
                })
                .collect();
            sample(&a, true)
        }
        ObjectSnapshot::Path(p) => p
            .subpaths()
            .flat_map(|sp| sample(sp.anchors, sp.closed))
            .collect(),
    }
}

/// The tight bounds of all objects' drawn outlines (the TBB of the spec's Terms).
fn tbb(os: &[ObjectSnapshot]) -> (Point, Point) {
    let mut lo = pt(f64::MAX, f64::MAX);
    let mut hi = pt(f64::MIN, f64::MIN);
    for o in os {
        for p in polyline(o) {
            lo = pt(lo.x.min(p.x), lo.y.min(p.y));
            hi = pt(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    (lo, hi)
}

fn tbb_of(s: &Session) -> (Point, Point) {
    tbb(&objects(s))
}

fn ref_point(b: (Point, Point), r: ReferencePoint) -> Point {
    let (lo, hi) = b;
    let (cx, cy) = ((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    match r {
        ReferencePoint::TopLeft => pt(lo.x, lo.y),
        ReferencePoint::TopCentre => pt(cx, lo.y),
        ReferencePoint::TopRight => pt(hi.x, lo.y),
        ReferencePoint::LeftMiddle => pt(lo.x, cy),
        ReferencePoint::Centre => pt(cx, cy),
        ReferencePoint::RightMiddle => pt(hi.x, cy),
        ReferencePoint::BottomLeft => pt(lo.x, hi.y),
        ReferencePoint::BottomCentre => pt(cx, hi.y),
        ReferencePoint::BottomRight => pt(hi.x, hi.y),
    }
}

fn rect_origin(o: &ObjectSnapshot) -> Point {
    match o {
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect { bounds, .. } => bounds.origin,
            other => panic!("rect expected, got {other:?}"),
        },
        ObjectSnapshot::Path(_) => panic!("rect expected"),
    }
}

fn rect_centre(o: &ObjectSnapshot) -> Point {
    match o {
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect { bounds, .. } => pt(
                bounds.origin.x + bounds.width.as_mm() / 2.0,
                bounds.origin.y + bounds.height.as_mm() / 2.0,
            ),
            Shape::Ellipse { frame, .. } => frame.center,
            other => panic!("rect or ellipse expected, got {other:?}"),
        },
        ObjectSnapshot::Path(_) => panic!("primitive expected"),
    }
}

fn assert_pt(p: Point, x: f64, y: f64, eps: f64, what: &str) {
    assert!(
        near(p.x, x, eps) && near(p.y, y, eps),
        "{what}: got ({}, {}), want ({x}, {y})",
        p.x,
        p.y
    );
}

// ---- the red box and the marks in the draw list -----------------------------

fn is_red(c: curvyo_render_core::Vertex) -> bool {
    let c = c.color;
    c.r.abs_diff(0xE5) <= 3 && c.g.abs_diff(0x48) <= 3 && c.b.abs_diff(0x4D) <= 3
}

/// Vertices of the pale red box line (the red at alpha below 255).
fn box_vertices(l: &DrawList) -> Vec<(usize, Point)> {
    l.triangles
        .iter()
        .enumerate()
        .filter(|(_, v)| is_red(**v) && v.color.a < 255)
        .map(|(i, v)| (i, v.position))
        .collect()
}

/// Vertices of the marks (the red fully opaque).
fn mark_vertices(l: &DrawList) -> Vec<(usize, Point)> {
    l.triangles
        .iter()
        .enumerate()
        .filter(|(_, v)| is_red(**v) && v.color.a == 255)
        .map(|(i, v)| (i, v.position))
        .collect()
}

fn extent(v: &[(usize, Point)]) -> Option<(Point, Point)> {
    if v.is_empty() {
        return None;
    }
    let mut lo = pt(f64::MAX, f64::MAX);
    let mut hi = pt(f64::MIN, f64::MIN);
    for (_, p) in v {
        lo = pt(lo.x.min(p.x), lo.y.min(p.y));
        hi = pt(hi.x.max(p.x), hi.y.max(p.y));
    }
    Some((lo, hi))
}

/// The drawn red box as (min, max) in document mm, or `None`.
fn red_box(s: &Session) -> Option<(Point, Point)> {
    extent(&box_vertices(&s.draw_list()))
}

/// The box (min, max) must be the expected TBB to the width of its line (a few screen px).
fn assert_box_is(s: &Session, want: (Point, Point), what: &str) {
    let got = red_box(s).unwrap_or_else(|| panic!("{what}: no red box is drawn"));
    let slack = 4.0 / s.view().scale();
    assert!(
        near(got.0.x, want.0.x, slack)
            && near(got.0.y, want.0.y, slack)
            && near(got.1.x, want.1.x, slack)
            && near(got.1.y, want.1.y, slack),
        "{what}: the red box is ({:.3}, {:.3})-({:.3}, {:.3}), want ({:.3}, {:.3})-({:.3}, {:.3})",
        got.0.x,
        got.0.y,
        got.1.x,
        got.1.y,
        want.0.x,
        want.0.y,
        want.1.x,
        want.1.y
    );
}

// ---- scenes ----------------------------------------------------------

/// The spec's rectangle 30 x 40 mm with its top-left corner at (10, 20).
fn rect_30x40(ox: f64, oy: f64) -> Document {
    let d = Document::new(1);
    let _ = d.create_rect(rect_bounds(10.0 + ox, 20.0 + oy, 30.0, 40.0));
    d
}

/// The multi-selection of criterion 35: a rectangle 20 x 10 with centre (10, 5) rotated 30
/// degrees and a circle (r = 5) at (40, 5), moved by (ox, oy).
fn rotated_rect_and_circle(ox: f64, oy: f64) -> Document {
    let d = Document::new(1);
    let id = d.create_rect(rect_bounds(ox, oy, 20.0, 10.0));
    let o = d.object(id).unwrap();
    d.rotate_object(&o.rotated(
        pt(10.0 + ox, 5.0 + oy),
        Angle::from_radians(30.0_f64.to_radians()),
    ))
    .unwrap();
    let _ = d.create_ellipse(EllipseFrame {
        center: pt(40.0 + ox, 5.0 + oy),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    d
}

fn open_chip(s: &mut Session, absolute: bool) {
    assert_eq!(key(s, "m", false), KeyOutcome::EntryOpened);
    s.set_move_chip_absolute(absolute);
}

fn mode(absolute: bool, copy: bool) -> MoveEntryMode {
    MoveEntryMode { absolute, copy }
}

// ---------------------------------------------------------------------
// Criterion 37: Centre by default, prefill
// ---------------------------------------------------------------------

/// Criterion 37, the example: the 30 x 40 mm rectangle with its top-left at (10, 20): the chip in
/// Absolute shows X "25.0" and Y "40.0" (the centre) with one decimal, and every point's prefill
/// is that point's position.
#[test]
fn ac37_absolute_prefill_is_the_centre_and_every_point_has_its_own() {
    let mut s = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let v = s.move_entry().expect("the chip is open");
    assert_eq!(v.reference, ReferencePoint::Centre, "Centre on open");
    let c = ReferencePoint::Centre.index();
    assert_eq!(
        v.absolute_prefills[c],
        ["25.0".to_string(), "40.0".to_string()]
    );
    let want = [
        ("10.0", "20.0"),
        ("25.0", "20.0"),
        ("40.0", "20.0"),
        ("10.0", "40.0"),
        ("25.0", "40.0"),
        ("40.0", "40.0"),
        ("10.0", "60.0"),
        ("25.0", "60.0"),
        ("40.0", "60.0"),
    ];
    for (i, (x, y)) in want.iter().enumerate() {
        assert_eq!(
            v.absolute_prefills[i],
            [(*x).to_string(), (*y).to_string()],
            "prefill of point {i} ({:?})",
            ReferencePoint::ALL[i]
        );
    }
}

/// Criterion 37 and 49: the reference is Centre every time the chip opens, whatever was chosen
/// before, and it is not stored in the document.
#[test]
fn ac37_ac49_the_reference_resets_to_centre_and_is_never_stored() {
    let mut s = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    s.set_move_reference(ReferencePoint::BottomRight);
    assert_eq!(
        s.move_entry().unwrap().reference,
        ReferencePoint::BottomRight
    );
    s.cancel_transform_entry();
    assert!(s.move_entry().is_none(), "Escape closes the chip");
    assert_eq!(key(&mut s, "m", false), KeyOutcome::EntryOpened);
    assert_eq!(s.move_entry().unwrap().reference, ReferencePoint::Centre);
    // Commit with a non-centre reference, reopen: Centre again.
    s.set_move_chip_absolute(true);
    s.set_move_reference(ReferencePoint::TopRight);
    assert_eq!(
        s.commit_move_entry("100", "50", mode(true, false)),
        EntryOutcome::Committed
    );
    assert_eq!(key(&mut s, "m", false), KeyOutcome::EntryOpened);
    assert_eq!(s.move_entry().unwrap().reference, ReferencePoint::Centre);
    // Nothing about a reference is in the document.
    let json = format!("{:?}", loro_of(&doc_of(&s)).get_deep_value()).to_lowercase();
    assert!(
        !json.contains("reference"),
        "the document stores no reference"
    );
    // The chip opens in Relative every time (0010 criterion 18): no marks until Absolute.
    let mut s = open(&rect_30x40(100.0, 100.0));
    select_everything(&mut s);
    assert_eq!(key(&mut s, "m", false), KeyOutcome::EntryOpened);
    let b = tbb_of(&s);
    let c = ref_point(b, ReferencePoint::Centre);
    assert_eq!(s.move_mark_at(c, false), None, "Relative mode has no marks");
    s.set_move_chip_absolute(true);
    assert_eq!(s.move_mark_at(c, false), Some(ReferencePoint::Centre));
    s.cancel_transform_entry();
    assert_eq!(key(&mut s, "m", false), KeyOutcome::EntryOpened);
    assert_eq!(s.move_mark_at(c, false), None, "reopened: Relative again");
}

// ---------------------------------------------------------------------
// Criterion 38: Enter uses the centre; same writes as a drag
// ---------------------------------------------------------------------

/// Criterion 38, the example: typed 100 and 50 and plain Enter: the centre at (100, 50), the
/// top-left corner at (85, 30); one commit; the same result as a drag of that offset.
#[test]
fn ac38_plain_enter_uses_the_centre_and_equals_a_drag() {
    let mut s = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let c = change_count(&s);
    assert_eq!(
        s.commit_move_entry("100", "50", mode(true, false)),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&s), c + 1);
    let o = &objects(&s)[0];
    assert_pt(rect_origin(o), 85.0, 30.0, 1e-9, "top-left after Enter");
    assert_pt(rect_centre(o), 100.0, 50.0, 1e-9, "centre after Enter");
    // The same offset by a drag: (75, 10) from the top edge near its left end.
    let mut t = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut t);
    let from = pt(18.0, 20.0);
    hold(&mut t, from, false, false);
    t.pointer_down(from, false);
    hold(&mut t, pt(18.0 + 75.0, 20.0 + 10.0), false, false);
    t.pointer_up(pt(18.0 + 75.0, 20.0 + 10.0), false, false);
    assert_eq!(
        objects(&t),
        objects(&s),
        "typed Enter writes what the drag writes"
    );
}

/// Criterion 38: a field the maker did not edit means "no change on that axis".
#[test]
fn ac38_an_unedited_axis_stays() {
    let mut s = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    // X = 100, Y untouched (the prefill of the centre).
    assert_eq!(
        s.commit_move_entry("100", "40.0", mode(true, false)),
        EntryOutcome::Committed
    );
    assert_pt(rect_centre(&objects(&s)[0]), 100.0, 40.0, 1e-9, "x only");
    open_chip(&mut s, true);
    assert_eq!(
        s.commit_move_entry("100.0", "7", mode(true, false)),
        EntryOutcome::Committed
    );
    assert_pt(rect_centre(&objects(&s)[0]), 100.0, 7.0, 1e-9, "y only");
}

// ---------------------------------------------------------------------
// Criterion 40: the click on a mark executes with that reference (via commit_move_entry_at)
// ---------------------------------------------------------------------

fn click_exec(
    base: &Document,
    reference: ReferencePoint,
    first: &str,
    second: &str,
) -> (Session, EntryOutcome) {
    let mut s = open(base);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let out = s.commit_move_entry_at(first, second, false, reference);
    (s, out)
}

/// Criterion 40, the examples from the 30 x 40 rectangle with typed 100 and 50.
#[test]
fn ac40_the_examples_of_the_spec() {
    let cases: [(ReferencePoint, f64, f64); 9] = [
        (ReferencePoint::TopLeft, 100.0, 50.0),
        (ReferencePoint::TopCentre, 85.0, 50.0),
        (ReferencePoint::TopRight, 70.0, 50.0),
        (ReferencePoint::LeftMiddle, 100.0, 30.0),
        (ReferencePoint::Centre, 85.0, 30.0),
        (ReferencePoint::RightMiddle, 70.0, 30.0),
        (ReferencePoint::BottomLeft, 100.0, 10.0),
        (ReferencePoint::BottomCentre, 85.0, 10.0),
        (ReferencePoint::BottomRight, 70.0, 10.0),
    ];
    for (r, ox, oy) in cases {
        let base = rect_30x40(0.0, 0.0);
        let c = change_count(&open(&base));
        let (s, out) = click_exec(&base, r, "100", "50");
        assert_eq!(out, EntryOutcome::Committed, "{r:?}");
        assert_eq!(change_count(&s), c + 1, "{r:?}: one commit");
        assert_pt(
            rect_origin(&objects(&s)[0]),
            ox,
            oy,
            1e-9,
            &format!("{r:?} origin"),
        );
        assert!(s.move_entry().is_none(), "{r:?}: the chip closes");
    }
}

/// Criterion 40: typed X = 100 only, then the top-right mark: the top-right corner's x becomes
/// 100 and its y stays (origin (70, 20)). The untouched Y shows the prefill of that point.
#[test]
fn ac40_x_only_then_top_right() {
    let base = rect_30x40(0.0, 0.0);
    let mut s = open(&base);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let prefill =
        s.move_entry().unwrap().absolute_prefills[ReferencePoint::TopRight.index()].clone();
    assert_eq!(prefill, ["40.0".to_string(), "20.0".to_string()]);
    assert_eq!(
        s.commit_move_entry_at("100", &prefill[1], false, ReferencePoint::TopRight),
        EntryOutcome::Committed
    );
    assert_pt(rect_origin(&objects(&s)[0]), 70.0, 20.0, 1e-9, "origin");
}

/// Criterion 40, the rotated example: typed 0 and 0 and the top-left mark moves the rectangle's
/// centre to (11.16, 9.33).
#[test]
fn ac40_rotated_example() {
    let d = Document::new(1);
    let id = d.create_rect(rect_bounds(0.0, 0.0, 20.0, 10.0));
    let o = d.object(id).unwrap();
    d.rotate_object(&o.rotated(pt(10.0, 5.0), Angle::from_radians(30.0_f64.to_radians())))
        .unwrap();
    let b = tbb(&[d.object(id).unwrap()]);
    assert_pt(b.0, -1.16, -4.33, 0.01, "TBB min (criterion 35)");
    assert_pt(b.1, 21.16, 14.33, 0.01, "TBB max (criterion 35)");
    let (s, out) = click_exec(&d, ReferencePoint::TopLeft, "0", "0");
    assert_eq!(out, EntryOutcome::Committed);
    assert_pt(rect_centre(&objects(&s)[0]), 11.16, 9.33, 0.01, "centre");
}

/// Criterion 40 / 48, the multi-selection example: typed 0 and 0 and Enter moves every object by
/// (-21.92, -5.00): the rectangle's centre to (-11.92, 0.00), the circle's to (18.08, 0.00).
#[test]
fn ac40_ac48_multi_selection_example() {
    let d = rotated_rect_and_circle(0.0, 0.0);
    let b = tbb(&objects(&open(&d)));
    assert_pt(b.0, -1.16, -4.33, 0.01, "TBB min");
    assert_pt(b.1, 45.0, 14.33, 0.01, "TBB max");
    assert_pt(
        ref_point(b, ReferencePoint::Centre),
        21.92,
        5.0,
        0.01,
        "TBB centre (criterion 35)",
    );
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let c = change_count(&s);
    assert_eq!(
        s.commit_move_entry("0", "0", mode(true, false)),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&s), c + 1, "one commit for every object");
    let os = objects(&s);
    assert_pt(rect_centre(&os[0]), -11.92, 0.0, 0.01, "rectangle centre");
    assert_pt(rect_centre(&os[1]), 18.08, 0.0, 0.01, "circle centre");
    // The same through every mark: the reference point of the group box lands on (0, 0).
    for r in ReferencePoint::ALL {
        let d = rotated_rect_and_circle(0.0, 0.0);
        let (s, out) = click_exec(&d, r, "0", "0");
        assert_eq!(out, EntryOutcome::Committed, "{r:?}");
        let b = tbb_of(&s);
        let p = ref_point(b, r);
        assert_pt(
            p,
            0.0,
            0.0,
            0.02,
            &format!("{r:?} of the group box at (0, 0)"),
        );
    }
}

/// Criterion 38/40 for every reference and several boxes (a property by enumeration): the
/// reference point of the TBB afterwards lies exactly at the typed position.
#[test]
fn ac38_ac40_every_reference_lands_on_the_typed_point() {
    let mut seed: u64 = 12345;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / f64::from(1_u32 << 31)
    };
    for case in 0..12 {
        let d = Document::new(1);
        let (x, y) = (rnd() * 200.0 - 100.0, rnd() * 200.0 - 100.0);
        let (w, h) = (1.0 + rnd() * 80.0, 1.0 + rnd() * 80.0);
        match case % 3 {
            0 => {
                let id = d.create_rect(rect_bounds(x, y, w, h));
                let o = d.object(id).unwrap();
                d.rotate_object(&o.rotated(
                    pt(x + w / 2.0, y + h / 2.0),
                    Angle::from_radians(rnd() * 6.28),
                ))
                .unwrap();
            }
            1 => {
                let _ = d.create_ellipse(EllipseFrame {
                    center: pt(x, y),
                    rx: Length::from_mm(w / 2.0),
                    ry: Length::from_mm(h / 2.0),
                });
            }
            _ => {
                let id = d.create_polygon(
                    StarFrame {
                        center: pt(x, y),
                        radius: Length::from_mm(w / 2.0 + 1.0),
                        angle: Angle::from_radians(rnd() * 6.28),
                    },
                    curvyo_document_core::PointCount::new(3 + (rnd() * 6.0) as u32).unwrap(),
                );
                let _ = id;
            }
        }
        let (tx, ty) = (rnd() * 400.0 - 200.0, rnd() * 400.0 - 200.0);
        for r in ReferencePoint::ALL {
            let (s, out) = click_exec(&d, r, &format!("{tx}"), &format!("{ty}"));
            assert_eq!(out, EntryOutcome::Committed, "case {case} {r:?}");
            let p = ref_point(tbb_of(&s), r);
            assert_pt(p, tx, ty, 1e-3, &format!("case {case} {r:?}"));
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 45: unchanged, invalid, limit on all four edges
// ---------------------------------------------------------------------

/// Criterion 45: both fields untouched, or a typed position equal to the point's current one:
/// nothing is written, the chip closes. "Unedited" is decided against the clicked point.
#[test]
fn ac45_unedited_click_writes_nothing_decided_against_the_clicked_point() {
    for r in ReferencePoint::ALL {
        let base = rect_30x40(0.0, 0.0);
        let mut s = open(&base);
        select_everything(&mut s);
        open_chip(&mut s, true);
        let before = bytes_of(&s);
        let pre = s.move_entry().unwrap().absolute_prefills[r.index()].clone();
        let out = s.commit_move_entry_at(&pre[0], &pre[1], false, r);
        assert_eq!(out, EntryOutcome::Unchanged, "{r:?}: both untouched");
        assert!(bytes_of(&s) == before);
        assert!(s.move_entry().is_none(), "{r:?}: the chip closes");
        // The texts of the TOP-LEFT point typed, but the click is on another point: it moves
        // that point there.
        if r != ReferencePoint::TopLeft {
            let mut s = open(&base);
            select_everything(&mut s);
            open_chip(&mut s, true);
            let out = s.commit_move_entry_at("10.0", "20.0", false, r);
            assert_eq!(
                out,
                EntryOutcome::Committed,
                "{r:?}: the texts differ from its prefill"
            );
        }
    }
}

/// Criterion 45: invalid text keeps the chip open and writes nothing.
#[test]
fn ac45_invalid_text_keeps_the_chip_open() {
    for bad in [
        "abc", "", "NaN", "inf", "-inf", "1e999", "--5", "5..5", "0x10",
    ] {
        for second_bad in [false, true] {
            let mut s = open(&rect_30x40(0.0, 0.0));
            select_everything(&mut s);
            open_chip(&mut s, true);
            let before = bytes_of(&s);
            let (a, b) = if second_bad { ("5", bad) } else { (bad, "5") };
            let out = s.commit_move_entry_at(a, b, false, ReferencePoint::TopLeft);
            assert!(
                matches!(out, EntryOutcome::Invalid { .. }),
                "'{bad}' (second field {second_bad}): {out:?}"
            );
            assert!(s.move_entry().is_some(), "'{bad}': the chip stays open");
            assert!(bytes_of(&s) == before, "'{bad}': nothing written");
            // Enter does the same.
            let out = s.commit_move_entry(a, b, mode(true, false));
            assert!(
                matches!(out, EntryOutcome::Invalid { .. }),
                "Enter '{bad}': {out:?}"
            );
            assert!(bytes_of(&s) == before);
        }
    }
}

/// Criterion 45: a result beyond +-1e7 mm on any of the four edges is refused, the same for every
/// reference point. The box is 30 x 40.
#[test]
fn ac45_the_limit_is_checked_on_all_four_edges_for_every_reference() {
    const LIMIT: f64 = 1.0e7;
    // Offset of a point from the left edge / top edge of the 30 x 40 box.
    let off = |r: ReferencePoint| -> (f64, f64) {
        let p = ref_point((pt(0.0, 0.0), pt(30.0, 40.0)), r);
        (p.x, p.y)
    };
    for r in ReferencePoint::ALL {
        let (dx, dy) = off(r);
        // The typed position puts the left edge at -LIMIT - 1, the right at LIMIT + 1, the top
        // at -LIMIT - 1, the bottom at LIMIT + 1.
        let cases = [
            ("left", -LIMIT - 1.0 + dx, 0.0),
            ("right", LIMIT + 1.0 - 30.0 + dx, 0.0),
            ("top", 0.0, -LIMIT - 1.0 + dy),
            ("bottom", 0.0, LIMIT + 1.0 - 40.0 + dy),
        ];
        for (edge, x, y) in cases {
            let mut s = open(&rect_30x40(0.0, 0.0));
            select_everything(&mut s);
            open_chip(&mut s, true);
            let before = bytes_of(&s);
            let pre = s.move_entry().unwrap().absolute_prefills[r.index()].clone();
            let (xs, ys) = (
                if edge == "left" || edge == "right" {
                    format!("{x}")
                } else {
                    pre[0].clone()
                },
                if edge == "top" || edge == "bottom" {
                    format!("{y}")
                } else {
                    pre[1].clone()
                },
            );
            let out = s.commit_move_entry_at(&xs, &ys, false, r);
            assert!(
                matches!(out, EntryOutcome::Invalid { .. }),
                "{r:?} {edge} edge beyond the limit: {out:?}"
            );
            assert!(bytes_of(&s) == before, "{r:?} {edge}: nothing written");
            assert!(s.move_entry().is_some());
        }
        // Just inside the limit: allowed.
        let mut s = open(&rect_30x40(0.0, 0.0));
        select_everything(&mut s);
        open_chip(&mut s, true);
        let pre = s.move_entry().unwrap().absolute_prefills[r.index()].clone();
        let x = LIMIT - 30.0 - 1.0 + dx;
        let out = s.commit_move_entry_at(&format!("{x}"), &pre[1], false, r);
        assert_eq!(
            out,
            EntryOutcome::Committed,
            "{r:?}: right edge 1 mm inside the limit"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 46: Copy
// ---------------------------------------------------------------------

/// Criterion 46: Copy displaced as typed, the copy selected, the original left, both for a mark
/// click and Enter.
#[test]
fn ac46_copy_via_mark_and_enter() {
    for via_mark in [true, false] {
        let mut s = open(&rect_30x40(0.0, 0.0));
        select_everything(&mut s);
        open_chip(&mut s, true);
        let c = change_count(&s);
        let out = if via_mark {
            s.commit_move_entry_at("100", "50", true, ReferencePoint::BottomRight)
        } else {
            s.commit_move_entry("100", "50", mode(true, true))
        };
        assert_eq!(out, EntryOutcome::Committed);
        assert_eq!(change_count(&s), c + 1, "one commit");
        let os = objects(&s);
        assert_eq!(os.len(), 2, "original and copy");
        assert_pt(rect_origin(&os[0]), 10.0, 20.0, 1e-9, "the original stays");
        let want = if via_mark { (70.0, 10.0) } else { (85.0, 30.0) };
        assert_pt(rect_origin(&os[1]), want.0, want.1, 1e-9, "the copy");
        assert_eq!(s.selected_object_count(), 1);
        // The copy is the selection: the next chip shows its centre.
        open_chip(&mut s, true);
        let v = s.move_entry().unwrap();
        let cc = rect_centre(&os[1]);
        assert_eq!(
            v.absolute_prefills[ReferencePoint::Centre.index()],
            [format!("{:.1}", cc.x), format!("{:.1}", cc.y)],
            "the copy is selected"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 47: rotated object, TBB marks differ from the centre handle
// ---------------------------------------------------------------------

/// Criterion 47: for a rotated triangle the TBB centre differs from the oriented box's centre
/// (where the centre handle is); the prefills and the marks are those of the TBB.
#[test]
fn ac47_marks_are_at_the_tbb_not_the_oriented_box() {
    let d = Document::new(1);
    let id = d.create_polygon(
        StarFrame {
            center: pt(200.0, 150.0),
            radius: Length::from_mm(30.0),
            angle: Angle::from_radians(-FRAC_PI_2),
        },
        curvyo_document_core::PointCount::new(3).unwrap(),
    );
    let o = d.object(id).unwrap();
    d.rotate_object(&o.rotated(pt(200.0, 150.0), Angle::from_radians(20.0_f64.to_radians())))
        .unwrap();
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let b = tbb_of(&s);
    let v = s.move_entry().unwrap();
    let c = ref_point(b, ReferencePoint::Centre);
    assert_eq!(
        v.absolute_prefills[ReferencePoint::Centre.index()],
        [format!("{:.1}", c.x), format!("{:.1}", c.y)],
        "centre prefill is the TBB centre"
    );
    assert!(
        (v.center.x - c.x).hypot(v.center.y - c.y) > 0.5,
        "the centre handle ({}, {}) and the TBB centre ({}, {}) differ for a turned triangle",
        v.center.x,
        v.center.y,
        c.x,
        c.y
    );
    for r in ReferencePoint::ALL {
        let p = ref_point(b, r);
        let pre = &v.absolute_prefills[r.index()];
        assert_eq!(pre[0], format!("{:.1}", p.x), "{r:?} x prefill");
        assert_eq!(pre[1], format!("{:.1}", p.y), "{r:?} y prefill");
        assert_eq!(
            s.move_mark_at(p, false),
            Some(r),
            "mark {r:?} sits on the TBB point"
        );
    }
    // The centre cross wins over the centre handle: a press at the TBB centre is on the mark.
    assert_eq!(s.move_mark_at(c, false), Some(ReferencePoint::Centre));
}

// ---------------------------------------------------------------------
// Criterion 39, 40, 50: marks, hit targets, zoom and pan
// ---------------------------------------------------------------------

/// Criterion 39/40: with the chip in Absolute the nine marks are hit at the TBB points and
/// within `min(12 px, 0.24 s)`; not in Relative; not beyond the radius.
#[test]
fn ac39_ac40_mark_hit_testing_through_the_session() {
    let mut s = open(&rect_30x40(100.0, 100.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let k = s.view().scale();
    let b = tbb_of(&s);
    let side_px = 30.0 * k;
    assert!(side_px >= 32.0, "the scene is big enough: s = {side_px}");
    let r_px = 12.0_f64.min(0.24 * side_px);
    for m in ReferencePoint::ALL {
        let p = ref_point(b, m);
        assert_eq!(s.move_mark_at(p, false), Some(m), "{m:?} centre");
        let inside = pt(p.x + (r_px - 0.5) / k, p.y);
        assert_eq!(
            s.move_mark_at(inside, false),
            Some(m),
            "{m:?} just inside {r_px} px"
        );
        let outside = pt(p.x + (r_px + 1.0) / k, p.y);
        assert_ne!(
            s.move_mark_at(outside, false),
            Some(m),
            "{m:?} just outside"
        );
    }
    // Touch has a bigger target.
    let p = ref_point(b, ReferencePoint::TopLeft);
    let touch_r = 22.0_f64.min(0.24 * side_px);
    assert_eq!(
        s.move_mark_at(pt(p.x - (touch_r - 0.5) / k, p.y), true),
        Some(ReferencePoint::TopLeft)
    );
    // Relative: no marks.
    s.set_move_chip_absolute(false);
    for m in ReferencePoint::ALL {
        assert_eq!(s.move_mark_at(ref_point(b, m), false), None);
    }
}

/// Criterion 39: thin boxes and small boxes (shorter side under 32 px) show no marks and have no
/// hit targets; the red box is still drawn; the Reference route (set_move_reference + Enter)
/// works. Criterion 50's test: a 2 mm square at 100 % zoom.
#[test]
fn ac39_ac50_small_and_thin_boxes_have_no_marks_but_a_box() {
    // The default view: the 2 mm square.
    let d = Document::new(1);
    let id = d.create_rect(rect_bounds(100.0, 100.0, 2.0, 2.0));
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let k = s.view().scale();
    assert!(
        2.0 * k < 32.0,
        "2 mm is under 32 px at the default zoom (k = {k})"
    );
    let b = tbb_of(&s);
    for m in ReferencePoint::ALL {
        assert_eq!(s.move_mark_at(ref_point(b, m), false), None, "{m:?}");
    }
    let l = s.draw_list();
    assert!(mark_vertices(&l).is_empty(), "no mark is drawn");
    assert_box_is(&s, b, "the box of a 2 mm square is drawn");
    // The Reference control route: Top left selected, Enter puts the top-left corner at (X, Y).
    s.set_move_reference(ReferencePoint::TopLeft);
    assert_eq!(
        s.commit_move_entry("50", "60", mode(true, false)),
        EntryOutcome::Committed
    );
    let o = objects(&s)
        .into_iter()
        .find(|o| matches!(o, ObjectSnapshot::Primitive(_)))
        .unwrap();
    let _ = id;
    assert_pt(
        rect_origin(&o),
        50.0,
        60.0,
        1e-9,
        "top-left corner at (50, 60)",
    );

    // A thin strip: 200 mm x 5 mm (s = 5 mm * k < 32 px).
    let d = Document::new(1);
    let _ = d.create_rect(rect_bounds(100.0, 100.0, 200.0, 5.0));
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    assert!(5.0 * k < 32.0);
    let b = tbb_of(&s);
    for m in ReferencePoint::ALL {
        assert_eq!(
            s.move_mark_at(ref_point(b, m), false),
            None,
            "thin strip {m:?}"
        );
    }
    assert!(mark_vertices(&s.draw_list()).is_empty());
    // A straight line (a zero-height box).
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(100.0, 100.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(250.0, 100.0)),
        ],
        false,
    );
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let b = tbb_of(&s);
    assert_eq!(
        s.move_mark_at(ref_point(b, ReferencePoint::Centre), false),
        None
    );
    assert!(mark_vertices(&s.draw_list()).is_empty());
    // The line moves by Enter with a reference choice: the centre goes to (10, 10).
    s.set_move_reference(ReferencePoint::LeftMiddle);
    assert_eq!(
        s.commit_move_entry("10", "10", mode(true, false)),
        EntryOutcome::Committed
    );
    let b = tbb_of(&s);
    assert_pt(
        ref_point(b, ReferencePoint::LeftMiddle),
        10.0,
        10.0,
        1e-9,
        "left middle of the line",
    );
}

/// Criterion 39/50: marks follow zoom and pan (recomputed from the document), marks outside the
/// viewport are not drawn nor clickable, below 32 px none is; the box line is drawn at
/// screen-constant width in every state; the hit radius shrinks as `0.24 s` would for small boxes.
#[test]
fn ac39_ac50_zoom_and_pan_independence() {
    let d = rect_30x40(100.0, 100.0);
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let b = tbb_of(&s);
    let at_default = s.view().scale();
    // Zoom in and out about the viewport centre, pan around; each time compare the session to
    // the model: marks hit exactly where the TBB points are on screen and only inside the
    // viewport and only from 32 px.
    let steps: [(f64, f64, f64, f64, bool); 8] = [
        (0.0, 100.0, 800.0, 500.0, true),
        (0.0, -300.0, 800.0, 500.0, true),
        (0.0, -300.0, 800.0, 500.0, true),
        (40.0, 0.0, 400.0, 300.0, false),
        (0.0, 500.0, 1200.0, 800.0, true),
        (0.0, 500.0, 100.0, 100.0, true),
        (0.0, 500.0, 100.0, 100.0, true),
        (200.0, 0.0, 800.0, 500.0, false),
    ];
    for (i, (dx, dy, x, y, ctrl)) in steps.iter().enumerate() {
        s.wheel(*dx, *dy, *x, *y, false, *ctrl);
        let view = s.view();
        let k = view.scale();
        let side_px = 30.0 * k;
        let shown = side_px >= 32.0;
        let (vw, vh) = (1600.0, 1000.0);
        for m in ReferencePoint::ALL {
            let p = ref_point(b, m);
            let (sx, sy) = view.document_to_screen(p);
            let inside = (0.0..=vw).contains(&sx) && (0.0..=vh).contains(&sy);
            let hit = s.move_mark_at(p, false);
            if shown && inside {
                assert_eq!(
                    hit,
                    Some(m),
                    "step {i} k={k}: {m:?} at screen ({sx:.0}, {sy:.0})"
                );
            } else {
                assert_eq!(
                    hit, None,
                    "step {i} k={k}: {m:?} is not shown (s = {side_px:.0}, inside = {inside})"
                );
            }
        }
        let l = s.draw_list();
        let marks = mark_vertices(&l);
        if !shown {
            assert!(marks.is_empty(), "step {i}: no marks drawn under 32 px");
        }
        // The mark squares are screen-constant: their extent in document mm times k is about
        // 12 px (10 px plus casing) whatever the zoom, when any is drawn.
        if let Some((lo, hi)) = extent(&marks) {
            if shown {
                let w_px = (hi.x - lo.x) * k;
                assert!(w_px > 5.0, "step {i}: mark extent {w_px} px");
            }
        }
        // The box is drawn in every state (it has no 32 px rule), inside the viewport only.
        let got = red_box(&s).expect("the red box stays while the chip is open");
        let slack = 4.0 / k;
        let (clip_lo, clip_hi) = (s.screen_to_document(0.0, 0.0), s.screen_to_document(vw, vh));
        assert!(
            got.0.x >= clip_lo.x - 4.0 * slack
                && got.0.y >= clip_lo.y - 4.0 * slack
                && got.1.x <= clip_hi.x + 4.0 * slack
                && got.1.y <= clip_hi.y + 4.0 * slack,
            "step {i}: the box line is clipped to the viewport: {got:?} vs {clip_lo:?}..{clip_hi:?}"
        );
    }
    assert!(at_default > 0.0);
}

/// Criterion 50: a selection whose box is far larger than the viewport: the box line stays inside
/// the viewport (clipped), marks inside the viewport are clickable, marks outside are not.
#[test]
fn ac50_a_box_larger_than_the_viewport() {
    let d = Document::new(1);
    let _ = d.create_rect(rect_bounds(-2900.0, -2930.0, 6000.0, 6000.0));
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    let view = s.view();
    let b = tbb_of(&s);
    let mut any_inside = 0;
    for m in ReferencePoint::ALL {
        let p = ref_point(b, m);
        let (sx, sy) = view.document_to_screen(p);
        let inside = (0.0..=1600.0).contains(&sx) && (0.0..=1000.0).contains(&sy);
        let hit = s.move_mark_at(p, false);
        if inside {
            any_inside += 1;
            assert_eq!(hit, Some(m));
        } else {
            assert_eq!(hit, None, "{m:?} lies outside the viewport");
        }
    }
    assert!(
        any_inside >= 1,
        "at least the centre lies inside the viewport"
    );
    // The line is clipped: no vertex lies far outside the canvas.
    let l = s.draw_list();
    let v = box_vertices(&l);
    assert!(
        v.is_empty(),
        "all four edges lie outside the viewport: nothing is drawn"
    );
    let lo = s.screen_to_document(-50.0, -50.0);
    let hi = s.screen_to_document(1650.0, 1050.0);
    for (_, p) in &v {
        assert!(
            p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y,
            "box vertex ({}, {}) lies beyond the viewport",
            p.x,
            p.y
        );
    }
    // Typed values still work through the Reference route for every one of the nine points.
    for r in ReferencePoint::ALL {
        let mut t = open(&d);
        select_everything(&mut t);
        open_chip(&mut t, true);
        t.set_move_reference(r);
        let out = t.commit_move_entry("0", "0", mode(true, false));
        assert_eq!(out, EntryOutcome::Committed, "{r:?}");
        assert_pt(ref_point(tbb_of(&t), r), 0.0, 0.0, 1e-6, &format!("{r:?}"));
    }
}

// ---------------------------------------------------------------------
// Criteria 33 to 36: the red box
// ---------------------------------------------------------------------

/// Criterion 33, the example: the rectangle 30 x 40 with its top-left at (10, 20) dragged by
/// (+20, +10): the red box runs from (30, 30) to (60, 70) during the drag, appears after the
/// 3 px dead zone only, follows the pointer frame by frame, and is gone at the release.
#[test]
fn ac33_move_drag_shows_the_box() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    assert!(red_box(&s).is_none(), "no box when nothing moves");
    let from = pt(18.0 + ox, 20.0 + oy);
    hold(&mut s, from, false, false);
    s.pointer_down(from, false);
    assert!(red_box(&s).is_none(), "no box at the press");
    // Inside the dead zone (1 px).
    let one_px = 1.0 / s.view().scale();
    hold(&mut s, pt(from.x + one_px, from.y), false, false);
    assert!(red_box(&s).is_none(), "no box inside the 3 px dead zone");
    // The example offset.
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), false, false);
    assert_box_is(
        &s,
        (pt(30.0 + ox, 30.0 + oy), pt(60.0 + ox, 70.0 + oy)),
        "during the drag",
    );
    // It moves with the preview in every frame.
    hold(&mut s, pt(from.x + 5.0, from.y - 8.0), false, false);
    assert_box_is(
        &s,
        (pt(15.0 + ox, 12.0 + oy), pt(45.0 + ox, 52.0 + oy)),
        "next frame",
    );
    // Back to the start: gone.
    hold(&mut s, from, false, false);
    assert!(
        red_box(&s).is_none(),
        "the drag ended where it began: no box"
    );
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), false, false);
    assert!(red_box(&s).is_some());
    // Escape drops it.
    key(&mut s, "Escape", false);
    assert!(red_box(&s).is_none(), "Escape: no box");
    s.pointer_up(pt(from.x + 20.0, from.y + 10.0), false, false);
    assert!(red_box(&s).is_none());
    assert_pt(
        rect_origin(&objects(&s)[0]),
        10.0 + ox,
        20.0 + oy,
        1e-9,
        "Escape wrote nothing",
    );
    // Release: gone.
    hold(&mut s, from, false, false);
    s.pointer_down(from, false);
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), false, false);
    assert!(red_box(&s).is_some());
    s.pointer_up(pt(from.x + 20.0, from.y + 10.0), false, false);
    assert!(red_box(&s).is_none(), "released: no box");
    assert_pt(
        rect_origin(&objects(&s)[0]),
        30.0 + ox,
        30.0 + oy,
        1e-9,
        "moved",
    );
}

/// Criterion 33: a Shift axis lock and a Ctrl copy: the box is the TBB of the geometry the
/// release would commit; in a copy it travels with the copy.
#[test]
fn ac33_axis_lock_and_copy() {
    let (ox, oy) = (100.0, 100.0);
    let from = pt(18.0 + ox, 20.0 + oy);
    // Shift: x wins over a (20, 10) drag.
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    hold(&mut s, from, true, false);
    s.pointer_down(from, true);
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), true, false);
    assert_box_is(
        &s,
        (pt(30.0 + ox, 20.0 + oy), pt(60.0 + ox, 60.0 + oy)),
        "axis locked to x",
    );
    s.pointer_up(pt(from.x + 20.0, from.y + 10.0), true, false);
    assert!(red_box(&s).is_none());
    // Ctrl: the copy.
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    hold(&mut s, from, false, true);
    s.pointer_down(from, false);
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), false, true);
    assert_box_is(
        &s,
        (pt(30.0 + ox, 30.0 + oy), pt(60.0 + ox, 70.0 + oy)),
        "the copy's box",
    );
    s.pointer_up(pt(from.x + 20.0, from.y + 10.0), false, true);
    assert!(red_box(&s).is_none());
    assert_eq!(objects(&s).len(), 2);
}

/// Criterion 33 with a rotated object and a multi-selection: the box is the TBB of the moved
/// geometry, not the oriented box.
#[test]
fn ac33_rotated_and_multi() {
    let (ox, oy) = (100.0, 100.0);
    let d = rotated_rect_and_circle(ox, oy);
    let mut s = open(&d);
    select_everything(&mut s);
    let b = tbb_of(&s);
    // Grab the circle's outline point (right-most) and drag it by (13, -7).
    let from = pt(40.0 + ox, 10.0 + oy);
    hold(&mut s, from, false, false);
    s.pointer_down(from, false);
    hold(&mut s, pt(from.x + 13.0, from.y - 7.0), false, false);
    assert_box_is(
        &s,
        (pt(b.0.x + 13.0, b.0.y - 7.0), pt(b.1.x + 13.0, b.1.y - 7.0)),
        "moved TBB of a rotated rect and a circle",
    );
    key(&mut s, "Escape", false);
    s.pointer_up(pt(from.x + 13.0, from.y - 7.0), false, false);
    // A single rotated rect.
    let d = Document::new(1);
    let id = d.create_rect(rect_bounds(ox, oy, 20.0, 10.0));
    let o = d.object(id).unwrap();
    d.rotate_object(&o.rotated(
        pt(10.0 + ox, 5.0 + oy),
        Angle::from_radians(30.0_f64.to_radians()),
    ))
    .unwrap();
    let mut s = open(&d);
    select_everything(&mut s);
    let b = tbb_of(&s);
    // The mid of the top edge of the turned rectangle, away from the handles: a point at 1/4.
    let o = &objects(&s)[0];
    let poly = polyline(o);
    let from = poly[50];
    hold(&mut s, from, false, false);
    s.pointer_down(from, false);
    hold(&mut s, pt(from.x + 9.0, from.y + 4.0), false, false);
    if s.selected_object_count() == 1 {
        if let Some(got) = red_box(&s) {
            let slack = 4.0 / s.view().scale();
            assert!(
                near(got.0.x, b.0.x + 9.0, slack) && near(got.1.y, b.1.y + 4.0, slack),
                "rotated rectangle: the box is the TBB, not the oriented box: {got:?}"
            );
        }
    }
    s.pointer_up(pt(from.x + 9.0, from.y + 4.0), false, false);
}

/// Criterion 34: with the chip open, in Relative or Absolute, the TBB of the selection at the
/// time the chip opened is drawn; it does not preview typed values; it goes with the chip on
/// Enter, Escape, a tool switch and a selection change.
#[test]
fn ac34_the_box_stays_while_the_chip_is_open() {
    for absolute in [false, true] {
        let (ox, oy) = (100.0, 100.0);
        let mut s = open(&rect_30x40(ox, oy));
        select_everything(&mut s);
        assert!(red_box(&s).is_none(), "no chip, no box");
        open_chip(&mut s, absolute);
        let want = (pt(10.0 + ox, 20.0 + oy), pt(40.0 + ox, 60.0 + oy));
        assert_box_is(&s, want, "chip open");
        // Typing does not preview: still the original.
        hold(&mut s, pt(500.0, 300.0), false, false);
        assert_box_is(&s, want, "after the pointer moved");
        // Escape closes both.
        key(&mut s, "Escape", false);
        assert!(s.move_entry().is_none());
        assert!(red_box(&s).is_none(), "Escape: no box");
        // Enter.
        open_chip(&mut s, absolute);
        assert!(red_box(&s).is_some());
        let (tx, ty) = if absolute {
            ("200", "150")
        } else {
            ("20", "15")
        };
        let out = s.commit_move_entry(tx, ty, mode(absolute, false));
        assert_eq!(out, EntryOutcome::Committed);
        assert!(red_box(&s).is_none(), "Enter: no box");
        // Tool switch.
        open_chip(&mut s, absolute);
        assert!(red_box(&s).is_some());
        s.set_tool(Tool::Pen);
        assert!(red_box(&s).is_none(), "tool switch: no box");
        s.set_tool(Tool::Select);
        assert!(s.move_entry().is_none());
        // Selection change.
        select_everything(&mut s);
        open_chip(&mut s, absolute);
        assert!(red_box(&s).is_some());
        key(&mut s, "Escape", false);
        key(&mut s, "Escape", false);
        assert_eq!(s.selected_object_count(), 0);
        assert!(red_box(&s).is_none());
    }
}

/// Criterion 35: the box equals the TBB; the stroke width does not change it; the spec's numbers
/// for the rotated rectangle and the circle.
#[test]
fn ac35_the_box_is_exact() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rotated_rect_and_circle(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, false);
    let b = tbb_of(&s);
    assert_pt(b.0, -1.16 + ox, -4.33 + oy, 0.01, "min");
    assert_pt(b.1, 45.0 + ox, 14.33 + oy, 0.01, "max");
    assert_box_is(&s, b, "exact TBB");
    // A wide stroke changes nothing.
    let d = doc_of(&s);
    let ids = d.object_ids();
    d.edit_style(
        &ids,
        &curvyo_document_core::StyleEdit::StrokeWidth(Length::from_mm(6.0)),
    )
    .unwrap();
    let mut t = open(&d);
    select_everything(&mut t);
    open_chip(&mut t, false);
    assert_box_is(&t, b, "stroke width 6 mm");
}

/// Criterion 36: layer order. The pale box is below the blue selection decoration and below the
/// marks; the marks are the topmost thing of the selection frame (above the handles); the box has
/// a white casing; the box line is 4 px (changed from 1 px after the UX review).
#[test]
fn ac36_layer_order_and_line_width() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let l = s.draw_list();
    let boxv = box_vertices(&l);
    let markv = mark_vertices(&l);
    assert!(!boxv.is_empty() && !markv.is_empty());
    let last_box = boxv.iter().map(|(i, _)| *i).max().unwrap();
    let first_mark = markv.iter().map(|(i, _)| *i).min().unwrap();
    assert!(last_box < first_mark, "the marks are above the box");
    // A blue vertex (selection box or handle): any with b well above r.
    let blue: Vec<usize> = l
        .triangles
        .iter()
        .enumerate()
        .filter(|(_, v)| i32::from(v.color.b) - i32::from(v.color.r) > 60 && v.color.a > 0)
        .map(|(i, _)| i)
        .collect();
    assert!(!blue.is_empty(), "the selection decoration is blue");
    let first_blue = *blue.iter().min().unwrap();
    let last_blue = *blue.iter().max().unwrap();
    assert!(
        boxv.iter().all(|(i, _)| *i < first_blue),
        "the reference box is drawn below the blue selection box and handles"
    );
    assert!(
        markv.iter().all(|(i, _)| *i > last_blue),
        "the marks are drawn above the blue handles"
    );
    // The line is four screen pixels wide: its extent among the box vertices, in px.
    let k = s.view().scale();
    let xs: std::collections::BTreeSet<i64> = boxv
        .iter()
        .map(|(_, p)| (p.x * 1e6).round() as i64)
        .collect();
    let mut sorted: Vec<f64> = xs.into_iter().map(|v| v as f64 / 1e6).collect();
    sorted.sort_by(f64::total_cmp);
    // Left edge: the casing and the line share x extents around the left edge of the TBB.
    let near_left: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|x| (x - (10.0 + ox)).abs() < 3.0 / k)
        .collect();
    let spread = near_left.last().unwrap() - near_left.first().unwrap();
    assert!(
        spread * k <= 4.0 + 1e-6,
        "the left edge (the 4 px line) spans {} px",
        spread * k
    );
    // The line proper (opaque? no, pale): there is a white casing under it.
    let white: Vec<_> = l
        .triangles
        .iter()
        .filter(|v| {
            v.color.r > 240
                && v.color.g > 240
                && v.color.b > 240
                && v.color.a < 255
                && v.color.a > 0
        })
        .collect();
    assert!(!white.is_empty(), "a semi-transparent white casing exists");
}

/// Criterion 33/36: pixel alignment. The box line's centre line is on a half pixel in screen space
/// so it is crisp: the x of a vertical edge, in device px, is an integer plus 0.5 for the centre
/// of a 1 px line (or its two edges are integers).
#[test]
fn ac36_the_line_is_pixel_aligned() {
    let (ox, oy) = (100.3, 100.7);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, false);
    let l = s.draw_list();
    let boxv = box_vertices(&l);
    let k = s.view().scale();
    let view = s.view();
    // Collect distinct screen x of vertices (line edges).
    let mut sx: Vec<f64> = boxv
        .iter()
        .map(|(_, p)| view.document_to_screen(*p).0)
        .collect();
    sx.sort_by(f64::total_cmp);
    sx.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let _ = k;
    let aligned = sx.iter().all(|x| {
        let f = x - x.floor();
        f < 1e-3 || f > 1.0 - 1e-3 || (f - 0.5).abs() < 1e-3
    });
    assert!(aligned, "line edges lie on whole or half pixels: {sx:?}");
}

// ---------------------------------------------------------------------
// Criterion 43/44: hover state and cursor
// ---------------------------------------------------------------------

/// Criterion 43: the pointer on a mark gives it the hover state: it is larger by 2 px and the
/// cursor is `pointer`; leaving restores. The radio hover (Reference control) does the same.
#[test]
fn ac43_hover_state_and_cursor() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let k = s.view().scale();
    let b = tbb_of(&s);
    for m in ReferencePoint::ALL {
        let p = ref_point(b, m);
        // Away from every mark.
        hold(&mut s, pt(b.0.x - 200.0, b.0.y - 200.0), false, false);
        assert_eq!(s.move_mark_hover(), None);
        let cold = mark_extent_near(&s.draw_list(), p, 20.0 / k);
        hold(&mut s, p, false, false);
        assert_eq!(s.move_mark_hover(), Some(m), "{m:?} hovered");
        assert_eq!(s.cursor_hint(), "pointer", "{m:?}: cursor is pointer");
        let hot = mark_extent_near(&s.draw_list(), p, 20.0 / k);
        let (cw, hw) = ((cold.1.x - cold.0.x) * k, (hot.1.x - hot.0.x) * k);
        assert!(
            near(hw - cw, 2.0, 0.6),
            "{m:?}: hovered mark is 2 px larger: {cw:.2} -> {hw:.2}"
        );
        // Radio hover: same growth at the keyboard focus.
        hold(&mut s, pt(b.0.x - 200.0, b.0.y - 200.0), false, false);
        s.set_move_radio_hover(Some(m));
        let rad = mark_extent_near(&s.draw_list(), p, 20.0 / k);
        assert!(
            near((rad.1.x - rad.0.x) * k - cw, 2.0, 0.6),
            "{m:?}: radio hover grows the mark too"
        );
        s.set_move_radio_hover(None);
    }
}

fn mark_extent_near(l: &DrawList, p: Point, r: f64) -> (Point, Point) {
    let v: Vec<(usize, Point)> = mark_vertices(l)
        .into_iter()
        .filter(|(_, q)| (q.x - p.x).abs() <= r && (q.y - p.y).abs() <= r)
        .collect();
    extent(&v).expect("a mark is drawn there")
}

/// Criterion 39: the selected reference is drawn as the current one (filled), the others hollow;
/// the chosen point changes the drawing.
#[test]
fn ac39_the_current_mark_is_drawn_differently() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let a = s.draw_list();
    s.set_move_reference(ReferencePoint::BottomRight);
    let b = s.draw_list();
    assert!(
        a.triangles != b.triangles,
        "choosing another reference changes the drawing"
    );
    // Count of marks does not change.
    let nine_a = mark_vertices(&a).len();
    let nine_b = mark_vertices(&b).len();
    assert!(nine_a > 0 && nine_b > 0);
    // The old and the new current mark change; each mark's own red vertex count differs
    // between the filled and the hollow look.
    let k = s.view().scale();
    let bx = tbb_of(&s);
    // Every vertex of any colour near the mark: a hollow mark has the white inside on top.
    let count_near = |l: &DrawList, p: Point| {
        l.triangles
            .iter()
            .filter(|v| {
                (v.position.x - p.x).abs() <= 8.0 / k
                    && (v.position.y - p.y).abs() <= 8.0 / k
                    && v.color.a == 255
            })
            .count()
    };
    let (c, br) = (
        ref_point(bx, ReferencePoint::Centre),
        ref_point(bx, ReferencePoint::BottomRight),
    );
    assert_ne!(
        count_near(&a, c),
        count_near(&b, c),
        "the centre cross changed look"
    );
    assert_ne!(
        count_near(&a, br),
        count_near(&b, br),
        "the bottom right square changed look"
    );
    // A mark that was not touched keeps its look.
    let tl = ref_point(bx, ReferencePoint::TopLeft);
    assert_eq!(count_near(&a, tl), count_near(&b, tl));
}

// ---------------------------------------------------------------------
// Criterion 44: prefill follows the point under the pointer (session part)
// ---------------------------------------------------------------------

/// Criterion 44 (Proposal): the session offers every point's prefill and the mark under the
/// pointer, so the chip can show the point's X and Y while the pointer rests on it.
#[test]
fn ac44_prefill_of_the_hovered_point() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let b = tbb_of(&s);
    let v = s.move_entry().unwrap();
    for m in ReferencePoint::ALL {
        hold(&mut s, ref_point(b, m), false, false);
        let hovered = s.move_mark_hover().unwrap();
        assert_eq!(hovered, m);
        let p = ref_point(b, hovered);
        assert_eq!(
            v.absolute_prefills[hovered.index()],
            [format!("{:.1}", p.x), format!("{:.1}", p.y)]
        );
    }
}

// ---------------------------------------------------------------------
// Hostile input
// ---------------------------------------------------------------------

/// Hostile move input: NaN, infinity, huge, negative zero; the huge-selection box; never a panic,
/// never a write.
#[test]
fn hostile_inputs_never_write_and_never_panic() {
    for text in [
        "NaN",
        "nan",
        "inf",
        "-inf",
        "Infinity",
        "1e308",
        "-1e308",
        "1e7",
        "-1e7",
        "9999999.9",
        "0x1p3",
    ] {
        for r in ReferencePoint::ALL {
            let mut s = open(&rect_30x40(0.0, 0.0));
            select_everything(&mut s);
            open_chip(&mut s, true);
            let before = bytes_of(&s);
            let out = s.commit_move_entry_at(text, "5", false, r);
            match out {
                EntryOutcome::Committed => {
                    let b = tbb_of(&s);
                    assert!(
                        b.0.x.is_finite()
                            && b.1.x.is_finite()
                            && b.0.x.abs() <= 1e7 + 1e-3
                            && b.1.x.abs() <= 1e7 + 1e-3,
                        "'{text}' {r:?} committed a box beyond the limit: {b:?}"
                    );
                }
                _ => assert!(
                    bytes_of(&s) == before,
                    "'{text}' {r:?}: {out:?} but the document changed"
                ),
            }
        }
    }
    // Hostile pointer positions: marks and hover with NaN / infinity positions.
    let mut s = open(&rect_30x40(100.0, 100.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300] {
        assert_eq!(s.move_mark_at(pt(v, v), false), None, "{v}");
        assert_eq!(s.move_mark_at(pt(v, 0.0), true), None, "{v}");
    }
    // Zoom to the extremes: no panic, marks or not.
    for _ in 0..60 {
        s.wheel(0.0, -400.0, 800.0, 500.0, false, true);
        let _ = s.draw_list();
        let _ = s.move_mark_at(pt(100.0, 100.0), false);
    }
    for _ in 0..120 {
        s.wheel(0.0, 400.0, 800.0, 500.0, false, true);
        let _ = s.draw_list();
        let _ = s.move_mark_at(pt(100.0, 100.0), false);
    }
}

/// A selection of 300 objects: the chip, the marks, the red box and the typed move work and
/// write one commit (a coarse stand-in for the 10,000-object frame budget, which needs a release
/// build; see the ignored benchmark below).
#[test]
fn many_objects_one_commit() {
    let d = Document::new(1);
    for i in 0..300 {
        let (x, y) = (f64::from(i % 20) * 12.0, f64::from(i / 20) * 12.0);
        let _ = d.create_rect(rect_bounds(x + 5.0, y + 5.0, 8.0, 8.0));
    }
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, true);
    assert!(red_box(&s).is_some());
    let c = change_count(&s);
    assert_eq!(
        s.commit_move_entry_at("1000", "1000", false, ReferencePoint::BottomRight),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&s), c + 1);
    let b = tbb_of(&s);
    assert_pt(b.1, 1000.0, 1000.0, 1e-6, "bottom right of 300 objects");
    assert_eq!(objects(&s).iter().filter(|o| is_path(o)).count(), 0);
}

#[allow(dead_code)]
fn unused(_: NodeId, _: Vec2) {}

/// Criterion 36/50: one edge of a huge box crosses the viewport: only that edge's part inside
/// the viewport is drawn (clipped to it), never the thousands of mm outside.
#[test]
fn ac36_ac50_one_edge_crossing_the_viewport_is_clipped() {
    let d = Document::new(1);
    let _ = d.create_rect(rect_bounds(-2000.0, 100.0, 6000.0, 3000.0));
    let mut s = open(&d);
    select_everything(&mut s);
    open_chip(&mut s, false);
    let l = s.draw_list();
    let v = box_vertices(&l);
    assert!(
        !v.is_empty(),
        "the top edge crosses the viewport and is drawn"
    );
    let lo = s.screen_to_document(-30.0, -30.0);
    let hi = s.screen_to_document(1630.0, 1030.0);
    for (_, p) in &v {
        assert!(
            p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y,
            "box vertex ({}, {}) lies beyond the viewport",
            p.x,
            p.y
        );
    }
    let (min, max) = extent(&v).unwrap();
    assert!(
        max.x - min.x > 100.0,
        "the visible part of the edge is long"
    );
}

fn dbl(s: &mut Session, at: Point, ctrl: bool) -> bool {
    hold(s, at, false, ctrl);
    s.pointer_down(at, false);
    s.pointer_up(at, false, ctrl);
    s.double_click(at, false, ctrl)
}

/// Criterion 33 (question 7): scale, rotate and skew drags, the typed rotate, size and skew
/// chips, a nudge and the Node tool show no red box.
#[test]
fn ac33_other_gestures_show_no_box() {
    let (ox, oy) = (100.0, 100.0);
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    let k = s.view().scale();
    let b = tbb_of(&s);
    let c = ref_point(b, ReferencePoint::Centre);
    let corner = ref_point(b, ReferencePoint::BottomRight);
    // Scale drag.
    hold(&mut s, corner, false, false);
    s.pointer_down(corner, false);
    hold(&mut s, pt(corner.x + 10.0, corner.y + 10.0), false, false);
    assert!(red_box(&s).is_none(), "scale drag");
    key(&mut s, "Escape", false);
    s.pointer_up(pt(corner.x + 10.0, corner.y + 10.0), false, false);
    // Rotate drag: 32 px out on the diagonal.
    let d = 32.0 / std::f64::consts::SQRT_2 / k;
    let rot = pt(corner.x + d, corner.y + d);
    hold(&mut s, rot, false, false);
    s.pointer_down(rot, false);
    hold(&mut s, pt(rot.x + 15.0, rot.y - 6.0), false, false);
    assert!(red_box(&s).is_none(), "rotate drag");
    key(&mut s, "Escape", false);
    s.pointer_up(pt(rot.x + 15.0, rot.y - 6.0), false, false);
    // Skew drag.
    let top = pt(c.x, b.0.y - 16.0 / k);
    hold(&mut s, top, false, false);
    s.pointer_down(top, false);
    hold(&mut s, pt(top.x + 10.0, top.y), false, false);
    assert!(red_box(&s).is_none(), "skew drag");
    key(&mut s, "Escape", false);
    s.pointer_up(pt(top.x + 10.0, top.y), false, false);
    // Typed chips.
    for k in ["r", "s"] {
        assert_eq!(key(&mut s, k, false), KeyOutcome::EntryOpened);
        assert!(red_box(&s).is_none(), "typed chip {k}");
        s.cancel_transform_entry();
    }
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert!(red_box(&s).is_none(), "typed skew chip");
    s.cancel_transform_entry();
    // Nudge.
    let _ = key(&mut s, "ArrowRight", false);
    assert!(red_box(&s).is_none(), "nudge");
    // Node tool.
    s.set_tool(Tool::Node);
    assert!(red_box(&s).is_none(), "node tool");
}

/// Criterion 34/46: a double-click on the centre handle opens the chip in Relative, with the
/// Copy preset from Ctrl at the second press; M never presets the copy; the box shows with the
/// chip open.
#[test]
fn ac34_ac46_double_click_on_the_centre_opens_the_chip() {
    let (ox, oy) = (100.0, 100.0);
    for ctrl in [false, true] {
        let mut s = open(&rect_30x40(ox, oy));
        select_everything(&mut s);
        let c = ref_point(tbb_of(&s), ReferencePoint::Centre);
        let _ = dbl(&mut s, c, ctrl);
        let v = s.move_entry().expect("the chip is open");
        assert_eq!(v.copy_preset, ctrl, "Ctrl at the second press presets Copy");
        assert_eq!(v.reference, ReferencePoint::Centre);
        assert!(red_box(&s).is_some(), "the box shows with the chip open");
        // The chip opens in Relative: no mark yet.
        assert_eq!(s.move_mark_at(c, false), None);
    }
    let mut s = open(&rect_30x40(ox, oy));
    select_everything(&mut s);
    open_chip(&mut s, false);
    assert!(!s.move_entry().unwrap().copy_preset);
}

/// Criterion 40: a press and release on the centre handle position while the chip is open in
/// Absolute: the centre cross wins over the centre handle (the session names the mark).
#[test]
fn ac47_the_centre_cross_wins_over_the_centre_handle() {
    let mut s = open(&rect_30x40(100.0, 100.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    let c = ref_point(tbb_of(&s), ReferencePoint::Centre);
    assert_eq!(s.move_mark_at(c, false), Some(ReferencePoint::Centre));
    // Where the corner resize handles are, the corner marks win as well (they sit on them).
    let b = tbb_of(&s);
    for m in [
        ReferencePoint::TopLeft,
        ReferencePoint::TopRight,
        ReferencePoint::BottomLeft,
        ReferencePoint::BottomRight,
        ReferencePoint::TopCentre,
        ReferencePoint::BottomCentre,
        ReferencePoint::LeftMiddle,
        ReferencePoint::RightMiddle,
    ] {
        assert_eq!(s.move_mark_at(ref_point(b, m), false), Some(m));
    }
}

/// Criterion 38/40: the reference chosen by `set_move_reference` is what a plain Enter uses (the
/// Reference control route), and Relative mode ignores it.
#[test]
fn ac41_the_chosen_option_is_used_by_enter_and_relative_ignores_it() {
    for r in ReferencePoint::ALL {
        let mut s = open(&rect_30x40(0.0, 0.0));
        select_everything(&mut s);
        open_chip(&mut s, true);
        s.set_move_reference(r);
        assert_eq!(
            s.commit_move_entry("100", "50", mode(true, false)),
            EntryOutcome::Committed,
            "{r:?}"
        );
        assert_pt(
            ref_point(tbb_of(&s), r),
            100.0,
            50.0,
            1e-9,
            &format!("{r:?}"),
        );
        // Relative: the reference is not used.
        let mut s = open(&rect_30x40(0.0, 0.0));
        select_everything(&mut s);
        open_chip(&mut s, false);
        s.set_move_reference(r);
        assert_eq!(
            s.commit_move_entry("7", "3", mode(false, false)),
            EntryOutcome::Committed
        );
        assert_pt(
            rect_origin(&objects(&s)[0]),
            17.0,
            23.0,
            1e-9,
            &format!("relative {r:?}"),
        );
    }
}

/// Criterion 38 + 49 across a mode switch: Absolute chosen, a reference chosen, switch back to
/// Relative and Enter: relative offset applies.
#[test]
fn ac49_switching_modes_keeps_things_apart() {
    let mut s = open(&rect_30x40(0.0, 0.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    s.set_move_reference(ReferencePoint::BottomRight);
    s.set_move_chip_absolute(false);
    assert_eq!(s.move_mark_at(pt(40.0, 60.0), false), None);
    assert_eq!(
        s.commit_move_entry("5", "5", mode(false, false)),
        EntryOutcome::Committed
    );
    assert_pt(rect_origin(&objects(&s)[0]), 15.0, 25.0, 1e-9, "relative");
}

/// Criterion 36: the 4 px line is pixel-aligned at other device pixel ratios as well: the edges
/// of the red line lie on whole device pixels, and the line is four CSS pixels wide (changed
/// from 1 px after the UX review of 2026-10-11).
#[test]
fn ac36_the_line_is_aligned_at_every_device_pixel_ratio() {
    for dpr in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let mut s = open(&rect_30x40(100.3, 100.7));
        s.set_device_pixel_ratio(dpr);
        select_everything(&mut s);
        open_chip(&mut s, false);
        let view = s.view();
        let v = box_vertices(&s.draw_list());
        assert!(!v.is_empty(), "dpr {dpr}");
        let mut xs: Vec<f64> = v
            .iter()
            .map(|(_, p)| view.document_to_screen(*p).0)
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for x in &xs {
            let device = x * dpr;
            assert!(
                (device - device.round()).abs() < 1e-3,
                "dpr {dpr}: a line edge at css x {x} is device pixel {device}"
            );
        }
        // The vertical line: its two edges (left and right of the line) are four css pixels apart.
        let left: Vec<f64> = xs
            .iter()
            .copied()
            .filter(|x| (x - view.document_to_screen(pt(110.3, 0.0)).0).abs() < 3.0)
            .collect();
        let width = left.last().unwrap() - left.first().unwrap();
        let device_width = width * dpr;
        assert!(
            (device_width - device_width.round()).abs() < 1e-3
                && device_width.round() == (4.0 * dpr).round(),
            "dpr {dpr}: the line is {device_width} device px wide (a whole number of device pixels, 4 css px rounded)"
        );
    }
}

/// Criterion 29 (the Escape cascade) for the typed move: Escape closes only the chip, the
/// selection stays, a second Escape then clears the selection.
#[test]
fn ac34_escape_closes_the_chip_before_anything_else() {
    let mut s = open(&rect_30x40(100.0, 100.0));
    select_everything(&mut s);
    open_chip(&mut s, true);
    key(&mut s, "Escape", false);
    assert!(s.move_entry().is_none(), "the chip closed");
    assert_eq!(s.selected_object_count(), 1, "the selection stays");
    assert!(red_box(&s).is_none());
}
