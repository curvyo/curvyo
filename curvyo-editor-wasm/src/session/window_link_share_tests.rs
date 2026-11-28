//! The pen window's link with pictures, pressure strokes and relay sync (`specs/0158-pen-display`
//! slice 2a criteria 3, 7, 17, 23 to 26, ADR 0096 A5, A7, A12): two sessions and a byte shuttle,
//! and for sharing the in-process relay of the sharing tests.
#![allow(clippy::assert_is_empty)]

use curvyo_document_core::{ObjectSnapshot, PathSnapshot, Point};
use curvyo_ui_core::window_bridge::{Bundle, Control, Updates, encode};

use super::align_test_support::draw_rect;
use super::sync_test_support::shared_pair;
use super::window_link::LinkState;
use super::window_link_tests::Pair;
use super::{Session, Tool};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn import_png(session: &mut Session) {
    session.import_begin(None);
    session.import_add(PNG, "/home/maker/photo.png");
    let outcome = session.import_finish(false);
    assert_eq!(outcome.imported, 1);
}

fn images(session: &Session) -> usize {
    session
        .objects()
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Image(_)))
        .count()
}

/// One Pencil stroke of 60 samples along x from `from` with pressure rising 0.1 to 0.9, then a
/// hover sample (ADR 0095 flat records in canvas CSS px).
fn pressure_stroke(session: &mut Session, from: Point) {
    session.set_tool(Tool::Pencil);
    let origin = session.screen_to_document(0.0, 0.0);
    let unit = session.screen_to_document(1.0, 0.0);
    let px_per_mm = 1.0 / (unit.x - origin.x);
    let record = |at: Point, pressure: f64, phase: f64, time: f64| {
        vec![
            (at.x - origin.x) * px_per_mm,
            (at.y - origin.y) * px_per_mm,
            pressure,
            f64::NAN,
            f64::NAN,
            0.0,
            phase,
            0.0,
            time,
        ]
    };
    let mut flat = Vec::new();
    for i in 0..60 {
        let t = f64::from(i) / 59.0;
        let at = Point::new(from.x + 80.0 * t, from.y + 4.0 * (t * 3.0).sin());
        flat.extend(record(at, 0.1 + 0.8 * t, 1.0, f64::from(i) * 4.0));
    }
    flat.extend(record(Point::new(from.x + 81.0, from.y), 0.0, 0.0, 240.0));
    let _ = session.draw_list();
    session.pen_samples(&flat);
}

fn paths(session: &Session) -> Vec<PathSnapshot> {
    session
        .document
        .object_ids()
        .into_iter()
        .filter_map(|id| session.document.path(id))
        .collect()
}

/// Criterion 3: a picture imported in the main window shows in the pen window and travels once;
/// the same picture placed again travels no more.
#[test]
fn a_picture_travels_once() {
    let mut pair = Pair::open(Session::new(0xA1));
    import_png(&mut pair.main);
    pair.settle();
    assert_eq!(images(&pair.pen), 1);
    assert!(pair.equal());
    assert_eq!(pair.assets_sent, (1, 0));
    import_png(&mut pair.pen);
    pair.settle();
    assert_eq!(images(&pair.main), 2);
    assert_eq!(
        pair.assets_sent,
        (1, 0),
        "the picture both have is not sent again"
    );
}

/// Criterion 3: an update whose picture has not arrived waits, asks for it, and imports once it
/// is here; the document never shows an empty picture.
#[test]
fn an_update_waits_for_its_picture() {
    let mut pair = Pair::open(Session::new(0xA1));
    pair.drop_main_assets = true;
    import_png(&mut pair.main);
    pair.arrive();
    assert_eq!(images(&pair.pen), 0, "the update waits for its picture");
    pair.drop_main_assets = false;
    pair.settle();
    assert_eq!(images(&pair.pen), 1);
    assert!(pair.equal());
}

/// Criteria 7 and 24: a pressure stroke drawn in the pen window is in the main window a frame
/// later with the same centre line and width profile, both ways, and after a reload.
#[test]
fn a_pressure_stroke_crosses_with_its_profile_both_ways() {
    let mut pair = Pair::open(Session::new(0xA1));
    pressure_stroke(&mut pair.pen, Point::new(20.0, 50.0));
    assert_eq!(
        pair.pen
            .history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Pencil stroke")
    );
    assert_eq!(
        pair.pen.selection.ids().len(),
        1,
        "selected in the pen window"
    );
    pair.arrive();
    let main = paths(&pair.main);
    assert_eq!(main.len(), 1);
    assert!(!main[0].width_points.is_empty(), "the profile came along");
    assert_eq!(main, paths(&pair.pen));
    pressure_stroke(&mut pair.main, Point::new(20.0, 90.0));
    pair.settle();
    assert_eq!(paths(&pair.pen).len(), 2);
    assert!(pair.equal());
    // After a reload from the main window's project.
    pair.pen.link_receive(b"\x01\x01garbage");
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live);
    assert_eq!(paths(&pair.pen), paths(&pair.main));
    assert!(paths(&pair.pen).iter().all(|p| !p.width_points.is_empty()));
}

