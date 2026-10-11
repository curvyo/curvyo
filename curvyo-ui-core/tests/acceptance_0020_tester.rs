//! Independent tester tests for the texts of `specs/0020-undo-redo` (criteria 1, 2, 4, 19, 24, 36,
//! 41): the operation-name table of the specification, the object summaries and the notices.
//! Written from the specification before the implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(missing_docs, clippy::doc_markdown, clippy::assert_is_empty)]

use curvyo_document_core::{
    Direction, Document, Length, NodeId, ObjectReport, ObjectResult, ObjectShape, Point,
    RectBounds, RestoreReport, Role, SkipReason,
};
use curvyo_ui_core::{nothing_notice, objects_summary, operation_name, restore_notice};

/// The table "One interaction, one step" of the specification: commit label, operation name.
const SPEC_TABLE: &[(&str, &str)] = &[
    ("create_path", "Draw path"),
    ("extend_path", "Extend path"),
    ("connect_paths", "Connect paths"),
    ("close_path", "Close path"),
    ("create_rect", "Draw rectangle"),
    ("create_ellipse", "Draw ellipse"),
    ("create_polygon", "Draw polygon"),
    ("create_star", "Draw star"),
    ("translate_objects", "Move"),
    ("duplicate_objects", "Duplicate"),
    ("resize_path", "Resize"),
    ("resize_rect", "Resize"),
    ("resize_ellipse", "Resize"),
    ("resize_star_frame", "Resize"),
    ("set_rect_bounds", "Resize"),
    ("set_ellipse_frame", "Resize"),
    ("set_star_frame", "Resize"),
    ("set_point_count", "Change point count"),
    ("set_inner_ratio", "Change inner ratio"),
    ("rotate_object", "Rotate"),
    ("set_corner_radius", "Change corner radius"),
    ("set_corner_radii", "Change corner radius"),
    ("convert_to_paths", "Object to path"),
    ("delete_objects", "Delete"),
    ("edit_style", "Change style"),
    ("resize_document", "Resize document"),
    ("fit_document_to_content", "Fit document to content"),
    ("set_display_unit", "Change display unit"),
    ("move_anchors", "Move nodes"),
    ("set_handle", "Move handle"),
    ("convert_anchor_kind", "Change node type"),
    ("insert_anchor", "Add node"),
    ("delete_anchors", "Delete nodes"),
    ("set_segment_line", "Straighten segment"),
    ("set_segment_curve", "Curve segment"),
    ("bend_segment", "Bend segment"),
    ("join_endpoints", "Join nodes"),
    ("split_at_anchor", "Split path"),
    ("boolean_union", "Union"),
    ("boolean_difference", "Difference"),
    ("boolean_intersection", "Intersection"),
    ("boolean_exclusion", "Exclusion"),
    ("boolean_reverse_difference", "Reverse difference"),
    ("combine_paths", "Combine"),
    ("break_apart", "Break apart"),
];

#[test]
fn every_label_of_the_specification_table_has_its_name() {
    for (label, name) in SPEC_TABLE {
        assert_eq!(operation_name(label), *name, "{label}");
    }
}

#[test]
fn skew_and_background_have_the_names_the_table_gives() {
    // The skew label is "the label the skew commit uses today"; the table names the operation.
    assert_eq!(operation_name("skew_path"), "Skew");
    assert_eq!(operation_name("transform_skew"), "Skew");
    assert_eq!(
        operation_name("set_document_background"),
        "Change background"
    );
}

#[test]
fn an_unknown_label_is_named_edit_and_odd_labels_do_not_panic() {
    for label in [
        "",
        "frobnicate",
        "undo",
        "Translate_objects",
        "translate_objects;s=1",
        "\u{0}",
        "é",
    ] {
        let name = operation_name(label);
        assert!(!name.is_empty());
    }
    assert_eq!(operation_name("frobnicate"), "Edit");
    assert_eq!(operation_name(""), "Edit");
}

