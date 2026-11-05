//! The membership lifecycle in the editing session (`specs/0097-membership-lifecycle` criteria
//! 1 to 5, 7, 10 to 12, 15 to 21, 30), against one in-process relay: roles, leave, Change keys
//! now, the ended record, Add back, the return check, Sync again and Keep as it is.

use std::io::Cursor;

use curvyo_crypto_core::keyring::Keyring;
use curvyo_document_core::EndReason;
use curvyo_sync_core::SessionError;
use serde_json::Value;

use super::Session;
use super::align_test_support::draw_rect;
use super::sync_test_support::{Net, SERVER, seeds, shared_pair, view};

/// The epoch keys one device stored.
type StoredKeys = Vec<(u32, [u8; 32])>;

/// A, B and C: B and C joined by A's invites and converged; the keys each one stored.
fn shared_three() -> (Net, [usize; 3], Vec<StoredKeys>) {
    let (mut net, a, b) = shared_pair();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    net.pump();
    let c = net.add(Session::new(0xc3));
    net.s(c)
        .join_document(&link, "Cleo", &seeds(3, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    let mut keys = Vec::new();
    for i in [a, b, c] {
        keys.push(
            net.s(i)
                .take_key_update()
                .map(|(_, k)| k)
                .unwrap_or_default(),
        );
    }
    (net, [a, b, c], keys)
}

fn row(session: &Session, label: &str) -> Value {
    view(session)["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["label"] == label)
        .cloned()
        .unwrap_or(Value::Null)
}

fn member_of(session: &Session, label: &str) -> String {
    row(session, label)["member"].as_str().unwrap().to_owned()
}

fn notice_kinds(session: &Session, previous: Option<&[String]>) -> Vec<String> {
    let json: Value =
        serde_json::from_str(&session.membership_notices_json(previous, &[])).unwrap();
    json.as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn notices(session: &mut Session) -> Vec<String> {
    std::iter::from_fn(|| session.take_sync_notice().map(|(t, _)| t)).collect()
}

fn member_names(bytes: &[u8]) -> Vec<String> {
    zip::ZipArchive::new(Cursor::new(bytes))
        .unwrap()
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .collect()
}

/// Criteria 1 to 5, 7 and 30: the rows offer the role actions an admin may take; A makes B an
/// admin and both see it with a notice; the only admin cannot step down.
#[test]
fn rows_offer_role_actions_and_a_promotion_is_a_notice() {
    let (mut net, [a, b, c], _) = shared_three();
    let ben = row(&net.devices[a].session, "Ben");
    assert_eq!(ben["canMakeAdmin"], true);
    assert_eq!(ben["canMakeMember"], false);
    let own = row(&net.devices[a].session, "Anna");
    assert_eq!(own["canStepDown"], true);
    assert_eq!(own["canLeave"], true);
    let as_member = row(&net.devices[b].session, "Cleo");
    assert_eq!(as_member["canMakeAdmin"], false, "a member changes no role");
    assert!(notice_kinds(&net.devices[a].session, None).contains(&"oneAdmin".to_owned()));

    let anna = member_of(&net.devices[a].session, "Anna");
    assert_eq!(
        net.s(a).set_participant_role(&anna, false),
        Err(SessionError::LastAdmin)
    );
    assert_eq!(net.s(a).leave_share("Poster"), Err(SessionError::LastAdmin));
    let seen_by_c = net.s(c).share_admin_set();
    let ben = member_of(&net.devices[a].session, "Ben");
    net.s(a).set_participant_role(&ben, true).unwrap();
    net.run(1_000);
    for i in [a, b, c] {
        assert_eq!(row(&net.devices[i].session, "Ben")["admin"], true);
    }
    assert_eq!(view(&net.devices[b].session)["admin"], true);
    let kinds = notice_kinds(&net.devices[c].session, Some(&seen_by_c));
    assert!(kinds.contains(&"nowAdmin".to_owned()), "{kinds:?}");
    assert!(!kinds.contains(&"oneAdmin".to_owned()));
    // Now A may step down.
    net.s(a).set_participant_role(&anna, false).unwrap();
    net.run(1_000);
    assert_eq!(row(&net.devices[c].session, "Anna")["admin"], false);
}

/// Criteria 10 to 13: B leaves; its file is local with the ended record "left", it is told
/// so; A sees the pending rotation until Change keys now.
#[test]
fn a_leave_keeps_the_way_back_and_asks_for_new_keys() {
    let (mut net, [a, b, _], _) = shared_three();
    net.s(b).leave_share("Poster").unwrap();
    net.run(1_000);
    let ended = net.devices[b].session.document.ended_share().unwrap();
    assert_eq!(ended.ended, EndReason::Left);
    assert_eq!(ended.server, SERVER);
    assert!(!net.devices[b].session.is_shared());
    assert_eq!(view(&net.devices[b].session)["ended"], "left");
    assert!(
        notices(net.s(b)).contains(&"You left Poster. Your copy stays on this device.".to_owned()),
        "the leaver's notice names the file (ux-notes 5.2)"
    );
    let file = curvyo_document_core::pack(&net.devices[b].session.document, "t").unwrap();
    let names = member_names(&file);
    assert!(names.iter().any(|n| n == "share.ended.json"), "{names:?}");
    assert!(!names.iter().any(|n| n == "share.json"));
    // Nothing more goes out from B.
    let connects = net.devices[b].connects;
    net.run(5_000);
    assert_eq!(net.devices[b].connects, connects);

    assert!(notice_kinds(&net.devices[a].session, None).contains(&"rotationPending".to_owned()));
    let former = view(&net.devices[a].session)["former"].clone();
    assert_eq!(former[0]["label"], "Ben", "the label is kept: {former}");
    assert_eq!(former[0]["left"], true);
    net.s(a).rotate_share_keys().unwrap();
    net.run(1_000);
    assert!(!notice_kinds(&net.devices[a].session, None).contains(&"rotationPending".to_owned()));
}

/// Criteria 17 to 21, 25 and 26: A removes B and adds B back "from now on"; B checks, is told,
/// syncs again and its edit made while away merges; C converges with both.
#[test]
fn a_removed_participant_checks_and_syncs_again() {
    let (mut net, [a, b, c], keys) = shared_three();
    let ben = member_of(&net.devices[a].session, "Ben");
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    let ended = net.devices[b].session.document.ended_share().unwrap();
    assert_eq!(ended.ended, EndReason::Removed);
    let removed_row = view(&net.devices[a].session)["former"][0].clone();
    assert_eq!(removed_row["left"], false);
    draw_rect(net.s(b), (80.0, 80.0), (90.0, 90.0));
    draw_rect(net.s(a), (60.0, 60.0), (70.0, 70.0));
    net.run(500);

    let now = net.now;
    assert!(
        net.s(b)
            .check_added_back(keys[1].clone(), &seeds(2, 1), now)
    );
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "notYet");

    net.s(a).add_back_participant(&ben, false).unwrap();
    net.run(1_000);
    let participants = view(&net.devices[a].session)["participants"].clone();
    assert!(
        participants
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["member"] == ben.as_str() && p["admin"] == false),
        "B is back as a member: {participants}"
    );
    let now = net.now;
    assert!(
        net.s(b)
            .check_added_back(keys[1].clone(), &seeds(2, 2), now)
    );
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "addedBack");
    assert!(
        !net.devices[b].session.is_shared(),
        "nothing syncs before B chooses"
    );

    let now = net.now;
    assert!(net.s(b).sync_again("Ben", &seeds(2, 3), now));
    net.pump();
    net.run(3_000);
    assert!(net.devices[b].session.is_shared());
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(net.json(a), net.json(c));
    assert!(
        net.s(b).take_key_update().is_some(),
        "the new key is stored"
    );
}

/// Criterion 21 (Frage 162 A): Keep as it is sends one own leave; the admins see B left and
/// the record says "left".
#[test]
fn keep_as_it_is_leaves_again() {
    let (mut net, [a, b, _], keys) = shared_three();
    let ben = member_of(&net.devices[a].session, "Ben");
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    net.s(a).add_back_participant(&ben, true).unwrap();
    net.run(1_000);
    let now = net.now;
    net.s(b)
        .check_added_back(keys[1].clone(), &seeds(2, 1), now);
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "addedBack");
    let before = net.devices[b].session.document.ended_share().unwrap().heads;
    assert!(!before.is_empty(), "the removed record keeps B's heads");
    let now = net.now;
    assert!(net.s(b).keep_as_it_is(&seeds(2, 2), now));
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "leftAgain");
    let ended = net.devices[b].session.document.ended_share().unwrap();
    assert_eq!(ended.ended, EndReason::Left);
    // The record's heads now hold the new leave, so the earlier add back answers no later check.
    assert_ne!(ended.heads, before);
    let doc_room = net.devices[a].session.sync.as_ref().unwrap().client.room();
    let room = net.relay.room(&doc_room).unwrap();
    assert!(ended.heads.iter().all(|h| {
        room.keyring()
            .is_accepted(&curvyo_crypto_core::keyring::EntryHash(*h))
    }));
    let former = view(&net.devices[a].session)["former"].clone();
    assert_eq!(former[0]["left"], true, "{former}");
    assert!(notice_kinds(&net.devices[a].session, None).contains(&"rotationPending".to_owned()));
}

