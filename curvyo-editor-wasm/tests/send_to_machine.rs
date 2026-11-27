//! Send to machine from a laser page to the connector with the fake controller, without the UI
//! (`specs/0125-send-to-machine` S6 to S12, S16, S17, S19): pair, send, Start, Pause, Resume,
//! Stop, a second send, a refused send, a transfer cut mid-way and a connection that died before
//! the answer. Waits are on statuses, never on sleeps.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod fixture_support;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    JobKind, JobResult, MAX_CHUNK_BYTES, Message, Resolution, SendWatch, State,
};
use curvyo_connector_io::Uuid;
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, Connection, connect, open_paired};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
use curvyo_connector_io::job_send::new_job_id;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::send_tracked::{SendEnd, send_tracked};
use curvyo_connector_io::server::{RunningServer, ServerConfig};
use curvyo_document_core::{Direction, Document, DocumentSize, NewOutputPage, Timestamp, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};
use curvyo_ui_core::FontBlock;
use fixture_support::add_fixture;
use serde_json::Value;

const WAIT: Duration = Duration::from_secs(30);
const NOW_MS: u64 = 1_762_948_800_000;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Rig {
    server: RunningServer,
    dir: tempfile::TempDir,
    record: PairingRecord,
}

impl Rig {
    fn address(&self) -> String {
        self.server.addr().to_string()
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("jobs.log")).unwrap_or_default()
    }

    fn open(&self) -> (Connection, Message) {
        open_paired(
            &self.address(),
            self.record.spki,
            &self.record.token,
            CONNECT_TIMEOUT,
        )
        .unwrap()
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
    .pair(
        &server.current_code().unwrap(),
        "test app",
        Instant::now() + WAIT,
    )
    .unwrap();
    Rig {
        server,
        dir,
        record,
    }
}

/// A laser page of 400 x 300 mm on a profile that names `address`, with the fonts loaded, a test
/// grid on Birch 3 mm and a fixture job that runs on it.
fn page(address: &str) -> Session {
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
    assert!(s.set_fixture_job_switch(job, curvyo_document_core::FixtureJobSwitch::Output, true));
    s.add_material_set(&birch, &profile.id, "score", &[4; 16])
        .unwrap();
    assert!(s.set_fixture_material(&birch));
    // The grid goes last: placing it leaves it selected, and its view reads Lasered.
    load_fonts(&mut s);
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(curvyo_document_core::Point::new(1.0, 1.0)))
        .unwrap();
    s.set_tool(Tool::Select);
    s.pick_test_grid_material(Some(&birch)).unwrap();
    s
}

fn load_fonts(s: &mut Session) {
    let fonts = manifest().join("../frontend/public/fonts");
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts.join("index.json")).unwrap()).unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
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
    s.set_tool(Tool::Select);
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}

fn lasered(s: &Session) -> bool {
    serde_json::from_str::<Value>(&s.test_grid_view_json()).unwrap()["lasered"] == true
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// What the app does on a press (the webview and the host together): hand out the job, send it by
/// its id, then commit on `Accepted` and drop the pending send otherwise. `open` gives the
/// connections. `None` when Send is disabled.
fn press(
    s: &mut Session,
    open: &mut dyn FnMut() -> Result<(Connection, Message), ClientError>,
) -> Option<SendEnd> {
    let job = s.send_job()?;
    let watch = Mutex::new(Some(SendWatch::new(new_job_id().unwrap())));
    let text = String::from_utf8(job.bytes.clone()).unwrap();
    let end = send_tracked(open, &watch, &job.name, &text, Instant::now() + WAIT);
    match &end {
        Ok(SendEnd::Resolved(Resolution::Accepted)) => {
            s.send_job_done(job.token, NOW_MS);
        }
        _ => s.send_job_failed(job.token),
    }
    Some(end.unwrap())
}

fn normal(rig: &Rig) -> impl FnMut() -> Result<(Connection, Message), ClientError> + '_ {
    || {
        open_paired(
            &rig.address(),
            rig.record.spki,
            &rig.record.token,
            CONNECT_TIMEOUT,
        )
    }
}

fn next_status(connection: &mut Connection) -> Message {
    let deadline = Instant::now() + WAIT;
    loop {
        let message = connection.recv_message(deadline).unwrap();
        if matches!(message, Message::Status { .. }) {
            return message;
        }
    }
}

fn wait_state(connection: &mut Connection, state: State) -> Message {
    loop {
        let status = next_status(connection);
        if matches!(&status, Message::Status { state: s, .. } if *s == state) {
            return status;
        }
    }
}

fn state_of(status: &Message) -> State {
    match status {
        Message::Status { state, .. } => *state,
        other => panic!("{other:?}"),
    }
}

