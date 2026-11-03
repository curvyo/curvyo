//! What a fixture draws on the output page: open line segments in page-local millimetres.

use curvyo_document_core::{Length, Point};

/// One straight open line from `start` to `end`, page-local mm (origin at the page's top-left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    /// Start point.
    pub start: Point,
    /// End point.
    pub end: Point,
}

/// The outline of one fixture: segments drawn as thin strokes of one width, below the placed
/// drawing, never output and never hit (ADR 0032 §8).
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureDrawing {
    /// The segments.
    pub lines: Vec<LineSegment>,
    /// The stroke width of every segment.
    pub line_width: Length,
}

impl FixtureDrawing {
    /// A drawing with no lines (an unknown kind or disabled extension draws this).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            lines: Vec::new(),
            line_width: Length::from_mm(0.0),
        }
    }
}
