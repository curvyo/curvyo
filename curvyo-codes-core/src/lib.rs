//! The codes extension (ADR 0091, `specs/0144-qr-and-barcodes`): a QR code as a live generated
//! object, a pure function from a text and style parameters to closed outlines in millimetres.
//!
//! [`encode`] (text to module grid) and [`style`] (grid to outlines) are separate public
//! functions. No clock, no randomness, no I/O; the content never appears in a log or a message.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod clear_area;
mod finder;
mod grid;
mod manifest;
mod qr;
mod shapes;
mod style;
mod trace;
mod values;

use curvyo_extension_core::{Extension, GenerateRequest, Generated, Manifest};

pub use finder::FinderShape;
pub use grid::ModuleGrid;
pub use manifest::{PROCESS_ID, QR, manifest};
pub use qr::{EcLevel, EncodeError, encode};
pub use style::{ModuleShape, Style, style};

/// The codes as a compiled-in extension.
#[derive(Debug, Clone, Copy)]
pub struct Codes;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static CODES: Codes = Codes;

impl Extension for Codes {
    fn manifest(&self) -> Manifest {
        manifest()
    }

    fn generate(&self, request: &GenerateRequest<'_>) -> Option<Generated> {
        match request.kind {
            QR => Some(qr::generate(request)),
            _ => None,
        }
    }
}
