//! Tester acceptance tests for `specs/0105-sharing-concept` texts and the people view, written
//! from the specification before the implementation was read: the fixed texts of criteria 3,
//! 4, 7, 9, 10 to 12, 16 and 19, the device word over every share text module (criterion 18),
//! and participants grouped by person (criterion 14).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_ui_core::sharing_text::{
    ADD_DEVICE, DESKTOP_LINK_REFUSAL, DOCUMENT_LINK_REFUSAL, INVITE_PERSON, JOIN,
    LEGACY_SHARE_NOTICE, MAKE_WORKSPACE, PersonDevice, SHARING_WORKS_ON_WORKSPACES, SYNC_CONFIRM,
    UNNAMED_DEVICE, WORKSPACES_ON_DESKTOP, group_people, make_workspace_confirm, workspace_state,
};

/// Criteria 3, 4, 7, 11, 12, 16 and 19: the texts the spec fixes word for word.
#[test]
fn fixed_texts_match_the_specification() {
    assert_eq!(
        DOCUMENT_LINK_REFUSAL,
        "This link is for a single shared document. Documents are now shared in workspaces: ask for a workspace invite."
    );
    assert_eq!(
        LEGACY_SHARE_NOTICE,
        "This document was shared on its own. Documents are now shared in a workspace; it opens as a local copy."
    );
    assert_eq!(SHARING_WORKS_ON_WORKSPACES, "Sharing works on workspaces.");
    assert_eq!(MAKE_WORKSPACE, "Make a workspace for this file");
    assert_eq!(INVITE_PERSON, "Invite person");
    assert_eq!(JOIN, "Join…");
    assert_eq!(ADD_DEVICE, "Add device");
    assert_eq!(DESKTOP_LINK_REFUSAL, "Open this link in the desktop app.");
    assert_eq!(UNNAMED_DEVICE, "A device");
}

/// Criterion 9: in the browser the line says workspaces are made in the desktop app.
#[test]
fn c9_the_browser_line_names_the_desktop_app() {
    assert!(WORKSPACES_ON_DESKTOP.contains("desktop app"));
    assert!(WORKSPACES_ON_DESKTOP.starts_with("Sharing works on workspaces"));
}

/// Criteria 10 and 11 (the Sync switch removed, customer feedback on PR #298): the confirm of
/// Invite person says names, folders and files leave encrypted (with
/// "device", criterion 18 wins over the spec's "computer").
#[test]
fn c10_the_sync_confirm_names_what_leaves() {
    for word in ["names", "folders", "files", "leave", "encrypted"] {
        assert!(SYNC_CONFIRM.to_lowercase().contains(word), "{word}");
    }
}

/// Criterion 10: the three workspace states.
#[test]
fn c10_workspace_states() {
    assert_eq!(workspace_state(false, false), "Local");
    assert_eq!(workspace_state(false, true), "Local");
    assert_eq!(workspace_state(true, false), "Synced");
    assert_eq!(workspace_state(true, true), "Shared");
}

/// Criterion 8: the inline confirm names the folder and says the file moves there.
#[test]
fn c8_the_confirm_names_the_folder_and_the_move() {
    let text = make_workspace_confirm("/home/marc/Boxes/Lid/", "Lid.curvyo");
    assert!(text.contains("/home/marc/Boxes/Lid/"), "{text}");
    assert!(text.contains("Lid.curvyo"), "{text}");
    assert!(text.contains("moves"), "{text}");
}

/// String literals of a source outside comments.
fn literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let mut parts = code.split('"');
        parts.next();
        while let Some(literal) = parts.next() {
            out.push(literal.to_owned());
            parts.next();
        }
    }
    out
}

/// Criterion 18 over every share-flow text module, including pairing and the workspace texts:
/// no "computer", no "this browser" for a device.
#[test]
fn c18_no_text_of_the_share_flows_says_computer_or_this_browser() {
    let sources = [
        ("share_text", include_str!("../src/share_text.rs")),
        ("sharing_text", include_str!("../src/sharing_text.rs")),
        ("devices_text", include_str!("../src/devices_text.rs")),
        ("pairing_text", include_str!("../src/pairing_text.rs")),
        ("membership_text", include_str!("../src/membership_text.rs")),
        (
            "membership_return_text",
            include_str!("../src/membership_return_text.rs"),
        ),
        (
            "workspace_share_text",
            include_str!("../src/workspace_share_text.rs"),
        ),
        ("workspace_text", include_str!("../src/workspace_text.rs")),
        (
            "workspace_tree_text",
            include_str!("../src/workspace_tree_text.rs"),
        ),
        ("key_backup_text", include_str!("../src/key_backup_text.rs")),
    ];
    for (module, source) in sources {
        for text in literals(source) {
            let lower = text.to_lowercase();
            assert!(!lower.contains("computer"), "{module}: {text}");
            assert!(!lower.contains("this browser"), "{module}: {text}");
        }
    }
}

fn d(label: &str, own: bool, admin: bool) -> PersonDevice<'_> {
    PersonDevice { label, own, admin }
}

/// Criterion 14: Marc with two devices (this one an admin) and Lena with one.
#[test]
fn c14_people_are_grouped_by_label() {
    let devices = [
        d("Lena", false, false),
        d("Marc", false, false),
        d("Marc", true, true),
    ];
    let people = group_people(&devices);
    let lines: Vec<&str> = people.iter().map(|p| p.line.as_str()).collect();
    assert_eq!(
        lines,
        vec!["Marc (you) · Admin · 2 devices", "Lena · Member · 1 device"]
    );
    assert_eq!(
        people[0].devices,
        vec![1, 2],
        "a row expands to its devices"
    );
    assert_eq!(people[1].devices, vec![0]);
    assert!(people[0].you && !people[1].you);
}

/// Criterion 14: a device with no label shows "A device" and is never merged with another
/// unlabeled device.
#[test]
fn c14_unlabeled_devices_stay_apart() {
    let people = group_people(&[
        d("", false, false),
        d("", false, true),
        d("Marc", true, true),
    ]);
    let lines: Vec<&str> = people.iter().map(|p| p.line.as_str()).collect();
    assert_eq!(
        lines,
        vec![
            "Marc (you) · Admin · 1 device",
            "A device · Member · 1 device",
            "A device · Admin · 1 device",
        ]
    );
}

/// Edge: an empty participant list and a single own device.
#[test]
fn c14_edges_empty_and_alone() {
    assert_eq!(group_people(&[]), Vec::new());
    let alone = group_people(&[d("Marc", true, true)]);
    assert_eq!(alone.len(), 1);
    assert_eq!(alone[0].line, "Marc (you) · Admin · 1 device");
}
