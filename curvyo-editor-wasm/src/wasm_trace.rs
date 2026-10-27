//! The Trace tool's `wasm-bindgen` surface (`specs/0025-raster-trace`, ADR 0023 §5): the free
//! functions the trace worker calls (`trace_job`, `trace_auto_threshold`, the settings schema)
//! and the session's view, pixels, preview, results and Create paths. Images cross the boundary
//! as the JSON of their node id; the sentences are the host's (ADR 0001 §5).

use curvyo_document_core::NodeId;
use curvyo_ui_core::{TRACE_PARAMS, TraceParamKind, TraceParamUnit};
use wasm_bindgen::prelude::*;

use crate::session::{TraceCreateOutcome, TraceSelection};
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

/// One trace for the worker (criterion 17): `rgba` (`width` x `height`, straight alpha) mapped
/// by `map` (`[a, b, c, d, e, f]`) with the bar's values; the result as the flat numbers of
/// `curvyo_ui_core::trace_job`. Never throws: bad input reads as the status "invalid".
#[wasm_bindgen]
#[must_use]
pub fn trace_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    threshold: u8,
    smooth: u8,
    specks_mm: f64,
) -> Vec<f64> {
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    curvyo_ui_core::trace_job(rgba, width, height, map, threshold, smooth, specks_mm)
}

/// The automatic Threshold of a picture (criterion 9), for the worker.
#[wasm_bindgen]
#[must_use]
pub fn trace_auto_threshold(rgba: &[u8], width: u32, height: u32) -> u8 {
    curvyo_ui_core::trace_auto_threshold(rgba, width, height)
}

/// The settings schema as JSON (criterion 6): a list of `{name, label, kind, unit, min, max,
/// default, step, big_step, fine_step, group, advanced}` the bar's fields are made from.
#[wasm_bindgen]
#[must_use]
pub fn trace_params() -> String {
    let list: Vec<serde_json::Value> = TRACE_PARAMS
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "label": p.label,
                "kind": match p.kind { TraceParamKind::Integer => "integer", TraceParamKind::Length => "length" },
                "unit": match p.unit { TraceParamUnit::None => "", TraceParamUnit::Millimetre => "mm" },
                "min": p.min,
                "max": p.max,
                "default": p.default,
                "step": p.step,
                "big_step": p.big_step,
                "fine_step": p.fine_step,
                "group": p.group,
                "advanced": p.advanced,
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
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
