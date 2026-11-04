//! The Curvyo wasm editor facade (ADR 0001 §3): binds
//! `curvyo-document-core`, `curvyo-ui-core` and `curvyo-render-core`
//! and owns the `wgpu` device/surface and GPU submission. No editing
//! logic of its own — the private `session` module (its public surface
//! is [`Session`] and [`Tool`]) is a thin orchestration layer over the
//! three crates above, and the private `wasm_api`/`gpu` modules
//! (wasm32-only, public surface `WasmSession`) are a thin
//! `wasm-bindgen`/`wgpu` shell over it.
//!
//! `session` is plain Rust and exercised by ordinary `cargo test` on the
//! host; `wasm_api` and `gpu` only compile for `wasm32` (`Cargo.toml`
//! makes `wgpu`/`wasm-bindgen`/`web-sys` wasm32-only dependencies), so
//! this crate's gate-required `cargo build --target wasm32-unknown-
//! unknown` is what actually type-checks them — there is no browser in
//! this environment to run them in.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod background_writer;
mod clipboard_glue;
mod device_group_glue;
mod font_archive_glue;
mod font_checks_glue;
mod font_file_glue;
mod font_library_glue;
mod font_preview_glue;
mod ocr_pool_glue;
mod pairing_glue;
mod session;
mod trace_analysis_glue;
mod trace_fit_glue;
mod trace_glue;
mod workspace_glue;
mod workspace_share_builder;
mod workspace_tree_glue;

#[cfg(target_arch = "wasm32")]
mod gpu;
#[cfg(target_arch = "wasm32")]
mod gpu_images;
#[cfg(target_arch = "wasm32")]
mod gpu_pipeline;
#[cfg(target_arch = "wasm32")]
mod wasm_api;
#[cfg(target_arch = "wasm32")]
mod wasm_background;
#[cfg(target_arch = "wasm32")]
mod wasm_background_writer;
#[cfg(target_arch = "wasm32")]
mod wasm_backup;
#[cfg(target_arch = "wasm32")]
mod wasm_boolean;
#[cfg(target_arch = "wasm32")]
mod wasm_clone;
#[cfg(target_arch = "wasm32")]
mod wasm_combine;
#[cfg(target_arch = "wasm32")]
mod wasm_cut;
#[cfg(target_arch = "wasm32")]
mod wasm_devices;
#[cfg(target_arch = "wasm32")]
mod wasm_devices_text;
#[cfg(target_arch = "wasm32")]
mod wasm_document;
#[cfg(target_arch = "wasm32")]
mod wasm_font_library;
#[cfg(target_arch = "wasm32")]
mod wasm_formats;
#[cfg(target_arch = "wasm32")]
mod wasm_fracture;
#[cfg(target_arch = "wasm32")]
mod wasm_group;
#[cfg(target_arch = "wasm32")]
mod wasm_history;
#[cfg(target_arch = "wasm32")]
mod wasm_image_clipboard;
#[cfg(target_arch = "wasm32")]
mod wasm_image_import;
#[cfg(target_arch = "wasm32")]
mod wasm_keys;
#[cfg(target_arch = "wasm32")]
mod wasm_layers;
#[cfg(target_arch = "wasm32")]
mod wasm_move;
#[cfg(target_arch = "wasm32")]
mod wasm_move_entry;
#[cfg(target_arch = "wasm32")]
mod wasm_navigation;
#[cfg(target_arch = "wasm32")]
mod wasm_node_tool;
#[cfg(target_arch = "wasm32")]
mod wasm_ocr_pool;
#[cfg(target_arch = "wasm32")]
mod wasm_offset;
#[cfg(target_arch = "wasm32")]
mod wasm_pages;
#[cfg(target_arch = "wasm32")]
mod wasm_pattern;
#[cfg(target_arch = "wasm32")]
mod wasm_pen;
#[cfg(target_arch = "wasm32")]
mod wasm_pen_entry;
#[cfg(target_arch = "wasm32")]
mod wasm_properties_panel;
#[cfg(target_arch = "wasm32")]
mod wasm_recognise;
#[cfg(target_arch = "wasm32")]
mod wasm_render;
#[cfg(target_arch = "wasm32")]
mod wasm_ruler;
#[cfg(target_arch = "wasm32")]
mod wasm_select_bar;
#[cfg(target_arch = "wasm32")]
mod wasm_select_bar_arc;
#[cfg(target_arch = "wasm32")]
mod wasm_select_tool;
#[cfg(target_arch = "wasm32")]
mod wasm_shape_tools;
#[cfg(target_arch = "wasm32")]
mod wasm_split;
#[cfg(target_arch = "wasm32")]
mod wasm_stroke_to_path;
#[cfg(target_arch = "wasm32")]
mod wasm_svg_export;
#[cfg(target_arch = "wasm32")]
mod wasm_svg_import;
#[cfg(target_arch = "wasm32")]
mod wasm_sync;
#[cfg(target_arch = "wasm32")]
mod wasm_text;
#[cfg(target_arch = "wasm32")]
mod wasm_text_layout;
#[cfg(target_arch = "wasm32")]
mod wasm_text_path;
#[cfg(target_arch = "wasm32")]
mod wasm_thumbnail;
#[cfg(target_arch = "wasm32")]
mod wasm_tiling;
#[cfg(target_arch = "wasm32")]
mod wasm_trace;
#[cfg(target_arch = "wasm32")]
mod wasm_trace_region;
#[cfg(target_arch = "wasm32")]
mod wasm_weld;
#[cfg(target_arch = "wasm32")]
mod wasm_workspace;
#[cfg(target_arch = "wasm32")]
mod wasm_workspace_sync;
#[cfg(target_arch = "wasm32")]
mod wasm_workspace_take_in;
#[cfg(target_arch = "wasm32")]
mod wasm_workspace_tree;

