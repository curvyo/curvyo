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
    // `specs/0074-text-objects` criteria 7 and 13.
    ("add_text", "Add text"),
    ("edit_text", "Edit text"),
    ("change_font", "Change font"),
    ("change_text_size", "Change text size"),
    // `specs/0079-text-layout-controls` criteria 4 and 12.
    ("change_text_alignment", "Change text alignment"),
    ("change_letter_spacing", "Change letter spacing"),
    ("change_word_spacing", "Change word spacing"),
    ("change_line_spacing", "Change line spacing"),
    ("change_character_rotation", "Change character rotation"),
    ("change_writing_mode", "Change writing mode"),
    ("change_glyph_orientation", "Change glyph orientation"),
    ("change_kerning", "Change kerning"),
    ("put_text_on_path", "Put text on path"),
    ("put_text_on_paths", "Put text on paths"),
    ("release_from_path", "Release from path"),
    ("change_text_side", "Change text side"),
    ("change_line_on_path", "Change line on path"),
    ("move_text_on_path", "Move text on path"),
    ("change_text_on_path", "Change text on path"),
    ("change_distance_on_path", "Change distance on path"),
    ("mirror_text_on_path", "Mirror text on path"),
    ("flip_letters", "Flip"),
    ("clear_kerning", "Clear kerning"),
    ("change_char_rotation", "Change per-character rotation"),
    ("clear_char_rotation", "Clear per-character rotation"),
    ("translate_objects", "Move"),
    ("duplicate_objects", "Duplicate"),
    // `specs/0042-history-branches` criteria 10 and 22; the row adds the version's id.
    ("go_to_version", "Go to version"),
    ("clone_version", "Clone version"),
    ("linked_clone", "Linked clone"),
    ("unlink_clone", "Unlink clone"),
    ("own_placement", "Own placement"),
    ("follow_placement", "Follow original's placement"),
    ("own_style", "Own style"),
    ("original_style", "Original's style"),
    ("spread_colours", "Spread colours"),
    ("remove_orphan_clones", "Remove orphan clones"),
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
    // `specs/0056-mirror-and-negative-scale` criterion 29.
    ("flip_horizontal", "Flip horizontally"),
    ("flip_vertical", "Flip vertically"),
    ("set_corner_radius", "Change corner radius"),
    ("set_corner_radii", "Change corner radius"),
    // `specs/0021-ellipse-arcs-and-shaping` criterion 59.
    ("set_ellipse_arc", "Change arc"),
    ("set_curve", "Change curve"),
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
    ("split", "Split"),
    ("break_apart", "Break apart"),
    ("cut_at_crossings", "Cut"),
    ("fracture", "Fracture"),
    ("flatten", "Flatten"),
    // `specs/0068-laserable-lettering` criterion 20.
    ("weld", "Weld"),
    ("offset_path", "Offset"),
    ("group", "Group"),
    ("ungroup", "Ungroup"),
    ("rename_node", "Rename"),
    ("set_node_visible", "Hide or show"),
    ("set_node_locked", "Lock or unlock"),
    ("set_node_opacity", "Change opacity"),
    ("move_nodes", "Reorder"),
    ("new_layer", "New layer"),
    ("delete_layer", "Delete layer"),
    // `specs/0046-document-shapes` criterion 28.
    ("set_guide", "Guide"),
    ("load_template", "Template"),
];

/// The labels the engine writes itself. They are no step of the maker's: the setup of a new
/// document, an undo, a redo, a wipe. The history list shows an undo and a redo as statuses of
/// other rows, a wipe as its own row; an object undo, an object redo and the wipe of one object are
/// rows named after the step they name (`specs/0041-object-history`).
pub const RESERVED_LABELS: &[&str] = &[
    curvyo_document_core::LABEL_NEW_DOCUMENT,
    curvyo_document_core::LABEL_UNDO,
    curvyo_document_core::LABEL_REDO,
    curvyo_document_core::WIPE_LABEL,
    curvyo_document_core::LABEL_OBJECT_UNDO,
    curvyo_document_core::LABEL_OBJECT_REDO,
    curvyo_document_core::LABEL_WIPE_OBJECT,
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
    fn the_guide_commands_are_named_guide_and_template() {
        assert_eq!(operation_name("set_guide"), "Guide");
        assert_eq!(operation_name("load_template"), "Template");
    }

    #[test]
    fn a_known_label_has_its_name_and_an_unknown_one_is_edit() {
        assert_eq!(operation_name("translate_objects"), "Move");
        assert_eq!(operation_name("boolean_union"), "Union");
        assert_eq!(operation_name("offset_path"), "Offset");
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
