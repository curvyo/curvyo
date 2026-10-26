//! The decoder's pixels to the RGBA8 working image (`specs/0058-bitmap-import` criteria 7 and
//! 18): read in the decoder's own layout (the full picture is never converted to RGBA8, ADR 0017
//! spike correction 2), 16-bit values rounded to 8 bits, grey expanded, and an area-averaging box
//! downscale in integer arithmetic, alpha-weighted so transparent pixels lend no colour.

use crate::error::ImageError;

/// A decoder pixel layout this crate reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    L8,
    La8,
    Rgb8,
    Rgba8,
    L16,
    La16,
    Rgb16,
    Rgba16,
}

impl Layout {
    pub(crate) fn of(colour: image::ColorType) -> Option<Self> {
        Some(match colour {
            image::ColorType::L8 => Self::L8,
            image::ColorType::La8 => Self::La8,
            image::ColorType::Rgb8 => Self::Rgb8,
            image::ColorType::Rgba8 => Self::Rgba8,
            image::ColorType::L16 => Self::L16,
            image::ColorType::La16 => Self::La16,
            image::ColorType::Rgb16 => Self::Rgb16,
            image::ColorType::Rgba16 => Self::Rgba16,
            _ => return None,
        })
    }

    const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::L8 => 1,
            Self::La8 | Self::L16 => 2,
            Self::Rgb8 => 3,
            Self::Rgba8 | Self::La16 => 4,
            Self::Rgb16 => 6,
            Self::Rgba16 => 8,
        }
    }

    /// The RGBA8 of the pixel starting at `p`.
    fn rgba(self, p: &[u8]) -> [u8; 4] {
        // A 16-bit value in the decoder's native byte order, rounded to 8 bits (criterion 7).
        let w = |i: usize| {
            let v = u32::from(u16::from_ne_bytes([p[2 * i], p[2 * i + 1]]));
            // invariant: (65535 + 128) / 257 = 255.
            #[allow(clippy::cast_possible_truncation)]
            let byte = ((v + 128) / 257) as u8;
            byte
        };
        match self {
            Self::L8 => [p[0], p[0], p[0], 255],
            Self::La8 => [p[0], p[0], p[0], p[1]],
            Self::Rgb8 => [p[0], p[1], p[2], 255],
            Self::Rgba8 => [p[0], p[1], p[2], p[3]],
            Self::L16 => [w(0), w(0), w(0), 255],
            Self::La16 => [w(0), w(0), w(0), w(1)],
            Self::Rgb16 => [w(0), w(1), w(2), 255],
            Self::Rgba16 => [w(0), w(1), w(2), w(3)],
        }
    }
}

fn fallible_buffer(len: usize) -> Result<Vec<u8>, ImageError> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(len)
        .map_err(|_| ImageError::OutOfMemory)?;
    Ok(buffer)
}

/// `source` (`width` x `height` pixels in `layout`) as RGBA8 of `out_width` x `out_height`,
/// which is at most the source size.
pub(crate) fn to_working(
    source: &[u8],
    layout: Layout,
    (width, height): (usize, usize),
    (out_width, out_height): (usize, usize),
) -> Result<Vec<u8>, ImageError> {
    let bpp = layout.bytes_per_pixel();
    if source.len() < width * height * bpp {
        return Err(ImageError::Damaged);
    }
    let mut out = fallible_buffer(out_width * out_height * 4)?;
    if (out_width, out_height) == (width, height) {
        for p in source.chunks_exact(bpp).take(width * height) {
            out.extend_from_slice(&layout.rgba(p));
        }
        return Ok(out);
    }
    let columns = spans(width, out_width);
    let rows = spans(height, out_height);
    // Per output pixel of the current and the next output row: alpha-weighted R, G, B and the
    // weighted alpha, all in units of (source span) x (output span).
    let mut current = vec![[0_u64; 4]; out_width];
    let mut next = vec![[0_u64; 4]; out_width];
    let mut row_sums = vec![[0_u64; 4]; out_width];
    let total = (width * height) as u64;
    let mut out_row = 0;
    for (y, row) in rows.iter().enumerate() {
        row_sums.fill([0; 4]);
        let line = &source[y * width * bpp..(y + 1) * width * bpp];
        for (x, column) in columns.iter().enumerate() {
            let [r, g, b, a] = layout.rgba(&line[x * bpp..(x + 1) * bpp]);
            let a = u64::from(a);
            let premultiplied = [u64::from(r) * a, u64::from(g) * a, u64::from(b) * a, a];
            add(
                &mut row_sums[column.first],
                premultiplied,
                column.first_weight,
            );
            if column.second_weight > 0 {
                add(
                    &mut row_sums[column.first + 1],
                    premultiplied,
                    column.second_weight,
                );
            }
        }
        for (target, sum) in current.iter_mut().zip(&row_sums) {
            add(target, *sum, row.first_weight);
        }
        if row.second_weight > 0 {
            for (target, sum) in next.iter_mut().zip(&row_sums) {
                add(target, *sum, row.second_weight);
            }
        }
        let row_done = y + 1 == height || rows[y + 1].first != row.first;
        if row_done {
            for sums in &current {
                out.extend_from_slice(&average(*sums, total));
            }
            out_row += 1;
            std::mem::swap(&mut current, &mut next);
            next.fill([0; 4]);
        }
    }
    debug_assert_eq!(out_row, out_height);
    Ok(out)
}

