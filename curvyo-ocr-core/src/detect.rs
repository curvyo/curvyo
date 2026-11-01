//! Finds word boxes in a raster with the detector.
//!
//! `PaddleOCR`'s DB detector on a copy whose longer side is at most [`DETECTION_SIDE`] px (sides
//! multiples of 32); the probability map is cut at [`PROBABILITY_CUT`], 4-connected components
//! of more than [`MIN_PIXELS`] become boxes oriented along their principal axis, grown by DB's
//! "unclip" ratio, in source pixels (ADR 0027 §3; constants are the spike's, Proposals).

use rten::Model;
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::OcrError;
use crate::resize::{Grid, Rgb, sample};

/// The longest side the detector sees, px.
pub const DETECTION_SIDE: usize = 960;
/// The probability above which a detector pixel is text.
const PROBABILITY_CUT: f32 = 0.3;
/// Components of this many detector pixels or fewer are noise.
const MIN_PIXELS: usize = 10;
/// DB's unclip ratio: the box grows by `area * ratio / perimeter` on every side.
const UNCLIP_RATIO: f32 = 1.5;
/// A component whose principal spread is less than this times the other is not elongated: its
/// box stays upright.
const MIN_ELONGATION: f32 = 1.5;
/// Directions within this many degrees of the horizontal are taken as horizontal.
const LEVEL_DEGREES: f32 = 1.0;
/// `ImageNet` mean and deviation, in the detector's channel order.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const DEVIATION: [f32; 3] = [0.229, 0.224, 0.225];

/// A box along a direction, in source pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RotatedBox {
    pub(crate) centre: [f32; 2],
    /// The unit direction of the text, pointing right (`u[0] >= 0`).
    pub(crate) u: [f32; 2],
    /// Half the length along `u`.
    pub(crate) half_u: f32,
    /// Half the height across `u`.
    pub(crate) half_v: f32,
}

impl RotatedBox {
    /// The unit normal, pointing down when `u` points right.
    pub(crate) fn v(&self) -> [f32; 2] {
        [-self.u[1], self.u[0]]
    }

    /// The corners: top left, top right, bottom right, bottom left.
    pub(crate) fn quad(&self) -> [[f32; 2]; 4] {
        let (u, v, c) = (self.u, self.v(), self.centre);
        let at = |a: f32, b: f32| [c[0] + u[0] * a + v[0] * b, c[1] + u[1] * a + v[1] * b];
        [
            at(-self.half_u, -self.half_v),
            at(self.half_u, -self.half_v),
            at(self.half_u, self.half_v),
            at(-self.half_u, self.half_v),
        ]
    }
}

/// The detector's input size for a `width` x `height` raster.
fn input_size(width: usize, height: usize) -> (usize, usize) {
    let longest = width.max(height) as f32;
    let scale = (DETECTION_SIDE as f32 / longest).min(1.0);
    let side = |n: usize| {
        // invariant: positive and far below usize::MAX.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let k = (n as f32 * scale / 32.0).round().max(1.0) as usize;
        k * 32
    };
    (side(width), side(height))
}

/// The word boxes of `image`, in source pixels, in component order (row-major first pixel).
pub(crate) fn detect(model: &Model, image: &Rgb) -> Result<Vec<RotatedBox>, OcrError> {
    let (dw, dh) = input_size(image.width, image.height);
    let (sx, sy) = (
        image.width as f32 / dw as f32,
        image.height as f32 / dh as f32,
    );
    let grid = Grid {
        origin: [0.0, 0.0],
        step_x: [sx, 0.0],
        step_y: [0.0, sy],
        width: dw,
        height: dh,
    };
    let rgb = sample(image, &grid);
    // Paddle's order: the blue channel first.
    let input = NdTensor::from_fn([1, 3, dh, dw], |[_, c, y, x]| {
        (rgb[y * dw + x][2 - c] / 255.0 - MEAN[c]) / DEVIATION[c]
    });
    let output: Tensor<f32> = model
        .run_one(input.view().into(), None)
        .map_err(|e| OcrError::BadModel(e.to_string()))?
        .try_into()
        .map_err(|e: rten::TryFromValueError| OcrError::BadModel(e.to_string()))?;
    let shape = output.shape().to_vec();
    if shape.len() != 4 || shape[2] != dh || shape[3] != dw {
        return Err(OcrError::BadModel(format!(
            "detector output {shape:?} for {dw} x {dh}"
        )));
    }
    let probability = output.to_vec();
    Ok(components(&probability, dw, dh)
        .iter()
        .filter_map(|pixels| principal_box(pixels, dw, sx, sy))
        .collect())
}

