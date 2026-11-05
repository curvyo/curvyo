//! The words of the way back in the Share tab (`specs/0097-membership-lifecycle` ux-notes
//! sections 7 and 8): the Removed block with its check and add-back prompt on the person who is
//! out, and the Removed or left rows with the add-back dialog on the admin.

use crate::membership_text::{middle_cut, named};
use crate::share_text::server_shown;

/// A file's name in a sentence, "this document" when the file has none yet (a joined copy
/// never saved).
fn file_name(name: &str) -> String {
    if name.trim().is_empty() {
        "this document".to_owned()
    } else {
        middle_cut(name.trim(), 24)
    }
}

/// The check's button (criterion 20).
pub const CHECK_BUTTON: &str = "Check whether I was added back";
/// Its busy label.
pub const CHECKING: &str = "Checking...";
/// The check's answer when the device is not a participant, or the server refused it.
pub const NOT_ADDED_BACK_YET: &str = "Not added back yet.";
/// The check's answer when the fetched keyring lists the device but does not prove the add back
/// (section 7.1).
pub const CHECK_UNVERIFIED: &str = "The server's answer could not be checked. Nothing was changed.";
/// The check found no such room on the server (section 7.1, row 5).
pub const CHECK_NOT_FOUND: &str = "Nothing to find here. This file stays local.";
/// The merge sentence of the add-back prompt (criterion 21).
pub const MERGE_SENTENCE: &str = "Changes you made while you were away will be merged in.";
/// Sync again's failure (section 7.3).
pub const SYNC_AGAIN_FAILED: &str = "Could not sync: cannot reach the server.";
/// The add-back dialog's radio group (section 8.2).
pub const HISTORY_LEGEND: &str = "History";
/// The preselected history choice.
pub const FULL_HISTORY: &str = "Full history";
/// The other history choice.
pub const FROM_NOW_ON: &str = "From now on";
/// A former row that left by its own hand.
pub const LEFT_WORD: &str = "Left";
/// A former row that was removed.
pub const REMOVED_WORD: &str = "Removed";

/// The Removed block's heading (criteria 13 and 20, section 7.1).
#[must_use]
pub fn ended_heading(name: &str, left: bool) -> String {
    let name = file_name(name);
    if left {
        format!("You left {name}")
    } else {
        format!("You were removed from {name}")
    }
}

/// What leaves the machine when Check is pressed (criterion 33).
#[must_use]
pub fn contacts_line(server: &str) -> String {
    format!("Contacts {}.", server_shown(server))
}

/// The check's answer when the server cannot be reached.
#[must_use]
pub fn cannot_reach_text(server: &str) -> String {
    format!("Cannot reach the server at {}.", server_shown(server))
}

/// The add-back prompt (criteria 20 and 25): `by` is the admin's label when known.
#[must_use]
pub fn added_back_prompt(by: &str, name: &str) -> String {
    let name = file_name(name);
    if by.trim().is_empty() {
        format!("You were added back to {name}. Sync again?")
    } else {
        format!("{} added you back to {name}. Sync again?", named(by))
    }
}

/// What Keep as it is does (Frage 162 A).
#[must_use]
pub fn keep_consequence(name: &str) -> String {
    format!(
        "Keep as it is leaves {} again. The admins will see that you left.",
        file_name(name)
    )
}

/// The disclosure of the Removed or left rows (criterion 17, section 8.1).
#[must_use]
pub fn former_disclosure(count: usize) -> String {
    format!("Removed or left ({count})")
}

/// The accessible name of a former row's Add back.
#[must_use]
pub fn add_back_name(label: &str) -> String {
    format!("Add back {}", named(label))
}

/// The consequence line of Full history (criterion 18).
#[must_use]
pub fn full_history_line(label: &str) -> String {
    format!(
        "{} gets the keys of every key period you hold, so the document's history comes along.",
        named(label)
    )
}

/// The consequence line of From now on (criteria 18 and 22: a rule of this app, not a lock).
#[must_use]
pub fn from_now_on_line(label: &str) -> String {
    let who = named(label);
    format!(
        "The app does not give {who} the keys of the key periods that began while {who} was \
         away. Someone who already holds them could pass them on outside the app. {who} still \
         holds the keys {who} had before."
    )
}

/// The sentence under the choices of a document's add back (criterion 18).
#[must_use]
pub fn add_back_common(label: &str) -> String {
    let who = named(label);
    format!(
        "{who} comes back as a member and keeps what {who} had. Nothing else changes for anybody."
    )
}

