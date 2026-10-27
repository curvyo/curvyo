//! The embedded images of an SVG import (`specs/0024-svg-import-export` criteria 100 to 104): the
//! bytes identified and decoded by `curvyo-image-core` as `0058` does, framed by the file's
//! viewport and `preserveAspectRatio`, stored once per distinct file within the project's room.

use std::collections::BTreeMap;
use std::sync::Arc;

use curvyo_document_core::AssetHash;
use curvyo_image_core::{WorkingImage, decode_working, probe};
use curvyo_svg_core::SvgImage;
use kurbo::{Affine, Point, Rect};

/// The assets of one import.
#[derive(Debug, Default)]
pub(crate) struct ImportAssets {
    /// The distinct files, in document order.
    pub(crate) bytes: Vec<Arc<[u8]>>,
    /// Their decoded working images.
    pub(crate) working: Vec<(AssetHash, Arc<WorkingImage>)>,
    /// The asset number of each file and its picture size in pixels.
    known: BTreeMap<AssetHash, (usize, (u32, u32))>,
    /// The bytes the project may still take (criterion 103).
    room: u64,
}

impl ImportAssets {
    /// Assets with `room` bytes left in the project.
    pub(crate) fn new(room: u64) -> Self {
        Self {
            room,
            ..Self::default()
        }
    }

    /// The asset number and the frame (unit square to the file's millimetres) of `image`, or
    /// `None` when it is ignored (criterion 103): not PNG, JPEG or WebP, damaged or cut off, over
    /// the ceilings, or past the project's room.
    pub(crate) fn add(&mut self, image: &SvgImage) -> Option<(usize, Affine)> {
        let hash = AssetHash::of(&image.bytes);
        let (asset, pixels) = if let Some(found) = self.known.get(&hash) {
            *found
        } else {
            {
                probe(&image.bytes).ok()?;
                let working = decode_working(&image.bytes).ok()?;
                if working.truncated {
                    return None;
                }
                let size = image.bytes.len() as u64;
                if size > self.room {
                    return None;
                }
                self.room -= size;
                let asset = self.bytes.len();
                let pixels = working.original_size;
                self.bytes.push(Arc::from(image.bytes.as_slice()));
                self.working.push((hash.clone(), Arc::new(working)));
                self.known.insert(hash, (asset, pixels));
                (asset, pixels)
            }
        };
        Some((asset, frame(image, pixels)?))
    }
}

/// The picture's frame (criterion 101): the viewport from `x`, `y`, `width`, `height` (the pixel
/// size where `auto`), the picture placed in it by the aspect rule (`slice` shows the whole
/// picture at the covering scale), then mapped by the image's matrix.
fn frame(image: &SvgImage, (pw, ph): (u32, u32)) -> Option<Affine> {
    let (pw, ph) = (f64::from(pw), f64::from(ph));
    if pw <= 0.0 || ph <= 0.0 {
        return None;
    }
    let (x, y) = image.origin;
    let (vw, vh) = (image.size.0.unwrap_or(pw), image.size.1.unwrap_or(ph));
    let (fx, fy, fw, fh) = match image.aspect.align {
        None => (x, y, vw, vh),
        Some((ax, ay)) => {
            let (sx, sy) = (vw / pw, vh / ph);
            let scale = if image.aspect.slice {
                sx.max(sy)
            } else {
                sx.min(sy)
            };
            let (fw, fh) = (pw * scale, ph * scale);
            (x + (vw - fw) * ax, y + (vh - fh) * ay, fw, fh)
        }
    };
    let map = image.matrix * Affine::translate((fx, fy)) * Affine::scale_non_uniform(fw, fh);
    let finite = map.as_coeffs().iter().all(|v| v.is_finite());
    (finite && map.determinant() != 0.0).then_some(map)
}

/// The bounds of a frame in the file's millimetres.
pub(crate) fn frame_bounds(map: Affine) -> Rect {
    let corners =
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| map * Point::new(x, y));
    corners
        .iter()
        .fold(Rect::from_points(corners[0], corners[0]), |r, p| {
            r.union_pt(*p)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_svg_core::ImageAspect;

    fn image(align: Option<(f64, f64)>, slice: bool) -> SvgImage {
        SvgImage {
            bytes: Vec::new(),
            origin: (0.0, 0.0),
            size: (Some(50.0), Some(50.0)),
            aspect: ImageAspect { align, slice },
            matrix: Affine::IDENTITY,
        }
    }

    /// Criterion 101's test: a 200 x 100 picture in a 50 x 50 viewport.
    #[test]
    fn ac101_aspect_rules() {
        let meet = frame(&image(Some((0.5, 0.5)), false), (200, 100)).map(kurbo::Affine::as_coeffs);
        assert_eq!(meet, Some([50.0, 0.0, 0.0, 25.0, 0.0, 12.5]));
        let none = frame(&image(None, false), (200, 100)).map(kurbo::Affine::as_coeffs);
        assert_eq!(none, Some([50.0, 0.0, 0.0, 50.0, 0.0, 0.0]));
        let slice = frame(&image(Some((0.0, 0.0)), true), (200, 100)).map(kurbo::Affine::as_coeffs);
        assert_eq!(slice, Some([100.0, 0.0, 0.0, 50.0, 0.0, 0.0]));
    }
}
