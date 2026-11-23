//! The three model runs on a picture (ADR 0075 §2; `specs/0135` criterion 47): the picture is read
//! opaque over white, sent to the model in fixed tiles in a fixed order, and the raw output is
//! returned for the caller to store; one tile at a time for a pool of workers (ADR 0078 §2).
//! `poll(done, total)` is called before each tile; `false` cancels.

use curvyo_image_core::{GreyPlane, MAX_WORKING_PIXELS, MAX_WORKING_SIDE, WorkingImage};
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::error::AiError;
use crate::model::Model;
use crate::models::ModelKind;
use crate::tiles::{TILE, TILE_OVERLAP, TileOutput, assemble, tile_count, tiles};

/// The side of the Remove background model's input and of the stored mask, in pixels.
pub const MASK_SIDE: u32 = 320;

/// The Upscale factor (criterion 47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Factor {
    /// Twice the pixels per side: the x4 model's output box-averaged 2 x 2 (ADR 0075 §2).
    Two,
    /// Four times the pixels per side.
    Four,
}

impl Factor {
    /// The factor as a number.
    #[must_use]
    pub const fn times(self) -> u32 {
        match self {
            Self::Two => 2,
            Self::Four => 4,
        }
    }
}

/// The `ImageNet` mean and deviation the mask model reads its input with.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const DEVIATION: [f32; 3] = [0.229, 0.224, 0.225];

fn check_kind(model: &Model, kind: ModelKind) -> Result<(), AiError> {
    if model.kind == kind {
        Ok(())
    } else {
        Err(AiError::BadModel(format!(
            "a {} model, not {}",
            model.kind.key(),
            kind.key()
        )))
    }
}

fn check_image(image: &WorkingImage) -> Result<(), AiError> {
    let (w, h) = (image.width, image.height);
    if w == 0 || h == 0 || image.pixels.len() != w as usize * h as usize * 4 {
        return Err(AiError::Invalid("the picture has no pixels".into()));
    }
    Ok(())
}

/// The channel `c` of pixel (`x`, `y`) composited over white, 0 to 1; outside the picture the
/// edge repeats.
fn sample(image: &WorkingImage, x: i64, y: i64, c: usize) -> f32 {
    let x = x.clamp(0, i64::from(image.width) - 1);
    let y = y.clamp(0, i64::from(image.height) - 1);
    let at = (usize::try_from(y).unwrap_or(0) * image.width as usize
        + usize::try_from(x).unwrap_or(0))
        * 4;
    let alpha = f32::from(image.pixels[at + 3]) / 255.0;
    let value = f32::from(image.pixels[at + c]) / 255.0;
    value * alpha + (1.0 - alpha)
}

fn to_byte(v: f32) -> u8 {
    // invariant: clamped to 0..=255 before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let b = (v * 255.0).round().clamp(0.0, 255.0) as u8;
    b
}

fn run_tensor(model: &Model, input: &NdTensor<f32, 4>) -> Result<Tensor<f32>, AiError> {
    model
        .graph
        .run_one(input.view().into(), None)
        .map_err(|e| AiError::BadModel(e.to_string()))?
        .try_into()
        .map_err(|e: rten::TryFromValueError| AiError::BadModel(e.to_string()))
}