/// Criteria 15 and 16 (ADR 0041 decision 3): with C (another maker) an admin of the synced
/// workspace, A's person leave removes A's other device B with one rotation, waits for the
/// snapshot that asks for, then leaves itself; the marker's ended record says "left" and C
/// keeps the workspace.
#[test]
fn a_person_leave_removes_the_own_devices_after_the_snapshot() {
    use super::workspace_sync_tests::{WORKSPACE, shared_workspace};
    let (mut net, a, _) = shared_workspace();
    let group = super::device_sync_tests::group();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    net.pump();
    let c = net.add(Session::new(0xc3));
    net.s(c)
        .join_workspace(&link, "Cleo", "Box job", &seeds(3, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    assert!(
        !net.s(a).person_leave_allowed(&group),
        "no admin outside A's devices yet"
    );
    let cleo = member_of(&net.devices[a].session, "Cleo");
    net.s(a).set_participant_role(&cleo, true).unwrap();
    net.s(a).reconcile_workspace(&group, true).unwrap();
    net.pump();
    let b = net.add(Session::new(0xb2));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(1_000);
    assert!(net.s(a).person_leave_allowed(&group));
    // B is still an admin here; a group that removed B (stolen) still counts it as the maker's
    // own device, so before C was an admin it was no outside admin (ADR 0041 decision 1).
    let without_b = Keyring::replay(&super::device_sync_tests::group_entries(true)).unwrap();
    assert!(
        net.s(a).person_leave_allowed(&without_b),
        "C is an outside admin"
    );
    net.s(a).set_participant_role(&cleo, false).unwrap();
    net.run(500);
    assert!(
        !net.s(a).person_leave_allowed(&without_b),
        "a device the group removed is no outside admin"
    );
    net.s(a).set_participant_role(&cleo, true).unwrap();
    net.run(500);

    let report = net.s(a).reconcile_left_workspace(&group).unwrap();
    assert!(report.leaving);
    assert_eq!(report.written, 1, "B's removal with its rotation");
    net.pump();
    net.run(200);
    assert!(net.s(b).workspace_removed(), "B is out");
    assert!(net.s(a).has_workspace_link(), "A waits for the snapshot");
    let covers = net
        .s(a)
        .workspace_snapshot_ready()
        .expect("a snapshot is due");
    net.s(a).finish_workspace_snapshot([8; 16]).unwrap();
    net.pump();
    net.run(500);
    assert!(net.s(a).workspace_removed(), "then A leaves");
    let told = notices(net.s(a));
    assert!(
        told.iter().any(|t| t.starts_with("You left ")),
        "a leave is told as a leave: {told:?}"
    );
    let ended = net.s(a).workspace_ended_record().unwrap();
    assert_eq!(ended.ended, EndReason::Left);
    assert_eq!(ended.room, WORKSPACE);
    let room = net
        .relay
        .room(&curvyo_sync_core::RoomId(WORKSPACE))
        .unwrap();
    assert_eq!(room.snapshot_seq(), covers, "the snapshot went out first");
    let keyring = room.keyring();
    let members: Vec<_> = keyring
        .participants()
        .iter()
        .map(|p| p.identity.member)
        .collect();
    assert_eq!(members.len(), 1, "only C stays");
    assert!(notice_kinds(&net.devices[c].session, None).contains(&"rotationPending".to_owned()));
}

/// Criterion 21 for a workspace: Keep as it is hands the host the marker's record "left".
#[test]
fn keep_as_it_is_for_a_workspace_gives_the_marker_a_left_record() {
    let (mut net, a, b, _) = super::workspace_sync_tests::joined();
    let ben = member_of(&net.devices[a].session, "Ben");
    let keys = net
        .s(b)
        .take_key_update()
        .map(|(_, k)| k)
        .unwrap_or_default();
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    let ended = net.s(b).workspace_ended_record().unwrap();
    assert_eq!(ended.ended, EndReason::Removed);
    net.s(a).add_back_participant(&ben, false).unwrap();
    net.run(1_000);
    let json = ended.to_bytes().unwrap();
    let now = net.now;
    assert!(
        net.s(b)
            .check_workspace_added_back(&json, keys, &seeds(2, 1), now)
    );
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "addedBack");
    let now = net.now;
    assert!(net.s(b).keep_as_it_is(&seeds(2, 2), now));
    net.pump();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["check"], "leftAgain");
    let record = net.s(b).workspace_ended_record().unwrap();
    assert_eq!(record.ended, EndReason::Left);
}

