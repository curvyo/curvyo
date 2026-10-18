//! Combine with open, touching and crossing outlines (`specs/0054-combine-open-paths` criteria 6 to
//! 12 and 14), on the plan: outlines, windings, the painted area and the notice counts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, Length, MarkerShape, NewAnchor, NodeId, ObjectSnapshot,
    Point, StyleEdit,
};
use curvyo_geometry_core::{Outline, OutlineTriple, contains_point_in_outlines};
use curvyo_ui_core::{
    AnchorIdMinter, BooleanAvailability, CombinePlan, CombineRefusal, ObjectSelection,
    path_availability, plan_combine,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn anchors(first: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), pt(x, y)))
        .collect()
}

fn poly(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    document.create_path(&anchors(first, points), closed)
}

fn square(document: &Document, first: u64, x0: f64, y0: f64, x1: f64, y1: f64) -> NodeId {
    poly(
        document,
        first,
        &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
        true,
    )
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    })
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn all_selected(document: &Document) -> ObjectSelection {
    let mut selection = ObjectSelection::new();
    for id in document.object_ids() {
        selection.add(id);
    }
    selection
}

fn plan(document: &Document) -> Result<CombinePlan, CombineRefusal> {
    plan_combine(
        &objects(document),
        &all_selected(document),
        &mut AnchorIdMinter::new(77),
    )
}

/// Whether the plan's outlines paint `point` (nonzero, open outlines closed by their chord).
fn painted(plan: &CombinePlan, point: Point) -> bool {
    let owned: Vec<(Vec<OutlineTriple>, bool)> = plan
        .outlines
        .iter()
        .map(|(anchors, closed)| {
            (
                anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect(),
                *closed,
            )
        })
        .collect();
    let outlines: Vec<Outline<'_>> = owned.iter().map(|(t, c)| Outline::new(t, *c)).collect();
    contains_point_in_outlines(&outlines, point)
}

/// Criterion 6: two open lines and a circle give one compound path of 3 outlines, 2 + 2 + 4
/// nodes, open, open, closed.
#[test]
fn c6_open_lines_and_a_circle_combine() {
    let document = Document::new(1);
    poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    poly(&document, 20, &[(0.0, 5.0), (10.0, 5.0)], false);
    disc(&document, 50.0, 50.0, 10.0);
    let plan = plan(&document).unwrap();
    let shape: Vec<(usize, bool)> = plan
        .outlines
        .iter()
        .map(|(anchors, closed)| (anchors.len(), *closed))
        .collect();
    assert_eq!(shape, vec![(2, false), (2, false), (4, true)]);
    assert_eq!((plan.open, plan.holes, plan.touching), (2, 0, 0));
}

/// Criterion 7a: concentric discs still give a ring.
#[test]
fn c7a_concentric_discs_give_a_ring() {
    let document = Document::new(1);
    disc(&document, 0.0, 0.0, 20.0);
    disc(&document, 0.0, 0.0, 10.0);
    let plan = plan(&document).unwrap();
    assert_eq!(plan.holes, 1);
    assert!(painted(&plan, pt(15.0, 0.0)));
    assert!(!painted(&plan, pt(0.0, 0.0)));
}

/// Criterion 7b: a square touching the left edge from inside is a hole.
#[test]
fn c7b_touching_from_inside_is_a_hole() {
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 40.0, 40.0);
    square(&document, 20, 0.0, 10.0, 20.0, 30.0);
    let plan = plan(&document).unwrap();
    assert_eq!(plan.holes, 1);
    assert!(!painted(&plan, pt(10.0, 20.0)));
    assert!(painted(&plan, pt(30.0, 20.0)));
    assert_eq!(plan.touching, 2);
}

