//! The grey value at a dot (ADR 0068 §3, `specs/0123` criteria 6, 7, 24): a box pyramid of an
//! image's grey read bilinearly, or a gradient's colour over white.

use curvyo_document_core::{PlacedGradient, Point, over_white_grey};
use curvyo_image_core::{Adjusted, GreyPlane, WorkingImage};

/// The grey pyramid of one working image, level 0 at full size down to 1 x 1; built once per
/// asset on first raster use (ADR 0068 §3, criterion 37 g).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreyPyramid {
    levels: Vec<GreyPlane>,
}

impl GreyPyramid {
    /// The pyramid of `image`: each pixel composited over white (straight alpha) and weighted
    /// by the luminance weights of `over_white_grey`.
    #[must_use]
    pub fn of(image: &WorkingImage) -> Self {
        Self::of_plane(GreyPlane::of_rgba(image))
    }

    /// The pyramid of an adjusted image (ADR 0073 §7): `image` itself when nothing changed it.
    #[must_use]
    pub fn of_adjusted(image: &WorkingImage, adjusted: &Adjusted) -> Self {
        match adjusted {
            Adjusted::Unchanged => Self::of(image),
            Adjusted::Rgba(rgba) => Self::of(rgba),
            Adjusted::Grey(plane) => Self::of_plane(plane.clone()),
        }
    }

    /// The pyramid with `plane` as its full-size level.
    #[must_use]
    pub fn of_plane(plane: GreyPlane) -> Self {
        let mut levels = vec![plane];
        while let Some(last) = levels.last()
            && (last.width > 1 || last.height > 1)
        {
            let next = last.halved();
            levels.push(next);
        }
        Self { levels }
    }

    /// The full-size level.
    #[must_use]
    pub fn base(&self) -> &GreyPlane {
        &self.levels[0]
    }
}

/// The grey of an image's dots: the pyramid level of the dot size, read bilinearly through the
/// inverse of the image's frame.
#[derive(Debug)]
pub struct ImageSampler<'a> {
    plane: &'a GreyPlane,
    origin: Point,
    /// Page mm to unit square: `u = (d · ux)`, `v = (d · vx)` for `d = p − origin`.
    ux: (f64, f64),
    vx: (f64, f64),
}

impl<'a> ImageSampler<'a> {
    /// The sampler of `pyramid` placed in `frame` (the picture's top-left, top-right,
    /// bottom-right and bottom-left in page mm) for dots of `interval_mm`; `None` for a frame of
    /// no area.
    #[must_use]
    pub fn new(pyramid: &'a GreyPyramid, frame: &[Point; 4], interval_mm: f64) -> Option<Self> {
        let base = pyramid.base();
        let pitch = pixel_pitch_mm(frame, base.width, base.height);
        let mut level = 0;
        while level + 1 < pyramid.levels.len() && pitch * f64::from(2_u32 << level) <= interval_mm {
            level += 1;
        }
        let a = (frame[1].x - frame[0].x, frame[1].y - frame[0].y);
        let b = (frame[3].x - frame[0].x, frame[3].y - frame[0].y);
        let det = a.0 * b.1 - a.1 * b.0;
        if !det.is_finite() || det == 0.0 {
            return None;
        }
        Some(Self {
            plane: &pyramid.levels[level],
            origin: frame[0],
            ux: (b.1 / det, -b.0 / det),
            vx: (-a.1 / det, a.0 / det),
        })
    }

    /// The grey at page point `p` (mm): the bilinear blend of the four nearest pixels, weights in
    /// steps of 1/256, edges clamped.
    #[must_use]
    pub fn grey_at(&self, p: Point) -> u8 {
        let (dx, dy) = (p.x - self.origin.x, p.y - self.origin.y);
        let u = dx * self.ux.0 + dy * self.ux.1;
        let v = dx * self.vx.0 + dy * self.vx.1;
        let qx = quantised(u * f64::from(self.plane.width) - 0.5);
        let qy = quantised(v * f64::from(self.plane.height) - 0.5);
        let (ix, fx) = (qx >> 8, u32::try_from(qx & 255).unwrap_or(0));
        let (iy, fy) = (qy >> 8, u32::try_from(qy & 255).unwrap_or(0));
        let at = |x: i64, y: i64| u32::from(self.plane.at(x, y));
        let top = at(ix, iy) * (256 - fx) + at(ix + 1, iy) * fx;
        let bottom = at(ix, iy + 1) * (256 - fx) + at(ix + 1, iy + 1) * fx;
        byte((top * (256 - fy) + bottom * fy + 32_768) >> 16)
    }
}

/// The larger page size of one source pixel along either image axis, in mm.
#[must_use]
pub fn pixel_pitch_mm(frame: &[Point; 4], width: u32, height: u32) -> f64 {
    let side = |a: Point, b: Point| ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)).sqrt();
    let across = side(frame[0], frame[1]) / f64::from(width.max(1));
    let down = side(frame[0], frame[3]) / f64::from(height.max(1));
    across.max(down)
}

