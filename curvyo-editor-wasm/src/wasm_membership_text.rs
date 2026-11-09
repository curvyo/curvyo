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
        "only-admin-title" => texts::ONLY_ADMIN_TITLE,
        "make-admin" => texts::MAKE_ADMIN,
        "make-member" => texts::MAKE_MEMBER,
        "step-down" => texts::STEP_DOWN,
        "leave" => texts::LEAVE,
        "stop-sharing" => texts::STOP_SHARING,
        "remove" => texts::REMOVE,
        "ok" => texts::OK,
        "tab-new-notice" => texts::TAB_NEW_NOTICE,
        "you-are-admin" => texts::YOU_ARE_ADMIN,
        "you-are-no-longer-admin" => texts::YOU_ARE_NO_LONGER_ADMIN,
        "keys-changed" => texts::KEYS_CHANGED,
        "person-only-admin" => texts::PERSON_ONLY_ADMIN,
        "not-connected" => texts::NOT_CONNECTED,
        "could-not" => texts::COULD_NOT,
        "change-keys-now" => texts::CHANGE_KEYS_NOW,
        "changing-keys" => texts::CHANGING_KEYS,
        "make-admin-anyway" => texts::MAKE_ADMIN_ANYWAY,
        "making-admin" => texts::RowBusy::MakingAdmin.text(),
        "making-member" => texts::RowBusy::MakingMember.text(),
        "removing" => texts::RowBusy::Removing.text(),
        "leaving" => texts::RowBusy::Leaving.text(),
        "pending-offline" => texts::PENDING_OFFLINE,
        _ => "",
    }
    .to_owned()
}

/// The admin-change line's time, from the page's local clock.
#[wasm_bindgen]
#[must_use]
pub fn membership_clock_time(hours: u32, minutes: u32) -> String {
    texts::clock_time(hours, minutes)
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

/// "Ben was added back."
#[wasm_bindgen]
#[must_use]
pub fn membership_added_back_notice(label: &str) -> String {
    back::added_back_notice(label)
}
