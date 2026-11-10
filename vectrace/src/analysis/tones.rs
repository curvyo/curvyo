//! The tone and colour properties the analysis reads from a picture's pixels.

use super::{real, real64};
use crate::background::suggest_background;
use crate::params::Rgb8;
use crate::raster::{Raster, auto_threshold, luma};
use crate::select::OPAQUE;

/// The longest side of the reduced copy the global measures run on, px.
pub(crate) const MAX_SIDE: usize = 1024;

/// A colour closer than this to the background (bytes, Euclidean) is background in the
/// measures (0.06 of the unit cube).
const NEAR_BACKGROUND: f64 = 15.3;

/// White.
const WHITE: Rgb8 = Rgb8::new(255, 255, 255);

/// Neighbours within this per channel (bytes) make a pixel flat (not an edge, not noise).
const FLAT_STEP: i32 = 12;

/// The side of the full-resolution crops the noise is read on, px, and how many per side.
const CROP: usize = 64;
const CROPS_PER_SIDE: usize = 4;

/// The picture reduced by a whole factor with area averaging, so its longest side is at most
/// [`MAX_SIDE`].
pub(crate) struct Reduced {
    pub width: usize,
    pub height: usize,
    /// Source pixels per reduced pixel along a side.
    pub factor: usize,
    pub rgba: Vec<u8>,
}

impl Reduced {
    /// The pixel at (`x`, `y`).
    pub(crate) fn at(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// The luma at (`x`, `y`), or `None` for a pixel that is not opaque.
    pub(crate) fn luma_at(&self, x: usize, y: usize) -> Option<u8> {
        let p = self.at(x, y);
        (p[3] >= OPAQUE).then(|| luma(p[0], p[1], p[2]))
    }
}

/// `raster` reduced by area averaging (each channel on its own, straight alpha averaged too).
pub(crate) fn reduce(raster: Raster<'_>) -> Reduced {
    let (width, height) = (raster.width as usize, raster.height as usize);
    let factor = width.max(height).div_ceil(MAX_SIDE).max(1);
    let (rw, rh) = (width.div_ceil(factor), height.div_ceil(factor));
    let mut rgba = Vec::with_capacity(rw * rh * 4);
    for y in 0..rh {
        let rows = y * factor..((y + 1) * factor).min(height);
        for x in 0..rw {
            let cols = x * factor..((x + 1) * factor).min(width);
            let mut sum = [0_u64; 4];
            for yy in rows.clone() {
                for xx in cols.clone() {
                    let i = (yy * width + xx) * 4;
                    for (k, s) in sum.iter_mut().enumerate() {
                        *s += u64::from(raster.rgba[i + k]);
                    }
                }
            }
            let n = (rows.len() * cols.len()) as u64;
            for s in sum {
                // invariant: an average of bytes is a byte.
                rgba.push(u8::try_from((s + n / 2) / n).unwrap_or(u8::MAX));
            }
        }
    }
    Reduced {
        width: rw,
        height: rh,
        factor,
        rgba,
    }
}

/// The tone and colour properties of a picture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tones {
    /// The share of pixels that are not opaque.
    pub transparent: f64,
    /// The background colour (Remove background's Auto), `None` when transparent.
    pub background: Option<Rgb8>,
    /// The share of the opaque border pixels near the background colour (0 when transparent).
    pub border_uniformity: f64,
    /// The share of the border's background pixels not exactly the background colour (paper
    /// grain, scanner dust; 0 for a digital background).
    pub grain: f64,
    /// The automatic Threshold (Otsu) and the lower cut of a three-class split.
    pub threshold: u8,
    pub lower_cut: u8,
    /// The share of all pixels Brightness makes content at the automatic Threshold.
    pub ink: f64,
    /// The share of all pixels that are flat, unlike the background and lighter than the
    /// Threshold: content Brightness misses.
    pub pale: f64,
    /// The distance from the background (bytes) that a tenth of those pixels are within.
    pub palest: f64,
    /// The mean luma residual of light flat areas on full-resolution crops (luma units).
    pub noise: f64,
    /// How many pixels the noise was read on.
    pub noise_samples: usize,
    /// The spread of the paper level across the picture (0 to 1).
    pub gradient: f64,
    /// The share of the not-paper pixels inside an area of mid tones (pencil, shading).
    pub midtones: f64,
    /// The colours of the flat content (coarse groups of at least 2 % of it).
    pub colours: usize,
    /// The share of the content with a visible chroma.
    pub chroma: f64,
}

