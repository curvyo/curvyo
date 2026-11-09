//! Removal, rejected updates, pictures added offline and the label cache of a shared session
//! (`specs/0092-cooperative-editing` criteria 19, 25, 32 to 34, through a shared workspace since
//! `specs/0105`), against one in-process relay.

use std::collections::BTreeMap;

use curvyo_sync_core::{RecordKind, SessionError};
use curvyo_ui_core::workspace_tree_text::WORKSPACE_REMOVED_TEXT as REMOVED_NOTICE;
use serde_json::Value;

use super::Session;
use super::align_test_support::draw_rect;
use super::sync_test_support::{Device, Net, join_open, kept, reopen, seeds, shared_pair, view};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

/// A, B and C: B and C joined by A's invites and converged.
fn shared_three() -> (Net, usize, usize, usize) {
    let (mut net, a, b) = shared_pair();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    net.pump();
    let c = join_open(&mut net, &link, "Cleo", 3, 0xc3);
    (net, a, b, c)
}

fn member_of(session: &Session, label: &str) -> String {
    let v = view(session);
    let row = v["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["label"] == label)
        .cloned()
        .unwrap_or(Value::Null);
    row["member"].as_str().unwrap().to_owned()
}

fn labels(session: &Session) -> Vec<String> {
    view(session)["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["label"].as_str().unwrap().to_owned())
        .collect()
}

fn notices(session: &mut Session) -> Vec<(String, bool)> {
    std::iter::from_fn(|| session.take_sync_notice()).collect()
}

/// Criteria 32 to 34: only an admin sees Remove, never on its own row; A removes B: A's list
/// shows the new epoch and no B, B is told and keeps its copy as a plain local document, nothing
/// travels between A and B any more, and C keeps editing with A across the rotation.
#[test]
fn an_admin_removes_a_participant_with_a_key_rotation() {
    let (mut net, a, b, c) = shared_three();
    let va = view(&net.devices[a].session);
    let removable: Vec<bool> = va["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["removable"].as_bool().unwrap())
        .collect();
    assert_eq!(
        removable,
        [false, true, true],
        "own row first, not removable"
    );
    let vb = view(&net.devices[b].session);
    assert!(
        vb["participants"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["removable"] == false),
        "a member sees no Remove"
    );
    let ben = member_of(&net.devices[a].session, "Ben");
    let cleo = member_of(&net.devices[a].session, "Cleo");
    assert_eq!(
        net.s(b).remove_participant(&cleo),
        Err(SessionError::NotAdmin)
    );
    let objects_before = net.devices[b].session.objects().len();

    net.s(a).remove_participant(&ben).unwrap();
    net.run(2_000);

    let va = view(&net.devices[a].session);
    assert_eq!(va["epoch"], 1, "the new key epoch is shown");
    assert_eq!(labels(&net.devices[a].session), ["Anna", "Cleo"]);
    assert_eq!(view(&net.devices[c].session)["epoch"], 1);
    let vb = view(&net.devices[b].session);
    assert_eq!(vb["ended"], "removed");
    assert_eq!(vb["shared"], false);
    assert!(
        !net.devices[b].session.is_shared(),
        "a plain local document"
    );
    assert!(notices(net.s(b)).contains(&(REMOVED_NOTICE.to_owned(), true)));
    assert_eq!(net.devices[b].session.objects().len(), objects_before);

    // Nothing travels any more; A and C go on across the rotation.
    draw_rect(net.s(a), (60.0, 60.0), (70.0, 70.0));
    draw_rect(net.s(b), (80.0, 80.0), (90.0, 90.0));
    draw_rect(net.s(c), (100.0, 100.0), (110.0, 110.0));
    net.run(1_000);
    assert_eq!(net.json(a), net.json(c));
    assert_eq!(net.devices[a].session.objects().len(), objects_before + 2);
    assert_eq!(net.devices[b].session.objects().len(), objects_before + 1);
    assert!(
        notices(net.s(a)).iter().all(|(_, alert)| !alert),
        "A shows no error"
    );
    // The saved file of B opens without a link.
    let bytes = net.s(b).pack_for_save().unwrap();
    assert!(!Session::open(0xb9, &bytes).unwrap().is_shared());
}

/// Criterion 34: B is removed while offline; when B reconnects it learns of the removal, is
/// told, and its offline edits never reach A.
#[test]
fn a_participant_removed_while_offline_learns_it_on_reconnect() {
    let (mut net, a, b) = shared_pair();
    let ben = member_of(&net.devices[a].session, "Ben");
    net.set_link(b, false);
    net.run(5_000);
    net.s(a).remove_participant(&ben).unwrap();
    net.run(500);
    draw_rect(net.s(b), (80.0, 80.0), (90.0, 90.0));
    let a_objects = net.devices[a].session.objects().len();
    net.devices[b].link_up = true;
    let now = net.now;
    net.s(b).sync_network_back(now);
    net.run(2_000);
    assert_eq!(view(&net.devices[b].session)["ended"], "removed");
    assert!(notices(net.s(b)).contains(&(REMOVED_NOTICE.to_owned(), true)));
    assert_eq!(net.devices[a].session.objects().len(), a_objects);
}

/// Criterion 32: Remove needs the connection and names why not.
#[test]
fn removal_while_offline_is_refused() {
    let (mut net, a, b) = shared_pair();
    let ben = member_of(&net.devices[a].session, "Ben");
    net.set_link(a, false);
    net.run(5_000);
    assert_eq!(
        net.s(a).remove_participant(&ben),
        Err(SessionError::Offline)
    );
    assert_eq!(
        net.s(a).remove_participant("zz"),
        Err(SessionError::NotAMember)
    );
    assert_eq!(labels(&net.devices[a].session).len(), 2);
    let _ = b;
}

/// Criterion 25 and UX notes §7.3: an update that does not import is not merged, "A received
/// update was rejected" is shown once, the tab says "Resyncing", and the session reconnects from
/// the room's snapshot and is connected and converged again.
#[test]
fn a_rejected_update_is_shown_once_and_resyncs_from_the_snapshot() {
    let (mut net, a, b) = shared_pair();
    draw_rect(net.s(a), (50.0, 50.0), (60.0, 60.0));
    net.run(200);
    let connects = net.devices[b].connects;
    let seq = net.devices[b]
        .session
        .sync
        .as_ref()
        .unwrap()
        .client
        .cursor();
    let junk = vec![0, 0, 0, 0, 1, 2, 3];
    {
        let link = net.s(b).sync.as_mut().unwrap();
        link.remote
            .push_back((seq, RecordKind::Update, junk.clone()));
    }
    net.s(b).sync_frame_start();
    assert_eq!(view(&net.devices[b].session)["state"], "resyncing");
    assert_eq!(
        view(&net.devices[b].session)["stateLine"],
        "Resyncing from the server's snapshot."
    );
    net.pump();
    net.run(1_000);
    assert_eq!(net.devices[b].connects, connects + 1, "reconnected once");
    assert_eq!(view(&net.devices[b].session)["state"], "connected");
    assert_eq!(net.json(a), net.json(b));
    // The same record again: no second notice, no second resync.
    {
        let link = net.s(b).sync.as_mut().unwrap();
        link.remote.push_back((seq, RecordKind::Update, junk));
    }
    net.run(500);
    assert_eq!(net.devices[b].connects, connects + 1);
    let shown: Vec<_> = notices(net.s(b))
        .into_iter()
        .filter(|(text, _)| text == "A received update was rejected.")
        .collect();
    assert_eq!(shown.len(), 1);
}

/// Criterion 19 with ADR 0031 decision 9: a picture placed while the shared file was closed and
/// offline goes out with the edits when the file is reopened, so A receives the picture too.
#[test]
fn a_picture_added_offline_before_a_reopen_is_pushed_on_connect() {
    let (mut net, a, b) = shared_pair();
    let (saved, marker, keys) = kept(&mut net, b);
    net.set_link(b, false);
    // B opens the file without a connection, places a picture and saves.
    let mut offline = Session::open(0xb4, &saved).unwrap();
    offline.import_begin(None);
    offline.import_add(PNG, "/home/maker/photo.png");
    let _ = offline.import_finish(false);
    let with_picture = offline.pack_for_save().unwrap();
    // Reopened with the network back: the picture and the edit reach A.
    let reopened = reopen(
        &net,
        0xb5,
        (with_picture, marker, keys),
        "Ben",
        &seeds(2, 2),
    );
    net.devices[b] = Device {
        session: reopened,
        conn: None,
        link_up: true,
        connects: 0,
    };
    net.pump();
    net.run(1_000);
    assert_eq!(net.json(a), net.json(b));
    let hashes = net.devices[a].session.referenced_assets();
    assert_eq!(hashes.len(), 1);
    let hash = hashes.first().unwrap();
    assert!(
        net.devices[a].session.document.asset(hash).is_some(),
        "A holds the picture's bytes"
    );
}

/// adrs.md decision 5: labels of other participants are kept by the device between runs; a
/// label announced in this run wins over a kept one.
#[test]
fn labels_are_kept_between_runs() {
    let (mut net, a, b) = shared_pair();
    let labels_kept = net.devices[a].session.share_labels();
    let ben = member_of(&net.devices[a].session, "Ben");
    assert_eq!(labels_kept.get(&ben).map(String::as_str), Some("Ben"));
    // A reopens its file while B is away: Ben is named from the kept labels.
    let files = kept(&mut net, a);
    net.set_link(b, false);
    let mut reopened = reopen(&net, 0xa5, files, "Anna", &seeds(1, 3));
    assert_eq!(labels(&reopened), ["Anna", "A device"]);
    let mut stale = labels_kept.clone();
    stale.insert("not hex".to_owned(), "x".to_owned());
    reopened.remember_share_labels(&stale);
    assert_eq!(labels(&reopened), ["Anna", "Ben"]);
    let mut renamed = BTreeMap::new();
    renamed.insert(ben, "Old name".to_owned());
    reopened.remember_share_labels(&renamed);
    assert_eq!(
        labels(&reopened),
        ["Anna", "Ben"],
        "the label known first wins"
    );
}

/// Customer 2026-11-09: Stop sharing is for the only participant left; it ends the share here
/// and the saved file carries no share record.
#[test]
fn the_only_participant_left_stops_sharing() {
    let (mut net, a, b) = shared_pair();
    assert_eq!(net.s(a).stop_sharing(), Err(SessionError::NotAdmin));
    let ben = member_of(&net.devices[a].session, "Ben");
    net.s(a).remove_participant(&ben).unwrap();
    net.pump();
    net.run(1_000);
    assert!(
        net.devices[b]
            .session
            .sync
            .as_ref()
            .is_some_and(|l| l.ended.is_some())
    );
    net.s(a).stop_sharing().unwrap();
    net.pump();
    net.run(1_000);
    assert!(!net.devices[a].session.is_shared());
    let bytes = net.s(a).pack_for_save().unwrap();
    let file = curvyo_document_core::unpack(1, &bytes).unwrap();
    assert!(
        file.workspace_share().is_none(),
        "no share record in the file"
    );
}
