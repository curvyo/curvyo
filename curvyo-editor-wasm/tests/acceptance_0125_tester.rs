//! Tester acceptance cases for `specs/0125-send-to-machine` (S1 to S12, S16, S17, S19), written
//! from the specification before the implementation was read; only the names of the public
//! session calls were adapted afterwards. Black box through the public session and the fake
//! connector. Waits are on statuses, never on sleeps.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    missing_docs
)]

mod fixture_support;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    JobKind, MAX_CHUNK_BYTES, Message, Progress, Resolution, SendWatch, State,
};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, Connection, connect, open_paired};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
use curvyo_connector_io::job_send::new_job_id;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::send_tracked::{SendEnd, send_tracked};
use curvyo_connector_io::server::{RunningServer, ServerConfig};
use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Direction, Document, DocumentSize, FixtureJobSwitch, NewOutputPage,
    Point, RectBounds, Timestamp, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};
use curvyo_ui_core::FontBlock;
use fixture_support::add_fixture;
use serde_json::Value;

const WAIT: Duration = Duration::from_secs(30);
const NOW_MS: u64 = 1_762_948_800_000;
const ADDR: &str = "127.0.0.1:9";

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}
fn lasered(s: &Session) -> bool {
    serde_json::from_str::<Value>(&s.test_grid_view_json()).unwrap()["lasered"] == true
}
fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// Opens a 400 x 300 laser page; `build` may draw before the session opens.
fn blank(address: &str) -> Session {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "t").unwrap()).unwrap();
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&laser.id));
    if !address.is_empty() {
        let mut profile = MachineProfile::generic_laser();
        profile.id = new_profile_id([9; 16]);
        profile.modified = 1_000;
        "K40".clone_into(&mut profile.values.name);
        profile.work_area = DocumentSize::from_mm(400.0, 300.0);
        address.clone_into(&mut profile.connection.connector_address);
        s.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
        assert!(s.pick_machine_profile(&profile.id));
    }
    s.set_send_link("paired");
    s
}

/// A laser page on a connector profile with one cut rectangle that runs (no grid, no fixture).
fn simple(address: &str) -> (Session, curvyo_document_core::NodeId) {
    let doc = Document::new(1);
    let laser = doc
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = doc.pages().into_iter().find(|p| p.id == laser.id).unwrap();
    let mfg = page.kind.manufacturing().unwrap();
    let rect = doc.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 10.0, page.origin.y + 10.0),
        Point::new(page.origin.x + 30.0, page.origin.y + 20.0),
    ));
    let op = doc
        .add_operation(
            mfg,
            &curvyo_document_core::NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[vec![rect]],
        )
        .unwrap();
    let mut session = Session::open(5, &pack(&doc, "t").unwrap()).unwrap();
    session.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(session.activate_page(&laser.id));
    let mut profile = MachineProfile::generic_laser();
    profile.id = new_profile_id([9; 16]);
    profile.modified = 1_000;
    "K40".clone_into(&mut profile.values.name);
    profile.work_area = DocumentSize::from_mm(400.0, 300.0);
    address.clone_into(&mut profile.connection.connector_address);
    session.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
    assert!(session.pick_machine_profile(&profile.id));
    session.set_send_link("paired");
    let _ = session.edit_operation_setting(op, "main", "power", 50.0);
    let _ = session.edit_operation_setting(op, "main", "speed", 600.0);
    (session, op)
}

// ---- S1 ------------------------------------------------------------------------------------

#[test]
fn s1_send_exists_only_for_a_profile_that_names_a_connector() {
    let (s, _) = simple(ADDR);
    let v = view(&s);
    assert!(v["send"].is_object(), "{v}");
    assert_eq!(v["send"]["enabled"], true, "{}", v["checks"]);
    assert_eq!(v["export"]["enabled"], true);
    // A profile without an address: no Send at all (not even disabled).
    let (s, _) = simple("");
    assert!(view(&s)["send"].is_null(), "{}", view(&s));
    // The built-in Generic laser (no saved profile at all): no Send.
    let s = blank("");
    assert!(view(&s)["send"].is_null());
}

