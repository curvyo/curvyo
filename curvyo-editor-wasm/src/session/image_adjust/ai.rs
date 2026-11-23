//! The AI steps of the selected picture in the session (`specs/0135` criteria 47, 48, 63 to 66;
//! ADR 0076 §7, ADR 0077): the input a Run hands the worker, the result it stores in one step
//! "Run ...", Remove stored result, the stored results the job and the looks read, and the model
//! states the host reports.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{AdjustmentsWrite, AssetHash, StepResult};
use curvyo_image_core::{GreyPlane, WorkingImage, encode_grey_png, encode_png};
use curvyo_imageai_core::{ModelKind, model};
use curvyo_laser_core::adjust::{AdjustRegisters, ManualStep, PyramidCache, StepKind};
use curvyo_ui_core::{remove_result, result_refusal, run_label, store_result, upscale_line};

/// The model kind an AI step runs, `None` for another step.
#[must_use]
pub(in crate::session) const fn model_kind(kind: StepKind) -> Option<ModelKind> {
    match kind {
        StepKind::RemoveBackground => Some(ModelKind::Background),
        StepKind::Upscale => Some(ModelKind::Upscale),
        StepKind::AiDenoise => Some(ModelKind::Denoise),
        _ => None,
    }
}

/// What the host reports about one model on this device (ADR 0076 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    /// Neither a complete nor a partial file: the next Run asks first.
    Absent,
    /// The consent line is shown, nothing downloaded yet.
    Asking,
    /// Downloading: the bytes so far.
    Downloading(u64),
    /// A partial file is kept: its bytes.
    Paused(u64),
    /// The host could not be reached.
    Unreachable,
    /// The host answered 404 or 410.
    Withdrawn,
    /// The browser storage is too small.
    NoStorage,
    /// The downloaded file did not match its pin and was removed.
    Checksum,
    /// The file is on the device.
    Ready,
}

impl ModelState {
    /// The state of the host's words (`absent`, `asking`, `downloading`, `paused`,
    /// `unreachable`, `withdrawn`, `storage`, `checksum`, `ready`) with `bytes` so far.
    #[must_use]
    pub fn of(state: &str, bytes: u64) -> Option<Self> {
        Some(match state {
            "absent" => Self::Absent,
            "asking" => Self::Asking,
            "downloading" => Self::Downloading(bytes),
            "paused" => Self::Paused(bytes),
            "unreachable" => Self::Unreachable,
            "withdrawn" => Self::Withdrawn,
            "storage" => Self::NoStorage,
            "checksum" => Self::Checksum,
            "ready" => Self::Ready,
            _ => return None,
        })
    }
}

/// A Run in progress: the row and its model's progress (`ux-notes.md` §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct Running {
    pub row: usize,
    pub done: u32,
    pub total: u32,
    /// The Run has a pool of one worker (`specs/0143` criterion 10).
    pub one_core: bool,
}

/// The AI steps' session state, never saved.
#[derive(Debug, Default)]
pub(in crate::session) struct AiState {
    /// The model states the host reported, by kind.
    pub models: HashMap<ModelKind, ModelState>,
    /// The Run in progress.
    pub running: Option<Running>,
    /// The refusal of the last Run, by row (a limit, the working limit).
    pub refusal: Option<(usize, String)>,
    /// The pixels of the last Run input, until the host takes them.
    pub pixels: Vec<u8>,
}

/// The input of a Run: the picture the model reads and its hash (ADR 0077 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRunInput {
    /// The model kind (`background`, `upscale`, `denoise`).
    pub kind: &'static str,
    /// Upscale's factor; 1 for the others.
    pub factor: u8,
    /// The picture after the steps before it, opaque RGBA.
    pub image: WorkingImage,
    /// The input hash, 64 hex.
    pub hash: String,
}

