//! The lines of a linear grid along one axis: adaptive levels with fade, fixed thinning.
//!
//! With `sp(j)` the screen spacing of level `j` (`base · subdivision^-j · scale`), a line is
//! emitted once, at its coarsest level `L`, with opacity `ramp(sp(L))` and emphasis
//! `ramp(sp(L + 1))` (`adrs.md` decision 4). Levels nest, so a coarser line keeps its document
//! coordinate at every zoom (criterion 5).

use curvyo_document_core::{GridMode, PageGrid};

use super::{AxisLine, FADE_FULL, FADE_START, LEVEL_EPS, ramp};
use crate::viewport::ScreenPx;

/// The most lines one axis may hold. The 4 px rule keeps a 4K canvas far below it; the cap only
/// guards a caller that passes a range far larger than the screen.
const MAX_LINES: f64 = 200_000.0;
/// The deepest valuation that can still change a line's look (coarser levels are saturated).
const MAX_DEPTH: u32 = 64;

/// The finest level `j` whose screen spacing is at least `threshold`.
pub(super) fn finest_level(grid: &PageGrid, scale: f64, threshold: ScreenPx) -> Option<i32> {
    let base_px = grid.spacing.as_mm() * scale;
    let level =
        ((base_px / threshold.px()).ln() / f64::from(grid.subdivision).ln() + LEVEL_EPS).floor();
    #[allow(clippy::cast_possible_truncation)] // Range-checked just below.
    (level.is_finite() && level.abs() < 1000.0).then_some(level as i32)
}

/// The spacing of level `j` in millimetres.
fn spacing_mm(grid: &PageGrid, level: i32) -> f64 {
    grid.spacing.as_mm() * f64::from(grid.subdivision).powi(-level)
}

/// How many times `subdivision` divides `index`, at most `cap`; zero counts as `cap`.
fn valuation(index: i64, subdivision: i64, cap: u32) -> u32 {
    let mut rest = index;
    let mut depth = 0;
    while depth < cap && (rest == 0 || rest % subdivision == 0) {
        if rest == 0 {
            return cap;
        }
        rest /= subdivision;
        depth += 1;
    }
    depth
}

/// The integer line indices `ceil(lo / step) ..= floor(hi / step)`, or `None` when the range is
/// not finite or too long.
#[allow(clippy::cast_possible_truncation)] // Bounded by MAX_LINES and finite checks.
fn index_range(lo: f64, hi: f64, step: f64) -> Option<(i64, i64)> {
    let first = (lo / step).ceil();
    let last = (hi / step).floor();
    if !first.is_finite() || !last.is_finite() || step <= 0.0 || last - first > MAX_LINES {
        return None;
    }
    Some((first as i64, last as i64))
}

/// The lines of a linear grid along one axis inside `lo..=hi` (grid coordinates, mm).
pub(super) fn linear_lines(grid: &PageGrid, scale: f64, lo: f64, hi: f64) -> Vec<AxisLine> {
    match grid.mode {
        GridMode::Adaptive => adaptive(grid, scale, lo, hi),
        GridMode::Fixed => fixed(grid, scale, lo, hi),
    }
}

fn adaptive(grid: &PageGrid, scale: f64, lo: f64, hi: f64) -> Vec<AxisLine> {
    let Some(finest) = finest_level(grid, scale, FADE_START) else {
        return Vec::new();
    };
    let px = |level: i32| ScreenPx(spacing_mm(grid, level) * scale);
    let mut depth = 0;
    while depth < MAX_DEPTH && px(finest - i32::try_from(depth).unwrap_or(0) + 1) < FADE_FULL {
        depth += 1;
    }
    let step = spacing_mm(grid, finest);
    let Some((first, last)) = index_range(lo, hi, step) else {
        return Vec::new();
    };
    let subdivision = i64::from(grid.subdivision);
    // A line's look depends only on its valuation: one (alpha, emphasis) per depth, not per line.
    let looks: Vec<(f32, f32)> = (0..=depth)
        .map(|m| {
            let level = finest - i32::try_from(m).unwrap_or(0);
            (
                ramp(px(level), FADE_START, FADE_FULL),
                ramp(px(level + 1), FADE_START, FADE_FULL),
            )
        })
        .collect();
    (first..=last)
        .filter_map(|index| {
            let (alpha, emphasis) = looks[valuation(index, subdivision, depth) as usize];
            #[allow(clippy::cast_precision_loss)] // Indices stay far below 2^53.
            let pos = index as f64 * step;
            (alpha > 0.0).then_some(AxisLine {
                pos,
                alpha,
                emphasis,
            })
        })
        .collect()
}

fn fixed(grid: &PageGrid, scale: f64, lo: f64, hi: f64) -> Vec<AxisLine> {
    let base = grid.spacing.as_mm();
    let subdivision = i64::from(grid.subdivision);
    let minor_px = ScreenPx(base * scale);
    let major_px = ScreenPx(base * f64::from(grid.subdivision) * scale);
    let (step, minors) = if minor_px > FADE_START {
        (base, true)
    } else if major_px > FADE_START {
        (base * f64::from(grid.subdivision), false)
    } else {
        return Vec::new();
    };
    let Some((first, last)) = index_range(lo, hi, step) else {
        return Vec::new();
    };
    let major_alpha = ramp(major_px, FADE_START, FADE_FULL);
    let minor_alpha = ramp(minor_px, FADE_START, FADE_FULL);
    (first..=last)
        .map(|index| {
            #[allow(clippy::cast_precision_loss)] // Indices stay far below 2^53.
            let pos = index as f64 * step;
            if !minors || index % subdivision == 0 {
                AxisLine {
                    pos,
                    alpha: major_alpha,
                    emphasis: 1.0,
                }
            } else {
                AxisLine {
                    pos,
                    alpha: minor_alpha,
                    emphasis: 0.0,
                }
            }
        })
        .collect()
}
