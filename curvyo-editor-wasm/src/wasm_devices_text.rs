//! My devices' fixed and formatted words for the page (`specs/0096-own-devices-sync`,
//! `ux-notes.md` 2 to 9), pass-throughs to `curvyo-ui-core::devices_text`, and the one container
//! read the browser's workspace store needs.

use curvyo_ui_core::devices_text as texts;
use wasm_bindgen::prelude::*;

/// The sentence under the pairing code (criterion 1).
#[wasm_bindgen]
#[must_use]
pub fn devices_pair_sentence() -> String {
    curvyo_ui_core::pairing_text::PAIR_SENTENCE.to_owned()
}

/// The privacy sentence of choosing workspaces (criteria 9 and 30).
#[wasm_bindgen]
#[must_use]
pub fn devices_privacy_sentence() -> String {
    texts::PRIVACY_SENTENCE.to_owned()
}

/// The removed device's alert (criterion 24).
#[wasm_bindgen]
#[must_use]
pub fn devices_removed_alert() -> String {
    texts::REMOVED_ALERT.to_owned()
}

/// The Leave confirm.
#[wasm_bindgen]
#[must_use]
pub fn devices_leave_confirm() -> String {
    texts::LEAVE_CONFIRM.to_owned()
}

/// The Remove confirm for the device named `label`.
#[wasm_bindgen]
#[must_use]
pub fn devices_remove_confirm(label: &str) -> String {
    texts::remove_device_confirm_text(label, false)
}

/// The switch-on confirm.
#[wasm_bindgen]
#[must_use]
pub fn devices_sync_on_confirm(
    name: &str,
    documents: usize,
    only_device: bool,
    taken_in_json: &str,
    not_added_json: &str,
) -> String {
    let taken_in: Vec<String> = serde_json::from_str(taken_in_json).unwrap_or_default();
    let not_added: Vec<String> = serde_json::from_str(not_added_json).unwrap_or_default();
    texts::sync_on_confirm_text(name, documents, only_device, &taken_in, &not_added)
}

/// The switch-off confirm naming the other devices (JSON array of names).
#[wasm_bindgen]
#[must_use]
pub fn devices_sync_off_confirm(name: &str, devices_json: &str, others_json: &str) -> String {
    let devices: Vec<String> = serde_json::from_str(devices_json).unwrap_or_default();
    let others: Vec<String> = serde_json::from_str(others_json).unwrap_or_default();
    texts::sync_off_confirm_text(name, &devices, &others)
}

/// The switch's line while on.
#[wasm_bindgen]
#[must_use]
pub fn devices_switch_on_line(other_devices: usize) -> String {
    texts::switch_on_line(texts::SyncState::InSync, other_devices)
}

/// The workspace switch's reason where this device is not an admin.
#[wasm_bindgen]
#[must_use]
pub fn devices_not_admin_text(workspace: &str) -> String {
    texts::not_admin_text(workspace)
}

/// The notice of an arrived workspace.
#[wasm_bindgen]
#[must_use]
pub fn devices_arrived_notice(name: &str) -> String {
    texts::arrived_notice(name)
}

/// The refusal of a Synced workspaces folder by the host's code.
#[wasm_bindgen]
#[must_use]
pub fn devices_synced_folder_refusal(code: &str) -> String {
    texts::synced_folder_refusal_text(code).to_owned()
}

/// The words of the joiner's waiting line.
#[wasm_bindgen]
#[must_use]
pub fn devices_waiting_for_other() -> String {
    curvyo_ui_core::pairing_text::WAITING_FOR_OTHER.to_owned()
}

/// The notice on a device whose workspace stopped syncing (6.4).
#[wasm_bindgen]
#[must_use]
pub fn devices_no_longer_synced_notice(name: &str) -> String {
    texts::no_longer_synced_notice(name)
}

/// The heading of choosing workspaces after Pair (3.1).
#[wasm_bindgen]
#[must_use]
pub fn devices_workspaces_for(label: &str) -> String {
    texts::workspaces_for_text(label)
}

/// The line naming workspaces that are synced already (3.1); `names_json` a JSON array.
#[wasm_bindgen]
#[must_use]
pub fn devices_already_synced(names_json: &str, label: &str) -> String {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    texts::already_synced_text(&names, label)
}

/// The progress after Sync these (3.1).
#[wasm_bindgen]
#[must_use]
pub fn devices_syncing_workspaces(workspaces: usize, done: usize, total: usize) -> String {
    texts::syncing_workspaces_text(workspaces, done, total)
}

