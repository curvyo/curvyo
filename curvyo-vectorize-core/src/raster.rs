//! The picture as the tracer reads it (`specs/0025-raster-trace` Terms, criteria 9 and 24): luma
//! of the straight-alpha RGBA8 pixels composited over white, and Otsu's automatic
//! Threshold.

/// A straight-alpha RGBA8 picture, row-major, top row first (ADR 0017 §6).
#[derive(Debug, Clone, Copy)]
pub struct Raster<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: &'a [u8],
}

impl Raster<'_> {
    /// The number of pixels, or `None` when the bytes do not match the size or a side is 0.
    pub(crate) fn checked_pixels(&self) -> Option<usize> {
        let pixels = usize::try_from(u64::from(self.width) * u64::from(self.height)).ok()?;
        (pixels > 0 && pixels.checked_mul(4) == Some(self.rgba.len())).then_some(pixels)
    }

    /// The luma of every pixel, in order.
    pub(crate) fn lumas(&self) -> impl Iterator<Item = u8> + '_ {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| luma(r, g, b, a))
    }
}

/// One channel composited over white: `c·a + 255·(1 − a)`, rounded.
fn over_white(c: u8, a: u8) -> u32 {
    let (c, a) = (u32::from(c), u32::from(a));
    (c * a + 255 * (255 - a) + 127) / 255
}

/// The luma of one straight-alpha pixel over white: `0.2126 R + 0.7152 G + 0.0722 B` of the
/// stored sRGB values, in integers, rounded (`0058` D10, no colour management).
#[must_use]
pub(crate) fn luma(r: u8, g: u8, b: u8, a: u8) -> u8 {
    let weighted = 2126 * over_white(r, a) + 7152 * over_white(g, a) + 722 * over_white(b, a);
    // invariant: the weights sum to 10 000, so the quotient is at most 255.
    u8::try_from((weighted + 5000) / 10_000).unwrap_or(u8::MAX)
}

/// The automatic Threshold (criterion 9): Otsu's method on the luma histogram, so that the
/// pixels below the returned value are the dark class. Where several cuts separate the classes
/// equally well, the middle one; a picture of one grey level (or an invalid raster) gives 128.
#[must_use]
pub fn auto_threshold(raster: Raster<'_>) -> u8 {
    if raster.checked_pixels().is_none() {
        return crate::schema::DEFAULT_THRESHOLD;
    }
    let mut histogram = [0_u64; 256];
    for value in raster.lumas() {
        histogram[usize::from(value)] += 1;
    }
    otsu(&histogram)
}

/// Otsu's cut of `histogram`: the Threshold `t` (dark class `0..t`) that maximises the
/// between-class variance; the middle of a plateau of equal maxima; 128 for one level.
fn otsu(histogram: &[u64; 256]) -> u8 {
    let total: u64 = histogram.iter().sum();
    #[allow(clippy::cast_precision_loss)] // at most 16 Mpx, exact in f64
    let total_f = total as f64;
    #[allow(clippy::cast_precision_loss)]
    let sum_all: f64 = histogram
        .iter()
        .enumerate()
        .map(|(level, &count)| level as f64 * count as f64)
        .sum();
    let (mut weight_dark, mut sum_dark) = (0.0_f64, 0.0_f64);
    let mut best = 0.0_f64;
    let (mut first, mut last) = (None, None);
    // The dark class is the levels `0..=k`; the Threshold is `k + 1`.
    for (k, &count) in histogram.iter().enumerate().take(255) {
        #[allow(clippy::cast_precision_loss)]
        {
            weight_dark += count as f64;
            sum_dark += k as f64 * count as f64;
        }
        let weight_light = total_f - weight_dark;
        if weight_dark == 0.0 || weight_light == 0.0 {
            continue;
        }
        let mean_dark = sum_dark / weight_dark;
        let mean_light = (sum_all - sum_dark) / weight_light;
        let between = weight_dark * weight_light * (mean_dark - mean_light).powi(2);
        // Equal within the rounding of the sums: one plateau.
        let tolerance = best * 1e-12;
        if between > best + tolerance {
            best = between;
            first = Some(k);
            last = Some(k);
        } else if (between - best).abs() <= tolerance && first.is_some() {
            last = Some(k);
        }
    }
    match (first, last) {
        (Some(first), Some(last)) => {
            u8::try_from(usize::midpoint(first, last) + 1).unwrap_or(u8::MAX)
        }
        _ => crate::schema::DEFAULT_THRESHOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(levels: &[u8]) -> Vec<u8> {
        levels.iter().flat_map(|&v| [v, v, v, 255]).collect()
    }

    #[test]
    fn luma_follows_rec_709_over_white() {
        assert_eq!(luma(0, 0, 0, 255), 0);
        assert_eq!(luma(255, 255, 255, 255), 255);
        assert_eq!(luma(255, 0, 0, 255), 54);
        assert_eq!(luma(0, 255, 0, 255), 182);
        assert_eq!(luma(0, 0, 255, 255), 18);
        // Criterion 24: a transparent pixel is white whatever its colour.
        assert_eq!(luma(0, 0, 0, 0), 255);
        assert_eq!(luma(0, 0, 0, 128), 127);
    }

    /// Criterion 9's test: left half luma 40, right half 200 gives a value between 41 and 199.
    #[test]
    fn otsu_splits_two_levels_between_them() {
        let mut levels = Vec::new();
        for _y in 0..100 {
            levels.extend(std::iter::repeat_n(40, 50));
            levels.extend(std::iter::repeat_n(200, 50));
        }
        let rgba = grey(&levels);
        let t = auto_threshold(Raster {
            width: 100,
            height: 100,
            rgba: &rgba,
        });
        assert!((41..=199).contains(&t), "{t}");
    }

    #[test]
    fn one_grey_level_gives_128() {
        let rgba = grey(&[77; 16]);
        let raster = Raster {
            width: 4,
            height: 4,
            rgba: &rgba,
        };
        assert_eq!(auto_threshold(raster), 128);
        let broken = Raster {
            width: 5,
            height: 4,
            rgba: &rgba,
        };
        assert_eq!(auto_threshold(broken), 128);
    }
}
