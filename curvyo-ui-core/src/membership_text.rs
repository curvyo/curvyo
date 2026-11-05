//! The words of roles, leaving and the notices in the Share tab (`specs/0097-membership-lifecycle`
//! ux-notes sections 2 to 5), formatted here so both builds say the same.
//!
//! Labels are cut at 24 characters in lists ([`cut_label`]) and with a middle ellipsis in
//! sentences ([`middle_cut`]); an empty label reads "A device" (criterion 9). The noun is
//! "device" (MEM-NOUN). The way back of a removed or left person and the add back are in
//! [`crate::membership_return_text`].

use std::fmt::Write as _;

use crate::share_text::cut_label;

/// What an unnamed identity is called (criterion 9).
pub const UNNAMED: &str = "A device";
/// The role word of an admin (criterion 1).
pub const ADMIN_WORD: &str = "Admin";
/// The role word of a member.
pub const MEMBER_WORD: &str = "Member";
/// The Share tab's accessible name while a notice is unread (section 2.1).
pub const TAB_NEW_NOTICE: &str = "Share, new notice";
/// The canvas notice when this device became an admin (section 2.4).
pub const YOU_ARE_ADMIN: &str = "You are now an admin.";
/// The canvas notice when this device is no longer an admin.
pub const YOU_ARE_NO_LONGER_ADMIN: &str = "You are no longer an admin.";
/// The canvas notice after Change keys now (section 4).
pub const KEYS_CHANGED: &str = "Keys changed.";
/// Step down's confirm (section 3.3).
pub const STEP_DOWN_CONFIRM: &str =
    "Step down as admin? You keep editing. You can no longer invite, remove or change roles.";
/// Why Step down and Leave are refused to the only admin (criteria 5 and 14).
pub const ONLY_ADMIN: &str = "You are the only admin. Make someone else an admin first.";
/// Why "Leave my devices" is refused to the group's only admin device (ADR 0041 decision 1;
/// a change of `0096` behaviour: before, the last device could leave and the group was gone).
pub const GROUP_ONLY_ADMIN: &str =
    "This device is the only one that can manage your devices. Pair another device first.";
/// Why the person leave of a synced workspace is refused (criterion 16).
pub const PERSON_ONLY_ADMIN: &str =
    "Your devices are the only admins. Make someone else an admin first.";
/// A person leave a device found refused after it began, when no admin outside the maker's
/// devices remains (ADR 0041 decision 1): the workspace stays synced here.
pub const PERSON_LEAVE_REFUSED: &str = "Could not leave this workspace: your devices are the \
    only admins. Make someone else an admin first.";
/// The suffix after this device's own label in its row (section 3.1).
pub const THIS_DEVICE: &str = "this device";
/// The suffix after a label of another device of the maker's group.
pub const MY_DEVICE: &str = "my device";
/// The Participants list's heading and name.
pub const PARTICIPANTS: &str = "Participants";
/// The Removed or left list's name (section 8.1).
pub const FORMER_LIST: &str = "Removed or left";
/// Why Step down or Leave is unavailable to the only admin, as the control's tooltip
/// (section 3.3).
pub const ONLY_ADMIN_TITLE: &str = "You are the only admin";

/// The key epoch beside the Participants heading.
#[must_use]
pub fn key_epoch(epoch: u32) -> String {
    format!("Key epoch {epoch}")
}

/// The time of the admin-change line, 24-hour "hh:mm" (section 2.3).
#[must_use]
pub fn clock_time(hours: u32, minutes: u32) -> String {
    format!("{:02}:{:02}", hours % 24, minutes % 60)
}

/// A membership act refused while not connected (section 10).
pub const NOT_CONNECTED: &str = "Not connected. Try again when connected.";
/// A membership act refused to a member.
pub const NOT_ADMIN: &str = "Only an admin can do this.";
/// A membership act that could not be done for another reason.
pub const COULD_NOT: &str = "Could not finish: cannot reach the server. Nothing was changed.";
/// The second-admin line, one admin device (criterion 7, section 3.5).
pub const ONE_ADMIN_LINE: &str = "Only this device is an admin. If you lose it, nobody can add or \
     remove people. Make someone else an admin.";
/// The second-admin line, every admin a device of mine (criterion 8).
pub const ONLY_OWN_ADMINS_LINE: &str = "All admins are your own devices. If you lose them all, \
     nobody can add or remove people. Make another person an admin.";
