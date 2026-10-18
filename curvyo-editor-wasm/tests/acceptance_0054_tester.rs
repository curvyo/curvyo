//! Independent session-level acceptance tests for `0054-combine-open-paths`, written from the
//! specification before the implementation was read. The fill area is evaluated here with a
//! nonzero-winding test that closes every open outline by its closing line (criterion 3), so the
//! tests do not share an oracle with the kernel.
//!
//! Criteria covered: 1, 4, 6, 7a-g, 8-13, 14, 15-19, 20, 21-23, 28, 31, 32 (release, `--ignored`
//! for the timing assert), 33. Criteria 2 and 3 (pixels) are in `curvyo-render-core`.

#![allow(
    unused_must_use,
    clippy::manual_midpoint,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, AnchorKind, Color, Document, MarkerShape, NewAnchor, NodeId, PathSnapshot, Point,
    StyleEdit, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{
    BooleanOutcome, BreakApartOutcome, CombineOutcome, Session, SplitOutcome, Tool,
};
use curvyo_ui_core::{
    BooleanAvailability, BooleanOp, BooleanRefusal, CombineRefusal, CutRefusal, SplitRefusal,
};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn aid() -> AnchorId {
    AnchorId::new(54, NEXT.fetch_add(1, Ordering::SeqCst))
}
fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn corner(x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(aid(), pt(x, y))
}
fn poly(points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points.iter().map(|&(x, y)| corner(x, y)).collect()
}
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<NewAnchor> {
    poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}
fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<NewAnchor> {
    poly(&[(x0, y0), (x1, y1)])
}
fn circle(cx: f64, cy: f64, r: f64) -> Vec<NewAnchor> {
    let k = 0.552_284_749_830_793_4 * r;
    let mk = |x, y, hi: (f64, f64), ho: (f64, f64)| NewAnchor {
        id: aid(),
        point: pt(x, y),
        handle_in: Vec2::new(hi.0, hi.1),
        handle_out: Vec2::new(ho.0, ho.1),
        kind: AnchorKind::Symmetric,
    };
    vec![
        mk(cx + r, cy, (0.0, -k), (0.0, k)),
        mk(cx, cy + r, (k, 0.0), (-k, 0.0)),
        mk(cx - r, cy, (0.0, k), (0.0, -k)),
        mk(cx, cy - r, (-k, 0.0), (k, 0.0)),
    ]
}
fn u_shape() -> Vec<NewAnchor> {
    poly(&[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)])
}

fn session_of(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s
}
fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}
fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}
fn paths(s: &Session) -> Vec<PathSnapshot> {
    let d = reread(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.path(id))
        .collect()
}
fn marquee(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(
        pt((from.x + to.x) / 2.0, (from.y + to.y) / 2.0),
        false,
        false,
    );
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}
fn select_all(s: &mut Session) {
    marquee(s, pt(-3000.0, -3000.0), pt(50000.0, 50000.0));
}
fn session_all(d: &Document) -> Session {
    let mut s = session_of(d);
    select_all(&mut s);
    s
}

// --- the fill-area oracle -----------------------------------------------------------------------

fn flatten(a: &[NewAnchor], closed: bool) -> Vec<(f64, f64)> {
    let n = a.len();
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::new();
    for i in 0..segs {
        let (p, q) = (&a[i], &a[(i + 1) % n]);
        let p0 = (p.point.x, p.point.y);
        let p1 = (p.point.x + p.handle_out.x, p.point.y + p.handle_out.y);
        let p2 = (q.point.x + q.handle_in.x, q.point.y + q.handle_in.y);
        let p3 = (q.point.x, q.point.y);
        for k in 0..64 {
            let t = f64::from(k) / 64.0;
            let u = 1.0 - t;
            let (c0, c1, c2, c3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            out.push((
                c0 * p0.0 + c1 * p1.0 + c2 * p2.0 + c3 * p3.0,
                c0 * p0.1 + c1 * p1.1 + c2 * p2.1 + c3 * p3.1,
            ));
        }
    }
    if !closed {
        let l = a.last().unwrap();
        out.push((l.point.x, l.point.y));
    }
    out
}
fn winding(poly: &[(f64, f64)], x: f64, y: f64) -> i32 {
    let mut w = 0;
    for i in 0..poly.len() {
        let (x0, y0) = poly[i];
        let (x1, y1) = poly[(i + 1) % poly.len()];
        if y0 <= y {
            if y1 > y && (x1 - x0) * (y - y0) - (x - x0) * (y1 - y0) > 0.0 {
                w += 1;
            }
        } else if y1 <= y && (x1 - x0) * (y - y0) - (x - x0) * (y1 - y0) < 0.0 {
            w -= 1;
        }
    }
    w
}
fn fill_winding(p: &PathSnapshot, x: f64, y: f64) -> i32 {
    p.subpaths()
        .map(|sp| winding(&flatten(sp.anchors, sp.closed), x, y))
        .sum()
}
fn painted(p: &PathSnapshot, x: f64, y: f64) -> bool {
    fill_winding(p, x, y) != 0
}
fn painted_area(p: &PathSnapshot, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let step = 0.25;
    let mut n = 0u64;
    let mut y = y0 + step / 2.0;
    while y < y1 {
        let mut x = x0 + step / 2.0;
        while x < x1 {
            if painted(p, x, y) {
                n += 1;
            }
            x += step;
        }
        y += step;
    }
    n as f64 * step * step
}
fn shoelace(a: &[NewAnchor]) -> f64 {
    let mut s = 0.0;
    for i in 0..a.len() {
        let (p, q) = (&a[i].point, &a[(i + 1) % a.len()].point);
        s += p.x * q.y - q.x * p.y;
    }
    s / 2.0
}
fn applied(o: &CombineOutcome) -> (usize, usize, usize, usize, usize, usize, bool) {
    match o {
        CombineOutcome::Applied {
            operands,
            holes,
            open,
            touching,
            touching_holes,
            single_nodes_removed,
            markers,
            ..
        } => (
            *operands,
            *holes,
            *open,
            *touching,
            *touching_holes,
            *single_nodes_removed,
            *markers,
        ),
        other => panic!("not applied: {other:?}"),
    }
}
fn counts(p: &PathSnapshot) -> Vec<(usize, bool)> {
    p.subpaths()
        .map(|sp| (sp.anchors.len(), sp.closed))
        .collect()
}
fn pos(a: &[NewAnchor]) -> Vec<(f64, f64)> {
    a.iter().map(|n| (n.point.x, n.point.y)).collect()
}

// === Criterion 6, 8, 9, 10, 11: Combine accepts everything ======================================

#[test]
fn c6_two_open_lines_and_a_circle_give_three_outlines() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&line(0.0, 20.0, 10.0, 20.0), false);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    let out = s.apply_combine();
    let (operands, holes, open, ..) = applied(&out);
    assert_eq!((operands, holes, open), (3, 0, 2));
    let ps = paths(&s);
    assert_eq!(ps.len(), 1);
    assert!(ps[0].is_compound());
    assert_eq!(counts(&ps[0]), vec![(2, false), (2, false), (4, true)]);
}

