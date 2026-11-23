//! Runs the three image models of `specs/0135` slice 3 (Remove background, Upscale, AI denoise) on
//! a picture with the weights handed in as bytes (ADR 0075).
//!
//! Pure compute (`CLAUDE.md` §6): no filesystem, network, clock or threads of its own; the caller
//! downloads and caches the model file (ADR 0076) and stores the result (ADR 0077). The job never
//! depends on this crate: `curvyo-laser-core` and `curvyo-image-core` read only the stored result
//! (checked in CI). Model output is run-to-run identical but not promised identical across
//! targets; the stored PNG is the source of truth (ADR 0075 §8).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod model;
mod models;
mod run;
mod tiles;

pub use error::AiError;
pub use model::{Model, verify};
pub use models::{MODEL_HOST, MODELS, ModelFile, ModelKind, model};
pub use run::{
    Factor, MASK_SIDE, denoise, denoise_tile, remove_background, upscale, upscale_tile,
    upscaled_size,
};
pub use tiles::{TILE, TILE_OVERLAP, TileOutput, assemble, tile_count};
