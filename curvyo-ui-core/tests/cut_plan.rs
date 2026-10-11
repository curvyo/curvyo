//! The pure part of Cut (`specs/0036-cut-at-crossings` criteria 2 to 6, 8 to 10, 12, 12a): which
//! operands are divided, what the pieces are, and the refusals.

#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    AnchorId, AnchorKind, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
    Tolerance, Vec2,
};
use curvyo_geometry_core::nearest_point_on_segment;
use curvyo_ui_core::{
    AnchorIdMinter, CutPlan, CutRefusal, MAX_CUT_PIECES, ObjectSelection, plan_cut,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn select(ids: &[NodeId]) -> ObjectSelection {
    let mut selection = ObjectSelection::new();
    for &id in ids {
        selection.add(id);
    }
    selection
}

fn minter() -> AnchorIdMinter {
    AnchorIdMinter::new(77)
}

fn poly(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), pt(x, y)))
        .collect();
    document.create_path(&anchors, closed)
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    })
}

/// A circle path of four smooth nodes, turned by `degrees` about its centre.
fn turned_circle(document: &Document, first: u64, radius: f64, degrees: f64) -> NodeId {
    let k = radius * 0.552_284_749_830_793_4;
    let (s, c) = degrees.to_radians().sin_cos();
    let turn = |x: f64, y: f64| (x * c - y * s, x * s + y * c);
    let node = |n: u64, x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
        let (px, py) = turn(x, y);
        let (ix, iy) = turn(hin.0, hin.1);
        let (ox, oy) = turn(hout.0, hout.1);
        NewAnchor {
            id: AnchorId::new(2, first + n),
            point: pt(px, py),
            handle_in: Vec2::new(ix, iy),
            handle_out: Vec2::new(ox, oy),
            kind: AnchorKind::Symmetric,
        }
    };
    let anchors = [
        node(0, radius, 0.0, (0.0, -k), (0.0, k)),
        node(1, 0.0, radius, (k, 0.0), (-k, 0.0)),
        node(2, -radius, 0.0, (0.0, k), (0.0, -k)),
        node(3, 0.0, -radius, (-k, 0.0), (k, 0.0)),
    ];
    document.create_path(&anchors, true)
}

fn plan(document: &Document, ids: &[NodeId]) -> Result<CutPlan, CutRefusal> {
    plan_cut(&objects(document), &select(ids), &mut minter())
}

fn points_of(piece: &[NewAnchor]) -> Vec<Point> {
    piece.iter().map(|a| a.point).collect()
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= 0.01
}

/// 12: two crossing lines give four pieces, each from or to (5, 5).
#[test]
fn two_crossing_lines_give_four_pieces_meeting_at_the_crossing() {
    let document = Document::new(1);
    let a = poly(&document, 10, &[(0.0, 0.0), (10.0, 10.0)], false);
    let b = poly(&document, 20, &[(0.0, 10.0), (10.0, 0.0)], false);
    let plan = plan(&document, &[a, b]).unwrap();
    assert_eq!(plan.removed, vec![a, b]);
    assert_eq!(plan.pieces.len(), 4);
    assert_eq!(plan.kept, vec![]);
    let centre = pt(5.0, 5.0);
    for (_, outlines) in &plan.pieces {
        assert_eq!(outlines.len(), 1);
        let (anchors, closed) = &outlines[0];
        assert!(!closed);
        assert_eq!(anchors.len(), 2);
        let ends = points_of(anchors);
        assert!(near(ends[0], centre) || near(ends[1], centre), "{ends:?}");
        // New end nodes are corners with a zero handle on the open side.
        let at_cut = usize::from(!near(ends[0], centre));
        assert_eq!(anchors[at_cut].kind, AnchorKind::Corner);
    }
    // The pieces of one operand come in order along it, directly after each other.
    assert_eq!(plan.pieces[0].0, a);
    assert_eq!(plan.pieces[1].0, a);
    assert_eq!(plan.pieces[2].0, b);
    assert!(near(points_of(&plan.pieces[0].1[0].0)[1], centre));
    assert!(near(points_of(&plan.pieces[1].1[0].0)[0], centre));
}