#[test]
fn c8_none_of_the_old_refusals_exist() {
    // plate with an open score line across it
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 100.0, 100.0), true);
    d.create_path(&line(-10.0, 50.0, 110.0, 50.0), false);
    let mut s = session_all(&d);
    applied(&s.apply_combine());

    // ten open lines forming a crossing grid
    let d = Document::new(1);
    for i in 0..5 {
        let o = f64::from(i) * 10.0;
        d.create_path(&line(0.0, o, 50.0, o), false);
        d.create_path(&line(o, 0.0, o, 50.0), false);
    }
    let mut s = session_all(&d);
    let (n, _, open, ..) = applied(&s.apply_combine());
    assert_eq!((n, open), (10, 10));
    assert_eq!(paths(&s)[0].subpaths().count(), 10);

    // two overlapping closed squares plus an open line
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 20.0, 20.0), true);
    d.create_path(&rect(10.0, 10.0, 30.0, 30.0), true);
    d.create_path(&line(0.0, 40.0, 30.0, 40.0), false);
    let mut s = session_all(&d);
    applied(&s.apply_combine());

    // a figure eight with two separate squares
    let d = Document::new(1);
    d.create_path(
        &poly(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]),
        true,
    );
    d.create_path(&rect(50.0, 0.0, 60.0, 10.0), true);
    d.create_path(&rect(80.0, 0.0, 90.0, 10.0), true);
    let mut s = session_all(&d);
    applied(&s.apply_combine());

    // a closed path of two nodes with a circle (no area)
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), true);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
}

#[test]
fn c9_a_zero_length_open_outline_is_kept() {
    let d = Document::new(1);
    d.create_path(&line(5.0, 5.0, 5.0, 5.0), false);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
    assert_eq!(counts(&paths(&s)[0]), vec![(2, false), (4, true)]);
}

#[test]
fn c10_single_node_outlines_are_removed_or_refuse() {
    // A compound path that holds a one-node outline (a foreign file can).
    let d = Document::new(1);
    let seed = d.create_path(&line(500.0, 500.0, 501.0, 500.0), false);
    d.replace_with_path(
        &[seed],
        seed,
        &[
            (line(0.0, 0.0, 10.0, 0.0), false),
            (poly(&[(20.0, 20.0)]), false),
        ],
        "boolean_union",
    )
    .unwrap();
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    let (_, _, _, _, _, removed, _) = applied(&s.apply_combine());
    assert_eq!(removed, 1);
    assert_eq!(counts(&paths(&s)[0]), vec![(2, false), (4, true)]);

    // Every outline has one node: refused with the dedicated refusal.
    let d = Document::new(1);
    d.create_path(&poly(&[(0.0, 0.0)]), false);
    d.create_path(&poly(&[(50.0, 0.0)]), false);
    let mut s = session_all(&d);
    assert_eq!(s.selected_object_count(), 2, "two one-node paths exist");
    {
        let before = json(&s);
        assert_eq!(
            s.apply_combine(),
            CombineOutcome::Refused(CombineRefusal::SingleNodesOnly)
        );
        assert_eq!(json(&s), before, "nothing was changed");
    }
}

