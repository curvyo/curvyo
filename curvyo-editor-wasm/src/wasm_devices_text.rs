//! My devices' fixed and formatted words for the page (`specs/0096-own-devices-sync`,
//! `ux-notes.md` 2 to 9), pass-throughs to `curvyo-ui-core::devices_text`, and the one container
//! read the browser's workspace store needs.

use curvyo_ui_core::devices_text as texts;
use wasm_bindgen::prelude::*;

/// The removed device's alert (criterion 24).
#[wasm_bindgen]
#[must_use]
pub fn devices_removed_alert() -> String {
    texts::REMOVED_ALERT.to_owned()
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

/// The one alert of the workspaces Sync these could not switch on (3.1); `names_json` is a JSON
/// array of their names, `problem` one of `unreachable`, `refused`, `local`, `needs-tls`;
/// `server` is the devices' server, `tab` the Share tab's (an unreachable server that differs from
/// it is named).
#[wasm_bindgen]
#[must_use]
pub fn devices_could_not_sync(names_json: &str, problem: &str, server: &str, tab: &str) -> String {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    let tab = curvyo_sync_core::normalize_server_address(tab).unwrap_or_default();
    let tab = tab.as_str();
    let problem = match problem {
        "refused" => texts::SyncProblem::Refused,
        "local" => texts::SyncProblem::Local,
        "needs-tls" => texts::SyncProblem::NeedsTls,
        _ if !tab.is_empty() && tab != server => {
            texts::SyncProblem::OtherServer { group: server, tab }
        }
        _ => texts::SyncProblem::Unreachable,
    };
    texts::could_not_sync_text(&names, problem)
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

/// The browser's fixed sentences by key (9, criteria 37 to 39): `cannot-start`, `needs-tls`,
/// `other-tab`; empty for another key.
#[wasm_bindgen]
#[must_use]
pub fn devices_browser_text(key: &str) -> String {
    match key {
        "cannot-start" => texts::BROWSER_CANNOT_START,
        "needs-tls" => texts::BROWSER_NEEDS_TLS,
        "other-tab" => texts::BROWSER_OTHER_TAB,
        _ => "",
    }
    .to_owned()
}

/// A fixed text of My devices and the switch by `key` (`sync-failed`, `not-connected`,
/// `not-admin-here`, `sharing`, `stopping`, `sharing-header`, `share-stalled`, `try-again`,
/// `only-this-device`); empty for an unknown key.
#[wasm_bindgen]
#[must_use]
pub fn devices_text(key: &str) -> String {
    match key {
        "sync-failed" => texts::SYNC_FAILED,
        "not-connected" => texts::NOT_CONNECTED,
        "not-admin-here" => texts::NOT_ADMIN_HERE,
        "sharing" => texts::SHARING_LINE,
        "stopping" => texts::STOPPING_LINE,
        "sharing-header" => texts::SHARING_HEADER,
        "share-stalled" => texts::SHARE_STALLED,
        "try-again" => texts::TRY_AGAIN,
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
/// browser's workspace store, criterion 35): `{"share": text or null}`.
#[wasm_bindgen]
#[must_use]
pub fn container_share_members(bytes: &[u8]) -> String {
    let members = curvyo_document_core::container_share_members(bytes);
    serde_json::json!({
        "share": members.share_json.map(|s| String::from_utf8_lossy(&s).into_owned()),
    })
    .to_string()
}

fn bytes(value: f64) -> u64 {
    // Storage estimates are whole numbers of bytes far below 2^53.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = value.max(0.0) as u64;
    whole
}