/// The alert of a workspace Sync these could not switch on (3.1).
#[wasm_bindgen]
#[must_use]
pub fn devices_could_not_sync(name: &str) -> String {
    texts::could_not_sync_text(name)
}

/// The pick list's line of a workspace in which this device is only a member (3.1).
#[wasm_bindgen]
#[must_use]
pub fn devices_not_admin_here() -> String {
    texts::NOT_ADMIN_HERE.to_owned()
}

/// The decision row of a copy that is here already (2.3, criterion 14).
#[wasm_bindgen]
#[must_use]
pub fn devices_connect_copy(name: &str, path: &str) -> String {
    texts::connect_copy_text(name, path)
}

/// The browser's fixed sentences by key (9, criteria 34, 37 to 39): `loss`, `own-row`,
/// `not-persisted`, `cannot-start`, `needs-tls`, `other-tab`; empty for another key.
#[wasm_bindgen]
#[must_use]
pub fn devices_browser_text(key: &str) -> String {
    match key {
        "loss" => texts::BROWSER_LOSS_SENTENCE,
        "own-row" => texts::BROWSER_OWN_ROW_LINE,
        "not-persisted" => texts::BROWSER_NOT_PERSISTED_LINE,
        "cannot-start" => texts::BROWSER_CANNOT_START,
        "needs-tls" => texts::BROWSER_NEEDS_TLS,
        "other-tab" => texts::BROWSER_OTHER_TAB,
        _ => "",
    }
    .to_owned()
}

/// A fixed text of My devices and the switch by `key` (`sync-failed`, `not-connected`,
/// `not-admin-here`, `only-this-device`); empty for an unknown key.
#[wasm_bindgen]
#[must_use]
pub fn devices_text(key: &str) -> String {
    match key {
        "sync-failed" => texts::SYNC_FAILED,
        "not-connected" => texts::NOT_CONNECTED,
        "not-admin-here" => texts::NOT_ADMIN_HERE,
        "only-this-device" => return texts::devices_count_text(1),
        _ => "",
    }
    .to_owned()
}

/// "1 document", "14 documents".
#[wasm_bindgen]
#[must_use]
pub fn devices_documents_text(documents: usize) -> String {
    texts::documents_text(documents)
}

/// The header of a workspace every member of which is an own device: "Synced, 2 devices".
#[wasm_bindgen]
#[must_use]
pub fn devices_synced_header(devices: usize) -> String {
    texts::synced_header_text(devices)
}

/// Whether the member `member` (64 hex characters) is an admin in the keyring log `log` (a
/// shared workspace's `keyring.log`, ux-notes 3.1); `false` for a log that does not replay.
#[wasm_bindgen]
#[must_use]
pub fn keyring_is_admin(log: &[u8], member: &str) -> bool {
    let Some(member) = crate::session::unhex::<32>(member) else {
        return false;
    };
    curvyo_crypto_core::keyring::decode_log(log)
        .ok()
        .and_then(|entries| curvyo_crypto_core::keyring::Keyring::replay(&entries).ok())
        .is_some_and(|keyring| {
            keyring.role(&curvyo_crypto_core::MemberId(member))
                == Some(curvyo_crypto_core::keyring::Role::Admin)
        })
}

/// What the browser holds (criterion 37); bytes as numbers, `names_json` a JSON array.
#[wasm_bindgen]
#[must_use]
pub fn devices_browser_storage(used: f64, quota: f64, names_json: &str) -> String {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    texts::browser_storage_text(bytes(used), bytes(quota), &names)
}

/// The alert of a document the browser could not store (criterion 37).
#[wasm_bindgen]
#[must_use]
pub fn devices_browser_storage_full(name: &str, used: f64, quota: f64) -> String {
    texts::browser_storage_full_text(name, bytes(used), bytes(quota))
}

/// What a shared workspace's scan needs of one document file where there is no file system (the
/// browser's workspace store, criterion 35): `{"share": text or null, "keyring": bool}`.
#[wasm_bindgen]
#[must_use]
pub fn container_share_members(bytes: &[u8]) -> String {
    let members = curvyo_document_core::container_share_members(bytes);
    serde_json::json!({
        "share": members.share_json.map(|s| String::from_utf8_lossy(&s).into_owned()),
        "keyring": members.keyring,
    })
    .to_string()
}

fn bytes(value: f64) -> u64 {
    // Storage estimates are whole numbers of bytes far below 2^53.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = value.max(0.0) as u64;
    whole
}
