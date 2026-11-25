//! The Curvyo document model (ADR 0002): units, the document root
//! record, and the `.curvyo` project container (ADR 0004 §1).
//!
//! This crate is pure and wasm-compatible (`CLAUDE.md` §6): no filesystem,
//! network, clock, threads or UI. Everything a caller needs — bytes in,
//! bytes out — is passed in explicitly; `curvyo-storage-io` and
//! `curvyo-app` own the impure edges (reading/writing files, native
//! dialogs).
//!
//! `project-file-foundation` (slice 1) implemented the thinnest possible
//! document: no nodes, just a `format_version` and a page `size`.
//! `path-node-editing` (slice 2) grows the node tree (ADR 0002 §5) with the
//! first geometry — paths and their anchors — without changing the shape
//! slice 1 fixed.

#![forbid(unsafe_code)]
// `CLAUDE.md` §5 allows unwrap/expect in tests; only production code is held
// to the stricter rule.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod width_profile;
pub use width_profile::{PathWidthPoints, WidthPoint, WidthPointError};

mod curve_kind;
mod spiro_transform;
pub use curve_kind::{CurveKind, CurveKindError};

mod affine;
mod assets;
mod came_from;
mod copy_roots;
mod credits;
mod fragment;
mod fragment_insert;
mod fragment_placements;
pub use fragment::{Fragment, FragmentError};
pub use fragment_insert::{
    FragmentReport, FragmentTarget, IMPORT_ELEMENTS_COMMIT_LABEL, IMPORT_ICON_COMMIT_LABEL,
    IMPORT_PAGE_COMMIT_LABEL, ImportError, LOAD_ONLINE_TEMPLATE_COMMIT_LABEL, PageFallback,
};
mod library_members;
pub use library_members::{
    LibraryMembers, MAX_PAGE_PICTURES, MAX_PICTURE_BYTES, NotAContainer, PAGES_PREFIX,
    THUMBNAIL_MEMBER, ZipMember, is_storable_png, read_library_members, read_page_picture,
    read_page_pictures, read_zip_member, with_pictures, with_thumbnail,
};
mod library_summary;
pub use came_from::{CameFrom, OnlineOrigin, UnixMillis};
pub use credits::Credit;
pub use library_summary::{
    LIBRARY_FORMAT, LIBRARY_MEMBER, LibrarySummary, MAX_LIBRARY_BYTES, PageSummary, kind_word,
    library_summary,
};
mod clone_codec;
mod clone_copy;
mod clone_cycle;
mod clone_hold;
mod clone_model;
mod clone_overrides;
mod clone_picture;
mod clone_place;
mod clone_preview;
mod clone_read;
mod clone_repair;
mod clone_transform;
mod clone_unlink;
mod clone_write;
mod clones;
mod container;
mod container_assets;
mod container_ended;
mod container_recovery;
mod container_share;
mod corner_radii;
mod corner_radii_codec;
mod derived_assets;
mod devices_list;
mod display_unit;
mod document;
mod document_background;
mod document_presets;
mod document_size;
mod page_frame;
mod page_grid;
mod page_kind;
mod page_kind_migration;
mod page_layout;
mod page_moves;
mod pages;
mod placement_commands;
mod placement_objects;
mod workspace_disk_plan;
mod workspace_list;
mod workspace_list_view;
mod workspace_names;
mod workspace_plan;
pub use page_grid::{GridMode, GridScale, OriginCorner, PageGrid, PageGridError};
pub use page_kind::PageKind;
pub use page_moves::PageContents;
pub use pages::{ADD_SKETCH_PAGE_COMMIT_LABEL, FIRST_PAGE_ID, Page, PageError};
mod dimension_codec;
mod dimension_commands;
mod dimension_keep;
mod dimension_model;
mod dimension_read;
mod generated_codec;
mod generated_commands;
#[cfg(test)]
mod generated_damage_tests;
mod generated_outline;
mod print_and_cut_codec;
mod print_and_cut_commands;
pub use print_and_cut_codec::{
    APPLY_PRINT_AND_CUT_COMMIT_LABEL, PRINT_AND_CUT_DEFAULT_TOLERANCE_UM,
    PRINT_AND_CUT_MAX_MEASURE_UM, PRINT_AND_CUT_MAX_TOLERANCE_UM,
    PRINT_AND_CUT_MEASURE_COMMIT_LABEL, PRINT_AND_CUT_MIN_TOLERANCE_UM,
    PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL, PrintAndCutMeasures, PrintAndCutRegister,
    RESET_PRINT_AND_CUT_COMMIT_LABEL,
};
mod registration_mark_codec;
mod registration_mark_commands;
pub use registration_mark_commands::{MarksObject, registration_mark_corners};
mod registration_mark_lines;
mod registration_mark_model;
pub use dimension_commands::{ConvertedPath, DimensionConversion};
pub use dimension_model::{
    CONVERT_DIMENSION_COMMIT_LABEL, CREATE_DIMENSION_COMMIT_LABEL, CurveSlot, DimensionKind,
    DimensionPlace, DimensionSlots, DimensionSnapshot, DimensionValue,
    EDIT_DIMENSION_NODES_COMMIT_LABEL, MOVE_DIMENSION_COMMIT_LABEL, Measured, PointSlot,
    ShapeFeature, staying_dimensions,
};
pub use dimension_read::{
    CIRCLE_TOLERANCE, angle_rays, arc_circle, cubic_at, measure, resolve as resolve_dimension,
    resolve_dimensions_in, resolve_into as resolve_dimension_into, segment, shape_point,
};
pub use registration_mark_lines::{MarkLines, mark_lines, mark_path};
pub use registration_mark_model::*;
mod drawing_above;
mod drawing_check;
mod drawing_content;
mod drawing_insert;
mod ellipse_arc;
mod error;
mod fixture_apply;
pub use fixture_apply::APPLY_PAGE_FIXTURES_COMMIT_LABEL;
mod fixture_job;
mod fixture_job_codec;
mod font_assets;
mod format_edit;
mod format_library;
mod format_library_file;
mod format_library_import;
mod group_tree;
mod groups;
mod guides;
mod history;
mod image_adjust_codec;
mod image_adjust_commands;
mod image_adjust_model;
mod image_codec;
mod image_crop;
mod image_model;
mod images;
mod junction;
mod layers;
mod legacy_fill;
mod library_fonts;
mod library_list;
pub use fixture_job::{
    ADD_FIXTURE_JOB_COMMIT_LABEL, ASSIGN_FIXTURE_TARGET_COMMIT_LABEL,
    DELETE_FIXTURE_JOB_COMMIT_LABEL, FixtureJobError, FixtureJobSwitch,
    MOVE_FIXTURE_JOB_COMMIT_LABEL, REMOVE_FIXTURE_TARGET_COMMIT_LABEL,
    RENAME_FIXTURE_JOB_COMMIT_LABEL, SET_FIXTURE_JOB_OUTPUT_OFF_COMMIT_LABEL,
    SET_FIXTURE_JOB_OUTPUT_ON_COMMIT_LABEL, SET_FIXTURE_JOB_SHOW_OFF_COMMIT_LABEL,
    SET_FIXTURE_JOB_SHOW_ON_COMMIT_LABEL,
};
pub use fixture_job_codec::{
    FixtureJobSnapshot, FixturePartKey, JobRecord, JobRole, JobTarget, PART_SUFFIXES, TargetRead,
};
mod job_fixture_commands;
mod manufacturing;
mod manufacturing_codec;
mod manufacturing_fixture_commands;
pub use job_fixture_commands::{
    ADD_JOB_FIXTURE_COMMIT_LABEL, LOCK_FIXTURE_COMMIT_LABEL, UNLOCK_FIXTURE_COMMIT_LABEL,
};
mod manufacturing_fixture_set_commands;
pub use manufacturing_fixture_set_commands::{
    ADD_FIXTURE_COMMIT_LABEL, ADD_FIXTURE_SET_COMMIT_LABEL, LOCK_FIXTURE_SET_COMMIT_LABEL,
    MOVE_FIXTURE_SET_COMMIT_LABEL, REMOVE_FIXTURE_SET_COMMIT_LABEL,
    RENAME_FIXTURE_SET_COMMIT_LABEL, SET_FIXTURE_SET_OFF_PAGE_COMMIT_LABEL,
    SET_FIXTURE_SET_ON_PAGE_COMMIT_LABEL, UNLOCK_FIXTURE_SET_COMMIT_LABEL,
};
mod manufacturing_fixture_align;
pub use manufacturing_fixture_align::AlignCommit;
mod manufacturing_fixture_sets;
pub use manufacturing_fixture_sets::{FixtureSetSnapshot, MAX_FIXTURE_SETS, MAX_JOB_FIXTURES};
mod manufacturing_fixtures;
mod manufacturing_slot_commands;
pub use manufacturing::{
    ADD_OUTPUT_PAGE_COMMIT_LABEL, AIR_ASSIST_OFF_COMMIT_LABEL, AIR_ASSIST_ON_COMMIT_LABEL,
    DELETE_PLACEMENT_COMMIT_LABEL, EDIT_DEFAULT_POWER_COMMIT_LABEL,
    EDIT_DEFAULT_SPEED_COMMIT_LABEL, EDIT_LASER_MODE_COMMIT_LABEL, EDIT_MAX_FEED_COMMIT_LABEL,
    EDIT_MAX_POWER_COMMIT_LABEL, EDIT_ORIGIN_CORNER_COMMIT_LABEL, EDIT_PROFILE_NAME_COMMIT_LABEL,
    FixtureParams, MOTORISED_Z_OFF_COMMIT_LABEL, MOTORISED_Z_ON_COMMIT_LABEL, ManufacturingError,
    ManufacturingSnapshot, NewOutputPage, PICK_PROFILE_COMMIT_LABEL, PageCorner, PlacementSnapshot,
    ProfileChange, ProfileSets, RESET_PROFILE_COMMIT_LABEL, SAVE_AS_PROFILE_COMMIT_LABEL,
    USE_MACHINE_SIZE_COMMIT_LABEL,
};
pub use manufacturing_fixture_commands::{
    ADD_CLAMP_COMMIT_LABEL, ADD_GRID_COMMIT_LABEL, ADD_HOLDER_COMMIT_LABEL, ADD_SCREW_COMMIT_LABEL,
    ADD_STOP_COMMIT_LABEL, DELETE_FIXTURE_COMMIT_LABEL, FixtureAdd, FixtureResize, FixtureTurn,
    MOVE_FIXTURE_COMMIT_LABEL, NewFixture, RENAME_FIXTURE_COMMIT_LABEL,
    RESIZE_FIXTURE_COMMIT_LABEL, ROTATE_FIXTURE_COMMIT_LABEL, SET_FIXTURE_ACTIVE_COMMIT_LABEL,
    SET_FIXTURE_INACTIVE_COMMIT_LABEL, SET_FIXTURE_PARAMS_COMMIT_LABEL,
    SET_FIXTURE_VISIBLE_COMMIT_LABEL, SET_GRID_MAJOR_COMMIT_LABEL, SET_GRID_ORIGIN_COMMIT_LABEL,
    SET_GRID_SPACING_COMMIT_LABEL, SET_TAPE_EDGE_COMMIT_LABEL,
};
pub use manufacturing_fixtures::{
    FixtureRecord, FixtureSetRecord, FixtureSnapshot, MAX_FIXTURE_NAME_CHARS,
    MAX_SLOTS_PER_FIXTURE, SlotRecord, SlotSnapshot,
};
pub use manufacturing_slot_commands::{
    ADD_SLOT_COMMIT_LABEL, REMOVE_SLOT_COMMIT_LABEL, RENAME_SLOT_COMMIT_LABEL,
    SET_SLOT_COMMIT_LABEL, SET_SLOT_POSITION_COMMIT_LABEL, SET_SLOT_SHAPE_COMMIT_LABEL,
    SET_SLOT_SIZE_COMMIT_LABEL,
};
mod laser_output_commands;
mod operation;
mod operation_codec;
mod operation_members;
mod operation_settings_commands;
mod test_grid_codec;
mod test_grid_commands;
mod test_grid_object;
mod test_grid_rows;
pub use laser_output_commands::{
    EXPORT_JOB_COMMIT_LABEL, SEND_JOB_COMMIT_LABEL, SET_FIXTURE_MATERIAL_COMMIT_LABEL,
    SET_START_HEAD_POSITION_COMMIT_LABEL, SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL,
    SET_START_MACHINE_ORIGIN_COMMIT_LABEL, SET_START_USER_ORIGIN_COMMIT_LABEL,
    SET_USER_ORIGIN_COMMIT_LABEL, TYPE_USER_ORIGIN_COMMIT_LABEL, USER_ORIGIN_LIMIT_UM,
};
pub use operation::{
    ADD_OPERATION_COMMIT_LABEL, ASSIGN_TO_OPERATION_COMMIT_LABEL, CUTS_LAST_OFF_COMMIT_LABEL,
    CUTS_LAST_ON_COMMIT_LABEL, DELETE_OPERATION_COMMIT_LABEL, MOVE_OPERATION_COMMIT_LABEL,
    NewOperation, OperationError, OperationSwitch, REMOVE_FROM_OPERATION_COMMIT_LABEL,
    RENAME_OPERATION_COMMIT_LABEL, SET_OPERATION_MARGIN_COMMIT_LABEL,
    SET_OPERATION_OUTPUT_COMMIT_LABEL, SET_OPERATION_SHOW_COMMIT_LABEL,
};
pub use operation_codec::OperationSnapshot;
pub use operation_settings_commands::{
    ADD_MATERIAL_COMMIT_LABEL, ADD_MATERIAL_SET_COMMIT_LABEL, CROP_TEST_PHOTO_COMMIT_LABEL,
    DELETE_MATERIAL_COMMIT_LABEL, DELETE_MATERIAL_SET_COMMIT_LABEL, DELETE_TEST_COMMIT_LABEL,
    EDIT_MATERIAL_COMMIT_LABEL, EDIT_OPERATION_SETTINGS_COMMIT_LABEL, EDIT_TEST_NOTE_COMMIT_LABEL,
    JUDGE_TEST_COMMIT_LABEL, MaterialStep, REMOVE_TEST_PHOTO_COMMIT_LABEL,
    RENAME_MATERIAL_COMMIT_LABEL, RESET_TO_MATERIAL_COMMIT_LABEL, SET_TEST_PHOTO_COMMIT_LABEL,
    SettingsStep, SettingsWrite, USE_MATERIAL_COMMIT_LABEL, USE_OWN_NUMBERS_COMMIT_LABEL,
};
pub use test_grid_codec::{GridItem, TestGridSnapshot};
pub use test_grid_commands::{
    ADD_TEST_GRID_COMMIT_LABEL, EDIT_TEST_GRID_COMMIT_LABEL, GridCellImage, GridItems, NewGridCell,
    NewGridLabel, TestGridError,
};
pub use test_grid_object::{
    DerivedPart, DerivedRow, GridStep, NewGridObject, PLACE_TEST_GRID_COMMIT_LABEL, TestGridWrite,
};
mod gradient_codec;
mod gradient_commands;
mod gradient_eval;
pub(crate) mod gradient_frame;
mod gradient_map;
mod gradient_measure;
mod gradient_model;
mod gradient_stops;
mod gradient_text;
mod node_edit;
mod node_move;
mod node_read;
mod node_registers;
mod object_read;
mod objects;
mod offset_write;
mod outline_map;
mod path_codec;
mod path_extend;
mod path_model;
mod path_reverse;
mod path_topology;
mod path_transform;
mod paths;
mod pattern_codec;
mod pattern_convert;
mod pattern_model;
mod pattern_schema;
mod pattern_values;
mod preset_units;
mod primitive_model;
mod primitive_outline;
mod replace;
mod ring_curve;
mod ring_random;
mod ring_rounded;
mod seed;
mod segment_bend;
mod shape_arc;
mod shape_arc_codec;
mod shape_codec;
mod shape_radii;
mod shapes;
mod smooth_handles;
mod snapping;
mod stroke_to_path_write;
mod style_codec;
mod style_model;
mod style_validation;
mod styles;
mod subpath_codec;
mod template;
mod text_carrier;
mod text_chars;
mod text_clone_codec;
mod text_clone_commands;
mod text_clone_read;
mod text_codec;
mod text_model;
mod text_path_codec;
mod text_path_commands;
mod text_path_model;
mod text_paths;
mod texts;
mod tiling_codec;
mod tiling_commands;
mod tiling_copy;
mod tiling_model;
mod tiling_pivot;
mod tiling_regenerate;
mod tiling_write;
mod transform_objects;
mod units;

