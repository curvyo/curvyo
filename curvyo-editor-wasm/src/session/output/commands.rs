//! The one-step commands of the laser page's output (`specs/0029` criteria 2, 25, 38; ADR
//! 0065 §1 to §3; UX notes §6.2, §6.3, §8, §12): the fixture material pick, Start and the
//! profile copy's Air assist, each one step.

use curvyo_document_core::ProfileChange;

use super::super::Session;

impl Session {
    /// Selects the items `ids` on the canvas (the count of a check line, UX P15); selecting is no
    /// step. `false` outside a laser page or with nothing to select.
    pub fn select_output_items(&mut self, ids: &[curvyo_document_core::NodeId]) -> bool {
        if self.output_page().is_none() || ids.is_empty() {
            return false;
        }
        self.selection.set(ids);
        true
    }

    /// The active laser page, with its extension on.
    fn output_page(&self) -> Option<curvyo_document_core::NodeId> {
        self.active_operations().map(|active| active.manufacturing)
    }

    /// Picks the material `id` the fixture jobs run with (criterion 2, Frage 221 B), one step
    /// "Pick fixture material"; an empty id removes the pick. A material that is not a loaded
    /// record of the library is not picked. Whether it changed.
    pub fn set_fixture_material(&mut self, id: &str) -> bool {
        let Some(manufacturing) = self.output_page() else {
            return false;
        };
        let material = (!id.is_empty()).then_some(id);
        if material.is_some_and(|id| self.materials().find(id).is_none()) {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_material(manufacturing, material)
                .unwrap_or(false)
        })
    }

    /// Start (criterion 38): `start` is one of the three stored spellings (`machine_origin`,
    /// `head_position`, `home_then_origin`); one step "Start: `<option>`". A pick writes whenever
    /// the stored value differs, a damaged one included. Whether it changed; `false` for another
    /// spelling or off a laser page.
    pub fn set_start(&mut self, start: &str) -> bool {
        let Some(manufacturing) = self.output_page() else {
            return false;
        };
        if curvyo_laser_core::job::Start::parse(start).is_none() {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .set_start(manufacturing, start)
                .unwrap_or(false)
        })
    }

    /// The Air assist switch of the page's profile copy (criterion 25, Frage 222 B): one step
    /// "Air assist on" or "off". Whether it changed.
    pub fn set_machine_air_assist(&mut self, on: bool) -> bool {
        let before = self.document.version();
        self.edit_copy(ProfileChange::AirAssist { on }, |copy| {
            if copy.values.air_assist == on {
                return None;
            }
            copy.values.air_assist = on;
            Some(None)
        });
        self.document.version() != before
    }
}