/// The tone and colour properties of `raster` (full resolution) and its `reduced` copy.
pub(crate) fn tones(raster: Raster<'_>, reduced: &Reduced) -> Tones {
    let background = suggest_background(raster);
    let threshold = auto_threshold(raster);
    let total = real(reduced.width * reduced.height);
    let transparent = reduced
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] < OPAQUE)
        .count();
    let transparent = real(transparent) / total;
    let (pale_count, palest) = pale(reduced, threshold, background);
    let (colours, chroma) = colours(reduced, threshold, background);
    let (noise, noise_samples) = noise(raster, threshold);
    let (border_uniformity, grain) = border_uniformity(raster, background);
    Tones {
        transparent,
        background,
        border_uniformity,
        grain,
        threshold,
        lower_cut: lower_cut(reduced),
        ink: content_share(reduced, threshold),
        pale: pale_count / total,
        palest,
        noise,
        noise_samples,
        gradient: gradient(reduced, threshold),
        midtones: midtones(reduced, threshold),
        colours,
        chroma,
    }
}

fn distance(a: [u8; 4], b: Rgb8) -> f64 {
    let d = [
        f64::from(a[0]) - f64::from(b.r),
        f64::from(a[1]) - f64::from(b.g),
        f64::from(a[2]) - f64::from(b.b),
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The whole-byte distance of `p` from `background` (from white for a transparent one), the
/// bin of the pale histogram.
fn bin(p: [u8; 4], background: Option<Rgb8>) -> usize {
    let bg = background.unwrap_or(WHITE);
    let d = |a: u8, b: u8| usize::from(a.abs_diff(b));
    (d(p[0], bg.r).pow(2) + d(p[1], bg.g).pow(2) + d(p[2], bg.b).pow(2)).isqrt()
}

/// Whether the opaque pixel at (`x`, `y`) has four opaque neighbours within [`FLAT_STEP`].
fn flat(reduced: &Reduced, x: usize, y: usize) -> bool {
    if x == 0 || y == 0 || x + 1 >= reduced.width || y + 1 >= reduced.height {
        return false;
    }
    let p = reduced.at(x, y);
    [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
        .into_iter()
        .all(|(nx, ny)| {
            let q = reduced.at(nx, ny);
            q[3] >= OPAQUE && (0..3).all(|k| (i32::from(p[k]) - i32::from(q[k])).abs() <= FLAT_STEP)
        })
}

/// The share of the opaque border pixels within [`NEAR_BACKGROUND`] of `background`, and the
/// share of those that are not exactly it (paper grain).
fn border_uniformity(raster: Raster<'_>, background: Option<Rgb8>) -> (f64, f64) {
    let Some(bg) = background else {
        return (0.0, 0.0);
    };
    let (w, h) = (raster.width as usize, raster.height as usize);
    let pixel = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        [
            raster.rgba[i],
            raster.rgba[i + 1],
            raster.rgba[i + 2],
            raster.rgba[i + 3],
        ]
    };
    let border = (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]));
    let (mut opaque, mut near, mut grain) = (0_u64, 0_u64, 0_u64);
    for (x, y) in border {
        let p = pixel(x, y);
        if p[3] >= OPAQUE {
            opaque += 1;
            let d = distance(p, bg);
            if d <= NEAR_BACKGROUND {
                near += 1;
                if d > 0.0 {
                    grain += 1;
                }
            }
        }
    }
    (
        real64(near) / real64(opaque.max(1)),
        real64(grain) / real64(near.max(1)),
    )
}

/// The luma histogram of the opaque pixels of `reduced`.
fn histogram(reduced: &Reduced) -> [u64; 256] {
    let mut histogram = [0_u64; 256];
    for &[r, g, b, a] in reduced.rgba.as_chunks::<4>().0 {
        if a >= OPAQUE {
            histogram[usize::from(luma(r, g, b))] += 1;
        }
    }
    histogram
}

