//! The picture analysis the presets read (ADR 0073 §4, spec `0135` criterion 39): a histogram of
//! greys with its percentiles, median and Otsu level, and a noise estimate, all in integers.

use crate::adjust_plane::GreyPlane;
use crate::working::WorkingImage;

/// The largest mip the analysis reads, in pixels.
pub const ANALYSIS_PIXELS: u64 = 1_000_000;

/// A 256-bin histogram of greys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Histogram {
    /// The count per grey, 0 black to 255 white.
    pub bins: [u32; 256],
}

impl Histogram {
    /// The histogram of `plane`.
    #[must_use]
    pub fn of(plane: &GreyPlane) -> Self {
        let mut bins = [0_u32; 256];
        for &g in &plane.grey {
            bins[usize::from(g)] += 1;
        }
        Self { bins }
    }

    /// The number of pixels counted.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.bins.iter().map(|&b| u64::from(b)).sum()
    }

    /// The `p`-th percentile, `p` in tenths of a percent (0 to 1000): the smallest grey whose
    /// cumulative share is at least `p`; 255 for an empty histogram.
    #[must_use]
    pub fn percentile(&self, p_tenths: u16) -> u8 {
        let total = self.total();
        let need = u64::from(p_tenths.min(1000)) * total;
        let mut cumulative = 0_u64;
        for (g, &bin) in (0_u8..=255).zip(&self.bins) {
            cumulative += u64::from(bin);
            if total > 0 && cumulative * 1000 >= need {
                return g;
            }
        }
        255
    }

    /// The 50 % percentile.
    #[must_use]
    pub fn median(&self) -> u8 {
        self.percentile(500)
    }

    /// The Otsu level: the grey `t` (class one `<= t`) that maximises the between-class variance
    /// `(μ_total · w₀ − μ₀)² / (w₀ · w₁)`, compared by cross-multiplication in `u128`, ties to
    /// the lower grey.
    #[must_use]
    pub fn otsu(&self) -> u8 {
        let total = u128::from(self.total());
        let sum: u128 = (0_u128..)
            .zip(&self.bins)
            .map(|(g, &b)| g * u128::from(b))
            .sum();
        let (mut w0, mut sum0) = (0_u128, 0_u128);
        let mut best: Option<(u8, u128, u128)> = None;
        for (g, &bin) in (0_u8..=255).zip(&self.bins) {
            w0 += u128::from(bin);
            sum0 += u128::from(g) * u128::from(bin);
            let w1 = total - w0;
            if w0 == 0 || w1 == 0 {
                continue;
            }
            // σ_b² · total² = (sum · w₀ − total · sum₀)² / (w₀ · w₁): numerator and denominator.
            let diff = (sum * w0).abs_diff(total * sum0);
            let (num, den) = (diff * diff, w0 * w1);
            let better = best.is_none_or(|(_, bn, bd)| wide_mul(num, bd) > wide_mul(bn, den));
            if better {
                best = Some((g, num, den));
            }
        }
        best.map_or_else(|| self.median(), |(g, _, _)| g)
    }
}

/// `a · b` as a 256-bit number `(high, low)`, so products of the Otsu comparison never wrap.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const HALF: u32 = 64;
    let mask = u128::from(u64::MAX);
    let (a1, a0) = (a >> HALF, a & mask);
    let (b1, b0) = (b >> HALF, b & mask);
    let (low, cross1, cross2, high) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let (mid, carry) = cross1.overflowing_add(cross2);
    let (low, carry_low) = low.overflowing_add(mid << HALF);
    let high = high + (mid >> HALF) + (u128::from(carry) << HALF) + u128::from(carry_low);
    (high, low)
}

/// What the presets read from a picture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Analysis {
    /// The histogram of the analysis mip.
    pub histogram: Histogram,
    /// The noise estimate in tenths of a grey level.
    pub noise_tenths: u32,
}

