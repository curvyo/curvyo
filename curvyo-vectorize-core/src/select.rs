//! The selection step of a trace (`specs/0089-trace-quality-and-colour` slice C, `adrs.md` §3):
//! per pixel a score from the method and the alpha, which Denoise and Upscale filter and whose
//! product `α/255 · s ≥ ½` is the mask; alpha is counted once and never composited over a
//! colour. Colours are compared as their stored sRGB bytes (trial C1, `plan.md`): Euclidean
//! distance for Remove background, the two-colour mixing model for Pick colours.

use crate::params::{Selection, TraceParams};
use crate::raster::{Raster, luma};

/// `α · s` at or above this (both 0 to 255) is content: membership ½ of 255².
const HALF: u32 = 32_513;

/// A pixel with alpha at or above this is opaque enough to vote for the background colour and
/// to count by its luma in the automatic Threshold (membership ½ of a fully selected colour).
pub(crate) const OPAQUE: u8 = 128;

/// The colour tolerance 100 as a distance in the sRGB unit cube.
const GATE_AT_100: f64 = 0.5;

/// The colour tolerance a first use of Pick colours starts at (Proposal): with the noise
/// filter of the Pick colours scorer it selects 99 % or more of a picked shape with +-6 per channel of
/// noise (19: 99.2 %), and picking #D9D9D9 leaves the star's anti-aliased rim and the solid
/// #DCD6CF out, which it no longer does from 20.
pub const DEFAULT_PICK_TOLERANCE: u8 = 19;

/// The colour tolerance a first use of Remove background starts at.
pub const DEFAULT_BACKGROUND_TOLERANCE: u8 = 40;

/// The colour distance (sRGB bytes / 255, Euclidean) of a colour tolerance 0 to 100:
/// `0.5 · (t/100)²`, fine at the low end where pale colours are told apart.
pub(crate) fn gate(tolerance: u8) -> f64 {
    let t = f64::from(tolerance.min(100)) / 100.0;
    GATE_AT_100 * t * t * 255.0
}

/// The per-pixel score (0 to 255) and, when any pixel is not opaque, the alpha.
pub(crate) struct Fields {
    pub score: Vec<u8>,
    pub alpha: Option<Vec<u8>>,
}

/// The score and alpha fields of `raster` for `params`' method: the luma for Brightness (cut
/// later at the Threshold), else the membership of the colour method times 255.
pub(crate) fn fields(raster: Raster<'_>, params: &TraceParams) -> Fields {
    let pixels = raster.rgba.as_chunks::<4>().0;
    let score = match params.selection {
        Selection::Brightness => pixels.iter().map(|&[r, g, b, _]| luma(r, g, b)).collect(),
        Selection::RemoveBackground {
            background,
            tolerance,
        } => {
            let limit = gate(tolerance);
            pixels
                .iter()
                .map(|&[r, g, b, _]| {
                    background.map_or(255, |bg| {
                        if distance(colour(r, g, b), colour(bg.r, bg.g, bg.b)) > limit {
                            255
                        } else {
                            0
                        }
                    })
                })
                .collect()
        }
        Selection::PickColours {
            swatches,
            background,
            tolerance,
        } => crate::picker::scores(raster, swatches.as_slice(), background, tolerance),
    };
    let alpha = pixels
        .iter()
        .any(|p| p[3] != 255)
        .then(|| pixels.iter().map(|p| p[3]).collect());
    Fields { score, alpha }
}

/// Whether a pixel of filtered `score` and `alpha` is content: the method's membership `s`
/// (Brightness: 255 below the Threshold), flipped by Invert, times the alpha, at least ½.
pub(crate) fn is_content(params: &TraceParams, score: u8, alpha: u8) -> bool {
    let s = match params.selection {
        Selection::Brightness => {
            if score < params.threshold {
                255
            } else {
                0
            }
        }
        _ => u32::from(score),
    };
    let s = if params.invert { 255 - s } else { s };
    u32::from(alpha) * s >= HALF
}

