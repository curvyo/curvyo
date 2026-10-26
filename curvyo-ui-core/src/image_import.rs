//! One file read as an image for the Import button (`specs/0058-bitmap-import` criteria 2, 6 to
//! 10, 16 to 18, 21, 24): the header and the decode of `curvyo-image-core`, the frame of
//! [`crate::place_image`] and the success lines of its notice. Bytes in, a plan out; the session
//! commits it.

use std::sync::Arc;

use curvyo_document_core::{DocumentSize, NewImage};
use curvyo_image_core::{WorkingImage, decode_working, probe};

use crate::image_import_text::{
    ANIMATED_NOTICE, REDUCED_NOTICE, failure_reason, fit_notice, image_display_name, size_notice,
    truncated_notice,
};
use crate::image_placement::{ImageTarget, place_image};

/// A file read as an image, ready to be added.
#[derive(Debug, Clone)]
pub struct PlannedImage {
    /// The object to add.
    pub image: NewImage,
    /// Its decoded working image, for the canvas.
    pub working: Arc<WorkingImage>,
    /// The success lines of its notice, in the order of the UX notes (fit, working limit, size,
    /// then animation and truncation); empty when nothing needs saying.
    pub lines: Vec<String>,
}

/// Reads `bytes` (the file `file_name`) as the `index`-th image of an import into a document of
/// `page` aimed at `target`.
///
/// # Errors
/// The reason line of the notice when the file is not imported (criteria 17 and 26).
pub fn plan_image_import(
    bytes: Arc<[u8]>,
    file_name: &str,
    page: DocumentSize,
    target: ImageTarget,
    index: usize,
) -> Result<PlannedImage, String> {
    let info = probe(&bytes).map_err(failure_reason)?;
    let working = decode_working(&bytes).map_err(failure_reason)?;
    let density = info.upright_density();
    let placed = place_image(
        working.original_size,
        (density.x_dpi(), density.y_dpi()),
        page,
        target,
        index,
    );
    let mut lines = Vec::new();
    if placed.fit < 1.0 {
        lines.push(fit_notice(placed.fit));
    }
    if working.is_reduced() {
        lines.push(REDUCED_NOTICE.to_owned());
    }
    lines.extend(size_notice(bytes.len() as u64));
    if working.animated {
        lines.push(ANIMATED_NOTICE.to_owned());
    }
    if working.truncated {
        lines.push(truncated_notice(file_name));
    }
    Ok(PlannedImage {
        image: NewImage {
            bytes,
            placement: placed.placement,
            name: image_display_name(file_name),
        },
        working: Arc::new(working),
        lines,
    })
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Length, Point};

    use super::*;

    const JPEG: &[u8] = include_bytes!(
        "../../curvyo-image-core/tests/fixtures/images/jpeg_baseline_jfif_300dpi.jpg"
    );
    const TRUNCATED: &[u8] =
        include_bytes!("../../curvyo-image-core/tests/fixtures/images/jpeg_truncated.jpg");

    fn a4() -> DocumentSize {
        DocumentSize::new(Length::from_mm(210.0), Length::from_mm(297.0))
    }

    #[test]
    fn ac9_a_300_dpi_jpeg_comes_in_at_its_physical_size_without_a_notice() {
        let plan = plan_image_import(
            Arc::from(JPEG),
            "/a/photo.jpg",
            a4(),
            ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
            0,
        )
        .unwrap();
        assert!((plan.image.placement.a - 60.0 / 300.0 * 25.4).abs() < 1e-9);
        assert_eq!(plan.image.name, "photo.jpg");
        assert_eq!(plan.lines, Vec::<String>::new());
    }

    #[test]
    fn ac21_a_truncated_jpeg_imports_with_its_notice_and_text_is_refused() {
        let plan = plan_image_import(
            Arc::from(TRUNCATED),
            "cut.jpg",
            a4(),
            ImageTarget::Drop(Point::new(0.0, 0.0)),
            0,
        )
        .unwrap();
        assert_eq!(
            plan.lines,
            ["cut.jpg is incomplete; the part that could be read was imported."]
        );
        let refused = plan_image_import(
            Arc::from(&b"plain text"[..]),
            "x.png",
            a4(),
            ImageTarget::Drop(Point::new(0.0, 0.0)),
            0,
        );
        assert_eq!(
            refused.err().as_deref(),
            Some("This file type isn't supported. Use SVG, PNG, JPEG or WebP.")
        );
    }
}
