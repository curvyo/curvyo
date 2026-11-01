//! The lines of a logarithmic grid along one axis (criterion 8).
//!
//! The grid is log paper with decades of length `base · subdivision` repeating from the grid
//! origin, inner lines at `log10(n)` of the decade. In adaptive mode every cell wider than 48 px
//! gets the next log subdivision.
//!
//! A line closer than 4 px to a neighbour is left out and fades in up to 12 px, so the number of
//! lines is bounded by the screen. Decades closer than 4 px thin out by powers of two, nesting
//! like linear levels.

use curvyo_document_core::{GridMode, PageGrid};

use super::{AxisLine, FADE_FULL, FADE_START, LOG_REFINE, LOG_REFINE_FULL, ramp};
use crate::viewport::ScreenPx;

/// The deepest adaptive refinement (tenths, hundredths, ...) inside one cell.
const MAX_REFINE_DEPTH: u32 = 8;
/// The most decades one axis may walk; the 4 px thinning keeps a screen far below it.
const MAX_DECADES: f64 = 100_000.0;

/// The lines of a logarithmic grid along one axis inside `lo..=hi` (grid coordinates, mm).
pub(super) fn log_lines(grid: &PageGrid, scale: f64, lo: f64, hi: f64) -> Vec<AxisLine> {
    let decade = grid.spacing.as_mm() * f64::from(grid.subdivision);
    let decade_px = decade * scale;
    if !(decade_px.is_finite() && decade_px > 0.0) {
        return Vec::new();
    }
    // Thin decades by powers of two until neighbours are at least 4 px apart.
    let mut stride: i64 = 1;
    #[allow(clippy::cast_precision_loss)] // Stride stays a small power of two.
    while (stride as f64) * decade_px <= FADE_START.px() && stride < (1 << 40) {
        stride *= 2;
    }
    let first = (lo / decade).floor();
    let last = (hi / decade).floor();
    if !first.is_finite() || !last.is_finite() || last - first > MAX_DECADES * 2.0 {
        return Vec::new();
    }
    #[allow(clippy::cast_possible_truncation)] // Finite and range-checked above.
    let (first, last) = (first as i64, last as i64);
    let mut out = Vec::new();
    let mut d = first.div_euclid(stride) * stride;
    while d <= last + 1 {
        #[allow(clippy::cast_precision_loss)] // Decade indices stay far below 2^53.
        let start = d as f64 * decade;
        if (lo..=hi).contains(&start) {
            out.push(AxisLine {
                pos: start,
                alpha: decade_alpha(d, decade_px),
                emphasis: 1.0,
            });
        }
        if stride == 1 && d <= last {
            inner_lines(grid.mode, start, decade, scale, (lo, hi), &mut out);
        }
        d += stride;
    }
    out
}

/// A decade line's opacity: the spacing of its own nesting level, as linear levels fade.
fn decade_alpha(index: i64, decade_px: f64) -> f32 {
    let mut spacing = decade_px;
    let mut rest = index;
    while rest != 0 && rest % 2 == 0 && spacing < FADE_FULL.px() {
        rest /= 2;
        spacing *= 2.0;
    }
    if rest == 0 {
        return 1.0;
    }
    ramp(ScreenPx(spacing), FADE_START, FADE_FULL)
}

/// The lines of one decade between its two decade lines, and their refinements.
fn inner_lines(
    mode: GridMode,
    start: f64,
    decade: f64,
    scale: f64,
    range: (f64, f64),
    out: &mut Vec<AxisLine>,
) {
    let at = |n: f64| start + n.log10() * decade;
    for n in 1..10_u32 {
        let (a, b) = (f64::from(n), f64::from(n + 1));
        if n > 1 {
            let gap = (at(a) - at(a - 1.0)).min(at(b) - at(a)) * scale;
            push(
                out,
                at(a),
                ramp(ScreenPx(gap), FADE_START, FADE_FULL),
                range,
            );
        }
        if mode == GridMode::Adaptive {
            refine(at, a, 1.0, scale, range, 1, out);
        }
    }
}

/// Splits the cell `[n, n + width]` (in log-paper units) into tenths when it is wider than
/// 48 px on screen, recursively.
fn refine(
    at: impl Fn(f64) -> f64 + Copy,
    n: f64,
    width: f64,
    scale: f64,
    range: (f64, f64),
    depth: u32,
    out: &mut Vec<AxisLine>,
) {
    let (left, right) = (at(n), at(n + width));
    let cell = ScreenPx((right - left) * scale);
    if cell <= LOG_REFINE || depth > MAX_REFINE_DEPTH || right < range.0 || left > range.1 {
        return;
    }
    let presence = ramp(cell, LOG_REFINE, LOG_REFINE_FULL);
    let step = width / 10.0;
    for k in 0..10_u32 {
        let a = n + f64::from(k) * step;
        if k > 0 {
            let gap = (at(a) - at(a - step)).min(at(a + step) - at(a)) * scale;
            push(
                out,
                at(a),
                ramp(ScreenPx(gap), FADE_START, FADE_FULL) * presence,
                range,
            );
        }
        refine(at, a, step, scale, range, depth + 1, out);
    }
}

fn push(out: &mut Vec<AxisLine>, pos: f64, alpha: f32, range: (f64, f64)) {
    if alpha > 0.0 && (range.0..=range.1).contains(&pos) {
        out.push(AxisLine {
            pos,
            alpha,
            emphasis: 0.0,
        });
    }
}