/// 12: a circle through whose nodes a line runs: the line gives three pieces, the circle two arcs
/// of three nodes each, and no node is added.
#[test]
fn a_line_through_the_nodes_of_a_circle() {
    let document = Document::new(1);
    let circle = turned_circle(&document, 100, 10.0, 0.0);
    let line = poly(&document, 10, &[(-20.0, 0.0), (20.0, 0.0)], false);
    let plan = plan(&document, &[circle, line]).unwrap();
    let of = |id: NodeId| -> Vec<&Vec<NewAnchor>> {
        plan.pieces
            .iter()
            .filter(|(source, _)| *source == id)
            .map(|(_, outlines)| &outlines[0].0)
            .collect()
    };
    assert_eq!(of(line).len(), 3);
    let arcs = of(circle);
    assert_eq!(arcs.len(), 2);
    for arc in arcs {
        assert_eq!(arc.len(), 3, "no node was added");
    }
    // The first arc runs from (10, 0) over (0, 10) to (-10, 0).
    let first = points_of(of(circle)[0]);
    assert_eq!(first, vec![pt(10.0, 0.0), pt(0.0, 10.0), pt(-10.0, 0.0)]);
}

/// 4 and 12: the same circle turned by 30 degrees: the cuts are inside segments and every point
/// of every arc stays within 1e-6 mm of the original curve.
#[test]
fn a_turned_circle_is_cut_on_its_curve() {
    let document = Document::new(1);
    let circle = turned_circle(&document, 100, 10.0, 30.0);
    let line = poly(&document, 10, &[(-20.0, 0.0), (20.0, 0.0)], false);
    let all = objects(&document);
    let ObjectSnapshot::Path(original) = all.iter().find(|o| o.id() == circle).unwrap() else {
        panic!("a path");
    };
    let plan = plan_cut(&all, &select(&[circle, line]), &mut minter()).unwrap();
    let arcs: Vec<&Vec<NewAnchor>> = plan
        .pieces
        .iter()
        .filter(|(source, _)| *source == circle)
        .map(|(_, outlines)| &outlines[0].0)
        .collect();
    assert_eq!(arcs.len(), 2);
    let segments: Vec<(NewAnchor, NewAnchor)> = (0..4)
        .map(|k| (original.anchors[k], original.anchors[(k + 1) % 4]))
        .collect();
    let distance_to_original = |p: Point| {
        segments
            .iter()
            .map(|(from, to)| {
                nearest_point_on_segment(
                    from.point,
                    from.handle_out,
                    to.handle_in,
                    to.point,
                    p,
                    Tolerance::from_mm(1e-9),
                )
                .1
                .as_mm()
            })
            .fold(f64::INFINITY, f64::min)
    };
    for arc in arcs {
        for pair in arc.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            for step in 0..=20 {
                let t = f64::from(step) / 20.0;
                let at = curvyo_geometry_core::subdivide_at_parameter(
                    from.point,
                    from.handle_out,
                    to.handle_in,
                    to.point,
                    t,
                )
                .new_point;
                assert!(distance_to_original(at) < 1e-6, "{at:?}");
            }
        }
        // The new end nodes: corners with zero handles on the open side.
        assert_eq!(arc[0].kind, AnchorKind::Corner);
        assert_eq!(arc[0].handle_in, Vec2::ZERO);
        assert_eq!(arc[arc.len() - 1].kind, AnchorKind::Corner);
        assert_eq!(arc[arc.len() - 1].handle_out, Vec2::ZERO);
        // Every other node keeps its place and its handles.
        for inner in &arc[1..arc.len() - 1] {
            assert!(
                original
                    .anchors
                    .iter()
                    .any(|o| o.point == inner.point && o.kind == inner.kind)
            );
        }
    }
}

