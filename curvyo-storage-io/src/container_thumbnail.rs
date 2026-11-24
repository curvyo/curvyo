//! Puts the page's PNG into `.curvyo` container bytes as the `thumbnail.png` member a workspace
//! row shows (`specs/0095-workspace` criterion 6, ADR 0004 §1).
//!
//! The join itself is `curvyo_document_core::with_thumbnail`; every other member is copied raw,
//! so the document's own bytes are not touched. A build that does not know the member never
//! reads it: containers are read member by member by name.

use crate::workspace_documents::MAX_THUMBNAIL_BYTES;

/// The eight bytes every PNG file starts with.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Why a thumbnail was not put into a container. The caller saves without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ThumbnailError {
    /// The thumbnail bytes are not a PNG file.
    #[error("the thumbnail is not a PNG file")]
    NotPng,
    /// The thumbnail is larger than a workspace row reads ([`MAX_THUMBNAIL_BYTES`]).
    #[error("the thumbnail is larger than a row reads")]
    TooLarge,
    /// The container bytes are not a readable zip archive.
    #[error("the document bytes are not a readable container")]
    NotAContainer,
}

/// `container` with `png` as its `thumbnail.png` member, replacing an earlier one.
///
/// # Errors
/// [`ThumbnailError::NotPng`], [`ThumbnailError::TooLarge`], [`ThumbnailError::NotAContainer`].
pub fn with_thumbnail(container: &[u8], png: &[u8]) -> Result<Vec<u8>, ThumbnailError> {
    if !png.starts_with(PNG_SIGNATURE) {
        return Err(ThumbnailError::NotPng);
    }
    if png.len() as u64 > MAX_THUMBNAIL_BYTES {
        return Err(ThumbnailError::TooLarge);
    }
    curvyo_document_core::with_thumbnail(container, png).map_err(|_| ThumbnailError::NotAContainer)
}