/// The button of a rotation pending notice for admins (criterion 12).
pub const CHANGE_KEYS_NOW: &str = "Change keys now";
/// Its busy label.
pub const CHANGING_KEYS: &str = "Changing keys...";

/// What a row's busy line says while its keyring entry is on the way (section 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowBusy {
    /// Make admin.
    MakingAdmin,
    /// Make member or Step down.
    MakingMember,
    /// Remove.
    Removing,
    /// Leave or Keep as it is.
    Leaving,
    /// Add back.
    AddingBack,
}

impl RowBusy {
    /// The words.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::MakingAdmin => "Making admin...",
            Self::MakingMember => "Making member...",
            Self::Removing => "Removing...",
            Self::Leaving => "Leaving...",
            Self::AddingBack => "Adding back...",
        }
    }
}

/// What changed in the admin set, for the admin-change line (section 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminChange {
    /// Promoted.
    NowAdmin,
    /// Demoted.
    NoLongerAdmin,
    /// An admin left.
    Left,
    /// An admin was removed.
    Removed,
}

/// `text` cut to `max` characters with an ellipsis in the middle, for names inside sentences
/// (ux-notes "Noun").
#[must_use]
pub fn middle_cut(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max || max < 3 {
        return text.to_owned();
    }
    let keep = max - 1;
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// A label inside a sentence: cut in the middle at 24, "A device" when empty.
#[must_use]
pub fn named(label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        UNNAMED.to_owned()
    } else {
        middle_cut(label, 24)
    }
}

/// A label in a list: cut at 24, "A device" when empty.
#[must_use]
pub fn listed(label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        UNNAMED.to_owned()
    } else {
        cut_label(label, 24)
    }
}

/// The row's third line: the role word, then ", offline" without presence (section 3.1).
#[must_use]
pub fn role_line(admin: bool, online: bool) -> String {
    let word = if admin { ADMIN_WORD } else { MEMBER_WORD };
    if online {
        word.to_owned()
    } else {
        format!("{word}, offline")
    }
}

/// The accessible name of the role slot's button: "Make Ben an admin", "Make Ben a member",
/// "Step down as admin".
#[must_use]
pub fn role_action_name(label: &str, make_admin: bool, own: bool) -> String {
    if own {
        return "Step down as admin".to_owned();
    }
    let who = named(label);
    if make_admin {
        format!("Make {who} an admin")
    } else {
        format!("Make {who} a member")
    }
}

/// The accessible name of the own row's Leave: "Leave Poster".
#[must_use]
pub fn leave_name(name: &str) -> String {
    format!("Leave {}", middle_cut(name, 24))
}

/// Make admin's confirm (criterion 3, section 3.2), with the line for another row of the same
/// label when there is one.
#[must_use]
pub fn make_admin_confirm(label: &str, same_label_device: Option<&str>) -> String {
    let who = named(label);
    let mut text = format!(
        "Make {who} an admin? {who} can invite people, remove people, change roles and change \
         keys. You can make {who} a member again."
    );
    if let Some(other) = same_label_device {
        let other = named(other);
        let _ = write!(text, " {who}'s other device, {other}, keeps its role.");
    }
    text
}

/// The warning that replaces Make admin's sentence in a frozen room (criterion 28, section 3.2):
/// `mutual` names the admins who removed each other; else `refused` is the admin whose own
/// removal or step down was not applied. `None` when the room is not frozen.
#[must_use]
pub fn frozen_room_warning(mutual: &[String], refused: Option<&str>) -> Option<String> {
    if mutual.len() >= 2 {
        return Some(format!(
            "{} removed each other earlier. If you make another admin, both removals apply and \
             both leave.",
            and_list(mutual)
        ));
    }
    refused.map(|who| {
        let who = named(who);
        format!(
            "{who}'s earlier removal was not applied because nobody else was an admin. If you \
             make another admin, it applies and {who} leaves."
        )
    })
}

/// The button labels and tooltips of the flow (ux-notes 3, 4, 7 and 8).
pub const MAKE_ADMIN: &str = "Make admin";
/// Make member's label.
pub const MAKE_MEMBER: &str = "Make member";
/// Step down's label.
pub const STEP_DOWN: &str = "Step down";
/// Leave's label.
pub const LEAVE: &str = "Leave";
/// Remove's label.
pub const REMOVE: &str = "Remove";
/// The add-back prompt's first button.
pub const SYNC_AGAIN: &str = "Sync again";
/// Its second button.
pub const KEEP_AS_IT_IS: &str = "Keep as it is";
/// A former row's action.
pub const ADD_BACK: &str = "Add back";
/// The second-admin line's button.
pub const DISMISS: &str = "Dismiss";
/// The mutual-removal notice's button.
pub const OK: &str = "OK";

