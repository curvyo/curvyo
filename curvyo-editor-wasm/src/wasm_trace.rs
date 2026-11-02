//! The Trace tool's `wasm-bindgen` surface (`specs/0025-raster-trace`, ADR 0023 §5): the free
//! functions the trace worker calls (`trace_job`, `trace_auto_threshold`, the settings schema)
//! and the session's view, pixels, preview, results and Create paths. Images cross the boundary
//! as the JSON of their node id; the sentences are the host's (ADR 0001 §5).

use curvyo_document_core::NodeId;
use wasm_bindgen::prelude::*;

use crate::session::{TraceCreateOutcome, TracePickMode, TraceSelection, TraceViewMode};
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
    pub(crate) fn report(this: &TraceProgressSink, percent: u8);
}

/// The bar notice when the text reader could not be used (`specs/0091` UX notes §3).
#[wasm_bindgen]
#[must_use]
pub fn trace_text_failed_notice() -> String {
    curvyo_ui_core::TEXT_FAILED_NOTICE.to_owned()
}

/// One trace for the worker (criterion 17): `rgba` (`width` x `height`, straight alpha) mapped
/// by `map` (`[a, b, c, d, e, f]`) with `settings` (JSON: `threshold`, `smooth`, `tolerance`,
/// `specks` in mm, `corners`, `upscale` and `denoise` as choice indices, `invert`,
/// `ignore_background`, `method`, `background`, `swatches`, `colour_tolerance`); the
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
    let mut report = |percent| progress.report(percent);
    // Multicolour (`specs/0089` slice M): the outlines layer by layer, the palette after them.
    match crate::trace_glue::trace_colour_settings(settings) {
        Some(colour) => {
            curvyo_ui_core::trace_colour_job_with(rgba, width, height, map, &colour, &mut report)
        }
        None => curvyo_ui_core::trace_job_with(
            rgba,
            width,
            height,
            map,
            &crate::trace_glue::trace_settings(settings),
            &mut report,
        ),
    }
}

thread_local! {
    /// The text reader of this worker, loaded once (`specs/0091`, ADR 0028 §5).
    pub(crate) static OCR_MODEL: std::cell::RefCell<Option<curvyo_ui_core::OcrModel>> =
        const { std::cell::RefCell::new(None) };
}

/// Loads the text reader's weights into this worker once (ADR 0028 §5): empty when it is
/// loaded, else why it is not (`"wrong_model"` or the reason).
#[wasm_bindgen]
#[must_use]
pub fn ocr_load(detector: &[u8], recogniser: &[u8], alphabet: &str) -> String {
    if OCR_MODEL.with(|m| m.borrow().is_some()) {
        return String::new();
    }
    match curvyo_ui_core::OcrModel::load(detector, recogniser, alphabet) {
        Ok(model) => {
            OCR_MODEL.with(|m| *m.borrow_mut() = Some(model));
            String::new()
        }
        Err(curvyo_ui_core::OcrError::WrongModel) => "wrong_model".to_string(),
        Err(error) => error.to_string(),
    }
}

/// Whether this worker has the text reader loaded.
#[wasm_bindgen]
#[must_use]
pub fn ocr_loaded() -> bool {
    OCR_MODEL.with(|m| m.borrow().is_some())
}

/// Reads the text lines of `rgba` (`width` x `height`) once (`specs/0091` decision 1): the JSON
/// of `trace_glue::ocr_lines_json`, or empty when the reader is not loaded or failed. `progress`
/// hears each higher whole percent.
#[wasm_bindgen]
#[must_use]
pub fn ocr_job(rgba: &[u8], width: u32, height: u32, progress: &TraceProgressSink) -> String {
    OCR_MODEL.with(|m| {
        let model = m.borrow();
        let Some(model) = model.as_ref() else {
            return String::new();
        };
        let mut report = |percent| {
            progress.report(percent);
            std::ops::ControlFlow::Continue(())
        };
        curvyo_ui_core::trace_read_text(model, rgba, width, height, &mut report)
            .map(|lines| crate::trace_glue::ocr_lines_json(width, height, &lines))
            .unwrap_or_default()
    })
}

thread_local! {
    /// This worker's faces and font fit (`specs/0091` decision 4), kept for the worker's life.
    static FIT_WORKER: std::cell::RefCell<crate::trace_fit_glue::FitWorker> =
        std::cell::RefCell::new(crate::trace_fit_glue::FitWorker::default());
}

