//! The parametric generators extension (ADR 0080, `specs/0141-parametric-generators`): a spur
//! gear and a laser box with finger joints, each a pure function from stored parameters to closed
//! outlines in millimetres.
//!
//! No clock, no randomness, no I/O; trigonometry through `libm`, so two peers write equal
//! outlines. The seed is part of the contract; neither generator uses it.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod boxes;
mod contour;
mod fingers;
mod footprint;
mod gear;
mod manifest;
mod outline;
mod path_box;
mod rounded_box;
mod schema;
mod shape_ops;
mod slit_bend;
mod tab_layout;
mod tab_runs;
mod values;
mod wall;
mod wall_bands;
mod wall_plates;
mod wave_slot;

use curvyo_extension_core::{Extension, GenerateRequest, Generated, Manifest};

pub use manifest::{BOX, GEAR, PATH_BOX, PROCESS_ID, ROUNDED_BOX, manifest};

/// The generators as a compiled-in extension.
#[derive(Debug, Clone, Copy)]
pub struct Generators;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static GENERATORS: Generators = Generators;

impl Extension for Generators {
    fn manifest(&self) -> Manifest {
        manifest()
    }

    fn generate(&self, request: &GenerateRequest<'_>) -> Option<Generated> {
        match request.kind {
            GEAR => Some(gear::generate(request)),
            BOX => Some(boxes::generate(request)),
            ROUNDED_BOX => Some(rounded_box::generate(request)),
            PATH_BOX => Some(path_box::generate(request)),
            _ => None,
        }
    }
}