#[test]
fn c11_a_rectangle_contributes_its_outline() {
    let d = Document::new(1);
    d.create_rect(curvyo_document_core::RectBounds {
        origin: pt(0.0, 0.0),
        width: curvyo_document_core::Length::from_mm(10.0),
        height: curvyo_document_core::Length::from_mm(10.0),
    });
    d.create_path(&line(50.0, 0.0, 60.0, 0.0), false);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
    assert_eq!(counts(&paths(&s)[0]), vec![(4, true), (2, false)]);
}

// === Criterion 7: windings from depth ============================================================

#[test]
fn c7a_concentric_discs_still_give_a_ring() {
    let d = Document::new(1);
    d.create_path(&circle(100.0, 100.0, 20.0), true);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    let (_, holes, ..) = applied(&s.apply_combine());
    assert_eq!(holes, 1);
    let p = &paths(&s)[0];
    assert!(painted(p, 115.0, 100.0) && !painted(p, 100.0, 100.0));
}

#[test]
fn c7b_a_square_touching_the_inside_edge_is_a_hole() {
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 40.0, 40.0), true);
    d.create_path(&rect(0.0, 10.0, 20.0, 30.0), true);
    let mut s = session_all(&d);
    let (_, holes, ..) = applied(&s.apply_combine());
    assert_eq!(holes, 1);
    let p = &paths(&s)[0];
    assert!(!painted(p, 10.0, 20.0));
    assert!(painted(p, 30.0, 20.0));
}

#[test]
fn c7c_crossing_squares_are_both_shapes_and_the_picture_is_kept() {
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 20.0, 20.0), true);
    d.create_path(&rect(10.0, 10.0, 30.0, 30.0), true);
    let mut s = session_all(&d);
    let (_, holes, _, touching, ..) = applied(&s.apply_combine());
    assert_eq!((holes, touching), (0, 2));
    let p = &paths(&s)[0];
    assert!(painted(p, 15.0, 15.0));
    let area = painted_area(p, -1.0, -1.0, 31.0, 31.0);
    assert!((area - 700.0).abs() < 1.0, "area {area}");
}

#[test]
fn c7c_crossing_squares_keep_the_picture_in_every_direction_and_order() {
    for rev_a in [false, true] {
        for rev_b in [false, true] {
            let mut a = rect(0.0, 0.0, 20.0, 20.0);
            let mut b = rect(10.0, 10.0, 30.0, 30.0);
            if rev_a {
                a.reverse();
            }
            if rev_b {
                b.reverse();
            }
            let d = Document::new(1);
            d.create_path(&a, true);
            d.create_path(&b, true);
            let mut s = session_all(&d);
            let (_, holes, ..) = applied(&s.apply_combine());
            assert_eq!(holes, 0);
            let p = &paths(&s)[0];
            let area = painted_area(p, -1.0, -1.0, 31.0, 31.0);
            assert!((area - 700.0).abs() < 1.0, "{rev_a}/{rev_b}: {area}");
        }
    }
}

#[test]
fn c7d_coincident_squares_are_both_shapes() {
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 20.0, 20.0), true);
    d.create_path(&rect(0.0, 0.0, 20.0, 20.0), true);
    let mut s = session_all(&d);
    let (_, holes, ..) = applied(&s.apply_combine());
    assert_eq!(holes, 0);
    let p = &paths(&s)[0];
    assert!(painted(p, 10.0, 10.0));
    let area = painted_area(p, -1.0, -1.0, 21.0, 21.0);
    assert!((area - 400.0).abs() < 1.0);
}

#[test]
fn c7e_a_closed_figure_eight_paints_both_lobes_and_keeps_its_nodes() {
    let eight = poly(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
    let want = pos(&eight);
    let d = Document::new(1);
    d.create_path(&eight, true);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    let (_, holes, _, touching, ..) = applied(&s.apply_combine());
    assert_eq!(holes, 0);
    assert_eq!(touching, 1, "the figure eight crosses itself");
    let p = &paths(&s)[0];
    assert!(painted(p, 8.0, 5.0) && painted(p, 2.0, 5.0));
    // Zero net area: stored direction and node order are kept.
    assert_eq!(pos(&p.anchors), want);
}

#[test]
fn c7f_an_open_line_keeps_its_first_node_through_combine_and_break_apart() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
    s.apply_break_apart();
    let ps = paths(&s);
    let open: Vec<&PathSnapshot> = ps.iter().filter(|p| !p.closed).collect();
    assert_eq!(open.len(), 1);
    assert_eq!(pos(&open[0].anchors), vec![(0.0, 0.0), (10.0, 0.0)]);
}

#[test]
fn c7g_two_crossing_holes_in_a_square() {
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 40.0, 40.0), true);
    d.create_path(&rect(5.0, 5.0, 25.0, 25.0), true);
    d.create_path(&rect(15.0, 15.0, 35.0, 35.0), true);
    let mut s = session_all(&d);
    let (_, holes, _, touching, touching_holes, ..) = applied(&s.apply_combine());
    assert_eq!((holes, touching, touching_holes), (2, 2, 2));
    let p = &paths(&s)[0];
    assert!(!painted(p, 10.0, 10.0));
    assert!(painted(p, 20.0, 20.0));
    assert_eq!(fill_winding(p, 20.0, 20.0), -1);
    assert!(painted(p, 2.0, 2.0));
}

