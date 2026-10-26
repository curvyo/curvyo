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
}
