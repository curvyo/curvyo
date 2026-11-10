//! The fit rule during a drag (`specs/0102-laser-fixtures` criteria 25 and 26, Frage 173 A): a
//! moved item whose box matches a slot within 0.1 mm, in its own orientation or turned by 90
//! degrees, and whose centre comes within the capture distance of the slot's centre, snaps
//! centre to centre. The slots are read once at the drag's start.

use curvyo_document_core::{Affine, Angle, Length, NodeId, ObjectSnapshot, Point, Vec2};
use curvyo_laser_core::{Slot, SlotShape, fit_slot};
use curvyo_render_core::SlotOutline;
use serde::Serialize;

use super::super::Session;
use super::placed::{PageFixtures, PlacedFixture};

/// The fit tolerance, centre to centre (customer, 2026-11-10).
const FIT_TOLERANCE_MM: f64 = 0.1;
/// A holder's turn counts as square to the page within this many radians.
const SQUARE_EPS: f64 = 1e-6;
/// The commit label of a fitted move ("Move to slot", a MOVE step).
pub(in crate::session) const MOVE_TO_SLOT_LABEL: &str = "move_to_slot";

/// What the drag answer shows about a fit: the slot lit, its name for the Drop badge ("Fits
/// Slot 2" or "Fits Slot 2, turned") and whether the item turns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FitCue {
    /// The slot's node id, as its full id text.
    pub slot: String,
    /// The slot's name.
    pub name: String,
    /// Whether the item turns by 90 degrees to fit.
    pub turned: bool,
}

/// One slot a moved item may fit, in document space.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FitSlot {
    pub slot: NodeId,
    pub name: String,
    pub centre: Point,
    /// The slot with its size along the page's axes.
    page_slot: Slot,
    /// Its outline, for the cue.
    pub outline: SlotOutline,
    /// Whether its holder is drawn (the eye): the cue is drawn only then (UX notes §7.2).
    pub drawn: bool,
}

/// The current fit of the drag.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Fit {
    pub index: usize,
    /// The move that puts the item's centre on the slot's centre.
    pub offset: Vec2,
    pub turned: bool,
}

/// The fit targets of the drag in flight and its current fit.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct FitState {
    pub slots: Option<Vec<FitSlot>>,
    pub current: Option<Fit>,
}

/// The slots of `holder`, sized along the page's axes; none when the holder is turned by
/// anything but a multiple of 90 degrees (an item is never turned by another angle, Proposal).
fn holder_slots(holder: &PlacedFixture, out: &mut Vec<FitSlot>) {
    let m = holder.global;
    let quarter = if m.b.abs() < SQUARE_EPS && m.c.abs() < SQUARE_EPS {
        false
    } else if m.a.abs() < SQUARE_EPS && m.d.abs() < SQUARE_EPS {
        true
    } else {
        return;
    };
    for placed in &holder.slots {
        let slot = placed.slot;
        let mut page_slot = slot;
        page_slot.centre = m.apply_point(slot.centre);
        if quarter {
            (page_slot.width, page_slot.height) = (slot.height, slot.width);
        }
        out.push(FitSlot {
            slot: placed.id,
            name: placed.name.clone(),
            centre: page_slot.centre,
            page_slot,
            outline: outline_of(&slot, m),
            drawn: holder.visible,
        });
    }
}

