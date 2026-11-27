//! The logo clear area: which modules are knocked out for a logo, and how many the error
//! correction can pay for (ADR 0091 §5).

use crate::grid::ModuleGrid;

/// The outline of the cleared area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoShape {
    /// A square.
    Square,
    /// A square with corners of a quarter of its side.
    Rounded,
    /// A circle.
    Circle,
}

impl LogoShape {
    /// The stored option id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Rounded => "rounded",
            Self::Circle => "circle",
        }
    }

    /// The shape a stored option id names.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        [Self::Square, Self::Rounded, Self::Circle]
            .into_iter()
            .find(|shape| shape.id() == id)
    }
}

/// The clear area a maker asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearArea {
    /// The shape.
    pub shape: LogoShape,
    /// The size in percent of the symbol width (without the quiet zone).
    pub size: u32,
    /// The margin around the shape, in modules.
    pub margin: u32,
}

/// The largest share of the correctable codewords the clear area may use, in modules per
/// codeword: 0.4 x 8 bits (ADR 0091 §5, measured against the decoder with a 30 % margin).
const BUDGET_PER_CODEWORD: f64 = 3.2;

/// The most modules the clear area of `grid` may clear.
#[must_use]
pub fn budget(grid: &ModuleGrid) -> usize {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let most = (grid.correctable() as f64 * BUDGET_PER_CODEWORD).floor() as usize;
    most
}

/// Whether the module at `(x, y)` lies inside the area: its centre inside the shape grown by the
/// margin, measured from the middle of the symbol, so the area is symmetric.
fn inside(width: usize, area: ClearArea, x: usize, y: usize) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let (w, x, y) = (width as f64, x as f64, y as f64);
    let (dx, dy) = ((x + 0.5 - w / 2.0).abs(), (y + 0.5 - w / 2.0).abs());
    let side = f64::from(area.size) / 100.0 * w;
    let margin = f64::from(area.margin);
    let half = side / 2.0 + margin;
    let eps = 1e-9;
    match area.shape {
        LogoShape::Square => dx <= half + eps && dy <= half + eps,
        LogoShape::Circle => dx * dx + dy * dy <= half * half + eps,
        LogoShape::Rounded => {
            let radius = side / 4.0 + margin;
            let (qx, qy) = (
                (dx - (half - radius)).max(0.0),
                (dy - (half - radius)).max(0.0),
            );
            dx <= half + eps && dy <= half + eps && qx * qx + qy * qy <= radius * radius + eps
        }
    }
}

/// The modules the clear area clears: row by row, `true` where cleared. Function patterns are
/// never cleared (an alignment pattern in the middle shows through).
#[must_use]
pub fn cleared(grid: &ModuleGrid, area: ClearArea) -> Vec<bool> {
    let width = grid.width();
    (0..width * width)
        .map(|at| {
            let (x, y) = (at % width, at / width);
            !grid.is_function(x, y) && inside(width, area, x, y)
        })
        .collect()
}

/// How many modules the clear area clears.
#[must_use]
pub fn count(grid: &ModuleGrid, area: ClearArea) -> usize {
    cleared(grid, area).into_iter().filter(|&c| c).count()
}

/// The smallest size the Size row takes, in percent.
pub const MIN_SIZE: u32 = 5;

/// `area` limited to the budget: the size shrinks first (down to [`MIN_SIZE`]), then the margin.
/// `None` when the area already fits.
#[must_use]
pub fn limited(grid: &ModuleGrid, area: ClearArea) -> Option<ClearArea> {
    let budget = budget(grid);
    if count(grid, area) <= budget {
        return None;
    }
    let mut size = area.size;
    while size > MIN_SIZE {
        size -= 1;
        let tried = ClearArea { size, ..area };
        if count(grid, tried) <= budget {
            return Some(tried);
        }
    }
    let mut margin = area.margin;
    while margin > 0 {
        margin -= 1;
        let tried = ClearArea {
            size: MIN_SIZE,
            margin,
            ..area
        };
        if count(grid, tried) <= budget {
            return Some(tried);
        }
    }
    Some(ClearArea {
        size: MIN_SIZE,
        margin: 0,
        ..area
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr::{EcLevel, encode};

    fn area(shape: LogoShape, size: u32, margin: u32) -> ClearArea {
        ClearArea {
            shape,
            size,
            margin,
        }
    }

    #[test]
    fn the_area_is_symmetric_and_never_touches_a_function_module() {
        let grid = encode(
            "https://example.com/a/longer/path/to/fill/version/five",
            EcLevel::H,
        )
        .unwrap();
        for shape in [LogoShape::Square, LogoShape::Rounded, LogoShape::Circle] {
            let cleared = cleared(&grid, area(shape, 20, 1));
            let n = grid.width();
            assert!(cleared.iter().any(|&c| c));
            for y in 0..n {
                for x in 0..n {
                    let at = cleared[y * n + x];
                    assert!(!(at && grid.is_function(x, y)));
                    // cleared modules are symmetric where both mirror images are non-function
                    if !grid.is_function(n - 1 - x, y) {
                        assert_eq!(at, cleared[y * n + n - 1 - x], "{shape:?} {x} {y}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_bigger_request_is_limited_to_the_budget() {
        let grid = encode("Hello", EcLevel::M).unwrap();
        let asked = area(LogoShape::Circle, 35, 2);
        let limited = limited(&grid, asked).expect("35 % at version 1 is over budget");
        assert!(limited.size < 35 || limited.margin < 2);
        assert!(count(&grid, limited) <= budget(&grid));
        assert_eq!(super::limited(&grid, limited), None, "a limited area stays");
        let small = area(LogoShape::Circle, 5, 0);
        assert_eq!(super::limited(&grid, small), None);
    }

    #[test]
    fn the_budget_is_a_share_of_the_correctable_codewords() {
        let grid = encode("Hello", EcLevel::M).unwrap();
        // version 1 at M: 10 error-correction codewords, 4 correctable (2 are held back for
        // detection): 0.4 x 8 x 4 = 12.8, rounded down
        assert_eq!(budget(&grid), 12);
    }
}
