//! The target slots of the active laser page, each with its computed filled state
//! (`specs/0154-library-into-slots` Terms, criteria 7 to 10, ADR 0088 decision 4).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{Affine, Length, NodeId, ObjectSnapshot, Point};
use curvyo_laser_core::{FIT_TOLERANCE, FitMode, FixtureKind, Slot, SlotFit, fit_into_slot};
use curvyo_render_core::SlotOutline;

use super::super::Session;
use super::fit::outline_of;
use super::placed::PageFixtures;
use super::read_cache::ReadKey;

/// The filled slots of the active page for the read and the page they were computed for.
pub(super) type FilledCache = RefCell<Option<(ReadKey, String, Arc<HashSet<NodeId>>)>>;

/// One slot of an Active holder of the active laser page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SlotTarget {
    pub id: NodeId,
    pub name: String,
    pub holder: NodeId,
    pub holder_name: String,
    /// Whether the holder is drawn (its eye); a hidden holder is still a target.
    pub drawn: bool,
    /// The slot in its board's frame.
    pub slot: Slot,
    /// The board's frame to document space (rigid, ADR 0049 rule 6).
    pub board: Affine,
    pub outline: SlotOutline,
}

impl SlotTarget {
    /// How `unit` fits the slot, in the board's frame (a turned board turns the unit with it).
    pub(super) fn fit(&self, unit: UnitBox, mode: FitMode) -> SlotFit {
        fit_into_slot(unit.size, &self.slot, mode)
    }

    /// Whether the document point `point` lies in the slot grown by `grow` mm.
    pub(super) fn holds(&self, point: Point, grow: Length) -> bool {
        self.board
            .inverse()
            .is_some_and(|inverse| self.slot.holds(inverse.apply_point(point), grow))
    }
}

/// The box of the unit that goes into a slot: its size and its centre in the source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitBox {
    /// Width and height.
    pub size: (Length, Length),
    /// The box centre, in the source document's space.
    pub centre: Point,
}

impl UnitBox {
    /// The box between `min` and `max`.
    #[must_use]
    pub fn between(min: Point, max: Point) -> Self {
        Self {
            size: (
                Length::from_mm(max.x - min.x),
                Length::from_mm(max.y - min.y),
            ),
            centre: Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
        }
    }
}

impl Session {
    /// The slots of the Active holders of the active laser page, holders in listing order (sets,
    /// then job fixtures), slots in creation order; damaged slots are not read (`0102`
    /// criterion 33). Empty when the active page is no laser page.
    pub(super) fn slot_targets(&self) -> Vec<SlotTarget> {
        self.active_fixtures()
            .map_or_else(Vec::new, |page| targets_of(&page))
    }

    /// The target slots of the other laser pages, with their page's id: a drop over one
    /// activates that page (criterion 9, Proposal).
    pub(super) fn other_page_targets(&self) -> Vec<(String, SlotTarget)> {
        let active = self.active_page().id;
        self.available_page_fixtures()
            .iter()
            .filter(|page| page.page.id != active)
            .flat_map(|page| {
                targets_of(page)
                    .into_iter()
                    .map(|t| (page.page.id.clone(), t))
            })
            .collect()
    }

    /// The filled target slots of the active laser page, computed once per document version and
    /// page (the canvas labels read it every frame, UX notes V16).
    pub(super) fn filled_slots(&self) -> Arc<HashSet<NodeId>> {
        let key = self.read_key();
        let page = self.active_page().id;
        if let Some((held, held_page, filled)) = self.fixtures.filled_cache.borrow().as_ref()
            && *held == key
            && *held_page == page
        {
            return Arc::clone(filled);
        }
        let targets = self.slot_targets();
        let filled: HashSet<NodeId> = targets
            .iter()
            .zip(self.slot_contents(&targets))
            .filter(|(_, content)| !content.is_empty())
            .map(|(target, _)| target.id)
            .collect();
        let filled = Arc::new(filled);
        *self.fixtures.filled_cache.borrow_mut() = Some((key, page, Arc::clone(&filled)));
        filled
    }

