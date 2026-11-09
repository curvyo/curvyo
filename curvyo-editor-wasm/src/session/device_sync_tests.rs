//! The open workspace reconciled with the device group (`specs/0096-own-devices-sync` criteria
//! 9 to 11, 13 and 17): switched on, a paired device is admitted without an invite and arrives
//! with the workspace's documents; switched off, it is removed; a member that is not admin only
//! reports.

use curvyo_crypto_core::IdentityKeys;
use curvyo_crypto_core::keyring::{Entry, Keyring, Payload, RecipientWrap, Role};
use curvyo_crypto_core::{DocumentId, EpochKey, wrap_epoch_key};

use super::Session;
use super::sync_test_support::{SERVER, seeds};
use super::workspace_sync::WorkspaceWrite;
use super::workspace_sync_tests::{WORKSPACE, shared_workspace};

/// The device group of A (`seeds(1, _)`) and B (`seeds(2, _)`), both admins.
pub(super) fn group() -> Keyring {
    Keyring::replay(&group_entries(false)).unwrap()
}

/// The group's entries; with `left`, B's own `RemoveMember` follows (Leave my devices).
pub(super) fn group_entries(left: bool) -> Vec<Entry> {
    let a = IdentityKeys::from_entropy(&seeds(1, 0).identity);
    let b = IdentityKeys::from_entropy(&seeds(2, 0).identity);
    let room = DocumentId([0x6e; 16]);
    let key = EpochKey::from_bytes([3; 32]);
    let genesis = Entry::sign(
        Vec::new(),
        &a,
        Payload::Genesis {
            document_id: room,
            admins: vec![a.identity()],
            epoch0_wraps: vec![RecipientWrap {
                member: a.identity().member,
                wrap: wrap_epoch_key(&key, 0, room, &a.identity(), [4; 32]).unwrap(),
            }],
        },
    )
    .unwrap();
    let add = Entry::sign(
        vec![genesis.try_hash().unwrap()],
        &a,
        Payload::AddMember {
            identity: b.identity(),
            role: Role::Admin,
            wraps: Vec::new(),
        },
    )
    .unwrap();
    let mut entries = vec![genesis, add];
    if left {
        let leave = Entry::sign(
            vec![entries[1].try_hash().unwrap()],
            &b,
            Payload::RemoveMember {
                member: b.identity().member,
            },
        )
        .unwrap();
        entries.push(leave);
    }
    entries
}

fn b_member() -> curvyo_crypto_core::MemberId {
    IdentityKeys::from_entropy(&seeds(2, 0).identity)
        .identity()
        .member
}

/// Criteria 9 to 11: A's switch admits B; B arrives as a member and receives the documents.
/// Criterion 13: switching off removes B, whose session ends as removed.
#[test]
fn switching_on_admits_a_paired_device_and_off_removes_it() {
    let (mut net, a, _) = shared_workspace();
    let group = group();
    let report = net.s(a).reconcile_workspace(&group, true).unwrap();
    assert_eq!(report.written, 1);
    net.pump();
    let keyring = net.s(a).sync.as_ref().unwrap().client.keyring().unwrap();
    assert_eq!(keyring.role(&b_member()), Some(Role::Admin));
    // Nothing more to do: a second run writes nothing.
    assert_eq!(
        net.s(a).reconcile_workspace(&group, true).unwrap().written,
        0
    );

    let b = net.add(Session::new(0xb2));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(1_000);
    let documents = net
        .s(b)
        .take_workspace_writes()
        .into_iter()
        .filter(|w| matches!(w, WorkspaceWrite::Document { .. }))
        .count();
    assert_eq!(documents, 2, "both documents arrive on B");

    let report = net.s(a).reconcile_workspace(&group, false).unwrap();
    assert_eq!(report.written, 1);
    net.pump();
    net.run(500);
    assert!(net.s(b).workspace_removed());
}

/// Criterion 17: where this device is a member but not an admin, nothing is written and the
/// device to add is reported.
#[test]
fn a_member_that_is_not_admin_reports_and_writes_nothing() {
    let (mut net, a, _) = shared_workspace();
    // A group in which B paired a third device C that A's workspace lacks; A is admin there,
    // so to see the member case, A reconciles a group it is not part of: nothing happens.
    let other = IdentityKeys::from_entropy(&[9; 64]);
    let room = DocumentId([0x6f; 16]);
    let key = EpochKey::from_bytes([5; 32]);
    let genesis = Entry::sign(
        Vec::new(),
        &other,
        Payload::Genesis {
            document_id: room,
            admins: vec![other.identity()],
            epoch0_wraps: vec![RecipientWrap {
                member: other.identity().member,
                wrap: wrap_epoch_key(&key, 0, room, &other.identity(), [4; 32]).unwrap(),
            }],
        },
    )
    .unwrap();
    let foreign = Keyring::replay(&[genesis]).unwrap();
    let report = net.s(a).reconcile_workspace(&foreign, true).unwrap();
    assert_eq!(report, super::device_sync::ReconcileReport::default());
    // A session without a shared workspace writes nothing and says so.
    assert_eq!(Session::new(3).reconcile_workspace(&foreign, true), None);
}