/// Starts the font fit `request` (the session's `trace_fit_request`) in this worker and runs it
/// until it needs faces (`{kind: "need", faces}`) or is finished (`{kind: "finished", lines}`).
#[wasm_bindgen]
#[must_use]
pub fn fit_start(request: &str, progress: &TraceProgressSink) -> String {
    let mut report = |percent| progress.report(percent);
    FIT_WORKER.with(|w| w.borrow_mut().start(request, &mut report))
}

/// Hands face `index` of the host's font file `file` to this worker; `false` when unreadable.
#[wasm_bindgen]
#[must_use]
pub fn fit_add_face(file: &str, index: u32, bytes: &[u8]) -> bool {
    FIT_WORKER.with(|w| w.borrow_mut().add_face(file, index, bytes))
}

/// Goes on with the fit after the faces it asked for were handed in.
#[wasm_bindgen]
#[must_use]
pub fn fit_step(progress: &TraceProgressSink) -> String {
    let mut report = |percent| progress.report(percent);
    FIT_WORKER.with(|w| w.borrow_mut().step(&mut report))
}

/// As [`trace_job`] with the OCR `lines` (the JSON of [`ocr_job`], scaled to this picture):
/// the text pass first, the main trace over the filled picture, the text section after
/// (`specs/0091` decisions 2, 3 and 6). Black and white only; Multicolour traces as
/// [`trace_job`].
#[wasm_bindgen]
#[must_use]
pub fn trace_text_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
    lines: &str,
    progress: &TraceProgressSink,
) -> Vec<f64> {
    if crate::trace_glue::trace_colour_settings(settings).is_some() {
        return trace_job(rgba, width, height, map, settings, progress);
    }
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    let mut report = |percent| progress.report(percent);
    curvyo_ui_core::trace_text_job_with(
        rgba,
        width,
        height,
        map,
        &crate::trace_glue::trace_settings(settings),
        &crate::trace_glue::read_ocr_lines(lines, width),
        &mut report,
    )
}

/// The text pass alone with the OCR `lines` (`specs/0091` criterion 25): a result without
/// outlines and the text section, for the fit that decides the lines before the main trace.
#[wasm_bindgen]
#[must_use]
pub fn trace_text_regions_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
    lines: &str,
) -> Vec<f64> {
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    curvyo_ui_core::trace_text_regions_with(
        rgba,
        width,
        height,
        map,
        &crate::trace_glue::trace_settings(settings),
        &crate::trace_glue::read_ocr_lines(lines, width),
    )
}

/// As [`trace_mask_job`] with the OCR `lines`: the pixels left out for text at 128
/// (`specs/0091` UX notes §2).
#[wasm_bindgen]
#[must_use]
pub fn trace_text_mask_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
    lines: &str,
) -> Option<TraceMaskJs> {
    if crate::trace_glue::trace_colour_settings(settings).is_some() {
        return trace_mask_job(rgba, width, height, map, settings);
    }
    let map6: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    curvyo_ui_core::trace_text_mask_job(
        rgba,
        width,
        height,
        map6,
        &crate::trace_glue::trace_settings(settings),
        &crate::trace_glue::read_ocr_lines(lines, width),
    )
    .map(|mask| TraceMaskJs {
        width: mask.width,
        height: mask.height,
        bits: mask.bits,
    })
}

/// The mask the trace of the same arguments walks (`specs/0089` criterion 21), for the worker's
/// Mask view: `width` x `height` mask pixels (twice the picture's after Upscale), `bits` row by
/// row, 255 for content and 0 for the rest; with Multicolour four bytes per pixel, the segmented
/// palette in straight RGBA (the background transparent); `undefined` when the picture cannot
/// be traced.
#[wasm_bindgen]
#[must_use]
pub fn trace_mask_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    map: &[f64],
    settings: &str,
) -> Option<TraceMaskJs> {
    let map: [f64; 6] = map.try_into().unwrap_or([f64::NAN; 6]);
    if let Some(colour) = crate::trace_glue::trace_colour_settings(settings) {
        return curvyo_ui_core::trace_colour_mask_job(rgba, width, height, map, &colour).map(
            |mask| TraceMaskJs {
                width: mask.width,
                height: mask.height,
                bits: mask.rgba,
            },
        );
    }
    curvyo_ui_core::trace_mask_job(
        rgba,
        width,
        height,
        map,
        &crate::trace_glue::trace_settings(settings),
    )
    .map(|mask| TraceMaskJs {
        width: mask.width,
        height: mask.height,
        bits: mask.bits,
    })
}

/// A mask for the Mask view.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TraceMaskJs {
    /// Width in mask pixels.
    pub width: u32,
    /// Height in mask pixels.
    pub height: u32,
    /// Row by row, 255 for content, 0 for the rest.
    #[wasm_bindgen(getter_with_clone)]
    pub bits: Vec<u8>,
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

