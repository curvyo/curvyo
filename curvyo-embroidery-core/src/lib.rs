//! The embroidery extension (ADR 0095 §2, `specs/0160-embroidery`): its manifest and its built-in machine
//! profiles. It receives document data as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod builtins;
mod manifest;

pub use builtins::{BUILTINS, Builtin, DEFAULT_BUILTIN};
pub use manifest::{ADD_PAGE_COMMIT_LABEL, PROCESS_ID, WORD, manifest};

use curvyo_extension_core::{Extension, Manifest};

/// The embroidery workbench as a compiled-in extension (ADR 0080 §1): `manifest` only.
#[derive(Debug, Clone, Copy)]
pub struct Embroidery;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static EMBROIDERY: Embroidery = Embroidery;

impl Extension for Embroidery {
    fn manifest(&self) -> Manifest {
        manifest()
    }
}
