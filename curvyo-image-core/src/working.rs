//! The working image (`specs/0058-bitmap-import` "Terms"): RGBA8, straight alpha, upright, at
//! most the working limits; and its premultiplied mip chain for the GPU upload (ADR 0017 §5).

/// The decoded picture the app draws and the Trace tool reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA8, top row first, straight (not premultiplied) alpha.
    pub pixels: Vec<u8>,
    /// The upright size of the file's own picture; larger than `width` x `height` when the
    /// working limit reduced it (criterion 18).
    pub original_size: (u32, u32),
    /// Whether the file is a JPEG cut off before its end; the grey rest is part of the picture
    /// (criterion 21).
    pub truncated: bool,
    /// Whether the file is an animated WebP of which only the first frame is here (criterion 6).
    pub animated: bool,
}

/// One level of a mip chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MipLevel {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA8, premultiplied alpha.
    pub pixels: Vec<u8>,
}

impl WorkingImage {
    /// Whether the working limit reduced the picture (criterion 18).
    #[must_use]
    pub fn is_reduced(&self) -> bool {
        self.original_size != (self.width, self.height)
    }

    /// The RGBA of pixel (`x`, `y`), or `None` outside.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        let p = self.pixels.get(at..at + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }

    /// The premultiplied mip chain down to 1 x 1: level 0 is this image premultiplied, every next
    /// level halves each side (rounding down, at least 1) with a 2x2 integer box, the last column
    /// or row repeated where a side is odd. Equal input gives equal bytes everywhere.
    #[must_use]
    pub fn premultiplied_mips(&self) -> Vec<MipLevel> {
        let mut level = MipLevel {
            width: self.width,
            height: self.height,
            pixels: self
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| {
                    let a = u16::from(p[3]);
                    let times = |c: u8| {
                        // invariant: (c * a + 127) / 255 <= 255.
                        #[allow(clippy::cast_possible_truncation)]
                        let value = ((u16::from(c) * a + 127) / 255) as u8;
                        value
                    };
                    [times(p[0]), times(p[1]), times(p[2]), p[3]]
                })
                .collect(),
        };
        let mut chain = Vec::new();
        while level.width > 1 || level.height > 1 {
            let next = half(&level);
            chain.push(level);
            level = next;
        }
        chain.push(level);
        chain
    }

    /// The alpha of level `level` of the mip chain, one byte per pixel in the order and halving
    /// of [`Self::reduced_over_white`] (the canvas look keeps a picture's transparency, ADR 0073
    /// §8); `None` when the picture is opaque.
    #[must_use]
    pub fn reduced_alpha(&self, level: u32) -> Option<Vec<u8>> {
        if self.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255) {
            return None;
        }
        let (mut w, mut h) = (self.width as usize, self.height as usize);
        let mut alpha: Vec<u8> = self
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3])
            .collect();
        for _ in 0..level {
            if w == 1 && h == 1 {
                break;
            }
            let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
            let mut next = Vec::with_capacity(nw * nh);
            for y in 0..nh {
                let (y0, y1) = ((2 * y).min(h - 1), (2 * y + 1).min(h - 1));
                for x in 0..nw {
                    let (x0, x1) = ((2 * x).min(w - 1), (2 * x + 1).min(w - 1));
                    let at = |xx: usize, yy: usize| u16::from(alpha[yy * w + xx]);
                    let sum = at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1);
                    // invariant: a sum of four bytes plus 2, divided by 4, is a byte.
                    #[allow(clippy::cast_possible_truncation)]
                    next.push(((sum + 2) / 4) as u8);
                }
            }
            (alpha, w, h) = (next, nw, nh);
        }
        Some(alpha)
    }

    /// Level `level` of the premultiplied mip chain composited over white, as an opaque picture
    /// (a reduced input for the adjustments of a coarse preview, ADR 0073 §8); level 0 is this
    /// picture over white.
    #[must_use]
    pub fn reduced_over_white(&self, level: u32) -> Self {
        let mut mip = MipLevel {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
        };
        for pixel in mip.pixels.as_chunks_mut::<4>().0 {
            let a = u16::from(pixel[3]);
            for c in &mut pixel[..3] {
                // invariant: (c * a + 127) / 255 <= 255.
                #[allow(clippy::cast_possible_truncation)]
                let value = ((u16::from(*c) * a + 127) / 255) as u8;
                *c = value;
            }
        }
        for _ in 0..level {
            if mip.width == 1 && mip.height == 1 {
                break;
            }
            mip = half(&mip);
        }
        for pixel in mip.pixels.as_chunks_mut::<4>().0 {
            let white = 255 - pixel[3];
            for c in &mut pixel[..3] {
                *c = c.saturating_add(white);
            }
            pixel[3] = 255;
        }
        Self {
            width: mip.width,
            height: mip.height,
            pixels: mip.pixels,
            original_size: self.original_size,
            truncated: self.truncated,
            animated: self.animated,
        }
    }
}

