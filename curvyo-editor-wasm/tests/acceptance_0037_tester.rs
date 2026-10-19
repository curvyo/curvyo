//! Independent session-level acceptance tests for `0037-fracture-and-flatten`, written from the
//! specification before the implementation was read. Only the criteria the implementer's tests
//! leave open or cover weakly: open outlines and refusals (2, 3), the untouched operand (6), the
//! complete style and the piece order (8), undo (9), compound results with their place and
//! style (10, 12, 18) and the 5,000 limit (4).

#![allow(
    unused_must_use,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::missing_panics_doc,
    clippy::many_single_char_names
)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, AnchorKind, Color, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot,
    PathSnapshot, Point, RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{Session, StackOutcome, Tool};
use curvyo_ui_core::{BooleanRefusal, StackRefusal};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn corner(x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(
        AnchorId::new(37, NEXT.fetch_add(1, Ordering::SeqCst)),
        Point::new(x, y),
    )
}

fn polyline(points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points.iter().map(|&(x, y)| corner(x, y)).collect()
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
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

fn select_all(s: &mut Session) {
    let (from, to) = (Point::new(-300.0, -300.0), Point::new(5000.0, 5000.0));
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(Point::new(2000.0, 2000.0), false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn styled(d: &Document, id: NodeId, stroke: Color, width: f64, fill: Color) {
    d.edit_style(&[id], &StyleEdit::StrokeColor(stroke));
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(width)));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true));
    d.edit_style(&[id], &StyleEdit::FillColor(fill));
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b }
}

fn path(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id).unwrap() {
        ObjectSnapshot::Path(p) => p,
        other => panic!("a path expected: {other:?}"),
    }
}

fn all_outlines(p: &PathSnapshot) -> Vec<Vec<(f64, f64)>> {
    std::iter::once(&p.anchors)
        .chain(p.extra_subpaths.iter().map(|s| &s.anchors))
        .map(|a| a.iter().map(|n| (n.point.x, n.point.y)).collect())
        .collect()
}

fn shoelace(poly: &[(f64, f64)]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

/// Criterion 18 (and `0016` criterion 24): a result is an ordinary path or a compound path
/// whose nodes are Corner nodes without handles.
fn assert_plain_corners(p: &PathSnapshot) {
    assert!(p.closed && p.extra_subpaths.iter().all(|s| s.closed));
    for n in std::iter::once(&p.anchors)
        .chain(p.extra_subpaths.iter().map(|s| &s.anchors))
        .flatten()
    {
        assert_eq!(n.kind, AnchorKind::Corner);
        assert!(n.handle_in.x == 0.0 && n.handle_in.y == 0.0);
        assert!(n.handle_out.x == 0.0 && n.handle_out.y == 0.0);
    }
}

fn applied(o: &StackOutcome) -> (usize, usize, usize, usize, usize) {
    match o {
        StackOutcome::Applied {
            operands,
            removed,
            pieces,
            hidden,
            closed_open,
            ..
        } => (*operands, *removed, *pieces, *hidden, *closed_open),
        other => panic!("applied expected: {other:?}"),
    }
}

/// Criterion 2 as amended by `0054` criterion 24: an open outline is closed with a straight
/// line and counted, not refused.
#[test]
fn c2_an_open_outline_is_closed_and_counted() {
    let d = Document::new(1);
    rect(&d, 0.0, 0.0, 20.0, 20.0);
    // An open L-shaped polyline whose closing line makes a triangle over the square's corner.
    d.create_path(
        &polyline(&[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)]),
        false,
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), 2);
    let (operands, removed, pieces, _, closed_open) = applied(&s.apply_fracture());
    assert_eq!((operands, removed, closed_open), (2, 2, 1));
    assert_eq!(
        pieces, 3,
        "the square without the triangle, the overlap, the triangle rest"
    );
}

