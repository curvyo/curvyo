//! The Trace tool's `wasm-bindgen` surface (`specs/0025-raster-trace`, ADR 0023 §5): the free
//! functions the trace worker calls (`trace_job`, `trace_auto_threshold`, the settings schema)
//! and the session's view, pixels, preview, results and Create paths. Images cross the boundary
//! as the JSON of their node id; the sentences are the host's (ADR 0001 §5).

use curvyo_document_core::NodeId;
use wasm_bindgen::prelude::*;

use crate::session::{TraceCreateOutcome, TraceSelection, TraceViewMode};
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn id_text(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap_or_default()
}

fn id_of(text: &str) -> Option<NodeId> {
    serde_json::from_str(text).ok()
}

#[wasm_bindgen]
extern "C" {
    /// Where the worker hears how far a job is (criterion 10): an object with `report(percent)`.
    pub type TraceProgressSink;

    #[wasm_bindgen(method)]
    fn report(this: &TraceProgressSink, percent: u8);
}

/// One trace for the worker (criterion 17): `rgba` (`width` x `height`, straight alpha) mapped
/// by `map` (`[a, b, c, d, e, f]`) with `settings` (JSON: `threshold`, `smooth`, `specks` in mm,
/// `corners`, `upscale` and `denoise` as choice indices, `invert`, `ignore_background`); the
/// result as the flat numbers of `curvyo_ui_core::trace_job_with`. `progress` hears each higher
/// whole percent (criterion 10). Never throws: bad input reads as the status
/// "invalid".
#[wasm_bindgen]
#[must_use]
pub fn trace_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
    progress: &TraceProgressSink,
) -> Vec<f64> {
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    curvyo_ui_core::trace_job_with(
        rgba,
        width,
        height,
        map,
        &crate::trace_glue::trace_settings(settings),
        &mut |percent| {
            progress.report(percent);
        },
    )
}

/// The presets as JSON (criteria 59 and 60), in menu order: a list of `{key, label, detail,
/// values: {smooth, specks, corners, upscale, denoise}}`.
#[wasm_bindgen]
#[must_use]
pub fn trace_presets() -> String {
    crate::trace_glue::trace_presets_json()
}

/// The automatic Threshold of a picture (criterion 9), for the worker.
#[wasm_bindgen]
#[must_use]
pub fn trace_auto_threshold(rgba: &[u8], width: u32, height: u32) -> u8 {
    curvyo_ui_core::trace_auto_threshold(rgba, width, height)
}

/// The settings schema as JSON (criteria 6 and 45): a list of `{name, label, kind, unit, min,
/// max, default, step, big_step, fine_step, group, advanced, choices}` the bar's fields and the
/// Advanced block are made from, the bar's first, then the Advanced block's in its order.
#[wasm_bindgen]
#[must_use]
pub fn trace_params() -> String {
    crate::trace_glue::trace_params_json()
}

/// What the Trace bar reads.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TraceViewJs {
    /// `"empty"`, `"other"` or `"images"`.
    #[wasm_bindgen(getter_with_clone)]
    pub selection: String,
    /// The selected images' ids, in selection order.
    #[wasm_bindgen(getter_with_clone)]
    pub images: Vec<String>,
    /// The id of the image the preview shows, or empty.
    #[wasm_bindgen(getter_with_clone)]
    pub shown: String,
}

/// One image's pixels and map for the worker.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TraceSourceJs {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight-alpha RGBA8.
    #[wasm_bindgen(getter_with_clone)]
    pub rgba: Vec<u8>,
    /// `[a, b, c, d, e, f]`.
    #[wasm_bindgen(getter_with_clone)]
    pub map: Vec<f64>,
}

