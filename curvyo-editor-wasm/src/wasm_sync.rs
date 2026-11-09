//! Thin host bridge for cooperative editing (`specs/0092-cooperative-editing`): the transport's
//! inputs and outputs, the Share tab's reads and commands, and the presence overlay.
//!
//! Time arrives as the host's `Date.now()` and entropy as bytes of `crypto.getRandomValues`
//! (ADR 0031 decision 6); keys come from the credential store through the host.

use curvyo_sync_core::{Millis, SessionError};
use curvyo_ui_core::share_text;
use serde_json::json;
use wasm_bindgen::prelude::*;

use crate::session::{SyncSeeds, TransportAction, unhex};
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

pub(crate) fn seeds(identity: &[u8], nonce: &[u8], entropy: &[u8]) -> Option<SyncSeeds> {
    Some(SyncSeeds {
        identity: identity.try_into().ok()?,
        nonce: nonce.try_into().ok()?,
        entropy: entropy.try_into().ok()?,
    })
}

/// How long an invite link is valid: 24 hours, no choice (customer 2026-11-09).
const INVITE_VALID_SECONDS: u64 = 24 * 3600;

#[wasm_bindgen]
impl WasmSession {
    /// Shares the open document on `server` (criterion 1); `room` is 16 random bytes. Empty on
    /// success, else the sentence of the failure.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn share_document(
        &mut self,
        server: &str,
        label: &str,
        room: &[u8],
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> String {
        let (Some(seeds), Ok(room)) = (seeds(identity, nonce, entropy), room.try_into()) else {
            return "Could not share.".to_owned();
        };
        match self
            .session
            .share_document(server, label, room, &seeds, millis(now))
        {
            Ok(()) => String::new(),
            Err(_) => "Could not share.".to_owned(),
        }
    }

    /// Joins by an invite link (criterion 3). Empty when the join started, else the refusal.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn join_document(
        &mut self,
        link: &str,
        label: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> String {
        let Some(seeds) = seeds(identity, nonce, entropy) else {
            return share_text::join_refusal_text(share_text::JoinRefusal::Invalid, "");
        };
        match self.session.join_document(link, label, &seeds, millis(now)) {
            Ok(()) => String::new(),
            Err(refusal) => share_text::join_refusal_text(refusal, ""),
        }
    }

    /// Reconnects a shared document opened from its file; `keys_json` is `[[epoch, "hex"], ...]`
    /// from the credential store.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn resume_shared(
        &mut self,
        label: &str,
        keys_json: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> bool {
        let Some(seeds) = seeds(identity, nonce, entropy) else {
            return false;
        };
        self.session
            .resume_shared(label, epoch_keys(keys_json), &seeds, millis(now))
    }

    /// Whether the open document is shared.
    pub fn is_shared(&self) -> bool {
        self.session.is_shared()
    }

    /// The shared document's room as hex, empty when not shared.
    pub fn shared_room_hex(&self) -> String {
        self.session.shared_room_hex()
    }

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
}

/// The epoch keys of `keys_json` (`[[epoch, "hex"], ...]`, the credential store's); malformed
/// entries are left out.
pub(crate) fn epoch_keys(keys_json: &str) -> Vec<(u32, [u8; 32])> {
    let keys: Vec<(u32, String)> = serde_json::from_str(keys_json).unwrap_or_default();
    keys.into_iter()
        .filter_map(|(n, hex)| Some((n, unhex::<32>(&hex)?)))
        .collect()
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
