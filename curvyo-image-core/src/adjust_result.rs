//! The stored results of the AI steps applied to a picture in integer arithmetic (ADR 0077 §3;
//! `specs/0135` criteria 47, 48): no model runs here, only the stored pixels are read.

use std::collections::HashMap;
use std::sync::Arc;

use crate::adjust_plane::{GreyPlane, byte, luminance, over_white};
use crate::working::WorkingImage;

/// The decoded stored results a run reads, by the SHA-256 of their PNG bytes (the derived asset's
/// hash). A mask is held as the grey of its pixels' red channel. Lookups only.
#[derive(Debug, Clone, Default)]
pub struct StoredPixels {
    held: HashMap<[u8; 32], Arc<WorkingImage>>,
}

impl StoredPixels {
    /// Holds `pixels` as the stored result `hash`.
    pub fn insert(&mut self, hash: [u8; 32], pixels: Arc<WorkingImage>) {
        self.held.insert(hash, pixels);
    }

    /// The stored result `hash`, when held.
    #[must_use]
    pub fn get(&self, hash: &[u8; 32]) -> Option<&Arc<WorkingImage>> {
        self.held.get(hash)
    }

    /// Whether the stored result `hash` is held.
    #[must_use]
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.held.contains_key(hash)
    }
}

/// `image` composited over white, opaque (the picture an AI step reads, ADR 0077 §2).
#[must_use]
pub fn opaque_over_white(image: &WorkingImage) -> WorkingImage {
    let pixels = image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            [
                over_white(p[0], p[3]),
                over_white(p[1], p[3]),
                over_white(p[2], p[3]),
                255,
            ]
        })
        .collect();
    WorkingImage {
        pixels,
        ..image.clone()
    }
}

/// The planes of an Upscale replaced by its stored pixels: one plane per channel when
/// `channels` is 3, else their grey.
pub(crate) fn replaced(stored: &WorkingImage, channels: usize) -> Vec<GreyPlane> {
    let plane = |grey: Vec<u8>| GreyPlane {
        width: stored.width,
        height: stored.height,
        grey,
    };
    let px = stored.pixels.as_chunks::<4>().0;
    if channels == 3 {
        (0..3)
            .map(|c| plane(px.iter().map(|p| over_white(p[c], p[3])).collect()))
            .collect()
    } else {
        vec![plane(
            px.iter()
                .map(|p| {
                    luminance(
                        over_white(p[0], p[3]),
                        over_white(p[1], p[3]),
                        over_white(p[2], p[3]),
                    )
                })
                .collect(),
        )]
    }
}

/// `input + (ai - input) · strength / 100`, rounded half away from zero (ADR 0077 §3).
#[must_use]
pub fn blend(input: u8, ai: u8, strength_pct: u8) -> u8 {
    let d = (i32::from(ai) - i32::from(input)) * i32::from(strength_pct.min(100));
    let q = (d.abs() + 50) / 100;
    let out = i32::from(input) + if d < 0 { -q } else { q };
    byte(u32::try_from(out.max(0)).unwrap_or(0))
}

