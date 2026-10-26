//! Decoding a file into its working image (`specs/0058-bitmap-import` criteria 7, 8, 18, 19, 21
//! and 42): the header of [`crate::probe`], the decoder's buffer size checked and allocated
//! fallibly before the decode, the box downscale, then the orientation.

use image::{DynamicImage, ImageDecoder, RgbaImage, metadata::Orientation};

use crate::downscale::{Layout, to_working};
use crate::error::ImageError;
use crate::format::{ImageFormat, is_truncated_jpeg};
use crate::limits::{MAX_DECODER_BYTES, working_size};
use crate::probe::open;
use crate::working::WorkingImage;

/// The working image of `bytes`: RGBA8, straight alpha, sRGB assumed (ICC and gamma ignored),
/// reduced to the working limit with the integer box filter, then turned upright (after the
/// downscale, so a quarter turn copies the smaller buffer). Equal bytes give equal pixels.
///
/// # Errors
/// Every error of [`crate::probe`]; [`ImageError::TooManyPixels`] when the decoder would need more
/// than [`MAX_DECODER_BYTES`]; [`ImageError::OutOfMemory`] when a buffer cannot be allocated;
/// [`ImageError::Damaged`] for a decoder error.
pub fn decode_working(bytes: &[u8]) -> Result<WorkingImage, ImageError> {
    let (info, decoder) = open(bytes)?;
    let (width, height) = (info.stored_width, info.stored_height);
    // Checked before any buffer exists (ADR 0017 spike correction 1).
    if decoder.total_bytes() > MAX_DECODER_BYTES {
        return Err(ImageError::TooManyPixels { width, height });
    }
    let layout = Layout::of(decoder.color_type()).ok_or(ImageError::Unsupported)?;
    let length = usize::try_from(decoder.total_bytes()).map_err(|_| ImageError::OutOfMemory)?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(length)
        .map_err(|_| ImageError::OutOfMemory)?;
    buffer.resize(length, 0);
    decoder
        .read_image(&mut buffer)
        .map_err(|_| ImageError::Damaged)?;
    let (out_width, out_height) = working_size(width, height);
    let pixels = to_working(
        &buffer,
        layout,
        (width as usize, height as usize),
        (out_width as usize, out_height as usize),
    )?;
    drop(buffer);
    let (width, height, pixels) = upright(out_width, out_height, pixels, info.orientation)?;
    Ok(WorkingImage {
        width,
        height,
        pixels,
        original_size: info.upright_size(),
        truncated: info.format == ImageFormat::Jpeg && is_truncated_jpeg(bytes),
        animated: info.animated,
    })
}

fn upright(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    orientation: u8,
) -> Result<(u32, u32, Vec<u8>), ImageError> {
    let orientation = Orientation::from_exif(orientation).unwrap_or(Orientation::NoTransforms);
    if orientation == Orientation::NoTransforms {
        return Ok((width, height, pixels));
    }
    let buffer = RgbaImage::from_raw(width, height, pixels).ok_or(ImageError::Damaged)?;
    let mut image = DynamicImage::ImageRgba8(buffer);
    image.apply_orientation(orientation);
    let turned = image.into_rgba8();
    Ok((turned.width(), turned.height(), turned.into_raw()))
}
