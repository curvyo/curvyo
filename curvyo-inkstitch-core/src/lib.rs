// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from Ink/Stitch (https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors (see
// git history); changed for Curvyo. The upstream commit and the ported files are in `NOTICE.md`.

//! The port of the Ink/Stitch stitch algorithms (ADR 0095 decision 7, `specs/0160-embroidery`):
//! plain geometry in millimetres goes in, stitch point lists come out.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, clock, threads or UI, and no
//! `curvyo-*` dependency. Code of Curvyo's own that is not derived from Ink/Stitch belongs in
//! `curvyo-embroidery-core`. Nothing is ported yet; the embroidery MVP adds the first algorithms.

#![forbid(unsafe_code)]