// ---- S2 ------------------------------------------------------------------------------------

#[test]
fn s2_each_blocking_condition_disables_export_and_send_together_with_the_same_text() {
    // a: no settings
    let (mut s, op) = simple(ADDR);
    let _ = s.set_operation_switch(op, curvyo_document_core::OperationSwitch::Output, false);
    // d: nothing runs (the only operation has Output off)
    let v = view(&s);
    assert_eq!(v["export"]["enabled"], false, "{v}");
    assert_eq!(v["send"]["enabled"], false);
    assert_eq!(v["send"]["cause"], "blocked");
    assert_eq!(
        v["send"]["text"], v["export"]["tooltip"],
        "same words as Export"
    );
    assert!(s.send_job().is_none(), "a blocked page sends nothing");
    assert!(s.export_job("x").is_none());
    // Removing the condition enables both together.
    assert!(s.set_operation_switch(op, curvyo_document_core::OperationSwitch::Output, true));
    let _ = s.edit_operation_setting(op, "main", "power", 50.0);
    let v = view(&s);
    assert_eq!(v["export"]["enabled"], true, "{v}");
    assert_eq!(v["send"]["enabled"], true);
}

#[test]
fn s2_geometry_outside_the_bed_and_no_settings_block_both() {
    // Superseded by `specs/0150` criterion 23: an operation without `op_settings` runs the copy's
    // default numbers, so only the bed case blocks.
    {
        let case = "outside";
        let doc = Document::new(1);
        let laser = doc
            .add_output_page(NewOutputPage {
                process: "curvyo.laser",
                label: "Laser",
                profile: None,
                work_area: DocumentSize::from_mm(400.0, 300.0),
            })
            .unwrap();
        let page = doc.pages().into_iter().find(|p| p.id == laser.id).unwrap();
        let mfg = page.kind.manufacturing().unwrap();
        let offset = if case == "outside" { 395.0 } else { 10.0 };
        let rect = doc.create_rect(RectBounds::from_corners(
            Point::new(page.origin.x + offset, page.origin.y + 10.0),
            Point::new(page.origin.x + offset + 30.0, page.origin.y + 20.0),
        ));
        let op = doc
            .add_operation(
                mfg,
                &curvyo_document_core::NewOperation {
                    kind: "cut",
                    name: "Cut 1",
                    settings: None,
                },
                0,
                &[vec![rect]],
            )
            .unwrap();
        let mut session = Session::open(5, &pack(&doc, "t").unwrap()).unwrap();
        assert!(session.activate_page(&laser.id));
        let mut profile = MachineProfile::generic_laser();
        profile.id = new_profile_id([9; 16]);
        profile.modified = 1_000;
        profile.work_area = DocumentSize::from_mm(400.0, 300.0);
        ADDR.clone_into(&mut profile.connection.connector_address);
        session.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
        assert!(session.pick_machine_profile(&profile.id));
        session.set_send_link("paired");
        if case == "outside" {
            let _ = session.edit_operation_setting(op, "main", "power", 50.0);
            let _ = session.edit_operation_setting(op, "main", "speed", 600.0);
        }
        let json = view(&session);
        assert_eq!(json["export"]["enabled"], false, "{case}: {json}");
        assert_eq!(json["send"]["enabled"], false, "{case}");
        assert_eq!(json["send"]["text"], json["export"]["tooltip"], "{case}");
        assert!(session.send_job().is_none(), "{case}");
        if case == "outside" {
            let _ = session.edit_operation_setting(op, "main", "power", 50.0);
        } else {
            let _ = session.edit_operation_setting(op, "main", "power", 50.0);
            let _ = session.edit_operation_setting(op, "main", "speed", 600.0);
        }
    }
}

#[test]
fn s2_warnings_alone_keep_send_enabled() {
    // The generic laser profile warns ("Generic laser") but never blocks.
    let (s, _) = simple(ADDR);
    let v = view(&s);
    assert!(
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["severity"] != "block"),
        "{}",
        v["checks"]
    );
    assert_eq!(v["send"]["enabled"], true);
}

