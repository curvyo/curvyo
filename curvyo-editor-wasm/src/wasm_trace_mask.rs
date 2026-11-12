//! The mask of the Trace tool across the wasm boundary (`specs/0114-image-trace-mask` slice 1,
//! ADR 0061 §9): the region tool, Subtract, the bar's settings as tool state, and what the Mask
//! group shows.

use wasm_bindgen::prelude::*;

use crate::TraceRegionTool;
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Presses a region tool: `"rect"` for Rectangle; anything else releases it.
    pub fn set_trace_region_tool(&mut self, tool: &str) {
        self.session.set_trace_region_tool(match tool {
            "rect" => TraceRegionTool::Rect,
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

    /// What the Mask group shows, as JSON: `{tool: "none" | "rect", subtract, selected}`,
    /// `selected` whether a region is selected.
    #[must_use]
    pub fn trace_mask_view(&self) -> String {
        serde_json::json!({
            "tool": match self.session.trace_region_tool() {
                TraceRegionTool::None => "none",
                TraceRegionTool::Rect => "rect",
            },
            "subtract": self.session.trace_subtract(),
            "selected": self.session.trace_selected_region().is_some(),
        })
        .to_string()
    }
}
