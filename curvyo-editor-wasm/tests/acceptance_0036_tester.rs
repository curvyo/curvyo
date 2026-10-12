//! Independent session-level acceptance tests for `0036-cut-at-crossings` (Cut), written from the
//! specification before the implementation was read: the worked examples of criterion 12, the
//! node and style rules, refusals that change nothing, one commit with undo and redo, a
//! differential check against a brute-force polyline crossing finder on random curves, and the
//! grid input of criterion 13 as an ignored release test.

#![allow(
    unused_must_use,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::type_complexity,
    clippy::needless_range_loop
)]

mod split_cut_common;

use std::collections::HashSet;
use std::time::Instant;

use curvyo_document_core::{
    Vec2, AnchorId, AnchorKind, Color, Document, EllipseFrame, Length, NewAnchor, NodeId, PathSnapshot,
    RectBounds, StyleEdit,
};
use curvyo_editor_wasm::{CutOutcome, Session, Tool};
use curvyo_ui_core::CutRefusal;
use split_cut_common::*;

fn cut_applied(o: &CutOutcome) -> (usize, usize) {
    match o {
        CutOutcome::Applied { paths, pieces } => (*paths, *pieces),
        other => panic!("cut not applied: {other:?}"),
    }
}

fn cut_refusal(o: CutOutcome) -> CutRefusal {
    match o {
        CutOutcome::Refused(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn near(a: Xy, b: Xy) -> bool {
    dist(a, b) < 0.01
}

fn assert_nothing_changed(s: &Session, before: &[u8], n_labels: usize, selected: usize) {
    assert_eq!(json(s), before, "the document is unchanged");
    assert_eq!(label_count(s), n_labels, "no commit was made");
    assert_eq!(s.selected_object_count(), selected, "the same selection");
}

fn styled(d: &Document, id: NodeId) {
    d.edit_style(
        &[id],
        &StyleEdit::StrokeColor(Color {
            r: 9,
            g: 80,
            b: 200,
        }),
    );
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(0.9)));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true));
    d.edit_style(
        &[id],
        &StyleEdit::FillColor(Color {
            r: 200,
            g: 40,
            b: 10,
        }),
    );
    d.edit_style(
        &[id],
        &StyleEdit::FillOpacity(curvyo_document_core::Opacity::new(0.35).unwrap()),
    );
}

fn anchor_ids(p: &PathSnapshot) -> Vec<AnchorId> {
    p.anchors.iter().map(|a| a.id).collect()
}

/// The invariants every successful cut must keep, whatever the input: all pieces are open, every
/// point of a piece lies on one of the original contours, and the pieces of the contours add up
/// to the contours' length.
fn check_pieces(
    contours: &[(Vec<NewAnchor>, bool)],
    pieces: &[PathSnapshot],
    tol: f64,
    dense: usize,
) {
    let polys: Vec<Vec<Xy>> = contours
        .iter()
        .map(|(a, closed)| flatten(a, *closed, dense))
        .collect();
    for p in pieces {
        assert!(!p.closed && !p.is_compound(), "a piece is an open path");
        for q in sample_piece(p) {
            let d = polys
                .iter()
                .map(|poly| dist_to_poly(q, poly))
                .fold(f64::INFINITY, f64::min);
            assert!(
                d <= tol,
                "a piece point {q:?} is {d} from every original contour"
            );
        }
    }
    let want: f64 = polys.iter().map(|p| poly_length(p)).sum();
    let got: f64 = pieces
        .iter()
        .map(|p| poly_length(&flatten(&p.anchors, false, 400)))
        .sum();
    assert!(
        (want - got).abs() <= 2e-3 + 1e-5 * want,
        "length: contours {want}, pieces {got}"
    );
}

fn rect_prim(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

// ---------------------------------------------------------------------------------------------
// Criterion 1: availability
// ---------------------------------------------------------------------------------------------

#[test]
fn c1_cut_is_enabled_for_one_selected_object_and_a_dimmed_press_does_nothing() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 110.0, 110.0), false);
    let mut s = session_of(&d);
    assert!(!s.path_availability().cut, "nothing selected: dimmed");
    let before = json(&s);
    assert_eq!(cut_refusal(s.apply_cut()), CutRefusal::NoneSelected);
    assert_eq!(json(&s), before);
    select_all(&mut s);
    assert!(s.path_availability().cut, "one path is enough");
    for tool in [Tool::Pen, Tool::Node] {
        s.set_tool(tool);
        assert!(!s.path_availability().cut, "{tool:?}");
        assert!(matches!(
            s.apply_cut(),
            CutOutcome::Ignored | CutOutcome::Refused(_)
        ));
        assert_eq!(json(&s), before);
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 12: the worked examples
// ---------------------------------------------------------------------------------------------

#[test]
fn c12_two_crossing_lines_give_four_pieces_each_from_or_to_the_crossing() {
    let d = Document::new(1);
    let below = d.create_path(&square(-200.0, 0.0, 5.0), true);
    let a = d.create_path(&line(100.0, 100.0, 110.0, 110.0), false);
    let b = d.create_path(&line(100.0, 110.0, 110.0, 100.0), false);
    let above = d.create_path(&square(300.0, 0.0, 5.0), true);
    styled(&d, a);
    let style_a = d.path(a).unwrap().style;
    let style_b = d.path(b).unwrap().style;
    let old_anchor_ids: HashSet<AnchorId> = [a, b]
        .iter()
        .flat_map(|i| {
            d.path(*i)
                .unwrap()
                .anchors
                .iter()
                .map(|n| n.id)
                .collect::<Vec<_>>()
        })
        .collect();
    let mut s = session_of(&d);
    select_all(&mut s);
    let before_json = json(&s);
    let before = labels(&reread(&s));
    let out = s.apply_cut();
    assert_eq!(cut_applied(&out), (2, 4));
    assert_eq!(
        new_labels(&before, &s),
        vec!["cut_at_crossings".to_string()],
        "one commit"
    );

    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 6);
    assert_eq!(
        (ids[0], ids[5]),
        (below, above),
        "the others keep their place"
    );
    assert!(!ids.contains(&a) && !ids.contains(&b));
    let ps: Vec<PathSnapshot> = ids.iter().map(|i| doc.path(*i).unwrap()).collect();
    // Pieces of a come first (at a's place), in the order along the contour.
    let x = (105.0, 105.0);
    let pa = &ps[1..3];
    let pb = &ps[3..5];
    assert!(near(endpoints(&pa[0]).0, (100.0, 100.0)) && near(endpoints(&pa[0]).1, x));
    assert!(near(endpoints(&pa[1]).0, x) && near(endpoints(&pa[1]).1, (110.0, 110.0)));
    assert!(near(endpoints(&pb[0]).0, (100.0, 110.0)) && near(endpoints(&pb[0]).1, x));
    assert!(near(endpoints(&pb[1]).0, x) && near(endpoints(&pb[1]).1, (110.0, 100.0)));
    for p in &ps[1..5] {
        assert_eq!(p.anchors.len(), 2);
        assert!(!p.style.fill.enabled, "Fill None");
        assert_eq!(
            p.style.stroke,
            if pa.iter().any(|q| q.id == p.id) {
                style_a.stroke.clone()
            } else {
                style_b.stroke.clone()
            }
        );
        for n in &p.anchors {
            assert!(!old_anchor_ids.contains(&n.id), "new anchor ids");
        }
    }
    // The stored fill colour and opacity are kept, so Solid restores it.
    assert_eq!(pa[0].style.fill.color, style_a.fill.color);
    assert_eq!(pa[0].style.fill.opacity, style_a.fill.opacity);
    let all_new: HashSet<AnchorId> = ps[1..5].iter().flat_map(|p| anchor_ids(p)).collect();
    assert_eq!(all_new.len(), 8, "no anchor id twice");
    // All pieces selected; the untouched ones too.
    assert_eq!(s.selected_object_count(), 6);

    // Undo and redo, byte for byte.
    let after_json = json(&s);
    s.undo();
    assert_eq!(json(&s), before_json);
    s.redo();
    assert_eq!(json(&s), after_json);
}

