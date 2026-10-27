//! The raster tracer of the Trace tool (`specs/0025-raster-trace`, ADR 0023): it turns an RGBA8
//! raster and trace settings into closed outlines in document millimetres.
//!
//! Our pre-processing (luma over white, Threshold, 4-connected `u32` region labelling, specks in
//! mm²) runs before `visioncortex` (the library under `VTracer`) smooths and fits each boundary;
//! our post-processing turns its splines into `kurbo` paths, makes near-straight cubics lines and
//! maps them through the pixel map. Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem,
//! network, clock or threads; the caller passes everything in and stops a trace through `poll`.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod outline;
mod params;
mod progress;
mod raster;
mod regions;
mod trace;

pub use params::{
    DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, DEFAULT_THRESHOLD, MAX_SPECKS_MM, PARAMS, ParamKind,
    ParamSpec, ParamUnit, TraceParams,
};
pub use progress::{Phase, Progress};
pub use raster::{Raster, auto_threshold};
pub use trace::{Counts, MAX_HOLES_PER_REGION, TraceError, TraceResult, trace};
