//! Denoise as a 5 x 5 bilateral filter with integer weight tables (ADR 0073 §4).

use crate::adjust_plane::{GreyPlane, clamp_byte};
use crate::adjust_tone::div_round;

/// The spatial weights `round(256 · exp(−(dx² + dy²) / (2 · 1.5²)))` for `dx, dy` in −2 to 2.
const SPATIAL: [[u32; 5]; 5] = [
    [43, 84, 105, 84, 43],
    [84, 164, 205, 164, 84],
    [105, 205, 256, 205, 105],
    [84, 164, 205, 164, 84],
    [43, 84, 105, 84, 43],
];

/// The range weights `round(256 · exp(−Δ² / (2 σr²)))` for Δ 0 to 255, with
/// `σr = 4 + (100 − detail) · 0.6` greys (`libm::exp`).
#[must_use]
fn range_table(detail_pct: u8) -> [u32; 256] {
    let sigma_tenths = 40 + 6 * (100 - i64::from(detail_pct.min(100)));
    let mut table = [0_u32; 256];
    for (delta, slot) in (0_i64..).zip(table.iter_mut()) {
        // In tenths: Δ² / (2 σr²) = 100 Δ² / (2 σ10²); exact in f64 from integers.
        #[allow(clippy::cast_precision_loss)]
        let exponent = (100 * delta * delta) as f64 / (2 * sigma_tenths * sigma_tenths) as f64;
        let weight = (256.0 * libm::exp(-exponent)).round();
        // invariant: `256 · exp(−x)` for `x >= 0` lies in 0..=256.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let weight = weight.clamp(0.0, 256.0) as u32;
        *slot = weight;
    }
    table
}

/// `plane` denoised in place: the bilateral mean `f` of each 5 x 5 neighbourhood (edges
/// clamped, weights spatial · range, rounded half up), then `g + (f − g) · strength / 100`
/// rounded half away from zero. Strength 0 changes nothing.
#[cfg(test)]
pub(crate) fn denoise(plane: &mut GreyPlane, strength_pct: u8, detail_pct: u8) {
    let mut run = DenoiseRun::new(plane, strength_pct, detail_pct);
    let mut all = u64::MAX;
    run.work(plane, &mut all);
}

/// The work units of one denoised pixel: 25 weighted taps against one of a box pass.
const PIXEL_WORK: u64 = 24;

/// Denoise worked a band of rows at a time (`specs/0135` criterion 13): the bytes of
/// [`denoise`].
#[derive(Debug, Clone)]
pub(crate) struct DenoiseRun {
    /// The input with two clamped pixels on every side; empty for Strength 0.
    padded: Vec<u8>,
    range: [u32; 256],
    strength: i64,
    row: usize,
}

impl DenoiseRun {
    /// The denoise of `plane`, nothing worked yet.
    pub(crate) fn new(plane: &GreyPlane, strength_pct: u8, detail_pct: u8) -> Self {
        let (w, h) = (plane.width as usize, plane.height as usize);
        Self {
            padded: if strength_pct == 0 {
                Vec::new()
            } else {
                padded(plane, w, h)
            },
            range: range_table(detail_pct),
            strength: i64::from(strength_pct.min(100)),
            row: 0,
        }
    }

    /// Works `plane` (the plane the run was made from) until `work` is spent, at least one row;
    /// whether the step is finished.
    pub(crate) fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        if self.padded.is_empty() {
            return true;
        }
        let (width, height) = (plane.width as usize, plane.height as usize);
        let pw = width + 4;
        while self.row < height {
            let y = self.row;
            for x in 0..width {
                let centre = self.padded[(y + 2) * pw + x + 2];
                let (mut sum, mut weights) = (0_u32, 0_u32);
                for (dy, spatial) in SPATIAL.iter().enumerate() {
                    let row = &self.padded[(y + dy) * pw + x..(y + dy) * pw + x + 5];
                    for (&s, &g) in spatial.iter().zip(row) {
                        let weight = s * self.range[usize::from(g.abs_diff(centre))];
                        sum += weight * u32::from(g);
                        weights += weight;
                    }
                }
                let f = i64::from((sum + weights / 2) / weights.max(1));
                let g = i64::from(centre);
                plane.grey[y * width + x] = clamp_byte(g + div_round((f - g) * self.strength, 100));
            }
            self.row += 1;
            *work = work.saturating_sub(PIXEL_WORK * width as u64);
            if *work == 0 && self.row < height {
                return false;
            }
        }
        true
    }
}

/// The plane with two clamped pixels on every side.
fn padded(plane: &GreyPlane, w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity((w + 4) * (h + 4));
    for y in 0..h + 4 {
        let sy = y.saturating_sub(2).min(h - 1);
        let row = &plane.grey[sy * w..(sy + 1) * w];
        out.extend([row[0]; 2]);
        out.extend_from_slice(row);
        out.extend([row[w - 1]; 2]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_table_falls_from_256() {
        let t = range_table(60);
        assert_eq!(t[0], 256);
        assert!(t.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(t[255], 0);
    }

    #[test]
    fn speckle_is_smoothed_and_a_strong_edge_kept() {
        let mut p = GreyPlane::white(16, 16);
        for (i, g) in p.grey.iter_mut().enumerate() {
            *g = if i % 16 < 8 { 40 } else { 220 };
            if i % 7 == 0 {
                *g = g.saturating_add(6);
            }
        }
        let before = p.clone();
        denoise(&mut p, 100, 60);
        let spread = |q: &GreyPlane| {
            let left: Vec<u8> = q.grey.chunks(16).map(|r| r[2]).collect();
            left.iter().max().unwrap() - left.iter().min().unwrap()
        };
        assert!(spread(&p) < spread(&before));
        assert!(
            p.grey[16 * 8 + 7] < 60 && p.grey[16 * 8 + 8] > 200,
            "edge kept"
        );
        let mut none = before.clone();
        denoise(&mut none, 0, 60);
        assert_eq!(none, before);
    }
}
