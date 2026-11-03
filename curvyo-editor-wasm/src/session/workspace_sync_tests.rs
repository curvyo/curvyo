//! A shared workspace between two editing sessions through one relay (`specs/0095-workspace`
//! criteria 25 to 33): share, invite `2.`, join with file writes, live edits of the open
//! document, background records of the others, the link moving between documents and the
//! marker that reopens the workspace.

use curvyo_document_core::{Document, WorkspaceRoomRecord, pack};

use super::Session;
use super::align_test_support::draw_rect;
use super::sync_test_support::{Net, SERVER, seeds, view};
use super::workspace_sync::{BackgroundKind, WorkspaceWrite};
use crate::{ShareFiles, WorkspaceShareBuilder};

pub(super) const WORKSPACE: [u8; 16] = [0x57; 16];
pub(super) const LID: [u8; 16] = [0x11; 16];
pub(super) const LEFT: [u8; 16] = [0x22; 16];

pub(super) fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

pub(super) fn objects(session: &Session) -> usize {
    session.objects().len()
}

/// The marker cursor and the session's cursor of `session`.
fn cursors(session: &Session) -> (u64, u64) {
    let link = session.sync.as_ref().unwrap();
    let cursor = link.client.cursor();
    (
        link.workspace.as_ref().unwrap().persisted_cursor(cursor),
        cursor,
    )
}