/// A workspace's add back (criterion 23): no choice.
#[must_use]
pub fn workspace_add_back(label: &str, name: &str, documents: usize) -> String {
    let who = named(label);
    let documents = match documents {
        1 => "the document".to_owned(),
        n => format!("all {n} documents"),
    };
    format!(
        "Add {who} back to {}? {who} gets every key you hold and the current state of \
         {documents}. {who} comes back as a member and keeps what {who} had.",
        middle_cut(name, 24)
    )
}

/// The canvas notice after an add back (section 8.3).
#[must_use]
pub fn added_back_notice(label: &str) -> String {
    format!("{} was added back.", named(label))
}

/// Every text of this module with sample inputs, for the wording test of criterion 34.
#[cfg(test)]
fn every_text() -> Vec<String> {
    let mut out: Vec<String> = [
        CHECK_BUTTON,
        CHECKING,
        NOT_ADDED_BACK_YET,
        CHECK_UNVERIFIED,
        CHECK_NOT_FOUND,
        MERGE_SENTENCE,
        SYNC_AGAIN_FAILED,
        HISTORY_LEGEND,
        FULL_HISTORY,
        FROM_NOW_ON,
        LEFT_WORD,
        REMOVED_WORD,
        crate::share_text::REMOVED_NOTICE,
        crate::share_text::LEFT_NOTICE,
    ]
    .iter()
    .map(|t| (*t).to_owned())
    .collect();
    out.extend([
        ended_heading("Poster", true),
        ended_heading("Poster", false),
        contacts_line("ws://192.168.1.20:8080"),
        cannot_reach_text("ws://192.168.1.20:8080"),
        added_back_prompt("Ana", "Poster"),
        added_back_prompt("", "Box job"),
        keep_consequence("Poster"),
        former_disclosure(2),
        add_back_name("Ben"),
        full_history_line("Ben"),
        from_now_on_line("Ben"),
        add_back_common("Ben"),
        workspace_add_back("Ben", "Box job", 17),
        added_back_notice("Ben"),
    ]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 34: no text of the share flow says "revoked" or "lost access", and none says
    /// "can no longer read" (or "cannot read") without saying what stays.
    #[test]
    fn no_text_of_the_share_flow_says_revoked_or_lost_access() {
        let mut texts = every_text();
        texts.extend(crate::membership_text::every_text());
        assert!(texts.len() > 60, "{}", texts.len());
        for text in &texts {
            let lower = text.to_lowercase();
            assert!(!lower.contains("revoked"), "{text}");
            assert!(!lower.contains("lost access"), "{text}");
            assert!(!lower.contains("access lost"), "{text}");
            for phrase in ["can no longer read", "cannot read"] {
                assert!(
                    !lower.contains(phrase)
                        || lower.contains("still holds")
                        || lower.contains("keeps"),
                    "{text}"
                );
            }
        }
    }

    /// Criteria 18, 20, 22 and 23: the way back says what is given and what stays.
    #[test]
    fn a_file_without_a_name_is_this_document() {
        assert_eq!(
            ended_heading("", false),
            "You were removed from this document"
        );
        assert_eq!(ended_heading(" ", true), "You left this document");
        assert_eq!(
            added_back_prompt("Ana", ""),
            "Ana added you back to this document. Sync again?"
        );
    }

    #[test]
    fn the_way_back_texts() {
        assert_eq!(
            ended_heading("Box job", false),
            "You were removed from Box job"
        );
        assert_eq!(
            contacts_line("ws://192.168.1.20:8080"),
            "Contacts 192.168.1.20:8080."
        );
        assert_eq!(
            added_back_prompt("Ana", "Poster"),
            "Ana added you back to Poster. Sync again?"
        );
        assert_eq!(
            from_now_on_line("Ben"),
            "The app does not give Ben the keys of the key periods that began while Ben was \
             away. Someone who already holds them could pass them on outside the app. Ben still \
             holds the keys Ben had before."
        );
        assert_eq!(
            workspace_add_back("Ben", "Box job", 17),
            "Add Ben back to Box job? Ben gets every key you hold and the current state of all \
             17 documents. Ben comes back as a member and keeps what Ben had."
        );
        assert_eq!(former_disclosure(2), "Removed or left (2)");
    }
}
