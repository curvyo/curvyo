//! The detail steps built on the blur (ADR 0073 §4): Sharpen (an unsharp mask) and Contours (a
//! difference of two blurs).

use crate::adjust_blur::{BlurRun, SubPx};
use crate::adjust_plane::{GreyPlane, clamp_byte};
use crate::adjust_tone::div_round;

/// `plane` sharpened in place by an unsharp mask of `radius` (the blur's sigma): with
/// `d = g − blur(g)`, a grey stays where `100 · |d| < threshold_pct · 255`, else becomes
/// `clamp(g + d · amount_pct / 100)` rounded half away from zero.
#[cfg(test)]
pub(crate) fn sharpen(plane: &mut GreyPlane, radius: SubPx, amount_pct: u16, threshold_pct: u8) {
    let mut run = DetailRun::sharpen(plane, radius, amount_pct, threshold_pct);
    let mut all = u64::MAX;
    run.work(plane, &mut all);
}

/// `plane` turned into its dark lines in place: blurs at `sigma` and `1.6 · sigma` (rounded to
/// 1/16 px); where the pixel is darker than its surroundings (`d = wide − narrow > 0`) the grey
/// is `255 − clamp(d · (100 + 15 · sensitivity_pct) / 100)` (a gain of 1 to 16, so a pen line
/// is dark enough for a 50 % threshold), else white.
#[cfg(test)]
pub(crate) fn contours(plane: &mut GreyPlane, sigma: SubPx, sensitivity_pct: u8) {
    let mut run = DetailRun::contours(plane, sigma, sensitivity_pct);
    let mut all = u64::MAX;
    run.work(plane, &mut all);
}

/// How a detail step combines a pixel with its blurs.
#[derive(Debug, Clone, Copy)]
enum Combine {
    /// Sharpen: amount in percent, threshold times 255.
    Sharpen { amount: i64, limit: i64 },
    /// Contours: the gain in percent.
    Contours { gain: i64 },
}

/// Sharpen or Contours worked a band of rows at a time (`specs/0135` criterion 13): the blurs
/// first, then the rows combined; the bytes of [`sharpen`] and [`contours`].
#[derive(Debug, Clone)]
pub(crate) struct DetailRun {
    /// The blurs (Sharpen one, Contours the narrow then the wide one); empty for a step that
    /// changes nothing.
    blurs: Vec<BlurRun>,
    combine: Combine,
    row: usize,
}

impl DetailRun {
    /// The unsharp mask of `plane`, nothing worked yet.
    pub(crate) fn sharpen(
        plane: &GreyPlane,
        radius: SubPx,
        amount_pct: u16,
        threshold_pct: u8,
    ) -> Self {
        Self {
            blurs: if amount_pct == 0 {
                Vec::new()
            } else {
                vec![BlurRun::new(plane, radius)]
            },
            combine: Combine::Sharpen {
                amount: i64::from(amount_pct),
                limit: i64::from(threshold_pct) * 255,
            },
            row: 0,
        }
    }

    /// The contours of `plane`, nothing worked yet.
    pub(crate) fn contours(plane: &GreyPlane, sigma: SubPx, sensitivity_pct: u8) -> Self {
        Self {
            blurs: vec![
                BlurRun::new(plane, sigma),
                BlurRun::new(plane, SubPx((sigma.0 * 16 + 5) / 10)),
            ],
            combine: Combine::Contours {
                gain: 100 + 15 * i64::from(sensitivity_pct.min(100)),
            },
            row: 0,
        }
    }

    /// Works `plane` (the plane the run was made from) until `work` is spent, at least one row;
    /// whether the step is finished.
    pub(crate) fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        if self.blurs.is_empty() {
            return true;
        }
        for blur in &mut self.blurs {
            if !blur.work(work) {
                return false;
            }
        }
        let w = plane.width as usize;
        let h = plane.height as usize;
        while self.row < h {
            let span = self.row * w..(self.row + 1) * w;
            let line = &mut plane.grey[span.clone()];
            match self.combine {
                Combine::Sharpen { amount, limit } => {
                    let blurred = &self.blurs[0].plane().grey[span];
                    for (g, &b) in line.iter_mut().zip(blurred) {
                        let d = i64::from(*g) - i64::from(b);
                        if 100 * d.abs() < limit {
                            continue;
                        }
                        *g = clamp_byte(i64::from(*g) + div_round(d * amount, 100));
                    }
                }
                Combine::Contours { gain } => {
                    let narrow = &self.blurs[0].plane().grey[span.clone()];
                    let wide = &self.blurs[1].plane().grey[span];
                    for ((g, &n), &wd) in line.iter_mut().zip(narrow).zip(wide) {
                        let d = i64::from(wd) - i64::from(n);
                        *g = if d > 0 {
                            255 - clamp_byte(div_round(d * gain, 100))
                        } else {
                            255
                        };
                    }
                }
            }
            self.row += 1;
            *work = work.saturating_sub(w as u64);
            if *work == 0 && self.row < h {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge() -> GreyPlane {
        let mut p = GreyPlane::white(32, 4);
        for row in p.grey.as_chunks_mut::<32>().0 {
            row[..16].fill(64);
            row[16..].fill(192);
        }
        p
    }

    #[test]
    fn sharpen_raises_the_edge_and_keeps_flat_areas() {
        let mut p = edge();
        sharpen(&mut p, SubPx(32), 100, 0);
        let row = &p.grey[..32];
        assert_eq!(row[0], 64, "flat stays");
        assert!(row[15] < 64 && row[16] > 192, "{row:?}");
    }

    #[test]
    fn a_threshold_keeps_small_differences() {
        let mut p = edge();
        sharpen(&mut p, SubPx(32), 100, 50);
        assert_eq!(p, edge(), "every difference is below 50 %");
    }

    #[test]
    fn contours_draw_a_dark_line_and_white_elsewhere() {
        let mut p = GreyPlane::white(32, 32);
        for y in 0..32 {
            p.grey[y * 32 + 15] = 0;
            p.grey[y * 32 + 16] = 0;
        }
        contours(&mut p, SubPx(16), 50);
        let row = &p.grey[16 * 32..17 * 32];
        assert_eq!(row[0], 255);
        assert!(row[15] < 64, "{row:?}");
        assert!(row.iter().filter(|&&g| g < 255).count() < 12);
    }
}
