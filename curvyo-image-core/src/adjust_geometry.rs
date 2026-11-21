//! The geometry steps of the adjustments on one plane (ADR 0073 §4; `specs/0135` criteria 5,
//! 14): Crop to a pixel rectangle and Resize by an integer area average.

use crate::adjust_plane::GreyPlane;

/// The pixel rectangle `(x, y, width, height)` that a crop of `left`, `top`, `width` and `height`
/// percent keeps of a `w` x `h` picture: `floor(left · w / 100)` to `ceil((left + width) · w /
/// 100)`, at least one pixel, within the picture.
#[must_use]
pub fn crop_rect(w: u32, h: u32, pct: [u8; 4]) -> (u32, u32, u32, u32) {
    let [left, top, width, height] = pct.map(u64::from);
    let span = |size: u32, from: u64, extent: u64| {
        let size = u64::from(size.max(1));
        let start = (from.min(100) * size / 100).min(size - 1);
        let end = ((from + extent).min(100) * size)
            .div_ceil(100)
            .clamp(start + 1, size);
        // invariant: both are at most `size`, a `u32`.
        (
            u32::try_from(start).unwrap_or(0),
            u32::try_from(end - start).unwrap_or(1),
        )
    };
    let (x, cw) = span(w, left, width);
    let (y, ch) = span(h, top, height);
    (x, y, cw, ch)
}

/// The size a resize to `width_px` gives a `w` x `h` picture: never wider than `w` (it only
/// reduces, criterion 14), at least one pixel, the height following the ratio rounded half up.
#[must_use]
pub fn resized_size(w: u32, h: u32, width_px: u32) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    let width = width_px.clamp(1, w);
    let height = (2 * u64::from(h) * u64::from(width) + u64::from(w)) / (2 * u64::from(w));
    (width, u32::try_from(height.max(1)).unwrap_or(1))
}

/// `plane` cropped to the pixel rectangle of `pct` (see [`crop_rect`]).
#[must_use]
pub fn crop_plane(plane: &GreyPlane, pct: [u8; 4]) -> GreyPlane {
    let (x, y, w, h) = crop_rect(plane.width, plane.height, pct);
    let stride = plane.width as usize;
    let mut grey = Vec::with_capacity(w as usize * h as usize);
    for row in y as usize..(y + h) as usize {
        let start = row * stride + x as usize;
        grey.extend_from_slice(&plane.grey[start..start + w as usize]);
    }
    GreyPlane {
        width: w,
        height: h,
        grey,
    }
}

/// `plane` resized to `width_px` (see [`resized_size`]): a separable area average, rows first,
/// each output pixel the mean of the source span it covers, spans in 1/65536 pixel with integer
/// weights, rounded half up. The same size is the plane itself.
#[must_use]
pub fn resize_plane(plane: &GreyPlane, width_px: u32) -> GreyPlane {
    let (w, h) = resized_size(plane.width, plane.height, width_px);
    if (w, h) == (plane.width, plane.height) {
        return plane.clone();
    }
    let across = spans(plane.width, w);
    let down = spans(plane.height, h);
    let src_w = plane.width as usize;
    let mut rows = vec![0_u8; w as usize * plane.height as usize];
    for (y, out) in rows.chunks_exact_mut(w as usize).enumerate() {
        let line = &plane.grey[y * src_w..(y + 1) * src_w];
        for (slot, span) in out.iter_mut().zip(&across) {
            *slot = span.mean(|i| line[i]);
        }
    }
    let mut grey = vec![0_u8; w as usize * h as usize];
    for (y, span) in down.iter().enumerate() {
        for x in 0..w as usize {
            grey[y * w as usize + x] = span.mean(|i| rows[i * w as usize + x]);
        }
    }
    GreyPlane {
        width: w,
        height: h,
        grey,
    }
}

/// The source pixels one output pixel covers and their weights in 1/65536 pixel.
#[derive(Debug, Clone)]
struct Span {
    first: usize,
    weights: Vec<u64>,
    total: u64,
}

impl Span {
    fn mean(&self, value: impl Fn(usize) -> u8) -> u8 {
        let sum: u64 = self
            .weights
            .iter()
            .enumerate()
            .map(|(k, &wt)| wt * u64::from(value(self.first + k)))
            .sum();
        u8::try_from((2 * sum + self.total) / (2 * self.total.max(1))).unwrap_or(u8::MAX)
    }
}

/// The spans of `to` output pixels over `from` source pixels.
fn spans(from: u32, to: u32) -> Vec<Span> {
    const ONE: u64 = 1 << 16;
    let (from, to) = (u64::from(from.max(1)), u64::from(to.max(1)));
    (0..to)
        .map(|i| {
            let start = i * from * ONE / to;
            let end = (i + 1) * from * ONE / to;
            let first = start / ONE;
            let last = (end - 1) / ONE;
            let weights = (first..=last)
                .map(|j| end.min((j + 1) * ONE) - start.max(j * ONE))
                .collect();
            Span {
                first: usize::try_from(first).unwrap_or(0),
                weights,
                total: end - start,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(width: u32, height: u32) -> GreyPlane {
        GreyPlane {
            width,
            height,
            grey: (0..width * height)
                .map(|i| u8::try_from(i % 256).unwrap())
                .collect(),
        }
    }

    #[test]
    fn a_crop_floors_its_start_and_ceils_its_end() {
        assert_eq!(crop_rect(10, 10, [0, 0, 100, 100]), (0, 0, 10, 10));
        assert_eq!(crop_rect(10, 10, [15, 25, 30, 50]), (1, 2, 4, 6));
        assert_eq!(
            crop_rect(3, 3, [99, 99, 1, 1]),
            (2, 2, 1, 1),
            "at least one pixel"
        );
        let cropped = crop_plane(&plane(10, 10), [20, 20, 60, 60]);
        assert_eq!((cropped.width, cropped.height), (6, 6));
        assert_eq!(cropped.grey[0], 22);
        assert_eq!(cropped.grey[7], 33);
    }

    #[test]
    fn resize_only_reduces_and_keeps_the_ratio() {
        assert_eq!(resized_size(100, 50, 400), (100, 50), "no enlargement");
        assert_eq!(resized_size(100, 50, 33), (33, 17));
        assert_eq!(resize_plane(&plane(8, 8), 8), plane(8, 8));
    }

    #[test]
    fn a_half_resize_is_the_2x2_mean() {
        let p = GreyPlane {
            width: 4,
            height: 2,
            grey: vec![0, 10, 100, 101, 20, 30, 200, 200],
        };
        let half = resize_plane(&p, 2);
        assert_eq!((half.width, half.height), (2, 1));
        // Each pass rounds half up: rows 5, 25 and 101, 200, then the columns 15 and 151.
        assert_eq!(half.grey, vec![15, 151]);
    }

    #[test]
    fn a_third_resize_weighs_the_partial_pixels() {
        let p = GreyPlane {
            width: 3,
            height: 1,
            grey: vec![0, 90, 180],
        };
        let two = resize_plane(&p, 2);
        // Spans 1.5 px: (0 + 90 / 2) / 1.5 = 30 and (90 / 2 + 180) / 1.5 = 150.
        assert_eq!(two.grey, vec![30, 150]);
    }
}