/// Criteria 20 and 33 (ux-notes 7.1 and 7.3): a removed file opened again names the server and
/// room Check would contact and connects nowhere by itself; the add-back prompt names the admin
/// by the label this device kept.
#[test]
fn a_reopened_removed_file_names_where_check_goes_and_who_added_back() {
    let (mut net, [a, b, _], keys) = shared_three();
    let room = net.devices[b].session.shared_room_hex();
    let ben = member_of(&net.devices[a].session, "Ben");
    let anna = member_of(&net.devices[a].session, "Anna");
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    assert_eq!(view(&net.devices[b].session)["endedRoom"], room.as_str());
    let file = curvyo_document_core::pack(&net.devices[b].session.document, "t").unwrap();
    let reopened = Session::open(0xb9, &file).unwrap();
    let shown = view(&reopened);
    assert_eq!(shown["ended"], "removed");
    assert_eq!(shown["server"], SERVER);
    assert_eq!(shown["endedRoom"], room.as_str());
    assert_eq!(reopened.shared_room_hex(), "", "nothing resumes the room");

    net.s(a).add_back_participant(&ben, true).unwrap();
    net.run(1_000);
    let now = net.now;
    assert!(
        net.s(b)
            .check_added_back(keys[1].clone(), &seeds(2, 1), now)
    );
    let labels = std::collections::BTreeMap::from([(anna, "Anna".to_owned())]);
    net.s(b).remember_share_labels(&labels);
    net.pump();
    net.run(1_000);
    let shown = view(&net.devices[b].session);
    assert_eq!(shown["check"], "addedBack");
    assert_eq!(shown["addedBy"], "Anna");
    assert_eq!(shown["endedRoom"], room.as_str());
}
