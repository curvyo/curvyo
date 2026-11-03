//! The session's binding of the relay session (`specs/0092-cooperative-editing` criteria 1 to
//! 7, 14 to 19, 21): two or three editing sessions against one in-process relay, with the
//! transport carried out as the host would (`TransportAction`s) on a virtual clock.

use curvyo_document_core::Point;
use serde_json::Value;

use super::align_test_support::{click, draw_rect};
use super::sync_test_support::{Device, Net, ROOM, SERVER, STEP_MS, seeds, shared_pair, view};
use super::{Session, Tool};

/// Criteria 1 and 3: sharing writes a genesis with A as the only admin, uploads, lists "this
/// device"; joining by the link admits B automatically, replaces B's blank document by A's,
/// opens it unsaved, and both lists show both participants with colours and fingerprints.
#[test]
fn share_and_join_converge_and_list_both_participants() {
    let (mut net, a, b) = shared_pair();
    assert_eq!(net.json(a), net.json(b), "B holds A's document");
    assert!(
        net.devices[b].session.is_dirty(),
        "a joined document opens unsaved"
    );
    for i in [a, b] {
        let v = view(&net.devices[i].session);
        assert_eq!(v["shared"], true);
        assert_eq!(v["state"], "connected");
        let rows = v["participants"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["self"], true, "own row first");
        for row in rows {
            assert_eq!(row["fingerprint"].as_str().unwrap().len(), 19);
            assert!((1..=6).contains(&row["colour"].as_u64().unwrap()));
        }
    }
    let va = view(&net.devices[a].session);
    assert_eq!(
        va["participants"][1]["label"], "Ben",
        "the label travels by presence"
    );
    assert_eq!(va["admin"], true);
    assert_eq!(view(&net.devices[b].session)["admin"], false);
    assert_eq!(va["epoch"], 0);
}

