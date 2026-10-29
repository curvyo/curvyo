//! The PNG of a working image for the clipboard (`specs/0087-image-clipboard-and-drop` criteria 11
//! and 12, `adrs.md` decision 2): the stored pixels as RGBA8, fast compression, no metadata chunk.

use image::ImageEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use crate::error::ImageError;
use crate::limits::{MAX_WORKING_PIXELS, MAX_WORKING_SIDE};
use crate::working::WorkingImage;

/// The PNG of `image`'s pixels: straight alpha, top row first, only the IHDR, IDAT and IEND
/// chunks (no density, colour profile or text), the same bytes for the same pixels.
///
/// # Errors
/// [`ImageError::TooManyPixels`] for a picture over the working limits (16 megapixels, 8192 px on a
/// side), checked before any buffer exists; [`ImageError::Damaged`] for an empty picture or a pixel
/// buffer that does not hold `width` x `height` RGBA pixels; [`ImageError::OutOfMemory`] when the
/// output cannot be allocated.
pub fn encode_png(image: &WorkingImage) -> Result<Vec<u8>, ImageError> {
    let (width, height) = (image.width, image.height);
    if width > MAX_WORKING_SIDE
        || height > MAX_WORKING_SIDE
        || u64::from(width) * u64::from(height) > MAX_WORKING_PIXELS
    {
        return Err(ImageError::TooManyPixels { width, height });
    }
    let expected = u64::from(width) * u64::from(height) * 4;
    if width == 0 || height == 0 || image.pixels.len() as u64 != expected {
        return Err(ImageError::Damaged);
    }
    let mut out = Vec::new();
    // A fast-compressed PNG is rarely larger than a quarter of its pixels; it grows past that.
    out.try_reserve(image.pixels.len() / 4)
        .map_err(|_| ImageError::OutOfMemory)?;
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            &image.pixels,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| ImageError::Damaged)?;
    Ok(out)
}
