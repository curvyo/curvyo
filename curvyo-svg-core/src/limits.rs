//! The named limits of the SVG import (`specs/0024-svg-import-export` criteria 6, 47 and 69):
//! far above real files, they keep a hostile one from exhausting memory or time.

/// The largest file read (50 MiB); a larger one is refused.
pub const MAX_FILE_BYTES: usize = 50 * 1024 * 1024;

/// The most elements read; the rest of the file is ignored.
pub const MAX_ELEMENTS: usize = 500_000;

/// The most XML nodes (elements, text, comments) the reader builds; more refuse the file.
pub const MAX_XML_NODES: u32 = 4_000_000;

/// The most path points (one per line or move, three per cubic) read, `use` copies included;
/// the reading stops there and keeps what it has.
pub const MAX_PATH_POINTS: usize = 5_000_000;

/// The longest attribute value read (16 MiB); a longer one is skipped.
pub const MAX_ATTRIBUTE_BYTES: usize = 16 * 1024 * 1024;

/// The most numbers of a `stroke-dasharray` read.
pub const MAX_DASHES: usize = 1000;

/// How deep `use` copies may nest.
pub const MAX_USE_DEPTH: usize = 16;

/// The most `use` copies one file makes.
pub const MAX_USE_COPIES: usize = 100_000;

/// The longest chain of linked clones an import makes (`0049` criterion 52).
pub const MAX_CLONE_CHAIN: usize = 8;

/// The largest coordinate (millimetres) a leaf may reach after all transforms.
pub const MAX_COORDINATE_MM: f64 = 1e7;

/// The page side limits (`0015`): a page from the file is clamped to them.
pub const PAGE_MM: (f64, f64) = (1.0, 100_000.0);
