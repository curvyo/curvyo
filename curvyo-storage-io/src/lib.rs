//! Impure capabilities the desktop client needs, one module each
//! (ADR 0011 §2). This slice only needs the filesystem module; the other
//! modules (OS CSPRNG, clock, credential store, HTTP, git wire, relay
//! socket) belong to later stories and get no code here
//! (`specs/0001-project-file-foundation/adrs.md`).

// `CLAUDE.md` §5 allows unwrap/expect in tests; only production code is held
// to the stricter rule.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod filesystem;
mod font_files;
mod fonts;
mod svg_fonts;

pub use filesystem::{
    FsError, ensure_parent_dir, read_optional, read_to_vec, rename_replacing, write_atomic,
};
pub use font_files::{AddedFonts, RemovedFonts, add_font_files, remove_font_files};
pub use fonts::{
    FolderFonts, FontFace, FontSource, MAX_FONT_FILE_BYTES, list_font_folder, list_system_fonts,
};