#[test]
fn c7_open_outlines_keep_direction_and_closed_ones_keep_every_node() {
    let d = Document::new(1);
    let open = poly(&[(5.0, 5.0), (5.0, 15.0), (15.0, 15.0)]);
    let open_pos = pos(&open);
    d.create_path(&rect(0.0, 0.0, 40.0, 40.0), true);
    d.create_path(&open, false);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
    let p = &paths(&s)[0];
    let outlines: Vec<_> = p.subpaths().collect();
    assert_eq!(pos(outlines[1].anchors), open_pos);
    assert!(!outlines[1].closed);
}

// === Criterion 12, 13, 14: notice counts, availability, markers ================================

#[test]
fn c12_holes_and_open_outlines_are_counted() {
    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 100.0, 100.0), true);
    d.create_path(&rect(10.0, 10.0, 30.0, 30.0), true);
    d.create_path(&line(50.0, 10.0, 90.0, 10.0), false);
    d.create_path(&line(-10.0, 60.0, 110.0, 60.0), false);
    let mut s = session_all(&d);
    let (n, holes, open, touching, th, ..) = applied(&s.apply_combine());
    assert_eq!((n, holes, open, touching, th), (4, 1, 2, 0, 0));
}

#[test]
fn c12_a_self_crossing_closed_outline_counts_once() {
    let d = Document::new(1);
    d.create_path(
        &poly(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]),
        true,
    );
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let mut s = session_all(&d);
    let (_, _, _, touching, ..) = applied(&s.apply_combine());
    assert_eq!(touching, 1);
}

#[test]
fn c12_combine_is_one_undo_step_labelled_combine_paths() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&rect(5.0, -5.0, 8.0, 5.0), true);
    let mut s = session_all(&d);
    let before = json(&s);
    let depth = s.undo_len();
    applied(&s.apply_combine());
    assert_eq!(s.undo_len(), depth + 1);
    s.undo();
    assert_eq!(json(&s), before, "undo restores both operands exactly");
    assert_eq!(s.undo_len(), depth);
}

#[test]
fn c13_availability_with_an_open_path_selected() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&circle(100.0, 100.0, 10.0), true);
    let s = session_all(&d);
    assert_eq!(s.path_availability().combine, BooleanAvailability::Ready);
}

#[test]
fn c14_markers_on_an_operand_are_reported() {
    let d = Document::new(1);
    let a = d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.edit_style(&[a], &StyleEdit::MarkerStart(MarkerShape::Arrow))
        .unwrap();
    d.create_path(&line(0.0, 10.0, 10.0, 10.0), false);
    let mut s = session_all(&d);
    let (.., markers) = applied(&s.apply_combine());
    assert!(markers);

    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&line(0.0, 10.0, 10.0, 10.0), false);
    let mut s = session_all(&d);
    let (.., markers) = applied(&s.apply_combine());
    assert!(!markers);
}

// === Criterion 4 and 1: selection and hit testing of a compound path ===========================

fn click(s: &mut Session, x: f64, y: f64) {
    s.pointer_hover(pt(x, y), false, false);
    s.pointer_down(pt(x, y), false);
    s.pointer_up(pt(x, y), false, false);
}

#[test]
fn c4_click_hits_the_open_outline_stroke_and_the_fill_area_only() {
    let d = Document::new(1);
    let seed = d.create_path(&line(500.0, 500.0, 501.0, 500.0), false);
    let id = d
        .replace_with_path(
            &[seed],
            seed,
            &[(rect(200.0, 200.0, 220.0, 220.0), true), (u_shape(), false)],
            "boolean_union",
        )
        .unwrap();
    d.edit_style(&[id], &StyleEdit::FillEnabled(false)).unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(true))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(1.0)),
    )
    .unwrap();
    let mut s = session_of(&d);
    click(&mut s, 5.0, 10.0);
    assert_eq!(s.selected_object_count(), 1, "stroke of the open outline");
    click(&mut s, 400.0, 400.0);
    assert_eq!(s.selected_object_count(), 0);
    click(&mut s, 5.0, 0.0);
    assert_eq!(
        s.selected_object_count(),
        0,
        "no stroke on the closing line"
    );
    click(&mut s, 5.0, 5.0);
    assert_eq!(
        s.selected_object_count(),
        0,
        "fill off: the inside is not hit"
    );

    let d2 = Document::new(1);
    let seed = d2.create_path(&line(500.0, 500.0, 501.0, 500.0), false);
    let id = d2
        .replace_with_path(
            &[seed],
            seed,
            &[(rect(200.0, 200.0, 220.0, 220.0), true), (u_shape(), false)],
            "boolean_union",
        )
        .unwrap();
    d2.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d2.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    let mut s = session_of(&d2);
    click(&mut s, 5.0, 5.0);
    assert_eq!(
        s.selected_object_count(),
        1,
        "fill on: the closed U area is hit"
    );
}

// === Criterion 15 to 17: Split ==================================================================

