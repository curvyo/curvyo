//! Send and start against a running connector with the fake controller (`specs/0134` SS1 to SS3,
//! SS6, SS11; architect addendum S7). Waits are by statuses, never by sleeps.

use std::time::{Duration, Instant};

use curvyo_connector_core::{Message, State};
use curvyo_connector_io::client::{
    CONNECT_TIMEOUT, Connection, DEFAULT_PORT, connect, open_paired,
};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
use curvyo_connector_io::job_send::new_job_id;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::send_tracked::{SharedWatch, send_tracked_open, watched_job};
use curvyo_connector_io::server::{RunningServer, ServerConfig};
use std::sync::{Arc, Mutex};

use super::{PageSendAnswer, Pending, ResolutionView, run_send};
use crate::connector_start::{StartView, start_own_job};
use curvyo_connector_core::SendWatch;

const JOB: &str = "G21\nG90\nG1 X1 F100 S10\n";
const WAIT: Duration = Duration::from_secs(10);

struct Rig {
    server: RunningServer,
    _dir: tempfile::TempDir,
    record: PairingRecord,
}

impl Rig {
    fn new(require_local_start: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let server = RunningServer::start(&ServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                // Slow, so a started job is still running when the test looks.
                lines_per_second: 1,
                alarm_at_line: None,
            }),
            require_local_start,
            connector_name: "Test connector".to_owned(),
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
            Instant::now() + CONNECT_TIMEOUT,
        )
        .unwrap();
        Self {
            server,
            _dir: dir,
            record,
        }
    }

    fn address(&self) -> String {
        self.server.addr().to_string()
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

    fn send(&self, held: &Pending, start: bool) -> PageSendAnswer {
        run_send(
            &self.address(),
            DEFAULT_PORT,
            &self.record,
            held,
            ("page", JOB),
            start,
        )
    }
}

fn pending() -> Arc<Pending> {
    Arc::new(Pending {
        watch: Mutex::new(Some(SendWatch::new(new_job_id().unwrap()))),
    })
}

fn state_of(status: &Message) -> Option<State> {
    match status {
        Message::Status { state, .. } => Some(*state),
        _ => None,
    }
}

fn job_of(status: &Message) -> Option<curvyo_connector_io::Uuid> {
    match status {
        Message::Status { job, .. } => job.as_ref().map(|j| j.job_id),
        _ => None,
    }
}

/// Reads statuses on `connection` until one satisfies `done`.
fn wait_for(connection: &mut Connection, done: impl Fn(&Message) -> bool) -> Message {
    let deadline = Instant::now() + WAIT;
    loop {
        let message = connection.recv_message(deadline).unwrap();
        if matches!(message, Message::Status { .. }) && done(&message) {
            return message;
        }
    }
}

/// The state a fresh connection reports first.
fn fresh_state(rig: &Rig) -> State {
    state_of(&rig.open().1).unwrap()
}

fn accepted_with(answer: &PageSendAnswer) -> Option<StartView> {
    match answer {
        PageSendAnswer::Resolved {
            resolution: ResolutionView::Accepted,
            start,
            ..
        }
        | PageSendAnswer::ResolvedElsewhere { start } => start.clone(),
        other => panic!("not accepted: {other:?}"),
    }
}

#[test]
fn ss1_a_free_machine_sends_and_starts_the_sent_job() {
    let rig = Rig::new(false);
    let held = pending();
    let id = watched_job(&held.watch).unwrap();
    let answer = rig.send(&held, true);
    assert_eq!(accepted_with(&answer), Some(StartView::Started));
    let (_, status) = rig.open();
    assert_eq!(state_of(&status), Some(State::Running));
    assert_eq!(job_of(&status), Some(id), "our own job runs");
}

#[test]
fn ss9_a_press_that_did_not_ask_only_sends() {
    let rig = Rig::new(false);
    let answer = rig.send(&pending(), false);
    assert_eq!(accepted_with(&answer), None);
    assert_eq!(fresh_state(&rig), State::Ready);
}

