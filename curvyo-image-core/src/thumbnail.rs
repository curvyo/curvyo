//! The small picture of a file for the import dialog's preview (`specs/0087-image-clipboard-and-drop`
//! criteria 26 and 28, `adrs.md` decision 6).

use crate::decode::decode_working;
use crate::error::ImageError;
use crate::working::WorkingImage;

/// The upright picture of `bytes` with its longest side at most `longest` pixels, aspect kept,
/// never enlarged. It goes through the import's own door: the header and every ceiling of
/// [`crate::probe`] before any pixel buffer, then the bounded decode of [`decode_working`], then
/// [`WorkingImage::reduced_copy`].
///
/// # Errors
/// Every error of [`decode_working`].
pub fn thumbnail(bytes: &[u8], longest: u32) -> Result<WorkingImage, ImageError> {
    Ok(decode_working(bytes)?.reduced_copy(longest))
}
