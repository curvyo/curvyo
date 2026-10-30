//! Page-edge gestures with a frozen contained-object set.
use super::{Session, Tool};
use curvyo_document_core::{NodeId, ObjectSnapshot, Page, Point, Vec2};
use curvyo_ui_core::{PressTarget, classify_press};
#[derive(Debug)]
pub(super) struct PageDrag {
    page: Page,
    start: Point,
    delta: Vec2,
    objects: Vec<NodeId>,
}
impl Session {
    pub(super) fn page_edge_at(&self, point: Point) -> Option<Page> {
        if self.tool != Tool::Select || self.context_group().is_some() {
            return None;
        }
        if !matches!(
            classify_press(
                &self.objects(),
                &self.pickable_objects(),
                &self.selection,
                point,
                self.object_tolerance(),
                self.transform_handle_tolerances(),
                self.held
            ),
            PressTarget::Empty
        ) {
            return None;
        }
        let tolerance = 6.0 / self.view().scale();
        let edge = |p: &Page| {
            let right = p.origin.x + p.size.width.as_mm();
            let bottom = p.origin.y + p.size.height.as_mm();
            // Hit the outside band so a marquee can still start anywhere inside
            // the page, including its exact boundary. Leave corners to marquees.
            let vertical = (point.x < p.origin.x && point.x >= p.origin.x - tolerance)
                || (point.x > right && point.x <= right + tolerance);
            let horizontal = (point.y < p.origin.y && point.y >= p.origin.y - tolerance)
                || (point.y > bottom && point.y <= bottom + tolerance);
            (vertical && point.y > p.origin.y + tolerance && point.y < bottom - tolerance)
                || (horizontal && point.x > p.origin.x + tolerance && point.x < right - tolerance)
        };
        let active = self.active_page();
        if edge(&active) {
            Some(active)
        } else {
            self.document.pages().into_iter().rev().find(edge)
        }
    }
    pub(super) fn page_drag_start(&mut self, point: Point) -> bool {
        let Some(page) = self.page_edge_at(point) else {
            return false;
        };
        let objects = if self.page_view.frame_only.contains(&page.id) {
            Vec::new()
        } else {
            self.contained_page_objects(&page, false)
        };
        self.page_view.active = Some(page.id.clone());
        self.page_drag = Some(PageDrag {
            page,
            start: point,
            delta: Vec2::new(0.0, 0.0),
            objects,
        });
        self.button_down = true;
        true
    }
    pub(super) fn page_drag_move(&mut self, point: Point) -> bool {
        let scale = self.view().scale();
        let Some(drag) = self.page_drag.as_mut() else {
            return false;
        };
        let delta = drag.start.vector_to(point);
        drag.delta = if delta.length() * scale >= 4.0 {
            delta
        } else {
            Vec2::new(0.0, 0.0)
        };
        true
    }
    pub(super) fn page_drag_end(&mut self, point: Point) -> bool {
        self.page_drag_move(point);
        let Some(drag) = self.page_drag.take() else {
            return false;
        };
        let _ = self
            .document
            .move_page(&drag.page.id, drag.delta, &drag.objects);
        self.page_view.press(&self.document.pages(), point);
        true
    }
    pub(super) fn shown_pages(&self) -> Vec<Page> {
        let mut pages = self.document.pages();
        if let Some(drag) = &self.page_drag
            && let Some(page) = pages.iter_mut().find(|p| p.id == drag.page.id)
        {
            page.origin = drag.page.origin.translated(drag.delta);
        }
        pages
    }
    pub(super) fn page_drag_artwork<'a>(
        &self,
        mut objects: std::borrow::Cow<'a, [ObjectSnapshot]>,
    ) -> std::borrow::Cow<'a, [ObjectSnapshot]> {
        if self.page_drag.is_some() {
            self.page_drag_preview(objects.to_mut());
        }
        objects
    }
    fn page_drag_preview(&self, objects: &mut [ObjectSnapshot]) {
        if let Some(drag) = &self.page_drag {
            for object in objects {
                if drag.objects.contains(&object.id()) {
                    *object = object.translated(drag.delta);
                }
            }
        }
    }
}
