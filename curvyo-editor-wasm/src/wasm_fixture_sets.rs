//! Thin host bridge of the Fixtures tab (`specs/0113-fixture-sets-and-pins`): its machine sets,
//! the add bars, armed placement of a job fixture, the pin and screw fields, Lock and Apply
//! fixtures from page. Ids are the full id texts the view JSON carries.

use curvyo_document_core::NodeId;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// "+ Set": the new set's id, `None` at 16 sets or off a laser page (criterion 3).
    pub fn add_fixture_set(&mut self) -> Option<String> {
        self.session.add_fixture_set().map(NodeId::full_id)
    }

    /// Renames the set `id` (criterion 4).
    pub fn rename_fixture_set(&mut self, id: &str, name: &str) -> bool {
        self.set(id)
            .is_some_and(|id| self.session.rename_fixture_set(id, name))
    }

    /// The second press of a set's armed trash (criterion 5).
    pub fn remove_fixture_set(&mut self, id: &str) -> bool {
        self.set(id)
            .is_some_and(|id| self.session.remove_fixture_set(id))
    }

    /// Moves the set `id` to `index` among the sets (criterion 6).
    pub fn move_fixture_set(&mut self, id: &str, index: u32) -> bool {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.set(id)
            .is_some_and(|id| self.session.move_fixture_set(id, index))
    }

    /// The On this page check of the set `id` (criteria 2, 8).
    pub fn set_fixture_set_on_page(&mut self, id: &str, on_page: bool) -> bool {
        self.set(id)
            .is_some_and(|id| self.session.set_fixture_set_on_page(id, on_page))
    }

    /// Folds or unfolds the set `id`; not a step (criterion 7).
    pub fn set_fixture_set_unfolded(&mut self, id: &str, unfolded: bool) -> bool {
        self.set(id)
            .is_some_and(|id| self.session.set_fixture_set_unfolded(id, unfolded))
    }

    /// A kind of the set `set`'s add bar: `grid`, `holder`, `clamp`, `stop`, `screw`, `pin`
    /// (criterion 7). The new fixture's id.
    pub fn add_set_fixture(&mut self, set: &str, kind: &str) -> Option<String> {
        let set = self.set(set)?;
        self.session.add_set_fixture(set, kind).map(NodeId::full_id)
    }

    /// A kind of the Job fixtures add bar: arms its placement, or ends it on a second press.
    /// Whether placement is armed now (criterion 13).
    pub fn arm_job_fixture(&mut self, kind: &str) -> bool {
        self.session.arm_job_fixture(kind)
    }

    /// Ends placement (Escape, a right click): whether it was armed.
    pub fn end_fixture_placement(&mut self) -> bool {
        self.session.end_fixture_placement()
    }

    /// Keyboard placement: moves the hanging fixture by `dx`, `dy` mm (UX notes 5.1).
    pub fn nudge_fixture_placement(&mut self, dx: f64, dy: f64) -> bool {
        self.session.nudge_fixture_placement(dx, dy)
    }

    /// Keyboard placement: Enter drops the hanging fixture; its id.
    pub fn drop_fixture_placement(&mut self) -> Option<String> {
        self.session.drop_fixture_placement().map(NodeId::full_id)
    }

    /// A field of a pin or screw row: `diameter_mm`, `columns`, `rows`, `column_spacing_mm`,
    /// `row_spacing_mm`, `mark_mm` (criteria 18 to 20, 25).
    pub fn set_fixture_grid_value(&mut self, id: &str, key: &str, value: f64) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_fixture_grid_value(id, key, value))
    }

    /// Lock or Unlock of the job fixture `id`, in its row or the Select bar (criterion 31).
    pub fn set_fixture_locked(&mut self, id: &str, locked: bool) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_fixture_locked(id, locked))
    }

    /// The Lock of the set `id` (criterion 32): writes the profile.
    pub fn set_fixture_set_locked(&mut self, id: &str, locked: bool) -> bool {
        self.set(id)
            .is_some_and(|id| self.session.set_fixture_set_locked(id, locked))
    }

    /// Whether the selected fixture's Lock can be pressed (not while its set is locked);
    /// `undefined` without a selected fixture.
    pub fn selected_fixture_lock_pressable(&self) -> Option<bool> {
        self.session.selected_fixture_lock_pressable()
    }

    /// Mixes `crypto.getRandomValues` bytes into the profile ids of new sets and fixtures.
    pub fn seed_record_ids(&mut self, random: &[u8]) {
        self.session.seed_record_ids(random);
    }

    /// The Select bar's Lock of the selected job fixture; `None` hides it.
    pub fn selected_fixture_lock(&self) -> Option<bool> {
        self.session.selected_fixture_lock()
    }

    /// Picks the page `id` of the Apply list, or with `None` folds it (criteria 33, 34).
    pub fn set_apply_preview(&mut self, id: Option<String>) -> bool {
        match id {
            None => self.session.set_apply_preview(None),
            Some(id) => self
                .session
                .apply_candidate_named(&id)
                .is_some_and(|source| self.session.set_apply_preview(Some(source))),
        }
    }

    /// "Apply <n> fixtures" from the page `id`: how many were copied, 0 for none (criterion 35).
    pub fn apply_fixtures_from_page(&mut self, id: &str) -> u32 {
        self.session
            .apply_candidate_named(id)
            .and_then(|source| self.session.apply_fixtures_from_page(source))
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
    }

    fn set(&self, id: &str) -> Option<NodeId> {
        self.session.fixture_set_named(id)
    }
}