/// The pixels `params` makes content, before Denoise and Upscale, row by row: what the tracer's
/// mask starts from (the Mask view of slice E and the harness's reference read it).
#[must_use]
pub fn selected(raster: Raster<'_>, params: &TraceParams) -> Vec<bool> {
    if raster.checked_pixels().is_none() {
        return Vec::new();
    }
    let fields = fields(raster, params);
    match &fields.alpha {
        Some(alpha) => fields
            .score
            .iter()
            .zip(alpha)
            .map(|(&s, &a)| is_content(params, s, a))
            .collect(),
        None => fields
            .score
            .iter()
            .map(|&s| is_content(params, s, 255))
            .collect(),
    }
}

/// A colour in byte units as `f64`.
pub(crate) fn colour(r: u8, g: u8, b: u8) -> [f64; 3] {
    [f64::from(r), f64::from(g), f64::from(b)]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use crate::background::{sample_colour, suggest_background};
    use crate::params::{Rgb8, Swatches};

    fn raster(width: u32, height: u32, rgba: &[u8]) -> Raster<'_> {
        Raster {
            width,
            height,
            rgba,
        }
    }

    fn pick(colours: &[Rgb8], tolerance: u8) -> TraceParams {
        TraceParams {
            selection: Selection::PickColours {
                swatches: Swatches::from_slice(colours),
                background: Some(Rgb8::new(255, 255, 255)),
                tolerance,
            },
            ..TraceParams::with_threshold(128)
        }
    }

    /// The preview copy and the full-size image give the same score for the same pixel: Pick
    /// colours uses the background it is given, never one read from the raster at hand.
    #[test]
    fn pick_colours_uses_the_given_background_at_any_size() {
        let ring = |edge: u8| {
            let mut px = Vec::new();
            for i in 0..9 {
                let v = if i == 4 { 236 } else { edge };
                px.extend_from_slice(&[v, v, v, 255]);
            }
            px
        };
        let (white_edge, black_edge) = (ring(255), ring(0));
        let params = pick(&[Rgb8::new(217, 217, 217)], 15);
        let on_white = fields(raster(3, 3, &white_edge), &params).score[4];
        let on_black = fields(raster(3, 3, &black_edge), &params).score[4];
        assert!(on_white > 0);
        assert_eq!(on_white, on_black);
    }

    /// Criterion 15: α = 0 is never content, whatever the RGB, the method or Invert; a 50 %
    /// alpha black pixel has membership ½ (counted once, not composited over white first).
    #[test]
    fn alpha_counts_once_and_transparent_is_never_content() {
        let px = [0_u8, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 127, 0, 0, 0, 255];
        let r = raster(4, 1, &px);
        let mut p = TraceParams::with_threshold(128);
        assert_eq!(selected(r, &p), [false, true, false, true]);
        p.invert = true;
        assert_eq!(selected(r, &p), [false, false, false, false]);
        let white = [255_u8, 255, 255, 0, 255, 255, 255, 255];
        assert_eq!(selected(raster(2, 1, &white), &p), [false, true]);
        let remove = TraceParams {
            selection: Selection::RemoveBackground {
                background: None,
                tolerance: 40,
            },
            ..p
        };
        assert_eq!(selected(r, &remove), [false, false, false, false]);
        let remove = TraceParams {
            invert: false,
            ..remove
        };
        assert_eq!(selected(r, &remove), [false, true, false, true]);
        let picked = pick(&[Rgb8::new(0, 0, 0)], 15);
        assert_eq!(selected(r, &picked), [false, true, false, true]);
    }

    /// Criterion 17: on white, #D9D9D9 picked takes its blends toward white at their coverage
    /// and leaves the solid #DCD6CF out.
    #[test]
    fn near_greys_are_told_apart_and_blends_count_by_coverage() {
        let grey = Rgb8::new(0xD9, 0xD9, 0xD9);
        let warm = Rgb8::new(0xDC, 0xD6, 0xCF);
        let blend = |c: Rgb8, f: f64| {
            let m = |v: u8| (f64::from(v) * f + 255.0 * (1.0 - f)).round() as u8;
            [m(c.r), m(c.g), m(c.b), 255]
        };
        // A monotone ramp, as an anti-aliased edge is (the 3 x 3 median keeps ramps).
        let mut px = vec![255_u8; 4 * 4];
        for c in [
            blend(grey, 0.4),
            blend(grey, 0.6),
            blend(grey, 1.0),
            blend(grey, 1.0),
        ] {
            px.extend(c);
        }
        px.extend(vec![255_u8; 4 * 3]);
        px.extend(blend(warm, 1.0));
        px.extend(blend(warm, 1.0));
        let r = raster(13, 1, &px);
        let got = selected(r, &pick(&[grey], DEFAULT_PICK_TOLERANCE));
        assert_eq!(got[4..8], [false, true, true, true]);
        assert!(!got[11] && !got[12], "the solid #DCD6CF stays out");
    }

    /// The default Pick tolerance takes at least 99 % of a picked shape with +-6 per channel of
    /// noise.
    #[test]
    fn the_default_pick_tolerance_survives_jpeg_like_noise() {
        let red = [0xCC_u8, 0x22, 0x22];
        let mut px = Vec::new();
        for y in 0..60_u32 {
            for x in 0..60_u32 {
                let h = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663);
                let noisy = |k: u32, v: u8| (i32::from(v) + ((h >> (5 * k)) % 13) as i32 - 6) as u8;
                px.extend([noisy(0, red[0]), noisy(1, red[1]), noisy(2, red[2]), 255]);
            }
        }
        let params = pick(&[Rgb8::new(red[0], red[1], red[2])], DEFAULT_PICK_TOLERANCE);
        let got = selected(raster(60, 60, &px), &params);
        let share = got.iter().filter(|&&c| c).count() as f64 / got.len() as f64;
        assert!(share >= 0.99, "selected {share}");
    }

    /// Criterion 19: a blend of two touching picked colours is content.
    #[test]
    fn a_blend_between_two_picks_is_content() {
        let (y, r) = (Rgb8::new(0xF2, 0xC4, 0), Rgb8::new(0xD0, 0x20, 0x20));
        let mid = [0xE1_u8, 0x72, 0x10, 255];
        let mut px = vec![255_u8; 8];
        px.extend(mid);
        let ras = raster(3, 1, &px);
        assert!(selected(ras, &pick(&[y, r], 15))[2]);
        assert!(!selected(ras, &pick(&[y], 15))[2]);
    }

    #[test]
    fn the_background_is_the_most_frequent_border_colour() {
        let (w, h) = (10_u32, 6_u32);
        let mut px = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let inner = x > 0 && y > 0 && x < w - 1 && y < h - 1;
                px.extend(if inner || (x == 0 && y == 2) {
                    [255, 242, 168, 255]
                } else {
                    [0x33, 0x66, 0xCC, 255]
                });
            }
        }
        assert_eq!(
            suggest_background(raster(w, h, &px)),
            Some(Rgb8::new(0x33, 0x66, 0xCC))
        );
        // A transparent border: no background colour.
        let clear: Vec<u8> = px.chunks(4).flat_map(|p| [p[0], p[1], p[2], 0]).collect();
        assert_eq!(suggest_background(raster(w, h, &clear)), None);
        assert_eq!(suggest_background(raster(3, 1, &px[..8])), None);
    }

    #[test]
    fn a_sample_is_the_median_of_the_opaque_window() {
        // 3 x 3: eight reds and one white at the centre's right; one transparent pixel.
        let mut px = Vec::new();
        for i in 0..9 {
            px.extend(match i {
                5 => [255, 255, 255, 255],
                8 => [0, 0, 0, 0],
                _ => [200, 10, 10, 255],
            });
        }
        let r = raster(3, 3, &px);
        assert_eq!(sample_colour(r, 1, 1), Some(Rgb8::new(200, 10, 10)));
        assert_eq!(sample_colour(r, 2, 2), None);
        assert_eq!(sample_colour(r, 3, 0), None);
    }

    #[test]
    fn the_gate_grows_with_the_square_of_the_tolerance() {
        assert_eq!(gate(0), 0.0);
        assert!((gate(100) - 127.5).abs() < 1e-12);
        assert!((gate(20) - 0.02 * 255.0).abs() < 1e-12);
        assert_eq!(gate(200), gate(100));
    }
}