fn add(target: &mut [u64; 4], value: [u64; 4], weight: u64) {
    for (t, v) in target.iter_mut().zip(value) {
        *t += v * weight;
    }
}

/// The straight RGBA8 of accumulated sums; `total` is the weight of one output pixel.
fn average(sums: [u64; 4], total: u64) -> [u8; 4] {
    let alpha = sums[3];
    let colour = |c: u64| {
        // invariant: an alpha-weighted mean of bytes is a byte.
        #[allow(clippy::cast_possible_truncation)]
        (c + alpha / 2)
            .checked_div(alpha)
            .map_or(0, |mean| mean as u8)
    };
    // invariant: a mean of bytes is a byte.
    #[allow(clippy::cast_possible_truncation)]
    let a = ((alpha + total / 2) / total) as u8;
    [colour(sums[0]), colour(sums[1]), colour(sums[2]), a]
}

/// Where source pixel `i` of `n` falls among `m <= n` output pixels: in scaled units, source pixel
/// `i` covers `[i * m, (i + 1) * m)` and output pixel `j` covers `[j * n, (j + 1) * n)`, so a
/// source pixel touches at most two output pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    first: usize,
    first_weight: u64,
    second_weight: u64,
}

fn spans(n: usize, m: usize) -> Vec<Span> {
    (0..n)
        .map(|i| {
            let (start, end) = (i * m, (i + 1) * m);
            let first = start / n;
            let boundary = (first + 1) * n;
            if end > boundary {
                Span {
                    first,
                    first_weight: (boundary - start) as u64,
                    second_weight: (end - boundary) as u64,
                }
            } else {
                Span {
                    first,
                    first_weight: m as u64,
                    second_weight: 0,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_constant_picture_stays_constant_at_any_scale() {
        let source = [10_u8, 200, 30].repeat(7 * 5);
        let out = to_working(&source, Layout::Rgb8, (7, 5), (3, 2)).unwrap();
        assert_eq!(out, [10, 200, 30, 255].repeat(6));
    }

    #[test]
    fn halving_averages_each_2x2_block() {
        let source = [0_u8, 100, 50, 150];
        let out = to_working(&source, Layout::L8, (2, 2), (1, 1)).unwrap();
        assert_eq!(out, [75, 75, 75, 255]);
    }

    #[test]
    fn transparent_pixels_lend_no_colour() {
        let source = [255_u8, 0, 0, 0, 0, 0, 255, 255];
        let out = to_working(&source, Layout::Rgba8, (2, 1), (1, 1)).unwrap();
        assert_eq!(out, [0, 0, 255, 128]);
    }

    #[test]
    fn sixteen_bits_round_to_eight() {
        let source: Vec<u8> = [0_u16, 257, 65535, 32896]
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        let out = to_working(&source, Layout::L16, (4, 1), (4, 1)).unwrap();
        assert_eq!(
            out,
            [
                0, 0, 0, 255, 1, 1, 1, 255, 255, 255, 255, 255, 128, 128, 128, 255
            ]
        );
    }

    #[test]
    fn spans_cover_each_output_pixel_with_the_full_weight() {
        for (n, m) in [(5, 3), (7, 2), (10, 10), (13, 1), (4472, 4000)] {
            let mut weight = vec![0_u64; m];
            for span in spans(n, m) {
                weight[span.first] += span.first_weight;
                if span.second_weight > 0 {
                    weight[span.first + 1] += span.second_weight;
                }
            }
            assert!(weight.iter().all(|&w| w == n as u64), "{n} -> {m}");
        }
    }
}