/// The automatic analysis of a picture (`specs/0089` slice F), for the worker: the JSON of
/// `analysis_json`, or empty when the picture cannot be read.
#[wasm_bindgen]
#[must_use]
pub fn trace_analyse(rgba: &[u8], width: u32, height: u32) -> String {
    curvyo_ui_core::trace_analyse(rgba, width, height)
        .map(|report| crate::trace_analysis_glue::analysis_json(&report))
        .unwrap_or_default()
}

/// The background colour Remove background's Auto suggests for a picture (`specs/0089`
/// criterion 16), for the worker: `#RRGGBB`, or empty for a transparent background.
#[wasm_bindgen]
#[must_use]
pub fn trace_suggest_background(rgba: &[u8], width: u32, height: u32) -> String {
    curvyo_ui_core::trace_suggest_background(rgba, width, height)
        .map(crate::trace_glue::hex)
        .unwrap_or_default()
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
    /// Colour paths written by a multicolour trace (0 for black paths).
    pub colours: u32,
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

    /// Keeps the worker's mask of image `id` (`width` x `height`, `bits` non-zero for content)
    /// for the Mask view; `false` when the bits do not match the size.
    pub fn set_trace_mask(&mut self, id: &str, width: u32, height: u32, bits: &[u8]) -> bool {
        id_of(id).is_some_and(|id| self.session.set_trace_mask(id, width, height, bits))
    }

    /// Forgets the full-size results (a value changed).
    pub fn clear_trace_results(&mut self) {
        self.session.clear_trace_results();
    }

    /// The palette of the shown multicolour preview as JSON (`specs/0089` UX notes §7): `{colours:
    /// [{hex, fixed, pixels}], layers: [hex], joined}`; empty for one colour or no preview.
    #[must_use]
    pub fn trace_palette(&self) -> String {
        self.session.trace_palette_json()
    }

    /// The maker's palette view (`specs/0089` criterion 29): `recolours` a JSON list of `[from,
    /// to]` `#RRGGBB` pairs (fills only, no trace), `isolated` a `#RRGGBB` or empty; the preview,
    /// the Mask view and Create paths follow.
    pub fn set_trace_palette_view(&mut self, recolours: &str, isolated: &str) {
        let pairs: Vec<(curvyo_ui_core::TraceRgb, curvyo_ui_core::TraceRgb)> =
            serde_json::from_str::<Vec<(String, String)>>(recolours)
                .unwrap_or_default()
                .iter()
                .filter_map(|(from, to)| {
                    Some((
                        crate::trace_glue::parse_hex(from)?,
                        crate::trace_glue::parse_hex(to)?,
                    ))
                })
                .collect();
        self.session
            .set_trace_palette_view(pairs, crate::trace_glue::parse_hex(isolated));
    }

    /// The view on the canvas, as the View pill shows it (criterion 51): `"overlay"`, `"mask"`,
    /// `"trace"` or `"image"`; Image while `\` is held.
    #[must_use]
    pub fn trace_view_mode(&self) -> String {
        match self.session.shown_trace_view_mode() {
            TraceViewMode::Overlay => "overlay",
            TraceViewMode::Mask => "mask",
            TraceViewMode::Trace => "trace",
            TraceViewMode::Image => "image",
        }
        .to_string()
    }

    /// Chooses the view `mode` (`"overlay"`, `"mask"`, `"trace"` or `"image"`; anything else is
    /// ignored).
    pub fn set_trace_view_mode(&mut self, mode: &str) {
        let mode = match mode {
            "overlay" => TraceViewMode::Overlay,
            "mask" => TraceViewMode::Mask,
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

    /// Arms the Trace pipette (`specs/0089` criteria 16, 17): `"background"` (one pick) or
    /// `"colours"` (stays armed); anything else disarms it.
    pub fn set_trace_pick(&mut self, mode: &str) {
        self.session.set_trace_pick(match mode {
            "background" => Some(TracePickMode::Background),
            "colours" => Some(TracePickMode::Colours),
            _ => None,
        });
    }

    /// What the Trace pipette is armed for: `"background"`, `"colours"` or empty.
    #[must_use]
    pub fn trace_pick_mode(&self) -> String {
        match self.session.trace_pick_mode() {
            Some(TracePickMode::Background) => "background",
            Some(TracePickMode::Colours) => "colours",
            None => "",
        }
        .to_string()
    }

    /// The colour a click would take while the Trace pipette is armed, as `#RRGGBB`; empty where
    /// there is none; `undefined` while it is disarmed.
    #[must_use]
    pub fn trace_pick_hover(&self) -> Option<String> {
        self.session
            .trace_pick_hover()
            .map(|colour| colour.map(crate::trace_glue::hex).unwrap_or_default())
    }

    /// The pick the host has not taken yet as `[#RRGGBB, "add"]` (Shift) or `[#RRGGBB, ""]`;
    /// empty when there is none.
    pub fn take_trace_pick(&mut self) -> Vec<String> {
        self.session
            .take_trace_pick()
            .map(|(colour, add)| {
                vec![
                    crate::trace_glue::hex(colour),
                    if add { "add" } else { "" }.to_string(),
                ]
            })
            .unwrap_or_default()
    }

    /// Keeps the OCR lines `json` (of `ocr_job`) of image `id` for the session (`specs/0091`
    /// decision 1).
    pub fn set_trace_ocr_lines(&mut self, id: &str, json: &str) -> bool {
        let Some(id) = id_of(id) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
            return false;
        };
        let side = |name: &str| {
            value
                .get(name)
                .and_then(serde_json::Value::as_u64)
                .and_then(|w| u32::try_from(w).ok())
                .unwrap_or(0)
        };
        let (width, height) = (side("width"), side("height"));
        let lines = crate::trace_glue::read_ocr_lines(json, width);
        self.session.set_trace_ocr_lines(id, lines, (width, height))
    }

    /// The OCR lines a trace of image `id` takes as the JSON of `ocr_job` at the full working
    /// size (those a fit set no text for no longer accepted), or empty when its picture was not
    /// read in this session.
    #[must_use]
    pub fn trace_ocr_lines(&mut self, id: &str) -> String {
        let Some(id) = id_of(id) else {
            return String::new();
        };
        // The size kept with the lines: no copy of the picture on the main thread.
        self.session
            .trace_lines_to_trace(id)
            .map(|(lines, (width, height))| {
                crate::trace_glue::ocr_lines_json(width, height, &lines)
            })
            .unwrap_or_default()
    }

    /// How many lines the last fit set no text for; the host then traces again with
    /// `trace_ocr_lines` (`specs/0091` lead decision 1).
    #[must_use]
    pub fn trace_fit_dropped(&self) -> usize {
        self.session.trace_fit_dropped()
    }

    /// The request of the font fit the trace worker runs (`specs/0091` decision 4) with the
    /// Recognise text `tolerance` in mm and the `favourites` (a JSON list of family names); empty
    /// when no kept result has a text line.
    pub fn trace_fit_request(&mut self, tolerance: f64, favourites: &str) -> String {
        let favourites: Vec<String> = serde_json::from_str(favourites).unwrap_or_default();
        self.session
            .trace_fit_request(tolerance, &favourites)
            .unwrap_or_default()
    }

    /// The request of the font fit of `regions` (the worker's `trace_text_regions_job` of image
    /// `id`) before its main trace (`specs/0091` criterion 25); empty without a line.
    pub fn trace_fit_request_for(
        &mut self,
        id: &str,
        regions: &[f64],
        tolerance: f64,
        favourites: &str,
    ) -> String {
        let Some(id) = id_of(id) else {
            return String::new();
        };
        let favourites: Vec<String> = serde_json::from_str(favourites).unwrap_or_default();
        self.session
            .trace_fit_request_for(id, regions, tolerance, &favourites)
            .unwrap_or_default()
    }

    /// Takes the worker's finished fit `result` of `request`; the preview then shows the texts.
    pub fn set_trace_fit(&mut self, request: &str, result: &str) -> bool {
        self.session.set_trace_fit(request, result)
    }

    /// Whether the fit of the kept results is finished (the preview shows the texts; Create
    /// paths need not fit again).
    #[must_use]
    pub fn trace_text_fitted(&self) -> bool {
        self.session.trace_text_fitted()
    }

    /// The warning after the last Create paths when letters were guessed (`specs/0091`
    /// criterion 5, UX notes §5), or empty when every letter is sure (no success notice).
    #[must_use]
    pub fn trace_created_warning(&self) -> String {
        self.session.trace_created_warning().unwrap_or_default()
    }

    /// The text lines of the shown preview: `[set, found]` (`specs/0091` UX notes §4).
    #[must_use]
    pub fn trace_text_counts(&self) -> Vec<u32> {
        let (set, found) = self.session.trace_text_counts();
        vec![count(set), count(found)]
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
                colours,
            } => TraceCreateJs {
                kind: "created".to_string(),
                images: count(images),
                paths: count(paths),
                nodes: count(nodes),
                colours: count(colours),
            },
        }
    }
}