fn half(level: &MipLevel) -> MipLevel {
    let (w, h) = (level.width as usize, level.height as usize);
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut pixels = Vec::with_capacity(nw * nh * 4);
    for y in 0..nh {
        let (y0, y1) = ((2 * y).min(h - 1), (2 * y + 1).min(h - 1));
        for x in 0..nw {
            let (x0, x1) = ((2 * x).min(w - 1), (2 * x + 1).min(w - 1));
            for c in 0..4 {
                let at = |xx: usize, yy: usize| u16::from(level.pixels[(yy * w + xx) * 4 + c]);
                let sum = at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1);
                // invariant: a sum of four bytes plus 2, divided by 4, is a byte.
                #[allow(clippy::cast_possible_truncation)]
                pixels.push(((sum + 2) / 4) as u8);
            }
        }
    }
    MipLevel {
        // invariant: halves of `u32` sides fit a `u32`.
        #[allow(clippy::cast_possible_truncation)]
        width: nw as u32,
        #[allow(clippy::cast_possible_truncation)]
        height: nh as u32,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mip_chain_ends_at_one_pixel_and_premultiplies() {
        let image = WorkingImage {
            width: 5,
            height: 2,
            pixels: [255, 255, 255, 128].repeat(10),
            original_size: (5, 2),
            truncated: false,
            animated: false,
        };
        let chain = image.premultiplied_mips();
        let sizes: Vec<_> = chain.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(sizes, [(5, 2), (2, 1), (1, 1)]);
        assert_eq!(&chain[0].pixels[..4], &[128, 128, 128, 128]);
        assert_eq!(&chain[2].pixels[..4], &[128, 128, 128, 128]);
    }

    #[test]
    fn a_reduced_picture_is_opaque_over_white_and_halved() {
        let image = WorkingImage {
            width: 4,
            height: 2,
            pixels: [0, 0, 0, 128].repeat(8),
            original_size: (4, 2),
            truncated: false,
            animated: false,
        };
        let full = image.reduced_over_white(0);
        assert_eq!(&full.pixels[..4], &[127, 127, 127, 255]);
        let half = image.reduced_over_white(1);
        assert_eq!((half.width, half.height), (2, 1));
        assert_eq!(&half.pixels[..4], &[127, 127, 127, 255]);
        assert_eq!(image.reduced_over_white(9).width, 1);
    }

    /// Criterion 15 (ADR 0073 §8): the look's alpha is the alpha of the reduced level, the same
    /// halving as the colour; an opaque picture has none.
    #[test]
    fn the_reduced_alpha_follows_the_halving() {
        let mut pixels = Vec::new();
        for a in [0_u8, 255, 255, 255, 0, 0, 255, 255] {
            pixels.extend_from_slice(&[10, 20, 30, a]);
        }
        let image = WorkingImage {
            width: 4,
            height: 2,
            pixels,
            original_size: (4, 2),
            truncated: false,
            animated: false,
        };
        assert_eq!(
            image.reduced_alpha(0).unwrap(),
            [0, 255, 255, 255, 0, 0, 255, 255]
        );
        let half = image.reduced_alpha(1).unwrap();
        assert_eq!(half, [64, 255]);
        assert_eq!(
            image.reduced_alpha(9).unwrap(),
            [160],
            "(64 + 255 + 64 + 255 + 2) / 4"
        );
        let opaque = WorkingImage {
            pixels: [1, 2, 3, 255].repeat(8),
            ..image
        };
        assert_eq!(opaque.reduced_alpha(1), None);
    }
}
