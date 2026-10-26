//! The fixed limits of the image import (`specs/0058-bitmap-import` criteria 9, 16 and 19), each
//! one named value.

/// The largest file the import reads: 100 MiB (criterion 16).
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

/// The most pixels a file header may declare, width times height: 50 megapixels (criterion 16).
pub const MAX_HEADER_PIXELS: u64 = 50_000_000;

/// The most pixels of a working image: 16 megapixels (criterion 16).
pub const MAX_WORKING_PIXELS: u64 = 16_000_000;

/// The longest side of a working image in pixels (criterion 16).
pub const MAX_WORKING_SIDE: u32 = 8192;

/// The most bytes a decoder may need for the full picture in its own pixel layout: eight bytes
/// (16-bit RGBA) per pixel of the header ceiling (criterion 19, ADR 0017 spike correction 2).
pub const MAX_DECODER_BYTES: u64 = MAX_HEADER_PIXELS * 8;

/// What a decode may use beyond the decoder buffer and the working image (criterion 19).
pub const DECODER_HEADROOM_BYTES: u64 = 64 * 1024 * 1024;

/// The density of a file without a usable one: the SVG/CSS pixel (criterion 9, decision D2).
pub const DEFAULT_DPI: f64 = 96.0;

/// The lowest density read from a file; below it the default applies (criterion 9).
pub const MIN_DPI: f64 = 10.0;

/// The highest density read from a file; above it the default applies (criterion 9).
pub const MAX_DPI: f64 = 2400.0;

/// The working size of a picture of `width` x `height` pixels (criterion 18): scaled by
/// `f = min(1, 8192 / max(w, h), sqrt(16,000,000 / (w * h)))` to `floor(w * f) x floor(h * f)`,
/// at least one pixel each way.
#[must_use]
pub fn working_size(width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (f64::from(width), f64::from(height));
    let side = f64::from(MAX_WORKING_SIDE) / w.max(h);
    #[allow(clippy::cast_precision_loss)]
    let area = (MAX_WORKING_PIXELS as f64 / (w * h)).sqrt();
    let factor = side.min(area).min(1.0);
    if factor >= 1.0 {
        return (width, height);
    }
    // invariant: `w * factor <= w <= u32::MAX`, so the casts below cannot overflow.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled = |side: f64| ((side * factor).floor() as u32).max(1);
    (scaled(w), scaled(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_example_of_criterion_18() {
        assert_eq!(working_size(5000, 4000), (4472, 3577));
    }

    #[test]
    fn small_pictures_keep_their_size_and_long_ones_stop_at_8192() {
        assert_eq!(working_size(4000, 3000), (4000, 3000));
        assert_eq!(working_size(16_000, 100), (8192, 51));
        assert_eq!(working_size(1, 1), (1, 1));
    }

    #[test]
    fn the_limits_are_the_spec_numbers() {
        assert_eq!(MAX_FILE_BYTES, 104_857_600);
        assert_eq!(MAX_HEADER_PIXELS, 50_000_000);
        assert_eq!(MAX_WORKING_PIXELS, 16_000_000);
        assert_eq!(MAX_DECODER_BYTES, 400_000_000);
    }
}