fn compound_of(d: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let seed = d.create_path(&line(9000.0, 9000.0, 9001.0, 9000.0), false);
    d.replace_with_path(&[seed], seed, outlines, "boolean_union")
        .unwrap()
}

#[test]
fn c15_plate_hole_engraving_and_far_line_give_two_objects() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 100.0, 100.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
            (line(50.0, 50.0, 80.0, 50.0), false),
            (line(300.0, 0.0, 340.0, 0.0), false),
        ],
    );
    let mut s = session_all(&d);
    let out = s.apply_split();
    assert!(
        matches!(out, SplitOutcome::Applied { pieces: 2, .. }),
        "{out:?}"
    );
    let ps = paths(&s);
    assert_eq!(ps.len(), 2);
    let shapes: Vec<_> = ps.iter().map(counts).collect();
    assert!(
        shapes.contains(&vec![(4, true), (4, true), (2, false)]),
        "{shapes:?}"
    );
    assert!(shapes.contains(&vec![(2, false)]), "{shapes:?}");
    let far = ps.iter().find(|p| !p.is_compound()).unwrap();
    assert!(!far.closed, "the far line is an ordinary open path");
}

#[test]
fn c15_crossing_squares_and_a_far_circle_give_two_objects() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
            (circle(200.0, 200.0, 10.0), true),
        ],
    );
    let mut s = session_all(&d);
    assert!(matches!(
        s.apply_split(),
        SplitOutcome::Applied { pieces: 2, .. }
    ));
    let ps = paths(&s);
    let shapes: Vec<_> = ps.iter().map(counts).collect();
    assert!(shapes.contains(&vec![(4, true), (4, true)]), "{shapes:?}");
    assert!(shapes.contains(&vec![(4, true)]), "{shapes:?}");
    assert!(ps.iter().any(|p| !p.is_compound() && p.closed));
}

#[test]
fn c15_rule_a_an_open_line_touching_a_part_joins_it_and_rule_c_makes_its_own() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(100.0, 0.0, 120.0, 20.0), true),
            // crosses the edge of the first square, first node outside both squares
            (line(-10.0, 10.0, 10.0, 10.0), false),
            // starts at the first node outside everything, far from any shape
            (line(500.0, 500.0, 510.0, 500.0), false),
        ],
    );
    let mut s = session_all(&d);
    assert!(matches!(
        s.apply_split(),
        SplitOutcome::Applied { pieces: 3, .. }
    ));
    let shapes: Vec<_> = paths(&s).iter().map(counts).collect();
    assert!(shapes.contains(&vec![(4, true), (2, false)]), "{shapes:?}");
    assert!(shapes.contains(&vec![(4, true)]));
    assert!(shapes.contains(&vec![(2, false)]));
}

#[test]
fn c15_rule_a_one_open_line_touching_two_parts_joins_them() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(100.0, 0.0, 120.0, 20.0), true),
            (line(10.0, 10.0, 110.0, 10.0), false),
        ],
    );
    let mut s = session_all(&d);
    let before = json(&s);
    let out = s.apply_split();
    assert!(
        matches!(
            out,
            SplitOutcome::Refused(SplitRefusal::OnePiece {
                with_holes: false,
                ..
            })
        ),
        "{out:?}"
    );
    assert_eq!(json(&s), before);
}

#[test]
fn c16_a_part_of_one_closed_outline_is_an_ordinary_closed_path_even_if_it_crosses_itself() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (
                poly(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]),
                true,
            ),
            (rect(100.0, 0.0, 120.0, 20.0), true),
        ],
    );
    let mut s = session_all(&d);
    assert!(matches!(
        s.apply_split(),
        SplitOutcome::Applied { pieces: 2, .. }
    ));
    for p in paths(&s) {
        assert!(!p.is_compound());
        assert!(p.closed);
    }
}

#[test]
fn c17_one_piece_refusal_depends_on_holes() {
    // plate with score lines across its edge: one piece, no hole
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 100.0, 100.0), true),
            (line(-10.0, 50.0, 110.0, 50.0), false),
        ],
    );
    let mut s = session_all(&d);
    let before = json(&s);
    assert_eq!(
        s.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );
    assert_eq!(json(&s), before);

    // with a hole
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 100.0, 100.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
            (line(-10.0, 50.0, 110.0, 50.0), false),
        ],
    );
    let mut s = session_all(&d);
    assert_eq!(
        s.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: true
        })
    );

    // two crossing squares
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
        ],
    );
    let mut s = session_all(&d);
    assert_eq!(
        s.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );
}

// === Criterion 18, 19: Break apart ==============================================================

#[test]
fn c18_break_apart_gives_two_closed_and_two_open_paths() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
            (line(0.0, 50.0, 30.0, 50.0), false),
            (poly(&[(0.0, 60.0), (15.0, 70.0), (30.0, 60.0)]), false),
        ],
    );
    let mut s = session_all(&d);
    let out = s.apply_break_apart();
    assert!(
        matches!(out, BreakApartOutcome::Applied { pieces: 4, .. }),
        "{out:?}"
    );
    let ps = paths(&s);
    assert_eq!(ps.len(), 4);
    assert!(ps.iter().all(|p| !p.is_compound()));
    assert_eq!(ps.iter().filter(|p| p.closed).count(), 2);
}