/// Criterion 7c: crossing squares are both shapes; the overlap stays painted.
#[test]
fn c7c_crossing_squares_keep_their_picture() {
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 20.0, 20.0);
    square(&document, 20, 10.0, 10.0, 30.0, 30.0);
    let plan = plan(&document).unwrap();
    assert_eq!((plan.holes, plan.touching, plan.touching_holes), (0, 2, 0));
    assert!(painted(&plan, pt(15.0, 15.0)));
    assert!(painted(&plan, pt(5.0, 5.0)));
    assert!(painted(&plan, pt(25.0, 25.0)));
}

/// Criterion 7d: two coincident squares are both shapes and the square is painted, whichever way
/// they were drawn.
#[test]
fn c7d_coincident_squares_are_painted() {
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 20.0, 20.0);
    poly(
        &document,
        20,
        &[(0.0, 0.0), (0.0, 20.0), (20.0, 20.0), (20.0, 0.0)],
        true,
    );
    let plan = plan(&document).unwrap();
    assert_eq!(plan.holes, 0);
    assert!(painted(&plan, pt(10.0, 10.0)));
}

/// Criterion 7e: a closed figure eight keeps both lobes painted.
#[test]
fn c7e_a_figure_eight_paints_both_lobes() {
    let document = Document::new(1);
    poly(
        &document,
        10,
        &[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)],
        true,
    );
    square(&document, 20, 100.0, 0.0, 110.0, 10.0);
    let plan = plan(&document).unwrap();
    assert!(painted(&plan, pt(2.0, 5.0)));
    assert!(painted(&plan, pt(8.0, 5.0)));
    assert_eq!(plan.touching, 1, "one closed outline crosses itself");
}

/// Criterion 7f: an open line keeps its direction; its first node stays first.
#[test]
fn c7f_an_open_line_keeps_its_direction() {
    let document = Document::new(1);
    poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    disc(&document, 0.0, 0.0, 30.0);
    let plan = plan(&document).unwrap();
    let line = &plan.outlines[0];
    assert!(!line.1);
    assert_eq!(line.0[0].point, pt(0.0, 0.0));
    assert_eq!(line.0[1].point, pt(10.0, 0.0));
}

/// Criterion 7g: two crossing squares inside a third are both holes; their overlap is painted
/// (winding -1); the notice counts 2 touching closed outlines and 2 touching holes.
#[test]
fn c7g_crossing_holes_paint_their_overlap() {
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 40.0, 40.0);
    square(&document, 20, 5.0, 5.0, 25.0, 25.0);
    square(&document, 30, 15.0, 15.0, 35.0, 35.0);
    let plan = plan(&document).unwrap();
    assert_eq!((plan.holes, plan.touching, plan.touching_holes), (2, 2, 2));
    assert!(!painted(&plan, pt(10.0, 10.0)));
    assert!(painted(&plan, pt(20.0, 20.0)));
}

/// Criterion 8: each of the inputs that used to be refused combines.
#[test]
fn c8_every_formerly_refused_input_combines() {
    // A plate with an open score line across it.
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 100.0, 100.0);
    poly(&document, 20, &[(-10.0, 50.0), (110.0, 50.0)], false);
    let plate = plan(&document).unwrap();
    assert_eq!(
        (plate.open, plate.touching),
        (1, 0),
        "open outlines are not counted"
    );

    // Ten open lines forming a crossing grid.
    let document = Document::new(1);
    for k in 0..5 {
        let at = f64::from(k) * 10.0;
        poly(
            &document,
            100 + 2 * k as u64,
            &[(at, -5.0), (at, 45.0)],
            false,
        );
        poly(
            &document,
            200 + 2 * k as u64,
            &[(-5.0, at), (45.0, at)],
            false,
        );
    }
    assert_eq!(plan(&document).unwrap().outlines.len(), 10);

    // Two overlapping closed squares plus an open line.
    let document = Document::new(1);
    square(&document, 10, 0.0, 0.0, 20.0, 20.0);
    square(&document, 20, 10.0, 10.0, 30.0, 30.0);
    poly(&document, 30, &[(0.0, 50.0), (30.0, 50.0)], false);
    assert_eq!(plan(&document).unwrap().outlines.len(), 3);

    // A figure eight with two separate squares.
    let document = Document::new(1);
    poly(
        &document,
        10,
        &[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)],
        true,
    );
    square(&document, 20, 50.0, 0.0, 60.0, 10.0);
    square(&document, 30, 80.0, 0.0, 90.0, 10.0);
    assert_eq!(plan(&document).unwrap().outlines.len(), 3);

    // A closed path of two nodes with a circle: it keeps its direction.
    let document = Document::new(1);
    poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], true);
    disc(&document, 50.0, 50.0, 10.0);
    let two = plan(&document).unwrap();
    assert_eq!(two.outlines[0].0[0].point, pt(0.0, 0.0));
    assert!(two.outlines[0].1);
}

