//! Acceptance tests of `specs/0068-laserable-lettering` written from the specification by the
//! tester: the criteria the implementer's tests left uncovered or weak, at the plan level
//! (`plan_weld` over a real document, written with `Document::replace_with_path_at`).

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

use std::f64::consts::PI;

use curvyo_document_core::{
    AnchorKind, Document, Length, LineCap, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds,
    StyleEdit, Vec2, pack, unpack,
};
use curvyo_geometry_core::MAX_COORDINATE_MM;
use curvyo_ui_core::{AnchorIdMinter, Units, WELD_COMMIT_LABEL, WeldPlan, WeldRefusal, plan_weld};

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// A regular `n`-gon of radius `r` around (`cx`, `cy`), counter-clockwise or clockwise.
fn polygon(minter: &mut AnchorIdMinter, cx: f64, cy: f64, r: f64, ccw: bool) -> Vec<NewAnchor> {
    let n = 256;
    (0..n)
        .map(|k| {
            let k = if ccw { k } else { n - k };
            let t = 2.0 * PI * f64::from(k) / f64::from(n);
            NewAnchor::corner(
                minter.mint(),
                Point::new(cx + r * t.cos(), cy + r * t.sin()),
            )
        })
        .collect()
}

fn corners(minter: &mut AnchorIdMinter, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
        .collect()
}