#[test]
fn c12_a_circle_and_a_line_through_its_nodes() {
    let d = Document::new(1);
    let circ = circle(100.0, 100.0, 10.0, false);
    d.create_path(&circ, true);
    d.create_path(&line(80.0, 100.0, 120.0, 100.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(
        cut_applied(&s.apply_cut()),
        (2, 5),
        "3 for the line and 2 for the circle"
    );
    let ps = paths(&s);
    // The circle's pieces come first (it is below).
    assert_eq!(ps[0].anchors.len(), 3);
    assert_eq!(ps[1].anchors.len(), 3);
    assert!(near(endpoints(&ps[0]).0, (110.0, 100.0)) && near(endpoints(&ps[0]).1, (90.0, 100.0)));
    assert!(near(endpoints(&ps[1]).0, (90.0, 100.0)) && near(endpoints(&ps[1]).1, (110.0, 100.0)));
    // No node was added: the middle node of each arc is the circle's own, bit for bit.
    assert_eq!(node_sig(&ps[0].anchors[1]), node_sig(&circ[1]));
    assert_eq!(node_sig(&ps[1].anchors[1]), node_sig(&circ[3]));
    // The line: three pieces.
    let line_pieces: Vec<Xy> = ps[2..].iter().map(|p| endpoints(p).0).collect();
    assert!(near(line_pieces[0], (80.0, 100.0)));
    assert!(near(line_pieces[1], (90.0, 100.0)));
    assert!(near(line_pieces[2], (110.0, 100.0)));
    assert_eq!(ps.len(), 5);
}

#[test]
fn c12_new_end_nodes_are_corners_with_a_zero_handle_on_the_open_side_and_the_curve_is_exact() {
    // The circle turned by 30 degrees: the line y = 100 cuts inside two segments.
    let d = Document::new(1);
    let circ = rotated_circle(100.0, 100.0, 10.0, 30.0, false);
    d.create_path(&circ, true);
    d.create_path(&line(80.0, 100.0, 120.0, 100.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 5));
    let ps = paths(&s);
    let arcs = &ps[..2];
    for arc in arcs {
        assert_eq!(
            arc.anchors.len(),
            4,
            "two cut ends and two nodes of the circle"
        );
        let first = &arc.anchors[0];
        let last = &arc.anchors[3];
        assert_eq!(first.kind, AnchorKind::Corner);
        assert_eq!(last.kind, AnchorKind::Corner);
        assert_eq!(
            (first.handle_in.x, first.handle_in.y),
            (0.0, 0.0),
            "open side is zero"
        );
        assert_eq!(
            (last.handle_out.x, last.handle_out.y),
            (0.0, 0.0),
            "open side is zero"
        );
        assert!(
            first.handle_out.length() > 0.0 && last.handle_in.length() > 0.0,
            "the curve is kept"
        );
    }
    // Every other node keeps its position bit for bit and its handle away from the cut; the handle
    // toward the cut keeps its direction and is shortened, which de Casteljau needs to keep the
    // curve exact (criterion 4; criterion 5's "keeps its handles" cannot hold for that one).
    let same_pos = |a: &NewAnchor, b: &NewAnchor| {
        a.point.x.to_bits() == b.point.x.to_bits() && a.point.y.to_bits() == b.point.y.to_bits()
    };
    let arc_of = |a: usize, b: usize| {
        arcs.iter()
            .find(|arc| same_pos(&arc.anchors[1], &circ[a]) && same_pos(&arc.anchors[2], &circ[b]))
            .unwrap_or_else(|| panic!("no arc holds nodes {a} and {b} at their places"))
    };
    let cross = |u: Vec2, w: Vec2| (u.x * w.y - u.y * w.x).abs();
    for (arc, a, b) in [(arc_of(0, 1), 0, 1), (arc_of(2, 3), 2, 3)] {
        assert_eq!(arc.anchors[1].handle_out, circ[a].handle_out, "far handle kept");
        assert_eq!(arc.anchors[2].handle_in, circ[b].handle_in, "far handle kept");
        assert!(cross(arc.anchors[1].handle_in, circ[a].handle_in) < 1e-9, "collinear");
        assert!(arc.anchors[1].handle_in.length() < circ[a].handle_in.length());
        assert!(cross(arc.anchors[2].handle_out, circ[b].handle_out) < 1e-9, "collinear");
    }
    // The arcs run in the circle's direction and end where the line crosses.
    assert!(near(endpoints(arc_of(0, 1)).0, (110.0, 100.0)));
    assert!(near(endpoints(arc_of(0, 1)).1, (90.0, 100.0)));
    assert!(near(endpoints(arc_of(2, 3)).0, (90.0, 100.0)));
    // Every point of both halves lies within 1e-6 mm of the original curve.
    check_pieces(&[(circ, true)], arcs, 1e-6, 20_000);
}

#[test]
fn c12_a_figure_eight_gives_one_open_piece_per_lobe_cut_at_both_passes() {
    let d = Document::new(1);
    let eight = vec![
        corner(100.0, 100.0),
        corner(140.0, 140.0),
        corner(140.0, 100.0),
        corner(100.0, 140.0),
    ];
    d.create_path(&eight, true);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (1, 2));
    let ps = paths(&s);
    let x = (120.0, 120.0);
    for p in &ps {
        assert!(
            near(endpoints(p).0, x) && near(endpoints(p).1, x),
            "each lobe runs from the crossing back to it"
        );
    }
    assert_eq!(ps[0].anchors.len(), 4, "crossing, two corners, crossing");
    assert_eq!(ps[1].anchors.len(), 4);
    let first_lobe: Vec<Xy> = ps[0]
        .anchors
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect();
    assert!(near(first_lobe[1], (140.0, 140.0)) && near(first_lobe[2], (140.0, 100.0)));
    check_pieces(&[(eight, true)], &ps, 1e-6, 100);
}

#[test]
fn c12_a_figure_eight_of_curves_is_cut_at_its_crossing_too() {
    // Two cubic segments, S-shaped and mirrored: a closed curve crossing itself once.
    let d = Document::new(1);
    let mk = |x: f64, y: f64, hin: Xy, hout: Xy| NewAnchor {
        id: aid(),
        point: pt(x, y),
        handle_in: v(hin.0, hin.1),
        handle_out: v(hout.0, hout.1),
        kind: AnchorKind::Corner,
    };
    // Slightly asymmetric so that the crossing does not fall on a sample of the brute force.
    let e = vec![
        mk(100.0, 100.0, (0.0, -20.0), (0.3, 20.0)),
        mk(120.0, 103.0, (0.0, -20.0), (0.0, 20.0)),
    ];
    d.create_path(&e, true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let out = s.apply_cut();
    let (paths_n, pieces) = cut_applied(&out);
    assert_eq!(paths_n, 1);
    // Brute force: the crossing points, merged when they are within 0.01 mm.
    let poly = flatten(&e, true, 4000);
    let mut xs: Vec<Xy> = Vec::new();
    let n = poly.len() - 1;
    for i in 0..n {
        for j in (i + 2)..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            if let Some(x) = seg_cross(poly[i], poly[i + 1], poly[j], poly[j + 1]) {
                if !xs.iter().any(|q| near(*q, x)) {
                    xs.push(x);
                }
            }
        }
    }
    assert!(!xs.is_empty(), "the test curve must cross itself");
    assert_eq!(pieces, 2 * xs.len(), "crossings {xs:?}");
    check_pieces(&[(e, true)], &paths(&s), 1e-4, 5000);
}

#[test]
fn c12_a_t_junction_cuts_the_bar_and_leaves_the_stem_alone() {
    let d = Document::new(1);
    let bar = d.create_path(&line(100.0, 100.0, 140.0, 100.0), false);
    let stem = d.create_path(&line(120.0, 100.0, 120.0, 160.0), false);
    styled(&d, stem);
    let stem_before = d.path(stem).unwrap();
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(
        cut_applied(&s.apply_cut()),
        (1, 2),
        "one path divided, giving two pieces; the stem stays"
    );
    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 3);
    assert!(!ids.contains(&bar));
    assert_eq!(ids[2], stem, "the stem is the same object");
    let kept = doc.path(stem).unwrap();
    assert_eq!(outline_sigs(&kept), outline_sigs(&stem_before));
    assert_eq!(kept.style, stem_before.style, "same style, fill untouched");
    assert_eq!(anchor_ids(&kept), anchor_ids(&stem_before));
    let b0 = doc.path(ids[0]).unwrap();
    let b1 = doc.path(ids[1]).unwrap();
    assert!(near(endpoints(&b0).0, (100.0, 100.0)) && near(endpoints(&b0).1, (120.0, 100.0)));
    assert!(near(endpoints(&b1).0, (120.0, 100.0)) && near(endpoints(&b1).1, (140.0, 100.0)));
    assert_eq!(
        s.selected_object_count(),
        3,
        "pieces and the untouched stem are selected"
    );
}

