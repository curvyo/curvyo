//! Samples the raster along any affine grid, averaged over the source pixels each output pixel
//! covers.
//!
//! Basic `f32` operations in a fixed order (no fused multiply-add, no threads), so natively and
//! in wasm the same grid gives the same values.

use crate::OcrError;

/// The most sub-samples per output pixel and axis.
const MAX_SUBSAMPLES: usize = 4;

/// The raster as opaque RGB, composited on white.
#[derive(Debug, Clone)]
pub(crate) struct Rgb {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// Row by row, three bytes per pixel.
    pixels: Vec<u8>,
}

impl Rgb {
    /// `rgba` (`width` x `height`, straight alpha) on white.
    pub(crate) fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self, OcrError> {
        let (w, h) = (width as usize, height as usize);
        if w == 0 || h == 0 || w.checked_mul(h).and_then(|n| n.checked_mul(4)) != Some(rgba.len()) {
            return Err(OcrError::Invalid(format!(
                "{width} x {height} with {} bytes",
                rgba.len()
            )));
        }
        let mut pixels = Vec::with_capacity(w * h * 3);
        for px in rgba.as_chunks::<4>().0 {
            let a = u32::from(px[3]);
            for &c in &px[..3] {
                // invariant: at most (255 * 255 + 255 * 255 + 127) / 255 = 255.
                #[allow(clippy::cast_possible_truncation)]
                let on_white = ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
                pixels.push(on_white);
            }
        }
        Ok(Self {
            width: w,
            height: h,
            pixels,
        })
    }

    /// The bilinear colour at `(x, y)` in pixel coordinates (pixel centres at `.5`), the edge
    /// pixels repeated outside.
    fn bilinear(&self, x: f32, y: f32) -> [f32; 3] {
        let fx = (x - 0.5).clamp(0.0, (self.width - 1) as f32);
        let fy = (y - 0.5).clamp(0.0, (self.height - 1) as f32);
        // invariant: clamped into the raster first.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let at =
            |x: usize, y: usize, c: usize| f32::from(self.pixels[(y * self.width + x) * 3 + c]);
        let mut out = [0.0; 3];
        for (c, value) in out.iter_mut().enumerate() {
            let top = at(x0, y0, c) * (1.0 - tx) + at(x1, y0, c) * tx;
            let bottom = at(x0, y1, c) * (1.0 - tx) + at(x1, y1, c) * tx;
            *value = top * (1.0 - ty) + bottom * ty;
        }
        out
    }
}

/// An affine grid over the raster: output pixel `(i, j)` covers the source parallelogram from
/// `origin + i * step_x + j * step_y` spanned by `step_x` and `step_y`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Grid {
    pub(crate) origin: [f32; 2],
    pub(crate) step_x: [f32; 2],
    pub(crate) step_y: [f32; 2],
    pub(crate) width: usize,
    pub(crate) height: usize,
}

fn length(v: [f32; 2]) -> f32 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

/// How many sub-samples per axis an output step of `step` source pixels needs.
fn subsamples(step: [f32; 2]) -> usize {
    // invariant: clamped to 1 to MAX_SUBSAMPLES.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = length(step).ceil().clamp(1.0, MAX_SUBSAMPLES as f32) as usize;
    n
}

/// The RGB values (0 to 255) of every output pixel of `grid`, row by row, each the mean of a
/// regular set of bilinear samples over the parallelogram it covers.
pub(crate) fn sample(image: &Rgb, grid: &Grid) -> Vec<[f32; 3]> {
    let (nx, ny) = (subsamples(grid.step_x), subsamples(grid.step_y));
    let count = (nx * ny) as f32;
    let mut out = Vec::with_capacity(grid.width * grid.height);
    for j in 0..grid.height {
        for i in 0..grid.width {
            let mut sum = [0.0_f32; 3];
            for sy in 0..ny {
                let fy = j as f32 + (sy as f32 + 0.5) / ny as f32;
                for sx in 0..nx {
                    let fx = i as f32 + (sx as f32 + 0.5) / nx as f32;
                    let x = grid.origin[0] + grid.step_x[0] * fx + grid.step_y[0] * fy;
                    let y = grid.origin[1] + grid.step_x[1] * fx + grid.step_y[1] * fy;
                    let v = image.bilinear(x, y);
                    sum[0] += v[0];
                    sum[1] += v[1];
                    sum[2] += v[2];
                }
            }
            out.push([sum[0] / count, sum[1] / count, sum[2] / count]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_pixels_are_white_and_the_size_must_match() {
        let image = Rgb::from_rgba(1, 1, &[0, 0, 0, 0]).unwrap();
        assert_eq!(image.pixels, [255, 255, 255]);
        assert!(Rgb::from_rgba(2, 1, &[0; 4]).is_err());
        assert!(Rgb::from_rgba(0, 0, &[]).is_err());
    }

    #[test]
    fn halving_averages_the_covered_pixels() {
        let rgba = [0, 0, 0, 255, 255, 255, 255, 255];
        let image = Rgb::from_rgba(2, 1, &rgba).unwrap();
        let grid = Grid {
            origin: [0.0, 0.0],
            step_x: [2.0, 0.0],
            step_y: [0.0, 1.0],
            width: 1,
            height: 1,
        };
        let v = sample(&image, &grid);
        assert!((v[0][0] - 127.5).abs() < 1e-3);
    }
}