/// Criterion 2: an operand that encloses no area is refused with its offender; nothing changes.
#[test]
fn c2_an_operand_without_area_is_refused_with_its_offender() {
    let d = Document::new(1);
    rect(&d, 0.0, 0.0, 20.0, 20.0);
    rect(&d, 10.0, 10.0, 20.0, 20.0);
    let flat = d.create_path(&polyline(&[(0.0, 5.0), (10.0, 5.0), (20.0, 5.0)]), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    for outcome in [s.apply_fracture(), s.apply_flatten()] {
        match outcome {
            StackOutcome::Refused(StackRefusal::Operands(BooleanRefusal::NoArea {
                offenders,
                of,
            })) => assert_eq!((offenders, of), (vec![flat], 3)),
            other => panic!("no-area refusal expected: {other:?}"),
        }
    }
    assert_eq!(json(&s), before);
    assert_eq!(s.selected_object_count(), 3);
}

/// Criterion 3: Flatten on shapes that only touch is refused too, and nothing changes.
#[test]
fn c3_flatten_refuses_touching_shapes() {
    let d = Document::new(1);
    rect(&d, 0.0, 0.0, 10.0, 10.0);
    rect(&d, 10.0, 0.0, 10.0, 10.0);
    rect(&d, 20.0, 10.0, 10.0, 10.0);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    assert!(matches!(
        s.apply_flatten(),
        StackOutcome::Refused(StackRefusal::NoOverlap)
    ));
    assert_eq!(json(&s), before);
}

/// Criteria 6 and 9: an ellipse that overlaps nothing keeps its id and stays a primitive (its
/// curves); it is selected with the pieces; one undo step brings the document back.
#[test]
fn c6_c9_an_operand_apart_is_kept_and_undo_restores_everything() {
    let d = Document::new(1);
    rect(&d, 0.0, 0.0, 20.0, 20.0);
    rect(&d, 10.0, 10.0, 20.0, 20.0);
    let apart = d.create_ellipse(EllipseFrame {
        center: Point::new(100.0, 10.0),
        rx: Length::from_mm(8.0),
        ry: Length::from_mm(5.0),
    });
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let ellipse_before = reread(&s).object(apart).unwrap();
    let (operands, removed, pieces, _, _) = applied(&s.apply_fracture());
    assert_eq!((operands, removed, pieces), (3, 2, 3));
    let after = reread(&s);
    assert_eq!(after.object(apart), Some(ellipse_before));
    assert!(matches!(
        after.object(apart),
        Some(ObjectSnapshot::Primitive(_))
    ));
    assert_eq!(
        s.selected_object_count(),
        4,
        "3 pieces and the kept ellipse"
    );
    assert_eq!(s.tool(), Tool::Select);
    s.undo();
    assert_eq!(json(&s), before, "one undo step takes Fracture back");
}

/// Criterion 8: every piece copies its owner's complete style (stroke colour and width, fill),
/// and the pieces of one owner are ordered by their first node, left to right.
#[test]
fn c8_complete_style_and_pieces_ordered_left_to_right() {
    let d = Document::new(1);
    // A U (arms at x 0..10 and 20..30, open at the top y < 20) below a bar across both arms.
    let u = d.create_path(
        &polyline(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 20.0),
            (20.0, 20.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (30.0, 30.0),
            (0.0, 30.0),
        ]),
        true,
    );
    let bar = rect(&d, -5.0, 5.0, 40.0, 5.0);
    styled(&d, u, rgb(10, 20, 30), 0.7, rgb(200, 100, 0));
    styled(&d, bar, rgb(40, 50, 60), 1.3, rgb(0, 100, 200));
    let (u_style, bar_style) = (
        d.object(u).unwrap().style().clone(),
        d.object(bar).unwrap().style().clone(),
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let (_, removed, pieces, _, _) = applied(&s.apply_fracture());
    // The bar owns two overlaps and three parts of its own (left of, between and right of the
    // arms); the U keeps the feet of both arms and its top.
    assert_eq!((removed, pieces), (2, 8));
    let after = reread(&s);
    let objects: Vec<PathSnapshot> = after
        .object_ids()
        .into_iter()
        .map(|id| path(&after, id))
        .collect();
    let first_node = |p: &PathSnapshot| {
        all_outlines(p)[0]
            .iter()
            .copied()
            .fold((f64::INFINITY, f64::INFINITY), |m, q| {
                if q.0 < m.0 || (q.0 == m.0 && q.1 < m.1) {
                    q
                } else {
                    m
                }
            })
    };
    let (u_pieces, bar_pieces) = objects.split_at(3);
    assert!(
        u_pieces.iter().all(|p| p.style == u_style),
        "the U's pieces are below"
    );
    assert!(bar_pieces.iter().all(|p| p.style == bar_style));
    for pieces in [u_pieces, bar_pieces] {
        let firsts: Vec<(f64, f64)> = pieces.iter().map(first_node).collect();
        assert!(
            firsts
                .windows(2)
                .all(|w| w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1)),
            "pieces of one owner by first node: {firsts:?}"
        );
    }
    for p in &objects {
        assert_plain_corners(p);
    }
}