#[test]
fn object_summaries_count_by_shape_and_mix_to_objects() {
    use ObjectShape::{Ellipse, Path, Polygon, Rectangle, Star};
    assert_eq!(
        objects_summary(&[]),
        None,
        "a step that touched no object has no summary"
    );
    assert_eq!(objects_summary(&[Rectangle]).unwrap(), "1 rectangle");
    assert_eq!(
        objects_summary(&[Rectangle, Rectangle]).unwrap(),
        "2 rectangles"
    );
    assert_eq!(objects_summary(&[Path, Path, Path]).unwrap(), "3 paths");
    assert_eq!(objects_summary(&[Path]).unwrap(), "1 path");
    assert_eq!(objects_summary(&[Ellipse]).unwrap(), "1 ellipse");
    assert_eq!(objects_summary(&[Polygon, Polygon]).unwrap(), "2 polygons");
    assert_eq!(objects_summary(&[Star]).unwrap(), "1 star");
    assert_eq!(
        objects_summary(&[Rectangle, Path, Star, Ellipse, Polygon]).unwrap(),
        "5 objects"
    );
    assert_eq!(objects_summary(&[Rectangle, Path]).unwrap(), "2 objects");
}

#[test]
fn nothing_notices() {
    assert_eq!(nothing_notice(Direction::Undo), "Nothing to undo.");
    assert_eq!(nothing_notice(Direction::Redo), "Nothing to redo.");
}

fn ids(count: usize) -> Vec<NodeId> {
    let d = Document::new(1);
    (0..count)
        .map(|n| {
            let x = f64::from(u32::try_from(n).unwrap()) * 20.0;
            d.create_rect(RectBounds {
                origin: Point::new(x, 0.0),
                width: Length::from_mm(10.0),
                height: Length::from_mm(10.0),
            })
        })
        .collect()
}

fn report(
    direction: Direction,
    role: Role,
    results: &[ObjectResult],
    wrote: bool,
) -> RestoreReport {
    RestoreReport {
        direction,
        objects: results
            .iter()
            .zip(ids(results.len()))
            .map(|(result, id)| ObjectReport {
                id,
                shape: ObjectShape::Rectangle,
                role,
                result: *result,
            })
            .collect(),
        root: None,
        wrote,
    }
}

#[test]
fn restore_notices_follow_the_texts_of_the_specification() {
    let ok = ObjectResult::Restored;
    let kept = ObjectResult::Skipped(SkipReason::ChangedLater);
    let gone = ObjectResult::Skipped(SkipReason::DeletedLater);
    // Criterion 1 and 2.
    let all = report(Direction::Undo, Role::Changed, &[ok, ok, ok], true);
    assert_eq!(
        restore_notice("Move", &all, Some("3 rectangles"), false),
        "Undid: Move (3 rectangles)."
    );
    let redo = report(Direction::Redo, Role::Changed, &[ok], true);
    assert_eq!(
        restore_notice("Move", &redo, Some("1 rectangle"), false),
        "Redid: Move (1 rectangle)."
    );
    // No objects: no parentheses.
    let doc = RestoreReport {
        direction: Direction::Undo,
        objects: vec![],
        root: Some(ok),
        wrote: true,
    };
    assert_eq!(
        restore_notice("Resize document", &doc, None, false),
        "Undid: Resize document."
    );
    // Criterion 24: the tail replaces the full stop, no second pair of parentheses.
    assert_eq!(
        restore_notice("Delete", &all, Some("2 rectangles"), true),
        "Undid: Delete (2 rectangles), off screen."
    );
    // Criterion 19: partly skipped.
    let partly = report(Direction::Undo, Role::Changed, &[ok, kept, ok], true);
    assert_eq!(
        restore_notice("Move", &partly, Some("3 rectangles"), false),
        "Undid: Move. 1 of 3 objects kept later changes by someone else."
    );
    // Whole step skipped.
    let nothing = report(Direction::Undo, Role::Changed, &[kept, gone], false);
    assert_eq!(
        restore_notice("Move", &nothing, Some("2 rectangles"), false),
        "Could not undo Move: everything it changed was changed or deleted by someone else since."
    );
    let nothing_redo = report(Direction::Redo, Role::Changed, &[gone], false);
    assert_eq!(
        restore_notice("Move", &nothing_redo, None, false),
        "Could not redo Move: everything it changed was changed or deleted by someone else since."
    );
    // Criterion 18: a created object a peer changed stays.
    let created = report(Direction::Undo, Role::Created, &[kept], false);
    assert_eq!(
        restore_notice("Draw rectangle", &created, Some("1 rectangle"), false),
        "Not undone: someone else changed this object after you drew it."
    );
}

#[test]
fn a_partly_skipped_notice_counts_the_root_register_too() {
    let root_kept = RestoreReport {
        direction: Direction::Undo,
        objects: vec![],
        root: Some(ObjectResult::Partial),
        wrote: true,
    };
    let text = restore_notice("Resize document", &root_kept, None, false);
    assert!(text.starts_with("Undid: Resize document."), "{text}");
    assert!(text.contains("1 of 1"), "{text}");
}
