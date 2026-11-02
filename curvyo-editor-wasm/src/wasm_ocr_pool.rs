//! The wasm entry points of the line-reading pool (`specs/0091-trace-text-recognition` round 2):
//! the trace worker detects and crops a picture's lines, pool workers read shares of the crops.

use wasm_bindgen::prelude::*;

use crate::wasm_trace::{OCR_MODEL, TraceProgressSink};

/// A picture's line crops for the pool: the JSON of their quads and widths, and their samples.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct OcrCropsJs {
    /// `{lines: [{quad, width}]}`, top to bottom.
    #[wasm_bindgen(getter_with_clone)]
    pub lines: String,
    /// Every line's samples, line after line.
    #[wasm_bindgen(getter_with_clone)]
    pub samples: Vec<f32>,
}

/// The lines of `rgba` (`width` x `height`) detected and cropped by this worker's reader, or
/// `undefined` when it is not loaded or failed.
#[wasm_bindgen]
#[must_use]
pub fn ocr_crops(rgba: &[u8], width: u32, height: u32) -> Option<OcrCropsJs> {
    OCR_MODEL.with(|m| {
        let model = m.borrow();
        let crops = curvyo_ui_core::trace_crop_text(model.as_ref()?, rgba, width, height).ok()?;
        let (lines, samples) = crate::ocr_pool_glue::crops_wire(&crops);
        Some(OcrCropsJs { lines, samples })
    })
}

/// Reads a share of the crops (`lines` and `samples` as [`ocr_crops`] gives them for that
/// share) with this worker's reader: the JSON array of their lines, `null` where nothing was
/// read; empty when the reader is not loaded or a crop cannot be read. `progress` hears the
/// crops read so far, in percent of the share.
#[wasm_bindgen]
#[must_use]
pub fn ocr_read_crops(lines: &str, samples: &[f32], progress: &TraceProgressSink) -> String {
    OCR_MODEL.with(|m| {
        let model = m.borrow();
        let Some(model) = model.as_ref() else {
            return String::new();
        };
        let Some(crops) = crate::ocr_pool_glue::read_crops_wire(lines, samples) else {
            return String::new();
        };
        let total = crops.len().max(1);
        let mut report = |done: usize| {
            progress.report(u8::try_from(100 * done / total).unwrap_or(100));
        };
        crate::ocr_pool_glue::read_crops(model, &crops, &mut report).unwrap_or_default()
    })
}
