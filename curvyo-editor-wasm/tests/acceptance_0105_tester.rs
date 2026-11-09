//! Tester acceptance tests for `specs/0105-sharing-concept` at the session, written from the
//! specification before the implementation was read: a file shared on its own opens with one
//! notice, no link and no connection, and saves without its old members (criteria 4, 5); a
//! `1.` link is refused by Join with nothing to send (criterion 3); a workspace document and a
//! plain file open without the notice (criterion 6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Cursor;

use curvyo_editor_wasm::{Session, SyncSeeds};
use curvyo_sync_core::{InviteKind, InviteLink, InviteSecret, Millis, RoomId};
use curvyo_ui_core::sharing_text::LEGACY_SHARE_NOTICE;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn names(bytes: &[u8]) -> Vec<String> {
    zip::ZipArchive::new(Cursor::new(bytes))
        .unwrap()
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .collect()
}

fn seeds() -> SyncSeeds {
    SyncSeeds {
        identity: [3; 64],
        nonce: [4; 32],
        entropy: [5; 32],
    }
}

/// Criteria 4 and 5 through Open and Save of the session.
#[test]
fn c4_c5_legacy_goldens_open_local_with_one_notice_and_save_clean() {
    for name in ["shared_v34.curvyo", "ended_v36.curvyo"] {
        let bytes = fixture(name);
        let mut session = Session::open_file(3, &bytes, name).unwrap();
        assert_eq!(session.take_open_notice(), LEGACY_SHARE_NOTICE, "{name}");
        assert_eq!(session.take_open_notice(), "", "{name}: one notice");
        assert!(!session.is_shared(), "{name}: no (shared)");
        assert_eq!(session.sync_next_action(), None, "{name}: no connection");
        assert!(!session.is_dirty(), "{name}: opening is no edit");
        let saved = session.pack_for_save().unwrap();
        let members = names(&saved);
        for legacy in ["share.json", "keyring.log", "share.ended.json"] {
            assert!(!members.iter().any(|m| m == legacy), "{name}: {members:?}");
        }
        let mut again = Session::open_file(4, &saved, name).unwrap();
        assert_eq!(
            again.take_open_notice(),
            "",
            "{name}: no notice after the save"
        );
        assert_eq!(fixture(name), bytes, "{name}: golden untouched");
    }
}

/// Criterion 6 and a plain file: no notice.
#[test]
fn c6_workspace_documents_and_plain_files_open_without_the_notice() {
    let mut ws =
        Session::open_file(3, &fixture("workspace_shared_v35.curvyo"), "W.curvyo").unwrap();
    assert_eq!(ws.take_open_notice(), "");
    let plain = Session::new(7)
        .pack(curvyo_editor_wasm::APP_VERSION)
        .unwrap();
    let mut plain = Session::open_file(8, &plain, "P.curvyo").unwrap();
    assert_eq!(plain.take_open_notice(), "");
}

/// Criterion 3: Join refuses a `1.` link before anything is set up: no transport action, no
/// link; a `2.` link of the same secret is taken (a connect follows).
#[test]
fn c3_join_refuses_a_document_link_with_nothing_to_send() {
    let link = |kind| {
        InviteLink {
            server: "ws://192.0.2.1:8787".to_owned(),
            kind,
            secret: InviteSecret {
                room: RoomId([0x07; 16]),
                seed: [9; 32],
            },
        }
        .to_text()
    };
    let now = Millis(1_800_000_000_000);
    let mut session = Session::new(1);
    let document = link(InviteKind::Document);
    assert!(document.contains("#1."));
    assert!(
        session
            .join_workspace(&document, "Ben", "Box job", &seeds(), now)
            .is_err()
    );
    assert_eq!(session.sync_next_action(), None, "nothing to send");
    assert!(!session.is_shared());
    // The control: a workspace link starts a connection.
    let mut other = Session::new(2);
    other
        .join_workspace(
            &link(InviteKind::Workspace),
            "Ben",
            "Box job",
            &seeds(),
            now,
        )
        .unwrap();
    assert!(other.sync_next_action().is_some(), "a 2. link connects");
}
