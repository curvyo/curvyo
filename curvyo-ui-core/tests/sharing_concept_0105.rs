//! `specs/0105-sharing-concept` texts and the people view: the device word (criterion 18), the
//! refusals of links (criteria 3, 19), the legacy notice (criterion 4) and participants grouped
//! by person for display (criterion 14).

#![allow(clippy::unwrap_used)]

use curvyo_ui_core::sharing_text::{
    DESKTOP_LINK_REFUSAL, DOCUMENT_LINK_REFUSAL, InviteFacts, LEGACY_SHARE_NOTICE, PersonDevice,
    group_people, invite_blocker, make_workspace_confirm,
};

/// The string literals of a source file outside comments.
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

/// Criterion 18: no text of the Share tab and the share flows calls a device "computer" or
/// "this browser", and the add action reads "Add device".
#[test]
fn share_texts_say_device() {
    let sources = [
        include_str!("../src/share_text.rs"),
        include_str!("../src/sharing_text.rs"),
        include_str!("../src/devices_text.rs"),
        include_str!("../src/membership_text.rs"),
        include_str!("../src/membership_return_text.rs"),
        include_str!("../src/workspace_share_text.rs"),
        include_str!("../src/workspace_tree_text.rs"),
        include_str!("../src/key_backup_text.rs"),
    ];
    let mut add_device = false;
    for source in sources {
        for text in literals(source) {
            let lower = text.to_lowercase();
            assert!(!lower.contains("computer"), "{text}");
            assert!(!lower.contains("this browser"), "{text}");
            assert!(!lower.contains("add my"), "{text}");
            add_device |= text == "Add device";
        }
    }
    assert!(add_device, "the add action reads Add device");
}

/// Criteria 3, 4 and 19: the fixed texts.
#[test]
fn refusals_and_the_legacy_notice() {
    assert_eq!(
        DOCUMENT_LINK_REFUSAL,
        "This link is for a single shared document. Documents are now shared in workspaces: ask for a workspace invite."
    );
    assert_eq!(DESKTOP_LINK_REFUSAL, "Open this link in the desktop app.");
    assert_eq!(
        LEGACY_SHARE_NOTICE,
        "This document was shared on its own. Documents are now shared in a workspace; it opens as a local copy."
    );
    assert_eq!(
        make_workspace_confirm("/home/marc/Box/Lid/", "Lid.curvyo"),
        "New folder /home/marc/Box/Lid/. Lid.curvyo moves into it."
    );
}

/// Criterion 14: Marc's two devices and Lena's one are two people; the own person comes first;
/// a device with no label is "A device" on its own.
#[test]
fn participants_are_grouped_by_their_label() {
    let devices = [
        PersonDevice {
            label: "Lena",
            own: false,
            admin: false,
        },
        PersonDevice {
            label: "Marc",
            own: true,
            admin: true,
        },
        PersonDevice {
            label: "",
            own: false,
            admin: false,
        },
        PersonDevice {
            label: "Marc",
            own: false,
            admin: false,
        },
    ];
    let people = group_people(&devices);
    let lines: Vec<&str> = people.iter().map(|p| p.line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "Marc (you) · Admin · 2 devices",
            "Lena · Member · 1 device",
            "A device · Member · 1 device"
        ]
    );
    assert_eq!(people[0].devices, vec![1, 3]);
    assert_eq!(people[1].devices, vec![0]);
    assert!(people[0].you && !people[1].you);
}

/// Customer feedback on PR #298: Invite person is greyed whenever it cannot work, with the reason
/// as its tooltip; it is offered only with an open workspace, a server that answers and a name.
#[test]
fn invite_person_says_why_it_cannot_work() {
    let ready = InviteFacts {
        desktop: true,
        workspace_open: true,
        server_set: true,
        server_reachable: true,
        name_set: true,
        linked: false,
        admin: false,
        busy: false,
    };
    assert_eq!(invite_blocker(&ready), None);
    let linked_admin = InviteFacts {
        linked: true,
        admin: true,
        ..ready
    };
    assert_eq!(invite_blocker(&linked_admin), None);
    let cases = [
        (
            InviteFacts {
                desktop: false,
                ..ready
            },
            "Invite people from the desktop app.",
        ),
        (
            InviteFacts {
                workspace_open: false,
                ..ready
            },
            "Open a workspace first.",
        ),
        (
            InviteFacts {
                server_set: false,
                ..ready
            },
            "Set a server address first.",
        ),
        (
            InviteFacts {
                server_reachable: false,
                ..ready
            },
            "The server cannot be reached.",
        ),
        (
            InviteFacts {
                name_set: false,
                ..ready
            },
            "Enter a name first.",
        ),
        (
            InviteFacts {
                linked: true,
                ..ready
            },
            "Only an admin can invite.",
        ),
        (
            InviteFacts {
                busy: true,
                ..ready
            },
            "Sharing is starting.",
        ),
    ];
    for (facts, why) in cases {
        assert_eq!(invite_blocker(&facts), Some(why), "{facts:?}");
    }
    // The first reason counts: no workspace open is said before a missing server.
    let nothing = InviteFacts {
        workspace_open: false,
        server_set: false,
        ..ready
    };
    assert_eq!(invite_blocker(&nothing), Some("Open a workspace first."));
}
