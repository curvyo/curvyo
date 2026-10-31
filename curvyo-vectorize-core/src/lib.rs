//! The raster tracer of the Trace tool (`specs/0025-raster-trace`, ADR 0023): it turns an RGBA8
//! raster and trace settings into closed outlines in document millimetres.
//!
//! A score per pixel from the selection method (Brightness, Remove background, Pick colours)
//! with the alpha counted once, Denoise, Upscale, the cut, `u32` region labelling (foreground 8-connected,
//! background 4-connected), background
//! and specks in mm², then per boundary our own crack-edge chain, structural corners found before
//! any smoothing, smoothing with the corners pinned, lines and cubics fitted within a tolerance in
//! source pixels, a guard against crossing outlines, and the map to millimetres
//! (`specs/0089-trace-quality-and-colour` slice D). Pure and wasm-compatible (`CLAUDE.md` §6): no
//! filesystem, network, clock or threads; the caller passes everything in and stops a trace
//! through `poll`. Basic operations and `sqrt` only, so native and wasm results are identical.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod analysis;
mod background;
mod boundary;
mod clearance;
mod colour_map;
mod colour_trace;
mod contour;
mod corner_place;
mod corners;
mod edges;
mod filter;
mod fit;
mod guard;
mod guarded;
mod line;
mod method;
mod outline;
mod palette;
mod params;
mod picker;
mod progress;
mod raster;
mod regions;
mod schema;
mod segment;
mod select;
mod shared_fit;
mod smooth;
mod trace;

// The integration tests' helpers (the quality set and its metrics) for the one unit test that
// needs the trace before the node reduction (`trace::tests`).
#[cfg(test)]
extern crate self as curvyo_vectorize_core;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
#[allow(clippy::pedantic, clippy::all, missing_docs)]
mod test_common;

pub use analysis::{
    AnalysisReport, Capture, ImageType, LOW_CONFIDENCE, Property, PropertyKind, Proposal, Reason,
    analyse,
};
pub use background::{sample_colour, suggest_background};
pub use colour_trace::{
    ColourLayer, ColourMask, ColourParams, ColourTrace, PaletteEntry, colour_mask, trace_colours,
};
pub use palette::{DEFAULT_COLOURS, MAX_COLOURS, MIN_COLOURS};
pub use params::{
    Denoise, MAX_SWATCHES, PRESETS, PixelLength, Preset, Rgb8, Selection, Swatches, TraceParams,
    UPSCALE_AUTO_BELOW_PX, Upscale,
};
pub use progress::{Phase, Progress};
pub use raster::{Raster, auto_threshold};
pub use schema::{
    ADVANCED_PARAMS, DEFAULT_CORNERS, DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, DEFAULT_THRESHOLD,
    DEFAULT_TOLERANCE_PX, MAX_SPECKS_MM, MAX_TOLERANCE_PX, METHOD_PARAMS, MIN_TOLERANCE_PX, PARAMS,
    ParamKind, ParamSpec, ParamUnit,
};
pub use select::{DEFAULT_BACKGROUND_TOLERANCE, DEFAULT_PICK_TOLERANCE, selected};
pub use trace::{
    Counts, Diagnostics, MAX_HOLES_PER_REGION, Mask, TraceError, TraceResult, mask, trace,
};
