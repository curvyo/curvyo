//! The reduced copy of a working image for the Trace tool's preview (`specs/0025-raster-trace`
//! criterion 10, ADR 0023 §5): the longest side brought down to a given length, area-averaged
//! with alpha weighting.

use crate::working::WorkingImage;

/// The longest side of the Trace preview's reduced copy, in pixels.
pub const TRACE_PREVIEW_SIDE: u32 = 1000;

/// For each output index along an axis of `from` pixels reduced to `to`: the source indices it
/// covers with their overlap, which sum to `from / to`.
fn weights(from: u32, to: u32) -> Vec<Vec<(usize, f64)>> {
    let ratio = f64::from(from) / f64::from(to);
    (0..to)
        .map(|i| {
            let (start, end) = (f64::from(i) * ratio, f64::from(i + 1) * ratio);
            // invariant: `start` and `end` lie in `0..=from`, a `u32`.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (first, last) = (
                start.floor() as usize,
                (end.ceil() as usize).min(from as usize),
            );
            (first..last)
                .filter_map(|s| {
                    #[allow(clippy::cast_precision_loss)] // at most 8192
                    let (s0, s1) = (s as f64, s as f64 + 1.0);
                    let overlap = s1.min(end) - s0.max(start);
                    (overlap > 0.0).then_some((s, overlap))
                })
                .collect()
        })
        .collect()
}

impl WorkingImage {
    /// A copy whose longest side is at most `longest` pixels (the other side in proportion, at
    /// least 1): every new pixel is the area-weighted mean of the pixels it covers, colour
    /// weighted by alpha so transparent pixels lend no colour. A picture that is small enough
    /// comes back unchanged. Equal input gives equal bytes.
    #[must_use]
    pub fn reduced_copy(&self, longest: u32) -> Self {
        let long = self.width.max(self.height);
        if long <= longest || longest == 0 {
            return self.clone();
        }
        let scale = f64::from(longest) / f64::from(long);
        // invariant: the product is at most `longest`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let side = |s: u32| ((f64::from(s) * scale).round() as u32).clamp(1, longest);
        let (width, height) = (side(self.width), side(self.height));
        let (columns, rows) = (weights(self.width, width), weights(self.height, height));
        let column_weights: Vec<f64> = columns
            .iter()
            .map(|c| c.iter().map(|&(_, w)| w).sum())
            .collect();
        let stride = self.width as usize * 4;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        // One row of premultiplied sums at a time: memory stays at the copy's size.
        let mut row_sums = vec![[0.0_f64; 4]; width as usize];
        for row in &rows {
            row_sums.fill([0.0; 4]);
            for &(sy, wy) in row {
                let source = &self.pixels[sy * stride..(sy + 1) * stride];
                for (sum, column) in row_sums.iter_mut().zip(&columns) {
                    for &(sx, wx) in column {
                        let p = &source[sx * 4..sx * 4 + 4];
                        let weight = wx * wy;
                        let alpha = f64::from(p[3]) * weight;
                        sum[0] += f64::from(p[0]) * alpha;
                        sum[1] += f64::from(p[1]) * alpha;
                        sum[2] += f64::from(p[2]) * alpha;
                        sum[3] += alpha;
                    }
                }
            }
            let row_weight: f64 = row.iter().map(|&(_, w)| w).sum();
            for (sum, &column_weight) in row_sums.iter().zip(&column_weights) {
                let alpha = sum[3] / (column_weight * row_weight);
                let colour = |c: f64| if sum[3] > 0.0 { c / sum[3] } else { 0.0 };
                // invariant: every value is a mean of bytes, so it rounds into 0..=255.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let byte = |v: f64| v.round().clamp(0.0, 255.0) as u8;
                pixels.extend([
                    byte(colour(sum[0])),
                    byte(colour(sum[1])),
                    byte(colour(sum[2])),
                    byte(alpha),
                ]);
            }
        }
        Self {
            width,
            height,
            pixels,
            original_size: self.original_size,
            truncated: self.truncated,
            animated: self.animated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, pixels: Vec<u8>) -> WorkingImage {
        WorkingImage {
            width,
            height,
            pixels,
            original_size: (width, height),
            truncated: false,
            animated: false,
        }
    }

    #[test]
    fn the_longest_side_becomes_the_given_length() {
        let big = image(3000, 1500, vec![255; 3000 * 1500 * 4]);
        let small = big.reduced_copy(TRACE_PREVIEW_SIDE);
        assert_eq!((small.width, small.height), (1000, 500));
        assert!(small.pixels.iter().all(|&b| b == 255));
        let tall = image(10, 2500, vec![0; 10 * 2500 * 4]);
        let small = tall.reduced_copy(TRACE_PREVIEW_SIDE);
        assert_eq!((small.width, small.height), (4, 1000));
    }

    #[test]
    fn it_averages_by_area_and_keeps_small_pictures() {
        // 3 x 1: black, white, black reduced to 2 x 1: each output covers 1.5 source pixels.
        let picture = image(3, 1, vec![0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255]);
        let copy = picture.reduced_copy(2);
        assert_eq!((copy.width, copy.height), (2, 1));
        assert_eq!(&copy.pixels[..4], &[85, 85, 85, 255]);
        assert_eq!(picture.reduced_copy(3), picture);
    }

    #[test]
    fn transparent_pixels_lend_no_colour() {
        let picture = image(2, 1, vec![255, 0, 0, 0, 0, 0, 0, 255]);
        let copy = picture.reduced_copy(1);
        assert_eq!(copy.pixels, [0, 0, 0, 128]);
    }
}
