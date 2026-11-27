//! Turns a module grid into closed outlines in millimetres: Merged (traced outlines) or Separate
//! (one shape per module), the finders by their own style, the clear area knocked out and the
//! whole thing optionally inverted (ADR 0091 §4). Separate from [`crate::encode`] on purpose
//! (spec criterion 1).

use curvyo_document_core::Length;
use curvyo_extension_core::ClosedPath;

use crate::finder::{FinderShape, finder, origins};
use crate::grid::ModuleGrid;
use crate::shapes::{Frame, circle, diamond, reversed, rounded_rect};
use crate::trace::outlines;

/// The shape of one data module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleShape {
    /// A square.
    Square,
    /// A square with round corners (the rounding row).
    Rounded,
    /// A circle.
    Dot,
    /// A square turned by 45 degrees.
    Diamond,
}

impl ModuleShape {
    /// The stored option id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Rounded => "rounded",
            Self::Dot => "dot",
            Self::Diamond => "diamond",
        }
    }

    /// The shape a stored option id names.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        [Self::Square, Self::Rounded, Self::Dot, Self::Diamond]
            .into_iter()
            .find(|shape| shape.id() == id)
    }
}

/// How a grid is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// One module in millimetres.
    pub module: Length,
    /// The quiet zone in modules (only an inverted code draws it, as part of its rectangle).
    pub quiet: u32,
    /// The data module shape.
    pub shape: ModuleShape,
    /// The corner radius of [`ModuleShape::Rounded`] in modules, 0 to 0.5.
    pub rounding: f64,
    /// The share of the cell a module fills, 0.5 to 1.
    pub fill: f64,
    /// The finders' outer ring.
    pub finder_outer: FinderShape,
    /// The finders' centre.
    pub finder_inner: FinderShape,
    /// Whether touching modules become one outline; honoured for Square and Rounded at a fill of 1.
    pub merged: bool,
    /// Whether the dark and light modules swap (one rectangle with the dark modules cut out).
    pub inverted: bool,
    /// The modules the clear area clears, row by row; empty for none.
    pub cleared: Vec<bool>,
}

/// The outlines of `grid` in `style`, around the centre of the symbol, every one clockwise on the
/// screen (a hole the other way round), meant for the non-zero rule.
#[must_use]
pub fn style(grid: &ModuleGrid, style: &Style) -> Vec<ClosedPath> {
    let width = grid.width();
    #[allow(clippy::cast_precision_loss)]
    let frame = Frame {
        half: width as f64 / 2.0,
        module: style.module.as_mm(),
    };
    let cleared = |x: usize, y: usize| style.cleared.get(y * width + x).copied().unwrap_or(false);
    // An inverted code cuts the dark modules (and the cleared ones) out of a rectangle.
    let ink = |x: usize, y: usize| {
        if grid.in_finder(x, y) {
            false
        } else if style.inverted {
            grid.is_dark(x, y) || cleared(x, y)
        } else {
            grid.is_dark(x, y) && !cleared(x, y)
        }
    };
    let merged = style.merged
        && matches!(style.shape, ModuleShape::Square | ModuleShape::Rounded)
        && style.fill >= 1.0 - 1e-9;
    let mut paths = Vec::new();
    if merged {
        let rounding = if style.shape == ModuleShape::Rounded {
            style.rounding.clamp(0.0, 0.5)
        } else {
            0.0
        };
        paths.extend(outlines(frame, width, ink, rounding, false));
    } else {
        for y in 0..width {
            for x in 0..width {
                if ink(x, y) {
                    let whole = style.inverted && cleared(x, y);
                    paths.push(module(frame, style, (x, y), whole));
                }
            }
        }
    }
    for origin in origins(width) {
        paths.extend(finder(
            frame,
            origin,
            style.finder_outer,
            style.finder_inner,
        ));
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

/// One module's outline; `whole` draws the full cell whatever the shape.
fn module(frame: Frame, style: &Style, (x, y): (usize, usize), whole: bool) -> ClosedPath {
    #[allow(clippy::cast_precision_loss)]
    let (x, y) = (x as f64, y as f64);
    let fill = if whole { 1.0 } else { style.fill };
    let (cx, cy) = (x + 0.5, y + 0.5);
    let half = fill / 2.0;
    match style.shape {
        _ if whole => rounded_rect(frame, (x, y), (x + 1.0, y + 1.0), 0.0),
        ModuleShape::Square => {
            rounded_rect(frame, (cx - half, cy - half), (cx + half, cy + half), 0.0)
        }
        ModuleShape::Rounded => rounded_rect(
            frame,
            (cx - half, cy - half),
            (cx + half, cy + half),
            style.rounding.clamp(0.0, 0.5),
        ),
        ModuleShape::Dot => circle(frame, (cx, cy), half),
        ModuleShape::Diamond => diamond(frame, (cx, cy), half),
    }
}

/// The ghost of a code under the pointer (`Edit::Draft`, `0144` UX notes §2): the square of the
/// symbol and the outer edge of the three finders, no data modules (hundreds of module outlines
/// at a low zoom read as noise; the finders say "QR" at once).
#[must_use]
pub(crate) fn draft(grid: &ModuleGrid, style: &Style) -> Vec<ClosedPath> {
    let width = grid.width();
    #[allow(clippy::cast_precision_loss)]
    let (frame, side) = (
        Frame {
            half: width as f64 / 2.0,
            module: style.module.as_mm(),
        },
        width as f64,
    );
    let mut paths = vec![rounded_rect(frame, (0.0, 0.0), (side, side), 0.0)];
    for origin in origins(width) {
        paths.extend(
            finder(frame, origin, style.finder_outer, style.finder_inner)
                .into_iter()
                .take(1),
        );
    }
    paths
}
