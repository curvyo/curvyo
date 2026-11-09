//! The mask region of a masked trace across the wasm boundary (`specs/0094-masked-composite-trace`
//! slice 1): the region-only automatic values, for the trace worker.

use wasm_bindgen::prelude::*;

/// The automatic Threshold, background and analysis of the pixels the mask region of `settings`
/// keeps (`specs/0094` criterion 9), for the worker; the whole picture's without a region.
#[wasm_bindgen]
#[must_use]
pub fn trace_region_defaults(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
) -> TraceDefaultsJs {
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    let Some(region) = crate::trace_glue::trace_mask_region(settings) else {
        return TraceDefaultsJs {
            threshold: crate::wasm_trace::trace_auto_threshold(rgba, width, height),
            background: crate::wasm_trace::trace_suggest_background(rgba, width, height),
            analysis: crate::wasm_trace::trace_analyse(rgba, width, height),
        };
    };
    let read = curvyo_ui_core::trace_region_defaults(rgba, width, height, map, &region);
    TraceDefaultsJs {
        threshold: read.threshold,
        background: read
            .background
            .map(crate::trace_glue::hex)
            .unwrap_or_default(),
        analysis: read
            .analysis
            .map(|report| crate::trace_analysis_glue::analysis_json(&report))
            .unwrap_or_default(),
    }
}

/// What [`trace_region_defaults`] proposes: as `trace_auto_threshold`, `trace_suggest_background`
/// and `trace_analyse`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TraceDefaultsJs {
    /// The automatic Threshold.
    pub threshold: u8,
    /// `#RRGGBB`, or empty for a transparent background.
    #[wasm_bindgen(getter_with_clone)]
    pub background: String,
    /// The analysis JSON, or empty.
    #[wasm_bindgen(getter_with_clone)]
    pub analysis: String,
}
