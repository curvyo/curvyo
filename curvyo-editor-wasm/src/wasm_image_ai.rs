//! Thin host bridge of the AI steps (`specs/0135` slice 3): the worker's model load and run (whole
//! or tile by tile, `specs/0143`), the pinned model list, and the session's Run input, stored
//! result and model states.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::wasm_trace::TraceProgressSink;

/// The pinned models as JSON (kind, name, version, bytes, sha256, url, licence and attribution).
#[wasm_bindgen]
#[must_use]
pub fn ai_models_json() -> String {
    crate::ai_glue::models_json()
}

/// Whether `bytes` are the pinned model `kind` (size and SHA-256), before the cache commits them.
#[wasm_bindgen]
#[must_use]
pub fn ai_verify_model(kind: &str, bytes: &[u8]) -> bool {
    crate::ai_glue::verify_model(kind, bytes)
}

/// Loads model `kind` into this worker once: empty when loaded, else `wrong_model` or the reason.
#[wasm_bindgen]
#[must_use]
pub fn ai_load_model(kind: &str, bytes: &[u8]) -> String {
    crate::ai_glue::load_model(kind, bytes)
}

/// Whether this worker has model `kind` loaded.
#[wasm_bindgen]
#[must_use]
pub fn ai_model_loaded(kind: &str) -> bool {
    crate::ai_glue::model_loaded(kind)
}

/// Runs model `kind` on `rgba` (`width` x `height`), Upscale by `factor`: the output pixels
/// (`320 x 320` grey for the mask, else RGBA of `factor` times the size, 1 for AI denoise).
/// `progress` hears each higher whole percent; Cancel terminates the worker.
///
/// # Errors
/// The reason in words.
#[wasm_bindgen]
pub fn ai_run(
    kind: &str,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    factor: u8,
    progress: &TraceProgressSink,
) -> Result<Vec<u8>, JsError> {
    let mut last = 0_u8;
    crate::ai_glue::run_model(kind, rgba, (width, height), factor, &mut |done, total| {
        let percent = u8::try_from(u64::from(done) * 100 / u64::from(total.max(1))).unwrap_or(100);
        if percent > last {
            last = percent;
            progress.report(percent);
        }
        true
    })
    .map(|out| out.pixels)
    .map_err(|reason| JsError::new(&reason))
}

/// The number of tiles of a tiled Run on `width` x `height` (ADR 0078 §2).
#[wasm_bindgen]
#[must_use]
pub fn ai_tile_count(width: u32, height: u32) -> u32 {
    curvyo_imageai_core::tile_count(width, height)
}

/// One tile's result: its index, size in output pixels and RGBA pixels (ADR 0078 §2).
#[wasm_bindgen]
pub struct AiTile {
    tile: curvyo_imageai_core::TileOutput,
}

#[wasm_bindgen]
impl AiTile {
    /// The tile's index.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn index(&self) -> u32 {
        self.tile.index
    }

    /// The core's width in output pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.tile.width
    }

    /// The core's height in output pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.tile.height
    }

    /// The core's RGBA pixels, handed over once.
    pub fn take_pixels(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.tile.rgba)
    }
}

/// Keeps `rgba` (`width` x `height`) as this worker's input for the tiles of a Run.
#[wasm_bindgen]
pub fn ai_set_tile_input(rgba: Vec<u8>, width: u32, height: u32) {
    crate::ai_glue::set_tile_input(rgba, (width, height));
}

/// Runs tile `index` of model `kind` on the input, Upscale by `factor`.
///
/// # Errors
/// The reason in words.
#[wasm_bindgen]
pub fn ai_run_tile(kind: &str, factor: u8, index: u32) -> Result<AiTile, JsError> {
    crate::ai_glue::run_tile(kind, factor, index)
        .map(|tile| AiTile { tile })
        .map_err(|reason| JsError::new(&reason))
}

/// Keeps a tile of the Run on the assembling worker.
#[wasm_bindgen]
pub fn ai_add_tile(index: u32, width: u32, height: u32, rgba: Vec<u8>) {
    crate::ai_glue::add_tile(curvyo_imageai_core::TileOutput {
        index,
        width,
        height,
        rgba,
    });
}

/// The Run's RGBA output of `width` x `height` from the kept tiles.
///
/// # Errors
/// The reason in words for a tile set that does not make up the picture.
#[wasm_bindgen]
pub fn ai_assemble(width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    crate::ai_glue::assemble_tiles(width, height).map_err(|reason| JsError::new(&reason))
}

#[wasm_bindgen]
impl WasmSession {
    /// The input of a Run of AI row `row` as JSON `{kind, factor, width, height, hash}`, or
    /// `{refusal}`; its pixels are read once by [`Self::take_ai_run_pixels`].
    pub fn ai_run_input_json(&mut self, row: usize) -> String {
        match self.session.ai_run_input(row) {
            Ok(input) => {
                let json = serde_json::json!({
                    "kind": input.kind,
                    "factor": input.factor,
                    "width": input.image.width,
                    "height": input.image.height,
                    "hash": input.hash,
                });
                self.session.stash_ai_pixels(input.image.pixels);
                json.to_string()
            }
            Err(refusal) => serde_json::json!({ "refusal": refusal }).to_string(),
        }
    }

    /// The pixels of the last Run input (RGBA), once.
    pub fn take_ai_run_pixels(&mut self) -> Vec<u8> {
        self.session.take_ai_pixels()
    }

    /// Progress of the Run of row `row`: shown on the row, not written.
    pub fn set_ai_progress(&mut self, row: usize, done: u32, total: u32) {
        self.session.set_ai_progress(row, done, total);
    }

    /// The Run of row `row` has a pool of one worker ("Running on one core", `specs/0143`).
    pub fn set_ai_one_core(&mut self, row: usize) {
        self.session.set_ai_one_core(row);
    }

    /// The Run ended without a result (Cancel, an error).
    pub fn end_ai_run(&mut self) {
        self.session.end_ai_run();
    }

    /// Stores the output of the Run of row `row` made from input `hash`: one step "Run …". The
    /// refusal text (a limit), empty when stored.
    pub fn store_ai_result(
        &mut self,
        row: usize,
        hash: &str,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        grey: bool,
    ) -> String {
        self.session
            .store_ai_result(row, hash, (width, height), pixels, grey)
            .err()
            .unwrap_or_default()
    }

    /// Remove stored result of row `row`: one step "Edit adjustments". Whether written.
    pub fn remove_ai_result(&mut self, row: usize) -> bool {
        self.session.remove_ai_result(row)
    }

    /// The host's report of model `kind`: `absent`, `asking`, `downloading`, `paused`,
    /// `unreachable`, `withdrawn`, `storage`, `checksum` or `ready`, with the bytes so far.
    pub fn set_ai_model_state(&mut self, kind: &str, state: &str, bytes: f64) -> bool {
        // invariant: the host passes whole byte counts below 2^53.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bytes = bytes.max(0.0) as u64;
        self.session.set_ai_model_state(kind, state, bytes)
    }
}
