//! The AI worker's runs and the pinned model list for the host (`specs/0135` criteria 47, 50, 63,
//! 64; ADR 0075 §3, ADR 0076 §1; `specs/0143`, ADR 0078 §2): a model loaded once per worker from
//! the bytes the host read from its cache, the run of one step on the input pixels or of one of
//! its tiles, the assembler's tiles, and the list the download and the About licence list read.

use std::cell::RefCell;
use std::collections::HashMap;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    AiError, Factor, MODELS, Model, ModelFile, ModelKind, TileOutput, assemble, denoise,
    denoise_tile, model, remove_background, upscale, upscale_tile, verify,
};
use serde::Serialize;

thread_local! {
    /// The models this worker has loaded, kept while it lives (ADR 0075 §3).
    static LOADED: RefCell<HashMap<ModelKind, Model>> = RefCell::new(HashMap::new());
    /// The input picture of the tiled Run this worker takes part in (ADR 0078 §3).
    static INPUT: RefCell<Option<WorkingImage>> = const { RefCell::new(None) };
    /// The tiles the assembling worker has received for the Run (ADR 0078 §3).
    static TILES: RefCell<Vec<TileOutput>> = const { RefCell::new(Vec::new()) };
}

/// The model kind of the host's words.
#[must_use]
pub fn kind_of(key: &str) -> Option<ModelKind> {
    MODELS.iter().find(|m| m.kind.key() == key).map(|m| m.kind)
}

/// One pinned model as the host reads it.
#[derive(Debug, Serialize)]
struct ModelEntry {
    kind: &'static str,
    name: &'static str,
    version: &'static str,
    bytes: u64,
    sha256: String,
    url: String,
    licence_url: String,
    licence: &'static str,
    copyright: &'static str,
    upstream: &'static str,
    training_data: &'static str,
    placeholder: bool,
}

/// The pinned models as JSON (ADR 0076 §1: the frontend reads the list through the wasm and
/// holds no second copy).
#[must_use]
pub fn models_json() -> String {
    let list: Vec<ModelEntry> = MODELS
        .iter()
        .map(|m| ModelEntry {
            kind: m.kind.key(),
            name: m.name,
            version: m.version,
            bytes: m.bytes,
            sha256: m.sha256_hex(),
            url: m.url(),
            licence_url: m.licence_url(),
            licence: m.licence,
            copyright: m.copyright,
            upstream: m.upstream,
            training_data: m.training_data,
            placeholder: m.placeholder,
        })
        .collect();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_owned())
}

/// Whether `bytes` are the pinned model of `key` (criterion 64: checked before the file is
/// committed to the cache).
#[must_use]
pub fn verify_model(key: &str, bytes: &[u8]) -> bool {
    kind_of(key).is_some_and(|kind| verify(model(kind), bytes).is_ok())
}

/// Loads the model of `key` from `bytes` into this worker once: empty when loaded, else
/// `wrong_model` or the reason.
#[must_use]
pub fn load_model(key: &str, bytes: &[u8]) -> String {
    let Some(kind) = kind_of(key) else {
        return "unknown model".to_owned();
    };
    load_pinned(model(kind), bytes)
}

/// [`load_model`] against `file` (a test pins its own net).
#[must_use]
pub fn load_pinned(file: &ModelFile, bytes: &[u8]) -> String {
    if LOADED.with(|m| m.borrow().contains_key(&file.kind)) {
        return String::new();
    }
    match Model::load(file, bytes) {
        Ok(model) => {
            LOADED.with(|m| m.borrow_mut().insert(file.kind, model));
            String::new()
        }
        Err(AiError::WrongModel) => "wrong_model".to_owned(),
        Err(error) => error.to_string(),
    }
}

/// Whether this worker has the model of `key` loaded.
#[must_use]
pub fn model_loaded(key: &str) -> bool {
    kind_of(key).is_some_and(|kind| LOADED.with(|m| m.borrow().contains_key(&kind)))
}

/// The output of one run: its pixels (one byte each for the mask, else RGBA) and size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    /// The pixels.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Whether the pixels are grey.
    pub grey: bool,
}