/// Criteria 10, 12 and 18: a square cut in two by a bar on top becomes one compound path of two
/// outlines; a square with a smaller one on top becomes a compound path with a hole (opposite
/// winding). Both keep their place below an unselected object and their complete style.
#[test]
fn c10_c12_visible_regions_of_several_areas_or_with_a_hole_are_one_compound_path() {
    let d = Document::new(1);
    let cut = rect(&d, 0.0, 0.0, 30.0, 30.0);
    let holed = rect(&d, 100.0, 0.0, 30.0, 30.0);
    let between = rect(&d, 300.0, 0.0, 10.0, 10.0);
    let bar = rect(&d, -5.0, 10.0, 40.0, 10.0);
    let inner = rect(&d, 110.0, 10.0, 10.0, 10.0);
    styled(&d, cut, rgb(1, 2, 3), 0.4, rgb(9, 8, 7));
    styled(&d, holed, rgb(4, 5, 6), 0.9, rgb(7, 7, 7));
    let (cut_style, holed_style) = (
        d.object(cut).unwrap().style().clone(),
        d.object(holed).unwrap().style().clone(),
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    // `between` overlaps nothing and is selected too: it is kept.
    let (operands, removed, pieces, hidden, _) = applied(&s.apply_flatten());
    assert_eq!((operands, removed, pieces, hidden), (5, 2, 2, 0));
    let after = reread(&s);
    let ids = after.object_ids();
    assert_eq!(ids.len(), 5);
    assert_eq!(
        &ids[2..],
        &[between, bar, inner],
        "results stay below the rest"
    );
    let (two, ring) = (path(&after, ids[0]), path(&after, ids[1]));
    assert_eq!(two.style, cut_style);
    assert_eq!(ring.style, holed_style);
    let two_areas: Vec<f64> = all_outlines(&two).iter().map(|o| shoelace(o)).collect();
    assert_eq!(two_areas.len(), 2);
    assert!(
        two_areas.iter().all(|a| (a.abs() - 300.0).abs() < 1e-6),
        "{two_areas:?}"
    );
    assert!(
        two_areas[0] * two_areas[1] > 0.0,
        "two islands wind the same way"
    );
    let ring_areas: Vec<f64> = all_outlines(&ring).iter().map(|o| shoelace(o)).collect();
    assert_eq!(ring_areas.len(), 2);
    assert!(
        ring_areas[0] * ring_areas[1] < 0.0,
        "a hole winds the other way"
    );
    assert!((ring_areas.iter().sum::<f64>().abs() - 800.0).abs() < 1e-6);
    assert_plain_corners(&two);
    assert_plain_corners(&ring);
    assert_eq!(s.selected_object_count(), 5);
}

/// Criterion 4: more than 5,000 pieces is refused, and nothing changes. A lattice of 72
/// horizontal and 72 vertical thin bars crosses in 5,184 places, so Fracture makes more than
/// 5,000 pieces while Flatten makes only 72 trimmed bars.
#[test]
fn c4_more_than_5000_pieces_is_refused() {
    let d = Document::new(1);
    for i in 0..72 {
        let at = f64::from(i) * 4.0;
        rect(&d, -2.0, at, 290.0, 1.0);
    }
    for i in 0..72 {
        let at = f64::from(i) * 4.0;
        rect(&d, at, -2.0, 1.0, 290.0);
    }
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), 144);
    let before = json(&s);
    assert!(
        matches!(
            s.apply_fracture(),
            StackOutcome::Refused(StackRefusal::TooManyPieces)
        ),
        "fracture refused"
    );
    assert_eq!(json(&s), before);
    assert_eq!(s.selected_object_count(), 144);
    let (_, removed, pieces, hidden, _) = applied(&s.apply_flatten());
    assert_eq!((removed, pieces, hidden), (72, 72, 0));
}
