//! The mask of the Trace tool across the wasm boundary (`specs/0114-image-trace-mask`, ADR 0061
//! §9): the region tools, Subtract, the lasso's right click and closing, the bar's settings as tool
//! state, a stored part's settings, and what the Mask group shows.

use wasm_bindgen::prelude::*;

use crate::TraceRegionTool;
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Presses a region tool: `"rect"` for Rectangle, `"lasso"` for Lasso; anything else releases
    /// it. A lasso in progress is closed first.
    pub fn set_trace_region_tool(&mut self, tool: &str) {
        self.session.set_trace_region_tool(match tool {
            "rect" => TraceRegionTool::Rect,
            "lasso" => TraceRegionTool::Lasso,
            _ => TraceRegionTool::None,
        });
    }

    /// Presses or releases Subtract.
    pub fn set_trace_subtract(&mut self, subtract: bool) {
        self.session.set_trace_subtract(subtract);
    }

    /// The bar's settings of the active part as JSON (ADR 0061 §5's keys), tool state: what the
    /// first gesture on an untouched image and Add part store. No step.
    pub fn set_trace_tool_settings(&mut self, json: &str) {
        self.session.set_trace_tool_settings(json);
    }

    /// The stored settings of part `index` as JSON, every key valid; empty for the implicit part
    /// of an untouched image (the bar's settings are its own).
    #[must_use]
    pub fn trace_part_settings(&self, index: usize) -> String {
        self.session
            .trace_part_settings(index)
            .map(|settings| serde_json::Value::Object(settings).to_string())
            .unwrap_or_default()
    }

    /// The stored settings of part `index` of the selected picture `id` (several images selected:
    /// each traced with its own mask, Frage 11 A); empty when it stores none.
    #[must_use]
    pub fn trace_image_part_settings(&self, id: &str, index: usize) -> String {
        let Some(image) = crate::wasm_trace::id_of(id) else {
            return String::new();
        };
        self.session
            .for_trace_image(image, |session| session.trace_part_settings(index))
            .map(|settings| serde_json::Value::Object(settings).to_string())
            .unwrap_or_default()
    }

    /// The end of a settings edit of part `index`: one step "Change trace settings" for a stored
    /// part, tool state for the implicit one. Whether a step was written.
    pub fn set_trace_part_settings(&mut self, index: usize, json: &str) -> bool {
        self.session.set_trace_part_settings(index, json)
    }

    /// A right press on the canvas: finishes a lasso in progress (closed); `false` without one,
    /// so the host passes the press on.
    pub fn trace_mask_secondary_press(&mut self) -> bool {
        self.session.trace_mask_secondary_press()
    }

    /// Closes a lasso in progress as finished (another control of the bar took over).
    pub fn close_trace_lasso(&mut self) {
        self.session.close_trace_lasso();
    }

    /// Whether a lasso is being drawn.
    #[must_use]
    pub fn trace_lasso_open(&self) -> bool {
        self.session.trace_lasso_open()
    }

    /// Whether the mask takes the first Escape and Enter: a region drag or a lasso in progress.
    #[must_use]
    pub fn trace_mask_holds_keys(&self) -> bool {
        self.session.trace_mask_holds_escape()
    }

    /// What the Mask group shows, as JSON: `{tool: "none" | "rect" | "lasso", subtract, selected,
    /// lasso, locked}`; `selected` whether a region is selected, `lasso` whether one is being
    /// drawn, `locked` whether the image is locked (the group's controls are unavailable).
    #[must_use]
    pub fn trace_mask_view(&self) -> String {
        serde_json::json!({
            "tool": match self.session.trace_region_tool() {
                TraceRegionTool::None => "none",
                TraceRegionTool::Rect => "rect",
                TraceRegionTool::Lasso => "lasso",
            },
            "subtract": self.session.trace_subtract(),
            "selected": self.session.trace_selected_region().is_some(),
            "lasso": self.session.trace_lasso_open(),
            "locked": self.session.trace_mask_locked(),
        })
        .to_string()
    }
}
