//! The three finder patterns of a QR code, drawn by their own style (not the module style): a
//! 7 x 7 ring with a 5 x 5 hole and a 3 x 3 centre, each Square, Rounded or Circle.

use curvyo_extension_core::ClosedPath;

use crate::shapes::{Frame, circle, reversed, rounded_rect};

/// The outline family of a finder's ring or centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinderShape {
    /// Square corners.
    Square,
    /// Corners of a fixed radius (1.5 modules on the ring, 0.75 on the centre).
    Rounded,
    /// A circle.
    Circle,
}

impl FinderShape {
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

    /// One shape of `side` x `side` modules at `(x, y)`, `inset` modules in from the 7 x 7 edge
    /// of the ring (the ring's hole is the same family one module in).
    fn shape(self, frame: Frame, (x, y): (f64, f64), side: f64, radius: f64) -> ClosedPath {
        match self {
            Self::Square => rounded_rect(frame, (x, y), (x + side, y + side), 0.0),
            Self::Rounded => rounded_rect(frame, (x, y), (x + side, y + side), radius),
            Self::Circle => circle(frame, (x + side / 2.0, y + side / 2.0), side / 2.0),
        }
    }
}

/// The outlines of the finder whose top-left module is `(x, y)`: the ring's outer edge, its hole
/// (the other way round) and the centre.
pub(crate) fn finder(
    frame: Frame,
    (x, y): (usize, usize),
    outer: FinderShape,
    inner: FinderShape,
) -> Vec<ClosedPath> {
    #[allow(clippy::cast_precision_loss)]
    let (x, y) = (x as f64, y as f64);
    vec![
        outer.shape(frame, (x, y), 7.0, 1.5),
        reversed(&outer.shape(frame, (x + 1.0, y + 1.0), 5.0, 0.5)),
        inner.shape(frame, (x + 2.0, y + 2.0), 3.0, 0.75),
    ]
}

/// The top-left modules of the three finders of a `width` x `width` code.
pub(crate) const fn origins(width: usize) -> [(usize, usize); 3] {
    [(0, 0), (width - 7, 0), (0, width - 7)]
}
