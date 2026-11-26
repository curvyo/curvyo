//! What a library drag over a slot of a laser page shows (`specs/0154-library-into-slots`
//! criteria 1, 8, 9; UX notes sections 3, 5).

use curvyo_document_core::{Length, Point};
use curvyo_laser_core::{SlotFit, slot_fit_matrix};
use curvyo_render_core::DrawList;
use curvyo_ui_core::slot_drop_text::{
    FIXTURE_SNAP_OFF_TEXT, fill_badge, fill_status, into_slot_badge, into_slot_status,
    no_empty_slot_text, short_name,
};
use serde::Serialize;

use super::super::Session;
use super::into_slot::{SlotTarget, UnitBox};
use super::into_slot_cue::mode_of;
use super::into_slot_ghost::{MAX_GHOST_LEAVES, UnitOutlines};
use super::into_slot_put::SlotSource;

/// The hit tolerance around a slot outline with a mouse and with a coarse pointer, in screen px.
const HIT_PX: (f64, f64) = (4.0, 12.0);

/// The targets of a library drag, read once at its start, and what the pointer is over now.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SlotDropState {
    /// The target slots of the active laser page, then those of the other laser pages.
    targets: Vec<SlotTarget>,
    filled: Vec<bool>,
    /// The page of each target: `None` for the active page, else the page a release activates.
    pages: Vec<Option<String>>,
    unit: Option<UnitBox>,
    /// The unit's content outlines once its source is read (UX notes V2).
    outlines: Option<UnitOutlines>,
    snapping: bool,
    /// The slot under the pointer, the fill flag and the real-size flag of the last cue.
    over: Option<(usize, bool, bool)>,
}

/// What the drag shows over the slot under the pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotDropCue {
    /// `slot` (a release puts the unit in), `fill` (Shift: every empty slot of the holder),
    /// `filled` (the release offers Replace), `refused` (nothing is written), or `none` (the drop
    /// of `0137`).
    pub kind: &'static str,
    /// The slot under the pointer, as its full id text.
    pub slot: Option<String>,
    /// Its holder, as its full id text.
    pub holder: Option<String>,
    /// The badge words.
    pub badge: Option<String>,
    /// The polite status words, said once per change (UX notes 3.1).
    pub status: Option<String>,
    /// The laser page a release activates first, when the slot lies on another laser page
    /// (criterion 9, Proposal 2026-11-26): the release then puts into the slot in one step.
    pub page: Option<String>,
}

impl SlotDropCue {
    const fn none() -> Self {
        Self {
            kind: "none",
            slot: None,
            holder: None,
            badge: None,
            status: None,
            page: None,
        }
    }
}

impl Session {
    /// Reads the targets of a library drag at its start (UX notes 3.1); `unit` is the dragged
    /// unit's box when it is known.
    pub fn begin_slot_drop(&mut self, unit: Option<UnitBox>) {
        let mut targets = self.slot_targets();
        let mut filled: Vec<bool> = self
            .slot_contents(&targets)
            .iter()
            .map(|c| !c.is_empty())
            .collect();
        let mut pages = vec![None; targets.len()];
        let others = self.other_page_targets();
        let mut on_page: Vec<(String, Vec<SlotTarget>)> = Vec::new();
        for (page, target) in others {
            match on_page.iter_mut().find(|(p, _)| *p == page) {
                Some((_, list)) => list.push(target),
                None => on_page.push((page, vec![target])),
            }
        }
        for (page, list) in on_page {
            filled.extend(
                self.slot_contents_on(&page, &list)
                    .iter()
                    .map(|c| !c.is_empty()),
            );
            pages.extend(std::iter::repeat_n(Some(page), list.len()));
            targets.extend(list);
        }
        self.fixtures.slot_drop = Some(SlotDropState {
            targets,
            filled,
            pages,
            unit,
            outlines: None,
            snapping: self.document.snapping().fixtures,
            over: None,
        });
    }

    /// Gives the drag in flight its source once it is read: the unit's box and its content
    /// outlines for the fitted ghost.
    pub fn set_slot_drop_source(&mut self, from: &SlotSource<'_>) {
        let Some(unit) = from.unit_parts() else {
            return;
        };
        if let Some(state) = self.fixtures.slot_drop.as_mut() {
            state.unit = Some(unit.0);
            state.outlines = UnitOutlines::of(&from.source.document, &unit.1);
        }
    }

    /// Ends a library drag (release, Escape, leaving the canvas).
    pub fn end_slot_drop(&mut self) {
        self.fixtures.slot_drop = None;
    }

    /// The cue of a library drag at the document point `at`: the slot under it (the one drawn
    /// last when slots overlap), with Alt (`real_size`) and Shift (`fill`).
    pub fn slot_drop_cue(&mut self, at: Point, real_size: bool, fill: bool) -> SlotDropCue {
        let px = if self.fixtures.coarse {
            HIT_PX.1
        } else {
            HIT_PX.0
        };
        let grow = Length::from_mm(px / self.view().scale());
        let Some(state) = self.fixtures.slot_drop.as_mut() else {
            return SlotDropCue::none();
        };
        state.over = None;
        // The active page's slots first (the one drawn last wins), then the other laser pages'.
        let active = state
            .pages
            .iter()
            .position(Option::is_some)
            .unwrap_or(state.targets.len());
        let hit =
            |range: std::ops::Range<usize>| range.rev().find(|&i| state.targets[i].holds(at, grow));
        let Some(index) = hit(0..active).or_else(|| hit(active..state.targets.len())) else {
            return SlotDropCue::none();
        };
        if !state.snapping {
            return SlotDropCue {
                kind: "refused",
                badge: Some(FIXTURE_SNAP_OFF_TEXT.to_owned()),
                ..SlotDropCue::none()
            };
        }
        state.over = Some((index, fill, real_size));
        let cue = if fill {
            fill_cue(state, index, real_size)
        } else {
            slot_cue(state, index, real_size)
        };
        SlotDropCue {
            page: state.pages[index].clone(),
            ..cue
        }
    }