/// The accessible name of a row's Remove: "Remove Ben".
#[must_use]
pub fn remove_name(label: &str) -> String {
    format!("Remove {}", named(label))
}

/// The action of Make admin in a frozen room.
pub const MAKE_ADMIN_ANYWAY: &str = "Make admin anyway";

/// Make member's confirm (criterion 4, section 3.3).
#[must_use]
pub fn make_member_confirm(label: &str) -> String {
    let who = named(label);
    format!(
        "Make {who} a member? {who} can no longer invite, remove or change roles; {who} keeps \
         editing."
    )
}

/// The admin-change line (criterion 30, section 2.3).
#[must_use]
pub fn admin_change_text(change: AdminChange, label: &str) -> String {
    let who = named(label);
    match change {
        AdminChange::NowAdmin => format!("{who} is now an admin"),
        AdminChange::NoLongerAdmin => format!("{who} is no longer an admin"),
        AdminChange::Left => format!("{who} left"),
        AdminChange::Removed => format!("{who} was removed"),
    }
}

/// The muted tail of the admin-change line for the changes it does not name.
#[must_use]
pub fn more_changes_text(more: usize) -> String {
    match more {
        0 => String::new(),
        1 => "and 1 more change".to_owned(),
        n => format!("and {n} more changes"),
    }
}

/// The rotation pending notice (criterion 11, section 4): `former` are the labels with whether
/// each left by its own hand; members also read who changes the keys.
#[must_use]
pub fn rotation_pending_text(former: &[(String, bool)], admin: bool) -> String {
    let Some((first, _)) = former.first() else {
        return String::new();
    };
    let all_left = former.iter().all(|(_, left)| *left);
    let who = match former.len() {
        1 => named(first),
        2 => format!("{} and 1 other", named(first)),
        n => format!("{} and {} others", named(first), n - 1),
    };
    let verb = match (all_left, former.len()) {
        (true, _) => "left",
        (false, 1) => "was removed",
        (false, _) if former.iter().all(|(_, left)| !*left) => "were removed",
        (false, _) => "left or were removed",
    };
    let mut text = format!("{who} {verb}. The keys have not been changed yet.");
    if !admin {
        text.push_str(" Waiting for an admin to change the keys.");
    }
    text
}

/// The mutual-removal notice (criteria 27 and 29): the admins by label, `me` when this device
/// is one of them (it reads "You and Chris").
#[must_use]
pub fn mutual_removal_text(others: &[String], me: bool) -> String {
    let mut names: Vec<String> = Vec::new();
    if me {
        names.push("You".to_owned());
    }
    names.extend(others.iter().map(|l| named(l)));
    format!(
        "{} removed each other at the same time. Nobody was removed.",
        join_names(&names)
    )
}

/// A removal or demotion replay held back (criterion 28).
#[must_use]
pub fn not_applied_text(label: &str, removal: bool) -> String {
    let who = named(label);
    if removal {
        format!("Removal of {who} was not applied: you are the only admins.")
    } else {
        format!("Making {who} a member was not applied: you are the only admins.")
    }
}

/// The confirm of Remove for an admin's row: the `0092` sentence led by "Remove Ben, an admin?"
/// (section 3.4).
#[must_use]
pub fn admin_removal_confirm_text(label: &str) -> String {
    let plain = crate::share_text::removal_confirm_text(label);
    let who = named(label);
    match plain.split_once("? ") {
        Some((_, rest)) => format!("Remove {who}, an admin? {rest}"),
        None => plain,
    }
}

/// Leave's confirm for a document (criterion 10, section 5.1).
#[must_use]
pub fn leave_document_confirm(name: &str) -> String {
    format!(
        "Leave {}? Syncing stops on this device and the copy stays here. Until an admin changes \
         the keys, this device still holds the key to what was shared so far.",
        middle_cut(name, 24)
    )
}

