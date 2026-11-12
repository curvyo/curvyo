//! Which commit labels a move-only selection may write (`specs/0106-move-versus-edit` criteria 5
//! and 9, `adrs.md` decision 3); a label in no list is an edit.

/// The move commands of criterion 5: they change where a whole object is (position, matrix,
/// removal), never what it is.
pub const MOVE_STEPS: &[&str] = &[
    "translate_objects",
    "duplicate_objects",
    "linked_clone",
    "place_on_page",
    "place_linked_clones",
    "place_page",
    "delete_objects",
    "delete_placement",
    "resize_path",
    "resize_rect",
    "resize_ellipse",
    "resize_star_frame",
    "set_rect_bounds",
    "set_ellipse_frame",
    "set_star_frame",
    "transform_resize",
    "rotate_object",
    "transform_rotate",
    "skew_path",
    "transform_skew",
    "transform_objects",
    "flip_horizontal",
    "flip_vertical",
    "align_left",
    "align_centre_h",
    "align_right",
    "align_top",
    "align_centre_v",
    "align_bottom",
    "distribute_h",
    "distribute_v",
    "align_left_page",
    "align_centre_h_page",
    "align_right_page",
    "align_top_page",
    "align_centre_v_page",
    "align_bottom_page",
    "distribute_h_page",
    "distribute_v_page",
    "arrange_grid",
    // `specs/0102-laser-fixtures` adrs.md section 4: the placed page or clone dropped into a slot.
    "move_to_slot",
    // A Spiro path's placement matrix (ADR 0053): where it is, not what it is.
    "set_spiro_transform",
];

/// The commands that are not about the selected objects: pages, manufacturing, profiles and
/// fixtures, creation and import, the Layers eye and lock, layers, document settings, history
/// versions, and the sketch commands, which only run on the active sketch page.
pub const FREE_STEPS: &[&str] = &[
    "add_page",
    "delete_page",
    "rename_page",
    "resize_page",
    "move_page",
    "reorder_pages",
    "add_manufacturing",
    "delete_manufacturing",
    "set_work_area",
    "add_sketch_page",
    "add_output_page",
    "delete_fixture",
    "set_fixture_visible",
    "set_fixture_params",
    // `specs/0102-laser-fixtures` adrs.md section 4: fixture commands are free (they only write
    // the manufacturing's fixture nodes, never the selected objects).
    "add_grid",
    "add_holder",
    "add_clamp",
    "add_stop",
    "add_screw",
    "resize_fixture",
    "set_grid_spacing",
    "set_grid_major",
    "set_tape_edge",
    "set_grid_origin",
    "rename_slot",
    "set_slot_shape",
    "set_slot_position",
    "set_slot_size",
    "set_fixtures_snap_on",
    "set_fixtures_snap_off",
    "rename_fixture",
    "set_fixture_active",
    "set_fixture_inactive",
    "move_fixture",
    "rotate_fixture",
    "add_slot",
    "remove_slot",
    "set_slot",
    // `specs/0103-fixture-jobs` adrs.md section 7: job commands write only fixture job nodes.
    "add_fixture_job",
    "rename_fixture_job",
    "delete_fixture_job",
    "set_fixture_job_show_on",
    "set_fixture_job_show_off",
    "set_fixture_job_output_on",
    "set_fixture_job_output_off",
    "move_fixture_job",
    "assign_fixture_target",
    "remove_fixture_target",
    // `specs/0101-laser-operations`: operations write only the manufacturing's operation nodes,
    // never the selected objects, so an assignment of a read-only inner object is allowed.
    "add_operation",
    "assign_to_operation",
    "remove_from_operation",
    "rename_operation",
    "set_operation_show",
    "set_operation_output",
    "set_operation_margin",
    "delete_operation",
    "move_operation",
    "cuts_last_on",
    "cuts_last_off",
    // `specs/0028-material-test-library` adrs.md section 7: a test grid is made and rebuilt from
    // the Operations tab, never from the selected objects' own handles; settings and material
    // edits write operation nodes and the root marker only.
    "add_test_grid",
    "edit_test_grid",
    "rebuild_test_grid",
    "mark_test_cell",
    "unmark_test_cell",
    "rename_test_grid",
    "delete_test_grid_row",
    "use_material",
    "use_own_numbers",
    "edit_operation_settings",
    "reset_to_material",
    "save_as_material",
    "edit_material",
    "rename_material",
    "delete_material",
    "remove_missing_items",
    "pick_profile",
    "reset_profile",
    "save_as_profile",
    "edit_profile_name",
    "edit_origin_corner",
    "edit_max_feed",
    "edit_max_power",
    "edit_laser_mode",
    "motorised_z_on",
    "motorised_z_off",
    "add_sketch",
    "edit_sketch",
    "place_sketch_clone",
    "convert_sketch_page",
    "delete_sketch_page",
    "create_path",
    "create_rect",
    "create_ellipse",
    "create_polygon",
    "create_star",
    "add_text",
    "create_dimension",
    "go_to_version",
    "clone_version",
    "remove_orphan_clones",
    "resize_document",
    "fit_document_to_content",
    "set_display_unit",
    "set_document_background",
    "set_document_snapping",
    "set_page_grid",
    "import_svg",
    "import_image",
    "import_images",
    "import",
    "paste_image",
    "set_node_visible",
    "set_node_locked",
    "new_layer",
    "delete_layer",
    "load_template",
];