/// A path object of the given outlines (written over a throw-away rectangle).
fn path(document: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let tmp = rect(document, 0.0, 0.0, 1.0, 1.0);
    document
        .replace_with_path(&[tmp], tmp, outlines, "test")
        .unwrap()
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn plan(
    document: &Document,
    context: Option<NodeId>,
    selected: &[NodeId],
    gap_mm: f64,
) -> Result<WeldPlan, WeldRefusal> {
    let units = Units::build(document, context);
    plan_weld(
        &objects(document),
        &units.selected(selected),
        Length::from_mm(gap_mm),
        &mut AnchorIdMinter::new(99),
    )
}

fn write(document: &Document, plan: &WeldPlan) -> NodeId {
    document
        .replace_with_path_at(
            &plan.removed,
            plan.style_from,
            plan.place,
            plan.name.as_deref(),
            &plan.outlines,
            WELD_COMMIT_LABEL,
        )
        .unwrap()
}

fn signed_area(anchors: &[NewAnchor]) -> f64 {
    let n = anchors.len();
    (0..n)
        .map(|i| {
            let (a, b) = (anchors[i].point, anchors[(i + 1) % n].point);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn net_area(plan: &WeldPlan) -> f64 {
    plan.outlines
        .iter()
        .map(|(a, _)| signed_area(a))
        .sum::<f64>()
        .abs()
}

fn bbox(plan: &WeldPlan) -> (f64, f64, f64, f64) {
    let points = plan
        .outlines
        .iter()
        .flat_map(|(a, _)| a.iter().map(|a| a.point));
    points.fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
    )
}

/// Criterion 24: closed, corner-only outlines with no duplicate or collinear consecutive nodes.
fn assert_clean(plan: &WeldPlan) {
    for (anchors, closed) in &plan.outlines {
        assert!(closed, "every outline is closed");
        assert!(anchors.len() >= 3);
        let n = anchors.len();
        for i in 0..n {
            let a = &anchors[i];
            assert_eq!(a.kind, AnchorKind::Corner);
            assert_eq!((a.handle_in, a.handle_out), (Vec2::ZERO, Vec2::ZERO));
            let (p, q, r) = (
                anchors[(i + n - 1) % n].point,
                a.point,
                anchors[(i + 1) % n].point,
            );
            assert!(
                (q.x - p.x).hypot(q.y - p.y) > 1e-4,
                "near-duplicate node at {q:?}"
            );
            let cross = (q.x - p.x) * (r.y - q.y) - (q.y - p.y) * (r.x - q.x);
            let len = (q.x - p.x).hypot(q.y - p.y) * (r.x - q.x).hypot(r.y - q.y);
            assert!(cross.abs() > 1e-9 * len, "collinear node at {q:?}");
        }
    }
}

/// Criterion 5: a ring keeps its full hole when a bar crosses its outer part; a disc inside
/// the hole stays an island and is a second piece.
#[test]
fn ac05_a_ring_keeps_its_full_hole_and_an_island_is_a_piece() {
    let document = Document::new(1);
    let mut m = AnchorIdMinter::new(7);
    let outer = polygon(&mut m, 0.0, 0.0, 20.0, true);
    let inner = polygon(&mut m, 0.0, 0.0, 10.0, false);
    let hole_area = signed_area(&inner).abs();
    let ring = path(&document, &[(outer, true), (inner, true)]);
    let bar = rect(&document, -25.0, 14.0, 50.0, 4.0);

    let two = plan(&document, None, &[ring, bar], 0.1).unwrap();
    assert_clean(&two);
    assert_eq!((two.outlines.len(), two.pieces), (2, 1));
    let hole = two
        .outlines
        .iter()
        .map(|(a, _)| signed_area(a).abs())
        .fold(f64::MAX, f64::min);
    assert!(
        (hole - hole_area).abs() < 2.0 * PI * 10.0 * 0.01,
        "the hole is the full disc: {hole} vs {hole_area}"
    );

    let island = path(&document, &[(polygon(&mut m, 0.0, 0.0, 5.0, true), true)]);
    let three = plan(&document, None, &[ring, bar, island], 0.1).unwrap();
    assert_clean(&three);
    assert_eq!(three.outlines.len(), 3, "outer, hole, island");
    assert_eq!(three.pieces, 2, "the island is a piece, the hole is not");
}

/// Criterion 11 (customer decision 2026-10-20): a closed outline contributes its region only,
/// whatever its stroke.
#[test]
fn ac11_a_closed_stroke_does_not_widen_the_result() {
    let document = Document::new(1);
    let square = rect(&document, 0.0, 0.0, 20.0, 20.0);
    document
        .edit_style(&[square], &StyleEdit::StrokeEnabled(true))
        .unwrap();
    document
        .edit_style(&[square], &StyleEdit::StrokeWidth(Length::from_mm(5.0)))
        .unwrap();
    let plan = plan(&document, None, &[square], 0.1).unwrap();
    assert!((net_area(&plan) - 400.0).abs() < 0.1, "{}", net_area(&plan));
    let (x0, y0, x1, y1) = bbox(&plan);
    assert!((x1 - x0 - 20.0).abs() < 0.01 && (y1 - y0 - 20.0).abs() < 0.01);
}

fn stroked_line(document: &Document, m: &mut AnchorIdMinter, dashed: bool) -> NodeId {
    let line = path(document, &[(corners(m, &[(0.0, 0.0), (40.0, 0.0)]), false)]);
    for edit in [
        StyleEdit::StrokeEnabled(true),
        StyleEdit::FillEnabled(false),
        StyleEdit::StrokeWidth(Length::from_mm(2.0)),
        StyleEdit::StrokeCap(LineCap::Round),
    ] {
        document.edit_style(&[line], &edit).unwrap();
    }
    if dashed {
        let dash = curvyo_document_core::DashPattern::new(vec![3.0, 2.0]).unwrap();
        document
            .edit_style(&[line], &StyleEdit::StrokeDash(dash))
            .unwrap();
    }
    line
}

/// Criteria 9 and 10: an open stroked line contributes the band of its stroke as drawn (round
/// caps); a dashed line welds as solid and the plan says so.
#[test]
fn ac09_ac10_an_open_line_welds_as_its_band_and_dashes_as_solid() {
    let expected = 40.0 * 2.0 + PI + 100.0 - 10.0 * 2.0;
    for dashed in [false, true] {
        let document = Document::new(1);
        let mut m = AnchorIdMinter::new(7);
        let line = stroked_line(&document, &mut m, dashed);
        let square = rect(&document, 15.0, -5.0, 10.0, 10.0);
        let plan = plan(&document, None, &[line, square], 0.0).unwrap();
        assert_clean(&plan);
        assert_eq!((plan.outlines.len(), plan.pieces), (1, 1));
        assert!(
            (net_area(&plan) - expected).abs() < 0.05,
            "dashed {dashed}: {} vs {expected}",
            net_area(&plan)
        );
        let (x0, _, x1, _) = bbox(&plan);
        assert!(
            (x0 + 1.0).abs() < 0.01 && (x1 - 41.0).abs() < 0.01,
            "round caps"
        );
        assert_eq!(plan.dashed, dashed);
    }
}

/// Criterion 10 and 28: an open outline without stroke closes with a straight line; one that
/// then encloses no area is counted, and alone it refuses Weld.
#[test]
fn ac10_ac28_open_outlines_without_stroke_close_or_refuse() {
    let document = Document::new(1);
    let mut m = AnchorIdMinter::new(7);
    let corner = path(
        &document,
        &[(
            corners(&mut m, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]),
            false,
        )],
    );
    let straight = path(
        &document,
        &[(corners(&mut m, &[(30.0, 0.0), (40.0, 0.0)]), false)],
    );
    for edit in [
        StyleEdit::StrokeEnabled(false),
        StyleEdit::FillEnabled(false),
    ] {
        document.edit_style(&[corner, straight], &edit).unwrap();
    }
    let both = plan(&document, None, &[corner, straight], 0.0).unwrap();
    assert!((net_area(&both) - 50.0).abs() < 0.01, "{}", net_area(&both));
    assert_eq!((both.closed_open, both.removed_open), (1, 1));

    let Err(WeldRefusal::NoArea { offenders, of }) = plan(&document, None, &[straight], 0.1) else {
        panic!("no area refuses");
    };
    assert_eq!((offenders, of), (vec![straight], 1));
}

/// Criterion 28: a coordinate beyond `MAX_COORDINATE_MM` refuses and names the offender.
#[test]
fn ac28_a_coordinate_out_of_range_refuses() {
    let document = Document::new(1);
    let fine = rect(&document, 0.0, 0.0, 10.0, 10.0);
    let far = rect(&document, MAX_COORDINATE_MM + 10.0, 0.0, 10.0, 10.0);
    let Err(WeldRefusal::OutOfRange {
        offenders,
        objects,
        of,
    }) = plan(&document, None, &[fine, far], 0.1)
    else {
        panic!("out of range refuses");
    };
    assert_eq!((offenders, objects, of), (vec![far], 1, 2));
}

/// Criterion 13: leaves of nested groups are operands; one compound path whose outlines
/// overlap welds into its region.
#[test]
fn ac13_nested_groups_and_an_overlapping_compound_weld() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 20.0, 20.0);
    let b = rect(&document, 10.0, 10.0, 20.0, 20.0);
    let inner = document.group_objects(&[a, b]).unwrap();
    let c = rect(&document, 25.0, 25.0, 10.0, 10.0);
    let middle = document.group_objects(&[inner, c]).unwrap();
    let d = rect(&document, 30.0, 30.0, 10.0, 10.0);
    let outer = document.group_objects(&[middle, d]).unwrap();
    let plan_ = plan(&document, None, &[a], 0.0).unwrap();
    assert_eq!(
        plan_.removed,
        vec![a, b, c, d],
        "a leaf picks its top group"
    );
    assert_eq!((plan_.place, plan_.objects, plan_.pieces), (outer, 1, 1));
    assert!((net_area(&plan_) - (700.0 + 75.0 + 75.0)).abs() < 0.01);
    let result = write(&document, &plan_);
    assert_eq!(document.children(None), vec![result]);

    let document = Document::new(1);
    let mut m = AnchorIdMinter::new(7);
    let square = |m: &mut AnchorIdMinter, x: f64| {
        corners(
            m,
            &[(x, x), (x + 20.0, x), (x + 20.0, x + 20.0), (x, x + 20.0)],
        )
    };
    let combined = path(
        &document,
        &[(square(&mut m, 0.0), true), (square(&mut m, 10.0), true)],
    );
    let plan = plan(&document, None, &[combined], 0.0).unwrap();
    assert_clean(&plan);
    assert_eq!((plan.outlines.len(), plan.outlines[0].0.len()), (1, 8));
    assert!((net_area(&plan) - 700.0).abs() < 0.01);
}

