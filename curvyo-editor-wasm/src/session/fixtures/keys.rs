//! The keys of a selected fixture (`specs/0102-laser-fixtures` criteria 13 and 14, UX notes §10):
//! the arrows nudge a holder, clamp, stop or screw, M, R and S open its chips (D27), Delete arms
//! its removal and a second Delete removes it, and Delete on a grid only says how to remove it
//! (Escape is in the session's cascade).

use curvyo_document_core::{Affine, FixtureTurn, Vec2};

use super::super::Session;
use super::super::keys::{KeyInput, KeyOutcome};
use super::commands::movable;

/// The notice of Delete on the canvas with the grid selected (criterion 13).
pub const GRID_DELETE_NOTICE: &str = "Remove the grid with its trash in the Fixtures block.";

/// The nudge of an arrow key in mm: 1, or 10 with Shift (`0044`).
fn nudge(key: &str, shift: bool) -> Option<Vec2> {
    let step = if shift { 10.0 } else { 1.0 };
    match key {
        "ArrowLeft" => Some(Vec2::new(-step, 0.0)),
        "ArrowRight" => Some(Vec2::new(step, 0.0)),
        "ArrowUp" => Some(Vec2::new(0.0, -step)),
        "ArrowDown" => Some(Vec2::new(0.0, step)),
        _ => None,
    }
}

/// The key as one lower-case letter, if it is one.
fn single_letter(key: &str) -> Option<char> {
    let mut chars = key.chars();
    let c = chars.next()?.to_ascii_lowercase();
    chars.next().is_none().then_some(c)
}

impl Session {
    /// A key while a fixture is selected and the focus is on the canvas; `None` leaves the key
    /// to the key table.
    pub(in crate::session) fn fixture_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if input.dom_blocked || input.ctrl || input.alt {
            return None;
        }
        let id = self.selected_fixture()?;
        let (_, fixture) = self.page_fixture(id)?;
        if matches!(input.key, "Delete" | "Backspace") {
            if !movable(&fixture) {
                self.fixtures.notice = Some(GRID_DELETE_NOTICE.to_owned());
                return Some(KeyOutcome::Consumed);
            }
            if self.fixtures.armed == Some(id) {
                self.delete_fixture(id);
                return Some(KeyOutcome::Deleted);
            }
            self.fixtures.armed = Some(id);
            return Some(KeyOutcome::Consumed);
        }
        if let Some(key @ ('m' | 'r' | 's')) = single_letter(input.key) {
            if !movable(&fixture) || input.shift {
                return Some(KeyOutcome::Consumed);
            }
            return Some(if self.open_fixture_entry_for_key(key) {
                KeyOutcome::EntryOpened
            } else {
                KeyOutcome::Consumed
            });
        }
        let offset = nudge(input.key, input.shift)?;
        if !movable(&fixture) {
            return Some(KeyOutcome::Consumed);
        }
        self.fixtures.armed = None;
        let moved = Affine::translation(offset).compose(fixture.matrix);
        self.write_fixture_matrix(id, moved, FixtureTurn::Move);
        Some(KeyOutcome::Nudged { new_run: true })
    }

    /// The fixture a Delete on the canvas armed (UX notes D17), for its row's armed trash.
    #[must_use]
    pub fn armed_fixture(&self) -> Option<curvyo_document_core::NodeId> {
        self.fixtures
            .armed
            .filter(|id| self.selected_fixture() == Some(*id))
    }
}