/// The AI denoise of `plane` (channel `channel`, or the grey of the stored pixels when
/// `channel` is `None`), blended with the stored output. A plane of another size (a reduced
/// level of a preview) reads the stored output at the nearest pixel.
pub(crate) fn denoised(
    plane: &mut GreyPlane,
    stored: &WorkingImage,
    channel: Option<usize>,
    strength_pct: u8,
) {
    let (sw, sh) = (stored.width.max(1), stored.height.max(1));
    let px = stored.pixels.as_chunks::<4>().0;
    if px.len() != sw as usize * sh as usize {
        return;
    }
    let (w, h) = (plane.width.max(1), plane.height.max(1));
    let ai_at = |p: &[u8; 4]| match channel {
        // An opaque pixel is itself over white (the stored AI denoise output is opaque).
        Some(c) if p[3] == 255 => p[c],
        Some(c) => over_white(p[c], p[3]),
        None => luminance(
            over_white(p[0], p[3]),
            over_white(p[1], p[3]),
            over_white(p[2], p[3]),
        ),
    };
    // `blend` as an offset by the difference `ai - input` (-255 to 255): one table per call
    // instead of a multiply and a division per pixel.
    let offsets: Vec<i16> = (-255_i16..=255)
        .map(|d| {
            let input = if d < 0 { 255 } else { 0 };
            let ai = u8::try_from(i16::from(input) + d).unwrap_or(0);
            i16::from(blend(input, ai, strength_pct)) - i16::from(input)
        })
        .collect();
    let mix = |input: &mut u8, ai: u8| {
        let off = offsets[usize::try_from(i16::from(ai) - i16::from(*input) + 255).unwrap_or(0)];
        // invariant: the blend lies between input and ai, so within 0..=255.
        *input = u8::try_from(i16::from(*input) + off).unwrap_or(*input);
    };
    if (w, h) == (sw, sh) {
        for (g, p) in plane.grey.iter_mut().zip(px) {
            mix(g, ai_at(p));
        }
        return;
    }
    let columns: Vec<usize> = (0..w)
        .map(|x| usize::try_from(u64::from(x) * u64::from(sw) / u64::from(w)).unwrap_or(0))
        .collect();
    for (y, row) in plane.grey.chunks_exact_mut(w as usize).enumerate() {
        let sy = usize::try_from(y as u64 * u64::from(sh) / u64::from(h)).unwrap_or(0);
        let source = &px[sy * sw as usize..(sy + 1) * sw as usize];
        for (g, &sx) in row.iter_mut().zip(&columns) {
            mix(g, ai_at(&source[sx]));
        }
    }
}

/// The stored mask (its red channel) resized to `width` x `height` by integer bilinear
/// sampling at pixel centres (weights in 1/256), rounded half up.
#[must_use]
pub fn mask_resized(stored: &WorkingImage, width: u32, height: u32) -> GreyPlane {
    let (sw, sh) = (stored.width.max(1), stored.height.max(1));
    let at = |x: u32, y: u32| u32::from(stored.pixels[(y as usize * sw as usize + x as usize) * 4]);
    if stored.pixels.len() != sw as usize * sh as usize * 4 {
        return GreyPlane::white(width, height);
    }
    // The source position of a target centre in 1/256 px: ((2t + 1) · s - d) / (2d) - 0.5.
    let axis = |t: u32, d: u32, s: u32| -> (u32, u32, u32) {
        let num = (2 * i64::from(t) + 1) * i64::from(s) * 256 - 256 * i64::from(d);
        let pos = (num / (2 * i64::from(d))).max(0);
        let max = (i64::from(s) - 1) * 256;
        let pos = pos.min(max);
        let i0 = u32::try_from(pos / 256).unwrap_or(0);
        let f = u32::try_from(pos % 256).unwrap_or(0);
        (i0, (i0 + 1).min(s - 1), f)
    };
    let cols: Vec<_> = (0..width).map(|x| axis(x, width, sw)).collect();
    let mut grey = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        let (y0, y1, fy) = axis(y, height, sh);
        for &(x0, x1, fx) in &cols {
            let top = at(x0, y0) * (256 - fx) + at(x1, y0) * fx;
            let bottom = at(x0, y1) * (256 - fx) + at(x1, y1) * fx;
            let v = top * (256 - fy) + bottom * fy;
            grey.push(byte((v + (1 << 15)) >> 16));
        }
    }
    GreyPlane {
        width,
        height,
        grey,
    }
}