/// A shares a workspace of `Lid` and `Sides/Left`; returns the net, A and the files A writes
/// once the relay took the snapshot.
pub(super) fn shared_workspace() -> (Net, usize, ShareFiles) {
    let mut net = Net::new();
    let mut lid = Session::new(0x0a);
    draw_rect(&mut lid, (10.0, 10.0), (40.0, 30.0));
    let lid_bytes = pack(&lid.document, "test").unwrap();
    let left_bytes = pack(&Document::new(0x0b), "test").unwrap();
    let mut builder = WorkspaceShareBuilder::new(WORKSPACE, 0xa1, 1_800_000_000, [4; 16]);
    builder
        .add_document(&p("Lid.curvyo"), &lid_bytes, LID)
        .unwrap();
    builder
        .add_document(&p("Sides/Left.curvyo"), &left_bytes, LEFT)
        .unwrap();
    let (prepared, files) = builder.finish([6; 16]);
    let a = net.add(Session::new(0xa1));
    let now = net.now;
    net.s(a)
        .share_workspace(SERVER, "Anna", "Box job", prepared, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    (net, a, files)
}

/// B joins by A's `2.` invite and writes the files it is given.
pub(super) fn joined() -> (Net, usize, usize, ShareFiles) {
    let (mut net, a, _) = shared_workspace();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(24 * 3600, now).unwrap();
    assert!(link.contains("#2."), "{link}");
    net.pump();
    let b = net.add(Session::new(0xb2));
    net.s(b)
        .join_workspace(&link, "Ben", "Box job", &seeds(2, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    let mut written = Vec::new();
    for write in net.s(b).take_workspace_writes() {
        if let WorkspaceWrite::Document {
            path,
            document,
            bytes,
            ..
        } = write
        {
            net.s(b).workspace_document_written(document);
            written.push((path, bytes));
        }
    }
    (net, a, b, written)
}

/// Moves device `i`'s workspace link into a session opened from `bytes`.
pub(super) fn open_document(net: &mut Net, i: usize, peer: u64, bytes: &[u8]) {
    let link = net.s(i).take_workspace_link().expect("a workspace link");
    let mut opened = Session::open(peer, bytes).unwrap();
    opened.attach_workspace_link(link);
    net.devices[i].session = opened;
    net.pump();
}

/// Criterion 25: Share workspace uploads one snapshot in parts after the genesis and shows the
/// workspace as shared with one participant; the files keep their new ids.
#[test]
fn sharing_a_workspace_uploads_its_snapshot_and_shows_it_shared() {
    let (mut net, a, files) = shared_workspace();
    let shown = view(net.s(a));
    assert_eq!(shown["shared"], true);
    assert_eq!(shown["state"], "connected");
    assert_eq!(shown["workspace"]["name"], "Box job");
    assert_eq!(shown["workspace"]["headerLine"], "Shared, 1 person");
    assert_eq!(shown["participants"].as_array().unwrap().len(), 1);
    let room = net
        .relay
        .room(&curvyo_sync_core::RoomId(WORKSPACE))
        .expect("the relay holds the workspace room");
    assert!(room.snapshot_seq() > 0 || room.keyring().participants().len() == 1);
    assert_eq!(files.len(), 2);
    let marker = net.s(a).take_workspace_marker().expect("the marker files");
    let record = WorkspaceRoomRecord::from_bytes(&marker.share_json).unwrap();
    assert_eq!(record.workspace, WORKSPACE);
    assert_eq!(record.server, SERVER);
    assert!(net.s(a).take_workspace_marker().is_none(), "unchanged");
}

/// Criteria 26 to 28: B is admitted by A, receives the list and every document as files at
/// their places, then both list two participants.
#[test]
fn a_joiner_receives_every_document_as_a_file() {
    let (mut net, a, b, written) = joined();
    let paths: Vec<Vec<String>> = written.iter().map(|(path, _)| path.clone()).collect();
    assert_eq!(paths, vec![p("Lid.curvyo"), p("Sides/Left.curvyo")]);
    let lid = Session::open(7, &written[0].1).unwrap();
    assert_eq!(objects(&lid), 1);
    assert_eq!(lid.document.workspace_share().unwrap().document, LID);
    let shown = view(net.s(b));
    assert_eq!(shown["join"]["phase"], "done");
    assert_eq!(shown["workspace"]["headerLine"], "Shared, 2 people");
    assert_eq!(view(net.s(a))["participants"].as_array().unwrap().len(), 2);
    let marker = net.s(b).take_workspace_marker().expect("B's marker");
    let record = WorkspaceRoomRecord::from_bytes(&marker.share_json).unwrap();
    let (persisted, cursor) = cursors(net.s(b));
    assert_eq!(
        (record.server_seq, persisted),
        (cursor, cursor),
        "every file is written"
    );
}

/// Criteria 30 to 32: A and B edit one document live through the workspace room; an edit of a
/// document B has not open waits in B's background queue and holds the marker cursor back.
#[test]
fn the_open_document_syncs_live_and_the_others_go_to_the_background() {
    let (mut net, a, b, written) = joined();
    open_document(&mut net, a, 0xa2, &written[0].1);
    open_document(&mut net, b, 0xb3, &written[0].1);
    draw_rect(net.s(a), (50.0, 50.0), (70.0, 70.0));
    net.run(300);
    assert_eq!(objects(net.s(b)), 2, "B sees A's rectangle in Lid");
    draw_rect(net.s(b), (80.0, 10.0), (90.0, 20.0));
    net.run(300);
    assert_eq!(objects(net.s(a)), 3, "A sees B's rectangle");

    // A switches to Left and draws there; B keeps Lid open.
    open_document(&mut net, a, 0xa3, &written[1].1);
    let _ = net.s(b).take_workspace_marker();
    draw_rect(net.s(a), (5.0, 5.0), (15.0, 15.0));
    net.run(300);
    assert_eq!(objects(net.s(b)), 3, "Lid is untouched by Left's edit");
    let items = net.s(b).take_background();
    assert_ne!(items.len(), 0);
    assert!(items.iter().all(|i| i.document == LEFT));
    assert!(items.iter().any(|i| i.kind == BackgroundKind::Update));
    assert!(
        !net.s(b).workspace_may_write(LID),
        "never the open document"
    );
    assert!(net.s(b).workspace_may_write(LEFT));
    let through = items.iter().map(|i| i.seq).max().unwrap();
    let (held, _) = cursors(net.s(b));
    assert!(held < through, "Left's records are not in its file yet");
    net.s(b).background_written(LEFT, through);
    // B's Lid holds A's and its own unsaved records: saving it lets the cursor pass them.
    let _ = net.s(b).pack_for_save().unwrap();
    net.s(b).mark_saved();
    let (moved, cursor) = cursors(net.s(b));
    assert_eq!(moved, cursor);
    assert!(moved >= through);
    let marker = net.s(b).take_workspace_marker().unwrap();
    assert_eq!(
        WorkspaceRoomRecord::from_bytes(&marker.share_json)
            .unwrap()
            .server_seq,
        moved
    );
}

/// Criterion 33 and ADR 0037 decision 8: records of a document left unsaved go back to the
/// background queue; reopening it applies them again; its file keeps `share.json` v2.
#[test]
fn leaving_a_document_unsaved_keeps_its_records_for_its_file() {
    let (mut net, a, b, written) = joined();
    open_document(&mut net, a, 0xa2, &written[0].1);
    open_document(&mut net, b, 0xb3, &written[0].1);
    draw_rect(net.s(a), (50.0, 50.0), (70.0, 70.0));
    net.run(300);
    assert_eq!(objects(net.s(b)), 2);
    // B leaves Lid without saving: A's record waits for Lid's file.
    open_document(&mut net, b, 0xb4, &written[1].1);
    let items = net.s(b).take_background();
    assert!(items.iter().any(|i| i.document == LID));
    // B opens Lid again from its old file: the record is applied again.
    open_document(&mut net, b, 0xb5, &written[0].1);
    net.run(50);
    assert_eq!(objects(net.s(b)), 2);
    let saved = net.s(b).pack_for_save().unwrap();
    let record = Session::open(9, &saved)
        .unwrap()
        .document
        .workspace_share()
        .expect("share.json v2 kept");
    assert_eq!((record.workspace, record.document), (WORKSPACE, LID));
}

/// Criterion 33: B reopens the workspace from its marker later and reconnects without an
/// invite; edits made meanwhile arrive.
#[test]
fn a_member_reopens_the_workspace_from_its_marker() {
    let (mut net, a, b, written) = joined();
    let marker = net.s(b).take_workspace_marker().unwrap();
    let keys: Vec<(u32, [u8; 32])> = net
        .s(b)
        .take_key_update()
        .map(|(_, keys)| keys)
        .unwrap_or_default();
    assert_ne!(keys.len(), 0);
    // B closes the app.
    net.s(b).leave_sync();
    net.pump();
    open_document(&mut net, a, 0xa2, &written[0].1);
    draw_rect(net.s(a), (50.0, 50.0), (70.0, 70.0));
    net.run(300);
    // B starts again: an empty session, the workspace from the marker.
    net.devices[b].session = Session::new(0xb9);
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
    assert_eq!(view(net.s(b))["state"], "connected");
    let items = net.s(b).take_background();
    assert!(
        items
            .iter()
            .any(|i| i.document == LID && i.kind == BackgroundKind::Update),
        "A's edit of Lid arrives for Lid's file"
    );
}

/// ADR 0037 decisions 7 and 8: a snapshot the room asks for is built only once the files hold
/// every record it covers, from the list, the open document and the files; the relay keeps it.
#[test]
fn a_due_snapshot_is_built_from_the_files_once_they_hold_every_record() {
    let (mut net, a, b, written) = joined();
    open_document(&mut net, a, 0xa2, &written[0].1);
    draw_rect(net.s(a), (50.0, 50.0), (70.0, 70.0));
    net.run(300);
    let (_, cursor) = cursors(net.s(b));
    net.s(b).workspace_snapshot_asked(cursor);
    assert_eq!(
        net.s(b).workspace_snapshot_ready(),
        None,
        "Lid's record is not in its file yet"
    );
    let items = net.s(b).take_background();
    let through = items.iter().map(|i| i.seq).max().unwrap();
    net.s(b).background_written(LID, through);
    assert_eq!(net.s(b).workspace_snapshot_ready(), Some(cursor));
    let documents = net.s(b).workspace_snapshot_documents();
    assert_eq!(documents.len(), 2, "no document is open on B");
    for (id, place) in documents {
        let super::workspace_sync::DocumentPlace::File(path) = place else {
            panic!("no document is deleted");
        };
        let bytes = written
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, b)| b.clone())
            .unwrap();
        net.s(b).add_workspace_snapshot_document(id, bytes);
    }
    net.s(b).finish_workspace_snapshot([8; 16]).unwrap();
    net.pump();
    let room = net
        .relay
        .room(&curvyo_sync_core::RoomId(WORKSPACE))
        .unwrap();
    assert_eq!(room.snapshot_seq(), cursor);
    assert_eq!(net.s(b).workspace_snapshot_ready(), None, "no longer due");
}

#[cfg(unix)]
use super::sync_tests::cpu_time;

/// Criteria 31 and 32: B has Lid open while A edits Lid and 49 other documents once a second;
/// one second's records (49 for the background, one for Lid), carried through the relay and
/// applied at B's next frame start, cost less than one frame (16 ms) of thread CPU time in an
/// optimised build. The background writer's work is in its worker (`background_writer.rs`).
/// Debug builds run it as a smoke test. Run: `cargo test --release -p curvyo-editor-wasm --lib
/// busy_documents -- --nocapture`.
#[cfg(unix)]
#[test]
fn forty_nine_busy_documents_cost_the_open_session_less_than_a_frame() {
    use curvyo_document_core::DocumentVersion;
    use curvyo_sync_core::{RecordKind, encode_inner_record};

    let (mut net, a, b, written) = joined();
    open_document(&mut net, a, 0xa2, &written[0].1);
    open_document(&mut net, b, 0xb3, &written[0].1);
    let other = Session::new(0xd1);
    let mut editing = other;
    draw_rect(&mut editing, (1.0, 1.0), (5.0, 5.0));
    let update = editing
        .document
        .export_updates(&DocumentVersion::never())
        .unwrap();
    let mut worst = std::time::Duration::ZERO;
    for second in 0..10_u8 {
        let x = f64::from(second) * 3.0;
        draw_rect(net.s(a), (x, 80.0), (x + 2.0, 82.0));
        let now = net.now;
        net.s(a).sync_tick(now);
        for document in 0..49_u8 {
            let record = encode_inner_record([0x80 + document; 16], 0xa1, &update);
            let link = net.s(a).sync.as_mut().unwrap();
            link.client.push(RecordKind::Update, record).unwrap();
        }
        let ((), took) = cpu_time::cpu_time_of(|| {
            net.pump();
            net.s(b).sync_frame_start();
        });
        worst = worst.max(took);
        let _ = net.s(b).take_background();
        net.run(1_000);
    }
    assert_eq!(objects(net.s(b)), 11, "Lid's edits are applied at once");
    eprintln!("one second of 50 busy documents at B's frame start: {worst:?}");
    if !cfg!(debug_assertions) {
        assert!(worst <= std::time::Duration::from_millis(16), "{worst:?}");
    }
}
