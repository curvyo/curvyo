//! The registration marks of a Drawing page in the session (`specs/0121-print-and-cut`
//! criteria 1 to 3): Add registration marks, the Registration marks block of the Style tab and
//! its edits, Add mark n.

use curvyo_document_core::{
    Length, MarksObject, NodeId, ObjectSnapshot, PageKind, Point, registration_mark_corners,
};
use serde_json::json;

use super::{Session, Tool};

/// The refusal of a delete that would keep one mark (criterion 3).
pub(super) const MARKS_BELOW_TWO_NOTICE: &str = "Registration marks need at least 2 marks.";

impl Session {
    /// Whether the active page is a Drawing page (the Document tab shows the button there only).
    #[must_use]
    pub fn registration_marks_offered(&self) -> bool {
        self.active_page().kind == PageKind::Drawing
    }

    /// Where marks 1 to 4 go on the active page now (criterion 1): the artwork box is every
    /// visible object of the page but guides and marks objects.
    fn mark_corners(&self) -> [Point; 4] {
        let page = self.active_page();
        let frame = (
            page.origin,
            Point::new(
                page.origin.x + page.size.width.as_mm(),
                page.origin.y + page.size.height.as_mm(),
            ),
        );
        let read = self.document_read();
        let membership = self.page_membership();
        let mut artwork: Option<(Point, Point)> = None;
        for object in read.visible.iter().filter(|o| {
            !read.guides.contains(&o.id()) && !matches!(o, ObjectSnapshot::RegistrationMark(_))
        }) {
            let Some((a, b)) = self.page_fit_bounds(object) else {
                continue;
            };
            if !membership.is_member(&page, object.id(), (a, b), curvyo_ui_core::Reach::Touches) {
                continue;
            }
            artwork = Some(artwork.map_or((a, b), |(lo, hi)| {
                (
                    Point::new(lo.x.min(a.x), lo.y.min(a.y)),
                    Point::new(hi.x.max(b.x), hi.y.max(b.y)),
                )
            }));
        }
        registration_mark_corners(frame, artwork)
    }

    /// Add registration marks (criterion 1): one marks object of marks 1 to 4 on the active
    /// Drawing page in one step, selected with the Select tool. `false` when nothing was added.
    pub fn add_registration_marks(&mut self) -> bool {
        if !self.registration_marks_offered() {
            return false;
        }
        self.recording(|this| {
            let corners = this.mark_corners();
            let Ok(group) = this.document.insert_registration_marks(corners) else {
                return false;
            };
            this.set_tool(Tool::Select);
            let leaves = this.document.leaves_of(group);
            this.selection.set(&leaves);
            true
        })
    }

    /// The marks object the selection is, or lies in, with the one entered mark if exactly one
    /// mark inside it is selected.
    fn selected_marks(&self) -> Option<(MarksObject, Option<u8>)> {
        let ids = self.selection.ids();
        let first = *ids.first()?;
        let object = self.document.marks_object_of(first)?;
        let inside = |id: &NodeId| object.marks.iter().any(|m| m.id == *id);
        if !ids.iter().all(inside) {
            return None;
        }
        let one = (ids.len() == 1 && self.context_group() == Some(object.id))
            .then(|| {
                object
                    .marks
                    .iter()
                    .find(|m| m.id == first)
                    .map(|m| m.number)
            })
            .flatten();
        Some((object, one))
    }

    /// Whether the selection is a marks object or marks of one: it only moves (UX notes P6).
    pub(super) fn marks_selected(&self) -> bool {
        self.selected_marks().is_some()
    }

    /// The Registration marks block of the Style tab as JSON, or `None` when the selection is no
    /// marks object (UX notes §3.3): `{ "group", "count", "mark", "diameter", "line", "add" }`
    /// with `add` the lowest missing number (absent at 4 marks).
    #[must_use]
    pub fn registration_marks_view(&self) -> Option<String> {
        let (object, mark) = self.selected_marks()?;
        let first = object.marks.first()?;
        Some(
            json!({
                "group": object.id,
                "count": object.marks.len(),
                "mark": mark,
                "diameter": first.diameter.as_mm(),
                "line": first.line.as_mm(),
                "add": object.lowest_free_number(),
            })
            .to_string(),
        )
    }

    /// Size or Line width (criterion 2): every mark of the selected marks object in one step.
    /// `false` when the values are out of range or nothing is selected.
    pub fn set_registration_mark_size(&mut self, diameter_mm: f64, line_mm: f64) -> bool {
        let Some((object, _)) = self.selected_marks() else {
            return false;
        };
        self.recording(|this| {
            this.document
                .set_registration_mark_size(
                    object.id,
                    Length::from_mm(diameter_mm),
                    Length::from_mm(line_mm),
                )
                .is_ok()
        })
    }

    /// Add mark n (criterion 3): the lowest free number at its nominal corner, selected inside
    /// the entered object.
    pub fn add_registration_mark(&mut self) -> bool {
        let Some((object, _)) = self.selected_marks() else {
            return false;
        };
        self.recording(|this| {
            let corners = this.mark_corners();
            let Ok(mark) = this.document.add_registration_mark(object.id, corners) else {
                return false;
            };
            this.enter_group(object.id, Some(mark));
            this.selection.set(&[mark]);
            true
        })
    }

    /// Whether deleting the selection is refused by the two-mark rule; sets the notice.
    pub(super) fn refuse_marks_delete(&mut self) -> bool {
        if !self.document.delete_leaves_one_mark(self.selection.ids()) {
            return false;
        }
        self.history.notice = Some(MARKS_BELOW_TWO_NOTICE.to_owned());
        true
    }
}

#[cfg(test)]
#[path = "registration_marks_tests.rs"]
mod tests;
