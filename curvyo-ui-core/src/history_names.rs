//! The user-facing name of every commit label (`specs/0020-undo-redo` criterion 41, the table "One
//! interaction, one step"). A command that commits under a label this table does not know fails
//! the guard test in `tests/history_names_guard.rs`, so a new command cannot ship without a name.

/// The name shown for a commit label that is not in the table (an old file, `specs/0020-undo-redo`
/// criterion 36).
pub const UNKNOWN_OPERATION_NAME: &str = "Edit";

/// Every commit label and the name the maker reads for it.
pub const OPERATION_NAMES: &[(&str, &str)] = &[
    ("add_page", "Add page"),
    ("delete_page", "Delete page"),
    ("rename_page", "Rename page"),
    ("resize_page", "Resize page"),
    ("move_page", "Move page"),
    ("reorder_pages", "Reorder pages"),
    // `specs/0093-laser-workbench-shell` criteria 2, 3, 9, 10 and 13.
    ("add_manufacturing", "Add manufacturing"),
    ("delete_manufacturing", "Delete manufacturing"),
    ("set_work_area", "Change work area"),
    // `specs/0098-page-kinds` criteria 3, 4 and 12.
    ("add_sketch_page", "Add sketch page"),
    ("add_output_page", "Add laser page"),
    ("delete_placement", "Delete placement"),
    ("delete_fixture", "Remove fixture"),
    ("set_fixture_visible", "Show or hide fixture"),
    ("set_fixture_params", "Change fixture"),
    // `specs/0102-laser-fixtures` criteria 2 to 5, 14, 19 and 25 (adrs.md section 4).
    ("add_grid", "Add grid"),
    ("add_holder", "Add holder"),
    ("add_clamp", "Add clamp"),
    ("add_stop", "Add stop"),
    ("add_screw", "Add screw"),
    ("resize_fixture", "Resize fixture"),
    ("set_grid_spacing", "Set grid spacing"),
    ("set_grid_major", "Set grid major"),
    ("set_tape_edge", "Set tape edge"),
    ("set_grid_origin", "Set grid origin"),
    ("rename_slot", "Rename slot"),
    ("set_slot_shape", "Set slot shape"),
    ("set_slot_position", "Set slot position"),
    ("set_slot_size", "Set slot size"),
    ("set_fixtures_snap_on", "Snap to fixtures on"),
    ("set_fixtures_snap_off", "Snap to fixtures off"),
    ("rename_fixture", "Rename fixture"),
    ("set_fixture_active", "Set fixture active"),
    ("set_fixture_inactive", "Set fixture inactive"),
    ("move_fixture", "Move fixture"),
    ("rotate_fixture", "Rotate fixture"),
    ("add_slot", "Add slot"),
    ("remove_slot", "Remove slot"),
    ("set_slot", "Change slot"),
    ("move_to_slot", "Move to slot"),
    // `specs/0103-fixture-jobs` criteria 2, 4 and 9 (adrs.md section 7): fixed per label, no job
    // name.
    ("add_fixture_job", "Add fixture job"),
    ("rename_fixture_job", "Rename fixture job"),
    ("delete_fixture_job", "Delete fixture job"),
    ("set_fixture_job_show_on", "Show on"),
    ("set_fixture_job_show_off", "Show off"),
    ("set_fixture_job_output_on", "Output on"),
    ("set_fixture_job_output_off", "Output off"),
    ("move_fixture_job", "Move fixture job"),
    ("assign_fixture_target", "Assign to fixture job"),
    ("remove_fixture_target", "Remove from fixture job"),
    // `specs/0101-laser-operations` criteria 2 to 8, 15, 16, 23 and 27; the session names an
    // assignment after its operation ("Assign to Cut 1").
    ("add_operation", "Add operation"),
    ("assign_to_operation", "Assign to operation"),
    ("remove_from_operation", "Remove from operation"),
    ("rename_operation", "Rename operation"),
    ("set_operation_show", "Show or hide operation"),
    ("set_operation_output", "Output on or off"),
    ("set_operation_margin", "Set margin"),
    ("delete_operation", "Delete operation"),
    ("move_operation", "Move operation"),
    ("cuts_last_on", "Cuts last on"),
    ("cuts_last_off", "Cuts last off"),
    // `specs/0028-material-test-library` adrs.md section 7: the grid, the settings of an
    // operation and the material edits of the open project.
    ("add_test_grid", "Add test grid"),
    ("edit_test_grid", "Edit test grid"),
    ("rebuild_test_grid", "Rebuild test grid"),
    ("mark_test_cell", "Mark cell"),
    ("unmark_test_cell", "Unmark cell"),
    ("rename_test_grid", "Rename test grid"),
    ("delete_test_grid_row", "Delete test grid row"),
    ("use_material", "Use material"),
    ("use_own_numbers", "Use own numbers"),
    ("edit_operation_settings", "Edit settings"),
    ("reset_to_material", "Reset to material"),
    ("save_as_material", "Save as material"),
    ("edit_material", "Edit material"),
    ("rename_material", "Rename material"),
    ("delete_material", "Delete material"),
    ("remove_missing_items", "Remove missing items"),
    // `specs/0026-machine-profile` criteria 3, 5 and 7 (UX notes §2).
    ("pick_profile", "Pick profile"),
    ("reset_profile", "Reset from profile"),
    ("save_as_profile", "Save as profile"),
    ("edit_profile_name", "Edit name"),
    ("edit_origin_corner", "Edit origin corner"),
    ("edit_max_feed", "Edit max feed"),
    ("edit_max_power", "Edit max power"),
    ("edit_laser_mode", "Edit laser mode"),
    ("add_sketch", "Add sketch"),
    ("edit_sketch", "Edit sketch"),
    ("convert_sketch_to_path", "Sketch to path"),
    // `specs/0099-sketch-pages` criteria 8, 15 and 17.
    ("place_sketch_clone", "Place sketch clone"),
    ("place_linked_clones", "Place linked clones"),
    ("place_page", "Place page"),
    ("place_on_page", "Place on page"),
    ("convert_sketch_page", "Convert sketch to paths"),
    ("delete_sketch_page", "Delete sketch page"),
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
    ("change_taper", "Change taper"),
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
    // `specs/0061-align-and-distribute` criterion 12.
    ("align_left", "Align left"),
    ("align_centre_h", "Align horizontal centres"),
    ("align_right", "Align right"),
    ("align_top", "Align top"),
    ("align_centre_v", "Align vertical centres"),
    ("align_bottom", "Align bottom"),
    ("distribute_h", "Distribute horizontally"),
    ("distribute_v", "Distribute vertically"),
    // `specs/0061-align-and-distribute` criteria 17 and 21.
    ("align_left_page", "Align left to page"),
    ("align_centre_h_page", "Align horizontal centres to page"),
    ("align_right_page", "Align right to page"),
    ("align_top_page", "Align top to page"),
    ("align_centre_v_page", "Align vertical centres to page"),
    ("align_bottom_page", "Align bottom to page"),
    ("distribute_h_page", "Distribute horizontally on page"),
    ("distribute_v_page", "Distribute vertically on page"),
    ("arrange_grid", "Arrange in a grid"),
    ("set_corner_radius", "Change corner radius"),
    ("set_corner_radii", "Change corner radius"),
    // `specs/0021-ellipse-arcs-and-shaping` criterion 59.
    ("set_ellipse_arc", "Change arc"),
    ("set_curve", "Change curve"),
    ("set_curve_kind", "Change curve type"),
    ("set_spiro_transform", "Transform Spiro path"),
    ("set_width_point", "Change width point"),
    ("remove_width_point", "Remove width point"),
    ("reset_width_profile", "Reset width profile"),
    ("set_width_profile", "Change width profile"),
    ("convert_to_paths", "Object to path"),
    ("convert_curve_to_bezier", "Curve to Bézier"),
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
    ("set_document_snapping", "Change snapping"),
    ("set_page_grid", "Change grid"),
    ("combine_paths", "Combine"),
    ("split", "Split"),
    ("break_apart", "Break apart"),
    ("cut_at_crossings", "Cut"),
    ("fracture", "Fracture"),
    ("flatten", "Flatten"),
    // `specs/0068-laserable-lettering` criterion 20.
    ("weld", "Weld"),
    // `specs/0085-stroke-to-path` criterion 21.
    ("stroke_to_path", "Stroke to path"),
    // `specs/0071-pattern-fill` criterion 24.
    ("convert_pattern", "Convert pattern to paths"),
    // `specs/0071-pattern-fill` UX notes §6: the pattern edits of the Fill section.
    ("fill_pattern", "Fill: Pattern"),
    ("pattern_generator_hatch", "Pattern: Hatch"),
    ("pattern_generator_crosshatch", "Pattern: Crosshatch"),
    ("pattern_generator_grid", "Pattern: Grid"),
    ("pattern_generator_dots", "Pattern: Dots"),
    ("pattern_generator_honeycomb", "Pattern: Honeycomb"),
    ("pattern_generator_brick_wall", "Pattern: Brick wall"),
    ("pattern_generator_leaf_forest", "Pattern: Leaf forest"),
    ("pattern_generator_sea_waves", "Pattern: Sea waves"),
    ("pattern_generator_waves", "Pattern: Waves"),
    (
        "pattern_generator_interpolation_waves",
        "Pattern: Interpolation waves",
    ),
    ("pattern_spacing", "Pattern spacing"),
    ("pattern_angle", "Pattern angle"),
    ("pattern_second_angle", "Pattern second angle"),
    ("pattern_diameter", "Pattern diameter"),
    ("pattern_cell_size", "Pattern cell size"),
    ("pattern_wall", "Pattern wall"),
    ("pattern_width", "Pattern width"),
    ("pattern_height", "Pattern height"),
    ("pattern_joint", "Pattern joint"),
    ("pattern_leaf_size", "Pattern leaf size"),
    ("pattern_size_jitter", "Pattern size jitter"),
    ("pattern_rotation_jitter", "Pattern rotation jitter"),
    ("pattern_density", "Pattern density"),
    ("pattern_scale_radius", "Pattern scale radius"),
    ("pattern_rings", "Pattern rings"),
    ("pattern_ring_spacing", "Pattern ring spacing"),
    ("pattern_wavelength", "Pattern wavelength"),
    ("pattern_amplitude", "Pattern amplitude"),
    ("pattern_lines_count", "Pattern lines"),
    ("pattern_margin", "Pattern margin"),
    ("pattern_mode_lines", "Pattern: Lines"),
    ("pattern_mode_shapes", "Pattern: Shapes"),
    ("pattern_seed", "Pattern: new seed"),
    ("pattern_whole_pieces", "Pattern: whole pieces"),
    // `specs/0025-raster-trace` criterion 13.
    ("trace_image", "Trace image"),
    // `specs/0088-trace-recognition` criterion 6.
    ("recognise_text", "Recognise text"),
    ("import_svg", "Import SVG"),
    ("import_image", "Import image"),
    ("import_images", "Import images"),
    ("import", "Import"),
    // `specs/0087-image-clipboard-and-drop` criterion 1.
    ("paste_image", "Paste image"),
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
    ("create_dimension", "Dimension"),
    ("move_dimension", "Move dimension"),
    ("edit_dimension_nodes", "Edit nodes"),
    ("convert_dimension", "Convert to path"),
    ("load_template", "Template"),
    // `specs/0069-tiling` criterion 37.
    ("tile", "Tile"),
    ("edit_tiling", "Edit tiling"),
    ("release_tiling", "Release tiling"),
];

