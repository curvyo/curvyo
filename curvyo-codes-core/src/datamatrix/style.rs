//! Draws a Data Matrix grid: data modules in the module style, the L finder, the clock edge and the
//! alignment patterns always as solid squares (spec 0144 criterion 12).

use curvyo_extension_core::ClosedPath;

use crate::grid::ModuleGrid;
use crate::shapes::{Frame, reversed, rounded_rect};
use crate::style::{ModuleShape, Style, module};
use crate::trace::outlines;

fn frame(grid: &ModuleGrid, style: &Style) -> Frame {
    #[allow(clippy::cast_precision_loss)]
    Frame {
        half: grid.width() as f64 / 2.0,
        module: style.module.as_mm(),
    }
}

/// The outlines of `grid` in `style`, around the centre of the symbol, clockwise on the screen.
pub(crate) fn style(grid: &ModuleGrid, style: &Style) -> Vec<ClosedPath> {
    let width = grid.width();
    let frame = frame(grid, style);
    let solid = |x: usize, y: usize| grid.is_function(x, y) && grid.is_dark(x, y);
    // An inverted code cuts the dark modules out of a rectangle.
    let data = |x: usize, y: usize| !grid.is_function(x, y) && grid.is_dark(x, y);
    let merged = style.merged
        && matches!(style.shape, ModuleShape::Square | ModuleShape::Rounded)
        && style.fill >= 1.0 - 1e-9;
    let rounding = if style.shape == ModuleShape::Rounded {
        style.rounding.clamp(0.0, 0.5)
    } else {
        0.0
    };
    let mut paths = Vec::new();
    if merged && rounding < 1e-9 {
        paths.extend(outlines(
            frame,
            width,
            |x, y| grid.is_dark(x, y),
            0.0,
            false,
        ));
    } else {
        if merged {
            paths.extend(outlines(frame, width, data, rounding, false));
        } else {
            for (x, y) in cells(width).filter(|&(x, y)| data(x, y)) {
                paths.push(module(frame, style, (x, y), false));
            }
        }
        paths.extend(outlines(frame, width, solid, 0.0, false));
    }
    if !style.inverted {
        return paths;
    }
    #[allow(clippy::cast_precision_loss)]
    let (low, high) = (
        -f64::from(style.quiet),
        width as f64 + f64::from(style.quiet),
    );
    let mut inverted = vec![rounded_rect(frame, (low, low), (high, high), 0.0)];
    inverted.extend(paths.iter().map(reversed));
    inverted
}

fn cells(width: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..width).flat_map(move |y| (0..width).map(move |x| (x, y)))
}

/// The ghost under the pointer (`Edit::Draft`): the square of the symbol and its L finder.
pub(crate) fn draft(grid: &ModuleGrid, style: &Style) -> Vec<ClosedPath> {
    let width = grid.width();
    let frame = frame(grid, style);
    #[allow(clippy::cast_precision_loss)]
    let side = width as f64;
    let mut paths = vec![rounded_rect(frame, (0.0, 0.0), (side, side), 0.0)];
    paths.extend(outlines(
        frame,
        width,
        |x, y| x == 0 || y + 1 == width,
        0.0,
        false,
    ));
    paths
}
