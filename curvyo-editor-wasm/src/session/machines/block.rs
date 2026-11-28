//! The Machine group of an output page's Document tab (ADR 0098 decision 4, UX unify 4.1): the
//! profile picker of the page's family, the "…" to the Machines tab and the state row ("Differs
//! from profile" with Reset, "Not in your machines" with Save as machine).

use serde_json::{Value, json};

use super::super::Session;
use super::super::output_page::FamilyPage;
use super::Family;

impl Session {
    /// The Machine group of the active output page of any family: `{ family, id, name, state,
    /// profiles: [{id, name, detail, chosen}] }`; `state` is `differs`, `missing` or `null`.
    pub(in crate::session) fn machine_picker_view(&self) -> Option<Value> {
        let active = self.active_output_page()?;
        let family = Family::of_process(&active.process)?;
        let id = super::edit::copy_id(family, &active);
        let name = super::edit::copy_name(family, &active).unwrap_or_default();
        let known = id.as_deref().and_then(|id| self.family_of_record(id)) == Some(family);
        let state = match &id {
            Some(id) if known => self
                .page_differs(family, active.id, id)
                .then_some("differs"),
            _ => Some("missing"),
        };
        let view = self.machines_view()?;
        let section = view
            .sections
            .iter()
            .find(|s| Family::of_key(s.family) == Some(family));
        let profiles: Vec<Value> = section
            .map(|s| {
                s.rows
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "detail": r.line2,
                            "chosen": id.as_deref() == Some(r.id.as_str()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(json!({
            "family": super::family_key(family),
            "id": id,
            "name": name,
            "state": state,
            "profiles": profiles,
        }))
    }

    /// The Machine group as JSON, `null` off an output page.
    #[must_use]
    pub fn machine_picker_json(&self) -> String {
        self.machine_picker_view()
            .map_or_else(|| "null".to_owned(), |v| v.to_string())
    }

    /// Pick profile on the active output page of any family: one step "Pick profile".
    pub fn pick_output_profile(&mut self, id: &str) -> bool {
        match self.active_family() {
            Some(FamilyPage::Plotter(_)) => self.pick_plotter_profile(id),
            Some(FamilyPage::Embroidery(_)) => self.pick_embroidery_profile(id),
            Some(FamilyPage::Cnc(_)) => self.pick_cnc_profile(id),
            None => self.pick_machine_profile(id),
        }
    }

    /// Reset from profile on the active output page of any family (ADR 0098 R3): the page's copy
    /// takes its record's values, one step on that page only.
    pub fn reset_output_profile(&mut self) -> bool {
        let Some(view) = self.machine_picker_view() else {
            return false;
        };
        let Some(id) = view["id"].as_str().map(str::to_owned) else {
            return false;
        };
        match self.active_family() {
            Some(_) => self.pick_output_profile(&id),
            None => self.reset_machine_profile(),
        }
    }

    /// Save as machine (ADR 0098 R2): the page's copy becomes a record of its family's library and
    /// the copy points at it. The new id.
    pub fn save_output_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        match self.active_family() {
            Some(FamilyPage::Plotter(_)) => self.save_plotter_profile(random, now_ms),
            Some(FamilyPage::Embroidery(_)) => self.save_embroidery_profile(random, now_ms),
            Some(FamilyPage::Cnc(_)) => self.save_cnc_profile(random, now_ms),
            None => {
                self.save_machine_profile(random, now_ms);
                self.active_copy().and_then(|copy| copy.read.copy.id)
            }
        }
    }
}