/// The labels the engine writes itself. They are no step of the maker's: the setup of a new
/// document, an undo, a redo, a wipe. The history list shows an undo and a redo as statuses of
/// other rows, a wipe as its own row; an object undo, an object redo and the wipe of one object are
/// rows named after the step they name (`specs/0041-object-history`).
pub const RESERVED_LABELS: &[&str] = &[
    curvyo_document_core::LABEL_NEW_DOCUMENT,
    curvyo_document_core::LABEL_MIGRATE_FORMAT,
    curvyo_document_core::LABEL_REFRESH_MATERIAL_NUMBERS,
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
    fn the_align_and_distribute_commands_are_named() {
        assert_eq!(operation_name("align_left"), "Align left");
        assert_eq!(operation_name("align_centre_h"), "Align horizontal centres");
        assert_eq!(operation_name("distribute_v"), "Distribute vertically");
    }

    #[test]
    fn the_page_and_grid_commands_are_named() {
        assert_eq!(operation_name("align_left_page"), "Align left to page");
        assert_eq!(
            operation_name("align_centre_v_page"),
            "Align vertical centres to page"
        );
        assert_eq!(
            operation_name("distribute_h_page"),
            "Distribute horizontally on page"
        );
        assert_eq!(
            operation_name("distribute_v_page"),
            "Distribute vertically on page"
        );
        assert_eq!(operation_name("arrange_grid"), "Arrange in a grid");
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
    fn the_material_test_library_steps_are_named_and_the_save_refresh_is_reserved() {
        use curvyo_document_core as dc;
        for (label, name) in [
            (dc::ADD_TEST_GRID_COMMIT_LABEL, "Add test grid"),
            (dc::EDIT_TEST_GRID_COMMIT_LABEL, "Edit test grid"),
            (dc::REBUILD_TEST_GRID_COMMIT_LABEL, "Rebuild test grid"),
            (dc::MARK_TEST_CELL_COMMIT_LABEL, "Mark cell"),
            (dc::UNMARK_TEST_CELL_COMMIT_LABEL, "Unmark cell"),
            (dc::RENAME_TEST_GRID_COMMIT_LABEL, "Rename test grid"),
            (
                dc::DELETE_TEST_GRID_ROW_COMMIT_LABEL,
                "Delete test grid row",
            ),
            (dc::USE_MATERIAL_COMMIT_LABEL, "Use material"),
            (dc::USE_OWN_NUMBERS_COMMIT_LABEL, "Use own numbers"),
            (dc::EDIT_OPERATION_SETTINGS_COMMIT_LABEL, "Edit settings"),
            (dc::RESET_TO_MATERIAL_COMMIT_LABEL, "Reset to material"),
            (dc::SAVE_AS_MATERIAL_COMMIT_LABEL, "Save as material"),
            (dc::EDIT_MATERIAL_COMMIT_LABEL, "Edit material"),
            (dc::RENAME_MATERIAL_COMMIT_LABEL, "Rename material"),
            (dc::DELETE_MATERIAL_COMMIT_LABEL, "Delete material"),
        ] {
            assert_eq!(operation_name(label), name, "{label}");
            assert!(
                crate::step_scope::FREE_STEPS.contains(&label),
                "{label} is in one step-scope list"
            );
        }
        assert!(RESERVED_LABELS.contains(&dc::LABEL_REFRESH_MATERIAL_NUMBERS));
        assert_eq!(
            known_operation_name(dc::LABEL_REFRESH_MATERIAL_NUMBERS),
            None
        );
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
