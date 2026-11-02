//! Reads text lines from an RGBA raster with model weights handed in as bytes (ADR 0028).
//!
//! The pipeline is ours (ADR 0028 §3): the `PaddleOCR` detector (DB) finds word boxes on a copy at
//! most [`DETECTION_SIDE`] px long, the boxes are grouped into lines with one quad each (rotated
//! text allowed), and the Latin recogniser reads each line along its quad with greedy CTC. Only
//! the inference runs in `rten`; it brings `rayon` (ADR 0028 §4: in wasm everything runs on the
//! calling thread, natively only the tests run it), this crate spawns nothing, keeps no global
//! state and reads no clock. Pixel space throughout: x right, y down, origin at the raster's top
//! left corner.

#![forbid(unsafe_code)]
// Pixel sizes and indices become `f32` coordinates; above 2^24 px a coordinate loses a fraction
// of a pixel, far below what the detector or the reader can see.
#![allow(clippy::cast_precision_loss)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod detect;
mod lines;
mod model;
mod read;
mod resize;

use std::ops::ControlFlow;

pub use detect::DETECTION_SIDE;
pub use model::{ALPHABET_SHA256, DETECTOR_SHA256, OcrModel, RECOGNISER_SHA256};
pub use read::{LINE_HEIGHT, LineCrop, read_crop};

/// One line of text the model read.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// The corners in source pixels: top left, top right, bottom right, bottom left along the
    /// line's direction.
    pub quad: [[f32; 2]; 4],
    /// The string, without leading or trailing spaces.
    pub text: String,
    /// The confidence of each character of `text`, 0 to 1.
    pub confidence: Vec<f32>,
}

/// Where a read is, for the caller's progress and cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrProgress {
    /// The detector is about to run.
    Detecting,
    /// `done` of `total` lines are read.
    Reading {
        /// Lines read.
        done: usize,
        /// Lines found.
        total: usize,
    },
}

/// Why a read or a load failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OcrError {
    /// A file is not the one this crate expects (its SHA-256 differs).
    #[error("the model files are not the expected ones")]
    WrongModel,
    /// A file has the expected hash but could not be parsed or run.
    #[error("the model could not be used: {0}")]
    BadModel(String),
    /// The caller's poll asked to stop.
    #[error("cancelled")]
    Cancelled,
    /// The raster is not a raster.
    #[error("invalid input: {0}")]
    Invalid(String),
}

/// Reads the text lines of `rgba` (`width` x `height`, straight alpha, composited on white)
/// with `model`, top to bottom: [`crop_lines`], then [`read_crop`] of each. `poll` is called
/// before the detector and after each line; a `Break` ends the read with
/// [`OcrError::Cancelled`]. Deterministic: the same input gives the same lines (ADR 0028 §6).
///
/// # Errors
/// [`OcrError::Invalid`] for a raster whose size does not match its bytes or that is empty,
/// [`OcrError::BadModel`] when inference fails, [`OcrError::Cancelled`] when `poll` stops it.
pub fn read_lines(
    model: &OcrModel,
    width: u32,
    height: u32,
    rgba: &[u8],
    poll: &mut dyn FnMut(OcrProgress) -> ControlFlow<()>,
) -> Result<Vec<TextLine>, OcrError> {
    let crops = crop_lines(model, width, height, rgba, poll)?;
    let total = crops.len();
    let mut out = Vec::with_capacity(total);
    for (done, line) in crops.iter().enumerate() {
        if poll(OcrProgress::Reading { done, total }).is_break() {
            return Err(OcrError::Cancelled);
        }
        out.extend(read_crop(model, line)?);
    }
    if poll(OcrProgress::Reading { done: total, total }).is_break() {
        return Err(OcrError::Cancelled);
    }
    Ok(out)
}

/// The lines `model`'s detector finds in `rgba`, top to bottom, each sampled for the recogniser
/// (the detection half of [`read_lines`]; the crops can be read apart, `specs/0091` round 2).
/// `poll` is called before the detector.
///
/// # Errors
/// As [`read_lines`].
pub fn crop_lines(
    model: &OcrModel,
    width: u32,
    height: u32,
    rgba: &[u8],
    poll: &mut dyn FnMut(OcrProgress) -> ControlFlow<()>,
) -> Result<Vec<LineCrop>, OcrError> {
    let image = resize::Rgb::from_rgba(width, height, rgba)?;
    if poll(OcrProgress::Detecting).is_break() {
        return Err(OcrError::Cancelled);
    }
    let boxes = detect::detect(model.detector(), &image)?;
    Ok(lines::group(&boxes)
        .into_iter()
        .filter_map(|quad| read::crop(&image, quad))
        .collect())
}
