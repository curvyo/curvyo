//! The named limits that keep hostile input from crashing or hanging the engine (ADR 0020 §2,
//! `specs/0074-text-objects/adrs.md` decision 5).

/// The largest font file the engine reads, in bytes (64 MiB).
pub const MAX_FACE_BYTES: usize = 64 * 1024 * 1024;

/// The most points one glyph outline may have; a larger glyph is drawn as nothing.
pub const MAX_GLYPH_POINTS: usize = 65_535;

/// The most Unicode scalar values one call lays out: the document's hostile-input bound
/// `MAX_STORED_TEXT_CHARS` (`curvyo-document-core`), which a `curvyo-ui-core` test checks equal;
/// this crate depends on no workspace crate, so it holds its own constant. A merged text longer
/// than the typing limit still draws and converts in full.
pub const MAX_SHAPE_CHARS: usize = 100_000;
