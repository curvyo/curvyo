//! The Into slot list (`specs/0154-library-into-slots` criterion 10, UX notes section 8): the
//! holders of the active laser page with their target slots, their size, outcome and "filled",
//! and the slot each member lies in for the Layers rows.

use curvyo_document_core::{Length, NodeId};
use curvyo_laser_core::{FitMode, SlotShape};
use curvyo_ui_core::slot_drop_text::row_outcome_words;
use serde::Serialize;

use super::super::Session;
use super::into_slot::{SlotTarget, UnitBox};

/// One holder of the Into slot list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HolderTargets {
    /// The holder's full id text.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its eye is off ("· hidden").
    pub hidden: bool,
    /// How many of its target slots are filled.
    pub filled: usize,
    /// How many target slots it has.
    pub of: usize,
    /// Its target slots in creation order.
    pub slots: Vec<SlotTargetRow>,
}

/// One slot row of the Into slot list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotTargetRow {
    /// The slot's full id text.
    pub id: String,
    /// Its name.
    pub name: String,
    /// `rect` or `circle`.
    pub shape: &'static str,
    /// "30 × 30 mm" or "⌀ 25 mm".
    pub size: String,
    /// Whether a member's centre lies in it (computed, never stored).
    pub filled: bool,
    /// "50 %", "67 %, turned", "turned", "real size", "too large", or none.
    pub outcome: Option<String>,
}

/// The Into slot list: the holders of the active laser page with their target slots.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotTargetsView {
    /// The holders with target slots, in the Fixtures tab order.
    pub holders: Vec<HolderTargets>,
    /// The selected target slot, focused first (UX notes V22) and the target of a load gesture.
    pub selected: Option<String>,
}

/// A size in mm in words: whole millimetres without decimals, else one decimal.
fn mm_words(value: Length) -> String {
    let mm = value.as_mm();
    if (mm - mm.round()).abs() < 0.05 {
        format!("{mm:.0}")
    } else {
        format!("{mm:.1}")
    }
}

fn shape_word(target: &SlotTarget) -> &'static str {
    match target.slot.shape {
        SlotShape::Rectangle => "rect",
        SlotShape::Circle => "circle",
    }
}

fn size_words(target: &SlotTarget) -> String {
    let slot = &target.slot;
    match slot.shape {
        SlotShape::Rectangle => format!("{} × {} mm", mm_words(slot.width), mm_words(slot.height)),
        SlotShape::Circle => format!("⌀ {} mm", mm_words(slot.diameter)),
    }
}

pub(super) const fn mode_of(real_size: bool) -> FitMode {
    if real_size {
        FitMode::RealSize
    } else {
        FitMode::Fit
    }
}

impl Session {
    /// The Into slot list for a unit of `unit` (criterion 10); no holder when the active page
    /// has no target slot (the button is hidden then).
    #[must_use]
    pub fn slot_targets_view(&self, unit: Option<UnitBox>, real_size: bool) -> SlotTargetsView {
        let targets = self.slot_targets();
        let contents = self.slot_contents(&targets);
        let mut holders: Vec<HolderTargets> = Vec::new();
        for (target, content) in targets.iter().zip(&contents) {
            let filled = !content.is_empty();
            let outcome = unit.and_then(|unit| {
                row_outcome_words(&target.fit(unit, mode_of(real_size)), real_size)
            });
            let row = SlotTargetRow {
                id: target.id.full_id(),
                name: target.name.clone(),
                shape: shape_word(target),
                size: size_words(target),
                filled,
                outcome,
            };
            let id = target.holder.full_id();
            if holders.last().is_none_or(|h| h.id != id) {
                holders.push(HolderTargets {
                    id,
                    name: target.holder_name.clone(),
                    hidden: !target.drawn,
                    filled: 0,
                    of: 0,
                    slots: Vec::new(),
                });
            }
            if let Some(holder) = holders.last_mut() {
                holder.of += 1;
                holder.filled += usize::from(filled);
                holder.slots.push(row);
            }
        }
        let selected = self
            .selected_slot()
            .filter(|slot| targets.iter().any(|t| t.id == *slot))
            .map(NodeId::full_id);
        SlotTargetsView { holders, selected }
    }

    /// The slot each top-level member of the active laser page lies in, by its box centre (the
    /// Layers " · Slot 2", UX notes V17): `(member, slot name)` pairs.
    #[must_use]
    pub fn member_slots(&self) -> Vec<(NodeId, String)> {
        let targets = self.slot_targets();
        if targets.is_empty() {
            return Vec::new();
        }
        self.member_centres()
            .into_iter()
            .filter_map(|(id, centre)| {
                let target = targets
                    .iter()
                    .rev()
                    .find(|t| t.holds(centre, curvyo_laser_core::FIT_TOLERANCE))?;
                Some((id, target.name.clone()))
            })
            .collect()
    }

    /// `view` with "· 3 filled" after the slot count of each holder row with a filled slot (UX
    /// notes V18); computed, never stored.
    pub(super) fn with_filled_holders(&self, mut view: super::FixturesView) -> super::FixturesView {
        let filled = self.filled_slots();
        if filled.is_empty() {
            return view;
        }
        let ids: std::collections::HashSet<String> = filled.iter().map(|id| id.full_id()).collect();
        let rows = view
            .job_fixtures
            .iter_mut()
            .chain(view.sets.iter_mut().flat_map(|set| set.fixtures.iter_mut()));
        for row in rows {
            let count = row
                .slots
                .iter()
                .filter(|slot| ids.contains(&slot.id))
                .count();
            if count == 0 {
                continue;
            }
            let mut parts: Vec<String> = row.line2.split(" · ").map(str::to_owned).collect();
            parts.insert(parts.len().min(2), format!("{count} filled"));
            row.line2 = parts.join(" · ");
        }
        view
    }
}
