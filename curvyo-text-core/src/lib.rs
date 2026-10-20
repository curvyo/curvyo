//! The Curvyo text engine (ADR 0020 §2): turns a face's bytes, a string and a size into
//! positioned glyph outlines and line metrics.
//!
//! Pure: no file, font enumeration, clock or thread. A platform crate finds the fonts and passes
//! the bytes of a face in once per face per session ([`FaceStore::add`]); everything here is
//! computed from those bytes (`specs/0074-text-objects` criterion 33). Hostile bytes give an
//! error, never a panic (criterion 32, limits in [`limits`]).
#![forbid(unsafe_code)]

mod error;
mod face;
mod face_store;
mod layout;
pub mod limits;
mod outline;
mod shape;
mod units;

pub use error::TextError;
pub use face::{Face, FaceMetrics};
pub use face_store::{FaceHash, FaceKey, FaceStore};
pub use layout::{LineLayout, PlacedGlyph, TextLayout, layout};
pub use outline::GlyphOutline;
pub use units::Mm;
