//! `plan_weld` against `specs/0068-laserable-lettering` criteria 11, 13, 17, 18, 24 and 25.

use super::*;
use curvyo_document_core::{AnchorId, Color, Document, Length, RectBounds, StyleEdit};

use crate::units::Units;

fn rect(document: &Document, x: f64, y: f64, side: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(side),
        height: Length::from_mm(side),
    })
}

fn line(document: &Document, points: &[(f64, f64)]) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .zip(1_u64..)
        .map(|(&(x, y), k)| NewAnchor::corner(AnchorId::new(7, k), Point::new(x, y)))
        .collect();
    document.create_path(&anchors, false)
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn plan(document: &Document, selected: &[NodeId]) -> Result<WeldPlan, WeldRefusal> {
    let units = Units::build(document, None).selected(selected);
    plan_weld(&objects(document), &units, &mut AnchorIdMinter::new(5))
}

/// Criteria 11, 17 and 25: the leaves of a nested group are operands, the place is the lowest
/// selected object (the group), and the selection order does not matter.
#[test]
fn ac11_nested_group_leaves_are_operands_and_the_group_gives_its_place() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 20.0);
    let b = rect(&document, 10.0, 10.0, 20.0);
    let d = rect(&document, 60.0, 0.0, 5.0);
    let inner = document.group_objects(&[b, d]).unwrap();
    let group = document.group_objects(&[a, inner]).unwrap();
    let c = rect(&document, 40.0, 0.0, 10.0);
    let forward = plan(&document, &[a, b, d, c]).unwrap();
    assert_eq!(forward, plan(&document, &[c, d, b, a]).unwrap());
    assert_eq!(forward.removed, vec![a, b, d, c]);
    assert_eq!((forward.place, forward.objects), (group, 2));
    assert_eq!(forward.outlines.iter().filter(|o| o.1).count(), 3);
    assert_eq!(forward.outlines.iter().filter(|o| !o.1).count(), 1);
    assert!(
        (forward.length_mm - 200.0).abs() < 1e-6,
        "{}",
        forward.length_mm
    );
}

/// Criterion 13: an open operand is closed by a straight line and counted; a straight one adds
/// nothing and is counted as removed.
#[test]
fn ac13_open_operands_are_closed_or_removed_and_counted() {
    let document = Document::new(1);
    let square = rect(&document, 0.0, 0.0, 10.0);
    let bent = line(&document, &[(20.0, 0.0), (25.0, 5.0), (29.0, 0.0)]);
    let straight = line(&document, &[(40.0, 0.0), (50.0, 0.0)]);
    let all = objects(&document);
    let units = Units::build(&document, None).selected(&[square, bent, straight]);
    assert_eq!(
        weld_availability(&all, &units),
        WeldAvailability::ClosesOpen { outlines: 1 }
    );
    let plan = plan(&document, &[square, bent, straight]).unwrap();
    assert_eq!((plan.closed_open, plan.removed_open), (1, 1));
    assert_eq!(plan.removed.len(), 3);
}

/// Criterion 18 (14 A): the lowest leaf's style with Fill off; Stroke off becomes Stroke on in
/// the fill colour at the default width; differing styles are reported.
#[test]
fn ac18_the_style_is_the_lowest_leafs_as_line_art() {
    let document = Document::new(1);
    let lower = rect(&document, 0.0, 0.0, 20.0);
    let upper = rect(&document, 10.0, 10.0, 20.0);
    let red = Color { r: 200, g: 0, b: 0 };
    for edit in [
        StyleEdit::FillEnabled(true),
        StyleEdit::FillColor(red),
        StyleEdit::StrokeEnabled(false),
    ] {
        document.edit_style(&[lower], &edit).unwrap();
    }
    let plan = plan(&document, &[upper, lower]).unwrap();
    assert!(!plan.style.fill.enabled);
    assert!(plan.style.stroke.enabled);
    assert_eq!(plan.style.stroke.color, red);
    assert_eq!(plan.style.stroke.width, Style::default().stroke.width);
    assert!(plan.fill_switched_off && plan.stroke_switched_on && plan.mixed_styles);
}

/// Criterion 24: nothing with area is refused with every leaf to outline; one object alone welds
/// (criterion 12).
#[test]
fn ac24_nothing_with_area_is_refused_and_one_object_welds() {
    let document = Document::new(1);
    let a = line(&document, &[(0.0, 0.0), (10.0, 0.0)]);
    let b = line(&document, &[(0.0, 5.0), (10.0, 5.0)]);
    assert_eq!(
        plan(&document, &[a, b]),
        Err(WeldRefusal::NoArea {
            offenders: vec![a, b],
            of: 2
        })
    );
    let square = rect(&document, 0.0, 20.0, 10.0);
    let alone = plan(&document, &[square]).unwrap();
    assert_eq!(alone.outlines.len(), 1);
    assert_eq!(alone.place, square);
}