/// 12: a closed path that crosses itself is cut at both passes of the crossing: two pieces, one
/// per lobe. (The spec text says "one open piece"; see the note in `adrs.md` of 0036.)
#[test]
fn a_figure_eight_gives_one_piece_per_lobe() {
    let document = Document::new(1);
    let eight = poly(
        &document,
        10,
        &[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)],
        true,
    );
    let plan = plan(&document, &[eight]).unwrap();
    assert_eq!(plan.pieces.len(), 2);
    for (_, outlines) in &plan.pieces {
        let ends = points_of(&outlines[0].0);
        assert!(near(ends[0], pt(5.0, 5.0)) && near(*ends.last().unwrap(), pt(5.0, 5.0)));
    }
}

/// 12: a circle cut at one point only gives one open piece whose ends meet at the cut.
#[test]
fn a_closed_path_with_one_cut_gives_one_piece_with_both_ends_at_the_cut() {
    let document = Document::new(1);
    let square = poly(
        &document,
        10,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        true,
    );
    // A stub that ends exactly on the square's bottom edge, from inside.
    let stub = poly(&document, 30, &[(5.0, 5.0), (5.0, 0.0)], false);
    let plan = plan(&document, &[square, stub]).unwrap();
    assert_eq!(plan.removed, vec![square]);
    assert_eq!(
        plan.kept,
        vec![stub],
        "the stub ends at the cut and is left alone"
    );
    assert_eq!(plan.pieces.len(), 1);
    let ends = points_of(&plan.pieces[0].1[0].0);
    assert_eq!(ends.len(), 6);
    assert_eq!(ends[0], pt(5.0, 0.0));
    assert_eq!(ends[5], pt(5.0, 0.0));
}

/// 12: a T. The line the stem ends on gives two pieces, the stem is left as it is.
#[test]
fn a_t_cuts_the_line_and_leaves_the_stem() {
    let document = Document::new(1);
    let bar = poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    let stem = poly(&document, 20, &[(5.0, 0.0), (5.0, 8.0)], false);
    let plan = plan(&document, &[bar, stem]).unwrap();
    assert_eq!(plan.removed, vec![bar]);
    assert_eq!(plan.kept, vec![stem]);
    assert_eq!(plan.pieces.len(), 2);
}

/// 12: collinear lines are cut at the ends of the shared stretch and the doubled stretch stays.
#[test]
fn collinear_lines_keep_the_doubled_stretch() {
    let document = Document::new(1);
    let a = poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    let b = poly(&document, 20, &[(5.0, 0.0), (15.0, 0.0)], false);
    let plan = plan(&document, &[a, b]).unwrap();
    let spans: Vec<(f64, f64)> = plan
        .pieces
        .iter()
        .map(|(_, outlines)| {
            let ends = points_of(&outlines[0].0);
            (ends[0].x, ends[ends.len() - 1].x)
        })
        .collect();
    assert_eq!(
        spans,
        vec![(0.0, 5.0), (5.0, 10.0), (5.0, 10.0), (10.0, 15.0)]
    );
}

/// 12a (E4): two overlapping circles of radius 10, centres 10 apart: four open pieces with Fill
/// left to the write, ends at the two crossings.
#[test]
fn two_overlapping_circles_give_four_arcs() {
    let document = Document::new(1);
    let a = disc(&document, 0.0, 0.0, 10.0);
    let b = disc(&document, 10.0, 0.0, 10.0);
    let plan = plan(&document, &[a, b]).unwrap();
    assert_eq!(plan.removed, vec![a, b]);
    assert_eq!(plan.pieces.len(), 4);
    let crossings = [pt(5.0, 8.66), pt(5.0, -8.66)];
    for (_, outlines) in &plan.pieces {
        let (anchors, closed) = &outlines[0];
        assert!(!closed);
        let ends = points_of(anchors);
        assert!(crossings.iter().any(|&c| near(ends[0], c)), "{ends:?}");
        assert!(
            crossings.iter().any(|&c| near(ends[ends.len() - 1], c)),
            "{ends:?}"
        );
    }
}

