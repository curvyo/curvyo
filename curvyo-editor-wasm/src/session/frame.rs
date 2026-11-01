//! The list the host submits each frame: the document's own area (its
//! background) under everything `Session::draw_list` builds
//! (`specs/0015-document-size-and-rulers/` criterion 28,
//! `specs/0040-document-background` criterion 13), each shown page grid over its
//! page's area (`specs/0090-grid` criterion 4).

use curvyo_document_core::Page;
use curvyo_render_core::{DrawList, GridStroke, build_page_areas, build_page_areas_with_grids};

use super::Session;

impl Session {
    /// The frame to submit: the document rectangle as the bottom artwork layer
    /// (it pans and zooms with the content, and the GPU's pasteboard clear
    /// shows around it), then [`Session::draw_list`]. Kept apart from
    /// `draw_list` so the area is not mistaken for artwork of the document.
    #[must_use]
    pub fn frame_draw_list(&self) -> DrawList {
        // A document preview draws the version, its page included, plain, with nothing over it
        // (`specs/0042-history-branches` criterion 19).
        if let Some(version) = self.document_preview() {
            let mut list = build_page_areas(
                &version.pages,
                version.background,
                self.view(),
                self.device_pixel_ratio,
            );
            list.extend(curvyo_render_core::build_artwork_with_guides(
                &version.objects,
                &version.guides,
                self.view(),
            ));
            return list;
        }
        let pages = self.shown_pages();
        let background = self.shown_background();
        let grids = self.grid_lists(&pages, background);
        let mut list = build_page_areas_with_grids(
            &pages,
            background,
            self.view(),
            self.device_pixel_ratio,
            &grids,
        );
        // An object preview draws the object only as it was at the version, in its place: its
        // current drawing, selection box and handles are left out (criterion 17).
        if let Some((subject, version)) = self.object_preview() {
            let others: Vec<_> = self
                .render_objects()
                .iter()
                .filter(|object| object.id() != subject)
                .cloned()
                .collect();
            list.extend(curvyo_render_core::build_artwork_with_guides(
                &others,
                &self.document_read().guides,
                self.view(),
            ));
            list.extend(curvyo_render_core::build_history_preview(
                &version.objects,
                self.view(),
            ));
            return list;
        }
        list.extend(self.draw_list());
        list
    }

    /// The grid lines of every page, in page order: empty for a hidden grid or a page off the
    /// canvas. Bounded by the canvas size (criterion 9).
    fn grid_lists(
        &self,
        pages: &[Page],
        background: curvyo_document_core::DocumentBackground,
    ) -> Vec<DrawList> {
        if pages.iter().all(|page| !page.grid.shown) {
            return Vec::new();
        }
        let view = self.view();
        let (w, h) = self.viewport.canvas_size();
        let visible = (
            view.screen_to_document(0.0, 0.0),
            view.screen_to_document(w, h),
        );
        let ink = curvyo_render_core::grid_ink(background);
        pages
            .iter()
            .map(|page| {
                let strokes: Vec<GridStroke> = curvyo_ui_core::grid_layout(page, view, visible)
                    .lines
                    .into_iter()
                    .map(|line| GridStroke {
                        start: line.start,
                        end: line.end,
                        emphasis: line.emphasis,
                        alpha: line.alpha,
                    })
                    .collect();
                curvyo_render_core::build_grid_lines(&strokes, ink, view, self.device_pixel_ratio)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::DocumentSize;
    use curvyo_render_core::Vertex;

    use super::*;

    /// The area is layer 0 and the artwork layers follow, with the overlay
    /// after them: nothing of `draw_list` is lost or reordered.
    #[test]
    fn the_document_area_is_the_bottom_layer_under_the_unchanged_draw_list() {
        let session = Session::new(1);
        let frame = session.frame_draw_list();
        let inner = session.draw_list();
        assert_eq!(frame.layers().first(), Some(&6), "two triangles");
        assert_eq!(
            frame.triangles.len(),
            inner.triangles.len() + 6,
            "area plus the unchanged list"
        );
        assert_eq!(frame.triangles[6..], inner.triangles[..]);
        assert_eq!(frame.layers().len(), inner.layers().len() + 1);
        assert_eq!(frame.overlay_start(), inner.overlay_start() + 6);
    }

    /// The area covers the document rectangle: (0, 0) to the document's size,
    /// within the device-pixel snap.
    #[test]
    fn the_area_covers_the_document_rectangle() {
        let mut session = Session::new(1);
        session.show_default_view();
        let frame = session.frame_draw_list();
        let area = &frame.triangles[..6];
        let extent = |value: fn(&Vertex) -> f64, pick: fn(f64, f64) -> f64, start: f64| {
            area.iter().map(value).fold(start, pick)
        };
        let size = DocumentSize::default();
        assert!(extent(|v| v.position.x, f64::min, f64::INFINITY).abs() < 0.3);
        assert!(extent(|v| v.position.y, f64::min, f64::INFINITY).abs() < 0.3);
        let right = extent(|v| v.position.x, f64::max, f64::NEG_INFINITY);
        let bottom = extent(|v| v.position.y, f64::max, f64::NEG_INFINITY);
        assert!((right - size.width.as_mm()).abs() < 0.3, "{right}");
        assert!((bottom - size.height.as_mm()).abs() < 0.3, "{bottom}");
    }
}
