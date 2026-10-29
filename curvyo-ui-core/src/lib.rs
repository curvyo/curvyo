//! The Curvyo interaction layer (ADR 0001 §1, §2): pen- and node-tool
//! state machines, hit-testing, selection and command dispatch, as plain
//! state and pure functions (`specs/0002-path-node-editing/adrs.md`).
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, network,
//! clock, threads or UI. The frontend renders this crate's state and
//! forwards input events into it; it holds no editing logic of its own.
//! Depends on `curvyo-document-core` (the commands these tools dispatch)
//! and `curvyo-geometry-core` (hit-testing a curved segment, subdividing
//! one for an insert) — never on `curvyo-render-core` or the wasm
//! facade (ADR 0011 §3).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod anchor_id_minter;
mod angle_snap;
mod boolean;
mod break_apart;
mod clone_colours;
mod clone_gesture;
mod clone_marks;
mod clone_panel;
mod close_path;
mod closing_join;
mod colour_hsv;
mod colour_pick;
mod combine;
mod conversion;
mod cut;
mod cut_pieces;
mod cuttable;
mod dash_text;
mod display_unit_text;
mod document_fit;
mod document_presets_view;
mod ellipse_tool;
mod face_style_pick;
mod family_key;
mod file_chords;
mod flatten_command;
mod flip;
mod font_library;
mod font_library_file;
mod font_match;
mod format_form;
mod format_list_view;
mod fracture;
mod glyph_table;
mod group_box;
mod group_entry;
mod group_transform;
mod grouping;
mod history_branch_text;
mod history_lanes;
mod history_names;
mod history_notice;
mod hit_test;
mod hit_test_object;
mod image_frame;
mod image_import;
mod image_import_text;
mod image_placement;
mod layer_move;
mod layer_rows;
mod layer_text;
mod linked_clone;
mod marquee;
mod mirror;
mod mirror_text;
mod modifiers;
mod move_entry;
mod node_marquee;
mod node_tool;
mod nudge;
mod object_bounds;
mod object_history_text;
mod object_selection;
mod offset_drag;
mod offset_entry;
mod offset_plan;
mod offset_primitive;
mod oriented_box;
mod panel_content;
mod panel_tabs;
mod param_edit;
mod param_entry;
mod param_handles;
mod pen_constraint;
mod pen_entry;
mod pen_entry_text;
mod pen_target;
mod pen_tool;
mod poly_star_tool;
mod preview_faces;
mod preview_sample;
mod rectangle_tool;
mod reference_point;
mod resize_direction;
mod ruler;
mod segment_bend;
mod select_bar;
mod select_bar_arc;
mod select_tool;
mod selection;
mod shape_arc_edit;
mod shape_arc_text;
mod shape_knobs;
mod shape_placement;
mod shape_tool_common;
mod skew_entry;
mod skew_math;
mod split;
mod stack_plan;
mod stroke_to_path_plan;
mod style_edit;
mod style_entry;
mod style_panel;
mod style_scope;
mod svg_font_match;
mod svg_import;
mod svg_import_arcs;
mod svg_import_faces;
mod svg_import_images;
mod svg_import_match;
mod svg_import_shapes;
mod svg_import_style;
mod svg_import_text;
mod svg_text;
mod svg_text_align;
mod svg_text_flow;
mod svg_text_glyphs;
mod svg_text_layout;
mod text_box;
mod text_caret;
mod text_convert;
mod text_edit;
mod text_edit_chars;
mod text_layout;
mod text_layout_fields;
mod text_on_path;
mod text_path_caret;
mod text_path_edit;
mod trace_plan;
mod trace_transfer;
mod transform_commit;
mod transform_drag;
mod transform_entry;
mod transform_handle_layout;
mod transform_math;
mod transform_primitive;
mod undo_stack;
mod units;
mod value_scale;
mod viewport;
mod weld_plan;

