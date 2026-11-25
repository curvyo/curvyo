//! `specs/0118` slice 2e at the hub with the fake controller (adrs R9, R10; `specs/0132` 12,
//! 13; Frage 390 C): a stopped job stays listed as interrupted, an owner restarts it from its
//! line, the fake travels there in passed time, `jobs.log` and `admin.log` record the restart,
//! a Stop before the line returns the entry with no result line, `--require-local-start` waits
//! for Enter, and a connector that came back names the cause on the row and in the refusal (no
//! position, not homed, a changed work offset after Home, an unknown word, an arc mode). The hub is driven by hand with made-up instants: no sleep, no wall time.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use curvyo_connector_core::{JobKind, JobResult, ListEntry, Message, Peer, Role, State};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings, Hub, HubSettings};
use curvyo_connector_io::spki::SpkiHash;
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn text() -> String {
    let mut text = "G21\nG90\nM4 S0\n".to_owned();
    for i in 0..40 {
        let _ = writeln!(text, "G1 X{} Y5 F600 S500", 10 + i);
    }
    text + "M5\n"
}

fn hub(dir: &tempfile::TempDir, started: Instant, local: bool) -> Hub {
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                lines_per_second: 20,
                alarm_at_line: None,
            }),
            require_local_start: local,
            connector_name: "Test".into(),
            spki: SpkiHash::of_spki_der(b"test"),
        },
        started,
    );
    hub
}

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn laptop() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn id() -> Uuid {
    Uuid::from_u128(5)
}

/// Homes, runs the job for two seconds (40 of 44 lines at 20 a second) and stops it.
fn stopped(hub: &Hub, t0: Instant) -> Instant {
    hub.home(&laptop(), t0);
    hub.tick(t0 + ms(2_500));
    assert_eq!(hub.state(), State::Idle);
    let text = text();
    let submit = Message::SubmitJob {
        job_id: id(),
        name: "sign.gcode".into(),
        kind: JobKind::Job,
        bytes: text.len() as u64,
        sha256: sha(&text),
    };
    hub.handle_message(submit, &laptop(), t0 + ms(2_500));
    let chunk = Message::JobChunk {
        job_id: id(),
        index: 0,
        text,
    };
    hub.handle_message(chunk, &laptop(), t0 + ms(2_500));
    hub.handle_message(Message::Start { job_id: id() }, &laptop(), t0 + ms(2_500));
    hub.tick(t0 + ms(4_500));
    hub.handle_message(Message::Stop, &laptop(), t0 + ms(4_500));
    t0 + ms(4_500)
}

fn row(hub: &Hub) -> Option<ListEntry> {
    match hub.job_list() {
        Message::JobList { waiting, .. } => waiting.into_iter().find(|e| e.position == 0),
        _ => None,
    }
}

fn progress(hub: &Hub) -> Option<(u32, Option<u32>, bool)> {
    match hub.status() {
        Message::Status { job: Some(p), .. } => Some((p.line, p.from_line, p.entering)),
        _ => None,
    }
}

fn read(dir: &tempfile::TempDir, name: &str) -> String {
    std::fs::read_to_string(dir.path().join(name)).unwrap_or_default()
}

#[test]
fn a_stopped_job_restarts_from_its_line_and_both_logs_say_so() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0, false);
    let at = stopped(&hub, t0);
    let shown = row(&hub).expect("Frage 390 C: listed as interrupted");
    assert_eq!(shown.interrupted_at, Some(40));
    // The fake has no `$I`: 16 blocks back from line 40.
    assert_eq!(shown.restart_line, Some(25));
    assert_eq!(shown.restart_cause, None);
    let answers = hub.handle_message(
        Message::RestartFromLine {
            job_id: id(),
            line: 25,
        },
        &laptop(),
        at,
    );
    assert!(
        answers.iter().all(|m| !matches!(m, Message::Error { .. })),
        "{answers:?}"
    );
    assert_eq!(hub.state(), State::Running);
    assert_eq!(progress(&hub), Some((24, Some(25), true)));
    assert_eq!(row(&hub), None);
    // The fake's head stands at its origin (jobs do not move it): to X 30, Y 5 is about 30 mm
    // at 50 mm/s, about 610 ms.
    hub.tick(at + ms(300));
    assert!(progress(&hub).unwrap().2, "still moving to line 25");
    hub.tick(at + ms(1_000));
    assert!(!progress(&hub).unwrap().2, "the head reached line 25");
    hub.tick(at + ms(5_000));
    assert_eq!(
        hub.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    assert_eq!(progress(&hub), Some((44, Some(25), false)));
    let jobs = read(&dir, "jobs.log");
    assert!(
        jobs.contains("\tsign.gcode\tRestart from line 25: Laptop\t"),
        "{jobs}"
    );
    assert!(jobs.contains("\tsign.gcode\tCompleted\t"), "{jobs}");
    let admin = read(&dir, "admin.log");
    assert!(
        admin.contains(&format!(
            "\trestart-from-line\tLaptop\tsign.gcode\t{} line 25",
            id()
        )),
        "{admin}"
    );
}