#[test]
fn c18_break_apart_keeps_a_figure_eight_as_one_path_and_drops_single_nodes() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (
                poly(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]),
                true,
            ),
            (poly(&[(50.0, 50.0)]), false),
            (rect(100.0, 0.0, 120.0, 20.0), true),
        ],
    );
    let mut s = session_all(&d);
    let out = s.apply_break_apart();
    assert!(
        matches!(
            out,
            BreakApartOutcome::Applied {
                pieces: 2,
                single_nodes_removed: 1,
                ..
            }
        ),
        "{out:?}"
    );
}

#[test]
fn c19_combine_then_break_apart_returns_open_paths_node_for_node() {
    let d = Document::new(1);
    let a = poly(&[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)]);
    let b = line(0.0, 30.0, 40.0, 30.0);
    let want_a = pos(&a);
    let want_b = pos(&b);
    d.create_path(&a, false);
    d.create_path(&b, false);
    d.create_path(&rect(100.0, 0.0, 120.0, 20.0), true);
    let mut s = session_all(&d);
    applied(&s.apply_combine());
    s.apply_break_apart();
    let ps = paths(&s);
    let open: Vec<Vec<(f64, f64)>> = ps
        .iter()
        .filter(|p| !p.closed)
        .map(|p| pos(&p.anchors))
        .collect();
    assert!(open.contains(&want_a) && open.contains(&want_b), "{open:?}");
    assert_eq!(ps.len(), 3);
}

#[test]
fn c19_split_or_break_apart_then_combine_gives_the_same_outlines() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (rect(10.0, 10.0, 30.0, 30.0), true),
            (line(0.0, 50.0, 30.0, 50.0), false),
        ],
    );
    let mut s = session_all(&d);
    let before = counts(&paths(&s)[0]);
    s.apply_break_apart();
    select_all(&mut s);
    applied(&s.apply_combine());
    let mut after = counts(&paths(&s)[0]);
    let mut b = before;
    b.sort_unstable();
    after.sort_unstable();
    assert_eq!(after, b);
}

#[test]
fn c31_split_and_break_apart_are_one_undo_step_and_restore_everything() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (line(0.0, 50.0, 30.0, 50.0), false),
            (line(300.0, 50.0, 330.0, 50.0), false),
        ],
    );
    let mut s = session_all(&d);
    let before = json(&s);
    let depth = s.undo_len();
    assert!(matches!(s.apply_split(), SplitOutcome::Applied { .. }));
    assert_eq!(s.undo_len(), depth + 1);
    s.undo();
    assert_eq!(json(&s), before);
    select_all(&mut s);
    assert!(matches!(
        s.apply_break_apart(),
        BreakApartOutcome::Applied { .. }
    ));
    assert_eq!(s.undo_len(), depth + 1);
    s.undo();
    assert_eq!(json(&s), before);
}

// === Criterion 20: Cut ==========================================================================

#[test]
fn c20_cut_refuses_a_compound_path_with_open_outlines() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (line(-5.0, 10.0, 25.0, 10.0), false),
        ],
    );
    let mut s = session_all(&d);
    let before = json(&s);
    let out = s.apply_cut();
    assert!(
        matches!(
            out,
            curvyo_editor_wasm::CutOutcome::Refused(CutRefusal::Compound { .. })
        ),
        "{out:?}"
    );
    assert_eq!(json(&s), before);
}

// === Criterion 21 to 23: Booleans ===============================================================

#[test]
fn c21_union_of_a_u_and_a_far_square_closes_the_u_with_a_straight_edge() {
    let d = Document::new(1);
    d.create_path(&u_shape(), false);
    d.create_path(&rect(100.0, 100.0, 120.0, 120.0), true);
    let mut s = session_all(&d);
    let out = s.apply_boolean(BooleanOp::Union);
    assert_eq!(
        out,
        BooleanOutcome::Applied {
            operands: 2,
            compound: true,
            closed_open: 1,
            removed_open: 0
        }
    );
    let p = &paths(&s)[0];
    assert_eq!(p.subpaths().count(), 2);
    assert!(
        p.subpaths().all(|sp| sp.closed),
        "Boolean results are closed"
    );
    let u_outline = p
        .subpaths()
        .find(|sp| {
            sp.anchors
                .iter()
                .all(|a| a.point.x <= 10.5 && a.point.y <= 10.5)
        })
        .expect("the U outline");
    assert!((shoelace(u_outline.anchors).abs() - 100.0).abs() < 0.5);
    let n = u_outline.anchors.len();
    let has_edge = (0..n).any(|i| {
        let (a, b) = (
            &u_outline.anchors[i].point,
            &u_outline.anchors[(i + 1) % n].point,
        );
        let ends = |p: &Point, x: f64, y: f64| (p.x - x).abs() < 1e-6 && (p.y - y).abs() < 1e-6;
        (ends(a, 10.0, 0.0) && ends(b, 0.0, 0.0)) || (ends(b, 10.0, 0.0) && ends(a, 0.0, 0.0))
    });
    assert!(
        has_edge,
        "straight edge (10,0)-(0,0): {:?}",
        pos(u_outline.anchors)
    );
}

