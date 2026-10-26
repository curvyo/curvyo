//! Why a file is not imported (`specs/0058-bitmap-import` criteria 17 and 26).

/// Why a file cannot be imported. The decoder's own error text is never part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    /// Not PNG, JPEG or WebP by its content.
    #[error("unsupported file type")]
    Unsupported,
    /// A supported type whose bytes cannot be read.
    #[error("damaged image")]
    Damaged,
    /// The file is larger than [`crate::MAX_FILE_BYTES`].
    #[error("file too large: {bytes} bytes")]
    FileTooLarge {
        /// The file's size.
        bytes: u64,
    },
    /// The header declares more than [`crate::MAX_HEADER_PIXELS`].
    #[error("image too large: {width} x {height} px")]
    TooManyPixels {
        /// The declared width.
        width: u32,
        /// The declared height.
        height: u32,
    },
    /// A buffer could not be allocated.
    #[error("not enough memory")]
    OutOfMemory,
}