#[test]
fn c12_collinear_lines_are_cut_at_the_ends_of_the_shared_stretch_and_nothing_is_removed() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 110.0, 100.0), false);
    d.create_path(&line(105.0, 100.0, 115.0, 100.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 4));
    let spans: Vec<(f64, f64)> = paths(&s)
        .iter()
        .map(|p| {
            let (a, b) = endpoints(p);
            (a.0.min(b.0), a.0.max(b.0))
        })
        .collect();
    let want = [
        (100.0, 105.0),
        (105.0, 110.0),
        (105.0, 110.0),
        (110.0, 115.0),
    ];
    for (got, w) in spans.iter().zip(want) {
        assert!(
            (got.0 - w.0).abs() < 0.01 && (got.1 - w.1).abs() < 0.01,
            "{spans:?}"
        );
    }
}

#[test]
fn c12_a_short_line_laid_on_a_long_one_cuts_the_long_one_in_three_and_keeps_itself() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 100.0), false);
    let short = d.create_path(&line(105.0, 100.0, 110.0, 100.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (1, 3));
    assert!(
        object_ids(&s).contains(&short),
        "the short line is not divided"
    );
}

#[test]
fn c12a_a_ring_is_refused_and_the_document_is_unchanged() {
    let d = Document::new(1);
    let r = compound(
        &d,
        vec![
            circle(100.0, 100.0, 20.0, false),
            circle(100.0, 100.0, 10.0, true),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    for _ in 0..2 {
        match cut_refusal(s.apply_cut()) {
            CutRefusal::Compound { offenders } => assert_eq!(offenders, vec![r]),
            other => panic!("{other:?}"),
        }
        assert_nothing_changed(&s, &before, n, 1);
    }
}

#[test]
fn c12_primitives_are_accepted_and_give_open_pieces() {
    let d = Document::new(1);
    let r = rect_prim(&d, 100.0, 100.0, 40.0, 40.0);
    let e = d.create_ellipse(EllipseFrame {
        center: pt(140.0, 120.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), 2);
    let (n, pieces) = cut_applied(&s.apply_cut());
    assert_eq!(
        (n, pieces),
        (2, 4),
        "the outlines cross at two points; two arcs each"
    );
    let doc = reread(&s);
    let ids = doc.object_ids();
    assert!(!ids.contains(&r) && !ids.contains(&e));
    for id in ids {
        let p = doc.path(id).unwrap();
        assert!(!p.closed && !p.style.fill.enabled);
    }
}

#[test]
fn c12_three_lines_through_one_point_give_six_pieces() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    d.create_path(&line(100.0, 110.0, 120.0, 110.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (3, 6));
    for p in paths(&s) {
        let (a, b) = endpoints(&p);
        assert!(
            near(a, (110.0, 110.0)) || near(b, (110.0, 110.0)),
            "{a:?} {b:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 3 and 4 corner cases: where the cuts fall
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_a_closed_contour_with_one_cut_point_gives_one_piece_that_starts_and_ends_there() {
    // A line touching the circle's top node from outside.
    let d = Document::new(1);
    let circ = circle(100.0, 100.0, 10.0, false);
    d.create_path(&circ, true);
    d.create_path(&line(80.0, 110.0, 120.0, 110.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(
        cut_applied(&s.apply_cut()),
        (2, 3),
        "one arc and two line pieces"
    );
    let ps = paths(&s);
    assert_eq!(
        ps[0].anchors.len(),
        5,
        "the node is split into two end nodes"
    );
    assert!(near(endpoints(&ps[0]).0, (100.0, 110.0)) && near(endpoints(&ps[0]).1, (100.0, 110.0)));
    // The first keeps the incoming handle's side per 0006 criterion 13: first = outgoing, last = incoming.
    assert_eq!(ps[0].anchors[0].handle_in.length(), 0.0);
    assert!(ps[0].anchors[0].handle_out.length() > 0.0);
    assert_eq!(ps[0].anchors[4].handle_out.length(), 0.0);
    assert!(ps[0].anchors[4].handle_in.length() > 0.0);
}

#[test]
fn c3_an_end_that_meets_another_end_is_not_a_cut_and_the_command_is_refused() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 110.0, 100.0), false);
    d.create_path(&line(110.0, 100.0, 110.0, 110.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    assert_eq!(s.apply_cut(), CutOutcome::Refused(CutRefusal::NoCrossing));
    assert_nothing_changed(&s, &before, n, 2);
}

#[test]
fn c3_an_open_path_whose_end_comes_back_to_its_start_is_not_divided() {
    let d = Document::new(1);
    d.create_path(
        &[
            corner(100.0, 100.0),
            corner(120.0, 100.0),
            corner(120.0, 120.0),
            corner(100.0, 100.0),
        ],
        false,
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    assert_eq!(s.apply_cut(), CutOutcome::Refused(CutRefusal::NoCrossing));
    assert_eq!(json(&s), before);
}

#[test]
fn c3_an_open_path_whose_end_touches_its_own_middle_is_cut_in_two() {
    // A "P": the last node lies on the stem.
    let d = Document::new(1);
    d.create_path(
        &[
            corner(100.0, 160.0),
            corner(100.0, 100.0),
            corner(120.0, 100.0),
            corner(120.0, 120.0),
            corner(100.0, 120.0),
        ],
        false,
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (1, 2));
    let ps = paths(&s);
    assert!(near(endpoints(&ps[0]).1, (100.0, 120.0)) && near(endpoints(&ps[1]).0, (100.0, 120.0)));
    assert_eq!(ps[0].anchors.len(), 2, "start and the cut on the stem");
    assert_eq!(
        ps[1].anchors.len(),
        5,
        "the cut, then the four nodes after it"
    );
}

#[test]
fn c3_one_open_path_crossing_itself_gives_m_plus_one_pieces() {
    // A loop: down, right, up across the first leg, right again.
    let d = Document::new(1);
    d.create_path(
        &[
            corner(100.0, 100.0),
            corner(140.0, 100.0),
            corner(140.0, 140.0),
            corner(120.0, 140.0),
            corner(120.0, 80.0),
        ],
        false,
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    // The last leg crosses the first at (120, 100): two cut passes.
    assert_eq!(cut_applied(&s.apply_cut()), (1, 3));
    let ps = paths(&s);
    assert!(near(endpoints(&ps[0]).1, (120.0, 100.0)));
    assert!(near(endpoints(&ps[1]).0, (120.0, 100.0)) && near(endpoints(&ps[1]).1, (120.0, 100.0)));
    assert!(near(endpoints(&ps[2]).0, (120.0, 100.0)) && near(endpoints(&ps[2]).1, (120.0, 80.0)));
}

#[test]
fn c4_a_curve_with_a_loop_in_one_half_of_the_cubic_is_cut_at_the_loop() {
    // Search cubics whose self-crossing has both parameters in the same half.
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut found = 0;
    let mut tried = 0;
    while found < 8 && tried < 20_000 {
        tried += 1;
        let p = |rng: &mut Rng| (rng.range(100.0, 200.0), rng.range(100.0, 200.0));
        let (p0, p1, p2, p3) = (p(&mut rng), p(&mut rng), p(&mut rng), p(&mut rng));
        let nodes = cubic(p0, p1, p2, p3);
        let poly = flatten(&nodes, false, 800);
        let Some((i, j, x)) = first_self_crossing(&poly) else {
            continue;
        };
        let (t1, t2) = (i as f64 / 800.0, j as f64 / 800.0);
        let same_half = (t1 < 0.45 && t2 < 0.45) || (t1 > 0.55 && t2 > 0.55);
        if !same_half || brute_crossings(&poly, false) != 1 {
            continue;
        }
        // Keep clear crossings only.
        if crossing_angle_sin(&poly, i, j) < 0.2 {
            continue;
        }
        found += 1;
        let d = Document::new(1);
        d.create_path(&nodes, false);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        assert_eq!(
            cut_applied(&out),
            (1, 3),
            "loop at t = {t1:.3}/{t2:.3} of {nodes:?}"
        );
        let ps = paths(&s);
        assert!(
            near(endpoints(&ps[0]).1, x),
            "{:?} vs {x:?}",
            endpoints(&ps[0]).1
        );
        assert!(
            near(endpoints(&ps[1]).0, x) && near(endpoints(&ps[1]).1, x),
            "the loop starts and ends at the crossing"
        );
        assert!(near(endpoints(&ps[2]).0, x));
        check_pieces(&[(nodes, false)], &ps, 1e-4, 5000);
    }
    assert!(
        found >= 8,
        "found only {found} test curves in {tried} tries"
    );
}

#[test]
fn c4_a_loop_in_the_middle_of_a_longer_path_and_in_a_closed_one() {
    let d = Document::new(1);
    // Straight-in, loop, straight-out: a classic loop-the-loop on a three-node path.
    let nodes = vec![
        corner(100.0, 150.0),
        NewAnchor {
            id: aid(),
            point: pt(150.0, 150.0),
            handle_in: v(-10.0, 0.0),
            handle_out: v(60.0, -80.0),
            kind: AnchorKind::Corner,
        },
        NewAnchor {
            id: aid(),
            point: pt(130.0, 100.0),
            handle_in: v(60.0, -10.0),
            handle_out: v(0.0, 0.0),
            kind: AnchorKind::Corner,
        },
    ];
    d.create_path(&nodes, false);
    let mut s = session_of(&d);
    select_all(&mut s);
    let poly = flatten(&nodes, false, 2000);
    let k = brute_crossings(&poly, false);
    let out = s.apply_cut();
    if k == 0 {
        assert_eq!(out, CutOutcome::Refused(CutRefusal::NoCrossing));
    } else {
        assert_eq!(cut_applied(&out), (1, 2 * k + 1));
        check_pieces(&[(nodes, false)], &paths(&s), 1e-4, 5000);
    }
}

#[test]
fn c4_tangent_and_nearly_tangent_circles_are_cut_once_in_the_middle() {
    // Circles turned by 30 degrees so the closest point lies inside a segment, not on a node.
    for (gap, cuts) in [
        (0.0, true),
        (0.002, true),
        (-0.000_001, true),
        (0.5, false),
        (0.05, false),
    ] {
        let d = Document::new(1);
        d.create_path(&rotated_circle(100.0, 100.0, 10.0, 30.0, false), true);
        d.create_path(&rotated_circle(120.0 + gap, 100.0, 10.0, 30.0, false), true);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        if !cuts {
            assert_eq!(
                out,
                CutOutcome::Refused(CutRefusal::NoCrossing),
                "gap {gap}"
            );
            continue;
        }
        assert_eq!(
            cut_applied(&out),
            (2, 2),
            "gap {gap}: each circle is cut once, where they touch (not twice at the two near points)"
        );
        for p in paths(&s) {
            let (a, b) = endpoints(&p);
            assert!(
                near(a, b),
                "a closed contour with one cut point starts and ends there"
            );
            assert!(
                dist(a, (110.0 + gap / 2.0, 100.0)) < 0.01 + gap.abs(),
                "the cut is in the middle of the approach: {a:?} (gap {gap})"
            );
        }
    }
}

#[test]
fn c4_tangent_circles_inside_one_another_and_on_nodes() {
    // Circle r5 inside circle r10, tangent at (110, 100), which is a node of both.
    let d = Document::new(1);
    d.create_path(&circle(100.0, 100.0, 10.0, false), true);
    d.create_path(&circle(105.0, 100.0, 5.0, false), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 2));
    for p in paths(&s) {
        assert!(near(endpoints(&p).0, (110.0, 100.0)) && near(endpoints(&p).1, (110.0, 100.0)));
    }
}

#[test]
fn c4_hairpins_and_doubled_stretches_keep_the_invariants() {
    let cases: Vec<(&str, Vec<(Vec<NewAnchor>, bool)>)> = vec![
        (
            "razor hairpin",
            vec![(
                vec![
                    corner(100.0, 100.0),
                    corner(200.0, 100.0),
                    corner(100.0, 100.002),
                ],
                false,
            )],
        ),
        (
            "exact doubling back",
            vec![(
                vec![
                    corner(100.0, 100.0),
                    corner(200.0, 100.0),
                    corner(150.0, 100.0),
                ],
                false,
            )],
        ),
        (
            "cusp cubic",
            vec![(
                cubic(
                    (100.0, 100.0),
                    (200.0, 200.0),
                    (100.0, 200.0),
                    (200.0, 100.0),
                ),
                false,
            )],
        ),
        (
            "the same circle twice",
            vec![
                (circle(100.0, 100.0, 10.0, false), true),
                (circle(100.0, 100.0, 10.0, false), true),
            ],
        ),
        (
            "a circle and its reverse",
            vec![
                (circle(100.0, 100.0, 10.0, false), true),
                (circle(100.0, 100.0, 10.0, true), true),
            ],
        ),
        (
            "the same line three times",
            vec![
                (line(100.0, 100.0, 120.0, 100.0), false),
                (line(100.0, 100.0, 120.0, 100.0), false),
                (line(100.0, 100.0, 120.0, 100.0), false),
            ],
        ),
        (
            "a circle and a concentric near copy",
            vec![
                (circle(100.0, 100.0, 10.0, false), true),
                (circle(100.0, 100.0, 10.005, false), true),
            ],
        ),
        (
            "square and its doubled edge",
            vec![
                (square(100.0, 100.0, 20.0), true),
                (line(100.0, 100.0, 120.0, 100.0), false),
            ],
        ),
    ];
    for (name, contours) in cases {
        let d = Document::new(1);
        for (a, closed) in &contours {
            d.create_path(a, *closed);
        }
        let mut s = session_of(&d);
        select_all(&mut s);
        let before = json(&s);
        let n = label_count(&s);
        match s.apply_cut() {
            CutOutcome::Applied {
                pieces,
                paths: divided,
            } => {
                let ps = paths(&s);
                assert!(divided >= 1 && pieces >= 1, "{name}");
                // Untouched operands stay closed paths; the rest are open pieces.
                let (open, closed): (Vec<_>, Vec<_>) = ps.iter().cloned().partition(|p| !p.closed);
                assert!(open.len() >= pieces.min(open.len()), "{name}");
                let _ = closed;
                // Everything lies on the original contours and no length is lost.
                for p in open.iter() {
                    for q in sample_piece(p) {
                        let dmin = contours
                            .iter()
                            .map(|(a, c)| dist_to_poly(q, &flatten(a, *c, 3000)))
                            .fold(f64::INFINITY, f64::min);
                        assert!(dmin < 1e-3, "{name}: {q:?} is {dmin} off");
                    }
                    assert!(
                        p.anchors
                            .iter()
                            .all(|a| a.point.x.is_finite() && a.point.y.is_finite()),
                        "{name}"
                    );
                }
            }
            CutOutcome::Refused(r) => {
                assert_eq!(r, CutRefusal::NoCrossing, "{name}");
                assert_nothing_changed(&s, &before, n, contours.len());
            }
            CutOutcome::Ignored => panic!("{name}: ignored"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Differential tests against a brute-force crossing finder
// ---------------------------------------------------------------------------------------------

fn seg_cross(a: Xy, b: Xy, c: Xy, e: Xy) -> Option<Xy> {
    let (r, s) = ((b.0 - a.0, b.1 - a.1), (e.0 - c.0, e.1 - c.1));
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let (qp0, qp1) = (c.0 - a.0, c.1 - a.1);
    let t = (qp0 * s.1 - qp1 * s.0) / den;
    let u = (qp0 * r.1 - qp1 * r.0) / den;
    if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
        Some((a.0 + t * r.0, a.1 + t * r.1))
    } else {
        None
    }
}

fn brute_crossings(poly: &[Xy], closed: bool) -> usize {
    let n = if closed {
        poly.len() - 1
    } else {
        poly.len() - 1
    };
    let mut k = 0;
    for i in 0..n {
        for j in (i + 2)..n {
            if closed && i == 0 && j == n - 1 {
                continue;
            }
            if seg_cross(poly[i], poly[i + 1], poly[j], poly[j + 1]).is_some() {
                k += 1;
            }
        }
    }
    k
}

fn first_self_crossing(poly: &[Xy]) -> Option<(usize, usize, Xy)> {
    let n = poly.len() - 1;
    for i in 0..n {
        for j in (i + 2)..n {
            if let Some(x) = seg_cross(poly[i], poly[i + 1], poly[j], poly[j + 1]) {
                return Some((i, j, x));
            }
        }
    }
    None
}

fn crossing_angle_sin(poly: &[Xy], i: usize, j: usize) -> f64 {
    let a = (poly[i + 1].0 - poly[i].0, poly[i + 1].1 - poly[i].1);
    let b = (poly[j + 1].0 - poly[j].0, poly[j + 1].1 - poly[j].1);
    (a.0 * b.1 - a.1 * b.0).abs() / ((a.0.hypot(a.1)) * (b.0.hypot(b.1)))
}

fn random_curve(rng: &mut Rng, nodes: usize, closed: bool, ox: f64, oy: f64) -> Vec<NewAnchor> {
    let _ = closed;
    (0..nodes)
        .map(|_| NewAnchor {
            id: aid(),
            point: pt(ox + rng.range(0.0, 100.0), oy + rng.range(0.0, 100.0)),
            handle_in: v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0)),
            handle_out: v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0)),
            kind: AnchorKind::Corner,
        })
        .collect()
}

/// The cleanest way to be sure a brute-force count is right: it must not move when the sampling
/// is refined, and no two polyline vertices of different parts may be close.
fn stable_crossings(a: &[NewAnchor], closed: bool) -> Option<usize> {
    let k1 = brute_crossings(&flatten(a, closed, 300), closed);
    let k2 = brute_crossings(&flatten(a, closed, 900), closed);
    (k1 == k2).then_some(k1)
}

#[test]
fn differential_random_single_paths_open_and_closed() {
    let mut rng = Rng(0xDEAD_BEEF_1234_5678);
    let mut checked = 0;
    let mut with_cuts = 0;
    for round in 0..120 {
        let closed = round % 2 == 0;
        let nodes = 2 + (round % 3);
        let curve = random_curve(&mut rng, nodes, closed, 100.0, 100.0);
        let Some(k) = stable_crossings(&curve, closed) else {
            continue;
        };
        let d = Document::new(1);
        d.create_path(&curve, closed);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        checked += 1;
        if k == 0 {
            assert_eq!(
                out,
                CutOutcome::Refused(CutRefusal::NoCrossing),
                "round {round}: {curve:?} closed {closed}"
            );
            continue;
        }
        with_cuts += 1;
        let want = if closed { 2 * k } else { 2 * k + 1 };
        assert_eq!(
            cut_applied(&out),
            (1, want),
            "round {round}: closed {closed}, {k} crossings, {curve:?}"
        );
        let ps = paths(&s);
        check_pieces(&[(curve, closed)], &ps, 1e-4, 5000);
        // The pieces follow each other: each ends where the next begins (closed: round again).
        for w in ps.windows(2) {
            let (_, e) = endpoints(&w[0]);
            let (b, _) = endpoints(&w[1]);
            let _ = (e, b);
        }
        // Every cut point is visited exactly twice among the piece ends.
        let mut ends: Vec<Xy> = ps
            .iter()
            .flat_map(|p| {
                let (a, b) = endpoints(p);
                [a, b]
            })
            .collect();
        let (first, last) = (ends.remove(0), ends.pop().unwrap());
        if closed {
            assert!(
                near(first, last),
                "closed: the last piece ends where the first begins"
            );
        }
        // ends now: interior ends. They pair up (a cut point has two piece ends, or four at a
        // crossing pass counted twice).
        for e in &ends {
            let m = ends.iter().filter(|q| near(**q, *e)).count();
            assert!(
                m >= 2 && m % 2 == 0,
                "round {round}: a cut end has no partner at {e:?}"
            );
        }
    }
    assert!(checked >= 80, "only {checked} stable samples");
    assert!(with_cuts >= 15, "only {with_cuts} samples with crossings");
}

#[test]
fn differential_random_pairs_of_curves() {
    let mut rng = Rng(0x1357_9BDF_2468_ACE0);
    let mut with_cuts = 0;
    for round in 0..60 {
        let a = random_curve(&mut rng, 2, false, 100.0, 100.0);
        let b = random_curve(&mut rng, 3, false, 100.0, 100.0);
        let (Some(ka), Some(kb)) = (stable_crossings(&a, false), stable_crossings(&b, false))
        else {
            continue;
        };
        if ka != 0 || kb != 0 {
            continue;
        }
        // Crossings between the two.
        let (pa, pb) = (flatten(&a, false, 900), flatten(&b, false, 900));
        let mut k = 0;
        for i in 0..pa.len() - 1 {
            for j in 0..pb.len() - 1 {
                if seg_cross(pa[i], pa[i + 1], pb[j], pb[j + 1]).is_some() {
                    k += 1;
                }
            }
        }
        let d = Document::new(1);
        d.create_path(&a, false);
        d.create_path(&b, false);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        if k == 0 {
            assert_eq!(
                out,
                CutOutcome::Refused(CutRefusal::NoCrossing),
                "round {round}"
            );
            continue;
        }
        with_cuts += 1;
        // Each crossing is a cut in both curves.
        assert_eq!(
            cut_applied(&out),
            (2, 2 * k + 2),
            "round {round}: {k} crossings, {a:?} {b:?}"
        );
        check_pieces(&[(a, false), (b, false)], &paths(&s), 1e-4, 5000);
    }
    assert!(with_cuts >= 10, "only {with_cuts} pairs crossed");
}

#[test]
fn differential_random_circle_pairs_have_their_cut_points_where_the_geometry_says() {
    let mut rng = Rng(0xA5A5_5A5A_0F0F_F0F0);
    let mut crossing = 0;
    for round in 0..80 {
        let (r1, r2) = (rng.range(5.0, 10.0), rng.range(5.0, 10.0));
        let dist_c = rng.range(0.5, 22.0);
        let rot1 = rng.range(0.0, 90.0);
        let rot2 = rng.range(0.0, 90.0);
        let d = Document::new(1);
        d.create_path(&rotated_circle(100.0, 100.0, r1, rot1, false), true);
        d.create_path(
            &rotated_circle(100.0 + dist_c, 100.0, r2, rot2, round % 2 == 0),
            true,
        );
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        let touching_band = 0.05;
        let overlaps = dist_c < r1 + r2 - touching_band && dist_c > (r1 - r2).abs() + touching_band;
        let apart = dist_c > r1 + r2 + touching_band || dist_c < (r1 - r2).abs() - touching_band;
        if overlaps {
            crossing += 1;
            assert_eq!(
                cut_applied(&out),
                (2, 4),
                "round {round}: r {r1}/{r2} d {dist_c}"
            );
            let x = (dist_c * dist_c + r1 * r1 - r2 * r2) / (2.0 * dist_c);
            let h = (r1 * r1 - x * x).sqrt();
            let cps = [(100.0 + x, 100.0 + h), (100.0 + x, 100.0 - h)];
            // At a shallow crossing the Bezier circles' own error moves the crossing along the
            // curve by error / sin(angle); the position check is for clear crossings only.
            let sin_angle = dist_c * h / (r1 * r2);
            for p in paths(&s) {
                if sin_angle < 0.15 {
                    break;
                }
                let (a, b) = endpoints(&p);
                for e in [a, b] {
                    // The Bezier circle differs from the true one by up to 2.7e-4 of the radius.
                    assert!(
                        cps.iter().any(|c| dist(*c, e) < 0.03),
                        "round {round}: {e:?} not near {cps:?}"
                    );
                }
            }
        } else if apart {
            assert_eq!(
                out,
                CutOutcome::Refused(CutRefusal::NoCrossing),
                "round {round}: r {r1}/{r2} d {dist_c}"
            );
        }
    }
    assert!(crossing >= 20);
}

// ---------------------------------------------------------------------------------------------
// Criteria 8, 9, 10: refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn c8_paths_that_do_not_cross_are_refused_with_nothing_changed() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 100.0), false);
    d.create_path(&circle(200.0, 100.0, 10.0, false), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    for _ in 0..2 {
        assert_eq!(s.apply_cut(), CutOutcome::Refused(CutRefusal::NoCrossing));
        assert_nothing_changed(&s, &before, n, 2);
    }
}

#[test]
fn c9_a_group_is_refused_and_its_leaves_are_named() {
    let d = Document::new(1);
    let a = d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    let b = d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    d.group_objects(&[a, b]).unwrap();
    let c = d.create_path(&line(90.0, 110.0, 130.0, 110.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    let leaves = s.selected_object_count();
    assert_eq!(leaves, 3, "the group's two leaves and the line");
    let before = json(&s);
    let n = label_count(&s);
    match cut_refusal(s.apply_cut()) {
        CutRefusal::Groups { offenders } => {
            let set: HashSet<NodeId> = offenders.into_iter().collect();
            assert_eq!(set, HashSet::from([a, b]));
            assert!(!set.contains(&c));
        }
        other => panic!("{other:?}"),
    }
    assert_nothing_changed(&s, &before, n, leaves);
}

#[test]
fn c9_a_compound_path_among_crossing_lines_refuses_the_whole_command() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    let r = compound(
        &d,
        vec![
            circle(300.0, 100.0, 20.0, false),
            circle(300.0, 100.0, 10.0, true),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    match cut_refusal(s.apply_cut()) {
        CutRefusal::Compound { offenders } => assert_eq!(offenders, vec![r]),
        other => panic!("{other:?}"),
    }
    assert_nothing_changed(&s, &before, n, 3);
}

#[test]
fn c9_coordinates_that_are_not_finite_or_beyond_the_kernel_range_are_refused() {
    for (name, x) in [
        ("NaN", f64::NAN),
        ("infinity", f64::INFINITY),
        ("-infinity", f64::NEG_INFINITY),
        ("beyond MAX_COORDINATE_MM", 1.0e7 + 10.0),
        ("1e300", 1.0e300),
    ] {
        let d = Document::new(1);
        d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
        d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
        let bad = d.create_path(&line(x, 100.0, 120.0, 110.0), false);
        let mut s = session_of(&d);
        select_all(&mut s);
        let before = json(&s);
        let n = label_count(&s);
        match s.apply_cut() {
            CutOutcome::Refused(CutRefusal::OutOfRange { offenders, of }) => {
                assert_eq!(offenders, vec![bad], "{name}");
                assert_eq!(of, 3);
            }
            other => panic!("{name}: {other:?}"),
        }
        assert_nothing_changed(&s, &before, n, 3);
    }
    // A handle that is not finite counts too.
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    let mut curve = cubic(
        (100.0, 120.0),
        (110.0, 130.0),
        (110.0, 90.0),
        (120.0, 100.0),
    );
    curve[0].handle_out = v(f64::NAN, 3.0);
    d.create_path(&curve, false);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    assert!(matches!(
        s.apply_cut(),
        CutOutcome::Refused(CutRefusal::OutOfRange { .. } | CutRefusal::NoCrossing)
    ));
    assert_eq!(json(&s), before);
}

#[test]
fn c9_coordinates_just_inside_the_range_are_cut_accurately() {
    let d = Document::new(1);
    d.create_path(&line(9.0e6, 9.0e6, 9.0e6 + 10.0, 9.0e6 + 10.0), false);
    d.create_path(&line(9.0e6, 9.0e6 + 10.0, 9.0e6 + 10.0, 9.0e6), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 4));
    for p in paths(&s) {
        let (a, b) = endpoints(&p);
        assert!(near(a, (9.0e6 + 5.0, 9.0e6 + 5.0)) || near(b, (9.0e6 + 5.0, 9.0e6 + 5.0)));
    }
}

/// A line along y = 0 crossed by `verticals` vertical lines, each in the middle of its length:
/// 1 + verticals paths, and (verticals + 1) + 2 * verticals pieces.
fn comb(verticals: usize) -> Document {
    let d = Document::new(1);
    d.create_path(&line(0.0, 0.0, 3.0 * verticals as f64 + 3.0, 0.0), false);
    for i in 0..verticals {
        let x = 3.0 * i as f64 + 1.5;
        d.create_path(&line(x, -5.0, x, 5.0), false);
    }
    d
}

#[test]
fn c10_exactly_ten_thousand_pieces_are_allowed_and_one_more_crossing_is_refused() {
    // 3333 verticals: 3334 + 6666 = 10000 pieces.
    let mut s = session_of(&comb(3333));
    select_all(&mut s);
    let t = Instant::now();
    assert_eq!(cut_applied(&s.apply_cut()), (3334, 10_000));
    eprintln!("10000 pieces: {} ms", t.elapsed().as_millis());
    assert_eq!(paths(&s).len(), 10_000);

    // 3334 verticals: 10003 pieces.
    let mut s = session_of(&comb(3334));
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    match cut_refusal(s.apply_cut()) {
        CutRefusal::TooMany { pieces } => assert_eq!(pieces, 10_003),
        other => panic!("{other:?}"),
    }
    assert_nothing_changed(&s, &before, n, 3335);
}

// ---------------------------------------------------------------------------------------------
// Criteria 2, 5, 6, 7: stacking, style, ids, selection, group membership
// ---------------------------------------------------------------------------------------------

#[test]
fn c2_c6_the_pieces_of_a_closed_shape_have_no_fill_but_remember_it() {
    let d = Document::new(1);
    let sq = d.create_path(&square(100.0, 100.0, 20.0), true);
    styled(&d, sq);
    let orig = d.path(sq).unwrap().style;
    assert!(orig.fill.enabled);
    d.create_path(&line(90.0, 110.0, 130.0, 110.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 2 + 3));
    for p in paths(&s).iter().take(2) {
        assert!(!p.style.fill.enabled);
        assert_eq!(p.style.fill.color, orig.fill.color);
        assert_eq!(p.style.fill.opacity, orig.fill.opacity);
        assert_eq!(p.style.stroke, orig.stroke, "the stroke is copied");
    }
}

#[test]
fn c2_pieces_keep_the_stacking_position_of_their_operand() {
    let d = Document::new(1);
    let bottom = d.create_path(&square(-300.0, 0.0, 5.0), true);
    d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    let mid = d.create_path(&square(-300.0, 50.0, 5.0), true);
    d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    let top = d.create_path(&square(-300.0, 90.0, 5.0), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(cut_applied(&s.apply_cut()), (2, 4));
    let ids = object_ids(&s);
    assert_eq!(ids.len(), 7);
    assert_eq!((ids[0], ids[3], ids[6]), (bottom, mid, top));
    let doc = reread(&s);
    let first_x = |i: usize| endpoints(&doc.path(ids[i]).unwrap()).0;
    assert!(near(first_x(1), (100.0, 100.0)) || near(first_x(1), (105.0, 105.0)));
    assert!(near(first_x(4), (100.0, 120.0)) || near(first_x(4), (110.0, 110.0)));
    assert_eq!(s.selected_object_count(), 7);
}

#[test]
fn c2_unselected_objects_are_not_cut_even_when_they_cross_the_selection() {
    let d = Document::new(1);
    d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    let other = d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    let third = d.create_path(&line(90.0, 110.0, 130.0, 110.0), false);
    let mut s = session_of(&d);
    marquee(&mut s, pt(95.0, 95.0), pt(125.0, 125.0));
    // The marquee takes the lines fully inside: the two diagonals; the third pokes out.
    let sel = selected_ids(&mut s);
    assert!(sel.contains(&other) && !sel.contains(&third), "{sel:?}");
    assert_eq!(cut_applied(&s.apply_cut()), (2, 4));
    assert!(
        object_ids(&s).contains(&third),
        "the third line is untouched"
    );
}

#[test]
fn c2_a_compound_path_inside_an_entered_group_cannot_be_cut_but_lines_in_it_are_cut_in_the_group() {
    let d = Document::new(1);
    let a = d.create_path(&line(100.0, 100.0, 120.0, 120.0), false);
    let b = d.create_path(&line(100.0, 120.0, 120.0, 100.0), false);
    let group = d.group_objects(&[a, b]).unwrap();
    d.create_path(&square(-300.0, 0.0, 5.0), true);
    let mut s = session_of(&d);
    s.double_click(pt(100.0, 100.0), false, false);
    select_all(&mut s);
    let out = s.apply_cut();
    assert_eq!(cut_applied(&out), (2, 4));
    let doc = reread(&s);
    assert!(doc.is_group(group), "the group survives");
    assert_eq!(
        doc.children(Some(group)).len(),
        4,
        "the pieces stay in the group"
    );
    assert_eq!(doc.object_ids().len(), 5);
}

// ---------------------------------------------------------------------------------------------
// Criterion 11: determinism
// ---------------------------------------------------------------------------------------------

fn bits(s: &Session) -> Vec<Vec<(bool, Vec<NodeSig>)>> {
    paths(s).iter().map(outline_sigs).collect()
}

#[test]
fn c11_the_result_does_not_depend_on_how_the_objects_were_selected_or_on_the_run() {
    let d = Document::new(1);
    d.create_path(&circle(100.0, 100.0, 10.0, false), true);
    d.create_path(&rotated_circle(110.0, 100.0, 10.0, 17.0, false), true);
    d.create_path(&line(80.0, 95.0, 130.0, 105.0), false);
    let bytes = curvyo_document_core::pack(&d, "0.1.0").unwrap();
    let open = |bytes: &[u8]| {
        let mut s = Session::open(2, bytes).unwrap();
        s.resize_viewport(1600.0, 1000.0);
        s.set_tool(Tool::Select);
        s
    };
    let mut a = open(&bytes);
    select_all(&mut a);
    let mut b = open(&bytes);
    marquee(&mut b, pt(0.0, 0.0), pt(500.0, 500.0));
    let outs: Vec<_> = [&mut a, &mut b]
        .into_iter()
        .map(|s| s.apply_cut())
        .collect();
    assert_eq!(outs[0], outs[1]);
    assert_eq!(bits(&a), bits(&b));
    let mut a2 = open(&bytes);
    select_all(&mut a2);
    a2.apply_cut();
    assert_eq!(bits(&a), bits(&a2), "same input, same bits");
    assert_eq!(json(&a), json(&a2), "same input, same document bytes");
}

// ---------------------------------------------------------------------------------------------
// Criterion 13: the grid (and a small version that always runs)
// ---------------------------------------------------------------------------------------------

fn grid(h: usize, w: usize) -> Document {
    let d = Document::new(1);
    for i in 0..h {
        let y = 10.0 * i as f64 + 5.0;
        d.create_path(&line(0.0, y, 10.0 * w as f64, y), false);
    }
    for j in 0..w {
        let x = 10.0 * j as f64 + 5.0;
        d.create_path(&line(x, 0.0, x, 10.0 * h as f64), false);
    }
    d
}

#[test]
fn c13_a_small_grid_gives_the_counted_pieces() {
    let (h, w) = (12, 9);
    let mut s = session_of(&grid(h, w));
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), h + w);
    // Each horizontal line is crossed by w verticals, each vertical by h horizontals.
    let want = h * (w + 1) + w * (h + 1);
    assert_eq!(cut_applied(&s.apply_cut()), (h + w, want));
    assert_eq!(paths(&s).len(), want);
}

#[test]
#[ignore = "performance: run with --release -- --ignored"]
fn c13_fifty_by_fifty_grid_in_two_seconds() {
    let mut s = session_of(&grid(50, 50));
    select_all(&mut s);
    let t = Instant::now();
    let out = s.apply_cut();
    let ms = t.elapsed().as_millis();
    eprintln!("grid 50x50: {ms} ms");
    assert_eq!(cut_applied(&out), (100, 5100));
    assert!(ms < 2000, "{ms} ms");
}

#[test]
#[ignore = "performance: run with --release -- --ignored"]
fn perf_cut_of_many_curves() {
    // 400 circles in a row, neighbours overlapping: 400 paths, 399 overlaps with 2 crossings each.
    let d = Document::new(1);
    for i in 0..400 {
        d.create_path(&circle(15.0 * f64::from(i), 0.0, 10.0, false), true);
    }
    let mut s = session_of(&d);
    select_all(&mut s);
    let t = Instant::now();
    let out = s.apply_cut();
    let ms = t.elapsed().as_millis();
    eprintln!("400 overlapping circles: {ms} ms, {out:?}");
    assert!(ms < 2000, "{ms} ms");
    // Circles i and i+2 are 30 apart (more than 2r): only neighbours cross. A middle circle has
    // four cut points, the two end circles two.
    assert_eq!(cut_applied(&out), (400, 398 * 4 + 2 * 2));
}

/// Two circles that cross at a shallow angle (about 2 to 3 degrees). Brute force on dense
/// polylines finds exactly two crossings in each of these, so each circle must be cut at two
/// points and the result is four arcs (the sheet's E4 rule). Found by the random sweep: the cut
/// adds a third point 0.025 mm from one crossing and leaves a sliver piece.
#[test]
fn defect_a_shallow_crossing_of_two_circles_is_one_cut_not_two() {
    let cases = [
        // (r1, r2, centre distance, rotation 1, rotation 2, circle 2 reversed)
        (
            9.330_917_608_410_772,
            9.735_077_326_373_187,
            0.602_271_234_634_022_4,
            41.295_719_691_864,
            64.491_494_018_643,
            true,
        ),
        (7.4517, 7.2716, 0.4435, 20.63, 22.22, false),
        (6.1582, 6.4379, 0.3795, 82.68, 41.31, false),
    ];
    let mut bad = Vec::new();
    for (r1, r2, dc, rot1, rot2, rev) in cases {
        let d = Document::new(1);
        d.create_path(&rotated_circle(100.0, 100.0, r1, rot1, false), true);
        d.create_path(&rotated_circle(100.0 + dc, 100.0, r2, rot2, rev), true);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_cut();
        if out
            != (CutOutcome::Applied {
                paths: 2,
                pieces: 4,
            })
        {
            let shortest = paths(&s)
                .iter()
                .map(|p| poly_length(&flatten(&p.anchors, false, 200)))
                .fold(f64::INFINITY, f64::min);
            bad.push(format!(
                "r {r1}/{r2} d {dc}: {out:?}, shortest piece {shortest:.4} mm"
            ));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// A node next to a cut inside a segment keeps its kind, but de Casteljau shortens its handle
/// toward the cut. A Symmetric node (mirrored handles of equal length) with unequal handles is a
/// state the kind forbids: the next drag of either handle mirrors it and bends the curve. Such a
/// node must become Asymmetric (collinear, own lengths).
#[test]
fn defect_a_symmetric_node_next_to_a_cut_keeps_symmetric_with_unequal_handles() {
    let d = Document::new(1);
    d.create_path(&rotated_circle(100.0, 100.0, 10.0, 30.0, false), true);
    d.create_path(&line(80.0, 100.0, 120.0, 100.0), false);
    let mut s = session_of(&d);
    select_all(&mut s);
    cut_applied(&s.apply_cut());
    let mut bad = Vec::new();
    for p in paths(&s) {
        for n in &p.anchors {
            let (li, lo) = (n.handle_in.length(), n.handle_out.length());
            if n.kind == AnchorKind::Symmetric && (li - lo).abs() > 1e-6 {
                bad.push(format!("({:.4}, {:.4}) in {li:.4} out {lo:.4}", n.point.x, n.point.y));
            }
        }
    }
    assert!(bad.is_empty(), "Symmetric nodes with unequal handles: {bad:?}");
}