impl Analysis {
    /// The analysis of `image` (alpha over white, the luminance grey) at its box mip of at most
    /// [`ANALYSIS_PIXELS`].
    #[must_use]
    pub fn of(image: &WorkingImage) -> Self {
        Self::of_plane(&GreyPlane::of_rgba(image))
    }

    /// The analysis of a grey plane at its box mip of at most [`ANALYSIS_PIXELS`].
    #[must_use]
    pub fn of_plane(plane: &GreyPlane) -> Self {
        let mip = plane.mip_at_most(ANALYSIS_PIXELS);
        Self {
            histogram: Histogram::of(&mip),
            noise_tenths: noise_tenths(&mip),
        }
    }
}

/// The noise estimate `1.4826 · MAD / 6` in tenths of a grey: `(14826 · m + 3000) / 6000` with
/// `m` the median absolute response of Immerkær's kernel `[1 −2 1; −2 4 −2; 1 −2 1]` over the
/// inner pixels; 0 below 3 x 3.
#[must_use]
pub fn noise_tenths(plane: &GreyPlane) -> u32 {
    let (w, h) = (plane.width as usize, plane.height as usize);
    if w < 3 || h < 3 {
        return 0;
    }
    // Responses reach 16 · 255; count them in a histogram for the median.
    let mut counts = vec![0_u32; 16 * 255 + 1];
    let g = |x: usize, y: usize| i32::from(plane.grey[y * w + x]);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let r = g(x - 1, y - 1) + g(x + 1, y - 1) + g(x - 1, y + 1) + g(x + 1, y + 1)
                - 2 * (g(x, y - 1) + g(x - 1, y) + g(x + 1, y) + g(x, y + 1))
                + 4 * g(x, y);
            counts[r.unsigned_abs() as usize] += 1;
        }
    }
    let total: u64 = counts.iter().map(|&c| u64::from(c)).sum();
    let mut cumulative = 0_u64;
    let mut m = 0_u32;
    for (value, &count) in (0_u32..).zip(&counts) {
        cumulative += u64::from(count);
        if 2 * cumulative >= total {
            m = value;
            break;
        }
    }
    (14_826 * m + 3_000) / 6_000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(width: u32, height: u32, mut f: impl FnMut(u32, u32) -> u8) -> GreyPlane {
        let mut grey = Vec::new();
        for y in 0..height {
            for x in 0..width {
                grey.push(f(x, y));
            }
        }
        GreyPlane {
            width,
            height,
            grey,
        }
    }

    /// A small deterministic generator for the noise test.
    fn noise(seed: &mut u32) -> i32 {
        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        i32::try_from(*seed >> 24).unwrap() % 17 - 8
    }

    #[test]
    fn a_noise_free_ramp_is_quiet_and_a_noisy_one_is_not() {
        let ramp = plane(256, 64, |x, _| u8::try_from(x).unwrap());
        assert!(noise_tenths(&ramp) < 10, "{}", noise_tenths(&ramp));
        let mut seed = 7;
        let noisy = plane(256, 64, |x, _| {
            u8::try_from((i32::try_from(x).unwrap() + noise(&mut seed)).clamp(0, 255)).unwrap()
        });
        assert!(noise_tenths(&noisy) > 30, "{}", noise_tenths(&noisy));
    }

    #[test]
    fn otsu_lies_between_two_greys() {
        let two = plane(10, 10, |x, _| if x < 3 { 40 } else { 200 });
        let t = Histogram::of(&two).otsu();
        assert!((40..200).contains(&t), "{t}");
        assert_eq!(t, 40, "ties go to the lower grey");
    }

    #[test]
    fn percentiles_follow_the_cumulative_share() {
        let ramp = plane(100, 1, |x, _| u8::try_from(x).unwrap());
        let h = Histogram::of(&ramp);
        assert_eq!(h.percentile(0), 0);
        assert_eq!(h.percentile(1000), 99);
        assert_eq!(h.median(), 49, "50 of 100 at grey 49");
        assert_eq!(h.percentile(20), 1, "2 %");
    }
}