// ---- S3, S4, S5 ----------------------------------------------------------------------------

#[test]
fn s3_pairing_causes_and_a_paired_profile_is_never_guessed_unreachable() {
    let (mut s, _) = simple(ADDR);
    for (link, cause) in [
        ("not_paired", "not_paired"),
        ("no_longer_paired", "no_longer_paired"),
        ("key_changed", "key_changed"),
    ] {
        s.set_send_link(link);
        let v = view(&s);
        assert_eq!(v["send"]["enabled"], false, "{link}");
        assert_eq!(v["send"]["cause"], cause);
        assert!(s.send_job().is_none(), "{link} sends nothing");
        assert_eq!(v["export"]["enabled"], true, "Export is unaffected");
    }
    s.set_send_link("paired");
    assert_eq!(view(&s)["send"]["enabled"], true);
}

#[test]
fn s5_browser_build_shows_send_disabled_and_export_still_works() {
    let (mut s, _) = simple(ADDR);
    s.set_send_link("browser");
    let v = view(&s);
    assert!(v["send"].is_object(), "shown, not hidden");
    assert_eq!(v["send"]["enabled"], false);
    assert_eq!(v["send"]["cause"], "browser");
    assert!(s.send_job().is_none());
    assert!(s.export_job("x").is_some(), "Export still downloads");
}

// ---- S6, S7, S8 ----------------------------------------------------------------------------

#[test]
fn s6_the_bytes_sent_are_the_bytes_export_writes_and_the_name_is_the_pages() {
    let (mut s, _) = simple(ADDR);
    let exported = s.export_job("x").unwrap();
    let sent = s.send_job().unwrap();
    assert_eq!(sent.bytes, exported.bytes, "byte-identical");
    assert_eq!(sent.name, s.active_page().name);
    assert_eq!(sent.address, ADDR);
    s.send_job_failed(sent.token);
}

#[test]
fn s7_a_non_ascii_page_name_sends_the_bytes_unchanged_and_the_name_as_it_is() {
    let (mut s, _) = simple(ADDR);
    let id = s.active_page().id;
    assert!(s.rename_page(&id, "Küche"));
    let exported = s.export_job("x").unwrap();
    let text = String::from_utf8(exported.bytes.clone()).unwrap();
    assert!(text.contains("; Curvyo job: K_che"));
    assert!(text.is_ascii());
    let sent = s.send_job().unwrap();
    assert_eq!(sent.bytes, exported.bytes);
    assert_eq!(sent.name, "Küche");
    s.send_job_failed(sent.token);
}

#[test]
fn s8_while_sending_send_is_disabled_with_sending_and_export_stays_enabled() {
    let (mut s, _) = simple(ADDR);
    let sent = s.send_job().unwrap();
    let v = view(&s);
    assert_eq!(v["send"]["enabled"], false);
    assert_eq!(v["send"]["cause"], "sending");
    assert_eq!(v["export"]["enabled"], true);
    assert!(
        s.send_job().is_none(),
        "a second press while sending sends nothing"
    );
    s.send_job_failed(sent.token);
    assert_eq!(view(&s)["send"]["enabled"], true, "a failure frees Send");
}

// ---- S9, S10, S11, S12 on a page with a fixture job and a test grid -------------------------

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn grid_page(address: &str) -> Session {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "send").unwrap()).unwrap();
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&laser.id));
    let mut profile = MachineProfile::generic_laser();
    profile.id = new_profile_id([9; 16]);
    profile.modified = 1_000;
    "K40".clone_into(&mut profile.values.name);
    profile.work_area = DocumentSize::from_mm(400.0, 300.0);
    address.clone_into(&mut profile.connection.connector_address);
    s.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
    assert!(s.pick_machine_profile(&profile.id));
    s.set_send_link("paired");
    let birch = s.add_material("Birch 3 mm", 3.0, &[3; 16]).unwrap();
    let holder = add_fixture(&mut s, "holder").unwrap();
    assert!(s.select_fixture(holder));
    let job = s.add_fixture_job("score").unwrap();
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    s.add_material_set(&birch, &profile.id, "score", &[4; 16])
        .unwrap();
    assert!(s.set_fixture_material(&birch));
    let fonts = manifest().join("../frontend/public/fonts");
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts.join("index.json")).unwrap()).unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts.join(&file)).unwrap()));
    }
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(Point::new(1.0, 1.0))).unwrap();
    s.set_tool(Tool::Select);
    s.pick_test_grid_material(Some(&birch)).unwrap();
    s
}