/// Leave's confirm for a workspace (criterion 15); `my_devices` are the other devices of mine
/// it is synced to (criterion 16, the person leave), empty when none.
#[must_use]
pub fn leave_workspace_confirm(name: &str, documents: usize, my_devices: &[String]) -> String {
    let name = middle_cut(name, 24);
    let documents = match documents {
        1 => "The document stays".to_owned(),
        n => format!("The {n} documents stay"),
    };
    let mut text = format!(
        "Leave {name}? {documents} in this folder as ordinary files and syncing stops here. \
         Until an admin changes the keys, this device still holds the key to what was shared so \
         far."
    );
    if !my_devices.is_empty() {
        let shown: Vec<String> = my_devices
            .iter()
            .take(3)
            .map(|d| cut_label(d, 16))
            .collect();
        let mut list = shown.join(", ");
        if my_devices.len() > 3 {
            let _ = write!(list, " and {} more", my_devices.len() - 3);
        }
        let _ = write!(
            text,
            " {name} is synced to your other devices. Leaving removes all of them: {list}."
        );
    }
    text
}

/// The canvas notice on the leaver (section 5.2).
#[must_use]
pub fn left_notice(name: &str, workspace: bool) -> String {
    let name = middle_cut(name, 24);
    if workspace {
        format!("You left {name}. The documents stay in this folder.")
    } else {
        format!("You left {name}. Your copy stays on this device.")
    }
}

/// The state line under the switch of a workspace the maker left (section 5.4).
#[must_use]
pub fn you_left_line(name: &str) -> String {
    format!("You left {}.", middle_cut(name, 24))
}

/// Sync again's busy label.
pub const SYNCING: &str = "Syncing...";

/// "Ana and Chris", "Ana, Ben and Chris", each label cut in the middle.
fn and_list(labels: &[String]) -> String {
    let names: Vec<String> = labels.iter().map(|l| named(l)).collect();
    join_names(&names)
}