/// The raw subject mask of `image` (Remove background): the picture resized to 320 x 320, the
/// model's saliency output stretched to 0 to 255 (ADR 0075 §2; the step's Threshold and Edge
/// softness are applied later to the stored mask, ADR 0077 §3).
///
/// # Errors
/// [`AiError::BadModel`] for another kind of model or an output of another shape;
/// [`AiError::Invalid`] for an empty picture; [`AiError::Cancelled`].
pub fn remove_background(
    model: &Model,
    image: &WorkingImage,
    poll: &mut dyn FnMut(u32, u32) -> bool,
) -> Result<GreyPlane, AiError> {
    check_kind(model, ModelKind::Background)?;
    check_image(image)?;
    if !poll(0, 1) {
        return Err(AiError::Cancelled);
    }
    let side = MASK_SIDE as usize;
    let (w, h) = (f64::from(image.width), f64::from(image.height));
    let input = NdTensor::from_fn([1, 3, side, side], |[_, c, y, x]| {
        // The nearest source pixel of the target's centre.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let (sx, sy) = (
            (((x as f64) + 0.5) * w / f64::from(MASK_SIDE)).floor() as i64,
            (((y as f64) + 0.5) * h / f64::from(MASK_SIDE)).floor() as i64,
        );
        (sample(image, sx, sy, c) - MEAN[c]) / DEVIATION[c]
    });
    let output = run_tensor(model, &input)?;
    let shape = output.shape().to_vec();
    if shape.len() != 4 || shape[1] != 1 || shape[2] != side || shape[3] != side {
        return Err(AiError::BadModel(format!("mask output {shape:?}")));
    }
    let values = output.to_vec();
    let (lo, hi) = values
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let span = if hi > lo { hi - lo } else { 1.0 };
    if !poll(1, 1) {
        return Err(AiError::Cancelled);
    }
    Ok(GreyPlane {
        width: MASK_SIDE,
        height: MASK_SIDE,
        grey: values.iter().map(|&v| to_byte((v - lo) / span)).collect(),
    })
}

/// Tile `index` of a tiled model of `scale` on `image`: its core in output pixels, RGB opaque.
fn run_tile(
    model: &Model,
    image: &WorkingImage,
    scale: u32,
    index: u32,
) -> Result<TileOutput, AiError> {
    let tile = *tiles(image.width, image.height)
        .get(index as usize)
        .ok_or_else(|| AiError::Invalid(format!("no tile {index}")))?;
    let side = TILE as usize;
    let (ox, oy) = tile.origin;
    let input = NdTensor::from_fn([1, 3, side, side], |[_, c, y, x]| {
        #[allow(clippy::cast_possible_wrap)]
        let (x, y) = (ox + x as i64, oy + y as i64);
        sample(image, x, y, c)
    });
    let output = run_tensor(model, &input)?;
    let shape = output.shape().to_vec();
    let out_side = side * scale as usize;
    if shape != [1, 3, out_side, out_side] {
        return Err(AiError::BadModel(format!("tile output {shape:?}")));
    }
    let values = output.to_vec();
    let (_, _, cw, ch) = tile.core;
    let (w, h) = (cw * scale, ch * scale);
    let offset = (TILE_OVERLAP * scale) as usize;
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let (tx, ty) = (offset + x, offset + y);
            for c in 0..3 {
                rgba.push(to_byte(values[(c * out_side + ty) * out_side + tx]));
            }
            rgba.push(255);
        }
    }
    Ok(TileOutput {
        index,
        width: w,
        height: h,
        rgba,
    })
}

/// Runs a tiled model on `image` tile by tile, `tile(index)` each, and assembles the result of
/// `width` x `height` (ADR 0078 §2: the one loop a single worker runs).
fn tiled(
    image: &WorkingImage,
    (width, height): (u32, u32),
    poll: &mut dyn FnMut(u32, u32) -> bool,
    tile: &dyn Fn(u32) -> Result<TileOutput, AiError>,
) -> Result<WorkingImage, AiError> {
    let total = tile_count(image.width, image.height);
    let mut out = Vec::with_capacity(total as usize);
    for index in 0..total {
        if !poll(index, total) {
            return Err(AiError::Cancelled);
        }
        out.push(tile(index)?);
    }
    if !poll(total, total) {
        return Err(AiError::Cancelled);
    }
    assemble(width, height, out)
}

/// The tile's pixels halved per side by a 2 x 2 integer box mean, rounded half up.
fn halved(tile: &TileOutput) -> TileOutput {
    let (w, h) = (tile.width / 2, tile.height / 2);
    let at = |x: u32, y: u32, c: usize| {
        u32::from(tile.rgba[(y as usize * tile.width as usize + x as usize) * 4 + c])
    };
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let sum = at(2 * x, 2 * y, c)
                    + at(2 * x + 1, 2 * y, c)
                    + at(2 * x, 2 * y + 1, c)
                    + at(2 * x + 1, 2 * y + 1, c);
                rgba.push(u8::try_from((sum + 2) / 4).unwrap_or(u8::MAX));
            }
            rgba.push(255);
        }
    }
    TileOutput {
        index: tile.index,
        width: w,
        height: h,
        rgba,
    }
}

