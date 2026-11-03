//! The laser workbench extension (ADR 0034 §1): its manifest, the outlines of its fixtures and
//! the text of its notices. It receives document data as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod engraved_grid;
mod manifest;
mod overhang;

pub use engraved_grid::{ENGRAVED_GRID, engraved_grid};
pub use manifest::{PROCESS_ID, manifest};
pub use overhang::{OVERHANG_NOTICE, OVERHANG_TOLERANCE, overhang_text};

use curvyo_document_core::DocumentSize;
use curvyo_extension_core::{FixtureDrawing, NoticeContext, StoredParams};

/// The outline of a fixture of kind `kind` (the part after `curvyo.laser/`) with the stored
/// values `params` over a work area; an unknown kind draws nothing.
#[must_use]
pub fn fixture_drawing(
    kind: &str,
    params: &StoredParams,
    work_area: DocumentSize,
) -> FixtureDrawing {
    match kind {
        ENGRAVED_GRID => engraved_grid(params, work_area),
        _ => FixtureDrawing::empty(),
    }
}

/// The text of the notice `id`, `None` while it does not apply or for an unknown id.
#[must_use]
pub fn notice(id: &str, context: &NoticeContext) -> Option<String> {
    match id {
        OVERHANG_NOTICE => overhang_text(context),
        _ => None,
    }
}