/// Criteria 17 and 25: in a shared project a pen-window step reaches the collaborator through
/// the main window's relay export and is named "<label> (pen window)" there; a pressure stroke
/// is uniform at Max with the notice once, writes no marker, and the main window keeps syncing.
#[test]
fn a_pen_step_reaches_a_collaborator_and_a_pressure_stroke_stays_shareable() {
    let (mut net, a, b) = shared_pair();
    let main = std::mem::replace(net.s(a), Session::new(1));
    let mut pair = Pair::open(main);
    assert!(
        pair.pen.extended_edits_blocked(),
        "the pen knows main is shared"
    );
    draw_rect(&mut pair.pen, (50.0, 50.0), (60.0, 60.0));
    pair.arrive();
    *net.s(a) = std::mem::replace(&mut pair.main, Session::new(1));
    net.run(300);
    assert_eq!(net.devices[b].session.objects().len(), 2);
    let rows: serde_json::Value = serde_json::from_str(&net.s(b).history_rows_json(0, 5)).unwrap();
    assert_eq!(rows[0]["peerLabel"], "Anna (pen window)", "{rows}");
    pair.main = std::mem::replace(net.s(a), Session::new(1));
    pressure_stroke(&mut pair.pen, Point::new(20.0, 120.0));
    let stroke = paths(&pair.pen).pop().unwrap();
    assert!(
        stroke.width_points.is_empty(),
        "uniform in a shared project"
    );
    assert_eq!(pair.pen.selection.ids().len(), 1, "and selected");
    assert_eq!(
        pair.pen.take_unsupported_width_notice(),
        super::pencil::SHARED_PRESSURE_NOTICE
    );
    pressure_stroke(&mut pair.pen, Point::new(20.0, 150.0));
    assert_eq!(
        pair.pen.take_unsupported_width_notice(),
        "",
        "once per session"
    );
    pair.arrive();
    assert!(!pair.main.document.has_extended_path_history());
    *net.s(a) = std::mem::replace(&mut pair.main, Session::new(1));
    net.run(300);
    assert_eq!(
        net.devices[b].session.objects().len(),
        4,
        "the main window keeps syncing"
    );
    assert_eq!(net.json(a), net.json(b));
}

/// Criterion 23: in the main window alone a shared pressure stroke is uniform at Max, selected,
/// one step "Pencil stroke", with the notice; unshared it keeps its profile; without pressure
/// nothing shows.
#[test]
fn a_shared_pressure_stroke_is_uniform_selected_and_said_once() {
    let mut alone = Session::new(5);
    pressure_stroke(&mut alone, Point::new(20.0, 50.0));
    assert!(!paths(&alone)[0].width_points.is_empty());
    assert_eq!(alone.take_unsupported_width_notice(), "");
    let (mut net, a, _) = shared_pair();
    pressure_stroke(net.s(a), Point::new(20.0, 80.0));
    let stroke = paths(net.s(a)).pop().unwrap();
    assert!(stroke.width_points.is_empty());
    assert!(
        (stroke.style.stroke.width.as_mm() - 2.0).abs() < 1e-9,
        "Max"
    );
    assert_eq!(net.s(a).selection.ids().len(), 1);
    assert_eq!(
        net.s(a)
            .history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Pencil stroke")
    );
    assert_eq!(
        net.s(a).take_unsupported_width_notice(),
        super::pencil::SHARED_PRESSURE_NOTICE
    );
}

/// Criterion 26: a pen update that carries a width profile while the main window is shared is
/// refused; the main document stays as it was and keeps syncing, the pen window reloads.
#[test]
fn a_profile_from_a_pen_that_ignored_the_shared_state_is_refused() {
    let (mut net, a, b) = shared_pair();
    let main = std::mem::replace(net.s(a), Session::new(1));
    let mut pair = Pair::open(main);
    let before = pair.main.document.export_json().unwrap();
    // A pen replica that ignored `Shared`: an unshared copy draws a profiled stroke.
    pair.pen.document.set_live_share(false);
    pair.pen.document.set_workspace_share(None);
    assert!(!pair.pen.extended_edits_blocked());
    pressure_stroke(&mut pair.pen, Point::new(20.0, 50.0));
    assert!(pair.pen.document.has_extended_path_history());
    pair.arrive();
    assert_eq!(pair.main.document.export_json().unwrap(), before);
    pair.settle();
    assert_eq!(pair.pen.link_state(), LinkState::Live, "reloaded");
    assert!(pair.equal());
    assert!(!pair.main.document.has_extended_path_history());
    *net.s(a) = std::mem::replace(&mut pair.main, Session::new(1));
    draw_rect(net.s(a), (70.0, 70.0), (80.0, 80.0));
    net.run(300);
    assert_eq!(net.json(a), net.json(b), "the main window keeps syncing");
}

/// A main session answers a stale bundle of the replaced replica with nothing but its requests.
#[test]
fn a_bundle_of_an_old_generation_is_dropped() {
    let mut pair = Pair::open(Session::new(0xA1));
    let stale = Bundle {
        generation: 0,
        seq: 0,
        updates: Some(Updates {
            bytes: vec![1, 2, 3],
            needs: None,
        }),
        controls: vec![Control::Save],
        ..Bundle::default()
    };
    pair.main.link_receive(&encode(&stale));
    pair.arrive();
    assert!(pair.main.link_take_save());
    assert_eq!(
        pair.pen.link_state(),
        LinkState::Live,
        "no reload for a stale bundle"
    );
    assert_eq!(pair.main.link_state(), LinkState::Live);
}
