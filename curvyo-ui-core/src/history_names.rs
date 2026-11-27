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
    // `specs/0113-fixture-sets-and-pins` UX section 11; the per-kind `add_<kind>` names above stay
    // so old history rows read.
    ("add_fixture_set", "Add fixture set"),
    ("rename_fixture_set", "Rename fixture set"),
    ("remove_fixture_set", "Remove fixture set"),
    ("move_fixture_set", "Move fixture set"),
    ("set_fixture_set_on_page", "Set fixture set on page"),
    ("set_fixture_set_off_page", "Set fixture set off page"),
    ("add_fixture", "Add fixture"),
    ("add_job_fixture", "Add job fixture"),
    ("lock_fixture", "Lock fixture"),
    ("unlock_fixture", "Unlock fixture"),
    ("lock_fixture_set", "Lock fixture set"),
    ("unlock_fixture_set", "Unlock fixture set"),
    ("apply_page_fixtures", "Apply fixtures from page"),
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
    ("detach_part", "Detach part"),
    ("attach_part", "Attach part"),
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
    ("use_material", "Use material"),
    ("use_pick", "Use pick"),
    ("use_own_numbers", "Use own numbers"),
    ("edit_operation_settings", "Edit settings"),
    ("reset_to_material", "Reset to material"),
    ("edit_material", "Edit material"),
    ("rename_material", "Rename material"),
    ("delete_material", "Delete material"),
    ("remove_missing_items", "Remove missing items"),
    // `specs/0139-laser-cut-path-options` criterion 12: the tabs of an item in one operation.
    ("add_tab", "Add tab"),
    ("move_tab", "Move tab"),
    ("delete_tab", "Delete tab"),
    ("tab_width", "Tab width"),
    ("own_tabs", "Own tabs"),
    ("operations_tabs", "Back to the operation's"),
    // `specs/0139` criterion 23: the path options, start points and directions of an item.
    ("own_path_options", "Own path options"),
    ("operations_path_options", "Back to the operation's"),
    ("move_start_point", "Move start point"),
    ("automatic_start", "Automatic start"),
    ("direction", "Direction"),
    // `specs/0112-test-grid-and-materials` plan.md (UX section 11): the grid object, the
    // Materials tab and its tests.
    ("place_test_grid", "Place test grid"),
    ("add_material", "Add material"),
    ("add_material_set", "Add set"),
    ("delete_material_set", "Delete set"),
    ("judge_test", "Mark pick"),
    ("unmark_pick", "Unmark pick"),
    ("edit_test_note", "Edit note"),
    ("set_test_photo", "Add photo"),
    ("crop_test_photo", "Crop photo"),
    ("remove_test_photo", "Remove photo"),
    ("delete_test", "Delete test"),
    // `specs/0026-machine-profile` criteria 3, 5 and 7 (UX notes §2).
    ("pick_profile", "Pick profile"),
    ("reset_profile", "Reset from profile"),
    ("save_as_profile", "Save as profile"),
    ("edit_profile_name", "Edit name"),
    ("edit_origin_corner", "Edit origin corner"),
    ("edit_max_feed", "Edit max feed"),
    ("edit_max_power", "Edit max power"),
    // `specs/0150-operation-always-has-numbers` criteria 1 and 22.
    ("edit_default_power", "Edit default power"),
    ("edit_default_speed", "Edit default speed"),
    ("edit_laser_mode", "Edit laser mode"),
    // `specs/0111-laser-operation-tools` criterion 22.
    ("motorised_z_on", "Motorised Z axis on"),
    ("motorised_z_off", "Motorised Z axis off"),
    // `specs/0029-laser-job-preview-and-output` UX notes §12 (ADR 0065 §1, §3; ADR 0064 §11).
    ("air_assist_on", "Air assist on"),
    ("air_assist_off", "Air assist off"),
    ("use_machine_size", "Use machine size"),
    // `specs/0118` slice 2d, ux-notes W60.
    ("focus_pulse_on", "Edit focus pulse"),
    ("focus_pulse_off", "Edit focus pulse"),
    ("edit_pulse_power", "Edit pulse power"),
    ("edit_pulse_length", "Edit pulse length"),
    ("set_fixture_material", "Pick fixture material"),
    ("set_start_machine_origin", "Start: Machine origin"),
    ("set_start_head_position", "Start: Head position"),
    (
        "set_start_home_then_origin",
        "Start: Home, then machine origin",
    ),
    // `specs/0118-machine-control` 2c (adrs O2, UX notes W41, W42).
    ("set_start_user_origin", "Start: User origin"),
    ("set_user_origin", "Set origin here"),
    ("type_user_origin", "User origin"),
    ("export_job", "Export job file"),
    ("send_job", "Send job to machine"),
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
    // `specs/0141-parametric-generators` (ADR 0080 §7): the fallback names; the steps also carry
    // their kind (`gk`) and parameter (`gp`) as fields for a kind-specific row text.
    ("add_generated", "Add generated object"),
    ("change_parameter", "Change parameter"),
    // `specs/0099-sketch-pages` revision 2026-11-19 (sketch instances), criteria 23, 24.
    ("expose_parameter", "Expose parameter"),
    ("unexpose_parameter", "Stop exposing parameter"),
    ("change_exposed_parameter", "Change exposed parameter"),
    ("reset_parameter", "Back to the sketch's value"),
    ("reset_parameters", "Back to the sketch's values"),
    ("own_parameters", "Own parameter values"),
    // `specs/0099-sketch-pages` slice 2, criterion 32.
    ("link_parameters", "Link parameters"),
    ("unlink_parameters", "Unlink parameters"),
    ("convert_generated", "Convert to paths"),
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
    // `specs/0137-document-library` criteria 4 and 5.
    ("import_page", "Import page"),
    ("import_elements", "Import elements"),
    // `specs/0154-library-into-slots` criteria 4, 7 and 8 (ADR 0088 decision 5).
    ("put_into_slot", "Put into slot"),
    ("place_page_into_slot", "Place page into slot"),
    ("replace_slot_content", "Replace slot content"),
    ("fill_slots", "Fill slots"),
    // The fit written to a fragment's private copy before the insert; never a step of its own.
    ("uniform_map", "Fit into slot"),
    // `specs/0138-template-library` criterion 3 (UX F3, F5).
    ("load_online_template", "Load template"),
    ("import_icon", "Import icon"),
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
    ("set_rounded", "Change rounding"),
    // `specs/0073-parameter-handle-clone` criterion 12: a parameter handle drag or entry written
    // to a copy.
    ("copy_with_corner_radius", "Copy with corner radius"),
    ("copy_with_inner_ratio", "Copy with inner ratio"),
    ("copy_with_arc", "Copy with arc"),
    ("copy_with_curve", "Copy with curve"),
    ("copy_with_rounding", "Copy with rounding"),
    // `specs/0145-shape-spiro-handles` slice 3 criteria 21 and 22.
    ("set_randomized", "Change randomizing"),
    ("reroll_seed", "New seed"),
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
    // `specs/0156-fused-lettering` criterion 11.
    ("fuse", "Bridge"),
    // `specs/0085-stroke-to-path` criterion 21.
    ("stroke_to_path", "Stroke to path"),
    // `specs/0117-centre-lines` criterion 2.
    ("centre_lines", "Find centre lines"),
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
    // `specs/0124-gradient-fill` criterion 18, UX notes §11.
    ("fill_gradient", "Fill: Gradient"),
    ("gradient_linear", "Gradient: Linear"),
    ("gradient_radial", "Gradient: Radial"),
    ("edit_gradient", "Edit gradient"),
    ("add_gradient_stop", "Add gradient stop"),
    ("remove_gradient_stop", "Remove gradient stop"),
    ("reverse_gradient", "Reverse gradient"),
    // `specs/0025-raster-trace` criterion 13.
    // `specs/0135-image-optimisation` criteria 2, 9, 24, 59 (ADR 0072 §7).
    ("edit_adjustments", "Edit adjustments"),
    ("restore_original", "Restore original"),
    ("paste_adjustments", "Paste adjustments"),
    ("run_remove_background", "Run Remove background"),
    ("run_upscale", "Run Upscale"),
    ("run_ai_denoise", "Run AI denoise"),
    ("follow_original", "Follow original"),
    ("trace_image", "Trace image"),
    // `specs/0114-image-trace-mask` criterion 17, ADR 0061 §6.
    ("draw_trace_rectangle", "Draw rectangle"),
    ("draw_trace_lasso", "Draw lasso"),
    ("move_trace_region", "Move region"),
    ("resize_trace_region", "Resize region"),
    ("move_trace_lasso_corner", "Move lasso corner"),
    ("delete_trace_region", "Delete region"),
    ("clear_trace_mask", "Clear mask"),
    ("trace_mask_inside", "Mask: Inside"),
    ("trace_mask_outside", "Mask: Outside"),
    ("add_trace_part", "Add part"),
    ("remove_trace_part", "Remove part"),
    ("edit_trace_settings", "Change trace settings"),
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
    // `specs/0121-print-and-cut` (ADR 0081 §2, §4, §7).
    ("insert_registration_marks", "Add registration marks"),
    ("add_registration_mark", "Add registration mark"),
    ("registration_mark_size", "Registration mark size"),
    ("print_and_cut_measure", "Measure mark"),
    ("print_and_cut_tolerance", "Print and cut tolerance"),
    ("apply_print_and_cut", "Fit sheet to marks"),
    ("reset_print_and_cut", "Reset sheet placement"),
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
    curvyo_document_core::LABEL_ALIGN_FIXTURE_SETS,
    curvyo_document_core::LABEL_MARK_TEST_GRID_LASERED,
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
    fn the_test_grid_and_materials_steps_are_named_and_free() {
        use curvyo_document_core as dc;
        for (label, name) in [
            (dc::ADD_TEST_GRID_COMMIT_LABEL, "Add test grid"),
            (dc::EDIT_TEST_GRID_COMMIT_LABEL, "Edit test grid"),
            (dc::PLACE_TEST_GRID_COMMIT_LABEL, "Place test grid"),
            (dc::USE_MATERIAL_COMMIT_LABEL, "Use material"),
            (dc::USE_PICK_COMMIT_LABEL, "Use pick"),
            (dc::ADD_MATERIAL_COMMIT_LABEL, "Add material"),
            (dc::ADD_MATERIAL_SET_COMMIT_LABEL, "Add set"),
            (dc::DELETE_MATERIAL_SET_COMMIT_LABEL, "Delete set"),
            (dc::JUDGE_TEST_COMMIT_LABEL, "Mark pick"),
            (dc::UNMARK_PICK_COMMIT_LABEL, "Unmark pick"),
            (dc::EDIT_TEST_NOTE_COMMIT_LABEL, "Edit note"),
            (dc::SET_TEST_PHOTO_COMMIT_LABEL, "Add photo"),
            (dc::CROP_TEST_PHOTO_COMMIT_LABEL, "Crop photo"),
            (dc::REMOVE_TEST_PHOTO_COMMIT_LABEL, "Remove photo"),
            (dc::DELETE_TEST_COMMIT_LABEL, "Delete test"),
        ] {
            assert_eq!(operation_name(label), name, "{label}");
            assert!(
                crate::step_scope::FREE_STEPS.contains(&label),
                "{label} is in one step-scope list"
            );
        }
        // `0029` (ADR 0064 §11): Lasered is a reserved commit of the export, no step.
        assert!(RESERVED_LABELS.contains(&dc::LABEL_MARK_TEST_GRID_LASERED));
        assert_eq!(known_operation_name(dc::LABEL_MARK_TEST_GRID_LASERED), None);
    }

    #[test]
    fn the_material_test_library_steps_are_named_and_the_save_refresh_is_reserved() {
        use curvyo_document_core as dc;
        for (label, name) in [
            (dc::ADD_TEST_GRID_COMMIT_LABEL, "Add test grid"),
            (dc::EDIT_TEST_GRID_COMMIT_LABEL, "Edit test grid"),
            (dc::USE_MATERIAL_COMMIT_LABEL, "Use material"),
            (dc::USE_OWN_NUMBERS_COMMIT_LABEL, "Use own numbers"),
            (dc::EDIT_OPERATION_SETTINGS_COMMIT_LABEL, "Edit settings"),
            (dc::RESET_TO_MATERIAL_COMMIT_LABEL, "Reset to material"),
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
    fn the_fixture_set_alignment_is_reserved_and_unnamed() {
        // `specs/0113-fixture-sets-and-pins` criterion 47: no step of the maker's.
        assert!(RESERVED_LABELS.contains(&curvyo_document_core::LABEL_ALIGN_FIXTURE_SETS));
        assert_eq!(
            known_operation_name(curvyo_document_core::LABEL_ALIGN_FIXTURE_SETS),
            None
        );
    }

    /// `specs/0124-gradient-fill` criterion 18.
    #[test]
    fn the_gradient_edits_are_named() {
        use curvyo_document_core::{FillPaint, GradientKind, StyleEdit};
        for (edit, name) in [
            (StyleEdit::FillPaint(FillPaint::Gradient), "Fill: Gradient"),
            (
                StyleEdit::GradientKind(GradientKind::Linear),
                "Gradient: Linear",
            ),
            (
                StyleEdit::GradientKind(GradientKind::Radial),
                "Gradient: Radial",
            ),
            (StyleEdit::GradientStopOffset(1, 0.5), "Edit gradient"),
            (StyleEdit::AddGradientStop(None), "Add gradient stop"),
            (StyleEdit::RemoveGradientStop(1), "Remove gradient stop"),
            (StyleEdit::ReverseGradient, "Reverse gradient"),
        ] {
            assert_eq!(operation_name(edit.commit_label()), name);
        }
        assert_eq!(
            operation_name(curvyo_document_core::EDIT_GRADIENT_LABEL),
            "Edit gradient"
        );
    }

    #[test]
    fn the_trace_mask_labels_are_named() {
        for label in curvyo_document_core::TRACE_MASK_LABELS {
            assert!(known_operation_name(label).is_some(), "{label}");
        }
        assert_eq!(operation_name("draw_trace_rectangle"), "Draw rectangle");
        assert_eq!(operation_name("trace_mask_outside"), "Mask: Outside");
        assert_eq!(
            operation_name("edit_trace_settings"),
            "Change trace settings"
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

    #[test]
    fn the_laser_output_steps_are_named_and_free() {
        use curvyo_document_core as dc;
        for (label, name) in [
            (dc::EXPORT_JOB_COMMIT_LABEL, "Export job file"),
            (
                dc::SET_FIXTURE_MATERIAL_COMMIT_LABEL,
                "Pick fixture material",
            ),
            // Superseded by 0029 rework (2026-11-17): one Start pick, one label per value.
            (
                dc::SET_START_MACHINE_ORIGIN_COMMIT_LABEL,
                "Start: Machine origin",
            ),
            (
                dc::SET_START_HEAD_POSITION_COMMIT_LABEL,
                "Start: Head position",
            ),
            (
                dc::SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL,
                "Start: Home, then machine origin",
            ),
            (dc::SET_START_USER_ORIGIN_COMMIT_LABEL, "Start: User origin"),
            (dc::SET_USER_ORIGIN_COMMIT_LABEL, "Set origin here"),
            (dc::TYPE_USER_ORIGIN_COMMIT_LABEL, "User origin"),
            (dc::AIR_ASSIST_ON_COMMIT_LABEL, "Air assist on"),
            (dc::AIR_ASSIST_OFF_COMMIT_LABEL, "Air assist off"),
            (dc::USE_MACHINE_SIZE_COMMIT_LABEL, "Use machine size"),
            (dc::FOCUS_PULSE_ON_COMMIT_LABEL, "Edit focus pulse"),
            (dc::FOCUS_PULSE_OFF_COMMIT_LABEL, "Edit focus pulse"),
            (dc::EDIT_PULSE_POWER_COMMIT_LABEL, "Edit pulse power"),
            (dc::EDIT_PULSE_LENGTH_COMMIT_LABEL, "Edit pulse length"),
        ] {
            assert_eq!(operation_name(label), name, "{label}");
            assert!(
                crate::step_scope::FREE_STEPS.contains(&label),
                "{label} is in one step-scope list"
            );
        }
        assert_eq!(known_operation_name("mark_test_grid_lasered"), None);
        for gone in [
            "set_start_from_machine_origin",
            "set_start_from_head_position",
            "set_home_first_on",
            "set_home_first_off",
        ] {
            assert_eq!(known_operation_name(gone), None, "{gone}");
        }
    }
}
