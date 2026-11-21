//! The tone steps of the adjustments as 256-entry tables (ADR 0073 §4): Levels, Curves and the
//! grey of Channel mix.

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

/// The Curves table of `points` (Input %, Output %), Input strictly increasing from 0 to 100: a
/// monotone cubic through the points (Fritsch-Carlson, so no overshoot) in `f64 + - * /`, each
/// grey `g` read at Input `100 · g / 255` and rounded to the nearest grey. Fewer than two points
/// with increasing Input is the identity.
#[must_use]
#[allow(clippy::many_single_char_names)] // the segment's own names
pub fn curves_table(points: &[(u8, u8)]) -> [u8; 256] {
    let mut table = [0_u8; 256];
    for (g, slot) in (0_u8..=255).zip(table.iter_mut()) {
        *slot = g;
    }
    // A point whose Input does not pass the one before is dropped (a stored curve never has one;
    // no secant divides by zero).
    let mut kept: Vec<(u8, u8)> = Vec::with_capacity(points.len());
    for &p in points {
        if kept.last().is_none_or(|last| p.0 > last.0) {
            kept.push(p);
        }
    }
    if kept.len() < 2 {
        return table;
    }
    let xs: Vec<f64> = kept.iter().map(|p| f64::from(p.0)).collect();
    let ys: Vec<f64> = kept.iter().map(|p| f64::from(p.1)).collect();
    let tangents = monotone_tangents(&xs, &ys);
    let end_grey = |y: f64| -> i64 {
        // invariant: Output is 0 to 100 %, so the grey is 0 to 255.
        #[allow(clippy::cast_possible_truncation)]
        let grey = (255.0 * y / 100.0).round() as i64;
        grey
    };
    let mut previous: Option<(usize, i64)> = None;
    for (g, slot) in (0_u32..).zip(table.iter_mut()) {
        let x = 100.0 * f64::from(g) / 255.0;
        let y = hermite(&xs, &ys, &tangents, x);
        #[allow(clippy::cast_possible_truncation)]
        let mut grey = (255.0 * y / 100.0).round() as i64;
        // Each segment stays between its end greys and keeps its direction, so float noise
        // before the rounding never dents a flat or rising part (the table is monotone where
        // the points are).
        let k = (0..xs.len() - 1)
            .find(|&k| x <= xs[k + 1])
            .unwrap_or(xs.len() - 2);
        let (a, b) = (end_grey(ys[k]), end_grey(ys[k + 1]));
        grey = grey.clamp(a.min(b), a.max(b));
        if let Some((pk, pg)) = previous
            && pk == k
        {
            grey = if b >= a { grey.max(pg) } else { grey.min(pg) };
        }
        previous = Some((k, grey));
        *slot = clamp_byte(grey);
    }
    table
}

/// The Fritsch-Carlson tangents of the points.
#[allow(clippy::many_single_char_names)] // the method's own names
fn monotone_tangents(xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = xs.len();
    let secants: Vec<f64> = (0..n - 1)
        .map(|k| (ys[k + 1] - ys[k]) / (xs[k + 1] - xs[k]))
        .collect();
    let mut m = vec![0.0; n];
    m[0] = secants[0];
    m[n - 1] = secants[n - 2];
    for k in 1..n - 1 {
        m[k] = if secants[k - 1] * secants[k] <= 0.0 {
            0.0
        } else {
            f64::midpoint(secants[k - 1], secants[k])
        };
    }
    for k in 0..n - 1 {
        if secants[k] == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
            continue;
        }
        let a = m[k] / secants[k];
        let b = m[k + 1] / secants[k];
        let s = a * a + b * b;
        if s > 9.0 {
            let t = 3.0 / libm::sqrt(s);
            m[k] = t * a * secants[k];
            m[k + 1] = t * b * secants[k];
        }
    }
    m
}

/// The cubic Hermite curve through the points with tangents `m` at `x`, flat past the ends.
#[allow(clippy::many_single_char_names)] // the method's own names
fn hermite(xs: &[f64], ys: &[f64], m: &[f64], x: f64) -> f64 {
    let n = xs.len();
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    let k = (0..n - 1).find(|&k| x <= xs[k + 1]).unwrap_or(n - 2);
    let h = xs[k + 1] - xs[k];
    let t = (x - xs[k]) / h;
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * ys[k]
        + (t3 - 2.0 * t2 + t) * h * m[k]
        + (-2.0 * t3 + 3.0 * t2) * ys[k + 1]
        + (t3 - t2) * h * m[k + 1]
}

/// The grey of Channel mix: `clamp(round((r · R + g · G + b · B) / 100))` with the weights in
/// whole percent, rounded half away from zero.
#[must_use]
pub fn channel_mix_grey(r: u8, g: u8, b: u8, [wr, wg, wb]: [i16; 3]) -> u8 {
    let sum =
        i64::from(r) * i64::from(wr) + i64::from(g) * i64::from(wg) + i64::from(b) * i64::from(wb);
    clamp_byte(div_round(sum, 100))
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

    /// The Curves of `points`, nothing worked yet.
    pub(crate) fn curves(points: &[(u8, u8)]) -> Self {
        Self {
            table: curves_table(points),
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
    fn a_straight_curve_is_the_identity_and_an_s_curve_stays_monotone() {
        let t = curves_table(&[(0, 0), (100, 100)]);
        assert!((0..=255).all(|g| t[g] == u8::try_from(g).unwrap()));
        let s = curves_table(&[(0, 0), (25, 15), (75, 85), (100, 100)]);
        assert!(s.windows(2).all(|w| w[0] <= w[1]), "monotone");
        assert!(s[64] < 64 && s[191] > 191, "an S");
        assert_eq!((s[0], s[255]), (0, 255));
        // No overshoot past a flat part.
        let flat = curves_table(&[(0, 0), (40, 50), (60, 50), (100, 100)]);
        assert!(flat[115..=140].iter().all(|&g| g == 128 || g == 127));
    }

    /// Architect review A6: a curve whose points repeat an Input (not a stored curve, but a
    /// `PixelStep` can hold it) neither divides by zero nor breaks monotony: the repeat is dropped.
    #[test]
    fn a_repeated_input_is_dropped_not_divided_by() {
        let t = curves_table(&[(0, 0), (0, 50), (50, 50), (50, 80), (100, 100)]);
        assert_eq!(t, curves_table(&[(0, 0), (50, 50), (100, 100)]));
        let one = curves_table(&[(40, 10), (40, 90)]);
        assert!(
            (0..=255).all(|g| one[g] == u8::try_from(g).unwrap()),
            "fewer than two: identity"
        );
    }

    #[test]
    fn channel_mix_weighs_the_channels_and_clamps() {
        assert_eq!(channel_mix_grey(200, 10, 10, [100, 0, 0]), 200);
        assert_eq!(channel_mix_grey(200, 10, 10, [200, 0, 0]), 255);
        assert_eq!(channel_mix_grey(10, 200, 10, [100, -200, 0]), 0);
        assert_eq!(channel_mix_grey(100, 100, 100, [21, 72, 7]), 100);
    }

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
