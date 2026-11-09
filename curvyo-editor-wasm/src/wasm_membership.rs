//! Thin host bridge for the membership lifecycle (`specs/0097-membership-lifecycle`): roles,
//! leave, Change keys now, the person leave of a synced workspace and the membership notices. Every command answers with an empty
//! string on success, else the sentence why not.

use curvyo_sync_core::SessionError;
use wasm_bindgen::prelude::*;

use crate::session::unhex;
use crate::wasm_api::WasmSession;
use crate::wasm_devices::WasmDeviceGroup;

/// The sentence of a refused membership command (`curvyo-ui-core::membership_text`).
fn refusal(result: Result<(), SessionError>) -> String {
    use curvyo_ui_core::membership_text as texts;
    match result {
        Ok(()) => String::new(),
        Err(SessionError::Offline) => texts::NOT_CONNECTED.to_owned(),
        Err(SessionError::NotAdmin) => texts::NOT_ADMIN.to_owned(),
        Err(SessionError::LastAdmin) => texts::ONLY_ADMIN_REFUSED.to_owned(),
        Err(_) => texts::COULD_NOT.to_owned(),
    }
}

fn hex_list(json: &str) -> Option<Vec<String>> {
    serde_json::from_str(json).ok()
}

#[wasm_bindgen]
impl WasmSession {
    /// Make admin (`admin`) or Make member of `member_hex`; Step down is the own id with
    /// `admin` false.
    pub fn set_participant_role(&mut self, member_hex: &str, admin: bool) -> String {
        refusal(self.session.set_participant_role(member_hex, admin))
    }

    /// Stop sharing: the only participant ends the share here (customer 2026-11-09).
    pub fn stop_sharing(&mut self) -> String {
        refusal(self.session.stop_sharing())
    }

    /// Change keys now.
    pub fn rotate_share_keys(&mut self) -> String {
        refusal(self.session.rotate_share_keys())
    }

    /// Leave this workspace (one device).
    pub fn leave_share(&mut self) -> String {
        refusal(self.session.leave_share())
    }

    /// A workspace's ended record for the marker's `share.ended.json`, empty while it goes on.
    pub fn workspace_ended_json(&self) -> Vec<u8> {
        self.session
            .workspace_ended_record()
            .and_then(|r| r.to_bytes().ok())
            .unwrap_or_default()
    }

    /// The member id (hex) that signed this device's removal, empty when the keyring holds none.
    #[must_use]
    pub fn share_removed_by_hex(&self) -> String {
        self.session.share_removed_by().unwrap_or_default()
    }

    /// The admin set to keep per room as the last seen one, as a JSON array of hex ids.
    pub fn share_admin_set_json(&self) -> String {
        serde_json::to_string(&self.session.share_admin_set()).unwrap_or_default()
    }

    /// The membership notices against `previous_json` (the kept admin set; empty or invalid:
    /// none kept) and `own_group_json` (the group's member ids as hex).
    pub fn membership_notices_json(&self, previous_json: &str, own_group_json: &str) -> String {
        let previous = hex_list(previous_json);
        let group = hex_list(own_group_json).unwrap_or_default();
        self.session
            .membership_notices_json(previous.as_deref(), &group)
    }

    /// Whether the maker may leave the open synced workspace from all their devices.
    pub fn person_leave_allowed(&self, group: &WasmDeviceGroup) -> bool {
        group
            .glue
            .keyring()
            .is_some_and(|keyring| self.session.person_leave_allowed(keyring))
    }

    /// Runs the person leave of the open synced workspace once the devices list marks it
    /// `left`: `{written, leaving}`, or empty while its room is not online here.
    pub fn reconcile_left_workspace(&mut self, group: &WasmDeviceGroup) -> String {
        let Some(keyring) = group.glue.keyring() else {
            return String::new();
        };
        let Some(report) = self.session.reconcile_left_workspace(keyring) else {
            return String::new();
        };
        serde_json::json!({"written": report.written, "leaving": report.leaving}).to_string()
    }
}

#[wasm_bindgen]
impl WasmDeviceGroup {
    /// Marks workspace `id` (hex) named `name` as left by the maker.
    pub fn leave_workspace(&mut self, id: &str, name: &str) {
        if let Some(id) = unhex::<16>(id) {
            self.glue.leave_workspace(id, name);
        }
    }

    /// Whether the relay acknowledged every change of the devices list (the leave's mark).
    pub fn list_acked(&self) -> bool {
        self.glue.list_acked()
    }

    /// Whether the devices list marks workspace `id` (hex) as left.
    pub fn workspace_left(&self, id: &str) -> bool {
        unhex::<16>(id)
            .and_then(|id| self.glue.list().workspace(&id))
            .is_some_and(|w| w.left)
    }
}