pub use affine::{Affine, MATRIX_ENTRY_LIMIT, SINGULAR_DETERMINANT};
pub use assets::{
    AssetError, AssetHash, MAX_ASSET_BYTES, MAX_DERIVED_ASSET_BYTES, MAX_DERIVED_PROJECT_BYTES,
    MAX_PROJECT_ASSET_BYTES, asset_extension,
};
pub use clone_hold::CloneHold;
pub use clone_model::{CloneChain, CloneSnapshot, MAX_CLONE_DEPTH};
pub use clone_overrides::{
    CloneColours, FOLLOW_PLACEMENT_COMMIT_LABEL, ORIGINAL_STYLE_COMMIT_LABEL,
    OWN_PLACEMENT_COMMIT_LABEL, OWN_STYLE_COMMIT_LABEL, SPREAD_COLOURS_COMMIT_LABEL,
};
pub use clone_picture::map_leaf;
pub use clone_place::{PLACE_LINKED_CLONES_COMMIT_LABEL, PLACE_ON_PAGE_COMMIT_LABEL};
pub use clone_preview::clone_previews;
pub use clone_read::ObjectRead;
pub use clone_repair::CloneRepair;
pub use clone_transform::{CloneLinks, CloneMatrices, HeldClones, Hold, clone_matrices_after};
pub use clones::{
    CloneError, LINKED_CLONE_COMMIT_LABEL, REMOVE_ORPHAN_CLONES_COMMIT_LABEL,
    UNLINK_CLONE_COMMIT_LABEL,
};
pub use container::{CURRENT_LORO_SNAPSHOT_VERSION, pack, pack_recovery, pack_snapshot, unpack};
pub use container_ended::{ENDED_FORMAT_VERSION, EndReason, EndedRecord};
pub use container_recovery::{
    RecoveryAsset, assemble_recovery_copy, check_recovery_copy, recovery_assets,
};
pub use container_share::{
    CURRENT_SHARE_FORMAT_VERSION, ContainerShareMembers, WorkspaceRoomRecord, WorkspaceShareRecord,
    container_share_members,
};
pub use corner_radii::{Corner, CornerRadii, SHARP_CORNER_EPSILON_MM, effective_corner_radii};
pub use devices_list::{
    DEVICES_LIST_FORMAT_VERSION, DeviceEntry, DevicesList, DevicesListError, SyncedWorkspace,
};
pub use display_unit::DisplayUnit;
pub use document::{CURRENT_FORMAT_VERSION, Document, DocumentVersion};
pub use document_background::{BackgroundPaint, DocumentBackground};
pub use document_presets::{
    DocumentPreset, PRESET_MATCH_TOLERANCE, PresetError, PresetGroup, PresetList, PresetReason,
    PresetSubject, checked_name,
};
pub use document_size::{
    DocumentSizeError, MAX_DOCUMENT_MM, MIN_DOCUMENT_MM, validated_document_side,
};
pub use drawing_above::TRACE_IMAGE_COMMIT_LABEL;
pub use drawing_content::DrawingText;
pub use drawing_insert::{
    DrawingAnchor, DrawingNode, DrawingOutline, DrawingPage, DrawingPageFrame, DrawingRegisters,
    IMPORT_SVG_COMMIT_LABEL,
};
pub use ellipse_arc::{
    ArcKind, EllipseArc, EllipseAxes, Linear, WHOLE_ARC_TOLERANCE_DEG, arc_after_linear_map,
    arc_bounds, ellipse_after_linear_map, ellipse_point, rotated_arc_bounds, wrapped_angle,
};
pub use error::{OpenError, SaveError};
pub use format_edit::{FormatError, FormatField, FormatReason, FormatSpec, GroupTarget};
pub use format_library::FormatLibrary;
pub use format_library_file::{
    MAX_USER_FILE_BYTES, MAX_USER_FORMATS, MAX_USER_GROUPS, USER_FILE_FORMAT, describe_error,
};
pub use format_library_import::ImportReport;
pub use generated_codec::{GeneratedRecord, GeneratedSnapshot, orthonormalised};
pub use generated_commands::{
    ADD_GENERATED_COMMIT_LABEL, CHANGE_PARAMETER_COMMIT_LABEL, CONVERT_GENERATED_COMMIT_LABEL,
    GeneratedError, KEY_SUBJECT_KIND, KEY_SUBJECT_PARAM,
};
pub use group_tree::MAX_GROUP_DEPTH;
pub use groups::{GROUP_COMMIT_LABEL, UNGROUP_COMMIT_LABEL};
pub use guides::{GuideMarking, SET_GUIDE_COMMIT_LABEL};
pub use history::{
    Author, BranchMark, Direction, Event, FAR_STEP_OPS, GoToReport, HistoricalDocument,
    HistoryBranch, HistoryError, HistoryRow, LABEL_ALIGN_FIXTURE_SETS, LABEL_CLONE_VERSION,
    LABEL_GO_TO_VERSION, LABEL_MARK_TEST_GRID_LASERED, LABEL_MIGRATE_FORMAT, LABEL_NEW_DOCUMENT,
    LABEL_OBJECT_REDO, LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_REFRESH_MATERIAL_NUMBERS, LABEL_UNDO,
    LABEL_WIPE_OBJECT, MIN_SHOWN_ID_LEN, ObjectReport, ObjectRestore, ObjectResult, ObjectRow,
    ObjectRowClass, ObjectShape, ObjectStatus, ObjectTarget, ObjectTargets, OwnStep, RestoreReport,
    Role, RowRevert, RowVersion, SMALL_STEP_OPS, STACK_LIMIT, SkipReason, StackMachine, Status,
    StepKind, StepRef, SyncError, TouchedObject, WIPE_LABEL, is_document_level_label, shown_ids,
};
pub use image_adjust_model::{
    AdjustDamage, AdjustEntry, AdjustError, AdjustOwner, AdjustStep, AdjustmentsWrite,
    DEFAULT_IMAGE_STRENGTH, EDIT_ADJUSTMENTS_COMMIT_LABEL, EXPERT_PRESET,
    FOLLOW_ORIGINAL_COMMIT_LABEL, ImageAdjustments, MAX_IMAGE_STRENGTH,
    PASTE_ADJUSTMENTS_COMMIT_LABEL, RESTORE_ORIGINAL_COMMIT_LABEL, RUN_AI_DENOISE_COMMIT_LABEL,
    RUN_REMOVE_BACKGROUND_COMMIT_LABEL, RUN_UPSCALE_COMMIT_LABEL, RanStep, ShownAdjustments,
    StepResult, StoredDither,
};
pub use image_crop::{CROP_KIND, UnitRect, visible_rect};
pub use image_model::{
    IMPORT_COMMIT_LABEL, IMPORT_IMAGE_COMMIT_LABEL, IMPORT_IMAGES_COMMIT_LABEL, ImageSnapshot,
    PASTE_IMAGE_COMMIT_LABEL, no_paint,
};
pub use images::{ImageInsertError, NewImage};
mod trace_mask_codec;
mod trace_mask_commands;
mod trace_mask_model;
pub use gradient_commands::EDIT_GRADIENT_LABEL;
pub use gradient_eval::{GradientBox, PlacedGradient, PremultipliedRgba8, over_white_grey};
pub use gradient_frame::{carry_gradient, outlines_gradient_box, primitive_gradient_box};
pub use gradient_model::{
    AxisRatio, BoxFraction, BoxPoint, GradientFill, GradientGeometry, GradientKind,
    GradientParamError, GradientStop, GradientStops, StopOffset,
};
pub use junction::{COINCIDENT_MM, interior_handle, merged_junction};
pub use layers::{DELETE_LAYER_COMMIT_LABEL, INITIAL_LAYER_NAME, NEW_LAYER_COMMIT_LABEL};
pub use library_fonts::{
    CuttableChoice, FontBudgetError, FontDetails, FontDocument, FontEntry, MAX_FONT_DOCUMENT_BYTES,
    MAX_FONT_DOCUMENTS, MAX_SYNCED_FONT_BYTES, MAX_SYNCED_FONTS_BYTES,
};
pub use library_list::{
    LIBRARY_LIST_FORMAT_VERSION, LibraryEntry, LibraryKind, LibraryList, LibraryListError,
    MAX_MACHINE_TEXT_BYTES, MAX_MATERIAL_TEXT_BYTES, SnapshotImport,
};
pub use node_edit::{
    RENAME_NODE_COMMIT_LABEL, SET_NODE_LOCKED_COMMIT_LABEL, SET_NODE_OPACITY_COMMIT_LABEL,
    SET_NODE_VISIBLE_COMMIT_LABEL,
};
pub use node_move::{MOVE_NODES_COMMIT_LABEL, MovePlan, MoveRefusal, MoveTarget};
pub use node_read::{NodeKind, NodeRead};
pub use node_registers::{MAX_NAME_CHARS, NodeRegisters, clean_name};
pub use objects::{CopySource, ObjectEditError};
pub use offset_write::{OffsetGeometry, OffsetWrite};
pub use path_extend::{PathEnd, PathGrowth};
pub use path_model::{
    AnchorId, AnchorKind, AnchorSnapshot, Color, HandleSlot, NewAnchor, NodeId, PathEditError,
    PathSnapshot, SubpathRef, SubpathSnapshot,
};
pub use path_reverse::reversed_anchors;
pub use paths::resolve_handle_pair;
pub use pattern_convert::{CONVERT_PATTERN_COMMIT_LABEL, PatternConversion};
pub use pattern_model::{
    FillPaint, GeneratorKind, PatternCount, PatternFill, PatternFraction, PatternGenerator,
    PatternMode, PatternParam, PatternParamError,
};
pub use pattern_schema::{MIN_LENGTH_MM, ParamKind, ParamSpec, spec_of, specs as pattern_params};
pub use placement_commands::{
    PLACE_PAGE_COMMIT_LABEL, PLACED_PAGE_MIN_OVERLAP_MM2, PlacedPage, overlaps_page,
};
pub use preset_units::{AuthoredSize, Orientation, PresetUnit};
pub use primitive_model::{
    EllipseFrame, InnerRatio, ObjectSnapshot, PointCount, PrimitiveSnapshot, RectBounds, Shape,
    ShapeParamError, StarFrame, is_beyond_ellipse, shape_center, shape_frame_bounds,
    translate_shape,
};
pub use primitive_outline::{
    KAPPA, OutlineAnchor, PrimitiveOutline, drawing_outline_of, drawing_outline_of_rotated,
    ellipse_outline, outline_of, outline_of_rotated, polygon_outline, rect_outline, star_outline,
};
pub use replace::{PieceFill, PieceOutlines};
pub use ring_curve::{
    Curve, curve_from_position, curve_handle_position, curve_limit, curve_max, effective_curve,
    ring_bounds, ring_extent,
};
pub use ring_random::Randomized;
pub use ring_rounded::{Q_C, Rounded, effective_rounded, rounded_bounds};
pub use seed::{Seed, splitmix64};
pub use shapes::ShapeEditError;
pub use smooth_handles::smooth_corner_handles;
pub use snapping::{DocumentSnapping, GridSnapPoint};
pub use stroke_to_path_write::{
    STROKE_TO_PATH_COMMIT_LABEL, StrokeOutlineWrite, StrokeSubject, StrokeToPath,
    stroke_outline_style,
};
pub use style_model::{
    DashPattern, Fill, LineCap, LineJoin, MarkerCount, MarkerPlace, MarkerShape, Markers, Opacity,
    Stroke, Style, StyleParamError,
};
pub use styles::{StyleEdit, StyleEditError};
pub use template::{LOAD_TEMPLATE_COMMIT_LABEL, TemplateNode};
pub use text_carrier::{DUPLICATE_TEXT_SHIFT, carrier_curve, text_path_previews};
pub use text_chars::{CharValues, KERN_X_FACTORS, KERN_Y_FACTORS, TextChars};
pub use text_clone_codec::TextSource;
pub use text_clone_commands::PUT_ON_PATHS_LABEL;
pub use text_model::{
    FontKind, FontRef, LETTER_SPACING_FACTORS, LINE_SPACING_FACTORS, MAX_STORED_TEXT_CHARS,
    MAX_TEXT_CHARS, MAX_TEXT_SIZE, MIN_TEXT_SIZE, MISSING_FACE_HASH, SINGLE_LINE_STROKE_WIDTH,
    TAPER_FRACTIONS, TEXT_NAME_CHARS, TextAlign, TextGlyphOrientation, TextLayoutValues,
    TextOutline, TextSnapshot, TextWritingMode, UNCREATED_TEXT_ID, WORD_SPACING_FACTORS,
    clamp_text_size, is_face_hash, single_line_style,
};
pub use text_path_model::{
    CarrierCurve, CarrierRef, EllipseCarrier, LineOnPath, OnPathLabel, OnPathValues,
    PUT_ON_PATH_LABEL, PathSide, RELEASE_FROM_PATH_LABEL, TextOnPath,
};
pub use text_paths::TextConversion;
pub use texts::{TextCharsLabel, TextEditError, TextLayoutLabel};
pub use tiling_commands::{StoredTiling, TilingOutcome};
pub use tiling_model::{
    CellKey, Channel, CircleOptions, EDIT_TILING_COMMIT_LABEL, GridOptions, OffsetLines, Percent,
    RELEASE_TILING_COMMIT_LABEL, RecipeError, SpacingMode, TILE_COMMIT_LABEL, TILING_GROUP_NAME,
    TILING_MAX_COUNT, TILING_WARN_COUNT, TilingCell, TilingError, TilingLayout, TilingRecipe,
    TilingRequest, TilingResult, Variance, hue_shifted,
};
pub use trace_mask_codec::FIRST_TRACE_PART;
pub use trace_mask_model::{
    ADD_TRACE_PART_LABEL, CLEAR_TRACE_MASK_LABEL, DELETE_TRACE_REGION_LABEL,
    DRAW_TRACE_LASSO_LABEL, DRAW_TRACE_RECTANGLE_LABEL, EDIT_TRACE_SETTINGS_LABEL,
    ImplicitTracePart, MOVE_TRACE_LASSO_CORNER_LABEL, MOVE_TRACE_REGION_LABEL, NewTraceRegion,
    REMOVE_TRACE_PART_LABEL, RESIZE_TRACE_REGION_LABEL, TRACE_MASK_INSIDE_LABEL, TRACE_MASK_LABELS,
    TRACE_MASK_OUTSIDE_LABEL, TraceMask, TraceMaskError, TraceMaskPart, TraceMaskRegion,
    TracePartRef, TracePartSettings, TraceRegionEdit, TraceRegionKind, UNIT_LIMIT, UnitPoint,
};
pub use units::{Angle, DocumentSize, Length, Point, Timestamp, Tolerance, Vec2, ViewTransform};
pub use workspace_disk_plan::{DiskOp, DiskPlan, DiskState, ListPlace, ScannedDocument, plan_disk};
pub use workspace_list::{
    Created, ListError, ListImportReport, ListNodeId, WORKSPACE_LIST_FORMAT_VERSION, WorkspaceList,
};
pub use workspace_list_view::{DeletedEntry, ListEntry};
pub use workspace_names::{
    MAX_ENTRY_NAME_CHARS, NameRefusal, check_entry_name, copy_entry_name, free_entry_name,
    same_entry_name,
};
pub use workspace_plan::{
    DOCUMENT_EXTENSION, EntryKind, FileOp, PlanRefusal, Planned, WorkspacePath, WorkspaceTree,
    document_name,
};

mod sketch_model;
pub use sketch_model::{SketchCheckpoint, SketchDelta, SketchEntityRecord, SketchSnapshot};
mod sketch_codec;
mod sketch_commands;
mod sketch_copy;
mod sketch_labels;
mod sketch_outline;
mod sketch_page;
mod sketch_page_commands;
pub use sketch_page_commands::{
    CONVERT_SKETCH_PAGE_COMMIT_LABEL, DELETE_SKETCH_PAGE_COMMIT_LABEL,
    PLACE_SKETCH_CLONE_COMMIT_LABEL,
};
mod sketch_parameter_repair;
mod sketch_read;
pub use sketch_commands::SketchEditError;
#[cfg(test)]
mod sketch_cache_tests;
#[cfg(test)]
mod sketch_fixtures;
#[cfg(test)]
mod sketch_tests;

pub use sketch_outline::{outline as sketch_outline, preview_outline as sketch_preview_outline};
