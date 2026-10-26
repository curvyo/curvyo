//! The bitmap decoder boundary of the image import (`specs/0058-bitmap-import`, ADR 0017 §4):
//! hostile bytes in, a checked header or a deterministic working image out.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, clock or threads; callers pass the
//! bytes in. A panic aborts a wasm module, so every size is checked before a buffer exists:
//! [`probe`] reads the header and refuses what is over a ceiling, [`decode_working`] checks the
//! decoder's own buffer size before it decodes and allocates fallibly.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod decode;
mod density;
mod downscale;
mod error;
mod format;
mod limits;
mod probe;
mod working;

pub use decode::decode_working;
pub use density::PixelDensity;
pub use error::ImageError;
pub use format::ImageFormat;
pub use limits::{
    DECODER_HEADROOM_BYTES, DEFAULT_DPI, MAX_DECODER_BYTES, MAX_DPI, MAX_FILE_BYTES,
    MAX_HEADER_PIXELS, MAX_WORKING_PIXELS, MAX_WORKING_SIDE, MIN_DPI, working_size,
};
pub use probe::{ImageInfo, probe};
pub use working::{MipLevel, WorkingImage};
