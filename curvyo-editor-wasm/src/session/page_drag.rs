//! Page-edge gestures with the ownership read frozen at the press.
use super::{Session, Tool};
use curvyo_document_core::{ObjectSnapshot, Page, PageContents, Point, Vec2};
use curvyo_ui_core::{PressTarget, classify_press_with_artwork};
#[derive(Debug)]
pub(super) struct PageDrag {
    page: Page,
    start: Point,
    delta: Vec2,
    /// What every page owns at the press (`specs/0109` criteria 21, 22); the dragged page has no
    /// entry when its frame moves alone.
    contents: PageContents,
}
impl Session {
    pub(super) fn page_edge_at(&self, point: Point) -> Option<Page> {
        if self.tool != Tool::Select || self.context_group().is_some() {
            return None;
        }
        if !matches!(
            classify_press_with_artwork(
                &self.objects(),
                &self.pickable_objects(),
                &self.selection,
                point,
                self.object_tolerance(),
                self.transform_handle_tolerances(),
                self.held,
                &|path| self.width_artwork(path),
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
        // A variable stroke still being calculated keeps the page where it is; the press is
        // taken so it starts no marquee.
        let Some(contents) = self.drag_contents(&page) else {
            return true;
        };
        // A page of another workbench (a sketch page dragged from a drawing page) activates on
        // the release, so the drag keeps the tools it started with (`specs/0099` criterion 5).
        if self.workbench_of_kind(&page.kind) == self.workbench() {
            self.page_view.active = Some(page.id.clone());
        }
        self.page_drag = Some(PageDrag {
            page,
            start: point,
            delta: Vec2::new(0.0, 0.0),
            contents,
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
        let pages = self.document.pages();
        let _ = self
            .document
            .move_page(&drag.page.id, drag.delta, &drag.contents);
        self.note_moved_pages(&pages, &drag.page.id);
        if self.workbench_of_kind(&drag.page.kind) == self.workbench() {
            self.page_view.press(&self.document.pages(), point, false);
        } else {
            self.activate_page(&drag.page.id);
        }
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
        if let Some(drag) = &self.page_drag
            && let Some(carried) = drag.contents.get(&drag.page.id)
        {
            let carried: std::collections::HashSet<_> = carried.iter().copied().collect();
            for object in objects {
                if carried.contains(&object.id()) {
                    *object = object.translated(drag.delta);
                }
            }
        }
    }
}
