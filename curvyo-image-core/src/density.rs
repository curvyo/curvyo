//! The physical density a file states (`specs/0058-bitmap-import` criterion 9): a PNG `pHYs`
//! chunk in pixels per metre, a JPEG JFIF density in dots per inch or per centimetre. EXIF is not
//! read; WebP has none. Fixed-offset reads that never allocate.

use crate::format::ImageFormat;
use crate::limits::{DEFAULT_DPI, MAX_DPI, MIN_DPI};

/// Dots per inch on each axis, both within [`MIN_DPI`]..=[`MAX_DPI`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelDensity {
    x_dpi: f64,
    y_dpi: f64,
}

impl PixelDensity {
    /// The default density of a file without a usable one: 96 dpi on both axes.
    pub const DEFAULT: Self = Self {
        x_dpi: DEFAULT_DPI,
        y_dpi: DEFAULT_DPI,
    };

    /// A density when both values are plausible, else `None` (the default then applies to both
    /// axes).
    #[must_use]
    pub fn new(x_dpi: f64, y_dpi: f64) -> Option<Self> {
        let plausible = |dpi: f64| (MIN_DPI..=MAX_DPI).contains(&dpi);
        (plausible(x_dpi) && plausible(y_dpi)).then_some(Self { x_dpi, y_dpi })
    }

    /// Dots per inch along the picture's width.
    #[must_use]
    pub const fn x_dpi(self) -> f64 {
        self.x_dpi
    }

    /// Dots per inch along the picture's height.
    #[must_use]
    pub const fn y_dpi(self) -> f64 {
        self.y_dpi
    }

    /// The same density for the picture turned a quarter: the axes swap.
    #[must_use]
    pub const fn swapped(self) -> Self {
        Self {
            x_dpi: self.y_dpi,
            y_dpi: self.x_dpi,
        }
    }
}

const METRES_PER_INCH: f64 = 0.0254;
const CM_PER_INCH: f64 = 2.54;

/// The density `bytes` of `format` state, or `None`.
pub(crate) fn read_density(format: ImageFormat, bytes: &[u8]) -> Option<PixelDensity> {
    match format {
        ImageFormat::Png => png_phys(bytes),
        ImageFormat::Jpeg => jfif_density(bytes),
        ImageFormat::WebP => None,
    }
}

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let chunk = bytes.get(at..at + 4)?;
    Some(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
}

fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let chunk = bytes.get(at..at + 2)?;
    Some(u16::from_be_bytes([chunk[0], chunk[1]]))
}

/// The `pHYs` chunk before the first `IDAT`, unit metre only.
fn png_phys(bytes: &[u8]) -> Option<PixelDensity> {
    let mut at = 8;
    loop {
        let length = usize::try_from(be_u32(bytes, at)?).ok()?;
        let kind = bytes.get(at + 4..at + 8)?;
        if kind == b"IDAT" || kind == b"IEND" {
            return None;
        }
        if kind == b"pHYs" && length == 9 {
            let data = at + 8;
            let unit = *bytes.get(data + 8)?;
            if unit != 1 {
                return None;
            }
            let per_metre = |value: u32| f64::from(value) * METRES_PER_INCH;
            return PixelDensity::new(
                per_metre(be_u32(bytes, data)?),
                per_metre(be_u32(bytes, data + 4)?),
            );
        }
        at = at.checked_add(12)?.checked_add(length)?;
    }
}

/// The JFIF `APP0` density right after the start marker: units 1 (dpi) or 2 (per cm).
fn jfif_density(bytes: &[u8]) -> Option<PixelDensity> {
    if bytes.get(2..4)? != [0xFF, 0xE0] || bytes.get(6..11)? != b"JFIF\0" {
        return None;
    }
    let units = *bytes.get(13)?;
    let x = f64::from(be_u16(bytes, 14)?);
    let y = f64::from(be_u16(bytes, 16)?);
    match units {
        1 => PixelDensity::new(x, y),
        2 => PixelDensity::new(x * CM_PER_INCH, y * CM_PER_INCH),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implausible_values_give_no_density() {
        assert!(PixelDensity::new(9.9, 300.0).is_none());
        assert!(PixelDensity::new(300.0, 2400.1).is_none());
        assert!(PixelDensity::new(0.0, 0.0).is_none());
        assert_eq!(
            PixelDensity::new(150.0, 300.0).map(PixelDensity::swapped),
            PixelDensity::new(300.0, 150.0)
        );
    }

    #[test]
    fn a_jfif_density_in_dots_per_cm_is_converted() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        jpeg.extend_from_slice(b"JFIF\0\x01\x01\x02\x00\x76\x00\x76");
        let density = jfif_density(&jpeg).unwrap();
        assert!((density.x_dpi() - 299.72).abs() < 1e-9);
    }

    #[test]
    fn a_png_without_chunks_or_with_a_huge_length_reads_none() {
        assert!(png_phys(b"\x89PNG\r\n\x1a\n").is_none());
        let mut huge = b"\x89PNG\r\n\x1a\n".to_vec();
        huge.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xF0]);
        huge.extend_from_slice(b"tEXt");
        assert!(png_phys(&huge).is_none());
    }
}
