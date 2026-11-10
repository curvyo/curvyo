//! The session's glue of a fixture's chip (`specs/0102-laser-fixtures` criteria 14, 18, 22, 23,
//! 38; feature `adrs.md` §6a.4): the chip opens from a double-click or a key, shows through the
//! rectangle's chip views and commits as one step; the rules are in `entry.rs` and `edit.rs`.

use curvyo_document_core::Point;
use curvyo_ui_core::{EntryOutcome, MoveEntry, ReferencePoint};

use super::super::move_entry::MoveEntryView;
use super::super::transform_entry::{EntryFieldView, EntryView};
use super::super::{Session, Tool};
use super::entry::{Chip, FixtureEntry};

impl Session {
    /// The open chip of the selected fixture, while the Select tool is active and the fixture is
    /// still the one it opened on.
    fn fixture_entry(&self) -> Option<&FixtureEntry> {
        let entry = self.fixtures.entry.as_ref()?;
        (self.tool == Tool::Select && self.selected_fixture() == Some(entry.id)).then_some(entry)
    }

    /// The chip a double-click opens on the handle the last press took (the rectangle's rule:
    /// the handle of the second press). `true` when one opened.
    pub(in crate::session) fn open_fixture_entry_at_press(&mut self) -> bool {
        let Some((id, handle)) = self.fixtures.last_press_handle else {
            return false;
        };
        let Some((fixtures, fixture)) = self
            .selected_fixture()
            .filter(|selected| *selected == id)
            .and_then(|id| self.page_fixture(id))
        else {
            return false;
        };
        self.fixtures.entry = FixtureEntry::at_handle(&fixture, &fixtures.page, handle);
        self.fixtures.entry.is_some()
    }

    /// The chip of the key `key` (`m`, `r`, `s`) on the selected fixture; `true` when one opened.
    pub(super) fn open_fixture_entry_for_key(&mut self, key: char) -> bool {
        let Some((fixtures, fixture)) =
            self.selected_fixture().and_then(|id| self.page_fixture(id))
        else {
            return false;
        };
        self.fixtures.entry = FixtureEntry::for_key(&fixture, &fixtures.page, key);
        self.fixtures.entry.is_some()
    }

    /// The size, angle, leg or diameter chip of the selected fixture as the rectangle's chip
    /// shows it; `None` without one or for the Move chip.
    pub(in crate::session) fn fixture_entry_view(&self) -> Option<EntryView> {
        let entry = self.fixture_entry()?;
        let kind = match &entry.chip {
            Chip::Board(_) => "size",
            Chip::Angle => "angle",
            Chip::Stop(parts) if parts.len() == 1 => "fixture-length",
            Chip::Screw(parts) if parts.len() == 1 => "fixture-length",
            Chip::Stop(_) | Chip::Screw(_) => "fixture-size",
            Chip::Move(_) => return None,
        };
        let (centre, handle) = entry.anchor(&self.transform_handle_tolerances());
        Some(EntryView {
            kind,
            fields: entry
                .fields()
                .iter()
                .map(|f| EntryFieldView {
                    label: f.label,
                    accessible_name: f.accessible_name,
                    prefill: f.prefill.clone(),
                    editable: f.editable,
                })
                .collect(),
            linked: false,
            scope: None,
            handle: handle.unwrap_or(centre),
            center: centre,
            glyph_reach_px: 6.0,
            at_centre: handle.is_none(),
            selection: false,
        })
    }

    /// The open Move chip of the selected fixture.
    fn fixture_move(&self) -> Option<&MoveEntry> {
        match &self.fixture_entry()?.chip {
            Chip::Move(entry) => Some(entry),
            _ => None,
        }
    }

    /// The Move chip of the selected fixture as the rectangle's Move chip shows it, with no Copy
    /// (a fixture is not cloned, D21).
    pub(in crate::session) fn fixture_move_view(&self) -> Option<MoveEntryView> {
        let entry = self.fixture_move()?;
        let box_ = entry.start_box();
        let (min, max) = entry.bounds();
        let page = self.fixture_entry()?.start_page().clone();
        let relative = |index: usize| entry.fields()[index].prefill.clone();
        Some(MoveEntryView {
            center: box_.to_document(box_.local_center()),
            relative_prefill: [relative(0), relative(1)],
            absolute_prefills: ReferencePoint::ALL
                .map(|reference| entry.absolute_prefill_from(reference, &page)),
            reference: entry.reference(),
            box_bottom: Point::new(f64::midpoint(min.x, max.x), max.y),
            copy_preset: false,
            copy_allowed: false,
        })
    }

    /// The open Move chip of the selected fixture, to report its mode and reference to.
    pub(in crate::session) fn fixture_move_mut(&mut self) -> Option<&mut MoveEntry> {
        self.fixture_entry()?;
        match &mut self.fixtures.entry.as_mut()?.chip {
            Chip::Move(entry) => Some(entry),
            _ => None,
        }
    }

    /// Enter in a fixture's chip with the typed `texts` (`absolute` for the Move chip): one step
    /// "Resize fixture", "Rotate fixture" or "Move fixture", a value past its limit set to it with
    /// its notice. `None` when no fixture chip is open. A committed or unchanged chip closes; a
    /// refused one stays open.
    pub(in crate::session) fn commit_fixture_entry(
        &mut self,
        texts: &[&str],
        absolute: bool,
    ) -> Option<EntryOutcome> {
        let entry = self.fixture_entry()?.clone();
        let current = self.page_fixture(entry.id).map(|(_, f)| f);
        if current.as_ref() != Some(entry.start()) {
            self.fixtures.entry = None;
            return Some(EntryOutcome::Unchanged);
        }
        let outcome = match entry.resolve(texts, absolute) {
            Err(outcome) => outcome,
            Ok((edit, label, notice)) => {
                // The Action notice at the chip's handle, as the rectangle's limit readout
                // (UX notes §4.1, §4.4); a press clears it.
                if let Some(text) = notice {
                    let (centre, handle) = entry.anchor(&self.transform_handle_tolerances());
                    self.limit_notice = Some(super::super::shapes::LiveReadout {
                        text,
                        anchor: handle.unwrap_or(centre),
                    });
                }
                if self.write_fixture_edit(entry.id, &edit, label) {
                    EntryOutcome::Committed
                } else {
                    EntryOutcome::Unchanged
                }
            }
        };
        if !matches!(outcome, EntryOutcome::Invalid { .. }) {
            self.fixtures.entry = None;
        }
        Some(outcome)
    }

    /// Closes a fixture's chip without writing; `true` when one was open.
    pub(in crate::session) fn cancel_fixture_entry(&mut self) -> bool {
        self.fixtures.entry.take().is_some()
    }

    /// The handle whose chip is open, for its dragging look.
    pub(super) fn fixture_entry_handle(&self) -> Option<super::handles::FixtureHandle> {
        self.fixture_entry().and_then(|entry| entry.handle)
    }
}