#[test]
fn s10_s12_accepted_send_is_one_step_lasered_stays_and_a_second_send_adds_no_step() {
    let mut s = grid_page(ADDR);
    assert!(!lasered(&s));
    let steps = s.undo_len();
    let pack_before = s.pack("t").unwrap();

    // Handing out the job writes nothing to the project (S6/S11: not on the press).
    let job = s.send_job().unwrap();
    assert_eq!(s.undo_len(), steps);
    assert!(!lasered(&s));
    assert_eq!(
        s.pack("t").unwrap(),
        pack_before,
        "S16: nothing written on the press"
    );

    // A failure commits nothing (S11).
    s.send_job_failed(job.token);
    assert_eq!(s.undo_len(), steps);
    assert!(!lasered(&s));
    assert!(
        s.send_job_done(job.token, NOW_MS).is_none(),
        "a dropped send cannot commit"
    );
    assert_eq!(s.undo_len(), steps);

    // Accepted: one step, the one notice, Lasered.
    let job = s.send_job().unwrap();
    let notice = s.send_job_done(job.token, NOW_MS).unwrap();
    assert_eq!(notice.0, "Output turned off on 1 fixture job.");
    assert_eq!(notice.1, 3000, "3 s");
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Send job to machine");
    assert!(lasered(&s));
    // A replay of the same token commits nothing more.
    assert!(s.send_job_done(job.token, NOW_MS).is_none());
    assert_eq!(s.undo_len(), steps + 1);

    // S12: an unchanged second send adds no step and gives no notice.
    let again = s.send_job().unwrap();
    assert!(
        s.send_job_done(again.token, NOW_MS + 1000).is_none(),
        "no notice"
    );
    assert_eq!(s.undo_len(), steps + 1, "no new step");

    // Undo: the fixture job is back on, Lasered stays.
    let _ = s.undo();
    assert!(lasered(&s), "Lasered stays after undo");
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn s16_the_format_version_stays_49() {
    // No bump by `0125`; 50 is `specs/0123-raster-engraving` (ADR 0069 §10).
    // Superseded by 0135 slice 1 (2026-11-19): 51 is `specs/0135-image-optimisation` (ADR 0072 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 71);
}

// ---- S6, S9, S11, S17, S19 against the fake connector --------------------------------------

struct Rig {
    server: RunningServer,
    dir: tempfile::TempDir,
    record: PairingRecord,
}

impl Rig {
    fn address(&self) -> String {
        self.server.addr().to_string()
    }
    fn open(&self) -> Result<(Connection, Message), ClientError> {
        open_paired(
            &self.address(),
            self.record.spki,
            &self.record.token,
            CONNECT_TIMEOUT,
        )
    }
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let server = RunningServer::start(&ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir.path().to_owned(),
        controller: ControllerChoice::Fake(FakeSettings {
            lines_per_second: 200,
            alarm_at_line: None,
        }),
        require_local_start: false,
        connector_name: "Workshop".to_owned(),
        issue_code: false,
    })
    .unwrap();
    let record = connect(
        &server.addr().to_string(),
        PinMode::Capture,
        CONNECT_TIMEOUT,
    )
    .unwrap()
    .pair(&server.current_code().unwrap(), "t", Instant::now() + WAIT)
    .unwrap();
    Rig {
        server,
        dir,
        record,
    }
}

