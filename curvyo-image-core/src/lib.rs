//! The bitmap decoder boundary of the image import (`specs/0058-bitmap-import`, ADR 0017 §4):
//! hostile bytes in, a checked header or a deterministic working image out; and the PNG of a
//! working image for the clipboard (`specs/0087-image-clipboard-and-drop`); and the integer pixel
//! steps of the image adjustments with the picture analysis (`specs/0135`, ADR 0073).
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, clock or threads; callers pass the
//! bytes in. A panic aborts a wasm module, so every size is checked before a buffer exists:
//! [`probe`] reads the header and refuses what is over a ceiling, [`decode_working`] checks the
//! decoder's own buffer size before it decodes and allocates fallibly.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod adjust_analysis;
mod adjust_blur;
mod adjust_clahe;
mod adjust_denoise;
mod adjust_detail;
mod adjust_plane;
mod adjust_run;
mod adjust_tone;
mod container;
mod decode;
mod density;
mod downscale;
mod encode;
mod error;
mod format;
mod limits;
mod probe;
mod reduced;
mod thumbnail;
mod working;

pub use adjust_analysis::{ANALYSIS_PIXELS, Analysis, Histogram, noise_tenths};
pub use adjust_blur::{SubPx, box_widths, gauss_blur};
pub use adjust_plane::GreyPlane;
pub use adjust_run::{AdjustRun, Adjusted, PixelStep, peak_bytes, run_steps, step_bytes};
pub use adjust_tone::levels_table;
pub use decode::decode_working;
pub use density::PixelDensity;
pub use encode::encode_png;
pub use error::ImageError;
pub use format::ImageFormat;
pub use limits::{
    DECODER_HEADROOM_BYTES, DEFAULT_DPI, MAX_DECODER_BYTES, MAX_DPI, MAX_FILE_BYTES,
    MAX_HEADER_PIXELS, MAX_WORKING_PIXELS, MAX_WORKING_SIDE, MIN_DPI, working_size,
};
pub use probe::{ImageInfo, probe};
pub use reduced::TRACE_PREVIEW_SIDE;
pub use thumbnail::thumbnail;
pub use working::{MipLevel, WorkingImage};