/// The mask value `m` mapped by Threshold `T = threshold_pct · 255 / 100` and Edge softness `w =
/// max(1, softness_pct · 128 / 100)` (ADR 0077 §3): 0 below `T - w`, 255 above `T + w`, linear
/// between.
#[must_use]
pub fn mask_map(m: u8, threshold_pct: u8, softness_pct: u8) -> u8 {
    let t = i32::from(threshold_pct.min(100)) * 255 / 100;
    let w = (i32::from(softness_pct.min(100)) * 128 / 100).max(1);
    let m = i32::from(m);
    if m <= t - w {
        0
    } else if m >= t + w {
        255
    } else {
        byte(u32::try_from(((m - (t - w)) * 255 + w) / (2 * w)).unwrap_or(0))
    }
}

/// The mask applied to one plane (composited over white): `255 - (255 - g) · m' / 255`, so
/// what is masked out is white and burns nothing (criterion 47).
pub(crate) fn masked(plane: &mut GreyPlane, mask: &GreyPlane) {
    for (g, &m) in plane.grey.iter_mut().zip(&mask.grey) {
        let dark = (255 - u32::from(*g)) * u32::from(m);
        *g = byte(255 - (dark + 127) / 255);
    }
}

/// The mask plane of a stored mask for a plane of `width` x `height`, mapped by the step's
/// fields.
pub(crate) fn mapped_mask(
    stored: &WorkingImage,
    (width, height): (u32, u32),
    threshold_pct: u8,
    softness_pct: u8,
) -> GreyPlane {
    let mut mask = mask_resized(stored, width, height);
    for m in &mut mask.grey {
        *m = mask_map(*m, threshold_pct, softness_pct);
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(width: u32, height: u32, g: impl Fn(u32, u32) -> u8) -> WorkingImage {
        WorkingImage {
            width,
            height,
            pixels: (0..height)
                .flat_map(|y| (0..width).map(move |x| (x, y)))
                .flat_map(|(x, y)| {
                    let v = g(x, y);
                    [v, v, v, 255]
                })
                .collect(),
            original_size: (width, height),
            truncated: false,
            animated: false,
        }
    }

    #[test]
    fn the_blend_rounds_half_away_from_zero() {
        assert_eq!(blend(100, 200, 0), 100);
        assert_eq!(blend(100, 200, 100), 200);
        assert_eq!(blend(100, 201, 50), 151, "50.5 rounds up");
        assert_eq!(blend(201, 100, 50), 150, "-50.5 rounds away from zero");
    }

    #[test]
    fn the_mask_map_is_a_ramp_around_the_threshold() {
        // T = 127, w = 6 at 5 %.
        assert_eq!(mask_map(0, 50, 5), 0);
        assert_eq!(mask_map(121, 50, 5), 0);
        assert_eq!(mask_map(133, 50, 5), 255);
        assert_eq!(mask_map(127, 50, 5), 128);
        assert_eq!(
            mask_map(255, 100, 0),
            128,
            "Threshold 100 keeps half of white"
        );
        assert_eq!(mask_map(254, 100, 0), 0);
    }

    #[test]
    fn a_resized_mask_keeps_flat_areas_and_its_size() {
        let half = grey(4, 4, |x, _| if x < 2 { 0 } else { 255 });
        let out = mask_resized(&half, 8, 2);
        assert_eq!((out.width, out.height), (8, 2));
        assert_eq!(&out.grey[..2], &[0, 0]);
        assert_eq!(&out.grey[6..8], &[255, 255]);
        assert!(out.grey[3] < out.grey[4], "a ramp at the edge");
        let same = mask_resized(&half, 4, 4);
        assert_eq!(
            same.grey,
            half.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[0])
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn masked_out_is_white() {
        let mut plane = GreyPlane {
            width: 3,
            height: 1,
            grey: vec![0, 0, 100],
        };
        masked(
            &mut plane,
            &GreyPlane {
                width: 3,
                height: 1,
                grey: vec![0, 255, 255],
            },
        );
        assert_eq!(plane.grey, vec![255, 0, 100]);
    }
}
