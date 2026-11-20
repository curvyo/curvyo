//! The Gaussian blur of the adjustments as three integer box passes (ADR 0073 §4, "boxes for
//! Gauss"): the same cost at every radius and the same bytes on every target.

use crate::adjust_plane::{GreyPlane, byte};

/// A length in 1/16 pixel: the pixel unit of the adjustment radii (ADR 0073 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SubPx(pub u32);

impl SubPx {
    /// Sixteenths per pixel.
    pub const PER_PX: u32 = 16;

    /// The length in whole pixels, rounded half up.
    #[must_use]
    pub const fn round_px(self) -> u32 {
        (self.0 + Self::PER_PX / 2) / Self::PER_PX
    }
}

/// The widths (odd, at least 1) of the three box passes whose sum of variances is closest to
/// `sigma²`: the lower odd width `wl` below the ideal `sqrt(4 sigma² + 1)` for `m` passes, the
/// next odd width for the others, all in integers.
#[must_use]
pub fn box_widths(sigma: SubPx) -> [u32; 3] {
    let s = u64::from(sigma.0);
    // sqrt(4 sigma² + 1) in pixels = sqrt(4 s² + 256) / 16.
    let ideal16 = (4 * s * s + 256).isqrt();
    let mut wl = ideal16 / 16;
    if wl % 2 == 0 {
        wl = wl.saturating_sub(1);
    }
    let wl = wl.max(1);
    let wu = wl + 2;
    // m = round((12 sigma² - 3 wl² - 12 wl - 9) / (-4 wl - 4)), scaled by 256.
    let num = 256 * (3 * wl * wl + 12 * wl + 9);
    let num = i64::try_from(num).unwrap_or(i64::MAX) - 12 * i64::try_from(s * s).unwrap_or(0);
    let den = i64::try_from(256 * (4 * wl + 4)).unwrap_or(1);
    let m = (2 * num + den).div_euclid(2 * den).clamp(0, 3);
    let width = |i: i64| u32::try_from(if i < m { wl } else { wu }).unwrap_or(1);
    [width(0), width(1), width(2)]
}

/// `plane` blurred with a Gaussian of `sigma`: per box width a horizontal then a vertical pass,
/// edges clamped, running sums in `u32`, each pass rounded half up. A sigma under half a pixel
/// gives three passes of width 1, the plane itself.
#[must_use]
pub fn gauss_blur(plane: &GreyPlane, sigma: SubPx) -> GreyPlane {
    let mut run = BlurRun::new(plane, sigma);
    let mut all = u64::MAX;
    run.work(&mut all);
    run.into_plane()
}

/// A Gaussian blur worked a band of rows at a time (`specs/0135` criterion 13, ADR 0073 §8): the
/// bytes of [`gauss_blur`], resumable between the host's ticks.
#[derive(Debug, Clone)]
pub(crate) struct BlurRun {
    out: GreyPlane,
    /// The radius of each box pass wider than one pixel, in order.
    radii: Vec<usize>,
    /// The pass being worked, its direction and the next row.
    pass: usize,
    vertical: bool,
    row: usize,
    /// The vertical pass's input and its running column sums, made at its first row.
    column: Option<(Vec<u8>, Vec<u32>)>,
    scratch: Vec<u8>,
}

impl BlurRun {
    /// The blur of `plane` (copied) with `sigma`, nothing worked yet.
    pub(crate) fn new(plane: &GreyPlane, sigma: SubPx) -> Self {
        Self {
            out: plane.clone(),
            radii: box_widths(sigma)
                .into_iter()
                .filter(|&w| w > 1)
                .map(|w| (w / 2) as usize)
                .collect(),
            pass: 0,
            vertical: false,
            row: 0,
            column: None,
            scratch: Vec::new(),
        }
    }

    /// Works rows until `work` (one unit per pixel visited) is spent, at least one row; whether
    /// the blur is finished.
    pub(crate) fn work(&mut self, work: &mut u64) -> bool {
        let (w, h) = (self.out.width as usize, self.out.height as usize);
        loop {
            let Some(&r) = self.radii.get(self.pass) else {
                return true;
            };
            if self.vertical {
                if self.column.is_none() {
                    *work = work.saturating_sub(((2 * r + 2) * w) as u64);
                    self.column = Some(vertical_start(&self.out.grey, w, h, r));
                }
                if let Some((source, sums)) = self.column.as_mut() {
                    let line = &mut self.out.grey[self.row * w..(self.row + 1) * w];
                    vertical_row(line, source, sums, (w, h, r), self.row);
                }
            } else {
                let line = &mut self.out.grey[self.row * w..(self.row + 1) * w];
                horizontal_row(line, &mut self.scratch, r);
            }
            *work = work.saturating_sub(w as u64);
            self.row += 1;
            if self.row == h {
                self.row = 0;
                if self.vertical {
                    self.column = None;
                    self.pass += 1;
                }
                self.vertical = !self.vertical;
            }
            if self.pass == self.radii.len() {
                return true;
            }
            if *work == 0 {
                return false;
            }
        }
    }

