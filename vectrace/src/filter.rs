//! The grey-level steps before the Threshold (`specs/0025-raster-trace` criteria 41 and 42,
//! `adrs.md` note 2026-10-28 points 3 and 4): Denoise as a median of the integer luma and Upscale as
//! a 2x integer bilinear copy, both in whole numbers so every build gives the same bytes.

use crate::TraceError;
use crate::progress::Poller;

/// The median of the `(2 radius + 1)²` window around every pixel of `luma` (`width` x `height`),
/// the picture's edge repeated outward; a sliding histogram per row (Huang), `poll` every 64
/// rows.
pub(crate) fn median(
    luma: &[u8],
    width: usize,
    height: usize,
    radius: usize,
    poller: &mut Poller<'_>,
) -> Result<Vec<u8>, TraceError> {
    if radius == 0 {
        return Ok(luma.to_vec());
    }
    let side = 2 * radius + 1;
    // The median is the value with `half` values below it and at least one equal.
    let half = side * side / 2;
    let mut out = vec![0_u8; luma.len()];
    for y in 0..height {
        poller.row(y, height)?;
        let rows: Vec<usize> = (0..side)
            .map(|k| (y + k).saturating_sub(radius).min(height - 1) * width)
            .collect();
        let mut histogram = [0_usize; 256];
        for k in 0..side {
            let x = k.saturating_sub(radius).min(width - 1);
            for &row in &rows {
                histogram[usize::from(luma[row + x])] += 1;
            }
        }
        let (mut value, mut below) = (0_usize, 0_usize);
        while below + histogram[value] <= half {
            below += histogram[value];
            value += 1;
        }
        for x in 0..width {
            // invariant: `value` indexes a 256-entry histogram of bytes.
            out[y * width + x] = u8::try_from(value).unwrap_or(u8::MAX);
            if x + 1 == width {
                break;
            }
            let (gone, new) = (x.saturating_sub(radius), (x + radius + 1).min(width - 1));
            for &row in &rows {
                let old = usize::from(luma[row + gone]);
                histogram[old] -= 1;
                if old < value {
                    below -= 1;
                }
                let add = usize::from(luma[row + new]);
                histogram[add] += 1;
                if add < value {
                    below += 1;
                }
            }
            while below > half {
                value -= 1;
                below -= histogram[value];
            }
            while below + histogram[value] <= half {
                below += histogram[value];
                value += 1;
            }
        }
    }
    Ok(out)
}

/// `luma` (`width` x `height`) at twice the size: each new pixel is the bilinear blend of the four
/// nearest old pixel centres with weights 9, 3, 3, 1 sixteenths, rounded; the edge repeated.
pub(crate) fn upscale_2x(
    luma: &[u8],
    width: usize,
    height: usize,
    poller: &mut Poller<'_>,
) -> Result<Vec<u8>, TraceError> {
    let (w2, h2) = (width * 2, height * 2);
    let mut out = vec![0_u8; w2 * h2];
    // The nearest old index and its neighbour on the side of the new pixel's centre.
    let pair = |i: usize, n: usize| {
        let near = i / 2;
        let other = if i.is_multiple_of(2) {
            near.saturating_sub(1)
        } else {
            (near + 1).min(n - 1)
        };
        (near, other)
    };
    for y in 0..h2 {
        poller.row(y, h2)?;
        let (ny, oy) = pair(y, height);
        for x in 0..w2 {
            let (nx, ox) = pair(x, width);
            let at = |px: usize, py: usize| u32::from(luma[py * width + px]);
            let sum = 9 * at(nx, ny) + 3 * at(ox, ny) + 3 * at(nx, oy) + at(ox, oy);
            // invariant: the weights sum to 16, so the quotient is at most 255.
            out[y * w2 + x] = u8::try_from((sum + 8) / 16).unwrap_or(u8::MAX);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
mod tests {
    use std::ops::ControlFlow;

    use super::*;

    fn go() -> impl FnMut(crate::Progress) -> ControlFlow<()> {
        |_| ControlFlow::Continue(())
    }

    /// The median by sorting each window, for comparison.
    fn slow_median(luma: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
        let r = radius as isize;
        let mut out = Vec::new();
        for y in 0..height as isize {
            for x in 0..width as isize {
                let mut window = Vec::new();
                for dy in -r..=r {
                    for dx in -r..=r {
                        let px = (x + dx).clamp(0, width as isize - 1) as usize;
                        let py = (y + dy).clamp(0, height as isize - 1) as usize;
                        window.push(luma[py * width + px]);
                    }
                }
                window.sort_unstable();
                out.push(window[window.len() / 2]);
            }
        }
        out
    }

    #[test]
    fn the_median_equals_the_sorted_window() {
        let (width, height) = (37, 23);
        let mut seed = 12_345_u32;
        let luma: Vec<u8> = (0..width * height)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (seed >> 16) as u8
            })
            .collect();
        for radius in [1, 2] {
            let mut poll = go();
            let fast = median(&luma, width, height, radius, &mut Poller::new(&mut poll)).unwrap();
            assert_eq!(
                fast,
                slow_median(&luma, width, height, radius),
                "radius {radius}"
            );
        }
    }

    #[test]
    fn a_lone_noise_pixel_is_removed_and_an_edge_kept() {
        // Left half black, right half white, one white pixel in the black.
        let (width, height) = (10, 10);
        let mut luma: Vec<u8> = (0..100).map(|i| if i % 10 < 5 { 0 } else { 255 }).collect();
        luma[4 * 10 + 2] = 255;
        let mut poll = go();
        let out = median(&luma, width, height, 1, &mut Poller::new(&mut poll)).unwrap();
        assert_eq!(out[4 * 10 + 2], 0);
        for y in 0..height {
            assert_eq!(out[y * 10 + 4], 0);
            assert_eq!(out[y * 10 + 5], 255);
        }
    }

    #[test]
    fn upscale_blends_the_nearest_centres() {
        let luma = [0_u8, 160, 160, 0];
        let mut poll = go();
        let out = upscale_2x(&luma, 2, 2, &mut Poller::new(&mut poll)).unwrap();
        assert_eq!(out.len(), 16);
        // The corner repeats the edge: all four nearest centres are the old corner.
        assert_eq!(out[0], 0);
        // 9/16 of the nearer old pixel, 3/16 of each neighbour, 1/16 of the diagonal.
        assert_eq!(out[1], 40);
        assert_eq!(out[2], 120);
        // A flat picture stays flat.
        let flat = upscale_2x(&[77; 6], 3, 2, &mut Poller::new(&mut poll)).unwrap();
        assert!(flat.iter().all(|&v| v == 77));
    }
}