/// A slot's outline mapped by its board's `m`.
pub(super) fn outline_of(slot: &Slot, m: Affine) -> SlotOutline {
    let (w, h) = slot.extent();
    match slot.shape {
        SlotShape::Circle => SlotOutline::Circle {
            centre: m.apply_point(slot.centre),
            diameter_mm: w.as_mm(),
        },
        SlotShape::Rectangle => {
            let (hw, hh) = (w.as_mm() / 2.0, h.as_mm() / 2.0);
            let (cx, cy) = (slot.centre.x, slot.centre.y);
            SlotOutline::Rect(
                [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
                    .map(|(x, y)| m.apply_point(Point::new(cx + x, cy + y))),
            )
        }
    }
}

/// The fit slots of the Active holders of `pages`.
fn fit_slots(pages: &[PageFixtures]) -> Vec<FitSlot> {
    let mut out = Vec::new();
    for page in pages {
        for holder in page.fixtures.iter().filter(|f| f.active) {
            holder_slots(holder, &mut out);
        }
    }
    out
}

impl Session {
    /// Reads the fit targets at a drag's start: the Active holders of the active laser page, or
    /// with `every_page` of every laser page (a page row drag). None while the Fixtures switch
    /// is off.
    pub(in crate::session) fn begin_fixture_fit(&mut self, every_page: bool) {
        let pages = if !self.document.snapping().fixtures {
            Vec::new()
        } else if every_page {
            self.available_page_fixtures()
        } else {
            self.active_fixtures().into_iter().collect()
        };
        let slots = fit_slots(&pages);
        self.fixtures.fit = FitState {
            slots: (!slots.is_empty()).then_some(slots),
            current: None,
        };
    }

    /// The fit of a moved item whose box is `min`–`max` now: the move that puts its centre on
    /// the nearest fitting slot's centre, or `None` (the drag lands as built).
    pub(in crate::session) fn fixture_fit(&mut self, min: Point, max: Point) -> Option<Vec2> {
        let capture = self.document.snapping().capture_px / self.view().scale();
        let state = &mut self.fixtures.fit;
        state.current = None;
        let slots = state.slots.as_ref()?;
        let item = (
            Length::from_mm(max.x - min.x),
            Length::from_mm(max.y - min.y),
        );
        let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        let tolerance = Length::from_mm(FIT_TOLERANCE_MM);
        let best = slots
            .iter()
            .enumerate()
            .filter(|(_, s)| centre.vector_to(s.centre).length() <= capture)
            .filter_map(|(index, s)| {
                let fit = fit_slot(item, std::slice::from_ref(&s.page_slot), tolerance)?;
                Some((index, fit.turned, centre.vector_to(s.centre)))
            })
            .min_by(|a, b| a.2.length().total_cmp(&b.2.length()))?;
        let (index, turned, offset) = best;
        state.current = Some(Fit {
            index,
            offset,
            turned,
        });
        Some(offset)
    }

    /// Forgets the current fit before a move that may not ask for one.
    pub(in crate::session) fn clear_fixture_fit_match(&mut self) {
        self.fixtures.fit.current = None;
    }

    /// Ends the fit of a drag (release, Escape).
    pub(in crate::session) fn end_fixture_fit(&mut self) {
        self.fixtures.fit = FitState::default();
    }

    /// The current fit's slot and whether the item turns, for the cue and the release.
    pub(super) fn current_fit(&self) -> Option<(&FitSlot, &Fit)> {
        let state = &self.fixtures.fit;
        let fit = state.current.as_ref()?;
        Some((state.slots.as_ref()?.get(fit.index)?, fit))
    }

    /// The fit cue of the drag in flight: the slot lit and the Drop badge words (criterion 25).
    #[must_use]
    pub fn fixture_fit_cue(&self) -> Option<FitCue> {
        let (slot, fit) = self.current_fit()?;
        Some(FitCue {
            slot: slot.slot.full_id(),
            name: slot.name.clone(),
            turned: fit.turned,
        })
    }

    /// The selected objects where a release with the current fit puts them: moved centre to
    /// centre, turned by 90 degrees about the slot's centre when the fit needs it.
    pub(super) fn fitted_objects(&self) -> Option<Vec<ObjectSnapshot>> {
        let (slot, fit) = self.current_fit()?;
        let (centre, offset, turned) = (slot.centre, fit.offset, fit.turned);
        let live = self
            .pointer_position
            .and_then(|p| self.select.live_move(p, self.held.shift, self.held.ctrl))
            .map_or(Vec2::ZERO, |live| live.offset);
        let ids: std::collections::HashSet<NodeId> = self.selection.ids().iter().copied().collect();
        let quarter = Angle::from_radians(std::f64::consts::FRAC_PI_2);
        Some(
            self.objects()
                .iter()
                .filter(|o| ids.contains(&o.id()))
                .map(|o| {
                    let moved = o.translated(live).translated(offset);
                    if turned {
                        moved.rotated(centre, quarter)
                    } else {
                        moved
                    }
                })
                .collect(),
        )
    }

    /// The release of a Select move with a fit: the moved objects land as
    /// [`Session::fitted_objects`] says, as one step "Move to slot". `false` without a fit.
    pub(in crate::session) fn release_fixture_fit(&mut self) -> bool {
        let Some(results) = self.fitted_objects() else {
            return false;
        };
        self.select.escape();
        self.end_fixture_fit();
        if !results.is_empty() {
            self.recording_unguarded(|this| {
                this.document
                    .batch(MOVE_TO_SLOT_LABEL, |d| d.transform_objects(&results, false))
                    .is_ok()
            });
        }
        true
    }
}
