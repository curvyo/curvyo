//! Thin host bridge for cooperative editing in a shared workspace (`specs/0092-cooperative-editing`,
//! `specs/0105`): the transport's inputs and outputs, the Share tab's reads and commands, and the
//! presence overlay.
//!
//! Time arrives as the host's `Date.now()` and entropy as bytes of `crypto.getRandomValues`
//! (ADR 0031 decision 6); keys come from the credential store through the host.

use curvyo_sync_core::{Millis, SessionError};
use curvyo_ui_core::share_text;
use serde_json::json;
use wasm_bindgen::prelude::*;

use crate::session::TransportAction;
use crate::wasm_api::WasmSession;

/// The next thing the transport does (`kind`: `connect`, `send` or `close`).
#[wasm_bindgen]
pub struct SyncAction {
    /// `connect`, `send` or `close`.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The address to open, for `connect`.
    #[wasm_bindgen(getter_with_clone)]
    pub address: String,
    /// The message, for `send`.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
}

pub(crate) fn millis(now: f64) -> Millis {
    // Date.now() is a whole number of milliseconds well inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Millis(now.max(0.0) as u64)
}

/// How long an invite link is valid: 24 hours, no choice (customer 2026-11-09).
const INVITE_VALID_SECONDS: u64 = 24 * 3600;

#[wasm_bindgen]
impl WasmSession {
    /// The socket opened.
    pub fn sync_connected(&mut self, now: f64) {
        self.session.sync_connected(millis(now));
    }

    /// The socket closed or could not open.
    pub fn sync_disconnected(&mut self, now: f64) {
        self.session.sync_disconnected(millis(now));
    }

    /// One binary message arrived.
    pub fn sync_received(&mut self, bytes: &[u8], now: f64) {
        self.session.sync_received(bytes, millis(now));
    }

    /// The network is back (`online` event): reconnect now.
    pub fn sync_network_back(&mut self, now: f64) {
        self.session.sync_network_back(millis(now));
    }

    /// The transport's timer, every 50 ms.
    pub fn sync_tick(&mut self, now: f64) {
        self.session.sync_tick(millis(now));
    }

    /// Before a frame is drawn: remote records are imported. `true` when the document changed.
    pub fn sync_frame_start(&mut self) -> bool {
        self.session.sync_frame_start()
    }

    /// The next transport action, `undefined` when there is none.
    pub fn sync_next_action(&mut self) -> Option<SyncAction> {
        let action = self.session.sync_next_action()?;
        Some(match action {
            TransportAction::Connect(address) => SyncAction {
                kind: "connect".to_owned(),
                address,
                bytes: Vec::new(),
            },
            TransportAction::Send(bytes) => SyncAction {
                kind: "send".to_owned(),
                address: String::new(),
                bytes,
            },
            TransportAction::Close => SyncAction {
                kind: "close".to_owned(),
                address: String::new(),
                bytes: Vec::new(),
            },
        })
    }

    /// Whether remote records changed the document since the last call.
    pub fn take_sync_changed(&mut self) -> bool {
        self.session.take_sync_changed()
    }

    /// The next notice of sharing as `{"text", "alert"}`, empty when there is none.
    pub fn take_sync_notice(&mut self) -> String {
        self.session
            .take_sync_notice()
            .map(|(text, alert)| json!({"text": text, "alert": alert}).to_string())
            .unwrap_or_default()
    }

    /// The epoch keys to store as `{"room", "keys": [[epoch, "hex"], ...]}`, empty when unchanged.
    pub fn take_key_update(&mut self) -> String {
        self.session
            .take_key_update()
            .map(|(room, keys)| {
                let keys: Vec<(u32, String)> = keys
                    .iter()
                    .map(|(n, k)| (*n, crate::session::hex(k)))
                    .collect();
                json!({"room": room, "keys": keys}).to_string()
            })
            .unwrap_or_default()
    }

    /// Changes whenever the Share tab or the pill would show something else.
    pub fn share_revision(&self) -> u32 {
        self.session.share_revision()
    }

    /// The Share tab and the pill as one JSON record.
    pub fn share_view_json(&self) -> String {
        self.session.share_view_json()
    }

    /// The other participants' pointers, selections and markers as one JSON array.
    pub fn presence_marks_json(&self) -> String {
        self.session.presence_marks_json()
    }

    /// "Ben is editing this." once when the selection gains another participant's object; empty
    /// otherwise.
    pub fn take_contested_notice(&mut self) -> String {
        self.session.take_contested_notice().unwrap_or_default()
    }

    /// Changes the "Name shown to others".
    pub fn set_share_label(&mut self, label: &str) {
        self.session.set_share_label(label);
    }

    /// Writes an invite valid for 24 hours: `{"link", "expires"}`, or `{"error"}`.
    pub fn create_invite(&mut self, now: f64) -> String {
        match self
            .session
            .create_invite(INVITE_VALID_SECONDS, millis(now))
        {
            Ok((link, expires)) => json!({"link": link, "expires": expires}).to_string(),
            Err(error) => {
                let text = match error {
                    SessionError::Offline => "Not connected. Try again when connected.",
                    SessionError::NotAdmin => "Only an admin can invite.",
                    _ => "Could not create a link.",
                };
                json!({"error": text}).to_string()
            }
        }
    }