/// Runs the loaded model of `key` on `rgba` (`width` x `height`), Upscale by `factor`;
/// `progress(done, total)` hears each tile and returns whether to go on.
///
/// # Errors
/// The reason in words: not loaded, cancelled, the working limit, a model that does not run.
pub fn run_model(
    key: &str,
    rgba: Vec<u8>,
    (width, height): (u32, u32),
    factor: u8,
    progress: &mut dyn FnMut(u32, u32) -> bool,
) -> Result<RunOutput, String> {
    let kind = kind_of(key).ok_or_else(|| "unknown model".to_owned())?;
    let image = working(rgba, (width, height));
    LOADED.with(|loaded| {
        let loaded = loaded.borrow();
        let model = loaded
            .get(&kind)
            .ok_or_else(|| "the model is not loaded".to_owned())?;
        let out = match kind {
            ModelKind::Background => {
                remove_background(model, &image, progress).map(|mask| RunOutput {
                    pixels: mask.grey,
                    width: mask.width,
                    height: mask.height,
                    grey: true,
                })
            }
            ModelKind::Upscale => {
                upscale(model, &image, factor_of(factor), progress).map(rgba_output)
            }
            ModelKind::Denoise => denoise(model, &image, progress).map(rgba_output),
        };
        out.map_err(|e| e.to_string())
    })
}

fn rgba_output(image: WorkingImage) -> RunOutput {
    RunOutput {
        pixels: image.pixels,
        width: image.width,
        height: image.height,
        grey: false,
    }
}

fn working(rgba: Vec<u8>, (width, height): (u32, u32)) -> WorkingImage {
    WorkingImage {
        width,
        height,
        pixels: rgba,
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

fn factor_of(factor: u8) -> Factor {
    if factor == 4 {
        Factor::Four
    } else {
        Factor::Two
    }
}

/// Keeps `rgba` (`width` x `height`) as the input of this worker's tiles and forgets the tiles of
/// an earlier Run (ADR 0078 §3: the picture is posted once per Run to each worker).
pub fn set_tile_input(rgba: Vec<u8>, size: (u32, u32)) {
    INPUT.with(|i| *i.borrow_mut() = Some(working(rgba, size)));
    TILES.with(|t| t.borrow_mut().clear());
}

/// Runs tile `index` of the loaded model of `key` (Upscale or AI denoise) on the input, Upscale
/// by `factor`.
///
/// # Errors
/// The reason in words: not loaded, no input, a model that is not tiled or does not run.
pub fn run_tile(key: &str, factor: u8, index: u32) -> Result<TileOutput, String> {
    let kind = kind_of(key).ok_or_else(|| "unknown model".to_owned())?;
    INPUT.with(|input| {
        let input = input.borrow();
        let image = input
            .as_ref()
            .ok_or_else(|| "no input picture".to_owned())?;
        LOADED.with(|loaded| {
            let loaded = loaded.borrow();
            let model = loaded
                .get(&kind)
                .ok_or_else(|| "the model is not loaded".to_owned())?;
            match kind {
                ModelKind::Upscale => upscale_tile(model, image, factor_of(factor), index),
                ModelKind::Denoise => denoise_tile(model, image, index),
                ModelKind::Background => {
                    Err(AiError::Invalid("Remove background is not tiled".into()))
                }
            }
            .map_err(|e| e.to_string())
        })
    })
}

/// Keeps a tile for [`assemble_tiles`] on the assembling worker.
pub fn add_tile(tile: TileOutput) {
    TILES.with(|t| t.borrow_mut().push(tile));
}

/// The RGBA pixels of the picture of `width` x `height` from the tiles kept by [`add_tile`]; the
/// tiles and the input are forgotten.
///
/// # Errors
/// The reason in words for a missing, doubled or wrongly sized tile.
pub fn assemble_tiles(width: u32, height: u32) -> Result<Vec<u8>, String> {
    let tiles = TILES.with(|t| std::mem::take(&mut *t.borrow_mut()));
    INPUT.with(|i| *i.borrow_mut() = None);
    assemble(width, height, tiles)
        .map(|image| image.pixels)
        .map_err(|e| e.to_string())
}
