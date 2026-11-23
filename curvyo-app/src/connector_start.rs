//! The start step of Send to machine (`specs/0134-send-and-start` SS1 to SS6; architect addendum
//! S1 to S3): whether a status lets the press start, and `Start` for the app's own job id on the
//! connection that got `JobAccepted`. It never starts whatever job is `Ready`.

use curvyo_connector_core::{Message, State};
use curvyo_connector_io::Uuid;
use std::time::{Duration, Instant};

use curvyo_connector_io::client::Connection;
use curvyo_connector_io::frames::Frame;
use curvyo_ui_core::{SendMachine, send_starts};
use serde::Serialize;

/// How long the connector gets to answer `Start`; it answers at once, the bound only ends a
/// silent connection (the watcher's status then decides).
const START_WAIT: Duration = Duration::from_secs(2);

/// The prefix of a status detail that carries the controller's own state word ("Door").
const WORD_PREFIX: &str = "Controller state: ";

/// What the start step of a send did, as the page reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum StartView {
    /// The connector took the start: the job runs, waits for the start at the machine or a
    /// confirmation, or its controller ended it at once (the status says how).
    Started,
    /// The connector refused the start or the connection dropped first; the job stays `Ready`.
    /// `reason` is the error code's name (`WrongState`, `UnknownJob`) or `ConnectionLost`.
    NotStarted { reason: String },
    /// No change within the command's wait: the watcher's status decides.
    NoAnswer,
}

/// The machine a status names for the start rule (`0134` addendum S1); anything but a `Status`
/// is no status.
pub(crate) fn machine_of(status: &Message) -> SendMachine {
    let Message::Status { state, detail, .. } = status else {
        return SendMachine::Unknown;
    };
    let word = detail
        .as_deref()
        .and_then(|d| d.strip_prefix(WORD_PREFIX))
        .unwrap_or("");
    SendMachine::parse(state_name(*state), word)
}

/// The connector's spelling of `state`, as the page's `state.name`.
fn state_name(state: State) -> &'static str {
    match state {
        State::NoController => "NoController",
        State::Idle => "Idle",
        State::Receiving => "Receiving",
        State::Ready => "Ready",
        State::AwaitingLocalStart => "AwaitingLocalStart",
        State::Homing => "Homing",
        State::Jogging => "Jogging",
        State::Running => "Running",
        State::Paused => "Paused",
        State::Done { .. } => "Done",
        State::Alarm => "Alarm",
    }
}

/// Whether the press asked for a start and the send's own first status still says it starts
/// (`0134` SS9, UX P4 at the confirming press).
pub(crate) fn starts_after_send(asked: bool, first_status: Option<&Message>) -> bool {
    asked && first_status.is_some_and(|status| send_starts(machine_of(status)))
}

/// Sends `Start` for `job_id`, the id this app sent, on `connection`, which just got
/// `JobAccepted` for it, and reads the answer (`0134` addendum S2, S3): the connector's `Error`
/// for this command, or a status that shows this job started. Statuses of other changes (another
/// client's stop, another job) are skipped, so a refusal is never mistaken for a start.
pub(crate) fn start_own_job(connection: &mut Connection, job_id: Uuid) -> StartView {
    if connection.send(&Message::Start { job_id }).is_err() {
        return lost();
    }
    let until = Instant::now() + START_WAIT;
    loop {
        match connection.try_recv(until) {
            Ok(None) => return StartView::NoAnswer,
            Err(_) => return lost(),
            Ok(Some(Frame::Core(Message::Error { code, .. }))) => {
                return StartView::NotStarted {
                    reason: format!("{code:?}"),
                };
            }
            Ok(Some(Frame::Core(status @ Message::Status { .. }))) if started(&status, job_id) => {
                return StartView::Started;
            }
            Ok(Some(_)) => {}
        }
    }
}

fn lost() -> StartView {
    StartView::NotStarted {
        reason: "ConnectionLost".to_owned(),
    }
}

