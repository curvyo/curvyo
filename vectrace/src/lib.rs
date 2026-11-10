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
mod inpaint;
pub mod job;
mod line;
pub mod mask;
mod method;
pub mod ocr;
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
pub mod shapes;
mod shared_fit;
mod smooth;
mod text_pass;
mod trace;
mod units;
pub mod wire;

pub use analysis::{
    AnalysisReport, Capture, ImageType, Property, PropertyKind, Proposal, Reason, analyse,
};
pub use background::{sample_colour, suggest_background};
pub use colour_trace::{
    ColourLayer, ColourParams, ColourTrace, PaletteEntry, colour_palette, trace_colours,
};
pub use outline::node_count;
pub use palette::MAX_COLOURS;
pub use params::{
    Denoise, MAX_SWATCHES, PRESETS, PixelLength, Preset, Rgb8, Selection, Swatches, TraceParams,
    Upscale,
};
pub use progress::{Phase, Progress};
pub(crate) use raster::luma;
pub use raster::{Raster, auto_threshold};
pub use schema::{
    ADVANCED_PARAMS, DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, DEFAULT_THRESHOLD, DEFAULT_TOLERANCE_PX,
    METHOD_PARAMS, PARAMS, ParamKind, ParamSpec, ParamUnit,
};
pub use select::{DEFAULT_BACKGROUND_TOLERANCE, DEFAULT_PICK_TOLERANCE, selected};
pub use trace::{Counts, MAX_HOLES_PER_REGION, TraceError, TraceResult, trace};
pub use units::{Angle, Mm, PixelMap};
pub use wire::Output;
