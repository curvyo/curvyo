//! Reads a line's string and character confidences with the recogniser from its crop.
//!
//! The line is sampled along its quad at [`LINE_HEIGHT`] px, its width rounded up to a multiple
//! of [`WIDTH_BUCKET`], at least [`MIN_WIDTH`], and padded with the recogniser's zero (mid grey), then decoded greedily:
//! per time step the most likely class, repeats and blanks dropped; a character's confidence is
//! its highest probability over the steps it spans (ADR 0028 §3).

use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::detect::RotatedBox;
use crate::model::OcrModel;
use crate::resize::{Grid, Rgb, sample};
use crate::{OcrError, TextLine};

/// The recogniser's input height, px.
pub const LINE_HEIGHT: usize = 48;

/// Widths are rounded up to a multiple of this, so similar lines share a shape (Proposal).
const WIDTH_BUCKET: usize = 32;
/// The narrowest input: shorter lines are padded to it, which reads them markedly better
/// than a tight pad (spike and reference set, Proposal).
const MIN_WIDTH: usize = 1024;
/// The widest input; a longer line is squeezed into it (Proposal).
const MAX_WIDTH: usize = 3200;

/// One detected line sampled for the recogniser: its quad and its pixels at [`LINE_HEIGHT`] px,
/// before the padding. Lines are independent, so their crops can be read anywhere, in any order.
#[derive(Debug, Clone, PartialEq)]
pub struct LineCrop {
    /// The corners in source pixels: top left, top right, bottom right, bottom left.
    pub quad: [[f32; 2]; 4],
    /// The sampled width, px.
    pub width: usize,
    /// `width` x [`LINE_HEIGHT`] RGB samples, row by row, three values each.
    pub rgb: Vec<f32>,
}

/// `quad` of `image` sampled at the recogniser's height; `None` for a degenerate quad.
pub(crate) fn crop(image: &Rgb, quad: RotatedBox) -> Option<LineCrop> {
    let (len_u, len_v) = (2.0 * quad.half_u, 2.0 * quad.half_v);
    if len_u < 1.0 || len_v < 1.0 {
        return None;
    }
    // invariant: positive, clamped below MAX_WIDTH.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let text_width = ((len_u * LINE_HEIGHT as f32 / len_v).ceil() as usize).clamp(8, MAX_WIDTH);
    let (u, v) = (quad.u, quad.v());
    let corners = quad.quad();
    let (step_u, step_v) = (len_u / text_width as f32, len_v / LINE_HEIGHT as f32);
    let grid = Grid {
        origin: corners[0],
        step_x: [u[0] * step_u, u[1] * step_u],
        step_y: [v[0] * step_v, v[1] * step_v],
        width: text_width,
        height: LINE_HEIGHT,
    };
    Some(LineCrop {
        quad: corners,
        width: text_width,
        rgb: sample(image, &grid).into_iter().flatten().collect(),
    })
}

/// The string of `line`, or `None` when nothing was read.
///
/// # Errors
/// [`OcrError::Invalid`] for a crop whose samples do not match its width, [`OcrError::BadModel`]
/// when inference fails.
pub fn read_crop(model: &OcrModel, line: &LineCrop) -> Result<Option<TextLine>, OcrError> {
    let text_width = line.width;
    if text_width == 0 || text_width > MAX_WIDTH || line.rgb.len() != text_width * LINE_HEIGHT * 3 {
        return Err(OcrError::Invalid(format!(
            "a crop {text_width} px wide with {} samples",
            line.rgb.len()
        )));
    }
    let width = (text_width.div_ceil(WIDTH_BUCKET) * WIDTH_BUCKET).max(MIN_WIDTH);
    let input = NdTensor::from_fn([1, 3, LINE_HEIGHT, width], |[_, c, y, x]| {
        if x < text_width {
            // Paddle's order: the blue channel first; [-1, 1].
            (line.rgb[(y * text_width + x) * 3 + 2 - c] / 255.0 - 0.5) / 0.5
        } else {
            0.0
        }
    });
    let output: Tensor<f32> = model
        .recogniser()
        .run_one(input.view().into(), None)
        .map_err(|e| OcrError::BadModel(e.to_string()))?
        .try_into()
        .map_err(|e: rten::TryFromValueError| OcrError::BadModel(e.to_string()))?;
    let shape = output.shape().to_vec();
    if shape.len() != 3 {
        return Err(OcrError::BadModel(format!("recogniser output {shape:?}")));
    }
    let (steps, classes) = (shape[1], shape[2]);
    let (text, confidence) = decode(model, &output.to_vec(), steps, classes);
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(TextLine {
        quad: line.quad,
        text,
        confidence,
    }))
}

/// Greedy CTC over `probabilities` (`steps` x `classes`, class 0 the blank): the string without
/// leading and trailing spaces, and each character's confidence.
fn decode(
    model: &OcrModel,
    probabilities: &[f32],
    steps: usize,
    classes: usize,
) -> (String, Vec<f32>) {
    let mut chars: Vec<(char, f32)> = Vec::new();
    let mut previous = 0;
    for step in 0..steps {
        let row = &probabilities[step * classes..(step + 1) * classes];
        let (best, p) = row
            .iter()
            .copied()
            .enumerate()
            .fold(
                (0, f32::MIN),
                |(bi, bp), (i, p)| {
                    if p > bp { (i, p) } else { (bi, bp) }
                },
            );
        if best != 0 && best == previous {
            if let Some(last) = chars.last_mut() {
                last.1 = last.1.max(p);
            }
        } else if best != 0
            && let Some(symbol) = model.symbol(best)
        {
            chars.extend(symbol.chars().map(|c| (c, p)));
        }
        previous = best;
    }
    let first = chars.iter().position(|(c, _)| *c != ' ');
    let last = chars.iter().rposition(|(c, _)| *c != ' ');
    match (first, last) {
        (Some(a), Some(b)) => chars[a..=b].iter().copied().unzip(),
        _ => (String::new(), Vec::new()),
    }
}
