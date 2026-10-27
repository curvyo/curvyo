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

mod font_archive_glue;
mod font_checks_glue;
mod font_file_glue;
mod font_library_glue;
mod font_preview_glue;
mod session;

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
mod wasm_boolean;
#[cfg(target_arch = "wasm32")]
mod wasm_clone;
#[cfg(target_arch = "wasm32")]
mod wasm_combine;
#[cfg(target_arch = "wasm32")]
mod wasm_cut;
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
mod wasm_offset;
#[cfg(target_arch = "wasm32")]
mod wasm_pen;
#[cfg(target_arch = "wasm32")]
mod wasm_pen_entry;
#[cfg(target_arch = "wasm32")]
mod wasm_properties_panel;
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
mod wasm_svg_import;
#[cfg(target_arch = "wasm32")]
mod wasm_text;
#[cfg(target_arch = "wasm32")]
mod wasm_text_layout;
#[cfg(target_arch = "wasm32")]
mod wasm_text_path;
#[cfg(target_arch = "wasm32")]
mod wasm_trace;
#[cfg(target_arch = "wasm32")]
mod wasm_weld;

pub use font_archive_glue::FontArchiveGlue;
pub use font_checks_glue::FontChecksGlue;
pub use font_file_glue::{face_preview_path, font_file_faces};
pub use font_library_glue::FontLibraryGlue;
pub use font_preview_glue::PreviewFacesGlue;
pub use session::{
    APP_VERSION, BooleanOutcome, BreakApartOutcome, CloneOutcome, ClosePathOutcome, ClosePathState,
    CombineOutcome, CutOutcome, DocumentPresetsRecord, DocumentSide, DoubleClickHint, DropPlace,
    EscapeStep, FAR_PREVIEW_ROWS, FitOutcome, FormTexts, FormatEdit, GoToOutcome, GroupingKind,
    GroupingResult, HistoryResult, HistoryRowPress, KeyHint, KeyInput, KeyOutcome, MoveIndicators,
    ObjectWipeOutcome, PanelReply, PanelTabsRecord, PanelView, PenJoinNotice, RowFlag, RowPress,
    Session, SizeOutcome, SplitOutcome, StackOutcome, Tool, WipeOutcome, WipeState,
};
pub use session::{DragStop, OFFSET_COMMIT_LABEL, OffsetOutcome, OffsetView};
pub use session::{FrameImages, ImportOutcome, StrokeToPathOutcome, WeldOutcome};
pub use session::{LayoutOutcome, ObjectToPathOutcome, StyleOutcome, TextBarView, TextSizeOutcome};
pub use session::{TextPathPress, TextPathView};
pub use session::{TraceCreateOutcome, TraceSelection, TraceSource, TraceView};

#[cfg(target_arch = "wasm32")]
pub use wasm_api::{WasmSession, init_panic_hook};