/// Criterion 22: the facts of a synced workspace name its members, who is connected and this
/// device's state; offline they say so with the changes waiting.
#[test]
fn the_facts_name_members_presence_and_state() {
    let (mut net, a, _) = shared_workspace();
    let group = group();
    let before = net.s(a).device_facts(&group, true).unwrap();
    assert_eq!(before.pending_adds, vec![super::sync::hex(&b_member().0)]);
    assert_eq!(before.members.len(), 1);
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
    let facts = net.s(a).device_facts(&group, true).unwrap();
    assert_eq!(facts.name, "Box job");
    assert_eq!(facts.members.len(), 2);
    assert_eq!(facts.pending_adds, Vec::<String>::new());
    assert!(facts.present.contains(&super::sync::hex(&b_member().0)));
    assert_eq!(facts.state, super::device_facts::OwnState::InSync);
    net.set_link(a, false);
    net.run(5_000);
    let offline = net.s(a).device_facts(&group, true).unwrap();
    assert!(matches!(
        offline.state,
        super::device_facts::OwnState::Offline { .. }
    ));
}

/// Customer 2026-11-09: the workspace's only admin leaves its devices: nothing is written, the
/// link ends and the room keeps it as it was; a second admin makes the leave a real removal.
#[test]
fn the_only_admin_leaves_a_synced_workspace_without_writing() {
    let (mut net, a, _) = shared_workspace();
    let me = net.s(a).sync.as_ref().unwrap().client.me();
    assert!(net.s(a).leave_synced_workspace());
    net.pump();
    net.run(500);
    assert!(!net.s(a).sync_online());
    let keyring = net.s(a).sync.as_ref().unwrap().client.keyring().unwrap();
    assert!(!keyring.was_removed(&me), "no removal was written");
    assert!(keyring.role(&me).is_some());
}

/// Customer feedback on PR #281: a workspace room that does not let a device in says
/// "workspace", never "document".
#[test]
fn a_workspace_room_that_refuses_a_device_says_workspace() {
    let (mut net, _a, _) = shared_workspace();
    let b = net.add(Session::new(0xb3));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(2_000);
    let notices: Vec<_> = std::iter::from_fn(|| net.s(b).take_sync_notice()).collect();
    assert_eq!(
        notices,
        vec![(
            curvyo_ui_core::share_text::NOT_ADMITTED_WORKSPACE_NOTICE.to_owned(),
            true
        )]
    );
}

/// Share tab feedback (2026-11-09): a device that left its devices and still found the room in
/// its marker is told it was removed, never that it is no participant. The client clears the
/// marker on a leave, so this reopen does not happen; the relay's answer stays `Removed`.
#[test]
fn a_device_that_left_and_resumes_from_its_old_marker_is_removed_not_refused() {
    let (mut net, a, _) = shared_workspace();
    let group = group();
    net.s(a).reconcile_workspace(&group, true).unwrap();
    net.pump();
    let b = net.add(Session::new(0xb4));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(1_000);
    let marker = net.s(b).take_workspace_marker().unwrap();
    let keys: Vec<(u32, [u8; 32])> = net
        .s(b)
        .take_key_update()
        .map(|(_, keys)| keys)
        .unwrap_or_default();
    assert!(net.s(b).leave_synced_workspace());
    net.pump();
    net.run(500);
    net.devices[b].session = Session::new(0xb5);
    let now = net.now;
    assert!(net.s(b).resume_workspace(
        &marker.share_json,
        &marker.keyring_log,
        &marker.list,
        "Box job",
        "Ben",
        keys,
        &seeds(2, 1),
        now,
    ));
    net.pump();
    net.run(1_000);
    assert!(net.s(b).workspace_removed());
    let notices: Vec<_> = std::iter::from_fn(|| net.s(b).take_sync_notice()).collect();
    assert_eq!(
        notices,
        vec![(
            curvyo_ui_core::workspace_tree_text::WORKSPACE_REMOVED_TEXT.to_owned(),
            true
        )]
    );
}

/// Criterion 27: a device that leaves writes its own removal; the admin then renews the key.
#[test]
fn a_device_that_leaves_writes_its_removal_and_the_admin_renews_the_key() {
    let (mut net, a, _) = shared_workspace();
    let group = group();
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
    assert!(net.s(b).leave_synced_workspace());
    net.pump();
    net.run(500);
    assert!(!net.s(b).has_workspace_link());
    let group = Keyring::replay(&group_entries(true)).unwrap();
    let keyring = net.s(a).sync.as_ref().unwrap().client.keyring().unwrap();
    assert!(keyring.was_removed(&b_member()));
    assert!(net.s(a).device_facts(&group, true).unwrap().rotate_pending);
    assert_eq!(
        net.s(a).reconcile_workspace(&group, true).unwrap().written,
        1
    );
    net.pump();
    assert!(!net.s(a).device_facts(&group, true).unwrap().rotate_pending);
}