/// 3: an operand without a cut is left as it is; objects that do not cross are refused.
#[test]
fn operands_that_do_not_cross_are_refused_and_untouched_ones_are_kept() {
    let document = Document::new(1);
    let a = poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    let b = poly(&document, 20, &[(0.0, 5.0), (10.0, 5.0)], false);
    assert_eq!(plan(&document, &[a, b]), Err(CutRefusal::NoCrossing));
    let circle = disc(&document, 100.0, 100.0, 5.0);
    assert_eq!(plan(&document, &[circle]), Err(CutRefusal::NoCrossing));
    assert_eq!(plan(&document, &[]), Err(CutRefusal::NoneSelected));
    let cross = poly(&document, 30, &[(5.0, -5.0), (5.0, 3.0)], false);
    let plan = plan(&document, &[a, b, cross]).unwrap();
    assert_eq!(
        plan.kept,
        vec![b],
        "only the line the cross passes is untouched: b is crossed too"
    );
}

/// 9: a compound path is refused, with the compound paths named.
#[test]
fn a_compound_path_is_refused() {
    let document = Document::new(1);
    let base = disc(&document, 0.0, 0.0, 1.0);
    let ring = document
        .replace_with_path(
            &[base],
            base,
            &[
                (
                    vec![
                        NewAnchor::corner(AnchorId::new(5, 0), pt(0.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(5, 1), pt(40.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(5, 2), pt(40.0, 40.0)),
                    ],
                    true,
                ),
                (
                    vec![
                        NewAnchor::corner(AnchorId::new(5, 3), pt(10.0, 10.0)),
                        NewAnchor::corner(AnchorId::new(5, 4), pt(10.0, 20.0)),
                        NewAnchor::corner(AnchorId::new(5, 5), pt(20.0, 10.0)),
                    ],
                    true,
                ),
            ],
            "combine_paths",
        )
        .unwrap();
    let line = poly(&document, 10, &[(-5.0, 5.0), (50.0, 5.0)], false);
    assert_eq!(
        plan(&document, &[ring, line]),
        Err(CutRefusal::Compound {
            offenders: vec![ring]
        })
    );
}

/// 9: a coordinate beyond the kernel's range is refused.
#[test]
fn a_coordinate_out_of_range_is_refused() {
    let document = Document::new(1);
    let a = poly(&document, 10, &[(0.0, 0.0), (1.0e9, 10.0)], false);
    let b = poly(&document, 20, &[(0.0, 10.0), (10.0, 0.0)], false);
    assert_eq!(
        plan(&document, &[a, b]),
        Err(CutRefusal::OutOfRange {
            offenders: vec![a],
            of: 2
        })
    );
}

/// 10: more than 10,000 pieces are refused before anything is built.
#[test]
fn more_than_ten_thousand_pieces_are_refused() {
    let document = Document::new(1);
    let mut ids = Vec::new();
    for k in 0..71_u64 {
        let y = k as f64 * 2.0 + 1.0;
        ids.push(poly(
            &document,
            1000 + k * 2,
            &[(0.0, y), (200.0, y)],
            false,
        ));
        let x = k as f64 * 2.0 + 1.0;
        ids.push(poly(
            &document,
            5000 + k * 2,
            &[(x, 0.0), (x, 200.0)],
            false,
        ));
    }
    let refused = plan(&document, &ids);
    let pieces = 2 * 71 * 71 + 2 * 71;
    assert!(pieces > MAX_CUT_PIECES);
    assert_eq!(refused, Err(CutRefusal::TooMany { pieces }));
}

/// 11: the stacking order, not the selection order, decides the result.
#[test]
fn the_selection_order_does_not_matter() {
    let document = Document::new(1);
    let a = poly(&document, 10, &[(0.0, 0.0), (10.0, 10.0)], false);
    let b = poly(&document, 20, &[(0.0, 10.0), (10.0, 0.0)], false);
    let forward = plan(&document, &[a, b]).unwrap();
    let backward = plan(&document, &[b, a]).unwrap();
    let shape = |p: &CutPlan| -> Vec<(NodeId, Vec<Point>)> {
        p.pieces
            .iter()
            .map(|(id, outlines)| (*id, points_of(&outlines[0].0)))
            .collect()
    };
    assert_eq!(shape(&forward), shape(&backward));
    let mut ids: Vec<AnchorId> = forward
        .pieces
        .iter()
        .flat_map(|(_, outlines)| outlines[0].0.iter().map(|a| a.id))
        .collect();
    let count = ids.len();
    ids.sort_by_key(|id| id.as_u128());
    ids.dedup();
    assert_eq!(ids.len(), count, "fresh ids");
}

mod pieces_rejoin {
    use super::*;
    use proptest::prelude::*;

    fn coordinate() -> impl Strategy<Value = f64> {
        (-100i32..=100).prop_map(|n| f64::from(n) * 0.2)
    }

    fn anchor_strategy() -> impl Strategy<Value = ((f64, f64), (f64, f64), (f64, f64))> {
        (
            (coordinate(), coordinate()),
            (coordinate(), coordinate()),
            (coordinate(), coordinate()),
        )
    }

    fn path_of(
        document: &Document,
        first: u64,
        nodes: &[((f64, f64), (f64, f64), (f64, f64))],
        closed: bool,
    ) -> NodeId {
        let anchors: Vec<NewAnchor> = nodes
            .iter()
            .enumerate()
            .map(|(k, &(p, hin, hout))| NewAnchor {
                id: AnchorId::new(3, first + k as u64),
                point: pt(p.0, p.1),
                handle_in: Vec2::new(hin.0 * 0.3, hin.1 * 0.3),
                handle_out: Vec2::new(hout.0 * 0.3, hout.1 * 0.3),
                kind: AnchorKind::Corner,
            })
            .collect();
        document.create_path(&anchors, closed)
    }

    /// The length of the cubic chain through `anchors` by dense sampling.
    fn length(anchors: &[NewAnchor], closed: bool) -> f64 {
        let count = anchors.len();
        let segments = if closed { count } else { count - 1 };
        let mut total = 0.0;
        for k in 0..segments {
            let (from, to) = (&anchors[k], &anchors[(k + 1) % count]);
            let mut last = from.point;
            for step in 1..=400 {
                let t = f64::from(step) / 400.0;
                let at = curvyo_geometry_core::subdivide_at_parameter(
                    from.point,
                    from.handle_out,
                    to.handle_in,
                    to.point,
                    t,
                )
                .new_point;
                total += (at.x - last.x).hypot(at.y - last.y);
                last = at;
            }
        }
        total
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Whatever crosses, the pieces of an operand lie end to end along it: the same total
        /// length, the first piece starts where the contour starts, the last one ends where it
        /// ends, and every piece is open with at least two nodes.
        #[test]
        fn the_pieces_of_an_operand_lie_end_to_end(
            a in proptest::collection::vec(anchor_strategy(), 2..4),
            b in proptest::collection::vec(anchor_strategy(), 2..4),
            closed_a in any::<bool>(),
        ) {
            let document = Document::new(1);
            let first = path_of(&document, 10, &a, closed_a && a.len() > 2);
            let second = path_of(&document, 40, &b, false);
            let all = objects(&document);
            let Ok(plan) = plan_cut(&all, &select(&[first, second]), &mut minter()) else {
                return Ok(());
            };
            for id in &plan.removed {
                let ObjectSnapshot::Path(original) = all.iter().find(|o| o.id() == *id).unwrap() else {
                    panic!("a path");
                };
                let pieces: Vec<&Vec<NewAnchor>> = plan
                    .pieces
                    .iter()
                    .filter(|(source, _)| source == id)
                    .map(|(_, outlines)| &outlines[0].0)
                    .collect();
                prop_assert!(pieces.iter().all(|piece| piece.len() >= 2));
                let total: f64 = pieces.iter().map(|piece| length(piece, false)).sum();
                let whole = length(&original.anchors, original.closed);
                prop_assert!((total - whole).abs() <= 1e-3 * whole.max(1.0), "{total} against {whole}");
                for pair in pieces.windows(2) {
                    let (end, start) = (pair[0].last().unwrap().point, pair[1][0].point);
                    prop_assert!((end.x - start.x).hypot(end.y - start.y) < 1e-6);
                }
                if !original.closed {
                    prop_assert_eq!(pieces[0][0].point, original.anchors[0].point);
                    prop_assert_eq!(
                        pieces.last().unwrap().last().unwrap().point,
                        original.anchors.last().unwrap().point
                    );
                }
            }
        }
    }
}