#[test]
fn c21_a_single_open_path_with_area_is_an_operand_too() {
    let d = Document::new(1);
    d.create_path(&u_shape(), false);
    d.create_path(&rect(5.0, 5.0, 30.0, 30.0), true);
    let mut s = session_all(&d);
    assert!(matches!(
        s.apply_boolean(BooleanOp::Intersection),
        BooleanOutcome::Applied { closed_open: 1, .. }
    ));
    let p = &paths(&s)[0];
    // U closed region = (0..10)x(0..10); square (5..30)^2; overlap = 25
    let area = painted_area(p, 0.0, 0.0, 12.0, 12.0);
    assert!((area - 25.0).abs() < 0.5, "{area}");
}

#[test]
fn c22_a_straight_score_line_vanishes_from_the_boolean_and_is_reported() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 100.0, 100.0), true),
            (line(10.0, 50.0, 90.0, 50.0), false),
        ],
    );
    d.create_path(&circle(50.0, 50.0, 20.0), true);
    let mut s = session_all(&d);
    let out = s.apply_boolean(BooleanOp::Difference);
    assert!(
        matches!(
            out,
            BooleanOutcome::Applied {
                removed_open: 1,
                closed_open: 0,
                ..
            }
        ),
        "{out:?}"
    );
    let p = &paths(&s)[0];
    assert!(p.subpaths().all(|sp| sp.closed));
    let area = painted_area(p, -1.0, -1.0, 101.0, 101.0);
    let want = 10000.0 - std::f64::consts::PI * 400.0;
    assert!((area - want).abs() < 2.0, "area {area}, want {want}");
    assert!(!painted(p, 50.0, 50.0));
}

#[test]
fn c22_an_operand_without_any_area_is_still_refused() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 50.0, 0.0), false);
    d.create_path(&rect(100.0, 0.0, 120.0, 20.0), true);
    let mut s = session_all(&d);
    let before = json(&s);
    let out = s.apply_boolean(BooleanOp::Union);
    assert!(
        matches!(out, BooleanOutcome::Refused(BooleanRefusal::NoArea { .. })),
        "{out:?}"
    );
    assert_eq!(json(&s), before);
}

#[test]
fn c22_a_zero_length_open_outline_inside_an_operand_is_removed() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 100.0, 100.0), true),
            (line(50.0, 50.0, 50.0, 50.0), false),
        ],
    );
    d.create_path(&circle(500.0, 500.0, 10.0), true);
    let mut s = session_all(&d);
    assert!(matches!(
        s.apply_boolean(BooleanOp::Union),
        BooleanOutcome::Applied {
            removed_open: 1,
            ..
        }
    ));
}

#[test]
fn c23_availability_counts_open_outlines() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 10.0, 0.0), false);
    d.create_path(&u_shape(), false);
    d.create_path(&rect(100.0, 0.0, 120.0, 20.0), true);
    let s = session_all(&d);
    // UX review finding 3: the straight line encloses nothing and is not counted; the U is.
    assert_eq!(
        s.boolean_availability(),
        BooleanAvailability::ClosesOpen { outlines: 1 }
    );

    let d = Document::new(1);
    d.create_path(&rect(0.0, 0.0, 10.0, 10.0), true);
    d.create_path(&rect(100.0, 0.0, 120.0, 20.0), true);
    let s = session_all(&d);
    assert_eq!(s.boolean_availability(), BooleanAvailability::Ready);
}

#[test]
fn c31_a_boolean_is_one_undo_step_and_undo_restores_open_operands() {
    let d = Document::new(1);
    d.create_path(&u_shape(), false);
    d.create_path(&rect(100.0, 100.0, 120.0, 120.0), true);
    let mut s = session_all(&d);
    let before = json(&s);
    let depth = s.undo_len();
    s.apply_boolean(BooleanOp::Union);
    assert_eq!(s.undo_len(), depth + 1);
    s.undo();
    assert_eq!(json(&s), before);
    let ps = paths(&s);
    assert!(ps.iter().any(|p| !p.closed && p.anchors.len() == 4));
}

// === Criterion 28: save and reopen ==============================================================

#[test]
fn c28_save_and_reopen_keeps_open_and_crossing_outlines() {
    let d = Document::new(1);
    compound_of(
        &d,
        &[
            (rect(0.0, 0.0, 20.0, 20.0), true),
            (poly(&[(5.0, 5.0), (30.0, 8.0), (12.0, 40.0)]), false),
            (rect(10.0, 10.0, 30.0, 30.0), true),
        ],
    );
    let s = session_of(&d);
    let before = paths(&s);
    let bytes = s.pack("0.1.0").unwrap();
    let again = unpack(7, &bytes).unwrap();
    let ids = again.object_ids();
    let reread: Vec<PathSnapshot> = ids.into_iter().filter_map(|i| again.path(i)).collect();
    assert_eq!(reread.len(), 1);
    assert_eq!(counts(&reread[0]), counts(&before[0]));
    for (a, b) in reread[0].subpaths().zip(before[0].subpaths()) {
        assert_eq!(pos(a.anchors), pos(b.anchors));
        assert_eq!(a.closed, b.closed);
    }
}