fn job_id_of(status: &Message) -> Uuid {
    match status {
        Message::Status { job: Some(p), .. } => p.job_id,
        other => panic!("{other:?}"),
    }
}

#[test]
fn s17_the_whole_path_from_pairing_to_a_second_send() {
    let rig = rig();
    let mut s = page(&rig.address());
    assert_eq!(
        view(&s)["send"]["enabled"],
        true,
        "{:#}",
        view(&s)["checks"]
    );
    let exported = s.export_job("").unwrap();
    let steps = s.undo_len();

    // S6, S9, S10: send; the connector holds the job; one step; Lasered; the fixture job is off.
    let end = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::Accepted));
    assert_eq!(s.undo_len(), steps + 1, "S10: one undo step");
    assert_eq!(undo_label(&mut s), "Send job to machine");
    assert!(lasered(&s));
    let (mut app, ready) = rig.open();
    assert_eq!(state_of(&ready), State::Ready);
    let lines = match &ready {
        Message::Status { job: Some(p), .. } => p.lines,
        other => panic!("{other:?}"),
    };
    let text_lines = u32::try_from(
        String::from_utf8(exported.bytes.clone())
            .unwrap()
            .lines()
            .count(),
    )
    .unwrap();
    assert!(
        lines > 0 && lines <= text_lines,
        "{lines} of {text_lines} lines"
    );

    // S12 (amended by `0132`): a second send while the job is held is listed behind it.
    let held = job_id_of(&ready);
    let listed = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(listed, SendEnd::Resolved(Resolution::Accepted));
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "S12: an unchanged page adds no step"
    );
    assert_eq!(
        view(&s)["send"]["enabled"],
        true,
        "the pending send was dropped"
    );

    // S9: Start, Pause, Resume, Stop on the held job.
    app.send(&Message::Start { job_id: held }).unwrap();
    wait_state(&mut app, State::Running);
    app.send(&Message::Pause).unwrap();
    wait_state(&mut app, State::Paused);
    app.send(&Message::Resume).unwrap();
    wait_state(&mut app, State::Running);
    app.send(&Message::Stop).unwrap();
    wait_state(
        &mut app,
        State::Done {
            result: JobResult::Stopped,
        },
    );
    let log = rig.log();
    assert!(
        log.contains(&format!("\t{}\t", job_name(&s))),
        "S6: the log names the page: {log}"
    );

    // The next send is accepted; the page is unchanged, so no new step (S12).
    let again = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(again, SendEnd::Resolved(Resolution::Accepted));
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "S12: an unchanged second send adds no step"
    );

    // S10: undo takes the step back, Lasered stays.
    let _ = s.undo();
    assert!(lasered(&s), "Lasered stays after undo");
}

fn job_name(s: &Session) -> String {
    s.active_page().name
}

#[test]
fn s7_a_page_named_with_an_umlaut_sends_and_the_connector_logs_the_name() {
    let rig = rig();
    let mut s = page(&rig.address());
    let id = s.active_page().id;
    assert!(s.rename_page(&id, "Küche"));
    let exported = s.export_job("").unwrap();
    let text = String::from_utf8(exported.bytes).unwrap();
    assert!(
        text.contains("; Curvyo job: K_che"),
        "{}",
        &text[..text.len().min(200)]
    );
    assert_eq!(
        press(&mut s, &mut normal(&rig)).unwrap(),
        SendEnd::Resolved(Resolution::Accepted)
    );
    let (mut app, ready) = rig.open();
    app.send(&Message::Start {
        job_id: job_id_of(&ready),
    })
    .unwrap();
    wait_state(&mut app, State::Running);
    app.send(&Message::Stop).unwrap();
    wait_state(
        &mut app,
        State::Done {
            result: JobResult::Stopped,
        },
    );
    assert!(rig.log().contains("\tKüche\t"), "{}", rig.log());
}

#[test]
fn s16_a_listed_send_commits_and_the_format_stays() {
    let rig = rig();
    let mut s = page(&rig.address());
    let first = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(first, SendEnd::Resolved(Resolution::Accepted));
    // `0132`: a second send while the job is held is listed, accepted and commits as lasered.
    let second = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(second, SendEnd::Resolved(Resolution::Accepted));
    // Superseded by 0135 slice 1 (2026-11-19): format 51 is `0135`'s, not this story's; 54 is
    // `0141`'s and 55 `0121`'s.
    assert_eq!(
        curvyo_document_core::CURRENT_FORMAT_VERSION,
        68,
        "no format_version change (60 is `0145` slice 3's, 61 `0118` slice 2c's, 62 `0099` slice 1's, 63 `0118` slice 2d's, 64 `0099` slice 2's, 65 `0147` slice 3's, 66 `0146`, 67 `0139` slice 1's)"
    );
}