#[test]
fn stop_before_the_line_returns_the_entry_and_writes_no_result_line() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0, false);
    let at = stopped(&hub, t0);
    hub.handle_message(
        Message::RestartFromLine {
            job_id: id(),
            line: 25,
        },
        &laptop(),
        at,
    );
    let before = read(&dir, "jobs.log");
    hub.tick(at + ms(100));
    hub.handle_message(Message::Stop, &laptop(), at + ms(100));
    assert_eq!(hub.state(), State::Idle);
    assert_eq!(row(&hub).unwrap().interrupted_at, Some(40));
    assert_eq!(read(&dir, "jobs.log"), before, "no result line");
}

#[test]
fn under_require_local_start_the_restart_waits_for_enter() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0, true);
    hub.home(&laptop(), t0);
    assert_eq!(hub.state(), State::AwaitingLocalStart);
    hub.local_start(t0);
    hub.tick(t0 + ms(2_500));
    let text = text();
    let submit = Message::SubmitJob {
        job_id: id(),
        name: "sign.gcode".into(),
        kind: JobKind::Job,
        bytes: text.len() as u64,
        sha256: sha(&text),
    };
    let at = t0 + ms(2_500);
    hub.handle_message(submit, &laptop(), at);
    hub.handle_message(
        Message::JobChunk {
            job_id: id(),
            index: 0,
            text,
        },
        &laptop(),
        at,
    );
    hub.handle_message(Message::Start { job_id: id() }, &laptop(), at);
    hub.local_start(at);
    hub.tick(at + ms(2_000));
    hub.handle_message(Message::Stop, &laptop(), at + ms(2_000));
    let line = row(&hub).unwrap().restart_line.unwrap();
    let at = at + ms(2_000);
    hub.handle_message(
        Message::RestartFromLine { job_id: id(), line },
        &laptop(),
        at,
    );
    assert_eq!(hub.state(), State::AwaitingLocalStart);
    assert_eq!(hub.local_start(at).as_deref(), Some("sign.gcode"));
    assert_eq!(hub.state(), State::Running);
}

// ---- The causes of a connector that came back with an interrupted job (`0132` 12, W68) ----

/// The state directory of a connector that was stopped while `text` ran at `line`, with the
/// stored machine position and `offset` (`None`: a list of slice 1, with no `stop`).
fn stored(dir: &tempfile::TempDir, text: &str, line: u32, offset: Option<[f64; 3]>) {
    let jobs = dir.path().join("jobs");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(jobs.join(format!("{}.gcode", id())), text).unwrap();
    let stop = offset.map_or(String::new(), |[x, y, z]| {
        format!(
            r#","stop":{{"machine_position":[0.0,0.0,0.0],"work_offset":[{x:?},{y:?},{z:?}],"run_from":1,"exact":true}}"#
        )
    });
    let list = format!(
        r#"{{"format":1,"current":{{"job_id":"{}","name":"sign.gcode","lines":{},"bytes":{},"sha256":"{}","sent_by":"Phone"{stop},"started":true,"line":{line}}},"waiting":[]}}"#,
        id(),
        text.lines().count(),
        text.len(),
        sha(text),
    );
    std::fs::write(jobs.join("list.json"), list).unwrap();
}

