//! The draw list of a document's thumbnail: the active page fitted into a small canvas with its
//! artwork and nothing of the editor over it (`specs/0095-workspace` criterion 6).

use std::collections::HashSet;

use curvyo_document_core::{ObjectSnapshot, Page, Point, ViewTransform};
use curvyo_render_core::{DrawList, build_page_areas};

use super::Session;

/// The share of each side left free around the page **(Proposal)**.
const MARGIN: f64 = 0.06;

/// The view that shows `page` whole and centred in a canvas of `width` x `height` CSS pixels.
#[must_use]
pub fn thumbnail_view(page: &Page, width: f64, height: f64) -> ViewTransform {
    let page_width = page.size.width.as_mm().max(f64::EPSILON);
    let page_height = page.size.height.as_mm().max(f64::EPSILON);
    let free = 1.0 - 2.0 * MARGIN;
    let scale = (width * free / page_width).min(height * free / page_height);
    let centre = Point::new(
        page.origin.x + page_width / 2.0,
        page.origin.y + page_height / 2.0,
    );
    let origin = Point::new(
        centre.x - width / 2.0 / scale,
        centre.y - height / 2.0 / scale,
    );
    ViewTransform::new(scale, origin)
}

impl Session {
    /// The thumbnail of the active page in a canvas of `width` x `height` CSS pixels: the page's
    /// area and the document's visible artwork, without guides, grid, selection or any tool's marks.
    #[must_use]
    pub fn thumbnail_frame(&self, width: f64, height: f64) -> (DrawList, ViewTransform) {
        let page = self.active_page();
        let view = thumbnail_view(&page, width, height);
        let read = self.document_read();
        let objects: Vec<ObjectSnapshot> = read
            .drawn
            .iter()
            .filter(|object| !read.guides.contains(&object.id()))
            .cloned()
            .collect();
        let mut list = build_page_areas(
            std::slice::from_ref(&page),
            self.document.background(),
            view,
            self.device_pixel_ratio,
        );
        list.extend(self.artwork_list(&objects, &HashSet::new(), view));
        (list, view)
    }
}

#[cfg(test)]
mod tests {
    use super::super::align_test_support::{EPS, draw_rect, select_all};
    use super::*;

    fn session() -> Session {
        let mut session = Session::new(1);
        session.resize_viewport(1200.0, 800.0);
        session
    }

    /// The page is centred and fits inside the margin on its tighter side.
    #[test]
    fn the_view_fits_the_page_into_the_thumbnail() {
        let session = session();
        let page = session.active_page();
        let view = thumbnail_view(&page, 80.0, 56.0);
        let (left, top) = view.document_to_screen(page.origin);
        let (right, bottom) = view.document_to_screen(Point::new(
            page.origin.x + page.size.width.as_mm(),
            page.origin.y + page.size.height.as_mm(),
        ));
        assert!((f64::midpoint(left, right) - 40.0).abs() < EPS);
        assert!((f64::midpoint(top, bottom) - 28.0).abs() < EPS);
        assert!(left >= 80.0 * MARGIN - EPS && top >= 56.0 * MARGIN - EPS);
        assert!(right <= 80.0 * (1.0 - MARGIN) + EPS && bottom <= 56.0 * (1.0 - MARGIN) + EPS);
        let fitted_side = (left - 80.0 * MARGIN)
            .abs()
            .min((top - 56.0 * MARGIN).abs());
        assert!(fitted_side < EPS);
    }

    /// The artwork is drawn; a guide and the selection's box and handles are not.
    #[test]
    fn the_thumbnail_draws_the_artwork_without_marks() {
        let mut session = session();
        let empty = session.thumbnail_frame(80.0, 56.0).0.triangles.len();
        draw_rect(&mut session, (20.0, 20.0), (60.0, 50.0));
        let with_rect = session.thumbnail_frame(80.0, 56.0).0.triangles.len();
        assert!(with_rect > empty);
        select_all(&mut session);
        assert_eq!(
            session.thumbnail_frame(80.0, 56.0).0.triangles.len(),
            with_rect
        );
        assert!(session.toggle_guide());
        assert_eq!(session.thumbnail_frame(80.0, 56.0).0.triangles.len(), empty);
    }
}
