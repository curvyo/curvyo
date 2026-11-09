//! The words of the workspace-only sharing model (`specs/0105-sharing-concept`): the refusals of
//! links, the legacy notice, Make a workspace for this file, Invite person and why it is greyed,
//! the one Join field, and the people view that groups participants by their label for display.

/// The notice on opening a file a `0092` or `0097` build shared on its own (criterion 4).
pub const LEGACY_SHARE_NOTICE: &str = "This document was shared on its own. Documents are now shared in a workspace; it opens as a local copy.";

/// A `1.` link in the Join field (criterion 3).
pub const DOCUMENT_LINK_REFUSAL: &str = "This link is for a single shared document. Documents are now shared in workspaces: ask for a workspace invite.";

/// A person's workspace link in the browser's Join field (criterion 19, Proposal).
pub const DESKTOP_LINK_REFUSAL: &str = "Open this link in the desktop app.";

/// The line above Make a workspace for this file (criterion 7).
pub const SHARING_WORKS_ON_WORKSPACES: &str = "Sharing works on workspaces.";

/// The line of the browser, which makes no workspaces (criterion 9, Proposal).
pub const WORKSPACES_ON_DESKTOP: &str = "Sharing works on workspaces, made in the desktop app.";

/// The button for a file outside any workspace (criterion 7).
pub const MAKE_WORKSPACE: &str = "Make a workspace for this file";

/// The inline confirm of Make a workspace for this file: the new folder and the move (criterion
/// 8).
#[must_use]
pub fn make_workspace_confirm(folder: &str, file: &str) -> String {
    format!("New folder {folder}. {file} moves into it.")
}

/// The one confirm of Invite person in a Local workspace, which starts sharing it (criteria 10
/// and 11).
pub const SYNC_CONFIRM: &str = "Names, folders and files leave this device encrypted.";

/// My devices' switch of a workspace (`0096`, kept by the customer on PR #298).
pub const SYNC_TO_MY_DEVICES: &str = "Sync to my devices";

/// The invite action (criterion 11).
pub const INVITE_PERSON: &str = "Invite person";

/// What decides whether Invite person can work (customer feedback on PR #298).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Independent facts of the Share tab.
pub struct InviteFacts {
    /// The desktop app (the browser takes part only as one of my devices).
    pub desktop: bool,
    /// A workspace is open in the tree, or the session carries one's link.
    pub workspace_open: bool,
    /// A server address is set.
    pub server_set: bool,
    /// The server answered the last check (or the link is live).
    pub server_reachable: bool,
    /// A name shown to others is set.
    pub name_set: bool,
    /// The workspace already syncs (a live link).
    pub linked: bool,
    /// This device is an admin of the linked workspace.
    pub admin: bool,
    /// A Share is starting.
    pub busy: bool,
}

/// Why Invite person is greyed, its tooltip; `None` when it works. The first reason counts.
#[must_use]
pub fn invite_blocker(facts: &InviteFacts) -> Option<&'static str> {
    if !facts.desktop {
        return Some("Invite people from the desktop app.");
    }
    if !facts.workspace_open {
        return Some("Open a workspace first.");
    }
    if facts.linked {
        return (!facts.admin).then_some("Only an admin can invite.");
    }
    if facts.busy {
        return Some("Sharing is starting.");
    }
    if !facts.server_set {
        return Some("Set a server address first.");
    }
    if !facts.server_reachable {
        return Some("The server cannot be reached.");
    }
    if !facts.name_set {
        return Some("Enter a name first.");
    }
    None
}

/// The one Join entry and its field (criterion 16).
pub const JOIN: &str = "Join…";
/// The label of the Join field.
pub const JOIN_FIELD: &str = "Link or code";

/// The action of My devices that pairs another device (criterion 18).
pub const ADD_DEVICE: &str = "Add device";

/// A device that sealed no label (criterion 14).
pub const UNNAMED_DEVICE: &str = "A device";

/// The workspace's state in the Workspace block (criterion 10).
#[must_use]
pub fn workspace_state(synced: bool, others: bool) -> &'static str {
    match (synced, others) {
        (false, _) => "Local",
        (true, false) => "Synced",
        (true, true) => "Shared",
    }
}

/// One participant device as the people view reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonDevice<'a> {
    /// The label the device sealed; empty when it sealed none.
    pub label: &'a str,
    /// This device.
    pub own: bool,
    /// The device is an admin.
    pub admin: bool,
}

/// One person of the people view: display only, the grouping is unverified (ADR 0044 decision
/// 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// "Marc (you) · Admin · 2 devices".
    pub line: String,
    /// Indices of the person's devices in the input, in input order.
    pub devices: Vec<usize>,
    /// The person holds this device.
    pub you: bool,
}

/// Groups participant devices by their label (criterion 14): the person holding this device
/// first, then in the order the devices come; a device without a label is a person of its own.
/// A person is an admin when any of its devices is.
#[must_use]
pub fn group_people(devices: &[PersonDevice<'_>]) -> Vec<Person> {
    let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
    for (index, device) in devices.iter().enumerate() {
        match groups
            .iter_mut()
            .find(|(label, _)| !device.label.is_empty() && *label == device.label)
        {
            Some((_, members)) => members.push(index),
            None => groups.push((device.label, vec![index])),
        }
    }
    let mut people: Vec<Person> = groups
        .into_iter()
        .map(|(label, members)| {
            let you = members.iter().any(|&i| devices[i].own);
            let admin = members.iter().any(|&i| devices[i].admin);
            let name = if label.is_empty() {
                UNNAMED_DEVICE
            } else {
                label
            };
            let count = members.len();
            Person {
                line: format!(
                    "{name}{} · {} · {count} {}",
                    if you { " (you)" } else { "" },
                    if admin { "Admin" } else { "Member" },
                    if count == 1 { "device" } else { "devices" },
                ),
                devices: members,
                you,
            }
        })
        .collect();
    people.sort_by_key(|person| !person.you);
    people
}
