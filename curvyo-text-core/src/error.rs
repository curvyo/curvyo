//! The engine's error type.

use crate::face_store::FaceKey;
use crate::limits::{MAX_FACE_BYTES, MAX_SHAPE_CHARS};

/// Why the engine could not read a face or lay out a text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    /// The file is larger than [`MAX_FACE_BYTES`].
    #[error("the font file is larger than {MAX_FACE_BYTES} bytes")]
    FaceTooLarge,
    /// The bytes hold no readable TrueType or OpenType face at this index.
    #[error("the bytes hold no readable font face at index {0}")]
    Unreadable(u32),
    /// The store holds no face under this key.
    #[error("the face {0} is not loaded")]
    UnknownFace(FaceKey),
    /// The string has more than [`MAX_SHAPE_CHARS`] characters.
    #[error("a text holds at most {MAX_SHAPE_CHARS} characters")]
    TextTooLong,
    /// The bytes are not a readable Unicode names file (`specs/0082-glyph-table`).
    #[error("the Unicode names file is damaged")]
    UnreadableNames,
}