/// The size an Upscale of `width` x `height` by `factor` gives, refused over the working limit
/// before any work (criteria 47, 66).
///
/// # Errors
/// [`AiError::TooLarge`] over 16 megapixels or 8192 px on a side.
pub fn upscaled_size(width: u32, height: u32, factor: Factor) -> Result<(u32, u32), AiError> {
    let (w, h) = (
        u64::from(width) * u64::from(factor.times()),
        u64::from(height) * u64::from(factor.times()),
    );
    if w > u64::from(MAX_WORKING_SIDE)
        || h > u64::from(MAX_WORKING_SIDE)
        || w * h > MAX_WORKING_PIXELS
    {
        return Err(AiError::TooLarge {
            width: w,
            height: h,
        });
    }
    Ok((
        u32::try_from(w).unwrap_or(u32::MAX),
        u32::try_from(h).unwrap_or(u32::MAX),
    ))
}

/// Tile `index` of an Upscale of `image` by `factor` (ADR 0078 §2): the x4 model's output core,
/// halved inside the tile for x2 (each core is a multiple of 896 output px, so this equals
/// halving the assembled x4 picture).
///
/// # Errors
/// [`AiError::TooLarge`]; [`AiError::BadModel`]; [`AiError::Invalid`] for an empty picture or an
/// index past the last tile.
pub fn upscale_tile(
    model: &Model,
    image: &WorkingImage,
    factor: Factor,
    index: u32,
) -> Result<TileOutput, AiError> {
    check_kind(model, ModelKind::Upscale)?;
    check_image(image)?;
    upscaled_size(image.width, image.height, factor)?;
    let x4 = run_tile(model, image, 4, index)?;
    Ok(match factor {
        Factor::Four => x4,
        Factor::Two => halved(&x4),
    })
}

/// Tile `index` of an AI denoise of `image` (ADR 0078 §2): the model's output core.
///
/// # Errors
/// [`AiError::BadModel`]; [`AiError::Invalid`] for an empty picture or an index past the last
/// tile.
pub fn denoise_tile(
    model: &Model,
    image: &WorkingImage,
    index: u32,
) -> Result<TileOutput, AiError> {
    check_kind(model, ModelKind::Denoise)?;
    check_image(image)?;
    run_tile(model, image, 1, index)
}

/// The upscaled picture (Upscale): the x4 model in tiles; x2 is its output halved by a 2 x 2
/// box (ADR 0075 §2). Refused before any work when the result would pass the working limit.
///
/// # Errors
/// [`AiError::TooLarge`]; [`AiError::BadModel`]; [`AiError::Invalid`]; [`AiError::Cancelled`].
pub fn upscale(
    model: &Model,
    image: &WorkingImage,
    factor: Factor,
    poll: &mut dyn FnMut(u32, u32) -> bool,
) -> Result<WorkingImage, AiError> {
    check_kind(model, ModelKind::Upscale)?;
    check_image(image)?;
    let size = upscaled_size(image.width, image.height, factor)?;
    tiled(image, size, poll, &|index| {
        upscale_tile(model, image, factor, index)
    })
}

/// The denoised picture (AI denoise): the model in tiles at the same size; the step's Strength
/// blends it later (ADR 0077 §3).
///
/// # Errors
/// [`AiError::BadModel`]; [`AiError::Invalid`]; [`AiError::Cancelled`].
pub fn denoise(
    model: &Model,
    image: &WorkingImage,
    poll: &mut dyn FnMut(u32, u32) -> bool,
) -> Result<WorkingImage, AiError> {
    check_kind(model, ModelKind::Denoise)?;
    check_image(image)?;
    tiled(image, (image.width, image.height), poll, &|index| {
        denoise_tile(model, image, index)
    })
}