/// Criterion 9: a zero-length open outline is kept.
#[test]
fn c9_a_zero_length_open_outline_is_kept() {
    let document = Document::new(1);
    poly(&document, 10, &[(5.0, 5.0), (5.0, 5.0)], false);
    disc(&document, 50.0, 50.0, 10.0);
    let plan = plan(&document).unwrap();
    assert_eq!(plan.outlines.len(), 2);
    assert_eq!(plan.open, 1);
}

/// Criterion 10: outlines of a single node are left out and counted; nothing but single nodes is
/// refused.
#[test]
fn c10_single_node_outlines_are_removed_or_refused() {
    let document = Document::new(1);
    let base = poly(&document, 10, &[(0.0, 0.0), (1.0, 0.0)], false);
    let compound = document
        .replace_with_path(
            &[base],
            base,
            &[
                (anchors(20, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]), true),
                (anchors(30, &[(50.0, 50.0)]), false),
            ],
            "test",
        )
        .unwrap();
    disc(&document, 100.0, 100.0, 10.0);
    let kept = plan(&document).unwrap();
    assert_eq!(kept.single_nodes_removed, 1);
    assert_eq!(kept.outlines.len(), 2);
    let _ = compound;

    let document = Document::new(1);
    for k in 0..2_u64 {
        let base = poly(&document, 10 * (k + 1), &[(0.0, 0.0), (1.0, 0.0)], false);
        document
            .replace_with_path(
                &[base],
                base,
                &[
                    (anchors(100 + 10 * k, &[(f64::from(k as u32), 0.0)]), false),
                    (anchors(200 + 10 * k, &[(5.0, f64::from(k as u32))]), false),
                ],
                "test",
            )
            .unwrap();
    }
    assert_eq!(plan(&document), Err(CombineRefusal::SingleNodesOnly));
}

/// Criterion 11: a primitive contributes its outline; Combine says nothing about the conversion
/// (the plan has no count for it).
#[test]
fn c11_a_primitive_contributes_its_outline() {
    let document = Document::new(1);
    disc(&document, 0.0, 0.0, 10.0);
    poly(&document, 10, &[(30.0, 0.0), (40.0, 0.0)], false);
    let plan = plan(&document).unwrap();
    assert_eq!(plan.outlines[0].0.len(), 4);
    assert!(plan.outlines[0].1);
}

/// Criterion 13: with an open path and a circle selected, Combine is enabled.
#[test]
fn c13_combine_is_enabled_with_an_open_path() {
    let document = Document::new(1);
    poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    disc(&document, 50.0, 50.0, 10.0);
    let availability = path_availability(&objects(&document), &all_selected(&document));
    assert_eq!(availability.combine, BooleanAvailability::Ready);
}

/// Criterion 14: an operand with a marker on is reported; one without is not.
#[test]
fn c14_markers_are_reported() {
    let document = Document::new(1);
    let line = poly(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
    disc(&document, 50.0, 50.0, 10.0);
    assert!(!plan(&document).unwrap().markers);
    document
        .edit_style(&[line], &StyleEdit::MarkerEnd(MarkerShape::Arrow))
        .unwrap();
    assert!(plan(&document).unwrap().markers);
}