    /// The blurred plane (the input where the blur is not finished).
    pub(crate) fn into_plane(self) -> GreyPlane {
        self.out
    }

    /// The blurred plane so far.
    pub(crate) const fn plane(&self) -> &GreyPlane {
        &self.out
    }
}

/// One row of a horizontal box pass of radius `r`, in place, edges clamped.
fn horizontal_row(line: &mut [u8], row: &mut Vec<u8>, r: usize) {
    let w = line.len();
    let width = u32::try_from(2 * r + 1).unwrap_or(u32::MAX);
    row.clear();
    row.extend_from_slice(line);
    let at = |i: isize| -> u32 {
        let i = usize::try_from(i.max(0)).unwrap_or(0).min(w - 1);
        u32::from(row[i])
    };
    let r_i = isize::try_from(r).unwrap_or(isize::MAX);
    let mut sum: u32 = (-r_i..=r_i).map(at).sum();
    for (x, slot) in line.iter_mut().enumerate() {
        *slot = byte((sum + width / 2) / width);
        let x = isize::try_from(x).unwrap_or(0);
        sum = sum + at(x + r_i + 1) - at(x - r_i);
    }
}

/// Row `y` of `source` (`w` wide, `h` high), clamped to its edge.
fn source_row(source: &[u8], (w, h): (usize, usize), y: isize) -> &[u8] {
    let y = usize::try_from(y.max(0)).unwrap_or(0).min(h - 1);
    &source[y * w..(y + 1) * w]
}

/// The input of a vertical box pass of radius `r` and its column sums for the first row.
fn vertical_start(grey: &[u8], w: usize, h: usize, r: usize) -> (Vec<u8>, Vec<u32>) {
    let source = grey.to_vec();
    let r_i = isize::try_from(r).unwrap_or(isize::MAX);
    let mut sums = vec![0_u32; w];
    for y in -r_i..=r_i {
        for (s, &g) in sums.iter_mut().zip(source_row(&source, (w, h), y)) {
            *s += u32::from(g);
        }
    }
    (source, sums)
}

/// Output row `y` of a vertical box pass from the running column sums, then one row in and one
/// out of the sums.
fn vertical_row(
    line: &mut [u8],
    source: &[u8],
    sums: &mut [u32],
    (w, h, r): (usize, usize, usize),
    y: usize,
) {
    let width = u32::try_from(2 * r + 1).unwrap_or(u32::MAX);
    for (slot, &s) in line.iter_mut().zip(sums.iter()) {
        *slot = byte((s + width / 2) / width);
    }
    let r_i = isize::try_from(r).unwrap_or(isize::MAX);
    let y = isize::try_from(y).unwrap_or(0);
    let enter = source_row(source, (w, h), y + r_i + 1);
    let leave = source_row(source, (w, h), y - r_i);
    for ((s, &a), &b) in sums.iter_mut().zip(enter).zip(leave) {
        *s = *s + u32::from(a) - u32::from(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_widths_follow_sigma() {
        assert_eq!(box_widths(SubPx(0)), [1, 1, 1]);
        assert_eq!(box_widths(SubPx(16)), [1, 1, 3], "sigma 1 px");
        assert_eq!(box_widths(SubPx(48)), [5, 5, 7], "sigma 3 px");
        let [a, b, c] = box_widths(SubPx(16 * 40));
        let variance = |w: u32| f64::from(w * w - 1) / 12.0;
        let sigma = (variance(a) + variance(b) + variance(c)).sqrt();
        assert!((sigma - 40.0).abs() < 1.0, "{sigma}");
    }

    #[test]
    fn a_flat_plane_stays_flat_and_a_step_is_smoothed() {
        let flat = GreyPlane {
            width: 9,
            height: 4,
            grey: vec![77; 36],
        };
        assert_eq!(gauss_blur(&flat, SubPx(40)), flat);
        let mut step = GreyPlane::white(16, 2);
        for row in step.grey.as_chunks_mut::<16>().0 {
            row[..8].fill(0);
        }
        let b = gauss_blur(&step, SubPx(32));
        let row = &b.grey[..16];
        assert_eq!(row[0], 0);
        assert_eq!(row[15], 255);
        assert!(row.windows(2).all(|w| w[0] <= w[1]), "{row:?}");
        assert!((100..=155).contains(&row[8]) || (100..=155).contains(&row[7]));
    }
}
