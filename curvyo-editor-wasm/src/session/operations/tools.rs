//! The session state of the laser operation tools (`specs/0111-laser-operation-tools` criteria
//! 11 to 13, 15 and 17; `adrs.md` "Session-only state" and architecture finding 1): which laser
//! pages draw the drawing's own style, which operation row is unfolded, and the read-only entry
//! of a group on a laser page. Nothing here is saved and nothing is a history step.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::{hit_test_objects, object_bounds};

use super::super::{Session, Workbench};

/// The one feature-owned state of the operation tools.
#[derive(Debug, Default)]
pub(in crate::session) struct OperationTools {
    /// The laser pages (manufacturing nodes) whose Operations view toggle is unpressed; every
    /// other laser page is pressed (the default).
    drawn: HashSet<NodeId>,
    /// The one unfolded operation row (Frage 197 A).
    unfolded: Option<NodeId>,
}

impl OperationTools {
    /// Whether the Operations view of laser page `manufacturing` is pressed.
    pub(in crate::session) fn view_on(&self, manufacturing: NodeId) -> bool {
        !self.drawn.contains(&manufacturing)
    }

    /// Presses or unpresses the Operations view of laser page `manufacturing`.
    fn toggle_view(&mut self, manufacturing: NodeId) {
        if !self.drawn.remove(&manufacturing) {
            self.drawn.insert(manufacturing);
        }
    }

    /// The unfolded row.
    pub(in crate::session) const fn unfolded(&self) -> Option<NodeId> {
        self.unfolded
    }

    /// A press on row `id`: it unfolds and any other folds; a press on the unfolded row folds it.
    fn unfold(&mut self, id: NodeId) {
        self.unfolded = if self.unfolded == Some(id) {
            None
        } else {
            Some(id)
        };
    }
}

impl Session {
    /// Whether the Select tool works inside a group of the active laser page, which is read-only
    /// there (criterion 11): only selection and assignment reach its members.
    pub(in crate::session) fn in_read_only_group(&self) -> bool {
        !self.context.is_empty() && matches!(self.workbench(), Workbench::Manufacturing(_))
    }

    /// Leaves every entered group of a laser page at once (Escape, a press outside, a page
    /// activation; criterion 13). With `select_group` the outermost group left is selected.
    /// Whether a group was left; not a history step.
    pub(in crate::session) fn leave_read_only_group(&mut self, select_group: bool) -> bool {
        if self.context.is_empty() {
            return false;
        }
        let left = self.context.first().copied();
        let _ = self.leave_to_level(0);
        if select_group && let Some(group) = left {
            let members = self.units().leaves_of_unit(group);
            self.selection.set(&members);
        }
        true
    }

    /// A Select press at `point` inside an entered laser group that hits no member and lies
    /// outside the group's box leaves the group (criterion 13), so the press then selects what it
    /// hits at the page. Whether it left; a press inside the box keeps the group (a marquee over
    /// the members, Proposal of criterion 12).
    pub(in crate::session) fn leave_read_only_group_at(&mut self, point: Point) -> bool {
        if !self.in_read_only_group() {
            return false;
        }
        let pickable = self.pickable_objects();
        if !hit_test_objects(&pickable, point, self.object_tolerance()).is_empty() {
            return false;
        }
        let inside = self
            .objects()
            .iter()
            .map(object_bounds)
            .reduce(|(a0, a1), (b0, b1)| {
                (
                    Point::new(a0.x.min(b0.x), a0.y.min(b0.y)),
                    Point::new(a1.x.max(b1.x), a1.y.max(b1.y)),
                )
            })
            .is_some_and(|(min, max)| {
                (min.x..=max.x).contains(&point.x) && (min.y..=max.y).contains(&point.y)
            });
        !inside && self.leave_read_only_group(false)
    }

    /// A Select double-click at `point` inside an entered laser group: a nested group under it is
    /// entered one level (criterion 11); empty canvas leaves one level, as built; anything else
    /// does nothing (no node edit, no text edit, no typed entry).
    pub(in crate::session) fn read_only_double_click(&mut self, point: Point) {
        let pickable = self.pickable_objects();
        let hit = hit_test_objects(&pickable, point, self.object_tolerance());
        match hit.first() {
            Some(&leaf) => {
                let unit = self.units().unit_of(leaf);
                if unit != leaf && self.document.is_group(unit) {
                    self.enter_group(unit, Some(leaf));
                }
            }
            None => {
                self.leave_group(false);
            }
        }
    }

    /// Presses the Operations view toggle in the label of laser page `page` (criterion 15).
    /// Session state, no step. `false` for a page that is not a laser page.
    pub fn toggle_operations_view(&mut self, page: &str) -> bool {
        let Some(manufacturing) = self
            .document
            .page_kind(page)
            .and_then(|kind| kind.manufacturing())
            .filter(|&m| self.manufacturing_available(m))
        else {
            return false;
        };
        self.operations.tools.toggle_view(manufacturing);
        true
    }

    /// Whether the Operations view of laser page `manufacturing` is pressed (criterion 15).
    pub(in crate::session) fn operations_view_pressed(&self, manufacturing: NodeId) -> bool {
        self.operations.tools.view_on(manufacturing)
    }

    /// A press on operation row `id` of the active laser page (criterion 17): it unfolds under
    /// itself and any unfolded row folds; a press on the unfolded row folds it. Session state, no
    /// step. `false` for an operation of another page.
    pub fn unfold_operation_row(&mut self, id: NodeId) -> bool {
        if !self.is_active_operation(id) {
            return false;
        }
        self.operations.tools.unfold(id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(counter: u32) -> NodeId {
        serde_json::from_str(&format!(r#"{{"peer":"1","counter":{counter}}}"#)).unwrap()
    }

    #[test]
    fn one_row_is_unfolded_at_a_time_and_a_second_press_folds_it() {
        let mut tools = OperationTools::default();
        tools.unfold(id(1));
        assert_eq!(tools.unfolded(), Some(id(1)));
        tools.unfold(id(2));
        assert_eq!(tools.unfolded(), Some(id(2)));
        tools.unfold(id(2));
        assert_eq!(tools.unfolded(), None);
    }

    #[test]
    fn the_view_is_pressed_by_default_and_per_page() {
        let mut tools = OperationTools::default();
        assert!(tools.view_on(id(1)));
        tools.toggle_view(id(1));
        assert!(!tools.view_on(id(1)));
        assert!(tools.view_on(id(2)));
        tools.toggle_view(id(1));
        assert!(tools.view_on(id(1)));
    }
}