/// What `create_traced_paths` did: `kind` `"ignored"` or `"created"` with the counts of the
/// notice (criteria 13, 46).
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TraceCreateJs {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Images traced.
    pub images: u32,
    /// Outlines written.
    pub paths: u32,
    /// Anchors written.
    pub nodes: u32,
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Trace bar shows (criteria 1 to 4, 46).
    #[must_use]
    pub fn trace_view(&self) -> TraceViewJs {
        let view = self.session.trace_view();
        TraceViewJs {
            selection: match view.selection {
                TraceSelection::Empty => "empty",
                TraceSelection::Other => "other",
                TraceSelection::Images => "images",
            }
            .to_string(),
            images: view.images.into_iter().map(id_text).collect(),
            shown: view.shown.map(id_text).unwrap_or_default(),
        }
    }

    /// Whether the Select bar shows the Trace button (criterion 1).
    #[must_use]
    pub fn trace_available(&self) -> bool {
        self.session.trace_available()
    }

    /// The pixels and map of the selected image `id` (a reduced copy with `reduced`), or
    /// `undefined` when its picture is not readable.
    #[must_use]
    pub fn trace_source(&mut self, id: &str, reduced: bool) -> Option<TraceSourceJs> {
        let source = self.session.trace_source(id_of(id)?, reduced)?;
        Some(TraceSourceJs {
            width: source.width,
            height: source.height,
            rgba: source.rgba,
            map: source.map.to_vec(),
        })
    }

    /// Shows the worker's result `wire` for image `id` as the preview; `false` when it is no
    /// result.
    pub fn set_trace_preview(&mut self, id: &str, wire: &[f64]) -> bool {
        id_of(id).is_some_and(|id| self.session.set_trace_preview(id, wire))
    }

    /// Marks the preview out of date (a stopped job) or current again.
    pub fn set_trace_preview_stale(&mut self, stale: bool) {
        self.session.set_trace_preview_stale(stale);
    }

    /// Keeps the worker's full-size result `wire` for image `id` for Create paths.
    pub fn set_trace_result(&mut self, id: &str, wire: &[f64]) -> bool {
        id_of(id).is_some_and(|id| self.session.set_trace_result(id, wire))
    }

    /// Forgets the full-size results (a value changed).
    pub fn clear_trace_results(&mut self) {
        self.session.clear_trace_results();
    }

    /// The view on the canvas, as the View pill shows it (criterion 51): `"overlay"`,
    /// `"trace"` or `"image"`; Image while `\` is held.
    #[must_use]
    pub fn trace_view_mode(&self) -> String {
        match self.session.shown_trace_view_mode() {
            TraceViewMode::Overlay => "overlay",
            TraceViewMode::Trace => "trace",
            TraceViewMode::Image => "image",
        }
        .to_string()
    }

    /// Chooses the view `mode` (`"overlay"`, `"trace"` or `"image"`; anything else is ignored).
    pub fn set_trace_view_mode(&mut self, mode: &str) {
        let mode = match mode {
            "overlay" => TraceViewMode::Overlay,
            "trace" => TraceViewMode::Trace,
            "image" => TraceViewMode::Image,
            _ => return,
        };
        self.session.set_trace_view_mode(mode);
    }

    /// Shift+`\`: the next view.
    pub fn cycle_trace_view_mode(&mut self) {
        self.session.cycle_trace_view_mode();
    }

    /// `\` held (`true`) or released.
    pub fn set_trace_peek(&mut self, held: bool) {
        self.session.set_trace_peek(held);
    }

    /// The view is moving (`true`) or has rested for 150 ms (`false`): while it moves, the Trace
    /// tool's kept meshes are drawn at any zoom (criterion 56).
    pub fn set_trace_view_moving(&mut self, moving: bool) {
        self.session.set_trace_view_moving(moving);
    }

    /// Create paths over every selected image, one step (criteria 12, 13, 46).
    pub fn create_traced_paths(&mut self) -> TraceCreateJs {
        match self.session.create_traced_paths() {
            TraceCreateOutcome::Ignored => TraceCreateJs {
                kind: "ignored".to_string(),
                ..TraceCreateJs::default()
            },
            TraceCreateOutcome::Created {
                images,
                paths,
                nodes,
            } => TraceCreateJs {
                kind: "created".to_string(),
                images: count(images),
                paths: count(paths),
                nodes: count(nodes),
            },
        }
    }
}
