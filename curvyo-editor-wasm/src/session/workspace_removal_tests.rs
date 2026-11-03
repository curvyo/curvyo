//! Removing a member from a shared workspace (`specs/0095-workspace` criteria 43 to 45): one
//! rotation for the whole workspace, one fresh workspace snapshot under the new epoch, and the
//! removed device keeps its documents as local files.

use super::sync_test_support::view;
use super::workspace_sync::DocumentPlace;
use super::workspace_sync_tests::{joined, open_document};

fn member_of(session: &super::Session, label: &str) -> String {
    let shown = view(session);
    shown["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["label"] == label)
        .and_then(|p| p["member"].as_str())
        .unwrap()
        .to_owned()
}

/// Criteria 43 and 44: A removes B; the keyring rotates once, a workspace snapshot under the new
/// epoch is due and is built from the files; B is out, its open document a local file.
#[test]
fn removal_rotates_the_workspace_key_and_writes_one_fresh_snapshot() {
    let (mut net, a, b, written) = joined();
    open_document(&mut net, b, 0xb3, &written[0].1);
    assert!(net.s(b).document.workspace_share().is_some());
    let ben = member_of(net.s(a), "Ben");
    assert!(
        view(net.s(b))["participants"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["removable"] == false),
        "a member sees no Remove"
    );
    net.s(a).remove_participant(&ben).unwrap();
    net.run(2_000);

    assert_eq!(
        view(net.s(a))["epoch"],
        1,
        "one epoch sequence for the workspace"
    );
    assert_eq!(view(net.s(a))["participants"].as_array().unwrap().len(), 1);
    assert!(net.s(b).workspace_removed());
    let told: Vec<String> =
        std::iter::from_fn(|| net.s(b).take_sync_notice().map(|(text, _)| text)).collect();
    assert!(
        told.contains(&curvyo_ui_core::workspace_tree_text::WORKSPACE_REMOVED_TEXT.to_owned()),
        "B is told in the workspace's words: {told:?}"
    );
    assert!(!net.s(b).has_workspace_link());
    // `ux-notes.md` section 4: the Share tab shows a local workspace, "Not shared".
    let shown = view(net.s(b));
    assert_eq!(shown["shared"], false);
    assert!(shown["workspace"].is_null(), "{shown}");
    assert!(
        net.s(b).document.workspace_share().is_none(),
        "B's open document is a local file"
    );
    assert!(
        net.s(b).take_workspace_marker().is_none(),
        "B writes no marker"
    );

    let covers = net
        .s(a)
        .workspace_snapshot_ready()
        .expect("a snapshot is due");
    for (id, place) in net.s(a).workspace_snapshot_documents() {
        let DocumentPlace::File(path) = place else {
            panic!("no document is deleted");
        };
        let bytes = written.iter().find(|(p, _)| *p == path).unwrap().1.clone();
        net.s(a).add_workspace_snapshot_document(id, bytes);
    }
    net.s(a).finish_workspace_snapshot([7; 16]).unwrap();
    net.pump();
    let room = net
        .relay
        .room(&curvyo_sync_core::RoomId(
            super::workspace_sync_tests::WORKSPACE,
        ))
        .unwrap();
    assert_eq!(room.snapshot_seq(), covers);
    assert_eq!(room.keyring().current_epoch().number, 1);
}

/// Criterion 45: B is removed while offline; on reconnect it learns it from the keyring, leaves
/// the workspace room, and its offline edits do not reach A.
#[test]
fn a_removal_found_on_reconnect_ends_the_workspace_and_keeps_offline_edits_local() {
    let (mut net, a, b, written) = joined();
    open_document(&mut net, a, 0xa2, &written[0].1);
    open_document(&mut net, b, 0xb3, &written[0].1);
    let before = super::workspace_sync_tests::objects(net.s(a));
    net.set_link(b, false);
    net.run(6_000);
    super::align_test_support::draw_rect(net.s(b), (80.0, 80.0), (90.0, 90.0));
    let ben = member_of(net.s(a), "Ben");
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    net.set_link(b, true);
    net.run(30_000);
    assert!(net.s(b).workspace_removed());
    assert_eq!(
        super::workspace_sync_tests::objects(net.s(a)),
        before,
        "B's offline edit is not merged"
    );
}
