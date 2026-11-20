//! The tone steps of the adjustments as 256-entry tables (ADR 0073 §4): Levels in slice 1.

use crate::adjust_plane::{GreyPlane, clamp_byte};

/// The Levels table: grey `B = round(255 · black / 100)` and below becomes 0, `W` and above 255,
/// the greys between stretched linearly and rounded half away from zero, then
/// `round(255 · (x / 255)^(1 / γ))` with `libm::pow` (the `0123` convention: above 1 lightens).
#[must_use]
pub fn levels_table(black_pct: u8, white_pct: u8, gamma_centi: u16) -> [u8; 256] {
    let point = |pct: u8| (255 * i64::from(pct.min(100)) + 50) / 100;
    let (b, w) = (point(black_pct), point(white_pct));
    let span = (w - b).max(1);
    let mut table = [0_u8; 256];
    for (g, slot) in (0_i64..).zip(table.iter_mut()) {
        let x = clamp_byte(div_round((g - b) * 255, span));
        *slot = gamma_of(x, gamma_centi.max(1));
    }
    table
}

/// A tone table applied a band of rows at a time (`specs/0135` criterion 13).
#[derive(Debug, Clone)]
pub(crate) struct TableRun {
    table: [u8; 256],
    row: usize,
}

impl TableRun {
    /// The Levels of `black_pct`, `white_pct` and `gamma_centi`, nothing worked yet.
    pub(crate) fn levels(black_pct: u8, white_pct: u8, gamma_centi: u16) -> Self {
        Self {
            table: levels_table(black_pct, white_pct, gamma_centi),
            row: 0,
        }
    }

    /// Works `plane` until `work` is spent, at least one row; whether the step is finished.
    pub(crate) fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        let w = plane.width as usize;
        let h = plane.height as usize;
        while self.row < h {
            for g in &mut plane.grey[self.row * w..(self.row + 1) * w] {
                *g = self.table[usize::from(*g)];
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

/// `round(255 · (g / 255)^(1 / γ))`, the ends kept.
fn gamma_of(g: u8, gamma_centi: u16) -> u8 {
    if g == 0 || g == 255 || gamma_centi == 100 {
        return g;
    }
    let gamma = f64::from(gamma_centi) / 100.0;
    let value = 255.0 * libm::pow(f64::from(g) / 255.0, 1.0 / gamma);
    // invariant: `0 < g / 255 < 1` and `γ > 0`, so the power lies in (0, 1).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = value.round().clamp(0.0, 255.0) as u8;
    value
}

/// `n / d` rounded half away from zero, `d > 0`.
#[must_use]
pub(crate) const fn div_round(n: i64, d: i64) -> i64 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_neutral_levels_are_the_identity() {
        let t = levels_table(0, 100, 100);
        assert!((0..=255).all(|g| t[g] == u8::try_from(g).unwrap()));
    }

    #[test]
    fn black_and_white_points_stretch_the_range() {
        let t = levels_table(20, 80, 100);
        assert_eq!(t[51], 0, "B = 51");
        assert_eq!(t[50], 0);
        assert_eq!(t[204], 255, "W = 204");
        assert_eq!(t[128], 128, "(128 - 51) x 255 / 153 = 128.3");
    }

    #[test]
    fn gamma_follows_the_0123_convention() {
        let t = levels_table(0, 100, 200);
        assert_eq!(t[128], 181, "above 1 lightens");
        let t = levels_table(0, 100, 50);
        assert_eq!(t[128], 64);
    }
}