pub use anchor_id_minter::AnchorIdMinter;
pub use angle_snap::{MAX_SKEW_SNAP_DEG, snap_angle, snap_skew_angle};
pub use boolean::{
    BooleanAvailability, BooleanOp, BooleanPlan, BooleanRefusal, boolean_availability, plan_boolean,
};
pub use break_apart::{
    BREAK_APART_COMMIT_LABEL, BreakApartPlan, BreakApartRefusal, plan_break_apart,
};
pub use clone_colours::spread_colours;
pub use clone_marks::{
    CloneMarks, EdgeChip, MarkForm, MarkFrame, MarkRect, MarkRole, MarkTag, MarkTrace, clone_marks,
};
pub use clone_panel::{
    CloneBlock, OverrideState, clone_block, followers_of, object_name, overrides_line,
    overrides_words, select_clones, select_originals, select_root_originals,
};
pub use close_path::{
    ClosableCounts, ClosePlan, CloseTarget, closable_counts, close_path_set, plan_close_paths,
};
pub use closing_join::{JoinType, resolve_closing_node};
pub use colour_hsv::{Hsv, hsv_to_rgb, rgb_to_hsv};
pub use colour_pick::{PaintTarget, PickedColour, pick_colour};
pub use combine::{
    COMBINE_COMMIT_LABEL, CombinePlan, CombineRefusal, PathAvailability, path_availability,
    plan_combine,
};
pub use conversion::{build_primitive_conversions, primitive_as_path};
pub use curvyo_svg_core::SvgDrawing;
/// The Trace tool's settings schema (criterion 6), passed through for the host.
pub use curvyo_vectorize_core::{
    ADVANCED_PARAMS as TRACE_ADVANCED_PARAMS, Denoise as TraceDenoise, PARAMS as TRACE_PARAMS,
    PRESETS as TRACE_PRESETS, ParamKind as TraceParamKind, ParamUnit as TraceParamUnit,
    TraceParams, Upscale as TraceUpscale,
};
pub use cut::{CUT_COMMIT_LABEL, CutPlan, CutRefusal, MAX_CUT_PIECES, plan_cut};
pub use cuttable::{
    CUTTABLE_CHECK_VERSION, CUTTABLE_SIZE_MM, CUTTABLE_WORD, FontChecks, cuttable_pieces,
    union_pieces,
};
pub use dash_text::{MAX_DASH_NUMBER, MAX_DASH_NUMBERS, dash_text, parse_dash_text};
pub use display_unit_text::{
    content_too_large_message, document_side_message, format_cursor, format_field_length,
    format_size, format_status_length, parse_document_side, parse_length,
};
pub use document_fit::fit_document_to_content;
pub use document_presets_view::{
    PresetEntry, PresetGroupView, PresetsView, orientation_swap, preset_pick_size, presets_view,
};
pub use ellipse_tool::EllipseTool;
pub use face_style_pick::{
    Greyed, Pressed, StyleButton, StyleButtons, StylePress, WEIGHT_STEPS, family_target,
    family_tile, italic_target, press_italic, press_weight, style_buttons, weight_target,
};
pub use family_key::family_key;
pub use file_chords::{
    FileCommand, Platform, chord_label, command_tooltip, file_chord, is_import_chord,
};
pub use flatten_command::{FLATTEN_COMMIT_LABEL, plan_flatten};
pub use flip::{FlipAxis, FlipOutcome, flip_selection};
pub use font_library::{
    ALL_ID, BuiltInTag, CheckResult, FAVOURITES_ID, FamilyFacts, FontLibrary, MAX_LIST_KEYS,
    MAX_OWN_TAGS, MAX_TAG_NAME_CHARS, MemberSource, Membership, Override, TagEntry, TagNameError,
    automatic, check_tag_name,
};
pub use font_library_file::{
    FONT_LIBRARY_FORMAT, FontLibraryFileError, MAX_FONT_LIBRARY_BYTES, read_font_library,
    write_font_library,
};
pub use font_match::{FaceMatch, FontBlock, ListedFace, match_face};
pub use format_form::{
    FieldError, FormDraft, FormField, FormPrefill, add_prefill, added_notice, check_format_form,
    edit_prefill, field_error, import_notice, import_refusal, saved_notice,
};
pub use format_list_view::{FormatGroupView, FormatListView, FormatRowView, format_list_view};
pub use fracture::{FRACTURE_COMMIT_LABEL, plan_fracture};
pub use glyph_table::{GlyphGroup, GlyphTable, MAX_GLYPH_QUERY_CHARS, NO_BLOCK, glyph_table};
pub use group_box::{
    GroupBoxShape, GroupSelection, group_handles, hit_group_handle, is_drawn_group_handle,
};
pub use group_entry::GroupEntry;
pub use grouping::{
    GroupingAvailability, TOO_DEEP_NOTICE, context_announcement, grouped_notice,
    grouping_availability, groups_refusal_notice, selection_announcement, ungrouped_notice,
};
pub use history_branch_text::{
    BRANCH_STATUS_WORD, branch_header_text, branch_lane_name, cloned_notice, more_branches_text,
    went_to_notice,
};
pub use history_lanes::{
    BranchSpan, DRAWN_BRANCHES, Edge, HistoryItem, LineStyle, ListLayout, MAX_LANE, RowLook,
    Segment,
};
pub use history_names::{
    OPERATION_NAMES, RESERVED_LABELS, UNKNOWN_OPERATION_NAME, known_operation_name, operation_name,
};
pub use history_notice::{
    FINISH_PEN_FIRST, NOTHING_FROM_ORIGINAL, OBJECT_TOOL_NOTICE, SELECT_ONE_OBJECT, nothing_notice,
    object_nothing_notice, object_restore_notice, objects_summary, restore_notice, summary_shapes,
};
pub use hit_test::{Hit, hit_test};
pub use hit_test_object::{hit_test_object, hit_test_objects, hit_test_objects_along};
pub use image_import::{PlannedImage, plan_image_import};
pub use image_import_text::{
    ANIMATED_NOTICE, BAD_ASSET_NOTICE, DAMAGED_IMAGE_NOTICE, IMAGE_COMMAND_REFUSAL,
    OUT_OF_MEMORY_NOTICE, REDUCED_NOTICE, UNSUPPORTED_FILE_NOTICE, batch_failures_notice,
    failure_notice, failure_reason, fit_notice, image_display_name, importing_notice,
    project_full_reason, size_notice, skipped_notice, truncated_notice,
};
pub use image_placement::{ImagePlacement, ImageTarget, STAGGER_MM, natural_size_mm, place_image};
pub use layer_move::{Reorder, move_refusal_text, reorder_plans, unchanged_text};
pub use layer_rows::{
    Cause, Container, Expansion, Flags, LayerRow, LayerRows, RowFacts, RowKind, RowMark, percent,
};
pub use layer_text::{
    DUPLICATE_FAILED_NOTICE, NothingToSelect, OpacityTarget, delete_locked_layer_notice,
    deleted_layer_notice, draw_refused_notice, inherited_lines, locked_object_notice,
    moved_announcement, nothing_to_select_notice, objects_count_text, opacity_caption,
    row_accessible_name, row_refusal, row_state_line,
};
pub use linked_clone::{
    LinkedClonePlan, MAX_REFUSAL_OUTLINES, linked_clone_shown, plan_linked_clone,
};
pub use marquee::{MarqueeMode, objects_in_marquee, units_in_marquee};
pub use mirror_text::{mirror_line, signed_size_text, zero_recovery_notice};
pub use modifiers::Modifiers;
pub use move_entry::MoveEntry;
pub use node_marquee::{
    NodeMarquee, anchors_in_box, editing_set_anchors, node_marquee_legend_text,
    nodes_selected_announcement,
};
pub use node_tool::{
    HitTolerances, JoinBlock, LiveNodeDrag, NodeSplitOutcome, NodeTool, NodeToolbarState,
    PointerDownOutcome as NodePointerDownOutcome, PointerUpOutcome as NodePointerUpOutcome,
};
pub use nudge::{
    Arrow, CONTINUATION_WINDOW_MS, NudgeEvent, NudgeOutcome, NudgeRun, nudge, selection_centre,
};
pub use object_bounds::{content_bounds, object_bounds, object_outline_bounds};
pub use object_history_text::{object_row_name, object_row_summary};
pub use object_selection::{ObjectSelection, SelectionCombine};
pub use offset_drag::{
    CORNER_SIGNS, HANDLE_HIT_RADIUS_PX, drag_distance, handle_positions, offset_box, snap_distance,
};
pub use offset_entry::{
    OFFSET_DEFAULT_MM, OFFSET_MAX_MM, OFFSET_MIN_MM, OffsetDirection, OffsetJoin,
    OffsetResultChoice, OffsetRole, OffsetSide, OffsetSides, OffsetValues, next_offset_join,
    offset_direction_from_code, offset_join_from_code, offset_result_from_code,
};
pub use offset_plan::{
    OffsetItem, OffsetPlan, OffsetRefusal, OffsetRequest, OffsetShape, offset_operands, plan_offset,
};
pub use oriented_box::{OrientedBox, oriented_bounds};
pub use panel_content::{PanelContent, panel_body};
pub use panel_tabs::{
    PanelTab, PanelTabEntry, PanelTabs, StyleBlocker, tab_entries, tab_entries_for,
};
pub use param_edit::{
    MAX_INNER_RATIO, MIN_INNER_RATIO, ParamValue, apply_param, clamped_ratio, commit_param_batch,
    value_from_pointer,
};
pub use param_entry::ParamEntry;
pub use param_handles::{
    Corner, HandleTiers, KNOB_DIAMETER_PX, KNOB_INSET_PX, KNOB_PITCH_PX, PARAM_HIT_PX,
    PARAM_MIN_SIDE_PX, ParamHandle, centre_drawn, corner_local_position, handle_tiers, knob_rho,
    param_handles, radius_gain, radius_travel,
};
pub use pen_constraint::constrain_segment;
pub use pen_entry::{LengthNotPositive, PenEntry, PenEntryField, PenEntryNotice, PenEntryPlaced};
pub use pen_entry_text::{
    PEN_ENTRY_GROUP_NAME, PEN_ENTRY_LENGTH_NOT_POSITIVE, PEN_ENTRY_OPENED_SENTENCE,
    pen_entry_added_sentence, pen_entry_label, pen_entry_name, pen_entry_notice, pen_entry_suffix,
};
pub use pen_target::{EndNode, EndNodeIndex, PenTarget, continuation_of, pen_target};
pub use pen_tool::{
    ClosingPreview, Continuation, PenTool, PointerUpOutcome as PenPointerUpOutcome, PressAction,
};
pub use poly_star_tool::{PolyStarMode, PolyStarSettings, PolygonStarTool};
pub use preview_faces::{PREVIEW_FACE_BUDGET, PreviewFaces};
pub use preview_sample::{MAX_SAMPLE_CHARS, sample_line};
pub use rectangle_tool::RectangleTool;
pub use reference_point::{
    MARKS_MIN_SIDE_PX, ReferencePoint, hit_mark, mark_hit_radius_px, marks_are_shown,
    shorter_side_px, visible_marks,
};
pub use resize_direction::ResizeDirection;
pub use ruler::{RulerAxis, RulerLabel, RulerLayout, RulerMajor, ruler_layout};
pub use segment_bend::{BendResolution, segment_is_bendable};
pub use select_bar::{
    BarPreview, BarValue, ObjectKind, SelectBarState, ids_of_kind, select_bar_state,
};
pub use select_bar_arc::{CurveBarState, EllipseBarState, GreyReason};
pub use select_tool::{
    Axis, EntryKey, GestureKind, GestureShape, KeyEntryRefusal, LiveEdit, LiveGesture,
    MoveEntryMode, MoveResolution, PressTarget, SelectDoubleClickOutcome, SelectPointerDownOutcome,
    SelectTool, TransformHandleTolerances, classify_press, double_click, double_click_in,
    entry_anchor,
};
pub use selection::NodeSelection;
pub use shape_arc_edit::{
    ARC_SEAM, ArcEnd, arc_angle_from_pointer, arc_capable, curve_capable, curve_from_pointer,
};
pub use shape_arc_text::{arc_announcement, arc_readout, curve_announcement, curve_readout};
pub use shape_placement::{
    CANCELLED_SENTENCE, Placed, PlacementError, PlacementField, PlacementKind, PlacementNoun,
    ShapePlacement, field_label, field_name, field_suffix, message as placement_message,
    placed_sentence,
};
pub use shape_tool_common::{CreateOutcome, CreatePreview};
pub use skew_entry::SkewEntry;
pub use skew_math::{SkewFrame, skew_angle, skew_factor, skew_frame};
pub use split::{SPLIT_COMMIT_LABEL, SplitPlan, SplitRefusal, plan_split};
pub use stack_plan::{MAX_STACK_PIECES, StackPlan, StackRefusal};
pub use stroke_to_path_plan::{
    StrokeToPathPlan, StrokeToPathRefusal, draws_stroke, plan_stroke_to_path,
    stroke_to_path_available,
};
pub use style_edit::StyleEditor;
pub use style_entry::{
    HexColour, MAX_STROKE_WIDTH_MM, MarkerSlot, StyleEntryError, StyleField, cap_from_name,
    hex_text, join_from_name, marker_place_from_name, marker_shape_from_name, opacity_from_percent,
    parse_hex, parse_marker_count, parse_opacity_percent, parse_stroke_width,
};
pub use style_panel::{
    DashChoice, DashShown, FillPanel, MarkersPanel, Rgba, StrokePanel, StylePanelState,
    style_panel_state,
};
pub use style_scope::{
    StyleScope, StyleTool, is_single_line_text, object_kind_name, owns_style, skipped_clones,
    skipped_single_line_texts, style_scope, style_scope_in,
};
pub use svg_font_match::{GENERIC_CURSIVE, GENERIC_SANS, GENERIC_SERIF, SvgFace, svg_face};
pub use svg_import::{
    SvgImport, SvgPlacement, import_svg_bytes, read_svg_drawing, svg_import, svg_page,
};
pub use svg_import_faces::{SvgFaceNeeds, substitute_face, svg_faces_needed};
pub use svg_import_text::{
    FileKind, NEITHER_PROJECT_NOR_SVG, file_kind, import_layer_name, nothing_imported_notice,
    notice_file_name, svg_busy_notice, svg_refusal_text,
};
pub use svg_text_glyphs::{SvgTextGlyphs, svg_text_glyphs};
pub use svg_text_layout::ImportFaces;
pub use text_box::{line_box_contains, line_box_corners, on_path_boxes};
pub use text_caret::{TextEditGeometry, edit_geometry, nearest_boundary};
pub use text_convert::{
    TextRefusal, area_refusal, build_text_conversions, text_path, texts_as_paths, texts_refusal,
};
pub use text_edit::{CaretMove, EditTarget, Finished, InputOutcome, TextEdit, TextInput};
pub use text_layout::{kept_placement, lay_out, layout_options, realigned_placement, text_layout};
pub use text_layout_fields::{
    LayoutChange, LayoutField, LayoutFieldView, LayoutSubject, field_view, parse_field_text,
    set_field, stepped, value_at,
};
pub use text_on_path::{PathPlacement, lay_out_on_path, measure_of};
pub use text_path_caret::{edit_geometry_on_path, nearest_boundary_on_path};
pub use text_path_edit::{
    default_position, dragged_values, ellipse_degrees, ellipse_point, handle_points,
    nearest_outline, nudged_values, position_at_degrees, projected_values, release_placement,
    rotated_position, shift_along, slid_values, snap_stops, stored_vertical_layout,
};
pub use trace_plan::{TracePlan, TraceTarget, plan_trace, trace_group_name, trace_style};
pub use trace_transfer::{
    TRACE_WIRE_HEADER, TraceStatus, TraceWire, read_trace_wire, trace_auto_threshold, trace_job,
    trace_job_with,
};
pub use transform_drag::{
    CornerLinking, CornerRadiusScaling, ParamDragInfo, ScaleModes, StrokeScaling,
};
pub use transform_entry::{
    EntryField, EntryKind, EntryOutcome, InvalidReason, TransformEntry, format_degrees,
    parse_entry_number,
};
pub use transform_handle_layout::{
    ALL_EIGHT, EditHandle, HandleSpec, Side, hit_transform_handle, is_corner, is_drawn_handle,
    resize_cursor_angle_degrees, resize_handle_local_position, skew_cursor_angle_degrees,
    transform_handles,
};
pub use transform_math::{
    ExtentSigns, RECOVERED_EXTENT_MM, ResizedBox, ZERO_EXTENT_MM, ZeroRecovery, opposite_direction,
    resize_anchor_local_position, resize_local_box, resize_local_box_with_prior,
    rotate_delta_angle, rotate_delta_for, rotate_pivot, scaled_and_floored,
    stroke_or_radius_factor,
};
pub use undo_stack::{SelectionSnapshot, Target as UndoTarget, UndoStack};
pub use units::{SelectedUnit, Units};
pub use value_scale::{FieldShown, Grid, ValueField, ValueScale};
pub use viewport::{DOCUMENT_INSET_PX, PX_PER_MM_AT_100, Viewport, Zoom};
pub use weld_plan::{
    WELD_COMMIT_LABEL, WeldAvailability, WeldPlan, WeldRefusal, plan_weld, weld_availability,
};
