//! The Boolean operations, Cut and Offset with open outlines (`specs/0054-combine-open-paths`
//! criteria 20 to 24), on the plans.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
};
use curvyo_ui_core::{
    AnchorIdMinter, BooleanAvailability, BooleanOp, BooleanRefusal, CutRefusal, ObjectSelection,
    boolean_availability, offset_operands, plan_boolean, plan_cut,
};

fn anchors(first: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), Point::new(x, y)))
        .collect()
}

fn square(first: u64, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<NewAnchor> {
    anchors(first, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn compound(document: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let base = document.create_path(&anchors(1, &[(0.0, 0.0), (1.0, 0.0)]), false);
    document
        .replace_with_path(&[base], base, outlines, "test")
        .unwrap()
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

fn shoelace(anchors: &[NewAnchor]) -> f64 {
    let n = anchors.len();
    (0..n)
        .map(|i| {
            let (a, b) = (anchors[i].point, anchors[(i + 1) % n].point);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

/// The open "U" of criterion 2.
fn u_shape(first: u64) -> Vec<NewAnchor> {
    anchors(first, &[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)])
}

/// Criteria 21 and 23: Union of the open U and a far square: 2 outlines, the U closed by a
/// straight edge and enclosing 100 mm², one open outline counted before and after.
#[test]
fn c21_union_closes_the_u_with_a_straight_line() {
    let document = Document::new(1);
    let u = document.create_path(&u_shape(10), false);
    let far = document.create_path(&square(20, 50.0, 0.0, 60.0, 10.0), true);
    let all = objects(&document);
    assert_eq!(
        boolean_availability(&all, &select(&[u, far])),
        BooleanAvailability::ClosesOpen { outlines: 1 }
    );
    let plan = plan_boolean(
        &all,
        &select(&[u, far]),
        BooleanOp::Union,
        &mut AnchorIdMinter::new(5),
    )
    .unwrap();
    assert_eq!(plan.outlines.len(), 2);
    assert!(plan.compound);
    assert_eq!((plan.closed_open, plan.removed_open), (1, 0));
    let mut areas: Vec<f64> = plan
        .outlines
        .iter()
        .map(|(a, _)| shoelace(a).abs())
        .collect();
    areas.sort_by(f64::total_cmp);
    assert!((areas[0] - 100.0).abs() < 0.05, "{areas:?}");
}

/// Criterion 22: a plate combined with a straight score line, minus a disc on top: the plate
/// minus the disc, the score line removed and counted; a straight open line alone has no area.
#[test]
fn c22_a_straight_score_line_is_removed_and_lines_alone_are_refused() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            (square(10, 0.0, 0.0, 100.0, 100.0), true),
            (anchors(20, &[(0.0, 50.0), (100.0, 50.0)]), false),
        ],
    );
    let disc = document.create_ellipse(EllipseFrame {
        center: Point::new(50.0, 50.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let all = objects(&document);
    let plan = plan_boolean(
        &all,
        &select(&[plate, disc]),
        BooleanOp::Difference,
        &mut AnchorIdMinter::new(5),
    )
    .unwrap();
    assert_eq!((plan.closed_open, plan.removed_open), (0, 1));
    assert_eq!(plan.outlines.len(), 2, "the plate and the disc's hole");

    let document = Document::new(1);
    let line = document.create_path(&anchors(10, &[(0.0, 0.0), (30.0, 0.0)]), false);
    let sq = document.create_path(&square(20, 0.0, 10.0, 10.0, 20.0), true);
    let all = objects(&document);
    assert_eq!(
        plan_boolean(
            &all,
            &select(&[line, sq]),
            BooleanOp::Union,
            &mut AnchorIdMinter::new(5)
        ),
        Err(BooleanRefusal::NoArea {
            offenders: vec![line],
            of: 2
        })
    );
}

/// Criterion 20: Cut refuses a compound path with open outlines as every compound path.
#[test]
fn c20_cut_refuses_a_compound_path_with_open_outlines() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            (square(10, 0.0, 0.0, 100.0, 100.0), true),
            (anchors(20, &[(-10.0, 50.0), (110.0, 50.0)]), false),
        ],
    );
    assert!(matches!(
        plan_cut(&objects(&document), &select(&[plate]), &mut AnchorIdMinter::new(5)),
        Err(CutRefusal::Compound { offenders, .. }) if offenders == vec![plate]
    ));
}

/// Criterion 24 as built (`plan.md`, question OFFSET-OPEN answered A): Offset keeps `0038`
/// criterion 55; a compound path with an open outline is not an operand, like an open path.
#[test]
fn c24_offset_ignores_a_compound_path_with_open_outlines() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            (square(10, 0.0, 0.0, 100.0, 100.0), true),
            (anchors(20, &[(10.0, 50.0), (90.0, 50.0)]), false),
        ],
    );
    let closed = document.create_path(&square(30, 200.0, 0.0, 210.0, 10.0), true);
    let all = objects(&document);
    let operands: Vec<NodeId> = offset_operands(&all, &select(&[plate, closed]))
        .iter()
        .map(|object| object.id())
        .collect();
    assert_eq!(operands, vec![closed]);
}

/// Criterion 23 with criterion 22 (UX review finding 3): the note counts only the open outlines
/// the press closes; a straight one, which encloses nothing, is not counted.
#[test]
fn c23_the_note_counts_only_outlines_that_enclose_an_area() {
    let document = Document::new(1);
    let u = document.create_path(&u_shape(10), false);
    let straight = document.create_path(&anchors(20, &[(30.0, 0.0), (40.0, 0.0)]), false);
    let plate = compound(
        &document,
        &[
            (square(30, 50.0, 0.0, 100.0, 50.0), true),
            (anchors(40, &[(50.0, 25.0), (100.0, 25.0)]), false),
        ],
    );
    let all = objects(&document);
    assert_eq!(
        boolean_availability(&all, &select(&[u, straight, plate])),
        BooleanAvailability::ClosesOpen { outlines: 1 }
    );
    assert_eq!(
        boolean_availability(&all, &select(&[straight, plate])),
        BooleanAvailability::Ready,
        "only straight open outlines: nothing to close"
    );
}
