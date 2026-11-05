//! The membership lifecycle's words for the page (`specs/0097-membership-lifecycle` ux-notes
//! sections 2 to 8): pass-throughs to `curvyo-ui-core::membership_text` and
//! `membership_return_text`. Lists cross as JSON arrays.

use curvyo_ui_core::membership_return_text as back;
use curvyo_ui_core::membership_text as texts;
use wasm_bindgen::prelude::*;

fn strings(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

/// A fixed word of the flow by `key`, empty for an unknown key.
#[wasm_bindgen]
#[must_use]
pub fn membership_word(key: &str) -> String {
    match key {
        "unnamed" => texts::UNNAMED,
        "this-device" => texts::THIS_DEVICE,
        "my-device" => texts::MY_DEVICE,
        "participants" => texts::PARTICIPANTS,
        "former-list" => texts::FORMER_LIST,
        "only-admin-title" => texts::ONLY_ADMIN_TITLE,
        "make-admin" => texts::MAKE_ADMIN,
        "make-member" => texts::MAKE_MEMBER,
        "step-down" => texts::STEP_DOWN,
        "leave" => texts::LEAVE,
        "remove" => texts::REMOVE,
        "sync-again" => texts::SYNC_AGAIN,
        "keep-as-it-is" => texts::KEEP_AS_IT_IS,
        "add-back" => texts::ADD_BACK,
        "dismiss" => texts::DISMISS,
        "ok" => texts::OK,
        "tab-new-notice" => texts::TAB_NEW_NOTICE,
        "you-are-admin" => texts::YOU_ARE_ADMIN,
        "you-are-no-longer-admin" => texts::YOU_ARE_NO_LONGER_ADMIN,
        "keys-changed" => texts::KEYS_CHANGED,
        "step-down-confirm" => texts::STEP_DOWN_CONFIRM,
        "only-admin" => texts::ONLY_ADMIN,
        "person-only-admin" => texts::PERSON_ONLY_ADMIN,
        "not-connected" => texts::NOT_CONNECTED,
        "could-not" => texts::COULD_NOT,
        "one-admin" => texts::ONE_ADMIN_LINE,
        "only-own-admins" => texts::ONLY_OWN_ADMINS_LINE,
        "change-keys-now" => texts::CHANGE_KEYS_NOW,
        "changing-keys" => texts::CHANGING_KEYS,
        "make-admin-anyway" => texts::MAKE_ADMIN_ANYWAY,
        "syncing" => texts::SYNCING,
        "making-admin" => texts::RowBusy::MakingAdmin.text(),
        "making-member" => texts::RowBusy::MakingMember.text(),
        "removing" => texts::RowBusy::Removing.text(),
        "leaving" => texts::RowBusy::Leaving.text(),
        "adding-back" => texts::RowBusy::AddingBack.text(),
        "pending-offline" => texts::PENDING_OFFLINE,
        "check" => back::CHECK_BUTTON,
        "checking" => back::CHECKING,
        "not-added-back" => back::NOT_ADDED_BACK_YET,
        "check-unverified" => back::CHECK_UNVERIFIED,
        "check-not-found" => back::CHECK_NOT_FOUND,
        "merge" => back::MERGE_SENTENCE,
        "sync-again-failed" => back::SYNC_AGAIN_FAILED,
        "history" => back::HISTORY_LEGEND,
        "full-history" => back::FULL_HISTORY,
        "from-now-on" => back::FROM_NOW_ON,
        "left" => back::LEFT_WORD,
        "removed" => back::REMOVED_WORD,
        _ => "",
    }
    .to_owned()
}

/// "Key epoch n" beside the Participants heading.
#[wasm_bindgen]
#[must_use]
pub fn membership_key_epoch(epoch: u32) -> String {
    texts::key_epoch(epoch)
}

/// The admin-change line's time, from the page's local clock.
#[wasm_bindgen]
#[must_use]
pub fn membership_clock_time(hours: u32, minutes: u32) -> String {
    texts::clock_time(hours, minutes)
}

/// Make admin's confirm; `same_label` names another device with this label ("" when none).
#[wasm_bindgen]
#[must_use]
pub fn membership_make_admin_confirm(label: &str, same_label: &str) -> String {
    texts::make_admin_confirm(label, (!same_label.is_empty()).then_some(same_label))
}

/// The frozen-room warning ("" when the room is not frozen): `mutual_json` the labels of the
/// admins who removed each other, `refused` the admin whose own act was not applied.
#[wasm_bindgen]
#[must_use]
pub fn membership_frozen_warning(mutual_json: &str, refused: &str) -> String {
    texts::frozen_room_warning(
        &strings(mutual_json),
        (!refused.is_empty()).then_some(refused),
    )
    .unwrap_or_default()
}

/// Make member's confirm.
#[wasm_bindgen]
#[must_use]
pub fn membership_make_member_confirm(label: &str) -> String {
    texts::make_member_confirm(label)
}

/// Remove's confirm for a participant, led by "an admin" for an admin.
#[wasm_bindgen]
#[must_use]
pub fn membership_removal_confirm(label: &str, admin: bool) -> String {
    if admin {
        texts::admin_removal_confirm_text(label)
    } else {
        curvyo_ui_core::share_text::removal_confirm_text(label)
    }
}

/// A row's role line.
#[wasm_bindgen]
#[must_use]
pub fn membership_role_line(admin: bool, online: bool) -> String {
    texts::role_line(admin, online)
}

/// The role slot's accessible name.
#[wasm_bindgen]
#[must_use]
pub fn membership_role_action_name(label: &str, make_admin: bool, own: bool) -> String {
    texts::role_action_name(label, make_admin, own)
}

/// Remove's accessible name.
#[wasm_bindgen]
#[must_use]
pub fn membership_remove_name(label: &str) -> String {
    texts::remove_name(label)
}

/// Leave's accessible name.
#[wasm_bindgen]
#[must_use]
pub fn membership_leave_name(name: &str) -> String {
    texts::leave_name(name)
}

/// The admin-change line: `kind` is a notice kind (`nowAdmin`, `noLongerAdmin`, `adminLeft`,
/// `adminRemoved`).
#[wasm_bindgen]
#[must_use]
pub fn membership_admin_change(kind: &str, label: &str) -> String {
    let change = match kind {
        "nowAdmin" => texts::AdminChange::NowAdmin,
        "noLongerAdmin" => texts::AdminChange::NoLongerAdmin,
        "adminLeft" => texts::AdminChange::Left,
        "adminRemoved" => texts::AdminChange::Removed,
        _ => return String::new(),
    };
    texts::admin_change_text(change, label)
}

/// "and 2 more changes".
#[wasm_bindgen]
#[must_use]
pub fn membership_more_changes(more: usize) -> String {
    texts::more_changes_text(more)
}

/// The rotation pending notice; `former_json` is `[[label, left], ...]`.
#[wasm_bindgen]
#[must_use]
pub fn membership_rotation_pending(former_json: &str, admin: bool) -> String {
    let former: Vec<(String, bool)> = serde_json::from_str(former_json).unwrap_or_default();
    texts::rotation_pending_text(&former, admin)
}

/// The mutual-removal notice: the other admins' labels, `me` when this device is one.
#[wasm_bindgen]
#[must_use]
pub fn membership_mutual_removal(others_json: &str, me: bool) -> String {
    texts::mutual_removal_text(&strings(others_json), me)
}

/// The crossed-demotions notice: the other admins' labels, `me` when this device is one.
#[wasm_bindgen]
#[must_use]
pub fn membership_mutual_demotion(others_json: &str, me: bool) -> String {
    texts::mutual_demotion_text(&strings(others_json), me)
}

/// The frozen-room warning after crossed demotions: the labels of the admins involved.
#[wasm_bindgen]
#[must_use]
pub fn membership_frozen_demotion(mutual_json: &str) -> String {
    texts::frozen_demotion_warning(&strings(mutual_json))
}

/// The held-back line.
#[wasm_bindgen]
#[must_use]
pub fn membership_not_applied(label: &str, removal: bool) -> String {
    texts::not_applied_text(label, removal)
}

/// Leave's confirm: a document, or a workspace with its document count and the other devices of
/// mine it is synced to (JSON array, empty for none).
#[wasm_bindgen]
#[must_use]
pub fn membership_leave_confirm(
    name: &str,
    workspace: bool,
    documents: usize,
    devices_json: &str,
) -> String {
    if workspace {
        texts::leave_workspace_confirm(name, documents, &strings(devices_json))
    } else {
        texts::leave_document_confirm(name)
    }
}

/// "You left Box job." under the switch.
#[wasm_bindgen]
#[must_use]
pub fn membership_you_left_line(name: &str) -> String {
    texts::you_left_line(name)
}

/// The Removed block's heading.
#[wasm_bindgen]
#[must_use]
pub fn membership_ended_heading(name: &str, left: bool) -> String {
    back::ended_heading(name, left)
}

/// "Contacts 192.168.1.20:8080."
#[wasm_bindgen]
#[must_use]
pub fn membership_contacts_line(server: &str) -> String {
    back::contacts_line(server)
}

/// "Cannot reach the server at ...".
#[wasm_bindgen]
#[must_use]
pub fn membership_cannot_reach(server: &str) -> String {
    back::cannot_reach_text(server)
}

/// The add-back prompt.
#[wasm_bindgen]
#[must_use]
pub fn membership_added_back_prompt(by: &str, name: &str) -> String {
    back::added_back_prompt(by, name)
}

/// What Keep as it is does.
#[wasm_bindgen]
#[must_use]
pub fn membership_keep_consequence(name: &str) -> String {
    back::keep_consequence(name)
}

/// "Removed or left (2)".
#[wasm_bindgen]
#[must_use]
pub fn membership_former_disclosure(count: usize) -> String {
    back::former_disclosure(count)
}

/// "Add back Ben".
#[wasm_bindgen]
#[must_use]
pub fn membership_add_back_name(label: &str) -> String {
    back::add_back_name(label)
}

/// The add-back dialog's texts as `{full, fromNow, common}` for a document, `{text}` for a
/// workspace.
#[wasm_bindgen]
#[must_use]
pub fn membership_add_back_texts(
    label: &str,
    workspace: bool,
    name: &str,
    documents: usize,
) -> String {
    if workspace {
        serde_json::json!({"text": back::workspace_add_back(label, name, documents)}).to_string()
    } else {
        serde_json::json!({
            "full": back::full_history_line(label),
            "fromNow": back::from_now_on_line(label),
            "common": back::add_back_common(label),
        })
        .to_string()
    }
}

/// "Ben was added back."
#[wasm_bindgen]
#[must_use]
pub fn membership_added_back_notice(label: &str) -> String {
    back::added_back_notice(label)
}
