//! Which fixture a press on the canvas reaches (`specs/0102-laser-fixtures` criteria 8 to 11):
//! the selected fixture's handles first, then the outline of a shown fixture of the active laser
//! page, never its inside, objects first (feature `adrs.md` §6a.5).

use curvyo_document_core::{NodeId, Point};

use super::super::{Session, Tool};
use super::commands::movable;
use super::handles::{FixtureHandle, hit_fixture_handle};
use super::placed::PageFixtures;

/// The outline hit tolerance with a mouse and with a coarse pointer, in screen px (D12).
const HIT_PX: (f64, f64) = (4.0, 12.0);

/// The distance from `p` to the segment `a`–`b`.
fn distance(p: Point, (a, b): (Point, Point)) -> f64 {
    let ab = a.vector_to(b);
    let length2 = ab.x * ab.x + ab.y * ab.y;
    let t = if length2 > 0.0 {
        let ap = a.vector_to(p);
        ((ap.x * ab.x + ap.y * ab.y) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.vector_to(a.translated(ab.scaled(t))).length()
}

/// Whether `p` lies within `tolerance` of the box of `corners`, for the bounding-box reject.
fn near_box(corners: &[Point; 4], p: Point, tolerance: f64) -> bool {
    let (mut min, mut max) = (corners[0], corners[0]);
    for c in corners {
        min = Point::new(min.x.min(c.x), min.y.min(c.y));
        max = Point::new(max.x.max(c.x), max.y.max(c.y));
    }
    p.x >= min.x - tolerance
        && p.x <= max.x + tolerance
        && p.y >= min.y - tolerance
        && p.y <= max.y + tolerance
}

/// Whether `p` lies inside the convex box `corners`.
pub(super) fn inside(corners: &[Point; 4], p: Point) -> bool {
    let side = |a: Point, b: Point| {
        let (ab, ap) = (a.vector_to(b), a.vector_to(p));
        ab.x * ap.y - ab.y * ap.x
    };
    let signs: Vec<f64> = (0..4)
        .map(|i| side(corners[i], corners[(i + 1) % 4]))
        .collect();
    signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
}

/// The topmost shown fixture whose outline (or a slot's outline) lies within `tolerance` of `p`:
/// later fixtures draw above earlier ones of their kind, so the list is read from its end.
pub(super) fn hit_outline(fixtures: &PageFixtures, p: Point, tolerance: f64) -> Option<NodeId> {
    fixtures
        .fixtures
        .iter()
        .rev()
        .filter(|f| f.shown())
        .filter(|f| near_box(&f.corners(&fixtures.page), p, tolerance) || !f.slots.is_empty())
        .find(|f| {
            f.outline(&fixtures.page)
                .into_iter()
                .any(|s| distance(p, s) <= tolerance)
        })
        .map(|f| f.id)
}

impl Session {
    /// A Select press on the canvas, before the objects' own (criteria 8 to 11). `page_before` is
    /// the page active before the press, so a press on another laser page's fixture only
    /// activates that page. `true` when a fixture took the press.
    pub(in crate::session) fn fixture_press(&mut self, point: Point, page_before: &str) -> bool {
        if self.tool != Tool::Select {
            return false;
        }
        let Some(fixtures) = self.active_fixtures().filter(|f| f.page.id == page_before) else {
            return false;
        };
        let scale = self.view().scale();
        if let Some(selected) = self
            .selected_fixture()
            .and_then(|id| fixtures.get(id))
            .cloned()
            && let Some(handle) = hit_fixture_handle(
                &selected,
                &fixtures.page,
                point,
                &self.transform_handle_tolerances(),
                self.held.shift,
            )
        {
            self.fixtures.last_press_handle = Some((selected.id, handle));
            self.begin_fixture_drag(&fixtures, selected.id, point, handle);
            return true;
        }
        if self.press_reaches_selection(point) {
            return false;
        }
        let px = if self.fixtures.coarse {
            HIT_PX.1
        } else {
            HIT_PX.0
        };
        if let Some(id) = hit_outline(&fixtures, point, px / scale) {
            self.select_fixture(id);
            self.edit_context.empty_press = None;
            let handle = if fixtures.get(id).is_some_and(movable) {
                FixtureHandle::Move
            } else {
                FixtureHandle::Hold
            };
            self.fixtures.last_press_handle = Some((id, handle));
            self.begin_fixture_drag(&fixtures, id, point, handle);
            return true;
        }
        // Inside the selected fixture's box, where no object is hit, the press moves it (D13).
        if let Some(selected) = self
            .selected_fixture()
            .and_then(|id| fixtures.get(id))
            .cloned()
            && movable(&selected)
            && inside(&selected.corners(&fixtures.page), point)
        {
            self.edit_context.empty_press = None;
            self.fixtures.last_press_handle = Some((selected.id, FixtureHandle::Move));
            self.begin_fixture_drag(&fixtures, selected.id, point, FixtureHandle::Move);
            return true;
        }
        self.fixtures.last_press_handle = None;
        self.clear_fixture_selection();
        false
    }
}
