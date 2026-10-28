//! The raster tracer of the Trace tool (`specs/0025-raster-trace`, ADR 0023): it turns an RGBA8
//! raster and trace settings into closed outlines in document millimetres.
//!
//! Our pre-processing (luma over white, Denoise, Upscale, Threshold, 4-connected `u32` region
//! labelling, background and specks in mm²) runs before `visioncortex` (the library under `VTracer`) smooths and fits each boundary;
//! our post-processing turns its splines into `kurbo` paths, makes near-straight cubics lines, merges
//! nodes above Smooth 80 within a tolerance and maps them through the pixel map. Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem,
//! network, clock or threads; the caller passes everything in and stops a trace through `poll`.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod filter;
mod outline;
mod params;
mod progress;
mod raster;
mod reduce;
mod regions;
mod trace;

// The integration tests' helpers (the quality set and its metrics) for the one unit test that
// needs the trace before the node reduction (`trace::tests`).
#[cfg(test)]
extern crate self as curvyo_vectorize_core;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
#[allow(clippy::pedantic, clippy::all, missing_docs)]
mod test_common;

pub use params::{
    ADVANCED_PARAMS, DEFAULT_CORNERS, DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, DEFAULT_THRESHOLD,
    Denoise, MAX_SPECKS_MM, PARAMS, PRESETS, ParamKind, ParamSpec, ParamUnit, Preset, TraceParams,
    UPSCALE_AUTO_BELOW_PX, Upscale,
};
pub use progress::{Phase, Progress};
pub use raster::{Raster, auto_threshold};
pub use trace::{Counts, MAX_HOLES_PER_REGION, TraceError, TraceResult, trace};