/// The lower cut of the three-class Otsu split of the opaque pixels' luma (dark class
/// `0..cut`): the cut between ink and a middle class such as a shadow; 0 for one level.
fn lower_cut(reduced: &Reduced) -> u8 {
    let histogram = histogram(reduced);
    let mut count = [0.0_f64; 257];
    let mut sum = [0.0_f64; 257];
    for (level, &n) in histogram.iter().enumerate() {
        count[level + 1] = count[level] + real64(n);
        sum[level + 1] = sum[level] + real(level) * real64(n);
    }
    let class = |from: usize, to: usize| {
        let n = count[to] - count[from];
        if n == 0.0 {
            0.0
        } else {
            let s = sum[to] - sum[from];
            s * s / n
        }
    };
    let (mut best, mut cut) = (-1.0_f64, 0_usize);
    for a in 1..255 {
        for b in a + 1..256 {
            let between = class(0, a) + class(a, b) + class(b, 256);
            if between > best {
                best = between;
                cut = a;
            }
        }
    }
    u8::try_from(cut).unwrap_or(0)
}

/// The share of all pixels that are opaque and darker than `threshold`.
fn content_share(reduced: &Reduced, threshold: u8) -> f64 {
    let pixels = reduced.rgba.as_chunks::<4>().0;
    let dark = pixels
        .iter()
        .filter(|&&[r, g, b, a]| a >= OPAQUE && luma(r, g, b) < threshold)
        .count();
    real(dark) / real(pixels.len())
}

/// How many flat opaque pixels are at or above `threshold` and unlike the background (with a
/// transparent background: unlike white), and the distance a tenth of them are within.
fn pale(reduced: &Reduced, threshold: u8, background: Option<Rgb8>) -> (f64, f64) {
    let mut distances = [0_u64; 443];
    let mut count = 0_u64;
    for y in 0..reduced.height {
        for x in 0..reduced.width {
            let p = reduced.at(x, y);
            if p[3] < OPAQUE || luma(p[0], p[1], p[2]) < threshold || !flat(reduced, x, y) {
                continue;
            }
            // On a transparent background white reads as a cut-out, not as content.
            let d = background.map_or_else(
                || {
                    if distance(p, WHITE) > NEAR_BACKGROUND {
                        442.0
                    } else {
                        0.0
                    }
                },
                |bg| distance(p, bg),
            );
            if d > NEAR_BACKGROUND {
                count += 1;
                distances[bin(p, background).min(442)] += 1;
            }
        }
    }
    let mut seen = 0_u64;
    let tenth = count.div_ceil(10);
    let palest = distances
        .iter()
        .position(|&n| {
            seen += n;
            seen >= tenth && n > 0
        })
        .unwrap_or(0);
    (real64(count), real(palest))
}

/// The colour groups of the flat content (3 bits per channel, a group at least 2 % of the flat
/// content) and the share of the content whose channels differ by 48 or more.
fn colours(reduced: &Reduced, threshold: u8, background: Option<Rgb8>) -> (usize, f64) {
    let mut bins = vec![0_u64; 512];
    let (mut flat_content, mut content, mut chromatic) = (0_u64, 0_u64, 0_u64);
    for y in 0..reduced.height {
        for x in 0..reduced.width {
            let p = reduced.at(x, y);
            if p[3] < OPAQUE {
                continue;
            }
            let dark = luma(p[0], p[1], p[2]) < threshold;
            let unlike = background.is_none_or(|bg| distance(p, bg) > NEAR_BACKGROUND);
            if !(dark || unlike) {
                continue;
            }
            content += 1;
            let (lo, hi) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
            if hi - lo >= 48 {
                chromatic += 1;
            }
            if flat(reduced, x, y) {
                flat_content += 1;
                bins[usize::from(p[0] >> 5) << 6
                    | usize::from(p[1] >> 5) << 3
                    | usize::from(p[2] >> 5)] += 1;
            }
        }
    }
    let groups = bins
        .iter()
        .filter(|&&n| n > 0 && n * 50 >= flat_content)
        .count();
    (groups, real64(chromatic) / real64(content.max(1)))
}

