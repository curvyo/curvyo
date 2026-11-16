//! The machine sets of the active laser page as the Fixtures tab edits them
//! (`specs/0113-fixture-sets-and-pins` criteria 2 to 8, 32): add, rename, remove, move, Lock and
//! On this page, each one step, plus which sets are unfolded (session only).

use std::collections::HashSet;

use curvyo_document_core::{FixtureSetSnapshot, NodeId};

use super::super::Session;

/// The sets unfolded in the Fixtures tab: kept for the session only, never an undo step
/// (criterion 7).
#[derive(Debug, Clone, Default)]
pub(in crate::session) struct SetFolds {
    unfolded: HashSet<NodeId>,
}

impl SetFolds {
    /// Whether the set `id` is unfolded.
    pub(super) fn is_unfolded(&self, id: NodeId) -> bool {
        self.unfolded.contains(&id)
    }
}

impl Session {
    /// The set `id` of the active laser page.
    fn page_set(&self, id: NodeId) -> Option<FixtureSetSnapshot> {
        self.active_fixtures()?
            .sets
            .into_iter()
            .find(|set| set.id == id)
    }

    /// "+ Set" (criterion 3): an empty set "Set N" added to the page's profile, On this page,
    /// unfolded; one step "Add fixture set" that also writes the record and adds the set, off, to
    /// every other page on that profile (Frage 218 A). `None` off a laser page, at 16 sets, or
    /// while the sets are not editable here (Generic laser, a profile not on this computer): the
    /// button is hidden then.
    pub fn add_fixture_set(&mut self) -> Option<NodeId> {
        let manufacturing = self.active_fixtures()?.manufacturing;
        if !self.fixture_sets_editable() {
            return None;
        }
        let uid = self.new_record_id();
        let id = self.set_edit(|this| this.document.add_fixture_set(manufacturing, &uid).ok())?;
        self.fixtures.sets.unfolded.insert(id);
        Some(id)
    }

    /// Renames the set `id` in place (criterion 4): cleaned, cut to 40 characters; one step
    /// "Rename fixture set".
    pub fn rename_fixture_set(&mut self, id: NodeId, name: &str) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        self.set_content(id, |this| {
            this.document.rename_fixture_set(id, name).unwrap_or(false)
        })
    }

    /// The second press of a set's armed trash (criterion 5): the set with its fixtures and their
    /// jobs; one step "Remove fixture set".
    pub fn remove_fixture_set(&mut self, id: NodeId) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        let in_set = self.fixture_in_set(id);
        let removed = self.set_content(id, |this| this.document.remove_fixture_set(id).is_ok());
        if removed && in_set {
            self.clear_fixture_selection();
        }
        removed
    }

    /// Moves the set `id` to `index` among the page's sets (criterion 6); one step "Move fixture
    /// set".
    pub fn move_fixture_set(&mut self, id: NodeId, index: usize) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        self.set_content(id, |this| {
            this.document.move_fixture_set(id, index).unwrap_or(false)
        })
    }

    /// The Lock of the set `id` (criterion 32): every fixture in it keeps its place; one step
    /// "Lock fixture set" or "Unlock fixture set" that writes the profile. A drag or chip of a
    /// fixture in it ends.
    pub fn set_fixture_set_locked(&mut self, id: NodeId, locked: bool) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        let changed = self.set_content(id, |this| {
            this.document
                .set_fixture_set_locked(id, locked)
                .unwrap_or(false)
        });
        if changed && locked && self.fixture_in_set(id) {
            self.fixtures.drag = None;
            self.fixtures.entry = None;
        }
        changed
    }

    /// The On this page check of the set `id` (criteria 2, 8); one step "Set fixture set on page"
    /// or "Set fixture set off page". A selected fixture of a set turned off is deselected.
    pub fn set_fixture_set_on_page(&mut self, id: NodeId, on_page: bool) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        let changed = self.recording_unguarded(|this| {
            this.document
                .set_fixture_set_on_page(id, on_page)
                .unwrap_or(false)
        });
        if changed && !on_page && self.fixture_in_set(id) {
            self.clear_fixture_selection();
        }
        changed
    }

    /// Folds or unfolds the set `id` (its chevron, name or line 2): not a step.
    pub fn set_fixture_set_unfolded(&mut self, id: NodeId, unfolded: bool) -> bool {
        if self.page_set(id).is_none() {
            return false;
        }
        if unfolded {
            self.fixtures.sets.unfolded.insert(id)
        } else {
            self.fixtures.sets.unfolded.remove(&id)
        }
    }

    /// Whether the fixture selected now lies in the set `set`.
    fn fixture_in_set(&self, set: NodeId) -> bool {
        let Some(selected) = self.fixtures.selected else {
            return false;
        };
        self.active_fixtures()
            .and_then(|f| f.get(selected.fixture).map(|f| f.set))
            .is_some_and(|home| home == Some(set))
    }

    /// The set named by its full id text on the active laser page.
    #[must_use]
    pub fn fixture_set_named(&self, name: &str) -> Option<NodeId> {
        self.active_fixtures()?
            .sets
            .into_iter()
            .find(|set| set.id.full_id() == name)
            .map(|set| set.id)
    }
}
