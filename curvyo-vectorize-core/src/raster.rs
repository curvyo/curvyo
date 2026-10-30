//! The picture as the tracer reads it (`specs/0025-raster-trace` Terms, criterion 9; `specs/0089`
//! criterion 15): the luma of the stored RGB of the straight-alpha RGBA8 pixels (alpha is
//! counted apart, never composited), and Otsu's automatic Threshold over the opaque pixels.

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
}

/// The luma of a pixel's stored RGB: `0.2126 R + 0.7152 G + 0.0722 B` in integers, rounded
/// (`0058` D10, no colour management); the alpha is not part of it (`specs/0089` criterion 15).
#[must_use]
pub(crate) fn luma(r: u8, g: u8, b: u8) -> u8 {
    let weighted = 2126 * u32::from(r) + 7152 * u32::from(g) + 722 * u32::from(b);
    // invariant: the weights sum to 10 000, so the quotient is at most 255.
    u8::try_from((weighted + 5000) / 10_000).unwrap_or(u8::MAX)
}

/// The automatic Threshold (criterion 9): Otsu's method on the luma histogram, so that the
/// pixels below the returned value are the dark class. A pixel with alpha under 128 is never
/// content (`specs/0089` criterion 15), so it counts in the top level, the class nothing below
/// any Threshold reaches, whatever its stored colour; the others count by the luma of their
/// stored RGB (nothing is composited). Where several cuts separate the classes equally well, the
/// middle one; a picture of one level (or an invalid raster) gives 128.
#[must_use]
pub fn auto_threshold(raster: Raster<'_>) -> u8 {
    if raster.checked_pixels().is_none() {
        return crate::schema::DEFAULT_THRESHOLD;
    }
    let mut histogram = [0_u64; 256];
    for &[r, g, b, a] in raster.rgba.as_chunks::<4>().0 {
        let level = if a >= crate::select::OPAQUE {
            luma(r, g, b)
        } else {
            u8::MAX
        };
        histogram[usize::from(level)] += 1;
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
    fn luma_follows_rec_709() {
        assert_eq!(luma(0, 0, 0), 0);
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(255, 0, 0), 54);
        assert_eq!(luma(0, 255, 0), 182);
        assert_eq!(luma(0, 0, 255), 18);
    }

    /// `specs/0089` criterion 15: a transparent pixel counts as nothing (the top level) whatever
    /// its stored colour, so the opaque dark and light parts are split apart from it.
    #[test]
    fn otsu_counts_transparent_pixels_as_nothing() {
        let mut rgba = Vec::new();
        for i in 0..100 {
            rgba.extend(match i % 4 {
                0 => [30, 30, 30, 255],
                1 => [220, 220, 220, 255],
                _ => [0, 0, 0, 0],
            });
        }
        let raster = Raster {
            width: 10,
            height: 10,
            rgba: &rgba,
        };
        let t = auto_threshold(raster);
        assert!((31..=220).contains(&t), "{t}");
        // Black ink on a transparent background whose hidden colour is black too: the ink is
        // the dark class.
        let ink: Vec<u8> = (0..100)
            .flat_map(|i| {
                if i % 5 == 0 {
                    [10, 10, 10, 255]
                } else {
                    [0, 0, 0, 0]
                }
            })
            .collect();
        let raster = Raster {
            width: 10,
            height: 10,
            rgba: &ink,
        };
        assert!(auto_threshold(raster) > 10);
        let clear = vec![0_u8; 400];
        let raster = Raster {
            width: 10,
            height: 10,
            rgba: &clear,
        };
        assert_eq!(auto_threshold(raster), 128);
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