pub use background_writer::{BackgroundWriter, WRITE_INTERVAL_MS, Waiting, WriteError, Written};
pub use clipboard_glue::{clipboard_notice, encode_rgba_png, paste_rank_code};
pub use device_group_glue::{DeviceGroupGlue, GroupFiles};
pub use font_archive_glue::FontArchiveGlue;
pub use font_checks_glue::FontChecksGlue;
pub use font_file_glue::{face_preview_path, font_file_faces};
pub use font_library_glue::FontLibraryGlue;
pub use font_preview_glue::PreviewFacesGlue;
pub use ocr_pool_glue::{crops_wire, read_crops, read_crops_wire};
pub use pairing_glue::{PairPhase, PairingGlue};
pub use session::SvgExportResult;
pub use session::{
    APP_VERSION, BooleanOutcome, BreakApartOutcome, CloneOutcome, ClosePathOutcome, ClosePathState,
    CombineOutcome, CutOutcome, DocumentPresetsRecord, DocumentSide, DoubleClickHint, DropPlace,
    EscapeStep, FAR_PREVIEW_ROWS, FitOutcome, FormTexts, FormatEdit, GoToOutcome, GroupingKind,
    GroupingResult, HistoryResult, HistoryRowPress, KeyHint, KeyInput, KeyOutcome, MoveIndicators,
    ObjectWipeOutcome, PanelReply, PanelTabsRecord, PanelView, PenJoinNotice, RowFlag, RowPress,
    Session, SizeOutcome, SnappingSettingsRecord, SnappingToolbarRecord, SplitOutcome,
    StackOutcome, Tool, WipeOutcome, WipeState,
};
pub use session::{
    BackgroundItem, BackgroundKind, DiskAnswer, DiskScan, DiskStep, DocumentPlace, ListEdit,
    MarkerFiles, PreparedShare, SyncSeeds, TakeIn, TransportAction, WorkspaceLink, WorkspaceWrite,
    take_in_allowed, take_in_check,
};
pub use session::{DragStop, OFFSET_COMMIT_LABEL, OffsetOutcome, OffsetView};
pub use session::{
    FrameImages, ImageCopy, ImportOrigin, ImportOutcome, StrokeToPathOutcome, WeldOutcome,
};
pub use session::{LayoutOutcome, ObjectToPathOutcome, StyleOutcome, TextBarView, TextSizeOutcome};
pub use session::{RecogniseOutcome, RecognisedLine};
pub use session::{TextPathPress, TextPathView};
pub use session::{TilingStatus, circle_handle};
pub use session::{
    TraceCreateOutcome, TracePickMode, TraceSelection, TraceSource, TraceView, TraceViewMode,
    TracedText,
};
pub use session::{align_command_from_code, distribute_from_code};
pub use trace_analysis_glue::analysis_json as trace_analysis_json;
pub use trace_fit_glue::{FitTextRead, FitWorker, read_fit_result};
pub use trace_glue::{
    hex as trace_hex, ocr_lines_json, read_ocr_lines, recognise_shapes as trace_recognise_shapes,
    trace_colour_settings, trace_mask_region, trace_params_json, trace_presets_json,
    trace_settings,
};
pub use workspace_share_builder::{NotAdded, ShareFiles, WorkspaceShareBuilder};

#[cfg(target_arch = "wasm32")]
pub use wasm_api::{WasmSession, init_panic_hook};

pub use session::{SketchTool, Workbench};
pub use workspace_glue::{WorkspaceGlue, workspace_copy_bytes};
pub use workspace_tree_glue::{deleted_json, disk_answer_json, disk_scan, list_edit, stamps};

#[cfg(target_arch = "wasm32")]
mod wasm_machine_profiles;
#[cfg(target_arch = "wasm32")]
mod wasm_manufacturing;
#[cfg(target_arch = "wasm32")]
mod wasm_measure;
#[cfg(target_arch = "wasm32")]
mod wasm_sketch;
