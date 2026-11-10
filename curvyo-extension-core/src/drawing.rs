//! An open line segment in page-local millimetres, the drawing unit of the laser grid.

use curvyo_document_core::Point;

/// One straight open line from `start` to `end`, page-local mm (origin at the page's top-left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    /// Start point.
    pub start: Point,
    /// End point.
    pub end: Point,
}