/// The 4-connected components above the cut with more than [`MIN_PIXELS`] pixels, each as its
/// pixel indices.
fn components(probability: &[f32], width: usize, height: usize) -> Vec<Vec<usize>> {
    let mut seen = vec![false; width * height];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..width * height {
        if seen[start] || probability[start] <= PROBABILITY_CUT {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut pixels = Vec::new();
        while let Some(i) = stack.pop() {
            pixels.push(i);
            let (x, y) = (i % width, i / width);
            let mut visit = |j: usize| {
                if !seen[j] && probability[j] > PROBABILITY_CUT {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < width {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - width);
            }
            if y + 1 < height {
                visit(i + width);
            }
        }
        if pixels.len() > MIN_PIXELS {
            pixels.sort_unstable();
            out.push(pixels);
        }
    }
    out
}

/// The box of `pixels` (indices into a `width` wide detector map with source scales `sx`, `sy`).
#[allow(clippy::many_single_char_names)] // the axes u, v and the coordinates a, b of the frame
fn principal_box(pixels: &[usize], width: usize, sx: f32, sy: f32) -> Option<RotatedBox> {
    if pixels.is_empty() {
        return None;
    }
    let points: Vec<[f32; 2]> = pixels
        .iter()
        .map(|&i| {
            [
                ((i % width) as f32 + 0.5) * sx,
                ((i / width) as f32 + 0.5) * sy,
            ]
        })
        .collect();
    let n = points.len() as f32;
    let (mut mx, mut my) = (0.0_f32, 0.0_f32);
    for p in &points {
        mx += p[0];
        my += p[1];
    }
    mx /= n;
    my /= n;
    let (mut xx, mut yy, mut xy) = (0.0_f32, 0.0_f32, 0.0_f32);
    for p in &points {
        let (dx, dy) = (p[0] - mx, p[1] - my);
        xx += dx * dx;
        yy += dy * dy;
        xy += dx * dy;
    }
    // The principal axis of the covariance, and its spread against the other.
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let (cos, sin) = (angle.cos(), angle.sin());
    let along = xx * cos * cos + 2.0 * xy * cos * sin + yy * sin * sin;
    let across = xx * sin * sin - 2.0 * xy * cos * sin + yy * cos * cos;
    let elongated = along > MIN_ELONGATION * MIN_ELONGATION * across.max(0.0);
    let level = angle.abs().to_degrees() < LEVEL_DEGREES;
    let mut u = if elongated && !level {
        [cos, sin]
    } else {
        [1.0, 0.0]
    };
    if u[0] < 0.0 {
        u = [-u[0], -u[1]];
    }
    let v = [-u[1], u[0]];
    let (mut a0, mut a1, mut b0, mut b1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &points {
        let (dx, dy) = (p[0] - mx, p[1] - my);
        let a = dx * u[0] + dy * u[1];
        let b = dx * v[0] + dy * v[1];
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    // Each point stands for a whole detector pixel.
    let half_pixel = 0.5 * sx.max(sy);
    let (len_u, len_v) = (a1 - a0 + 2.0 * half_pixel, b1 - b0 + 2.0 * half_pixel);
    let grow = n * sx * sy * UNCLIP_RATIO / (2.0 * (len_u + len_v));
    let (cu, cv) = (f32::midpoint(a0, a1), f32::midpoint(b0, b1));
    Some(RotatedBox {
        centre: [mx + u[0] * cu + v[0] * cv, my + u[1] * cu + v[1] * cv],
        u,
        half_u: 0.5 * len_u + grow,
        half_v: 0.5 * len_v + grow,
    })
}