// === Criterion 33: determinism ==================================================================

#[test]
fn c33_selection_order_does_not_change_the_result_and_nothing_moves_a_node() {
    let build = || {
        let d = Document::new(1);
        d.create_path(&line(0.0, 0.0, 10.0, 3.0), false);
        d.create_path(&circle(100.0, 0.0, 10.0), true);
        d.create_path(&poly(&[(0.0, 50.0), (10.0, 55.0), (20.0, 50.0)]), false);
        d
    };
    let a = session_all(&build());
    let mut a = a;
    applied(&a.apply_combine());
    let mut b = session_of(&build());
    for x in [10.0, 105.0, 0.0] {
        b.pointer_hover(pt(x, 0.0), true, false);
        b.pointer_down(pt(x, 0.0), true);
        b.pointer_up(pt(x, 0.0), true, false);
    }
    let _ = b.selected_object_count();
    select_all(&mut b);
    applied(&b.apply_combine());
    let flat = |p: &PathSnapshot| -> Vec<(f64, f64)> {
        p.subpaths().flat_map(|sp| pos(sp.anchors)).collect()
    };
    let pa = flat(&paths(&a)[0]);
    let pb = flat(&paths(&b)[0]);
    assert_eq!(pa.len(), pb.len());
    for (x, y) in pa.iter().zip(&pb) {
        assert!((x.0 - y.0).abs() < 1e-9 && (x.1 - y.1).abs() < 1e-9);
    }
    // Original coordinates survive exactly (no arithmetic on nodes).
    assert!(pa.contains(&(10.0, 3.0)) && pa.contains(&(10.0, 55.0)));
}

// === Criterion 32: budgets (assert timing in release only) ======================================

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    let e = t.elapsed();
    println!("{label}: {e:?}");
    if !cfg!(debug_assertions) {
        assert!(e.as_secs_f64() < 2.0, "{label} took {e:?}");
    }
    r
}

fn gen_a(d: &Document) {
    d.create_path(&rect(-10.0, -10.0, 520.0, 420.0), true);
    for r in 0..40 {
        for c in 0..50 {
            let (x, y) = (f64::from(c) * 10.0, f64::from(r) * 10.0);
            d.create_path(&line(x, y, x + 5.0, y), false);
        }
    }
}

#[test]
fn c32_budget_a_plate_with_2000_open_lines() {
    let d = Document::new(1);
    gen_a(&d);
    let mut s = session_all(&d);
    let out = timed("combine a", || s.apply_combine());
    applied(&out);
    let p = &paths(&s)[0];
    assert_eq!(p.subpaths().count(), 2001);
    // (c) split / break apart
    let before = json(&s);
    let out = timed("split c", || s.apply_split());
    assert_eq!(
        out,
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );
    assert_eq!(json(&s), before);
    let out = timed("break apart c", || s.apply_break_apart());
    assert!(
        matches!(out, BreakApartOutcome::Applied { pieces: 2001, .. }),
        "{out:?}"
    );
    let ps = paths(&s);
    assert_eq!(ps.iter().filter(|p| p.closed).count(), 1);
    assert_eq!(ps.iter().filter(|p| !p.closed).count(), 2000);
}

#[test]
fn c32_budget_b_crossing_grid_of_2000_lines() {
    let d = Document::new(1);
    for i in 0..1000 {
        let o = f64::from(i);
        d.create_path(&line(0.0, o, 1000.0, o), false);
        d.create_path(&line(o, 0.0, o, 1000.0), false);
    }
    let mut s = session_all(&d);
    let out = timed("combine b", || s.apply_combine());
    applied(&out);
    assert_eq!(paths(&s)[0].subpaths().count(), 2000);
}

#[test]
fn c32_budget_d_chain_of_1000_crossing_squares() {
    let d = Document::new(1);
    for i in 0..1000 {
        let x = f64::from(i) * 8.0;
        d.create_path(&rect(x, 0.0, x + 10.0, 10.0), true);
    }
    let mut s = session_all(&d);
    let out = timed("combine d", || s.apply_combine());
    let (n, holes, _, touching, ..) = applied(&out);
    assert_eq!((n, holes, touching), (1000, 0, 1000));
    assert_eq!(paths(&s)[0].subpaths().count(), 1000);
}

// === Hostile input: refusals that stay ==========================================================

#[test]
fn combine_and_boolean_refuse_a_coordinate_beyond_the_kernel_range_without_changing_anything() {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 2.0e7, 0.0), false);
    d.create_path(&rect(0.0, 0.0, 10.0, 10.0), true);
    let mut s = session_of(&d);
    marquee(&mut s, pt(-1.0e8, -1.0e8), pt(1.0e8, 1.0e8));
    assert_eq!(s.selected_object_count(), 2);
    {
        let before = json(&s);
        assert!(matches!(
            s.apply_combine(),
            CombineOutcome::Refused(CombineRefusal::OutOfRange { .. })
        ));
        assert_eq!(json(&s), before);
        assert!(matches!(
            s.apply_boolean(BooleanOp::Union),
            BooleanOutcome::Refused(BooleanRefusal::OutOfRange { .. })
        ));
        assert_eq!(json(&s), before);
    }
}