/// UX notes §2.5: the joiner's Share tab stays shown when the room's document replaces the blank
/// one ("the tab switches to the shared state").
#[test]
fn a_join_keeps_the_share_tab_shown() {
    let mut net = Net::new();
    let mut a = Session::new(0xa1);
    draw_rect(&mut a, (10.0, 10.0), (40.0, 30.0));
    let a = net.add(a);
    let now = net.now;
    net.s(a)
        .share_document(SERVER, "Anna", ROOM, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    let (link, _) = net.s(a).create_invite(24 * 3600, now).unwrap();
    net.pump();
    let b = net.add(Session::new(0xb2));
    assert!(net.s(b).press_panel_tab("share"));
    net.s(b)
        .join_document(&link, "Ben", &seeds(2, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    assert_eq!(net.json(a), net.json(b), "B holds A's document");
    assert_eq!(
        net.devices[b].session.panel_content(),
        curvyo_ui_core::PanelContent::Share
    );
}

/// Criteria 6 and 8: an edit in either session reaches the other at the next frame, as one
/// update per commit, and both keep their own.
#[test]
fn edits_travel_both_ways_and_converge() {
    let (mut net, a, b) = shared_pair();
    draw_rect(net.s(a), (50.0, 50.0), (60.0, 60.0));
    draw_rect(net.s(b), (70.0, 70.0), (80.0, 80.0));
    net.run(100);
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(net.devices[a].session.objects().len(), 3);
    assert!(net.devices[b].session.take_sync_changed());
}

/// Criteria 7, 14 and 15: B sees A's pointer and selection in A's colour, A's move drag moves
/// the object in B's frame before the release, B hovering it gets the marker naming A, and the
/// presence disappears when A's connection goes.
#[test]
fn presence_shows_pointer_selection_drag_and_marker() {
    let (mut net, a, b) = shared_pair();
    click(net.s(a), (20.0, 20.0), false);
    net.s(a).pointer_hover(Point::new(20.0, 20.0), false, false);
    net.run(200);
    let marks: Value = serde_json::from_str(&net.devices[b].session.presence_marks_json()).unwrap();
    let mark = &marks[0];
    assert_eq!(mark["label"], "Anna");
    assert!(mark["pointer"].is_array());
    assert!(mark["selection"].is_array(), "A's selection is outlined");
    assert!(mark["marker"].is_null());
    // B hovers the object A has selected: "Anna is editing this".
    net.s(b).set_tool(Tool::Select);
    net.s(b).pointer_hover(Point::new(10.0, 20.0), false, false);
    assert!(net.devices[b].session.hovered_object.is_some());
    let marks: Value = serde_json::from_str(&net.devices[b].session.presence_marks_json()).unwrap();
    assert_eq!(marks[0]["marker"], "Anna is editing this");
    // A drags the rectangle; B draws it moved while A still holds the button.
    let before = net.devices[b]
        .session
        .with_remote_drags(net.devices[b].session.objects());
    net.s(a).pointer_down(Point::new(20.0, 20.0), false);
    net.s(a).pointer_hover(Point::new(45.0, 20.0), false, false);
    net.run(200);
    let during = net.devices[b]
        .session
        .with_remote_drags(net.devices[b].session.objects());
    let x = |objects: &[curvyo_document_core::ObjectSnapshot]| {
        curvyo_ui_core::object_bounds(&objects[0]).0.x
    };
    assert!((x(&during) - x(&before) - 25.0).abs() < 1e-6, "moved live");
    assert_eq!(
        net.json(a),
        net.json(b),
        "nothing committed during the drag"
    );
    let history_before = net.devices[b].session.document.history_len();
    net.s(a).pointer_up(Point::new(45.0, 20.0), false, false);
    net.run(200);
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(
        net.devices[b].session.document.history_len(),
        history_before + 1,
        "one history entry for the drag"
    );
    // A's network goes: B's marks are gone within 5 s.
    net.set_link(a, false);
    net.run(5_000);
    assert_eq!(net.devices[b].session.presence_marks_json(), "[]");
}

/// Criterion 5: a saved shared document keeps its room and keyring and reconnects when
/// reopened, without an invite, with the keys from the store.
#[test]
fn a_saved_shared_document_reopens_and_reconnects() {
    let (mut net, a, b) = shared_pair();
    let (room, keys) = net.s(b).take_key_update().expect("B's epoch keys to store");
    assert_eq!(room, super::sync::hex(&ROOM));
    let bytes = net.s(b).pack_for_save().unwrap();
    net.s(b).mark_saved();
    net.set_link(b, false);
    // A edits while B is closed.
    draw_rect(net.s(a), (90.0, 90.0), (95.0, 95.0));
    net.run(100);
    let mut reopened = Session::open(0xb3, &bytes).unwrap();
    assert!(reopened.is_shared());
    let now = net.now;
    assert!(reopened.resume_shared("Ben", keys, &seeds(2, 1), now));
    net.devices[b] = Device {
        session: reopened,
        conn: None,
        link_up: true,
        connects: 0,
    };
    net.pump();
    net.run(1_000);
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(view(&net.devices[b].session)["state"], "connected");
}

/// Criteria 17 and 18: B offline keeps editing ("Offline" within 5 s), A edits meanwhile; when
/// the network is back B reconnects at once and both hold everything.
#[test]
fn offline_edits_merge_when_the_network_is_back() {
    let (mut net, a, b) = shared_pair();
    net.set_link(b, false);
    net.run(4_500);
    assert_eq!(view(&net.devices[b].session)["state"], "offline");
    for n in 0..5 {
        let x = f64::from(n) * 6.0;
        draw_rect(net.s(b), (100.0 + x, 0.0), (104.0 + x, 4.0));
        draw_rect(net.s(a), (100.0 + x, 50.0), (104.0 + x, 54.0));
    }
    net.run(30_000);
    net.devices[b].link_up = true;
    let connects = net.devices[b].connects;
    let now = net.now;
    net.s(b).sync_network_back(now);
    net.pump();
    assert_eq!(net.devices[b].connects, connects + 1, "reconnects at once");
    net.run(500);
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(net.devices[a].session.objects().len(), 11);
    let notice = net.s(b).take_sync_notice();
    assert_eq!(notice, Some(("Back online.".to_owned(), false)));
}

/// Criterion 4: a used link is refused with its reason and shows nothing of the document.
#[test]
fn a_used_link_is_refused_and_shows_nothing() {
    let mut net = Net::new();
    let a = net.add(Session::new(0xa1));
    let now = net.now;
    net.s(a)
        .share_document(SERVER, "Anna", ROOM, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    let b = net.add(Session::new(0xb2));
    net.s(b)
        .join_document(&link, "Ben", &seeds(2, 0), now)
        .unwrap();
    net.run(1_000);
    let c = net.add(Session::new(0xc3));
    draw_rect(net.s(c), (1.0, 1.0), (2.0, 2.0));
    let blank = net.json(c);
    net.s(c)
        .join_document(&link, "Cleo", &seeds(3, 0), now)
        .unwrap();
    net.run(1_000);
    let v = view(&net.devices[c].session);
    assert_eq!(v["join"]["phase"], "refused");
    assert_eq!(v["join"]["text"], "This link was already used.");
    assert_eq!(net.json(c), blank, "nothing of A's document");
    assert!(
        Session::new(1)
            .join_document("not a link", "x", &seeds(4, 0), now)
            .is_err()
    );
}

/// Criterion 21: A's undo reverts A's own last edit and leaves B's alone, in both replicas.
#[test]
fn undo_reverts_only_the_own_edit() {
    let (mut net, a, b) = shared_pair();
    draw_rect(net.s(a), (50.0, 50.0), (60.0, 60.0));
    net.run(100);
    draw_rect(net.s(b), (70.0, 70.0), (80.0, 80.0));
    net.run(100);
    let result = net.s(a).undo();
    assert!(result.changed);
    net.run(100);
    assert_eq!(net.json(a), net.json(b));
    assert_eq!(
        net.devices[b].session.objects().len(),
        2,
        "A's rectangle gone, B's kept"
    );
    let rect = |s: &Session| {
        s.objects()
            .iter()
            .any(|o| (curvyo_ui_core::object_bounds(o).0.x - 70.0).abs() < 1e-6)
    };
    assert!(rect(&net.devices[a].session));
}

/// Criterion 22: A's undo of a rectangle B has since deleted is skipped with a notice of at most
/// 70 characters naming B, and nothing comes back.
#[test]
fn an_undo_of_what_a_peer_deleted_is_skipped_and_names_the_peer() {
    let (mut net, a, b) = shared_pair();
    draw_rect(net.s(a), (50.0, 50.0), (60.0, 60.0));
    net.run(200);
    click(net.s(b), (50.0, 55.0), false);
    assert_eq!(net.devices[b].session.selection.ids().len(), 1);
    net.s(b).delete_selected();
    net.run(200);
    assert_eq!(net.devices[a].session.objects().len(), 1);
    // The History tab names the live participant's step (adrs.md decision 4).
    let rows: Value = serde_json::from_str(&net.s(a).history_rows_json(0, 5)).unwrap();
    assert_eq!(rows[0]["peerLabel"], "Ben");
    let result = net.s(a).undo();
    assert!(!result.changed);
    assert_eq!(result.notice, "Skipped: Ben deleted that object.");
    assert!(result.notice.chars().count() <= 70);
    net.run(200);
    assert_eq!(net.devices[a].session.objects().len(), 1, "not resurrected");
    assert_eq!(net.json(a), net.json(b));
}

#[cfg(unix)]
#[path = "../../tests/cpu_time/mod.rs"]
mod cpu_time;

/// adrs.md decision 9 (criterion 13): importing one remote edit into a document of 1,000 objects
/// at the start of a frame costs at most 2 ms of thread CPU time, asserted at four times (8 ms)
/// in an optimised build; an idle frame start of a shared document stays far below a frame.
/// Debug builds run it as a smoke test. Run: `cargo test --release -p curvyo-editor-wasm --lib
/// frame_start_budget -- --nocapture`.
#[cfg(unix)]
#[test]
fn frame_start_budget_for_one_remote_edit_in_1000_objects() {
    use curvyo_document_core::{Length, RectBounds};
    let mut net = Net::new();
    let a = Session::new(0xa1);
    for n in 0..1_000u32 {
        let x = f64::from(n % 40) * 6.0;
        let y = f64::from(n / 40) * 6.0;
        let _ = a.document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(4.0),
            height: Length::from_mm(4.0),
        });
    }
    let a = net.add(a);
    let now = net.now;
    net.s(a)
        .share_document(SERVER, "Anna", ROOM, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    let b = net.add(Session::new(0xb2));
    net.s(b)
        .join_document(&link, "Ben", &seeds(2, 0), now)
        .unwrap();
    net.run(2_000);
    assert_eq!(net.json(a), net.json(b));
    let mut worst = std::time::Duration::ZERO;
    let mut idle = std::time::Duration::ZERO;
    for n in 0..5 {
        draw_rect(net.s(a), (300.0 + f64::from(n), 0.0), (305.0, 5.0));
        let now = net.now.plus(STEP_MS);
        net.now = now;
        net.s(a).sync_tick(now);
        net.pump();
        let (changed, took) = cpu_time::cpu_time_of(|| net.s(b).sync_frame_start());
        assert!(changed);
        worst = worst.max(took);
        let (_, took) = cpu_time::cpu_time_of(|| net.s(b).sync_frame_start());
        idle = idle.max(took);
    }
    eprintln!("frame start: one remote edit {worst:?}, idle {idle:?}");
    if !cfg!(debug_assertions) {
        assert!(worst <= std::time::Duration::from_millis(8), "{worst:?}");
        assert!(idle <= std::time::Duration::from_millis(1), "{idle:?}");
    }
}

/// Criterion 27: a copy of A's shared file reopened on a device the keyring does not list is
/// refused by the relay; the session ends, says so once and does not reconnect.
#[test]
fn a_copy_on_a_device_that_is_no_participant_is_refused_and_stops() {
    let (mut net, a, _) = shared_pair();
    let bytes = net.s(a).pack_for_save().unwrap();
    let mut copy = Session::open(0xc9, &bytes).unwrap();
    let now = net.now;
    assert!(copy.resume_shared("Eve", Vec::new(), &seeds(9, 1), now));
    let c = net.add(copy);
    net.pump();
    net.run(30_000);
    let device = &mut net.devices[c];
    assert_eq!(device.connects, 1, "no reconnect after the refusal");
    assert_eq!(view(&device.session)["ended"], "refused");
    assert_eq!(
        device.session.take_sync_notice(),
        Some((
            curvyo_ui_core::share_text::NOT_ADMITTED_NOTICE.to_owned(),
            true
        ))
    );
    assert_eq!(device.session.take_sync_notice(), None);
}
