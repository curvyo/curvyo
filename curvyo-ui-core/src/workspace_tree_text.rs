//! The words of a shared workspace's live tree in the File tab (`specs/0095-workspace` criteria
//! 35 to 42 and 49, `ux-notes.md` sections 3, 3.3, 5, 6 and 7): notices of other members'
//! changes, row states, the Deleted list, the shared delete confirm and the take-in texts.

/// The longest item name a notice shows whole; longer ones are cut in the middle. `ux-notes.md`
/// section 7 says 24, but a rename notice with two names and a 16-character label then passes
/// its 70 characters **(Proposal)**.
const NOTICE_NAME_MAX: usize = 20;
/// The longest participant label a notice shows; longer ones are cut at the end.
const NOTICE_LABEL_MAX: usize = 16;
/// The longest item name a sentence of the File tab shows whole.
const SENTENCE_NAME_MAX: usize = 40;

/// The row state of a file that is not part of the shared workspace (criterion 41).
pub const NOT_IN_WORKSPACE_TEXT: &str = "Not in the shared workspace";
/// The row state of a document the list names whose file was removed (criterion 41).
pub const MISSING_TEXT: &str = "Missing on this device";
/// The row state of a document whose file has not arrived yet (criterion 35).
pub const RECEIVING_TEXT: &str = "Receiving...";
/// The notice of a folder move the loop rule did not apply (criterion 39).
pub const LOOP_NOTICE: &str = "Folder move skipped: it would loop.";
/// The block the File tab shows on a device removed from the workspace (criterion 44).
pub const WORKSPACE_REMOVED_TEXT: &str =
    "You were removed from this workspace. Your copies stay on this device.";
/// The Deleted list's line when it is empty (`ux-notes.md` section 5).
pub const NO_DELETED_TEXT: &str = "No deleted documents.";

/// A participant's label as a notice shows it: cut at 16 characters, "Someone" when unknown.
#[must_use]
pub fn who(label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        return "Someone".to_owned();
    }
    let chars: Vec<char> = label.chars().collect();
    if chars.len() <= NOTICE_LABEL_MAX {
        label.to_owned()
    } else {
        let mut cut: String = chars[..NOTICE_LABEL_MAX - 1].iter().collect();
        cut.push('…');
        cut
    }
}

/// `name` cut in the middle with an ellipsis past `max` characters.
fn cut(name: &str, max: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= max {
        return name.to_owned();
    }
    let head = max / 2;
    let tail = max - head - 1;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

fn item(name: &str) -> String {
    cut(name, NOTICE_NAME_MAX)
}

/// "Ben renamed Lid to Cover." (criterion 36).
#[must_use]
pub fn renamed_notice(by: &str, from: &str, to: &str) -> String {
    format!("{} renamed {} to {}.", who(by), item(from), item(to))
}

/// "Ben moved Lid to Sides." (criterion 37); `folder` is the workspace's name for the top.
#[must_use]
pub fn moved_notice(by: &str, name: &str, folder: &str) -> String {
    format!("{} moved {} to {}.", who(by), item(name), item(folder))
}

/// "Ben deleted Lid." or "Ben deleted 3 documents." (criterion 40).
#[must_use]
pub fn deleted_notice(by: &str, names: &[String]) -> String {
    match names {
        [one] => format!("{} deleted {}.", who(by), item(one)),
        many => format!("{} deleted {} documents.", who(by), many.len()),
    }
}

/// The project block's status when the open document with unsaved changes was deleted by
/// another member (criterion 40, `ux-notes.md` section 7).
#[must_use]
pub fn deleted_while_open_text(by: &str) -> String {
    format!("Deleted by {}. Save as to keep it.", who(by))
}

/// The inline confirm of Delete in a shared workspace (`ux-notes.md` section 3.3).
#[must_use]
pub fn shared_delete_confirm_text(name: &str, folder: bool, documents: usize) -> String {
    let name = cut(name, SENTENCE_NAME_MAX);
    let subject = match (folder, documents) {
        (false, _) | (true, 0) => name,
        (true, 1) => format!("{name} and its 1 document"),
        (true, n) => format!("{name} and its {n} documents"),
    };
    format!(
        "Delete {subject} for everyone? Every device moves it to its trash. You can restore it \
         from Deleted."
    )
}

/// A Deleted row's second line: "Deleted by Ben, 12:03" or "Deleted by you, 12:03".
#[must_use]
pub fn deleted_row_text(by: &str, mine: bool, time: &str) -> String {
    let by = if mine { "you".to_owned() } else { who(by) };
    format!("Deleted by {by}, {time}")
}

/// The status after Restore: "Restored Lid." (`ux-notes.md` section 5).
#[must_use]
pub fn restored_text(name: &str) -> String {
    format!("Restored {}.", cut(name, SENTENCE_NAME_MAX))
}

/// The status after Add to workspace: "Added Lid to the workspace." (criteria 41, 42).
#[must_use]
pub fn added_text(name: &str) -> String {
    format!("Added {} to the workspace.", cut(name, SENTENCE_NAME_MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ux-notes.md` section 7: notices stay within 70 characters with long labels and names.
    #[test]
    fn notices_name_the_member_and_fit_seventy_characters() {
        assert_eq!(
            renamed_notice("Ben", "Lid", "Cover"),
            "Ben renamed Lid to Cover."
        );
        assert_eq!(
            moved_notice("", "Lid", "Sides"),
            "Someone moved Lid to Sides."
        );
        assert_eq!(
            deleted_notice("Ben", &["Lid".to_owned()]),
            "Ben deleted Lid."
        );
        assert_eq!(
            deleted_notice("Ben", &["a".to_owned(), "b".to_owned(), "c".to_owned()]),
            "Ben deleted 3 documents."
        );
        let long = "A very long participant label indeed";
        let name = "An extremely long document name that goes on";
        for text in [
            renamed_notice(long, name, name),
            moved_notice(long, name, name),
            deleted_notice(long, &[name.to_owned()]),
            LOOP_NOTICE.to_owned(),
        ] {
            assert!(text.chars().count() <= 70, "{text}");
        }
    }

    #[test]
    fn confirms_and_statuses_read_as_the_ux_notes_say() {
        assert_eq!(
            shared_delete_confirm_text("Sides", true, 6),
            "Delete Sides and its 6 documents for everyone? Every device moves it to its \
             trash. You can restore it from Deleted."
        );
        assert_eq!(
            deleted_row_text("Ben", false, "12:03"),
            "Deleted by Ben, 12:03"
        );
        assert_eq!(
            deleted_row_text("Ben", true, "12:03"),
            "Deleted by you, 12:03"
        );
        assert_eq!(
            deleted_while_open_text("Ben"),
            "Deleted by Ben. Save as to keep it."
        );
        assert_eq!(restored_text("Lid"), "Restored Lid.");
        assert_eq!(added_text("Lid"), "Added Lid to the workspace.");
    }
}