    /// The top-level members of the active page that fill slots, with their box centre: own items
    /// and groups (a whole group once) by the membership read and the page's placed pages by their
    /// frame; dimensions never fill (Terms "Filled").
    pub(super) fn member_centres(&self) -> Vec<(NodeId, Point)> {
        self.member_centres_on(&self.active_page().id)
    }

    /// [`Session::member_centres`] of the page `page`.
    pub(super) fn member_centres_on(&self, page: &str) -> Vec<(NodeId, Point)> {
        let objects = self.objects();
        let by_id: HashMap<NodeId, &ObjectSnapshot> = objects.iter().map(|o| (o.id(), o)).collect();
        let mut out = Vec::new();
        for placed in self.document.placed_pages() {
            if placed.output.id != page {
                continue;
            }
            let (Some(source), Some(map)) = (&placed.source, placed.global_map()) else {
                continue;
            };
            let centre = Point::new(
                source.origin.x + source.size.width.as_mm() / 2.0,
                source.origin.y + source.size.height.as_mm() / 2.0,
            );
            out.push((placed.id, map.apply_point(centre)));
        }
        // Guides and dimensions never fill a slot (Terms "Filled").
        let guides = Arc::clone(&self.document_read().guides);
        for top in self.page_top_level(page) {
            if out.iter().any(|(id, _)| *id == top) {
                continue;
            }
            let leaves = if self.document.is_group(top) {
                self.document.leaves_of(top)
            } else {
                vec![top]
            };
            let boxes: Vec<(Point, Point)> = leaves
                .iter()
                .filter_map(|leaf| by_id.get(leaf))
                .filter(|o| !matches!(o, ObjectSnapshot::Dimension(_)) && !guides.contains(&o.id()))
                .map(|o| curvyo_ui_core::object_outline_bounds(o))
                .collect();
            if let Some((min, max)) = joint(&boxes) {
                out.push((top, UnitBox::between(min, max).centre));
            }
        }
        out
    }

    /// The members whose centre lies in each of `targets`, in the order of `targets`.
    pub(super) fn slot_contents(&self, targets: &[SlotTarget]) -> Vec<Vec<NodeId>> {
        self.slot_contents_on(&self.active_page().id, targets)
    }

    /// [`Session::slot_contents`] of `targets` on the page `page`.
    pub(super) fn slot_contents_on(&self, page: &str, targets: &[SlotTarget]) -> Vec<Vec<NodeId>> {
        let centres = self.member_centres_on(page);
        targets
            .iter()
            .map(|target| {
                centres
                    .iter()
                    .filter(|(_, centre)| target.holds(*centre, FIT_TOLERANCE))
                    .map(|(id, _)| *id)
                    .collect()
            })
            .collect()
    }
}

/// The slots of the Active holders of `page`, holders in listing order (sets, then job fixtures),
/// slots in creation order; damaged slots are not read (`0102` criterion 33).
fn targets_of(page: &PageFixtures) -> Vec<SlotTarget> {
    let mut out = Vec::new();
    for holder in page
        .fixtures
        .iter()
        .filter(|f| f.active && f.kind == FixtureKind::Holder)
    {
        for placed in &holder.slots {
            out.push(SlotTarget {
                id: placed.id,
                name: placed.name.clone(),
                holder: holder.id,
                holder_name: holder.name.clone(),
                drawn: holder.visible,
                slot: placed.slot,
                board: holder.global,
                outline: outline_of(&placed.slot, holder.global),
            });
        }
    }
    out
}

/// The joint box of `boxes`, or `None` without any.
pub(super) fn joint(boxes: &[(Point, Point)]) -> Option<(Point, Point)> {
    boxes.iter().copied().reduce(|(a, b), (c, d)| {
        (
            Point::new(a.x.min(c.x), a.y.min(c.y)),
            Point::new(b.x.max(d.x), b.y.max(d.y)),
        )
    })
}
