//! A grey plane of a picture (ADR 0073 §1): one byte per pixel, 0 black to 255 white.

use crate::working::WorkingImage;

/// A row-major grey picture, top row first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreyPlane {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The greys, top row first.
    pub grey: Vec<u8>,
}

impl GreyPlane {
    /// The grey of `image`: each pixel composited over white (straight alpha) and weighted by the
    /// fixed luminance weights of `over_white_grey` (ADR 0066 §4), so the same bytes as the
    /// `0123` grey pyramid. A pixel buffer that does not match the size reads as one white pixel.
    #[must_use]
    pub fn of_rgba(image: &WorkingImage) -> Self {
        let (width, height) = (image.width.max(1), image.height.max(1));
        if image.pixels.len() != width as usize * height as usize * 4 {
            return Self::white(1, 1);
        }
        let grey = image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                luminance(
                    over_white(p[0], p[3]),
                    over_white(p[1], p[3]),
                    over_white(p[2], p[3]),
                )
            })
            .collect();
        Self {
            width,
            height,
            grey,
        }
    }

    /// A white plane of `width` x `height`.
    #[must_use]
    pub fn white(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            grey: vec![255; width as usize * height as usize],
        }
    }

    /// The number of pixels.
    #[must_use]
    pub const fn pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// The grey at `(x, y)`, clamped to the plane's edge.
    #[must_use]
    pub fn at(&self, x: i64, y: i64) -> u8 {
        let x = clamp_index(x, self.width);
        let y = clamp_index(y, self.height);
        self.grey[y * self.width as usize + x]
    }

    /// The next level: each side halved (rounding down, at least 1) by a 2 x 2 integer box mean
    /// rounded half up, the last column or row repeated where a side is odd.
    #[must_use]
    pub fn halved(&self) -> Self {
        let (width, height) = ((self.width / 2).max(1), (self.height / 2).max(1));
        let mut grey = Vec::with_capacity(width as usize * height as usize);
        for y in 0..i64::from(height) {
            for x in 0..i64::from(width) {
                let (x0, y0) = (2 * x, 2 * y);
                let sum = u32::from(self.at(x0, y0))
                    + u32::from(self.at(x0 + 1, y0))
                    + u32::from(self.at(x0, y0 + 1))
                    + u32::from(self.at(x0 + 1, y0 + 1));
                grey.push(byte((sum + 2) / 4));
            }
        }
        Self {
            width,
            height,
            grey,
        }
    }

    /// The first level of the halving chain with at most `max_pixels` pixels (this plane itself
    /// when it is small enough).
    #[must_use]
    pub fn mip_at_most(&self, max_pixels: u64) -> std::borrow::Cow<'_, Self> {
        let mut level = std::borrow::Cow::Borrowed(self);
        while level.pixels() > max_pixels && (level.width > 1 || level.height > 1) {
            level = std::borrow::Cow::Owned(level.halved());
        }
        level
    }
}

/// One straight-alpha channel composited over white: the premultiplied value plus the white
/// that shows through, as `over_white_grey` reads a premultiplied colour.
#[must_use]
pub(crate) fn over_white(c: u8, a: u8) -> u8 {
    let premultiplied = (u32::from(c) * u32::from(a) + 127) / 255;
    byte(premultiplied + 255 - u32::from(a))
}

/// The grey of an opaque colour with the luminance weights of `over_white_grey`.
#[must_use]
pub(crate) fn luminance(r: u8, g: u8, b: u8) -> u8 {
    let grey =
        (13_933 * u32::from(r) + 46_871 * u32::from(g) + 4_732 * u32::from(b) + 32_768) >> 16;
    byte(grey)
}

fn clamp_index(i: i64, size: u32) -> usize {
    usize::try_from(i.clamp(0, i64::from(size.max(1)) - 1)).unwrap_or(0)
}

/// `value` as a byte, saturating.
pub(crate) fn byte(value: u32) -> u8 {
    u8::try_from(value.min(255)).unwrap_or(u8::MAX)
}

/// `value` clamped to a byte.
pub(crate) fn clamp_byte(value: i64) -> u8 {
    u8::try_from(value.clamp(0, 255)).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, pixels: Vec<u8>) -> WorkingImage {
        WorkingImage {
            width,
            height,
            pixels,
            original_size: (width, height),
            truncated: false,
            animated: false,
        }
    }

    #[test]
    fn transparent_reads_white_and_the_weights_are_fixed() {
        let p = GreyPlane::of_rgba(&image(
            5,
            1,
            [
                [0, 0, 0, 0],
                [0, 0, 0, 128],
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
            ]
            .concat(),
        ));
        assert_eq!(p.grey, vec![255, 127, 54, 182, 18]);
    }

    #[test]
    fn the_mip_is_halved_until_small_enough() {
        let p = GreyPlane::white(2_000, 1_000);
        let m = p.mip_at_most(1_000_000);
        assert_eq!((m.width, m.height), (1_000, 500));
        let same = GreyPlane::white(10, 10);
        assert!(matches!(
            same.mip_at_most(100),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_bad_buffer_reads_one_white_pixel() {
        let p = GreyPlane::of_rgba(&image(3, 3, vec![0; 5]));
        assert_eq!(p, GreyPlane::white(1, 1));
    }
}