/// Whether `status` shows the job `job_id` past `Ready`: running, waiting for the start at the
/// machine or a confirmation, paused, or already ended by its controller.
fn started(status: &Message, job_id: Uuid) -> bool {
    matches!(
        status,
        Message::Status {
            state: State::Running | State::AwaitingLocalStart | State::Paused | State::Done { .. },
            job: Some(progress),
            ..
        } if progress.job_id == job_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::{Event, Machine, Options, Peer, StartRule};
    use curvyo_connector_core::{JobKind, JobResult};

    fn submit(job_id: Uuid) -> Event {
        Event::Message(
            Message::SubmitJob {
                job_id,
                name: "page".to_owned(),
                kind: JobKind::Job,
                bytes: 3,
                sha256: "3b7fe25639867d0e186bd77a042b8cfd7a1dc313b72ce8603a5681a9093ac19c"
                    .to_owned(),
            },
            Peer::host(),
        )
    }

    /// A machine driven into `state` by events only.
    fn machine_in(state: &str) -> Machine {
        let options = Options {
            start_rule: if state == "AwaitingLocalStart" {
                StartRule::LocalPress
            } else {
                StartRule::Immediate
            },
        };
        let mut m = Machine::new(options);
        if state == "NoController" {
            return m;
        }
        m.handle(Event::ControllerConnected);
        let id = Uuid::from_u128(7);
        match state {
            "Idle" => {}
            "Alarm" => {
                m.handle(Event::Alarm {
                    text: "Hard limit".to_owned(),
                });
            }
            "Homing" => {
                m.handle(Event::Message(Message::Home, Peer::host()));
            }
            "Receiving" => {
                m.handle(submit(id));
            }
            _ => {
                m.handle(submit(id));
                m.handle(Event::Message(
                    Message::JobChunk {
                        job_id: id,
                        index: 0,
                        text: "G0\n".to_owned(),
                    },
                    Peer::host(),
                ));
                if state != "Ready" {
                    m.handle(Event::Message(Message::Start { job_id: id }, Peer::host()));
                }
                if state == "Paused" {
                    m.handle(Event::Message(Message::Pause, Peer::host()));
                }
                if state == "Done" {
                    m.handle(Event::JobFinished { elapsed_ms: 1 });
                }
            }
        }
        m
    }

    /// Whether a send would become the current job (`specs/0132`: any other accepted send is
    /// listed behind it and starts nothing).
    fn accepts_a_job(m: &mut Machine) -> bool {
        let before = m.state();
        let out = m.handle(submit(Uuid::from_u128(9)));
        let refused = out
            .messages
            .iter()
            .any(|message| matches!(message, Message::JobRejected { .. }));
        !refused && before != State::Receiving && m.state() == State::Receiving
    }

    #[test]
    fn ss11_the_start_rule_is_the_connectors_accept_rule_in_every_state() {
        let names = [
            "NoController",
            "Idle",
            "Receiving",
            "Ready",
            "AwaitingLocalStart",
            "Homing",
            "Running",
            "Paused",
            "Done",
            "Alarm",
        ];
        for name in names {
            let mut m = machine_in(name);
            assert_eq!(state_name(m.state()), name, "the rig reached {name}");
            let starts = send_starts(machine_of(&m.status()));
            assert_eq!(starts, accepts_a_job(&mut m), "{name}");
        }
    }

    #[test]
    fn ss5_a_controller_word_or_laser_mode_off_never_starts() {
        for word in ["Door", "Check", "Sleep"] {
            let mut m = machine_in("Idle");
            m.handle(Event::ControllerWord(Some(word.to_owned())));
            assert!(!send_starts(machine_of(&m.status())), "{word}");
            assert!(!accepts_a_job(&mut m), "{word}");
        }
        let mut m = machine_in("Idle");
        m.handle(Event::LaserModeOff(true));
        let out = m.handle(submit(Uuid::from_u128(9)));
        assert!(
            out.messages.iter().any(|message| matches!(
                message,
                Message::JobRejected {
                    reason: curvyo_connector_core::RejectReason::LaserModeOff,
                    ..
                }
            )),
            "$32=0 refuses the send itself, so nothing is accepted and nothing starts"
        );
    }

    #[test]
    fn ss9_a_press_that_did_not_ask_never_starts() {
        let idle = machine_in("Idle").status();
        assert!(starts_after_send(true, Some(&idle)));
        assert!(
            !starts_after_send(false, Some(&idle)),
            "P4: only send stays only send"
        );
        assert!(!starts_after_send(true, None));
        let held = machine_in("Ready").status();
        assert!(
            !starts_after_send(true, Some(&held)),
            "the fresh status holds a job"
        );
        let done = Message::Status {
            state: State::Done {
                result: JobResult::Completed,
            },
            job: None,
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
        };
        assert!(starts_after_send(true, Some(&done)));
    }

    #[test]
    fn the_start_answer_is_tagged_camel_case() {
        let json = serde_json::to_value(StartView::NotStarted {
            reason: "WrongState".to_owned(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "notStarted", "reason": "WrongState"})
        );
    }
}