    /// Removes a participant by its member id as hex, with a key rotation (criterion 32).
    /// Empty on success, else the sentence why not.
    pub fn remove_participant(&mut self, member_hex: &str) -> String {
        match self.session.remove_participant(member_hex) {
            Ok(()) => String::new(),
            Err(SessionError::Offline) => "Not connected. Try again when connected.".to_owned(),
            Err(SessionError::NotAdmin) => "Only an admin can remove a participant.".to_owned(),
            Err(_) => "Could not remove this participant.".to_owned(),
        }
    }

    /// The labels other participants announced, as `{"<member hex>": "label"}`, to keep
    /// between runs.
    pub fn share_labels_json(&self) -> String {
        json!(self.session.share_labels()).to_string()
    }

    /// Labels kept from earlier runs, as `share_labels_json` wrote them; anything else is
    /// ignored.
    pub fn remember_share_labels(&mut self, labels_json: &str) {
        let labels = serde_json::from_str(labels_json).unwrap_or_default();
        self.session.remember_share_labels(&labels);
    }

    /// "Made from version a1b2c3d." after an export of a shared document; empty otherwise.
    pub fn export_version_note(&mut self) -> String {
        self.session.export_version_note()
    }

    /// Leaves the room (Cancel while joining).
    pub fn leave_sync(&mut self) {
        self.session.leave_sync();
    }

    /// Drops an ended link (a Share that failed or was cancelled).
    pub fn drop_ended_link(&mut self) {
        self.session.drop_ended_link();
    }
}

/// A typed server address normalised (`ws://host:port`), empty when it is not one.
#[wasm_bindgen]
#[must_use]
pub fn normalize_server_address(text: &str) -> String {
    curvyo_sync_core::normalize_server_address(text).unwrap_or_default()
}

/// The address as sentences show it.
#[wasm_bindgen]
#[must_use]
pub fn server_shown(address: &str) -> String {
    share_text::server_shown(address).to_owned()
}

/// "Expires Tue 14:32." in local time `offset_minutes` east of UTC.
#[wasm_bindgen]
#[must_use]
pub fn invite_expiry_text(expires: f64, offset_minutes: i32) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    share_text::expiry_text(expires.max(0.0) as u64, offset_minutes)
}

/// A participant's initials.
#[wasm_bindgen]
#[must_use]
pub fn participant_initials(label: &str) -> String {
    share_text::initials(label)
}

/// A fixed word of the workspace-only sharing model (`specs/0105`) by key; empty for an unknown
/// key.
#[wasm_bindgen]
#[must_use]
pub fn sharing_word(key: &str) -> String {
    use curvyo_ui_core::sharing_text as texts;
    match key {
        "legacy-notice" => texts::LEGACY_SHARE_NOTICE,
        "document-link" => texts::DOCUMENT_LINK_REFUSAL,
        "desktop-link" => texts::DESKTOP_LINK_REFUSAL,
        "works-on-workspaces" => texts::SHARING_WORKS_ON_WORKSPACES,
        "workspaces-on-desktop" => texts::WORKSPACES_ON_DESKTOP,
        "make-workspace" => texts::MAKE_WORKSPACE,
        "sync-to-my-devices" => texts::SYNC_TO_MY_DEVICES,
        "sync-confirm" => texts::SYNC_CONFIRM,
        "invite-person" => texts::INVITE_PERSON,
        "join" => texts::JOIN,
        "join-field" => texts::JOIN_FIELD,
        "add-device" => texts::ADD_DEVICE,
        _ => "",
    }
    .to_owned()
}

/// The inline confirm of Make a workspace for this file (`specs/0105` criterion 8).
#[wasm_bindgen]
#[must_use]
pub fn sharing_make_workspace_confirm(folder: &str, file: &str) -> String {
    curvyo_ui_core::sharing_text::make_workspace_confirm(folder, file)
}

/// The Workspace block's state word: Local, Synced or Shared (`specs/0105` criterion 10).
#[wasm_bindgen]
#[must_use]
pub fn sharing_workspace_state(synced: bool, others: bool) -> String {
    curvyo_ui_core::sharing_text::workspace_state(synced, others).to_owned()
}

/// Why Invite person is greyed, its tooltip; empty when it works (customer feedback on PR #298).
#[wasm_bindgen]
#[must_use]
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // The host's facts cross as flat arguments.
pub fn sharing_invite_blocker(
    desktop: bool,
    workspace_open: bool,
    server_set: bool,
    server_reachable: bool,
    name_set: bool,
    linked: bool,
    admin: bool,
    busy: bool,
) -> String {
    curvyo_ui_core::sharing_text::invite_blocker(&curvyo_ui_core::sharing_text::InviteFacts {
        desktop,
        workspace_open,
        server_set,
        server_reachable,
        name_set,
        linked,
        admin,
        busy,
    })
    .unwrap_or_default()
    .to_owned()
}
