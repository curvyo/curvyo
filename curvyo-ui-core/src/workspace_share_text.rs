//! The words of a shared workspace in the Share tab and the File tab's header line
//! (`specs/0095-workspace` criteria 25 to 29, `ux-notes.md` sections 2.1, 4, 8 and 9).

use crate::share_text::ConnectionState;

/// The join's waiting line (`ux-notes.md` section 8).
pub const WORKSPACE_WAITING_TEXT: &str = "Waiting for a member to let you in";

/// The Workspace block's state at the right of its heading: "1 person", "2 people".
#[must_use]
pub fn people_text(people: usize) -> String {
    match people {
        1 => "1 person".to_owned(),
        n => format!("{n} people"),
    }
}

/// The line naming the documents Share left out: "Not added: Lid, Base.".
#[must_use]
pub fn not_added_text(names: &[String]) -> String {
    format!("Not added: {}.", names.join(", "))
}

/// "Uploading 3 of 14" while Share workspace prepares and sends the documents.
#[must_use]
pub fn uploading_text(done: usize, total: usize) -> String {
    format!("Uploading {done} of {total}")
}

/// "Receiving 7 of 20" while a joined workspace's documents arrive (criterion 28).
#[must_use]
pub fn receiving_text(done: usize, total: usize) -> String {
    format!("Receiving {done} of {total}")
}

/// The header line of a shared workspace (section 2.1): "Receiving 7 of 20" while a join's
/// documents arrive, "Offline" without a connection, else "Shared, 2 people".
#[must_use]
pub fn shared_header_line(
    people: usize,
    state: ConnectionState,
    receiving: Option<(usize, usize)>,
) -> String {
    if let Some((done, total)) = receiving {
        return receiving_text(done, total);
    }
    crate::share_text::share_subject(true, people, state)
}

/// The Document block's line for a document of the shared workspace `name` (section 9).
#[must_use]
pub fn shared_through_text(name: &str) -> String {
    format!("Shared through workspace {name}")
}

/// The refusal of a folder chosen for a joined workspace (section 8), by the host's code
/// (`not-empty`, `inside-workspace`, `not-writable`); empty for an unknown code.
#[must_use]
pub fn join_folder_refusal_text(code: &str) -> &'static str {
    match code {
        "not-empty" => "Choose an empty folder.",
        "inside-workspace" => "Choose a folder outside any workspace.",
        "not-writable" | "not-found" => "Cannot write to this folder.",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sections 2.1 and 8: the header line and the join texts.
    #[test]
    fn header_line_and_join_texts() {
        assert_eq!(
            shared_header_line(2, ConnectionState::Connected, None),
            "Shared, 2 people"
        );
        assert_eq!(
            shared_header_line(2, ConnectionState::Offline, None),
            "Offline"
        );
        assert_eq!(
            shared_header_line(2, ConnectionState::Connected, Some((7, 20))),
            "Receiving 7 of 20"
        );
        assert_eq!(uploading_text(3, 14), "Uploading 3 of 14");
        assert_eq!(people_text(1), "1 person");
        assert_eq!(people_text(3), "3 people");
        assert_eq!(not_added_text(&["Lid".to_owned()]), "Not added: Lid.");
        assert_eq!(
            shared_through_text("Box job"),
            "Shared through workspace Box job"
        );
        assert_eq!(
            join_folder_refusal_text("not-empty"),
            "Choose an empty folder."
        );
        assert_eq!(
            join_folder_refusal_text("inside-workspace"),
            "Choose a folder outside any workspace."
        );
        assert_eq!(
            join_folder_refusal_text("not-writable"),
            "Cannot write to this folder."
        );
        assert_eq!(join_folder_refusal_text("other"), "");
    }
}