/// A connection that is closed before the send starts.
fn dead_first(rig: &Rig) -> impl FnMut() -> Result<(Connection, Message), ClientError> + '_ {
    let mut dead = true;
    move || {
        let (mut connection, status) = open_paired(
            &rig.address(),
            rig.record.spki,
            &rig.record.token,
            CONNECT_TIMEOUT,
        )?;
        if std::mem::take(&mut dead) {
            connection.close();
        }
        Ok((connection, status))
    }
}

#[test]
fn s19_a_transfer_that_was_cut_commits_nothing_and_the_retry_commits_once() {
    let rig = rig();
    let mut s = page(&rig.address());
    let steps = s.undo_len();
    // The app dies after one chunk of a bigger job: the connector frees itself.
    let big = "G1 X1 Y1\n".repeat(MAX_CHUNK_BYTES / 9 + 10);
    let (mut cut, _) = rig.open();
    let cut_id = new_job_id().unwrap();
    cut.send(&Message::SubmitJob {
        job_id: cut_id,
        name: "cut".to_owned(),
        kind: JobKind::Job,
        bytes: big.len() as u64,
        sha256: "0".repeat(64),
    })
    .unwrap();
    cut.send(&Message::JobChunk {
        job_id: cut_id,
        index: 0,
        text: big[..MAX_CHUNK_BYTES].to_owned(),
    })
    .unwrap();
    drop(cut);

    // The connection dies before the answer: nothing arrived, so nothing is committed.
    let lost = press(&mut s, &mut dead_first(&rig)).unwrap();
    assert_eq!(lost, SendEnd::Resolved(Resolution::NotReceived));
    assert_eq!(s.undo_len(), steps, "nothing committed");
    assert!(!lasered(&s));
    assert_eq!(view(&s)["send"]["enabled"], true, "Send works again");

    // The retry is accepted (not "Connector busy") and commits once.
    let retry = press(&mut s, &mut normal(&rig)).unwrap();
    assert_eq!(retry, SendEnd::Resolved(Resolution::Accepted));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(lasered(&s));
}

#[test]
fn s11_a_connection_that_died_after_the_connector_took_the_job_is_resolved_by_a_fresh_status() {
    let rig = rig();
    let mut s = page(&rig.address());
    let steps = s.undo_len();
    // The connector holds the job of this very press; the app never saw the answer. The press
    // below uses a connection that died, a fresh one shows the job.
    let job = s.send_job().unwrap();
    let id = new_job_id().unwrap();
    let (mut sender, _) = rig.open();
    sender
        .send_job_with_id(
            id,
            &job.name,
            &String::from_utf8(job.bytes.clone()).unwrap(),
            Instant::now() + WAIT,
        )
        .unwrap();
    drop(sender);
    let watch = Mutex::new(Some(SendWatch::new(id)));
    let end = send_tracked(
        &mut dead_first(&rig),
        &watch,
        &job.name,
        &String::from_utf8(job.bytes).unwrap(),
        Instant::now() + WAIT,
    )
    .unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::Accepted));
    let notice = s.send_job_done(job.token, NOW_MS).unwrap().0;
    assert_eq!(notice, "Output turned off on 1 fixture job.");
    assert_eq!(s.undo_len(), steps + 1, "committed once");
    assert!(
        s.send_job_done(job.token, NOW_MS).is_none(),
        "a replay commits nothing"
    );
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn s7_a_send_that_timed_out_enables_send_again_and_a_late_acceptance_commits_once() {
    let rig = rig();
    let mut s = page(&rig.address());
    let steps = s.undo_len();
    let job = s.send_job().unwrap();
    assert_eq!(view(&s)["send"]["cause"], "sending");
    s.send_job_timed_out(job.token);
    assert_eq!(view(&s)["send"]["enabled"], true, "Send is enabled again");
    assert_eq!(s.undo_len(), steps, "nothing is committed by the timeout");
    assert!(s.send_job_done(job.token, NOW_MS).is_some());
    assert_eq!(s.undo_len(), steps + 1, "the late acceptance commits");
    assert!(s.send_job_done(job.token, NOW_MS).is_none());
    assert_eq!(s.undo_len(), steps + 1, "once");
}

#[test]
fn s7_a_timed_out_send_that_failed_or_was_followed_by_a_new_send_never_commits() {
    let rig = rig();
    let mut s = page(&rig.address());
    let steps = s.undo_len();
    let first = s.send_job().unwrap();
    s.send_job_timed_out(first.token);
    s.send_job_failed(first.token);
    assert!(s.send_job_done(first.token, NOW_MS).is_none());
    let second = s.send_job().unwrap();
    s.send_job_timed_out(second.token);
    let third = s.send_job().unwrap();
    assert!(s.send_job_done(second.token, NOW_MS).is_none(), "replaced");
    s.send_job_failed(third.token);
    assert_eq!(s.undo_len(), steps);
}
