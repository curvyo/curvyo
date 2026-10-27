//! The header of a file, read without any pixel buffer (`specs/0058-bitmap-import` criteria 2, 6,
//! 9 and 17): format by content, the declared size checked against the ceilings, the density and
//! the orientation.

use std::io::Cursor;

use image::{ImageDecoder, ImageReader, Limits};

use crate::container::walk;
use crate::density::{PixelDensity, read_density};
use crate::error::ImageError;
use crate::format::{ImageFormat, is_animated_webp};
use crate::limits::{MAX_FILE_BYTES, MAX_HEADER_PIXELS};

/// What the header of a supported file says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageInfo {
    /// The format by content.
    pub format: ImageFormat,
    /// The width as stored, before the orientation.
    pub stored_width: u32,
    /// The height as stored, before the orientation.
    pub stored_height: u32,
    /// The EXIF orientation, 1 to 8 (1 when the file has none).
    pub orientation: u8,
    /// The density the file states, along the stored axes, when it is plausible.
    pub density: Option<PixelDensity>,
    /// Whether it is an animated WebP (only the first frame is used).
    pub animated: bool,
}

impl ImageInfo {
    /// Whether the orientation turns the picture a quarter (EXIF 5 to 8).
    #[must_use]
    pub const fn quarter_turn(&self) -> bool {
        self.orientation >= 5
    }

    /// The upright size in pixels, the orientation applied (criterion 8).
    #[must_use]
    pub const fn upright_size(&self) -> (u32, u32) {
        if self.quarter_turn() {
            (self.stored_height, self.stored_width)
        } else {
            (self.stored_width, self.stored_height)
        }
    }

    /// The density along the upright axes, 96 dpi when the file states none (criterion 9).
    #[must_use]
    pub fn upright_density(&self) -> PixelDensity {
        let density = self.density.unwrap_or(PixelDensity::DEFAULT);
        if self.quarter_turn() {
            density.swapped()
        } else {
            density
        }
    }
}

/// Reads the header of `bytes`: the file ceiling first, then the format by content, then the
/// chunk structure and the declared size against the header ceiling, all before any pixel buffer
/// exists and before the decoder reads a chunk.
///
/// # Errors
/// [`ImageError::FileTooLarge`] over 100 MiB; [`ImageError::Unsupported`] for anything but PNG,
/// JPEG and WebP; [`ImageError::TooManyPixels`] over 50 megapixels; [`ImageError::Damaged`] for
/// a header that cannot be read, a chunk that does not fit in the file or a zero side.
pub fn probe(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    open(bytes).map(|(info, _, _)| info)
}

/// [`probe`], the decoder positioned after the header and whether a JPEG ends before its end
/// marker, for [`crate::decode_working`].
pub(crate) fn open(bytes: &[u8]) -> Result<(ImageInfo, impl ImageDecoder + '_, bool), ImageError> {
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > MAX_FILE_BYTES {
        return Err(ImageError::FileTooLarge { bytes: size });
    }
    let format = ImageFormat::detect(bytes).ok_or(ImageError::Unsupported)?;
    let structure = walk(format, bytes)?;
    // Before the decoder: on wasm32 it fails on a size it cannot address, before it reports it.
    if let Some((width, height)) = structure.declared {
        check_header_pixels(width, height)?;
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format.decoder_format());
    // Our own checks replace the decoder's: its `max_alloc` is not enforced on this path (ADR
    // 0017 spike correction 1), and a refusal by it would hide the numbers of the notice.
    reader.limits(Limits::no_limits());
    let mut decoder = reader.into_decoder().map_err(|_| ImageError::Damaged)?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err(ImageError::Damaged);
    }
    check_header_pixels(width, height)?;
    let orientation = decoder
        .orientation()
        .map_or(1, image::metadata::Orientation::to_exif);
    let info = ImageInfo {
        format,
        stored_width: width,
        stored_height: height,
        orientation: if (1..=8).contains(&orientation) {
            orientation
        } else {
            1
        },
        density: read_density(format, bytes),
        animated: format == ImageFormat::WebP && is_animated_webp(bytes),
    };
    Ok((info, decoder, structure.truncated))
}

fn check_header_pixels(width: u32, height: u32) -> Result<(), ImageError> {
    if u64::from(width) * u64::from(height) > MAX_HEADER_PIXELS {
        return Err(ImageError::TooManyPixels { width, height });
    }
    Ok(())
}
