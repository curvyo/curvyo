//! What an extension sees when it computes a `Notice` block's text.

use curvyo_document_core::{DocumentSize, Point};

/// The facts of one manufacturing a notice may depend on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoticeContext {
    /// The work area (the output page's size).
    pub work_area: DocumentSize,
    /// The union of the placed content's bounds in page-local mm, `None` when nothing is placed.
    pub placed_bounds: Option<(Point, Point)>,
}