impl crate::session::Session {
    /// Holds the stored results `registers` read in `cache`, decoded from the document's derived
    /// assets once; whether every one is held (a missing or damaged one does not burn, ADR 0077
    /// §4).
    pub(in crate::session) fn hold_stored_results(
        &self,
        cache: &PyramidCache,
        registers: &AdjustRegisters,
    ) -> bool {
        registers.stored_results().iter().all(|result| {
            let hash = result.asset_hash();
            if cache.holds_result(&hash) {
                return true;
            }
            let decoded = self
                .document
                .asset(&hash)
                .filter(|bytes| AssetHash::of(bytes) == hash)
                .and_then(|bytes| curvyo_image_core::decode_working(&bytes).ok());
            decoded.is_some_and(|pixels| {
                cache.hold_result(&hash, Arc::new(pixels));
                true
            })
        })
    }

    /// The stack row `row` of the selected picture as a step, with its registers.
    fn ai_row(&self, row: usize) -> Option<(AdjustRegisters, ManualStep)> {
        let target = self.adjust_target()?;
        let shown = self.document.shown_adjustments(target).ok()?;
        let registers = AdjustRegisters::read(&shown.adjustments);
        let step = registers.steps.get(row)?.step?;
        model_kind(step.kind())?;
        Some((registers, step))
    }

    /// The input of a Run of AI row `row` of the selected picture (criterion 47): the picture
    /// after the geometry steps before it, at full resolution, and its hash with the pinned
    /// model. Upscale over the working limit is refused before anything runs.
    ///
    /// # Errors
    /// The refusal in words: not an AI row, the picture cannot be read, or the Upscale limit.
    pub fn ai_run_input(&mut self, row: usize) -> Result<AiRunInput, String> {
        self.image_adjust.ai.refusal = None;
        let (registers, step) = self
            .ai_row(row)
            .ok_or_else(|| "This step runs no model.".to_owned())?;
        let target = self.adjust_target().ok_or_else(String::new)?;
        let leaf = self.picture_leaf(target).ok_or_else(String::new)?;
        let pixels = self
            .job_pixels(&leaf.asset)
            .ok_or_else(|| "The picture cannot be read.".to_owned())?;
        let cache = self.adjust_pictures();
        self.hold_stored_results(&cache, &registers);
        let kind = model_kind(step.kind()).ok_or_else(String::new)?;
        let file = model(kind);
        let input = registers
            .ai_input(row, &pixels, &cache.results(), (file.name, file.version))
            .ok_or_else(String::new)?;
        let factor = match step {
            ManualStep::Upscale { factor, .. } => factor,
            _ => 1,
        };
        if factor > 1
            && let Err(refusal) = upscale_line(input.image.width, input.image.height, factor)
        {
            self.image_adjust.ai.refusal = Some((row, refusal.clone()));
            return Err(refusal);
        }
        self.image_adjust.ai.running = Some(Running {
            row,
            done: 0,
            total: 1,
            one_core: false,
        });
        Ok(AiRunInput {
            kind: kind.key(),
            factor,
            image: input.image,
            hash: AssetHash::from_bytes(&input.hash).as_str().to_owned(),
        })
    }

    /// Keeps the pixels of a Run input until the host takes them.
    pub fn stash_ai_pixels(&mut self, pixels: Vec<u8>) {
        self.image_adjust.ai.pixels = pixels;
    }