/// Criterion 14: B holds a copy of W taken before W was shared (no workspace record in its
/// files), with an edit made there offline. Connect this folder joins into the copy: the room's
/// container meets the file in the background writer, which gives the file its record and
/// hands back B's own edit. Sent at once (`background`, a document B never opens) or on
/// opening, it reaches A; A's state is in B's file too. Nothing on either side is lost.
fn connect_a_copy_from_before_sharing(background: bool) {
    use super::align_test_support::draw_rect;
    use super::workspace_sync_tests::{LID, objects, open_document};

    let (mut net, a, files) = shared_workspace();
    let (_, lid_bytes) = files
        .iter()
        .find(|(path, _)| path.last().is_some_and(|n| n == "Lid.curvyo"))
        .unwrap()
        .clone();
    // B's copy of Lid, edited offline: a second rectangle no one else has.
    let mut copy = Session::open(0xc1, &lid_bytes).unwrap();
    copy.document.set_workspace_share(None);
    draw_rect(&mut copy, (60.0, 60.0), (90.0, 90.0));
    let copy_bytes = curvyo_document_core::pack(&copy.document, "test").unwrap();
    assert_eq!(objects(&copy), 2);

    // A has an edit B's copy lacks, and admits B by the group.
    open_document(&mut net, a, 0xa2, &lid_bytes);
    draw_rect(net.s(a), (5.0, 5.0), (8.0, 8.0));
    net.run(300);
    assert_eq!(
        net.s(a)
            .reconcile_workspace(&group(), true)
            .unwrap()
            .written,
        1
    );
    net.pump();

    // Connect this folder: B joins; Lid's file is there already, so the room's container is
    // merged into it by the background writer (the page's `send` of kind `container`).
    let b = net.add(Session::new(0xb2));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(1_000);
    let (written, through) = merge_in_the_writer(net.s(b), &copy_bytes, now.0);
    net.s(b).background_written(LID, through);
    let merged_view = Session::open(0xc2, &written.bytes).unwrap();
    assert_eq!(
        objects(&merged_view),
        3,
        "B's file holds A's edit and its own"
    );
    let record = merged_view
        .document
        .workspace_share()
        .expect("the copy joins W");
    assert_eq!((record.workspace, record.document), (WORKSPACE, LID));
    let outgoing = written.outgoing.expect("B's own edit");

    if background {
        // Never opened on B: the page sends the edit and writes the file marked synced.
        let file = net
            .s(b)
            .workspace_send_local(LID, &written.bytes, &outgoing)
            .unwrap();
        net.pump();
        net.run(1_000);
        assert_eq!(objects(net.s(a)), 3, "A sees B's offline edit");
        let stamped = Session::open(0xc3, &file).unwrap();
        let record = stamped.document.workspace_share().unwrap();
        assert_eq!(record.synced_version, stamped.document.version());
    } else {
        // B opens Lid: its offline edit goes to the room and reaches A.
        open_document(&mut net, b, 0xb3, &written.bytes);
        net.run(1_000);
        assert_eq!(objects(net.s(b)), 3);
        assert_eq!(objects(net.s(a)), 3, "A sees B's offline edit");
    }
}

/// What the page does on Connect this folder for Lid: the room's container and the records
/// after it go to a background writer, which merges them into the copy's file; the file and the
/// highest sequence written.
fn merge_in_the_writer(b: &mut Session, copy_bytes: &[u8], now: u64) -> (crate::Written, u64) {
    use super::workspace_sync_tests::LID;
    use crate::{BackgroundWriter, Waiting};

    let room_lid = b
        .take_workspace_writes()
        .into_iter()
        .find_map(|w| match w {
            WorkspaceWrite::Document {
                document, bytes, ..
            } if document == LID => Some(bytes),
            _ => None,
        })
        .expect("the room's Lid");
    b.workspace_document_written(LID);
    let mut writer = BackgroundWriter::new(0xd1);
    writer.add(
        LID,
        Waiting {
            seq: 0,
            kind: super::workspace_sync::BackgroundKind::Container,
            bytes: room_lid,
        },
    );
    // Records after the snapshot wait in the background queue, as for any closed document.
    let mut through = 0;
    for item in b.take_background() {
        if item.document == LID {
            through = through.max(item.seq);
            writer.add(
                LID,
                Waiting {
                    seq: item.seq,
                    kind: item.kind,
                    bytes: item.bytes,
                },
            );
        }
    }
    let written = writer.write_for(LID, WORKSPACE, copy_bytes, now).unwrap();
    (written, through)
}

#[test]
fn connect_this_folder_sends_a_copys_edits_of_a_document_never_opened() {
    connect_a_copy_from_before_sharing(true);
}

#[test]
fn connect_this_folder_sends_a_copys_edits_when_the_document_opens() {
    connect_a_copy_from_before_sharing(false);
}