/// Criterion 14: one rectangle that overlaps nothing still welds, to its region as 1 piece.
#[test]
fn ac14_one_lone_rectangle_still_welds() {
    let document = Document::new(1);
    let r = rect(&document, 3.0, 4.0, 20.0, 10.0);
    let plan = plan(&document, None, &[r], 0.1).unwrap();
    assert_clean(&plan);
    assert_eq!(
        (plan.outlines.len(), plan.outlines[0].0.len(), plan.pieces),
        (1, 4, 1)
    );
    assert!((net_area(&plan) - 200.0).abs() < 0.01);
}

/// Criteria 19 and 20: unselected objects keep ids and order; the result takes the lowest
/// operand's complete style and the lowest selected object's place, inside an entered group.
#[test]
fn ac19_ac20_place_style_and_untouched_neighbours() {
    let document = Document::new(1);
    let below = rect(&document, -50.0, 0.0, 10.0, 10.0);
    let a = rect(&document, 0.0, 0.0, 20.0, 20.0);
    let middle = rect(&document, 100.0, 0.0, 10.0, 10.0);
    let b = rect(&document, 10.0, 10.0, 20.0, 20.0);
    let above = rect(&document, 200.0, 0.0, 10.0, 10.0);
    let group = document
        .group_objects(&[below, a, middle, b, above])
        .unwrap();
    document
        .edit_style(&[b], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let a_style = document.object(a).unwrap().style().clone();
    let plan = plan(&document, Some(group), &[b, a], 0.0).unwrap();
    assert_eq!((plan.place, plan.style_from), (a, a));
    assert!(plan.mixed_styles);
    let result = write(&document, &plan);
    assert_eq!(
        document.children(Some(group)),
        vec![below, result, middle, above]
    );
    assert_eq!(document.object(result).unwrap().style(), &a_style);
}

/// Criterion 26: the saved result reopens with the same outlines, node order and style.
#[test]
fn ac26_a_welded_result_round_trips() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 20.0, 20.0);
    let b = rect(&document, 10.0, 10.0, 20.0, 20.0);
    let far = rect(&document, 50.0, 0.0, 5.0, 5.0);
    let plan = plan(&document, None, &[a, b, far], 0.1).unwrap();
    let result = write(&document, &plan);
    let reopened = unpack(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.object(result), document.object(result));
    let Some(ObjectSnapshot::Path(path)) = reopened.object(result) else {
        panic!("a path");
    };
    let saved: Vec<Vec<Point>> = path
        .subpaths()
        .map(|s| s.anchors.iter().map(|a| a.point).collect())
        .collect();
    let planned: Vec<Vec<Point>> = plan
        .outlines
        .iter()
        .map(|(a, _)| a.iter().map(|a| a.point).collect())
        .collect();
    assert_eq!(saved, planned);
}

/// Criterion 19: two selected groups both go, with all their members; nothing else is left.
#[test]
fn ac19_two_selected_groups_go_with_their_members() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 20.0, 20.0);
    let b = rect(&document, 10.0, 10.0, 20.0, 20.0);
    let first = document.group_objects(&[a, b]).unwrap();
    let c = rect(&document, 25.0, 25.0, 10.0, 10.0);
    let d = rect(&document, 60.0, 0.0, 10.0, 10.0);
    let second = document.group_objects(&[c, d]).unwrap();
    let plan = plan(&document, None, &[c, a], 0.0).unwrap();
    assert_eq!((plan.place, plan.objects, plan.pieces), (first, 2, 2));
    let result = write(&document, &plan);
    assert_eq!(document.children(None), vec![result]);
    assert!(!document.contains_node(first) && !document.contains_node(second));
}