    /// The pixels of the last Run input, once.
    pub fn take_ai_pixels(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.image_adjust.ai.pixels)
    }

    /// The progress of the Run of row `row` (`done` of `total` tiles); shown, not written.
    pub fn set_ai_progress(&mut self, row: usize, done: u32, total: u32) {
        let one_core = self.ai_one_core(row);
        self.image_adjust.ai.running = Some(Running {
            row,
            done,
            total,
            one_core,
        });
    }

    fn ai_one_core(&self, row: usize) -> bool {
        self.image_adjust
            .ai
            .running
            .is_some_and(|r| r.row == row && r.one_core)
    }

    /// The Run of row `row` has a pool of one worker: its row says "Running on one core"
    /// (`specs/0143` criterion 10) until the Run ends.
    pub fn set_ai_one_core(&mut self, row: usize) {
        if let Some(r) = self
            .image_adjust
            .ai
            .running
            .as_mut()
            .filter(|r| r.row == row)
        {
            r.one_core = true;
        }
    }

    /// The Run ended without a result (Cancel, an error): the row shows its state again.
    pub fn end_ai_run(&mut self) {
        self.image_adjust.ai.running = None;
    }

    /// Stores the model output of the Run of row `row` (criterion 48): the PNG (grey for a mask)
    /// as a derived asset and the row's result record with the pinned model and the input hash
    /// `hash` (64 hex), in one step "Run Remove background", "Run Upscale" or "Run AI denoise".
    /// `grey` pixels are one byte each, else RGBA.
    ///
    /// # Errors
    /// The refusal of a limit (64 MB per result, 512 MB per document) or of pixels that do not
    /// match the size; nothing is written.
    pub fn store_ai_result(
        &mut self,
        row: usize,
        hash: &str,
        (width, height): (u32, u32),
        pixels: Vec<u8>,
        grey: bool,
    ) -> Result<bool, String> {
        self.image_adjust.ai.running = None;
        let (_, step) = self.ai_row(row).ok_or_else(String::new)?;
        let kind = model_kind(step.kind()).ok_or_else(String::new)?;
        let input_hash = AssetHash::parse(hash).ok_or_else(String::new)?;
        let png = if grey {
            encode_grey_png(&GreyPlane {
                width,
                height,
                grey: pixels,
            })
        } else {
            encode_png(&WorkingImage {
                width,
                height,
                pixels,
                original_size: (width, height),
                truncated: false,
                animated: false,
            })
        }
        .map_err(|e| e.to_string())?;
        let asset = self.document.insert_derived_asset(&png).map_err(|error| {
            let text = result_refusal(error, png.len() as u64);
            self.image_adjust.ai.refusal = Some((row, text.clone()));
            text
        })?;
        let file = model(kind);
        let result = StepResult {
            asset,
            model: file.name.to_owned(),
            model_version: file.version.to_owned(),
            input_hash,
            width_px: width,
            height_px: height,
        };
        let target = self.adjust_target().ok_or_else(String::new)?;
        let entries = self.shown_steps().ok_or_else(String::new)?;
        let steps = store_result(&entries, row, result).ok_or_else(String::new)?;
        let label = run_label(step.kind()).ok_or_else(String::new)?;
        self.document
            .run_image_ai_step(
                target,
                &AdjustmentsWrite {
                    steps: Some(steps),
                    ..AdjustmentsWrite::default()
                },
                label,
            )
            .map_err(|e| e.to_string())
    }

    /// Remove stored result of row `row` (criterion 66, P21): one step "Edit adjustments"; the
    /// picture does not burn until the next Run.
    pub fn remove_ai_result(&mut self, row: usize) -> bool {
        self.shown_steps()
            .and_then(|entries| remove_result(&entries, row))
            .is_some_and(|steps| {
                self.write_adjustments(&AdjustmentsWrite {
                    steps: Some(steps),
                    ..AdjustmentsWrite::default()
                })
            })
    }

    /// The host's report of model `kind` (`background`, `upscale`, `denoise`): its state words
    /// and the bytes so far; whether it was taken.
    pub fn set_ai_model_state(&mut self, kind: &str, state: &str, bytes: u64) -> bool {
        let kind = match kind {
            "background" => ModelKind::Background,
            "upscale" => ModelKind::Upscale,
            "denoise" => ModelKind::Denoise,
            _ => return false,
        };
        ModelState::of(state, bytes).is_some_and(|state| {
            self.image_adjust.ai.models.insert(kind, state);
            true
        })
    }

    /// The reported state of model `kind`, [`ModelState::Absent`] until reported.
    pub(in crate::session) fn ai_model_state(&self, kind: ModelKind) -> ModelState {
        self.image_adjust
            .ai
            .models
            .get(&kind)
            .copied()
            .unwrap_or(ModelState::Absent)
    }
}
