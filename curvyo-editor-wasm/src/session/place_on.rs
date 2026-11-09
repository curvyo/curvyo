//! Place on… (`specs/0100-bring-content-in` criterion 25, Frage 163 B): linked clones of the
//! selection on a chosen laser page, as a default-mode drop would make them, by button.

use curvyo_document_core::{Page, PageKind, Point, Vec2};
use curvyo_ui_core::object_bounds;
use serde::Serialize;

use super::Session;

/// One row of the Place on… list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaceOnTarget {
    /// The laser page.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its width in mm.
    pub width_mm: f64,
    /// Its height in mm.
    pub height_mm: f64,
    /// Its size in the display unit, as the Place page rows show it ("400.0 × 300.0 mm").
    pub size: String,
}

impl Session {
    /// The laser pages Place on… offers, in list order; empty hides the button.
    #[must_use]
    pub fn place_on_targets(&self) -> Vec<PlaceOnTarget> {
        self.document
            .pages()
            .into_iter()
            .filter(|page| match &page.kind {
                PageKind::Output { process, .. } => self.process_enabled(process),
                _ => false,
            })
            .map(|page| PlaceOnTarget {
                id: page.id,
                name: page.name,
                width_mm: page.size.width.as_mm(),
                height_mm: page.size.height.as_mm(),
                size: curvyo_ui_core::format_size(page.size, self.display_unit()),
            })
            .collect()
    }

    /// Places linked clones of the selection (guides and dimensions left out) on laser page
    /// `laser` at the place the selection has relative to its page's top-left, moved inside the
    /// laser page where it would not fit; one step "Place on page", the laser page activated
    /// and the clones selected. `false` when nothing is placed.
    pub fn place_on(&mut self, laser: &str) -> bool {
        let pages = self.document.pages();
        let Some(target) = pages.iter().find(|p| p.id == laser).cloned() else {
            return false;
        };
        if !matches!(target.kind, PageKind::Output { .. }) {
            return false;
        }
        let read = self.document_read();
        // One clone per selected top-level object, a group whole, as the drag makes them;
        // guides and dimensions are left out (criteria 6 and 25).
        let units = self.units().selected(self.selection.ids());
        let leaf = |id| read.leaves.iter().find(|o| o.id() == id);
        let mut originals = Vec::new();
        let mut boxes = Vec::new();
        for unit in &units {
            let usable: Vec<_> = unit
                .leaves
                .iter()
                .filter_map(|&id| leaf(id))
                .filter(|o| !matches!(o, curvyo_document_core::ObjectSnapshot::Dimension(_)))
                .filter(|o| !read.guides.contains(&o.id()))
                .collect();
            if usable.is_empty() || (!unit.is_group && usable.len() != unit.leaves.len()) {
                continue;
            }
            originals.push(unit.id);
            boxes.extend(usable.iter().map(|o| object_bounds(o)));
        }
        let Some(bounds) = boxes.into_iter().reduce(|(a, b), (c, d)| {
            (
                Point::new(a.x.min(c.x), a.y.min(c.y)),
                Point::new(b.x.max(d.x), b.y.max(d.y)),
            )
        }) else {
            return false;
        };
        let source = self.active_page();
        let translation = clamped_translation(&source, &target, bounds);
        let placed = self.recording(|this| {
            this.leave_to_level(0);
            this.document.set_draw_parent(this.draw_parent(None));
            this.document
                .place_linked_clones_on_page(&originals, translation)
                .ok()
        });
        let Some(clones) = placed else {
            return false;
        };
        self.activate_page(laser);
        self.node.clear_selection();
        self.selection.set(&clones);
        true
    }
}

/// The translation that keeps `bounds`' place relative to `source`'s top-left on `target`,
/// moved inside `target` where it would leave it (its top-left kept when it is larger).
fn clamped_translation(source: &Page, target: &Page, (min, max): (Point, Point)) -> Vec2 {
    let mut dx = target.origin.x - source.origin.x;
    let mut dy = target.origin.y - source.origin.y;
    let (w, h) = (target.size.width.as_mm(), target.size.height.as_mm());
    let right = target.origin.x + w;
    let bottom = target.origin.y + h;
    if max.x + dx > right {
        dx -= max.x + dx - right;
    }
    if min.x + dx < target.origin.x {
        dx += target.origin.x - (min.x + dx);
    }
    if max.y + dy > bottom {
        dy -= max.y + dy - bottom;
    }
    if min.y + dy < target.origin.y {
        dy += target.origin.y - (min.y + dy);
    }
    Vec2::new(dx, dy)
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{FIRST_PAGE_ID, ObjectSnapshot};

    use crate::session::align_test_support::draw_rect;
    use crate::session::{Session, Tool};

    #[test]
    fn criterion_25_place_on_makes_clones_at_the_same_place_on_the_laser_page() {
        let mut s = Session::new(100_025);
        let a = draw_rect(&mut s, (10.0, 10.0), (30.0, 30.0));
        let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
        s.activate_page(FIRST_PAGE_ID);
        assert_eq!(s.place_on_targets().len(), 1);
        s.set_tool(Tool::Select);
        s.selection.set(&[a]);
        let steps = s.undo_len();
        assert!(s.place_on(&laser));
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(s.active_page().id, laser);
        let clone = s.selection.ids()[0];
        let Some(ObjectSnapshot::Clone(c)) = s.document.object(clone) else {
            panic!("a clone")
        };
        assert!(c.chain.own_placement);
        let page = s.active_page();
        let (min, _) = curvyo_ui_core::object_bounds(&ObjectSnapshot::Clone(c));
        assert!((min.x - page.origin.x - 10.0).abs() < 1e-6);
        assert!((min.y - page.origin.y - 10.0).abs() < 1e-6);
    }
}