    /// The lit slots and the fitted units of the drag's last cue (UX notes 3.1, 3.3, 5.2): the
    /// content outlines while they stay under the leaf limit, else the box alone.
    pub(super) fn slot_drop_overlay(&self) -> DrawList {
        let mut list = DrawList::default();
        let Some(state) = self.fixtures.slot_drop.as_ref() else {
            return list;
        };
        let Some((index, fill, real_size)) = state.over else {
            return list;
        };
        let view = self.view();
        let shown: Vec<usize> = if fill {
            (0..state.targets.len())
                .filter(|&i| state.targets[i].holder == state.targets[index].holder)
                .filter(|&i| !state.filled[i])
                .collect()
        } else {
            vec![index]
        };
        let outlines = state
            .outlines
            .as_ref()
            .filter(|o| o.leaves * shown.len() <= MAX_GHOST_LEAVES);
        for &i in &shown {
            let target = &state.targets[i];
            if !target.drawn {
                continue;
            }
            let taking = !state.filled[i];
            list.extend(curvyo_render_core::build_slot_cue(
                &target.outline,
                taking,
                view,
            ));
            let Some(unit) = state.unit.filter(|_| taking) else {
                continue;
            };
            let fit = target.fit(unit, mode_of(real_size));
            let mut segments = fitted_box(unit, fit, target);
            if let Some(outlines) = outlines {
                let m = slot_fit_matrix(unit.centre, fit, target.slot.centre, target.board);
                segments.extend(outlines.segments(m));
            }
            list.extend(curvyo_render_core::build_fixture_ghost(&segments, view));
        }
        list
    }
}

/// The cue of a Shift drag over the slot `index`: every empty slot of its holder.
fn fill_cue(state: &SlotDropState, index: usize, real_size: bool) -> SlotDropCue {
    let target = &state.targets[index];
    let slot = Some(target.id.full_id());
    let holder = Some(target.holder.full_id());
    let mates: Vec<usize> = (0..state.targets.len())
        .filter(|&i| state.targets[i].holder == target.holder)
        .collect();
    let empty: Vec<usize> = mates
        .iter()
        .copied()
        .filter(|&i| !state.filled[i])
        .collect();
    if empty.is_empty() {
        let text = no_empty_slot_text(&target.holder_name);
        return SlotDropCue {
            kind: "refused",
            slot,
            holder,
            badge: Some(text.clone()),
            status: Some(text),
            page: None,
        };
    }
    let mode = mode_of(real_size);
    let scales: Vec<f64> = state.unit.map_or_else(Vec::new, |unit| {
        empty
            .iter()
            .map(|&i| state.targets[i].fit(unit, mode).scale)
            .collect()
    });
    SlotDropCue {
        kind: "fill",
        slot,
        holder,
        badge: Some(fill_badge(empty.len(), mates.len(), &scales, real_size)),
        status: Some(fill_status(empty.len(), mates.len(), &target.holder_name)),
        page: None,
    }
}

/// The cue over the slot `index` without Shift: into it, or its filled refusal.
fn slot_cue(state: &SlotDropState, index: usize, real_size: bool) -> SlotDropCue {
    let target = &state.targets[index];
    let slot = Some(target.id.full_id());
    let holder = Some(target.holder.full_id());
    if state.filled[index] {
        let name = short_name(&target.name);
        return SlotDropCue {
            kind: "filled",
            slot,
            holder,
            badge: Some(format!("{name} is filled")),
            status: Some(format!("{name} is filled.")),
            page: None,
        };
    }
    let twin = state
        .targets
        .iter()
        .filter(|t| t.name == target.name)
        .count()
        > 1;
    let holder_name = twin.then_some(target.holder_name.as_str());
    let fit = state.unit.map_or(
        SlotFit {
            scale: 1.0,
            turned: false,
            matched: true,
            overflows: false,
        },
        |unit| target.fit(unit, mode_of(real_size)),
    );
    SlotDropCue {
        kind: "slot",
        slot,
        holder,
        badge: Some(into_slot_badge(&target.name, holder_name, &fit, real_size)),
        status: Some(into_slot_status(
            &target.name,
            &target.holder_name,
            &fit,
            real_size,
        )),
        page: None,
    }
}

/// The four sides of `unit`'s box fitted into `target`, in document space.
fn fitted_box(unit: UnitBox, fit: SlotFit, target: &SlotTarget) -> Vec<(Point, Point)> {
    let m = slot_fit_matrix(Point::new(0.0, 0.0), fit, target.slot.centre, target.board);
    let (hw, hh) = (unit.size.0.as_mm() / 2.0, unit.size.1.as_mm() / 2.0);
    let corners =
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| m.apply_point(Point::new(x, y)));
    (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect()
}
