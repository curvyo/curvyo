// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Ink/Stitch port (https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors
// (see git history); changed for Curvyo. The upstream commit and the ported files are in
// `NOTICE.md`.

//! The port of the Ink/Stitch stitch algorithms (ADR 0097 decision 7, `specs/0160-embroidery`):
//! plain geometry in millimetres goes in, stitch point lists come out.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, clock, threads or UI, and no
//! `curvyo-*` dependency. Code of Curvyo's own that is not derived from Ink/Stitch belongs in
//! `curvyo-embroidery-core`: block order, ties between blocks, rounding and the file formats.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod running_stitch;
mod satin_zigzag;
mod tatami_fill;
mod ties;
mod units;

pub use running_stitch::{RUNNING_TOLERANCE, repeat_passes, running_stitch};
pub use satin_zigzag::satin_zigzag;
pub use tatami_fill::tatami_fill;
pub use ties::{tie_in, tie_off};
pub use units::{Angle, Mm};