/// The mean luma residual (`|4 l − Σ four neighbours| / 4`) of the light pixels whose eight
/// neighbours are light too, on a grid of full-resolution crops, and how many were read.
fn noise(raster: Raster<'_>, threshold: u8) -> (f64, usize) {
    let (width, height) = (raster.width as usize, raster.height as usize);
    let level = |x: usize, y: usize| {
        let i = (y * width + x) * 4;
        let a = raster.rgba[i + 3];
        (a >= OPAQUE).then(|| luma(raster.rgba[i], raster.rgba[i + 1], raster.rgba[i + 2]))
    };
    let light = |x: usize, y: usize| level(x, y).filter(|&l| l >= threshold);
    let (mut sum, mut n) = (0_u64, 0_usize);
    for cy in 0..CROPS_PER_SIDE {
        for cx in 0..CROPS_PER_SIDE {
            let x0 = (width * (2 * cx + 1) / (2 * CROPS_PER_SIDE)).saturating_sub(CROP / 2);
            let y0 = (height * (2 * cy + 1) / (2 * CROPS_PER_SIDE)).saturating_sub(CROP / 2);
            for y in (y0 + 1).max(1)..(y0 + CROP).min(height.saturating_sub(1)) {
                for x in (x0 + 1).max(1)..(x0 + CROP).min(width.saturating_sub(1)) {
                    let mut v = [0_u8; 9];
                    let mut all = true;
                    for (k, slot) in v.iter_mut().enumerate() {
                        let Some(l) = light(x + k % 3 - 1, y + k / 3 - 1) else {
                            all = false;
                            break;
                        };
                        *slot = l;
                    }
                    if !all {
                        continue;
                    }
                    let centre = 4 * i32::from(v[4]);
                    let around =
                        i32::from(v[1]) + i32::from(v[3]) + i32::from(v[5]) + i32::from(v[7]);
                    sum += u64::from((centre - around).unsigned_abs());
                    n += 1;
                }
            }
        }
    }
    (real64(sum) / 4.0 / real(n.max(1)), n)
}

/// The spread of the paper level (each 4 x 4 block's 90th luma percentile of its opaque
/// pixels, where that is at or above `threshold`) across the picture, 0 to 1.
fn gradient(reduced: &Reduced, threshold: u8) -> f64 {
    const BLOCKS: usize = 4;
    let mut levels = Vec::new();
    for by in 0..BLOCKS {
        for bx in 0..BLOCKS {
            let mut histogram = [0_u32; 256];
            let mut n = 0_u32;
            for y in reduced.height * by / BLOCKS..reduced.height * (by + 1) / BLOCKS {
                for x in reduced.width * bx / BLOCKS..reduced.width * (bx + 1) / BLOCKS {
                    if let Some(l) = reduced.luma_at(x, y) {
                        histogram[usize::from(l)] += 1;
                        n += 1;
                    }
                }
            }
            if n == 0 {
                continue;
            }
            let mut seen = 0;
            let p90 = histogram
                .iter()
                .position(|&c| {
                    seen += c;
                    seen * 10 >= n * 9
                })
                .unwrap_or(255);
            if p90 >= usize::from(threshold) {
                levels.push(p90);
            }
        }
    }
    match (levels.iter().min(), levels.iter().max()) {
        (Some(lo), Some(hi)) => real(hi - lo) / 255.0,
        _ => 0.0,
    }
}

/// The share of the not-paper pixels that are mid tones with four mid-tone neighbours: mid
/// tones lie in the middle half between the ink level and the paper level (the medians below
/// and above `threshold`).
fn midtones(reduced: &Reduced, threshold: u8) -> f64 {
    let mut histogram = [0_u64; 256];
    for y in 0..reduced.height {
        for x in 0..reduced.width {
            if let Some(l) = reduced.luma_at(x, y) {
                histogram[usize::from(l)] += 1;
            }
        }
    }
    let t = usize::from(threshold);
    let median = |range: std::ops::Range<usize>| {
        let n: u64 = histogram[range.clone()].iter().sum();
        let mut seen = 0;
        range
            .clone()
            .find(|&l| {
                seen += histogram[l];
                seen * 2 >= n && n > 0
            })
            .unwrap_or(range.start)
    };
    let (ink, paper) = (median(0..t), median(t..256));
    if paper <= ink + 8 {
        return 0.0;
    }
    let quarter = (paper - ink) / 4;
    let (lo, hi) = (ink + quarter, paper - quarter);
    let mid = |x: usize, y: usize| {
        reduced
            .luma_at(x, y)
            .is_some_and(|l| (lo..hi).contains(&usize::from(l)))
    };
    let (mut inside, mut not_paper) = (0_u64, 0_u64);
    for y in 0..reduced.height {
        for x in 0..reduced.width {
            if reduced.luma_at(x, y).is_some_and(|l| usize::from(l) < hi) {
                not_paper += 1;
            }
            if x > 0
                && y > 0
                && x + 1 < reduced.width
                && y + 1 < reduced.height
                && mid(x, y)
                && mid(x - 1, y)
                && mid(x + 1, y)
                && mid(x, y - 1)
                && mid(x, y + 1)
            {
                inside += 1;
            }
        }
    }
    real64(inside) / real64(not_paper.max(1))
}
