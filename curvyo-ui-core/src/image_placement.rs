//! Where an imported image lands (`specs/0058-bitmap-import` criteria 4, 5, 9 to 12): its
//! natural size from pixels and density, scaled down to fit the document, centred on the view
//! centre and pulled inside the document, or centred on a drop point as it is; several files
//! step 10 mm right and down. Numbers in, a frame out.

use curvyo_document_core::{Affine, DocumentSize, Point};

/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;

/// The step between the centres of several images of one import (criterion 5).
pub const STAGGER_MM: f64 = 10.0;

/// Where the import aims.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageTarget {
    /// The centre of the visible canvas: the image is pulled inside the document (criterion 12).
    ViewCentre(Point),
    /// The point a file was dropped on: no clamp (criterion 4).
    Drop(Point),
}

/// The frame of an imported image and the factor the fit applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePlacement {
    /// The unit square to document millimetres.
    pub placement: Affine,
    /// The fit factor, 1 when the natural size fits the document (criterion 10).
    pub fit: f64,
}

/// The natural size in millimetres of `pixels` (upright width, height) at `dpi` (x, y).
#[must_use]
pub fn natural_size_mm(pixels: (u32, u32), dpi: (f64, f64)) -> (f64, f64) {
    (
        f64::from(pixels.0) / dpi.0 * MM_PER_INCH,
        f64::from(pixels.1) / dpi.1 * MM_PER_INCH,
    )
}

/// The frame of the `index`-th image (0 for the first) of `pixels` at `dpi` imported into a
/// document of `page` aimed at `target`.
#[must_use]
pub fn place_image(
    pixels: (u32, u32),
    dpi: (f64, f64),
    page: DocumentSize,
    target: ImageTarget,
    index: usize,
) -> ImagePlacement {
    let (natural_w, natural_h) = natural_size_mm(pixels, dpi);
    let (page_w, page_h) = (page.width.as_mm(), page.height.as_mm());
    let fit = (page_w / natural_w).min(page_h / natural_h).min(1.0);
    let (w, h) = (natural_w * fit, natural_h * fit);
    #[allow(clippy::cast_precision_loss)]
    let step = STAGGER_MM * index as f64;
    let centre = match target {
        ImageTarget::ViewCentre(centre) => {
            let pull = |c: f64, size: f64, page: f64| {
                if size <= page {
                    c.clamp(size / 2.0, page - size / 2.0)
                } else {
                    c
                }
            };
            Point::new(
                pull(centre.x + step, w, page_w),
                pull(centre.y + step, h, page_h),
            )
        }
        ImageTarget::Drop(point) => Point::new(point.x + step, point.y + step),
    };
    ImagePlacement {
        placement: Affine::new(w, 0.0, 0.0, h, centre.x - w / 2.0, centre.y - h / 2.0),
        fit,
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Length;

    use super::*;

    fn page(w: f64, h: f64) -> DocumentSize {
        DocumentSize::new(Length::from_mm(w), Length::from_mm(h))
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn ac9_natural_sizes_of_the_spec_examples() {
        let (w, h) = natural_size_mm((3000, 2000), (300.0, 300.0));
        assert!(near(w, 254.0) && near(h, 169.333_333_333));
        let (w, h) = natural_size_mm((800, 600), (96.0, 96.0));
        assert!(near(w, 211.666_666_667) && near(h, 158.75));
        let (w, h) = natural_size_mm((300, 300), (150.0, 300.0));
        assert!(
            near(w, 50.8) && near(h, 25.4),
            "non-square pixels keep the aspect"
        );
    }

    #[test]
    fn ac10_a_large_photo_is_scaled_to_fit_a4() {
        let placed = place_image(
            (4000, 3000),
            (96.0, 96.0),
            page(210.0, 297.0),
            ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
            0,
        );
        assert!(near(placed.fit, 0.198_425_196_850));
        assert!(near(placed.placement.a, 210.0) && near(placed.placement.d, 157.5));
        assert_eq!(placed.fit.mul_add(100.0, 0.0).round(), 20.0);
    }

    #[test]
    fn ac12_the_centre_is_pulled_inside_the_document() {
        let dpi = (25.4, 25.4); // one pixel per millimetre
        let placed = place_image(
            (100, 50),
            dpi,
            page(297.0, 210.0),
            ImageTarget::ViewCentre(Point::new(500.0, 100.0)),
            0,
        );
        assert!(near(placed.placement.e, 197.0) && near(placed.placement.f, 75.0));
        let inside = place_image(
            (100, 50),
            dpi,
            page(297.0, 210.0),
            ImageTarget::ViewCentre(Point::new(150.0, 105.0)),
            0,
        );
        assert!(near(inside.placement.e, 100.0) && near(inside.placement.f, 80.0));
        assert!(near(placed.fit, 1.0), "never enlarged");
    }

    #[test]
    fn ac4_ac5_a_drop_is_not_clamped_and_further_images_step_10_mm() {
        let dpi = (25.4, 25.4);
        let dropped = place_image(
            (20, 10),
            dpi,
            page(100.0, 100.0),
            ImageTarget::Drop(Point::new(-50.0, 300.0)),
            1,
        );
        assert!(near(dropped.placement.e, -50.0) && near(dropped.placement.f, 305.0));
    }
}