/// The edit commands of criterion 9 that have a label today. Any label in none of the three lists
/// is an edit as well (Proposal: unknown future commands default to guarded).
pub const EDIT_STEPS: &[&str] = &[
    "convert_sketch_to_path",
    "extend_path",
    "connect_paths",
    "close_path",
    "edit_text",
    "change_font",
    "change_text_size",
    "change_text_alignment",
    "change_letter_spacing",
    "change_word_spacing",
    "change_line_spacing",
    "change_character_rotation",
    "change_writing_mode",
    "change_glyph_orientation",
    "change_taper",
    "change_kerning",
    "put_text_on_path",
    "put_text_on_paths",
    "release_from_path",
    "change_text_side",
    "change_line_on_path",
    "move_text_on_path",
    "change_text_on_path",
    "change_distance_on_path",
    "mirror_text_on_path",
    "flip_letters",
    "clear_kerning",
    "change_char_rotation",
    "clear_char_rotation",
    "unlink_clone",
    "own_placement",
    "follow_placement",
    "own_style",
    "original_style",
    "spread_colours",
    "set_point_count",
    "set_inner_ratio",
    "set_corner_radius",
    "set_corner_radii",
    "set_ellipse_arc",
    "set_curve",
    "set_curve_kind",
    "set_width_point",
    "remove_width_point",
    "reset_width_profile",
    "set_width_profile",
    "convert_curve_to_bezier",
    "convert_to_paths",
    "edit_style",
    "move_anchors",
    "set_handle",
    "convert_anchor_kind",
    "insert_anchor",
    "delete_anchors",
    "set_segment_line",
    "set_segment_curve",
    "bend_segment",
    "join_endpoints",
    "split_at_anchor",
    "boolean_union",
    "boolean_difference",
    "boolean_intersection",
    "boolean_exclusion",
    "boolean_reverse_difference",
    "combine_paths",
    "split",
    "break_apart",
    "cut_at_crossings",
    "fracture",
    "flatten",
    "weld",
    "stroke_to_path",
    "convert_pattern",
    "fill_pattern",
    "pattern_generator_hatch",
    "pattern_generator_crosshatch",
    "pattern_generator_grid",
    "pattern_generator_dots",
    "pattern_generator_honeycomb",
    "pattern_generator_brick_wall",
    "pattern_generator_leaf_forest",
    "pattern_generator_sea_waves",
    "pattern_generator_waves",
    "pattern_generator_interpolation_waves",
    "pattern_spacing",
    "pattern_angle",
    "pattern_second_angle",
    "pattern_diameter",
    "pattern_cell_size",
    "pattern_wall",
    "pattern_width",
    "pattern_height",
    "pattern_joint",
    "pattern_leaf_size",
    "pattern_size_jitter",
    "pattern_rotation_jitter",
    "pattern_density",
    "pattern_scale_radius",
    "pattern_rings",
    "pattern_ring_spacing",
    "pattern_wavelength",
    "pattern_amplitude",
    "pattern_lines_count",
    "pattern_margin",
    "pattern_mode_lines",
    "pattern_mode_shapes",
    "pattern_seed",
    "pattern_whole_pieces",
    "trace_image",
    "recognise_text",
    "offset_path",
    "group",
    "ungroup",
    "rename_node",
    "set_node_opacity",
    "move_nodes",
    "set_guide",
    "move_dimension",
    "edit_dimension_nodes",
    "convert_dimension",
    "tile",
    "edit_tiling",
    "release_tiling",
];

/// What a commit label is to a move-only selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepScope {
    /// A move command: allowed for any selection.
    Move,
    /// Not about the selection: allowed for any selection.
    Free,
    /// An edit command: refused for a move-only selection.
    Edit,
}

/// The scope of `label`; [`StepScope::Edit`] for a label in no list.
#[must_use]
pub fn step_scope(label: &str) -> StepScope {
    if MOVE_STEPS.contains(&label) {
        StepScope::Move
    } else if FREE_STEPS.contains(&label) {
        StepScope::Free
    } else {
        StepScope::Edit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_names::OPERATION_NAMES;

    /// Criterion 9's guard: every command label is classified in exactly one list, and the lists
    /// name no label the history does not know.
    #[test]
    fn every_command_label_is_classified_once() {
        for (label, _) in OPERATION_NAMES {
            let lists = [MOVE_STEPS, FREE_STEPS, EDIT_STEPS]
                .iter()
                .filter(|list| list.contains(label))
                .count();
            assert_eq!(lists, 1, "{label} is in {lists} lists");
        }
        let known: Vec<&str> = OPERATION_NAMES.iter().map(|(label, _)| *label).collect();
        for label in MOVE_STEPS.iter().chain(FREE_STEPS).chain(EDIT_STEPS) {
            assert!(known.contains(label), "{label} is no command label");
        }
    }

    #[test]
    fn criterion_five_moves_are_moves_and_unknown_labels_are_edits() {
        for label in [
            "translate_objects",
            "arrange_grid",
            "delete_objects",
            "place_page",
        ] {
            assert_eq!(step_scope(label), StepScope::Move);
        }
        for label in ["move_nodes", "group", "edit_style", "unlink_clone"] {
            assert_eq!(step_scope(label), StepScope::Edit);
        }
        assert_eq!(step_scope("set_node_visible"), StepScope::Free);
        assert_eq!(step_scope("a_future_command"), StepScope::Edit);
    }
}