/// The cause the row shows and the answer to a restart from its line, which must agree.
fn refused(hub: &Hub, at: Instant) -> (Option<String>, Option<String>) {
    let shown = row(hub).expect("the interrupted row");
    let line = shown.restart_line.expect("a restart line");
    let answers = hub.handle_message(
        Message::RestartFromLine { job_id: id(), line },
        &laptop(),
        at,
    );
    let error = answers.iter().find_map(|m| match m {
        Message::Error {
            code: curvyo_connector_core::ErrorCode::WrongState,
            message,
        } => Some(message.clone()),
        _ => None,
    });
    assert_ne!(hub.state(), State::Running, "refused: {answers:?}");
    (shown.restart_cause, error)
}

fn homed_hub(dir: &tempfile::TempDir, t0: Instant) -> (Hub, Instant) {
    let hub = hub(dir, t0, false);
    hub.home(&laptop(), t0);
    hub.tick(t0 + ms(2_500));
    assert_eq!(hub.state(), State::Idle);
    (hub, t0 + ms(2_500))
}

fn both(cause: &str) -> (Option<String>, Option<String>) {
    (Some(cause.to_owned()), Some(cause.to_owned()))
}

#[test]
fn after_a_connector_restart_with_no_report_yet_a_restart_waits_for_the_home() {
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text(), 40, Some([0.0; 3]));
    let t0 = Instant::now();
    let hub = hub(&dir, t0, false);
    hub.tick(t0 + ms(500));
    // The fake's head stands at its origin and has reported nothing since the connector came.
    assert_eq!(
        refused(&hub, t0 + ms(500)),
        both("No position from the machine")
    );
    hub.home(&laptop(), t0 + ms(500));
    hub.tick(t0 + ms(3_000));
    let shown = row(&hub).unwrap();
    assert_eq!(shown.restart_cause, None, "homed, same offset");
}

#[test]
fn a_restored_job_is_refused_not_homed_when_the_head_reports_but_never_homed() {
    // The fake reports its position after a jog: known, but not homed since the connector came.
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text(), 40, Some([0.0; 3]));
    let t0 = Instant::now();
    let hub = hub(&dir, t0, false);
    hub.handle_message(
        Message::Jog {
            axis: curvyo_connector_core::JogAxis::X,
            step: curvyo_connector_core::JogStep::One,
            direction: curvyo_connector_core::JogDirection::Plus,
            hold: false,
        },
        &laptop(),
        t0,
    );
    hub.tick(t0 + ms(1_000));
    assert_eq!(refused(&hub, t0 + ms(1_000)), both("Not homed"));
}

#[test]
fn a_changed_work_offset_is_refused_also_after_a_home() {
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text(), 40, Some([5.0, 0.0, 0.0]));
    let t0 = Instant::now();
    let (hub, at) = homed_hub(&dir, t0);
    assert_eq!(refused(&hub, at), both("Work offset changed"));
}

#[test]
fn a_list_without_a_stop_point_is_refused_no_position_from_the_machine() {
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text(), 40, None);
    let t0 = Instant::now();
    let (hub, at) = homed_hub(&dir, t0);
    assert_eq!(refused(&hub, at), both("No position from the machine"));
}

#[test]
fn an_unknown_word_before_the_line_is_refused_with_the_word() {
    let mut text = "G21\nG90\nM4 S0\nG28\n".to_owned();
    for i in 0..40 {
        let _ = writeln!(text, "G1 X{} Y5 F600 S500", 10 + i);
    }
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text, 40, Some([0.0; 3]));
    let t0 = Instant::now();
    let (hub, at) = homed_hub(&dir, t0);
    let n = row(&hub).unwrap().restart_line.unwrap();
    assert_eq!(
        refused(&hub, at),
        both(&format!("Cannot rebuild the state before line {n}: G28"))
    );
}

#[test]
fn an_arc_mode_with_no_motion_word_on_line_n_is_refused() {
    // Every move is an arc and line n gives only its axes: GRBL would need `G2` on it.
    let mut text = "G21\nG90\nM4 S0\nG1 X0 Y0 F600\nG2 X1 Y0 I0.5 J0 S500\n".to_owned();
    for i in 0..40 {
        let _ = writeln!(text, "X{} Y0 I0.5 J0", 2 + i);
    }
    let dir = tempfile::tempdir().unwrap();
    stored(&dir, &text, 44, Some([0.0; 3]));
    let t0 = Instant::now();
    let (hub, at) = homed_hub(&dir, t0);
    let n = row(&hub).unwrap().restart_line.unwrap();
    assert_eq!(
        refused(&hub, at),
        both(&format!("Cannot rebuild the state before line {n}: G2"))
    );
}
