//! The fixed tiling of a picture for a model with a static input size (ADR 0075 §2): tiles of
//! [`TILE`] px overlapping by [`TILE_OVERLAP`] px on each side, in row-major order, each output
//! pixel taken from the tile whose core holds it; the tiles' results put back together by index.

use curvyo_image_core::WorkingImage;

use crate::error::AiError;

/// The side of a model input tile in pixels.
pub const TILE: u32 = 256;

/// The overlap of neighbouring tiles on each side in pixels.
pub const TILE_OVERLAP: u32 = 16;

/// One tile: where it reads the picture (its top-left, may be outside: the edge repeats) and
/// the core it writes, in input pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tile {
    /// The input pixel of the tile's top-left corner.
    pub origin: (i64, i64),
    /// The core: left, top, width and height in input pixels.
    pub core: (u32, u32, u32, u32),
}

/// The tiles of a picture of `width` x `height`, row-major.
pub(crate) fn tiles(width: u32, height: u32) -> Vec<Tile> {
    let step = TILE - 2 * TILE_OVERLAP;
    let mut out = Vec::new();
    let mut y = 0;
    while y < height {
        let ch = step.min(height - y);
        let mut x = 0;
        while x < width {
            let cw = step.min(width - x);
            out.push(Tile {
                origin: (
                    i64::from(x) - i64::from(TILE_OVERLAP),
                    i64::from(y) - i64::from(TILE_OVERLAP),
                ),
                core: (x, y, cw, ch),
            });
            x += step;
        }
        y += step;
    }
    out
}

/// The number of tiles of a picture of `width` x `height` (ADR 0078 §2).
#[must_use]
pub fn tile_count(width: u32, height: u32) -> u32 {
    let step = TILE - 2 * TILE_OVERLAP;
    width.div_ceil(step) * height.div_ceil(step)
}

/// One tile's result: the core of tile `index` in output pixels, RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileOutput {
    /// The tile's index in row-major order.
    pub index: u32,
    /// The core's width in output pixels.
    pub width: u32,
    /// The core's height in output pixels.
    pub height: u32,
    /// The core's pixels, RGBA.
    pub rgba: Vec<u8>,
}

/// The picture of `width` x `height` output pixels from its tiles in any order, each placed by
/// its index (ADR 0078 §2). The grid is read from tile 0, whose core is a whole step or the
/// whole picture on each axis.
///
/// # Errors
/// [`AiError::Invalid`] for a missing, doubled or wrongly sized tile, an index out of range, or
/// pixels that do not match a tile's size.
pub fn assemble(width: u32, height: u32, tiles: Vec<TileOutput>) -> Result<WorkingImage, AiError> {
    let invalid =
        |why: &str| AiError::Invalid(format!("the tiles do not make up the picture: {why}"));
    let first = tiles
        .iter()
        .find(|t| t.index == 0)
        .ok_or_else(|| invalid("tile 0 is missing"))?;
    let (sx, sy) = (first.width, first.height);
    if sx == 0 || sy == 0 || width == 0 || height == 0 {
        return Err(invalid("an empty tile"));
    }
    let (columns, rows) = (width.div_ceil(sx), height.div_ceil(sy));
    let count = u64::from(columns) * u64::from(rows);
    if tiles.len() as u64 != count {
        return Err(invalid("a tile is missing or extra"));
    }
    let stride = width as usize * 4;
    let mut pixels = vec![0_u8; stride * height as usize];
    let mut seen = vec![false; tiles.len()];
    for tile in tiles {
        let index = tile.index as usize;
        if index >= seen.len() || seen[index] {
            return Err(invalid("an index out of range or twice"));
        }
        seen[index] = true;
        let (column, row) = (tile.index % columns, tile.index / columns);
        let (x0, y0) = (column * sx, row * sy);
        let (w, h) = (sx.min(width - x0), sy.min(height - y0));
        if (tile.width, tile.height) != (w, h) || tile.rgba.len() != w as usize * h as usize * 4 {
            return Err(invalid("a tile of the wrong size"));
        }
        let line = w as usize * 4;
        for (y, source) in tile.rgba.chunks_exact(line).enumerate() {
            let at = (y0 as usize + y) * stride + x0 as usize * 4;
            pixels[at..at + line].copy_from_slice(source);
        }
    }
    Ok(WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cores_cover_the_picture_once() {
        for (w, h) in [(1, 1), (224, 224), (225, 10), (500, 700)] {
            let mut seen = vec![0_u8; (w * h) as usize];
            for tile in tiles(w, h) {
                let (x0, y0, cw, ch) = tile.core;
                assert_eq!(tile.origin, (i64::from(x0) - 16, i64::from(y0) - 16));
                for y in y0..y0 + ch {
                    for x in x0..x0 + cw {
                        seen[(y * w + x) as usize] += 1;
                    }
                }
            }
            assert!(seen.iter().all(|&n| n == 1), "{w} x {h}");
        }
    }
}
