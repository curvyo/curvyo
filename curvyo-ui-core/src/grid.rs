//! Page grid geometry for one view (`specs/0090-grid` criteria 4 to 8 and 12 to 14, `adrs.md`
//! decisions 3 to 5).
//!
//! Pure arithmetic over a [`Page`](curvyo_document_core::Page) and a
//! [`ViewTransform`](curvyo_document_core::ViewTransform), no document read: the renderer draws
//! what [`grid_layout`] returns, and snapping targets exactly the lines it draws at full
//! opacity ([`snap_to_grid`]), so what you see is what you snap.

use crate::viewport::ScreenPx;

/// Lines whose screen spacing is at or below this are left out; finer lines fade in above it.
pub(crate) const FADE_START: ScreenPx = ScreenPx(4.0);
/// Lines whose screen spacing is at or above this are drawn at full opacity and pull snapping.
pub(crate) const FADE_FULL: ScreenPx = ScreenPx(12.0);
/// A logarithmic cell wider than this on screen gets the next log subdivision (criterion 8).
pub(crate) const LOG_REFINE: ScreenPx = ScreenPx(48.0);
/// The refinement of a logarithmic cell is at full opacity from this width on, so a zoom never
/// makes the new lines pop in (Proposal).
pub(crate) const LOG_REFINE_FULL: ScreenPx = ScreenPx(72.0);
/// Added to a level's logarithm before `floor`, so 500 % and 2500 % land on their level exactly.
pub(crate) const LEVEL_EPS: f64 = 1e-9;
/// Below this, in radians, a grid counts as axis aligned.
pub(crate) const ANGLE_EPS: f64 = 1e-12;
/// An opacity within this of 1 counts as full (a snap target).
pub(crate) const FULL_ALPHA_EPS: f32 = 1e-6;

/// The linear position of `x` between `start` and `full`, clamped to `0..=1`.
pub(crate) fn ramp(x: ScreenPx, start: ScreenPx, full: ScreenPx) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // An opacity in 0..=1 fits an f32.
    let t = ((x.px() - start.px()) / (full.px() - start.px())).clamp(0.0, 1.0) as f32;
    t
}

/// One grid line along one grid axis, in grid coordinates (mm from the grid origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AxisLine {
    /// The coordinate across the line.
    pub pos: f64,
    /// Opacity, `0..=1`.
    pub alpha: f32,
    /// `0` for a minor line, `1` for a major line.
    pub emphasis: f32,
}

mod frame;
mod layout;
mod level;
mod log;
mod snap;

pub(crate) use frame::GridFrame;

/// The lines of `grid` along one grid axis with grid coordinate in `lo..=hi`, at `scale` screen
/// pixels per millimetre. Both axes share the same set; lines at zero opacity are left out.
pub(crate) fn axis_lines(
    grid: &curvyo_document_core::PageGrid,
    scale: f64,
    lo: f64,
    hi: f64,
) -> Vec<AxisLine> {
    match grid.scale {
        curvyo_document_core::GridScale::Linear => level::linear_lines(grid, scale, lo, hi),
        curvyo_document_core::GridScale::Logarithmic => log::log_lines(grid, scale, lo, hi),
    }
}
pub use layout::{GridLayout, GridLine, grid_layout};
pub(crate) use snap::snap_to_grid_along;
pub use snap::{GridCandidate, GridFeature, GridSnapInput, HeldGrid, snap_to_grid};

#[cfg(test)]
mod snap_tests;
#[cfg(test)]
mod tests;
