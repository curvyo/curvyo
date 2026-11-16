//! Lock of a fixture (`specs/0113-fixture-sets-and-pins` criteria 31 and 32): the toggle of its
//! row and of the Select bar, and the one gate the drag, nudge, handle and chip paths ask.

use curvyo_document_core::NodeId;

use super::super::Session;
use super::placed::PlacedFixture;

/// Whether `fixture` may change its place or size: it moves and it is not locked.
pub(super) fn may_move(fixture: &PlacedFixture) -> bool {
    super::commands::movable(fixture) && !fixture.keeps_place()
}

impl Session {
    /// Lock or Unlock of the fixture `id`, a job fixture or a set fixture (whose Lock is the
    /// profile's and writes it): one step "Lock fixture" or "Unlock fixture". `false` off the
    /// active laser page, and for a set fixture while its set is locked (its Lock is not
    /// pressable then). A drag or a chip of the fixture in flight ends.
    pub fn set_fixture_locked(&mut self, id: NodeId, locked: bool) -> bool {
        let Some((_, fixture)) = self.page_fixture(id) else {
            return false;
        };
        if fixture.set_locked {
            return false;
        }
        let changed = self.set_content(id, |this| {
            this.document
                .set_fixture_locked(id, locked)
                .unwrap_or(false)
        });
        if changed && locked && self.fixtures.selected.is_some_and(|s| s.fixture == id) {
            self.fixtures.drag = None;
            self.fixtures.entry = None;
            self.fixtures.armed = None;
        }
        changed
    }

    /// Whether the selected fixture is locked: the Select bar's Lock is pressed (a set fixture
    /// of a locked set reads pressed). `None` without a selected fixture.
    #[must_use]
    pub fn selected_fixture_lock(&self) -> Option<bool> {
        let (_, fixture) = self
            .selected_fixture()
            .and_then(|id| self.page_fixture(id))?;
        Some(fixture.locked())
    }

    /// Whether the selected fixture's Lock can be pressed: not while its set is locked
    /// (criterion 32). `None` without a selected fixture.
    #[must_use]
    pub fn selected_fixture_lock_pressable(&self) -> Option<bool> {
        let (_, fixture) = self
            .selected_fixture()
            .and_then(|id| self.page_fixture(id))?;
        Some(!fixture.set_locked && (fixture.is_job_fixture() || self.fixture_sets_editable()))
    }
}
