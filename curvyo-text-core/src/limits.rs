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

/// The largest SVG font file the engine reads, in bytes (16 MiB), checked before parsing
/// (`specs/0076-single-line-fonts` criterion 23).
pub const MAX_SVG_FONT_BYTES: usize = 16 * 1024 * 1024;

/// The most XML nodes one SVG font file may have.
pub const MAX_SVG_NODES: u32 = 262_144;

/// The deepest element nesting an SVG font file may have, checked on the bytes before parsing
/// (the XML reader recurses per level).
pub const MAX_SVG_DEPTH: usize = 256;

/// The most glyphs one SVG font may have.
pub const MAX_SVG_GLYPHS: usize = 65_535;

/// The most points all glyphs of one SVG font may have together, so a 16 MiB file of arcs cannot
/// fill the memory.
pub const MAX_SVG_FONT_POINTS: usize = 1 << 20;

/// The most kerning pairs one SVG font may state (the product of its `hkern` lists).
pub const MAX_SVG_KERN_PAIRS: usize = 1 << 20;

/// The largest absolute value of a number in an SVG font (font units). A real em is at most
/// 16,384 units; a larger number is hostile and would make arcs split into billions of curves.
pub const MAX_SVG_COORDINATE: f64 = 1.0e6;

/// The largest zip archive Add font reads, in bytes (256 MiB, `specs/0083-font-info-and-zip`
/// criterion 15).
pub const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;

/// The most entries one archive may declare; checked before its table is read.
pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;

/// The most bytes the kept entries (fonts and documents) of one archive may decompress to
/// together (256 MiB), counted on the bytes actually read.
pub const MAX_ARCHIVE_KEPT_BYTES: usize = 256 * 1024 * 1024;

/// The most font entries one archive may hold.
pub const MAX_ARCHIVE_FONTS: usize = 500;

/// The largest document entry kept from an archive (16 MiB).
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
