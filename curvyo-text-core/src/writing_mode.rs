//! The writing modes and glyph orientations of a text and the map from line space to text space
//! (`specs/0079-text-layout-controls` criteria 20, 22, 23).

use kurbo::Affine;

use crate::vertical_orientation::is_upright_char;

/// Which way the lines of a text run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritingMode {
    /// Lines run left to right, the next line below (today's layout).
    #[default]
    Horizontal,
    /// Lines are columns top to bottom, the next column to the left.
    VerticalRl,
    /// Lines are columns top to bottom, the next column to the right.
    VerticalLr,
}

impl WritingMode {
    /// Whether the lines are columns.
    #[must_use]
    pub const fn is_vertical(self) -> bool {
        !matches!(self, Self::Horizontal)
    }

    /// The map from line space (x along the line, y across it, the first baseline at y = 0) to
    /// text space (y down). Vertical: turned a quarter clockwise, so the line runs down and a
    /// larger line-space y lies further left.
    #[must_use]
    pub const fn frame(self) -> Affine {
        match self {
            Self::Horizontal => Affine::IDENTITY,
            Self::VerticalRl | Self::VerticalLr => Affine::new([0.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
        }
    }

    /// The line-space y of line `index`'s baseline, `pitch` apart: down the page for horizontal
    /// and right-to-left columns, up (so to the right in text space) for left-to-right columns.
    pub(crate) fn baseline(self, index: f64, pitch: f64) -> f64 {
        match self {
            Self::VerticalLr => -index * pitch,
            Self::Horizontal | Self::VerticalRl => index * pitch,
        }
    }
}

/// How the glyphs of a vertical line stand (criterion 23); horizontal text ignores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlyphOrientation {
    /// Upright for characters whose Unicode `Vertical_Orientation` is U or Tu, else sideways.
    #[default]
    Auto,
    /// Every glyph upright, advanced by its vertical advance.
    Upright,
    /// The line set as horizontal text turned a quarter clockwise.
    Sideways,
}

impl GlyphOrientation {
    /// Whether character `c` stands upright in a vertical line.
    #[must_use]
    pub fn is_upright(self, c: char) -> bool {
        match self {
            Self::Auto => is_upright_char(c),
            Self::Upright => true,
            Self::Sideways => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Point;

    #[test]
    fn a_column_runs_down_and_the_next_one_lies_left_or_right() {
        let frame = WritingMode::VerticalRl.frame();
        let down = frame * Point::new(10.0, 0.0);
        assert!(down.x.abs() < 1e-12 && (down.y - 10.0).abs() < 1e-12);
        let rl = frame * Point::new(0.0, WritingMode::VerticalRl.baseline(1.0, 5.0));
        let lr = frame * Point::new(0.0, WritingMode::VerticalLr.baseline(1.0, 5.0));
        assert!((rl.x + 5.0).abs() < 1e-12 && (lr.x - 5.0).abs() < 1e-12);
    }

    #[test]
    fn auto_follows_uax_50() {
        assert!(GlyphOrientation::Auto.is_upright('日'));
        assert!(!GlyphOrientation::Auto.is_upright('A'));
        assert!(GlyphOrientation::Upright.is_upright('A'));
        assert!(!GlyphOrientation::Sideways.is_upright('日'));
    }
}
