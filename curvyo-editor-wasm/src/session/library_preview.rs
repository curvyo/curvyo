//! The page preview pane's reading of a read-only library source (`specs/0137` criterion 5, UX
//! notes sections 6.2 to 6.4): its view (fit, zoom about the pane's centre, pan), and picking by
//! the Select tool's rules (the outline hit of a click, the marquee's touch and contain modes),
//! each pick taken at its top-level object.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, ObjectSnapshot, Point, Tolerance, ViewTransform};
use curvyo_ui_core::{MarqueeMode, Units};

use super::Session;

/// How near a click must come to an outline, in CSS pixels (the Select tool's).
const HIT_PX: f64 = 4.0;

/// The pane's view of a page: fitted, then `zoom` about the pane's centre and moved by `pan`
/// CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewView {
    /// The zoom over the fit, 1 for the fit.
    pub zoom: f64,
    /// The move in CSS pixels.
    pub pan: (f64, f64),
}

impl Default for PreviewView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
        }
    }
}

impl Session {
    /// The transform of page `page` in a pane of `width` x `height` CSS pixels under `view`.
    #[must_use]
    pub fn preview_transform(
        &self,
        page: &str,
        (width, height): (f64, f64),
        view: PreviewView,
    ) -> Option<ViewTransform> {
        let page = self.document.pages().into_iter().find(|p| p.id == page)?;
        let fit = super::thumbnail::thumbnail_view(&page, width, height);
        let centre = (width / 2.0, height / 2.0);
        let at_centre = fit.screen_to_document(centre.0, centre.1);
        let scale = fit.scale() * view.zoom.clamp(0.05, 64.0);
        let origin = Point::new(
            at_centre.x - (centre.0 + view.pan.0) / scale,
            at_centre.y - (centre.1 + view.pan.1) / scale,
        );
        Some(ViewTransform::new(scale, origin))
    }

    /// The visible leaves of the top-level objects of `page`, in z-order, and each leaf's
    /// top-level object.
    fn pickable(&self, page: &str) -> (Vec<ObjectSnapshot>, Vec<NodeId>) {
        let tops: HashSet<NodeId> = self.page_top_level(page).into_iter().collect();
        let read = self.document_read();
        let mut objects = Vec::new();
        let mut owners = Vec::new();
        for object in read.drawn.iter() {
            let id = object.id();
            let top = super::library_import::top_level(&self.document, id);
            if tops.contains(&top) && !read.guides.contains(&id) {
                objects.push(object.clone());
                owners.push(top);
            }
        }
        (objects, owners)
    }

    /// The top-level object of `page` a click at the pane point `at` picks (the Select tool's
    /// outline hit), `None` on empty pasteboard.
    #[must_use]
    pub fn preview_hit(
        &self,
        page: &str,
        size: (f64, f64),
        view: PreviewView,
        at: (f64, f64),
    ) -> Option<NodeId> {
        let transform = self.preview_transform(page, size, view)?;
        let (objects, owners) = self.pickable(page);
        let point = transform.screen_to_document(at.0, at.1);
        let tolerance = Tolerance::from_mm(HIT_PX / transform.scale());
        let hit = curvyo_ui_core::hit_test_object(&objects, point, tolerance)?;
        objects
            .iter()
            .position(|o| o.id() == hit)
            .map(|index| owners[index])
    }

    /// The top-level objects of `page` the marquee from `from` to `to` (pane points) picks:
    /// rightward contains, leftward touches (the Select marquee's modes), groups as wholes.
    #[must_use]
    pub fn preview_marquee(
        &self,
        page: &str,
        size: (f64, f64),
        view: PreviewView,
        (from, to): ((f64, f64), (f64, f64)),
    ) -> Vec<NodeId> {
        let Some(transform) = self.preview_transform(page, size, view) else {
            return Vec::new();
        };
        let (objects, owners) = self.pickable(page);
        let a = transform.screen_to_document(from.0, from.1);
        let b = transform.screen_to_document(to.0, to.1);
        let mode = MarqueeMode::for_drag(a, b, false);
        let units = Units::build(&self.document, None);
        let tolerance = Tolerance::from_mm(0.5 / transform.scale());
        let picked = curvyo_ui_core::units_in_marquee(&objects, &units, a, b, mode, tolerance);
        let mut tops = Vec::new();
        for leaf in picked {
            if let Some(index) = objects.iter().position(|o| o.id() == leaf)
                && !tops.contains(&owners[index])
            {
                tops.push(owners[index]);
            }
        }
        tops
    }
}

#[cfg(test)]
mod tests {
    use super::super::align_test_support::draw_rect;
    use super::*;

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn a_click_picks_by_outline_and_a_marquee_by_its_mode() {
        let mut source = Session::new(1);
        source.resize_viewport(1200.0, 800.0);
        let a = draw_rect(&mut source, (10.0, 10.0), (40.0, 30.0));
        let b = draw_rect(&mut source, (100.0, 100.0), (130.0, 120.0));
        let size = (400.0, 300.0);
        let view = PreviewView::default();
        let t = source.preview_transform("first", size, view).unwrap();
        let on_a = t.document_to_screen(Point::new(10.0, 20.0));
        assert_eq!(source.preview_hit("first", size, view, on_a), Some(a));
        let empty = t.document_to_screen(Point::new(200.0, 200.0));
        assert_eq!(source.preview_hit("first", size, view, empty), None);
        let from = t.document_to_screen(Point::new(0.0, 0.0));
        let to = t.document_to_screen(Point::new(50.0, 50.0));
        assert_eq!(
            source.preview_marquee("first", size, view, (from, to)),
            vec![a]
        );
        let wide = t.document_to_screen(Point::new(150.0, 150.0));
        assert_eq!(
            source.preview_marquee("first", size, view, (from, wide)),
            vec![a, b]
        );
        // A leftward drag touches: crossing b's box picks it.
        let inside_b = t.document_to_screen(Point::new(110.0, 110.0));
        let left_of = t.document_to_screen(Point::new(90.0, 90.0));
        assert_eq!(
            source.preview_marquee("first", size, view, (inside_b, left_of)),
            vec![b]
        );
        // Zoom about the centre keeps the centre.
        let zoomed = PreviewView {
            zoom: 2.0,
            pan: (0.0, 0.0),
        };
        let z = source.preview_transform("first", size, zoomed).unwrap();
        let c = t.screen_to_document(200.0, 150.0);
        let zc = z.screen_to_document(200.0, 150.0);
        assert!((c.x - zc.x).abs() < 1e-9 && (c.y - zc.y).abs() < 1e-9);
        assert!((z.scale() - 2.0 * t.scale()).abs() < 1e-12);
    }
}
