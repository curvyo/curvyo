//! Tests of [`super::commit_param_copy`]: a parameter written to a copy in place
//! (`specs/0073-parameter-handle-clone` criteria 3, 4, 8 to 10, 12, 13).

use curvyo_document_core::{
    Angle, Document, EllipseFrame, InnerRatio, Length, NodeId, ObjectRowClass, ObjectSnapshot,
    Point, PointCount, RectBounds, Shape, StarFrame,
};

use super::{ParamValue, apply_param, commit_param_copy};
use crate::param_handles::{Corner, ParamHandle};
use crate::shape_arc_edit::ArcEnd;

fn star(document: &Document) -> NodeId {
    document.create_star(
        StarFrame {
            center: Point::new(50.0, 50.0),
            radius: Length::from_mm(20.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(5).expect("count"),
        InnerRatio::new(0.5).expect("ratio"),
    )
}

fn rect(document: &Document) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(20.0),
    })
}

fn ellipse(document: &Document) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: Point::new(0.0, 0.0),
        rx: Length::from_mm(20.0),
        ry: Length::from_mm(10.0),
    })
}

fn polygon(document: &Document) -> NodeId {
    document.create_polygon(
        StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(20.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(6).expect("count"),
    )
}

fn shape_of(object: &ObjectSnapshot) -> Option<Shape> {
    match object {
        ObjectSnapshot::Primitive(primitive) => Some(primitive.shape),
        _ => None,
    }
}

/// What a copy of `id` through `handle` with `value` must leave: the original as it was, one
/// new object directly above it that equals the plain result but for its id, one step named
/// `label`, and after one undo the original alone.
fn assert_copy(
    document: &Document,
    id: NodeId,
    handle: ParamHandle,
    value: ParamValue,
    label: &str,
) -> ObjectSnapshot {
    let start = document.object(id).expect("exists");
    let result = apply_param(&start, value);
    assert_ne!(result, start, "{label}: the value changes the shape");
    let steps = document.own_step_count();
    let copy = commit_param_copy(document, handle, &result).expect("a copy");
    assert_eq!(
        document.object(id),
        Some(start.clone()),
        "{label}: original"
    );
    let ids = document.object_ids();
    let at = ids.iter().position(|&o| o == id).expect("original listed");
    assert_eq!(ids.get(at + 1), Some(&copy), "{label}: directly above");
    assert_eq!(ids.len(), 2, "{label}: one new object");
    let ObjectSnapshot::Primitive(mut written) = document.object(copy).expect("copy") else {
        panic!("{label}: a primitive copy");
    };
    written.id = id;
    assert_eq!(ObjectSnapshot::Primitive(written), result, "{label}: value");
    let new_steps = document.own_steps_from(steps);
    assert_eq!(new_steps.len(), 1, "{label}: one step");
    assert_eq!(new_steps[0].label, label);
    document.undo_step(new_steps[0].step).expect("undo");
    assert_eq!(
        document.object_ids(),
        vec![id],
        "{label}: undo leaves the original"
    );
    assert_eq!(
        document.object(id),
        Some(start),
        "{label}: original after undo"
    );
    result
}

/// A shape maker, the handle, the value and the step label of one copy case.
type Case = (
    fn(&Document) -> NodeId,
    ParamHandle,
    ParamValue,
    &'static str,
);

#[test]
fn each_parameter_family_writes_one_copy_under_its_own_label() {
    let cases: [Case; 6] = [
        (
            ellipse,
            ParamHandle::Curve,
            ParamValue::Curve(0.4),
            "copy_with_curve",
        ),
        (
            ellipse,
            ParamHandle::ArcStart,
            ParamValue::ArcAngle(ArcEnd::Start, Angle::from_radians(0.5)),
            "copy_with_arc",
        ),
        (
            rect,
            ParamHandle::CornerRadius(Corner::Tr),
            ParamValue::Radius(Length::from_mm(4.0)),
            "copy_with_corner_radius",
        ),
        (
            rect,
            ParamHandle::CornerRadius(Corner::Tl),
            ParamValue::CornerRadius(Corner::Tl, Length::from_mm(3.0)),
            "copy_with_corner_radius",
        ),
        (
            star,
            ParamHandle::InnerRadius,
            ParamValue::Ratio(InnerRatio::new(0.3).expect("ratio")),
            "copy_with_inner_ratio",
        ),
        (
            polygon,
            ParamHandle::Rounded,
            ParamValue::Rounded(0.6),
            "copy_with_rounding",
        ),
    ];
    for (make, handle, value, label) in cases {
        let document = Document::new(1);
        let id = make(&document);
        assert_copy(&document, id, handle, value, label);
    }
}

/// Criterion 10: an arc on a whole ellipse makes the copy an arc, the original stays whole.
#[test]
fn an_arc_copy_of_a_whole_ellipse_is_an_arc_and_the_original_stays_whole() {
    let document = Document::new(1);
    let id = ellipse(&document);
    let start = document.object(id).expect("exists");
    let result = apply_param(
        &start,
        ParamValue::ArcAngle(ArcEnd::End, Angle::from_radians(2.0)),
    );
    let copy = commit_param_copy(&document, ParamHandle::ArcEnd, &result).expect("a copy");
    let arc_of = |id| match document.primitive(id).expect("primitive").shape {
        Shape::Ellipse { arc, .. } => arc,
        _ => panic!("an ellipse"),
    };
    assert!(arc_of(copy).is_some());
    assert!(arc_of(id).is_none());
}

/// Criterion 8: the copy gets the limited value, as the plain drag would.
#[test]
fn a_limited_curve_and_corner_radius_reach_the_copy_limited() {
    let document = Document::new(1);
    let id = rect(&document);
    let result = assert_copy(
        &document,
        id,
        ParamHandle::CornerRadius(Corner::Br),
        ParamValue::Radius(Length::from_mm(500.0)),
        "copy_with_corner_radius",
    );
    let Some(Shape::Rect { corner_radii, .. }) = shape_of(&result) else {
        panic!("a rectangle");
    };
    assert_eq!(
        corner_radii.tl,
        Length::from_mm(10.0),
        "half the shorter side"
    );

    let document = Document::new(1);
    let id = polygon(&document);
    let result = assert_copy(
        &document,
        id,
        ParamHandle::Curve,
        ParamValue::Curve(-50.0),
        "copy_with_curve",
    );
    let Some(Shape::Polygon { curve, .. }) = shape_of(&result) else {
        panic!("a polygon");
    };
    assert!(curve.get() > -50.0, "limited at k_min");
}

/// Criteria 11 (V5) and 12: a result equal to the original makes a plain copy in one step, and
/// the copy's history begins with "Duplicate of" the original.
#[test]
fn an_unchanged_result_makes_a_plain_copy_whose_history_starts_at_its_original() {
    let document = Document::new(1);
    let id = star(&document);
    let start = document.object(id).expect("exists");
    let steps = document.own_step_count();
    let copy = commit_param_copy(&document, ParamHandle::InnerRadius, &start).expect("a copy");
    let new_steps = document.own_steps_from(steps);
    assert_eq!(new_steps.len(), 1);
    assert_eq!(new_steps[0].label, "copy_with_inner_ratio");
    let rows = document.object_history_page(copy, 0, document.object_history_len(copy));
    // Newest first: the copy's own first row, above the rows it inherits from its original.
    let first = rows.first().expect("a row");
    assert_eq!(first.class, ObjectRowClass::CloneStart { source: id });
}

/// Criterion 16: an original that is gone writes nothing and makes no copy.
#[test]
fn a_gone_original_writes_nothing() {
    let document = Document::new(1);
    let id = star(&document);
    let start = document.object(id).expect("exists");
    document.delete_objects(&[id]).expect("delete");
    let steps = document.own_step_count();
    assert_eq!(
        commit_param_copy(&document, ParamHandle::InnerRadius, &start),
        None
    );
    assert_eq!(document.own_step_count(), steps);
    assert_eq!(document.object_ids(), Vec::<NodeId>::new());
}
