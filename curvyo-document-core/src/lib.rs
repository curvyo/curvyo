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

mod affine;
mod clone_codec;
mod clone_copy;
mod clone_hold;
mod clone_model;
mod clone_overrides;
mod clone_preview;
mod clone_read;
mod clone_repair;
mod clone_transform;
mod clone_write;
mod clones;
mod container;
mod corner_radii;
mod corner_radii_codec;
mod display_unit;
mod document;
mod document_background;
mod document_presets;
mod document_size;
mod ellipse_arc;
mod error;
mod format_edit;
mod format_library;
mod format_library_file;
mod format_library_import;
mod group_tree;
mod groups;
mod history;
mod junction;
mod layers;
mod legacy_fill;
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
mod preset_units;
mod primitive_model;
mod primitive_outline;
mod replace;
mod ring_curve;
mod segment_bend;
mod shape_arc;
mod shape_arc_codec;
mod shape_codec;
mod shape_radii;
mod shapes;
mod smooth_handles;
mod style_codec;
mod style_model;
mod style_validation;
mod styles;
mod subpath_codec;
mod transform_objects;
mod units;

pub use affine::{Affine, MATRIX_ENTRY_LIMIT, SINGULAR_DETERMINANT};
pub use clone_hold::CloneHold;
pub use clone_model::{CloneChain, CloneSnapshot, MAX_CLONE_DEPTH};
pub use clone_overrides::{
    CloneColours, FOLLOW_PLACEMENT_COMMIT_LABEL, ORIGINAL_STYLE_COMMIT_LABEL,
    OWN_PLACEMENT_COMMIT_LABEL, OWN_STYLE_COMMIT_LABEL, SPREAD_COLOURS_COMMIT_LABEL,
};
pub use clone_preview::clone_previews;
pub use clone_read::ObjectRead;
pub use clone_repair::CloneRepair;
pub use clone_transform::{CloneLinks, CloneMatrices, clone_matrices_after};
pub use clones::{
    CloneError, LINKED_CLONE_COMMIT_LABEL, REMOVE_ORPHAN_CLONES_COMMIT_LABEL,
    UNLINK_CLONE_COMMIT_LABEL,
};
pub use container::{CURRENT_LORO_SNAPSHOT_VERSION, pack, unpack};
pub use corner_radii::{Corner, CornerRadii, SHARP_CORNER_EPSILON_MM, effective_corner_radii};
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
pub use ellipse_arc::{
    ArcKind, EllipseArc, EllipseAxes, Linear, WHOLE_ARC_TOLERANCE_DEG, arc_after_linear_map,
    arc_bounds, ellipse_after_linear_map, ellipse_point, wrapped_angle,
};
pub use error::{OpenError, SaveError};
pub use format_edit::{FormatError, FormatField, FormatReason, FormatSpec, GroupTarget};
pub use format_library::FormatLibrary;
pub use format_library_file::{
    MAX_USER_FILE_BYTES, MAX_USER_FORMATS, MAX_USER_GROUPS, USER_FILE_FORMAT, describe_error,
};
pub use format_library_import::ImportReport;
pub use group_tree::MAX_GROUP_DEPTH;
pub use groups::{GROUP_COMMIT_LABEL, UNGROUP_COMMIT_LABEL};
pub use history::{
    Author, BranchMark, Direction, Event, FAR_STEP_OPS, GoToReport, HistoricalDocument,
    HistoryBranch, HistoryError, HistoryRow, LABEL_CLONE_VERSION, LABEL_GO_TO_VERSION,
    LABEL_NEW_DOCUMENT, LABEL_OBJECT_REDO, LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_UNDO,
    LABEL_WIPE_OBJECT, MIN_SHOWN_ID_LEN, ObjectReport, ObjectRestore, ObjectResult, ObjectRow,
    ObjectRowClass, ObjectShape, ObjectStatus, ObjectTarget, ObjectTargets, OwnStep, RestoreReport,
    Role, RowRevert, RowVersion, SMALL_STEP_OPS, STACK_LIMIT, SkipReason, StackMachine, Status,
    StepKind, StepRef, SyncError, TouchedObject, WIPE_LABEL, is_document_level_label, shown_ids,
};
pub use junction::{COINCIDENT_MM, interior_handle, merged_junction};
pub use layers::{DELETE_LAYER_COMMIT_LABEL, INITIAL_LAYER_NAME, NEW_LAYER_COMMIT_LABEL};
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
pub use preset_units::{AuthoredSize, Orientation, PresetUnit};
pub use primitive_model::{
    EllipseFrame, InnerRatio, ObjectSnapshot, PointCount, PrimitiveSnapshot, RectBounds, Shape,
    ShapeParamError, StarFrame, shape_center, shape_frame_bounds, translate_shape,
};
pub use primitive_outline::{
    KAPPA, OutlineAnchor, PrimitiveOutline, drawing_outline_of, drawing_outline_of_rotated,
    ellipse_outline, outline_of, outline_of_rotated, polygon_outline, rect_outline, star_outline,
};
pub use replace::{PieceFill, PieceOutlines};
pub use ring_curve::{
    Curve, curve_from_position, curve_handle_position, curve_limit, effective_curve, ring_extent,
};
pub use shapes::ShapeEditError;
pub use smooth_handles::smooth_corner_handles;
pub use style_model::{
    DashPattern, Fill, LineCap, LineJoin, MarkerCount, MarkerPlace, MarkerShape, Markers, Opacity,
    Stroke, Style, StyleParamError,
};
pub use styles::{StyleEdit, StyleEditError};
pub use units::{Angle, DocumentSize, Length, Point, Timestamp, Tolerance, Vec2, ViewTransform};