/// The grey of a gradient-filled shape at page point `p` (mm): the gradient's colour over white
/// (ADR 0066 §4), no resampling.
#[must_use]
pub fn gradient_grey(gradient: &PlacedGradient, p: Point) -> u8 {
    over_white_grey(gradient.colour_at(p))
}

/// `value` in steps of 1/256, rounded to the nearest; saturating far outside the image.
fn quantised(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; the clamp at the edge handles any far value
    let q = (value * 256.0).round() as i64;
    q
}

fn byte(value: u32) -> u8 {
    u8::try_from(value.min(255)).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> WorkingImage {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&pixel(x, y));
            }
        }
        WorkingImage {
            width,
            height,
            pixels,
            original_size: (width, height),
            truncated: false,
            animated: false,
        }
    }

    fn frame(x: f64, y: f64, w: f64, h: f64) -> [Point; 4] {
        [
            Point::new(x, y),
            Point::new(x + w, y),
            Point::new(x + w, y + h),
            Point::new(x, y + h),
        ]
    }

    #[test]
    fn transparent_reads_white_and_half_transparent_black_reads_mid_grey() {
        let p = GreyPyramid::of(&image(2, 1, |x, _| {
            if x == 0 { [0, 0, 0, 0] } else { [0, 0, 0, 128] }
        }));
        assert_eq!(p.base().grey, vec![255, 127]);
    }

    #[test]
    fn the_image_grey_is_over_white_grey_for_every_alpha() {
        use curvyo_document_core::PremultipliedRgba8;
        let colours: Vec<[u8; 4]> = (0..=255_u8)
            .step_by(5)
            .flat_map(|a| [[a, 255 - a, a / 3, a], [200, 13, 77, 255 - a], [9, 9, 9, a]])
            .collect();
        let p = GreyPyramid::of(&image(u32::try_from(colours.len()).unwrap(), 1, |x, _| {
            colours[x as usize]
        }));
        for (c, &g) in colours.iter().zip(&p.base().grey) {
            let a = u32::from(c[3]);
            let pre = |v: u8| u8::try_from((u32::from(v) * a + 127) / 255).unwrap();
            let expected = over_white_grey(PremultipliedRgba8 {
                r: pre(c[0]),
                g: pre(c[1]),
                b: pre(c[2]),
                a: c[3],
            });
            assert_eq!(g, expected, "{c:?}");
        }
    }

    #[test]
    fn luminance_uses_the_fixed_weights() {
        let p = GreyPyramid::of(&image(3, 1, |x, _| match x {
            0 => [255, 0, 0, 255],
            1 => [0, 255, 0, 255],
            _ => [0, 0, 255, 255],
        }));
        assert_eq!(p.base().grey, vec![54, 182, 18]);
    }

    #[test]
    fn the_pyramid_halves_down_to_one_pixel() {
        let p = GreyPyramid::of(&image(5, 3, |_, _| [100, 100, 100, 255]));
        let sizes: Vec<(u32, u32)> = p.levels.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(sizes, vec![(5, 3), (2, 1), (1, 1)]);
        assert!(p.levels.iter().all(|l| l.grey.iter().all(|&g| g == 100)));
    }

    #[test]
    fn a_fine_source_is_box_averaged_at_the_dot_size() {
        // A 1 mm wide image of 64 px alternating black and white columns: 1/64 mm per pixel.
        let p = GreyPyramid::of(&image(64, 64, |x, _| {
            if x % 2 == 0 { [0, 0, 0, 255] } else { [255; 4] }
        }));
        let s = ImageSampler::new(&p, &frame(0.0, 0.0, 1.0, 1.0), 0.1).unwrap();
        // 2^n / 64 <= 0.1: n = 2, 16 x 16 level, every pixel the mean of black and white.
        assert_eq!(s.plane.width, 16);
        let g = s.grey_at(Point::new(0.33, 0.5));
        assert!((126..=129).contains(&g), "{g}");
    }

    #[test]
    fn a_coarse_source_is_bilinear_between_pixel_centres() {
        let p = GreyPyramid::of(&image(
            2,
            1,
            |x, _| if x == 0 { [0, 0, 0, 255] } else { [255; 4] },
        ));
        let s = ImageSampler::new(&p, &frame(0.0, 0.0, 2.0, 1.0), 0.1).unwrap();
        assert_eq!(s.grey_at(Point::new(0.5, 0.5)), 0, "pixel centre");
        assert_eq!(s.grey_at(Point::new(1.0, 0.5)), 128, "half way");
        assert_eq!(s.grey_at(Point::new(1.5, 0.5)), 255);
        assert_eq!(s.grey_at(Point::new(0.1, 0.5)), 0, "clamped at the edge");
    }

    #[test]
    fn a_mirrored_frame_reads_the_picture_mirrored() {
        let p = GreyPyramid::of(&image(
            2,
            1,
            |x, _| if x == 0 { [0, 0, 0, 255] } else { [255; 4] },
        ));
        let mut f = frame(0.0, 0.0, 2.0, 1.0);
        f.swap(0, 1);
        f.swap(2, 3);
        let s = ImageSampler::new(&p, &f, 0.1).unwrap();
        assert_eq!(s.grey_at(Point::new(1.5, 0.5)), 0, "black on the right");
    }
}
