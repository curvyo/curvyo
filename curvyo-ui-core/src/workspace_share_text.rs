//! The words of a shared workspace in the Share tab and the File tab's header line
//! (`specs/0095-workspace` criteria 25 to 29, `ux-notes.md` sections 2.1, 4, 8 and 9).

use crate::share_text::{ConnectionState, server_shown};

/// The sentence of criterion 26 under a workspace invite.
pub const WORKSPACE_INVITE_SENTENCE: &str =
    "Anyone with this link can edit every document of this workspace once, until it expires.";

/// The join's waiting line (`ux-notes.md` section 8).
pub const WORKSPACE_WAITING_TEXT: &str = "Waiting for a member to let you in";

/// The inline confirm of Share workspace (section 4): the workspace, its document count and
/// the server, the one place the privacy fact is said; the documents shared on their own that
/// are taken in (criterion 42) and the ones that stay out (criterion 49) are named beforehand.
#[must_use]
pub fn workspace_share_confirm_text(
    name: &str,
    documents: usize,
    server: &str,
    taken_in: &[String],
    not_added: &[String],
) -> String {
    let count = match documents {
        1 => "1 document".to_owned(),
        n => format!("{n} documents"),
    };
    let text = format!(
        "Upload the workspace {name} ({count}) to {}? Names, folders and files leave this \
         computer encrypted; the server cannot read them.",
        server_shown(server)
    );
    format!("{text}{}", takeover_text(taken_in, not_added))
}

/// The sentences about documents shared on their own that a share takes in, and those it leaves
/// out, each with a leading space; empty for none.
#[must_use]
pub fn takeover_text(taken_in: &[String], not_added: &[String]) -> String {
    let taken_in = match taken_in {
        [] => String::new(),
        [one] => format!(
            " {one} is shared on its own. It becomes part of the workspace and its own link \
             stops working."
        ),
        [first, rest @ ..] => {
            let others = match rest.len() {
                1 => "1 other document".to_owned(),
                n => format!("{n} other documents"),
            };
            format!(
                " {first} and {others} are shared on their own. They become part of the \
                 workspace and their own links stop working."
            )
        }
    };
    let outside = if not_added.is_empty() {
        String::new()
    } else {
        format!(
            " Not added: {} (shared with people outside this workspace).",
            not_added.join(", ")
        )
    };
    format!("{taken_in}{outside}")
}

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

    /// Section 4: the confirm names the workspace, its count, the server, the documents taken
    /// in (criterion 42) and the ones left out (criterion 49).
    #[test]
    fn the_share_confirm_names_the_workspace_and_what_stays_out() {
        assert_eq!(
            workspace_share_confirm_text("Box job", 14, "ws://192.168.1.20:8080", &[], &[]),
            "Upload the workspace Box job (14 documents) to 192.168.1.20:8080? Names, folders \
             and files leave this computer encrypted; the server cannot read them."
        );
        let text = workspace_share_confirm_text(
            "Box job",
            1,
            "ws://192.168.1.20:8080",
            &[],
            &["Lid".to_owned(), "Base".to_owned()],
        );
        assert!(text.contains("(1 document)"));
        assert!(
            text.ends_with(" Not added: Lid, Base (shared with people outside this workspace).")
        );
        let one = workspace_share_confirm_text("Box job", 3, "ws://h:1", &["Lid".to_owned()], &[]);
        assert!(one.ends_with(
            " Lid is shared on its own. It becomes part of the workspace and its own link stops \
             working."
        ));
        let three = workspace_share_confirm_text(
            "Box job",
            3,
            "ws://h:1",
            &["Lid".to_owned(), "Base".to_owned(), "Side".to_owned()],
            &[],
        );
        assert!(three.ends_with(
            " Lid and 2 other documents are shared on their own. They become part of the \
             workspace and their own links stop working."
        ));
    }

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
