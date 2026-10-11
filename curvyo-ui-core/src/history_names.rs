//! The user-facing name of every commit label (`specs/0020-undo-redo` criterion 41, the table "One
//! interaction, one step"). A command that commits under a label this table does not know fails
//! the guard test in `tests/history_names_guard.rs`, so a new command cannot ship without a name.

/// The name shown for a commit label that is not in the table (an old file, `specs/0020-undo-redo`
/// criterion 36).
pub const UNKNOWN_OPERATION_NAME: &str = "Edit";

/// Every commit label and the name the maker reads for it.
pub const OPERATION_NAMES: &[(&str, &str)] = &[
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
    ("transform_resize", "Resize"),
    ("set_point_count", "Change point count"),
    ("set_inner_ratio", "Change inner ratio"),
    ("rotate_object", "Rotate"),
    ("transform_rotate", "Rotate"),
    ("skew_path", "Skew"),
    ("transform_skew", "Skew"),
    ("transform_objects", "Transform"),
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
    ("set_document_background", "Change background"),
    ("combine_paths", "Combine"),
    ("break_apart", "Break apart"),
    ("group", "Group"),
    ("ungroup", "Ungroup"),
];

/// The labels the engine writes itself. They are no step of the maker's: the setup of a new
/// document, an undo, a redo, a wipe. The history list shows the last two as statuses of other
/// rows and the wipe as its own row.
pub const RESERVED_LABELS: &[&str] = &[
    curvyo_document_core::LABEL_NEW_DOCUMENT,
    curvyo_document_core::LABEL_UNDO,
    curvyo_document_core::LABEL_REDO,
    curvyo_document_core::WIPE_LABEL,
];

/// The name for `label`, or `None` when the table does not know it.
#[must_use]
pub fn known_operation_name(label: &str) -> Option<&'static str> {
    OPERATION_NAMES
        .iter()
        .find(|(known, _)| *known == label)
        .map(|(_, name)| *name)
}

/// The name for `label`: the table's, or [`UNKNOWN_OPERATION_NAME`].
#[must_use]
pub fn operation_name(label: &str) -> &'static str {
    known_operation_name(label).unwrap_or(UNKNOWN_OPERATION_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_label_has_its_name_and_an_unknown_one_is_edit() {
        assert_eq!(operation_name("translate_objects"), "Move");
        assert_eq!(operation_name("boolean_union"), "Union");
        assert_eq!(operation_name("something_old"), "Edit");
        assert_eq!(known_operation_name("something_old"), None);
    }

    #[test]
    fn no_label_is_listed_twice() {
        let mut labels: Vec<_> = OPERATION_NAMES.iter().map(|(label, _)| *label).collect();
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), count);
        assert!(
            RESERVED_LABELS
                .iter()
                .all(|l| known_operation_name(l).is_none())
        );
    }
}
