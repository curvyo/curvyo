//! The Curvyo text engine (ADR 0020 §2): turns a face's bytes, a string and a size into
//! positioned glyph outlines and line metrics. Reads OpenType faces and SVG fonts
//! (`specs/0076-single-line-fonts`).
//!
//! Pure: no file, font enumeration, clock or thread. A platform crate finds the fonts and passes
//! the bytes of a face in once per face per session ([`FaceStore::add`]); everything here is
//! computed from those bytes (`specs/0074-text-objects` criterion 33). Hostile bytes give an
//! error, never a panic (criterion 32, limits in [`limits`]).
#![forbid(unsafe_code)]

mod align;
mod char_offsets;
mod error;
mod face;
mod face_store;
mod face_style;
mod font_class;
mod font_names;
mod glyph_cell;
mod glyph_list;
mod glyph_map;
mod layout;
pub mod limits;
mod outline;
mod path_data;
mod preview;
mod shape;
mod svg_font;
mod taper;
mod unicode_blocks;
mod unicode_names;
mod units;
mod vertical_orientation;
mod writing_mode;

pub use align::Align;
pub use char_offsets::CharOffset;
pub use error::TextError;
pub use face::{Face, FaceKind, FaceMetrics};
pub use face_store::{FaceHash, FaceKey, FaceStore};
pub use face_style::{FaceStyle, NORMAL_WIDTH, Slant, WeightBand};
pub use font_class::{DeclaredClass, OS2_CLASS_BYTES, declared_class, os2_class_bytes};
pub use font_names::{ListableFace, family_name, listable_faces, style_name, svg_listable_faces};
pub use glyph_cell::{GlyphCell, glyph_cell};
pub use glyph_list::ListedChar;
pub use layout::{LayoutOptions, LineLayout, PlacedGlyph, TAPER_ACCURACY, TextLayout, layout};
pub use outline::GlyphOutline;
pub use preview::{preview_path, preview_path_within};
pub use unicode_blocks::{BLOCKS, UNICODE_VERSION, block_of};
pub use unicode_names::UnicodeNames;
pub use units::Mm;
pub use writing_mode::{GlyphOrientation, WritingMode};