#[test]
fn ss2_a_held_job_is_listed_behind_and_nothing_starts() {
    let rig = Rig::new(false);
    let first = pending();
    let first_id = watched_job(&first.watch).unwrap();
    rig.send(&first, false);
    let answer = rig.send(&pending(), true);
    assert_eq!(
        answer,
        PageSendAnswer::Resolved {
            job_id: answer_id(&answer),
            resolution: ResolutionView::Accepted,
            start: None,
        }
    );
    let (_, status) = rig.open();
    assert_eq!(
        state_of(&status),
        Some(State::Ready),
        "the held job stays Ready"
    );
    assert_eq!(job_of(&status), Some(first_id));
}

fn answer_id(answer: &PageSendAnswer) -> String {
    match answer {
        PageSendAnswer::Resolved { job_id, .. } => job_id.clone(),
        _ => String::new(),
    }
}

#[test]
fn ss3_under_require_local_start_the_start_waits_at_the_machine() {
    let rig = Rig::new(true);
    let answer = rig.send(&pending(), true);
    assert_eq!(accepted_with(&answer), Some(StartView::Started));
    assert_eq!(fresh_state(&rig), State::AwaitingLocalStart);
}

/// Our job accepted on `connection`; a second client then changes the connector before our
/// `Start` goes out, by `between`.
fn accepted_then(rig: &Rig, between: impl FnOnce(&Rig)) -> (StartView, curvyo_connector_io::Uuid) {
    let id = new_job_id().unwrap();
    let watch: SharedWatch = Mutex::new(Some(SendWatch::new(id)));
    let mut open = || {
        open_paired(
            &rig.address(),
            rig.record.spki,
            &rig.record.token,
            CONNECT_TIMEOUT,
        )
    };
    let (_, connection) =
        send_tracked_open(&mut open, &watch, "page", JOB, Instant::now() + WAIT).unwrap();
    let mut connection = connection.unwrap();
    between(rig);
    (start_own_job(&mut connection, id), id)
}

#[test]
fn ss6_a_start_refused_after_acceptance_is_not_started_and_another_job_is_untouched() {
    let rig = Rig::new(false);
    let other = new_job_id().unwrap();
    let (view, _) = accepted_then(&rig, |rig| {
        let (mut second, _) = rig.open();
        second.send(&Message::Stop).unwrap();
        wait_for(&mut second, |s| state_of(s) == Some(State::Idle));
        second
            .send_job_with_id(other, "other", JOB, Instant::now() + WAIT)
            .unwrap();
        wait_for(&mut second, |s| state_of(s) == Some(State::Ready));
    });
    assert_eq!(
        view,
        StartView::NotStarted {
            reason: "UnknownJob".to_owned()
        },
        "our Start names our id, never the Ready job"
    );
    let (_, status) = rig.open();
    assert_eq!(state_of(&status), Some(State::Ready));
    assert_eq!(
        job_of(&status),
        Some(other),
        "the other app's job stays Ready"
    );
}

#[test]
fn ss6_a_job_another_client_started_first_reads_wrong_state() {
    let rig = Rig::new(false);
    let (view, id) = accepted_then(&rig, |rig| {
        let (mut second, status) = rig.open();
        second
            .send(&Message::Start {
                job_id: job_of(&status).unwrap(),
            })
            .unwrap();
        wait_for(&mut second, |s| state_of(s) == Some(State::Running));
    });
    // Our job runs either way: by the other client's Start, and ours is refused, or ours was
    // read after theirs. Never a "Not started" with another reason.
    assert!(
        matches!(&view, StartView::NotStarted { reason } if reason == "WrongState")
            || view == StartView::Started,
        "{view:?}"
    );
    let (_, status) = rig.open();
    assert_eq!(job_of(&status), Some(id));
    assert_eq!(state_of(&status), Some(State::Running));
}