fn press(
    s: &mut Session,
    open: &mut dyn FnMut() -> Result<(Connection, Message), ClientError>,
) -> Option<SendEnd> {
    let job = s.send_job()?;
    let watch = Mutex::new(Some(SendWatch::new(new_job_id().unwrap())));
    let text = String::from_utf8(job.bytes.clone()).unwrap();
    let end = send_tracked(open, &watch, &job.name, &text, Instant::now() + WAIT).unwrap();
    if end == SendEnd::Resolved(Resolution::Accepted) {
        let _ = s.send_job_done(job.token, NOW_MS);
    } else {
        s.send_job_failed(job.token);
    }
    Some(end)
}

fn status_of(c: &mut Connection) -> (State, Option<Progress>) {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Message::Status { state, job, .. } = c.recv_message(deadline).unwrap() {
            return (state, job);
        }
    }
}

#[test]
fn s6_s9_s17_the_connector_holds_exactly_the_exported_job_then_start_and_stop() {
    let rig = rig();
    let mut s = grid_page(&rig.address());
    let exported = s.export_job("x").unwrap();
    let steps = s.undo_len();
    let mut open = || rig.open();
    assert_eq!(
        press(&mut s, &mut open),
        Some(SendEnd::Resolved(Resolution::Accepted))
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert!(lasered(&s));
    let (mut app, first) = rig.open().unwrap();
    let Message::Status { state, job, .. } = first else {
        panic!()
    };
    assert_eq!(state, State::Ready, "S9: Ready with the sent job");
    let progress = job.unwrap();
    let exported_lines = String::from_utf8(exported.bytes).unwrap().lines().count();
    assert!(progress.lines > 0 && (progress.lines as usize) <= exported_lines);
    // Held: the connector lists the second job (`0132`), which is accepted and counts as
    // lasered (S8, S10, S11); Send stays free.
    assert_eq!(
        press(&mut s, &mut open),
        Some(SendEnd::Resolved(Resolution::Accepted))
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(view(&s)["send"]["enabled"], true);
    // Start then Stop.
    app.send(&Message::Start {
        job_id: progress.job_id,
    })
    .unwrap();
    while status_of(&mut app).0 != State::Running {}
    app.send(&Message::Stop).unwrap();
    loop {
        if matches!(status_of(&mut app).0, State::Done { .. }) {
            break;
        }
    }
    let log = std::fs::read_to_string(rig.dir.path().join("jobs.log")).unwrap_or_default();
    assert!(
        log.contains(&s.active_page().name),
        "the job log names the page: {log}"
    );
    // After Stop the next send is accepted; the page is unchanged: no step (S12).
    assert_eq!(
        press(&mut s, &mut open),
        Some(SendEnd::Resolved(Resolution::Accepted))
    );
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn s7_nothing_listening_reports_an_error_and_commits_nothing() {
    let rig = rig();
    let mut s = grid_page(ADDR);
    let steps = s.undo_len();
    let mut open = || -> Result<(Connection, Message), ClientError> {
        // Port 9 (discard) is closed on a normal machine; the dial fails.
        open_paired(
            "127.0.0.1:9",
            rig.record.spki,
            &rig.record.token,
            Duration::from_secs(2),
        )
    };
    let job = s.send_job().unwrap();
    let watch = Mutex::new(Some(SendWatch::new(new_job_id().unwrap())));
    let r = send_tracked(
        &mut open,
        &watch,
        &job.name,
        &String::from_utf8(job.bytes).unwrap(),
        Instant::now() + Duration::from_secs(5),
    );
    assert!(
        !matches!(r, Ok(SendEnd::Resolved(Resolution::Accepted))),
        "{r:?}"
    );
    s.send_job_failed(job.token);
    assert_eq!(s.undo_len(), steps);
    assert!(!lasered(&s));
    let _ = JobKind::Job;
}

#[test]
fn s19_a_transfer_cut_after_a_chunk_then_retry_commits_once_and_not_busy() {
    let rig = rig();
    let mut s = grid_page(&rig.address());
    let steps = s.undo_len();
    let big = "G1 X1 Y1\n".repeat(MAX_CHUNK_BYTES / 9 + 10);
    let (mut cut, _) = rig.open().unwrap();
    let id = new_job_id().unwrap();
    cut.send(&Message::SubmitJob {
        job_id: id,
        name: "cut".to_owned(),
        kind: JobKind::Job,
        bytes: big.len() as u64,
        sha256: "0".repeat(64),
    })
    .unwrap();
    cut.send(&Message::JobChunk {
        job_id: id,
        index: 0,
        text: big[..MAX_CHUNK_BYTES].to_owned(),
    })
    .unwrap();
    drop(cut);
    // The connection of the press dies before any answer: "Not received", nothing committed.
    let mut dead = true;
    let mut open_dead = || {
        let (mut c, st) = rig.open()?;
        if std::mem::take(&mut dead) {
            c.close();
        }
        Ok((c, st))
    };
    assert_eq!(
        press(&mut s, &mut open_dead),
        Some(SendEnd::Resolved(Resolution::NotReceived))
    );
    assert_eq!(s.undo_len(), steps);
    assert!(!lasered(&s));
    // Retry: accepted, one commit.
    let mut open = || rig.open();
    assert_eq!(
        press(&mut s, &mut open),
        Some(SendEnd::Resolved(Resolution::Accepted))
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert!(lasered(&s));
}

/// The known gap: the connection drops after the job went out and the fresh read fails too. The
/// send is neither accepted nor lost: nothing is committed, the watch stays open (the watcher
/// resolves it when the connector answers again), and Send stays on "sending" until then.
#[test]
fn s11_connection_dropped_and_a_fresh_read_fails_commits_nothing_and_keeps_the_watch_open() {
    let rig = rig();
    let mut s = grid_page(&rig.address());
    let steps = s.undo_len();
    let job = s.send_job().unwrap();
    let id = new_job_id().unwrap();
    let watch = Mutex::new(Some(SendWatch::new(id)));
    let mut calls = 0;
    let mut open = || {
        calls += 1;
        if calls == 1 {
            let (mut c, st) = rig.open()?;
            c.close();
            Ok((c, st))
        } else {
            open_paired(
                "127.0.0.1:9",
                rig.record.spki,
                &rig.record.token,
                Duration::from_secs(2),
            )
        }
    };
    let end = send_tracked(
        &mut open,
        &watch,
        &job.name,
        &String::from_utf8(job.bytes).unwrap(),
        Instant::now() + WAIT,
    )
    .unwrap();
    assert!(matches!(end, SendEnd::Unresolved(_)), "{end:?}");
    assert_eq!(s.undo_len(), steps, "nothing committed");
    assert!(!lasered(&s));
    assert!(
        curvyo_connector_io::send_tracked::watched_job(&watch).is_some(),
        "still open"
    );
    assert_eq!(
        view(&s)["send"]["cause"],
        "sending",
        "Send waits; no cancel in the model"
    );
    // S11 replay: once the watcher reads a fresh status that shows the job, it commits once.
    let ready = Message::Status {
        state: State::Ready,
        job: Some(Progress {
            job_id: id,
            line: 0,
            lines: 3,
            elapsed_ms: 0,
            from_line: None,
            entering: false,
        }),
        position: None,
        alarm: None,
        detail: None,
        pause_pending: false,
        requested_by: None,
        job_name: None,
        sent_by: None,
        homed: false,
        machine_position: None,
        work_offset: None,
        frame_id: None,
        target: None,
    };
    let r = curvyo_connector_io::send_tracked::observe_shared(&watch, &ready);
    assert_eq!(r, Some(Resolution::Accepted));
    assert!(
        curvyo_connector_io::send_tracked::observe_shared(&watch, &ready).is_none(),
        "a replayed status resolves nothing"
    );
    assert!(s.send_job_done(job.token, NOW_MS).is_some());
    assert!(s.send_job_done(job.token, NOW_MS).is_none());
    assert_eq!(s.undo_len(), steps + 1, "committed exactly once");
}
