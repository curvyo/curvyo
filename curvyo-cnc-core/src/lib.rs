//! The CNC milling extension (ADR 0095 §2, `specs/0161-cnc-output`): its manifest and its built-in machine
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

/// The CNC milling workbench as a compiled-in extension (ADR 0080 §1): `manifest` only.
#[derive(Debug, Clone, Copy)]
pub struct Cnc;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static CNC: Cnc = Cnc;

impl Extension for Cnc {
    fn manifest(&self) -> Manifest {
        manifest()
    }
}
