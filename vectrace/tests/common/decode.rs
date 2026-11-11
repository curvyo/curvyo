//! Decodes a PNG or JPEG fixture into upright RGBA8, as `curvyo-image-core::decode_working` does
//! for pictures within its working limits (every fixture is): the file's EXIF orientation applied,
//! gray and RGB widened to RGBA with alpha 255, straight alpha kept. Tests only.

#![allow(dead_code)]

use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageReader, Limits, RgbaImage};

/// The working limit of `curvyo-image-core`: larger pictures would be reduced there, which this
/// helper does not do, so it refuses them.
const MAX_WORKING_PIXELS: u64 = 16_000_000;

/// An upright RGBA8 picture, top row first.
pub struct WorkingImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The picture in `bytes` (PNG or JPEG).
pub fn decode_working(bytes: &[u8]) -> Result<WorkingImage, ImageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(Limits::no_limits());
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let (width, height) = decoder.dimensions();
    assert!(u64::from(width) * u64::from(height) <= MAX_WORKING_PIXELS);
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    // 16-bit channels are rounded to 8 bits, as `curvyo-image-core` does.
    let rgba = if image.color().bits_per_pixel() / u16::from(image.color().channel_count()) == 16 {
        let wide = image.into_rgba16();
        let (w, h) = wide.dimensions();
        let bytes = wide
            .into_raw()
            .into_iter()
            .map(|v| u8::try_from((u32::from(v) + 128) / 257).unwrap_or(u8::MAX))
            .collect();
        RgbaImage::from_raw(w, h, bytes).expect("same sample count")
    } else {
        image.into_rgba8()
    };
    Ok(WorkingImage {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}