fn join_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Every text of this module with sample inputs, for the wording test of criterion 34.
#[cfg(test)]
pub(crate) fn every_text() -> Vec<String> {
    let ben = "Ben";
    let mut out: Vec<String> = [
        UNNAMED,
        TAB_NEW_NOTICE,
        YOU_ARE_ADMIN,
        YOU_ARE_NO_LONGER_ADMIN,
        KEYS_CHANGED,
        STEP_DOWN_CONFIRM,
        ONLY_ADMIN,
        GROUP_ONLY_ADMIN,
        PERSON_ONLY_ADMIN,
        PERSON_LEAVE_REFUSED,
        NOT_CONNECTED,
        NOT_ADMIN,
        COULD_NOT,
        ONE_ADMIN_LINE,
        ONLY_OWN_ADMINS_LINE,
        CHANGE_KEYS_NOW,
        CHANGING_KEYS,
        MAKE_ADMIN_ANYWAY,
        MAKE_ADMIN,
        MAKE_MEMBER,
        STEP_DOWN,
        LEAVE,
        REMOVE,
        SYNC_AGAIN,
        KEEP_AS_IT_IS,
        ADD_BACK,
        DISMISS,
        OK,
        SYNCING,
    ]
    .iter()
    .map(|t| (*t).to_owned())
    .collect();
    for busy in [
        RowBusy::MakingAdmin,
        RowBusy::MakingMember,
        RowBusy::Removing,
        RowBusy::Leaving,
        RowBusy::AddingBack,
    ] {
        out.push(busy.text().to_owned());
    }
    for change in [
        AdminChange::NowAdmin,
        AdminChange::NoLongerAdmin,
        AdminChange::Left,
        AdminChange::Removed,
    ] {
        out.push(admin_change_text(change, ben));
    }
    let pair = ["Ana".to_owned(), "Chris".to_owned()];
    out.extend([
        role_line(true, false),
        role_action_name(ben, true, false),
        role_action_name(ben, false, false),
        role_action_name(ben, false, true),
        leave_name("Poster"),
        remove_name(ben),
        make_admin_confirm(ben, Some("Phone")),
        frozen_room_warning(&pair, None).unwrap_or_default(),
        frozen_room_warning(&[], Some("Ana")).unwrap_or_default(),
        make_member_confirm(ben),
        more_changes_text(2),
        rotation_pending_text(&[(ben.to_owned(), true)], true),
        rotation_pending_text(&[(ben.to_owned(), false), ("Cy".to_owned(), true)], false),
        mutual_removal_text(&pair, false),
        mutual_removal_text(&["Chris".to_owned()], true),
        not_applied_text("Chris", true),
        not_applied_text("Chris", false),
        admin_removal_confirm_text(ben),
        crate::share_text::removal_confirm_text(ben),
        leave_document_confirm("Poster"),
        leave_workspace_confirm("Box job", 17, &["Laptop".to_owned(), "Phone".to_owned()]),
        left_notice("Poster", false),
        left_notice("Box job", true),
        you_left_line("Box job"),
    ]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_admin_change_time_is_24_hour_with_two_digits() {
        assert_eq!(clock_time(9, 5), "09:05");
        assert_eq!(clock_time(23, 59), "23:59");
        assert_eq!(key_epoch(3), "Key epoch 3");
    }

    #[test]
    fn names_are_cut_in_the_middle_in_sentences_and_at_the_end_in_lists() {
        let long = "Workshop laptop of the north hall";
        assert_eq!(middle_cut(long, 24).chars().count(), 24);
        assert!(middle_cut(long, 24).starts_with("Workshop la"));
        assert!(middle_cut(long, 24).ends_with("north hall"));
        assert_eq!(listed(long).chars().count(), 24);
        assert_eq!(named(""), "A device");
        assert_eq!(listed("  "), "A device");
    }

    /// Criteria 3, 4, 5 and 28: the confirms say what the role means and what stays.
    #[test]
    fn role_confirms_and_the_frozen_room_warning() {
        assert_eq!(
            make_admin_confirm("Ben", Some("Phone")),
            "Make Ben an admin? Ben can invite people, remove people, change roles and change \
             keys. You can make Ben a member again. Ben's other device, Phone, keeps its role."
        );
        assert_eq!(
            make_member_confirm("Ben"),
            "Make Ben a member? Ben can no longer invite, remove or change roles; Ben keeps \
             editing."
        );
        assert_eq!(
            frozen_room_warning(&["Ana".to_owned(), "Chris".to_owned()], None).unwrap(),
            "Ana and Chris removed each other earlier. If you make another admin, both removals \
             apply and both leave."
        );
        assert!(
            frozen_room_warning(&[], Some("Ana"))
                .unwrap()
                .starts_with("Ana's earlier removal was not applied")
        );
        assert_eq!(frozen_room_warning(&[], None), None);
        assert_eq!(role_line(false, false), "Member, offline");
        assert_eq!(role_line(true, true), "Admin");
    }

    /// Criteria 11, 27, 28 and 30: the notices.
    #[test]
    fn notices_name_who_and_what_waits() {
        assert_eq!(
            rotation_pending_text(&[("Ben".to_owned(), true)], true),
            "Ben left. The keys have not been changed yet."
        );
        assert_eq!(
            rotation_pending_text(
                &[
                    ("Ben".to_owned(), true),
                    ("Cy".to_owned(), true),
                    ("Di".to_owned(), true)
                ],
                false
            ),
            "Ben and 2 others left. The keys have not been changed yet. Waiting for an admin to \
             change the keys."
        );
        assert_eq!(
            mutual_removal_text(&["Chris".to_owned()], true),
            "You and Chris removed each other at the same time. Nobody was removed."
        );
        assert_eq!(
            not_applied_text("Chris", true),
            "Removal of Chris was not applied: you are the only admins."
        );
        assert_eq!(
            admin_change_text(AdminChange::NowAdmin, "Ben"),
            "Ben is now an admin"
        );
        assert_eq!(more_changes_text(2), "and 2 more changes");
        assert!(admin_removal_confirm_text("Ben").starts_with("Remove Ben, an admin? Removing"));
    }

    /// Criteria 10, 15 and 16: Leave says what leaves and what stays.
    #[test]
    fn leave_says_what_stays() {
        assert_eq!(
            leave_document_confirm("Poster"),
            "Leave Poster? Syncing stops on this device and the copy stays here. Until an admin \
             changes the keys, this device still holds the key to what was shared so far."
        );
        let devices: Vec<String> = ["Laptop", "Phone", "Tablet", "Desk", "Shop"]
            .iter()
            .map(|d| (*d).to_owned())
            .collect();
        let text = leave_workspace_confirm("Box job", 17, &devices);
        assert!(
            text.starts_with(
                "Leave Box job? The 17 documents stay in this folder as ordinary files"
            )
        );
        assert!(text.ends_with(
            "Box job is synced to your other devices. Leaving removes all of them: Laptop, \
             Phone, Tablet and 2 more."
        ));
        assert_eq!(
            left_notice("Poster", false),
            "You left Poster. Your copy stays on this device."
        );
    }
}
