//! The laser fixtures of the active laser page as session state (`specs/0102-laser-fixtures`,
//! ADR 0049, feature `adrs.md` §6): the one feature-owned field the session holds for fixtures.
//! Other session modules reach it only through the methods of this module and its children.

mod chips;
mod commands;
mod drag;
mod draw;
mod edit;
mod entry;
mod feedback;
mod fit;
mod handles;
mod hit;
mod jobs;
mod keys;
mod params;
mod placed;
mod slots;
mod snap;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_canvas;
#[cfg(test)]
mod tests_fit;
#[cfg(test)]
mod tests_handles;
#[cfg(test)]
mod tests_refusal;
#[cfg(test)]
mod tests_select;
mod view_json;

use curvyo_document_core::NodeId;
use curvyo_laser_core::FixtureKind;

pub use fit::FitCue;
pub use jobs::{
    AddTarget, FixtureJobsView, JobGroupView, JobKindView, JobRowView, PartRowView, PartsView,
};
pub use view_json::{
    FixtureKindView, FixtureLabel, FixtureRow, FixturesView, GridValues, SlotRow, TapesView,
};

use super::Session;
use placed::PageFixtures;

/// What a selection of a fixture holds: the fixture, and the slot pressed in its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Selected {
    fixture: NodeId,
    slot: Option<NodeId>,
}

/// What the session keeps for fixtures. View state only; nothing here is saved.
#[derive(Debug, Default)]
pub(in crate::session) struct FixtureState {
    /// The selected fixture: never part of an object selection (criterion 8).
    selected: Option<Selected>,
    /// The fixture a Delete on the canvas armed (UX notes D17); a second Delete removes it.
    armed: Option<NodeId>,
    /// A move, turn, resize or knob drag of the selected fixture in flight.
    drag: Option<drag::FixtureDrag>,
    /// The handle the last press on a fixture took: a double-click opens its chip.
    last_press_handle: Option<(NodeId, handles::FixtureHandle)>,
    /// The open chip of the selected fixture (`adrs.md` §6a.4).
    entry: Option<entry::FixtureEntry>,
    /// The fit targets and the current fit of a moved item (criteria 25, 26).
    fit: fit::FitState,
    /// The grid's layout for the parameters it was built from (criterion 35 d).
    grid_cache: draw::GridCache,
    /// The last notice of a fixture command, until the host takes it.
    notice: Option<String>,
    /// A coarse pointer: the hit tolerance is 12 px instead of 4 (UX notes D12).
    coarse: bool,
    /// The fixture job whose Parts list is open (`specs/0103` criterion 8, ADR 0052).
    job: Option<NodeId>,
}

impl Session {
    /// The fixtures of the active page while it is a laser page with its extension on.
    fn active_fixtures(&self) -> Option<PageFixtures> {
        let manufacturing = self.active_manufacturing()?;
        if !self.manufacturing_available(manufacturing.id) {
            return None;
        }
        PageFixtures::read(&manufacturing)
    }

    /// The selected fixture, `None` when it is gone, off the bed, hidden, on a page that is not
    /// active, or when objects are selected (criteria 5, 6, 8, 11).
    #[must_use]
    pub fn selected_fixture(&self) -> Option<NodeId> {
        let selected = self.fixtures.selected?;
        if !self.selection.ids().is_empty() {
            return None;
        }
        self.active_fixtures()?
            .get(selected.fixture)
            .filter(|f| f.shown())
            .map(|f| f.id)
    }

    /// The slot selected in the selected fixture's row, if it still exists.
    #[must_use]
    pub fn selected_slot(&self) -> Option<NodeId> {
        let fixture = self.selected_fixture()?;
        let slot = self.fixtures.selected?.slot?;
        let fixtures = self.active_fixtures()?;
        fixtures
            .get(fixture)?
            .slots
            .iter()
            .any(|s| s.id == slot)
            .then_some(slot)
    }

    /// The active laser page's fixtures as its Layers tree lists them, in creation order: id,
    /// name, kind, Active and eye (`specs/0102` criterion 12).
    pub(in crate::session) fn fixture_tree(
        &self,
    ) -> Vec<(NodeId, String, FixtureKind, bool, bool, bool)> {
        let selected = self.selected_fixture();
        self.active_fixtures().map_or_else(Vec::new, |page| {
            page.fixtures
                .iter()
                .map(|f| {
                    (
                        f.id,
                        f.name.clone(),
                        f.kind,
                        f.active,
                        f.visible,
                        selected == Some(f.id),
                    )
                })
                .collect()
        })
    }

    /// Selects the fixture `id` of the active laser page alone (its row in the Fixtures block,
    /// criterion 10): the object selection is cleared. `false` for anything else.
    pub fn select_fixture(&mut self, id: NodeId) -> bool {
        let Some(fixtures) = self.active_fixtures() else {
            return false;
        };
        if !fixtures.get(id).is_some_and(placed::PlacedFixture::shown) {
            return false;
        }
        self.selection.clear();
        self.node.clear_selection();
        if self.fixtures.selected.is_none_or(|s| s.fixture != id) {
            self.fixtures.entry = None;
        }
        self.fixtures.selected = Some(Selected {
            fixture: id,
            slot: None,
        });
        self.fixtures.armed = None;
        true
    }

    /// Selects the slot `slot` in its holder's row; the holder is the canvas selection.
    pub fn select_slot(&mut self, slot: NodeId) -> bool {
        let Some(fixtures) = self.active_fixtures() else {
            return false;
        };
        let Some(holder) = fixtures
            .fixtures
            .iter()
            .find(|f| f.slots.iter().any(|s| s.id == slot))
            .map(|f| f.id)
        else {
            return false;
        };
        if !self.select_fixture(holder) {
            return false;
        }
        self.fixtures.selected = Some(Selected {
            fixture: holder,
            slot: Some(slot),
        });
        true
    }

    /// Ends the fixture selection: an object press, Select all, a page switch or Escape.
    pub(in crate::session) fn clear_fixture_selection(&mut self) -> bool {
        self.fixtures.armed = None;
        self.fixtures.drag = None;
        self.fixtures.entry = None;
        self.fixtures.last_press_handle = None;
        self.fixtures.selected.take().is_some()
    }

    /// The fixture or slot of the active laser page named by its full id text.
    #[must_use]
    pub fn fixture_part_named(&self, name: &str) -> Option<NodeId> {
        let fixtures = self.active_fixtures()?;
        let manufacturing = self.document.manufacturing(fixtures.manufacturing)?;
        manufacturing.fixtures.iter().find_map(|f| {
            if f.id.full_id() == name {
                return Some(f.id);
            }
            f.slots
                .iter()
                .find(|s| s.id.full_id() == name)
                .map(|s| s.id)
        })
    }

    /// Posts a fixture notice from another part of the session (a profile pick that removed
    /// fixtures, criterion 29).
    pub(in crate::session) fn post_fixture_notice(&mut self, text: &str) {
        self.fixtures.notice = Some(text.to_owned());
    }

    /// The notice of the last fixture command, once: a clamped value, a slot past its board,
    /// the grid's Delete (criteria 13, 15, 19, 20).
    pub fn take_fixture_notice(&mut self) -> Option<String> {
        self.fixtures.notice.take()
    }

    /// Whether the pointer is coarse (touch): the outline hit grows to 12 px (UX notes D12).
    pub fn set_coarse_pointer(&mut self, coarse: bool) {
        self.fixtures.coarse = coarse;
    }
}
