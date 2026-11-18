//! The job state machine of one connector: events in, new state and messages out, no clock.

mod controls;

use uuid::Uuid;

use crate::assembly::{Assembly, ChunkError, Job};
use crate::message::{ErrorCode, JobKind, Message, Progress, RejectReason, State};
use crate::millimetres::Millimetres;

/// Settings of the machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// `Start` waits for the button on the connector's host (CONN-SAFETY, off by default).
    pub require_local_start: bool,
}

/// Something that happened: a message from the app or an event from the controller side.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A message from a paired, greeted client.
    Message(Message),
    /// A controller was attached.
    ControllerConnected,
    /// The controller was lost; a job in flight is dropped and named in the detail.
    ControllerLost,
    /// The controller got through `line` lines; `elapsed_ms` is counted by the caller.
    Progress {
        /// Lines done.
        line: u32,
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller reports its position.
    Position([Millimetres; 3]),
    /// The controller ran every line.
    JobFinished {
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller reported an error while running.
    JobFailed {
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller failed the job, running or paused, for the reason in `detail`.
    JobFailedWith {
        /// Milliseconds since the start.
        elapsed_ms: u64,
        /// The sentence the app shows ("Failed at line 12: No feed rate (error 22)").
        detail: String,
    },
    /// The controller raised an alarm.
    Alarm {
        /// Its text.
        text: String,
    },
    /// The alarm was cleared.
    AlarmCleared,
    /// The button on the connector's host was pressed.
    LocalStartPressed,
    /// A client connection that sent a job ended; a job still being received is dropped
    /// (back to `Idle`), a job that is `Ready` or further is not affected.
    ClientDisconnected,
    /// The controller holds the job (asked for or not): `Running` becomes `Paused`.
    Held,
    /// The controller runs the held job again.
    Resumed,
    /// The controller confirms that a stop ended: the job is `Done { Stopped }`.
    Stopped,
    /// The controller could not confirm a stop: the job is dropped, `NoController`.
    StopUnconfirmed,
    /// Why there is no controller ("Port /dev/ttyUSB0 not found"); only in `NoController`.
    NoControllerDetail {
        /// The sentence.
        detail: String,
    },
    /// The controller reports a state outside the job's states ("Door"); `None` once it left it.
    ControllerWord(Option<String>),
    /// Whether the controller's laser mode is off: jobs are refused while it is (G34).
    LaserModeOff(bool),
    /// Whether the controller's laser mode could not be read: jobs are refused while so (G34).
    LaserModeUnknown(bool),
}

/// What the transport has to tell the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerCommand {
    /// Stream the held job.
    Start {
        /// The job.
        job_id: Uuid,
    },
    /// Hold the stream.
    Pause,
    /// Continue the stream.
    Resume,
    /// End the stream.
    Stop,
    /// Leave the controller's alarm lock.
    Unlock,
}

/// The result of one event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    /// Messages for the app, in order.
    pub messages: Vec<Message>,
    /// Commands for the controller, in order.
    pub commands: Vec<ControllerCommand>,
}

#[derive(Debug, Clone)]
struct Held {
    job: Job,
    lines: u32,
    line: u32,
    elapsed_ms: u64,
}

/// The state of one connector and the one job it holds.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // independent flags of the job machine, each read on its own
pub struct Machine {
    options: Options,
    /// Pause, Resume and Stop wait for the controller's `Held`, `Resumed` and `Stopped`.
    confirm: bool,
    state: State,
    assembly: Option<Assembly>,
    held: Option<Held>,
    position: Option<[Millimetres; 3]>,
    alarm: Option<String>,
    /// The sentence that explains the state.
    detail: Option<String>,
    /// The detail names a lost job and stays through the reconnect until the next job state.
    sticky: bool,
    /// A controller state outside the job's states.
    word: Option<String>,
    /// A stop was sent and waits for the controller.
    stop_pending: bool,
    /// The controller's laser mode is off: `SubmitJob` is refused.
    laser_off: bool,
    /// The controller's laser mode is unknown: `SubmitJob` is refused.
    laser_unknown: bool,
}

/// Controller states in which no job may start (`specs/0127` G4): the door, check mode, sleep.
const BUSY_WORDS: [&str; 3] = ["Door", "Check", "Sleep"];

/// A count with a thousands comma: "12,345".
pub(crate) fn count(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Machine {
    /// A connector without a controller whose Pause, Resume and Stop take effect at once.
    #[must_use]
    pub fn new(options: Options) -> Self {
        Self {
            options,
            confirm: false,
            state: State::NoController,
            assembly: None,
            held: None,
            position: None,
            alarm: None,
            detail: None,
            sticky: false,
            word: None,
            stop_pending: false,
            laser_off: false,
            laser_unknown: false,
        }
    }

    /// A connector whose Pause, Resume and Stop take effect only when the controller confirms
    /// them (`Held`, `Resumed`, `Stopped`), as a real machine needs (`specs/0127` G10 to G12).
    #[must_use]
    pub fn confirming(options: Options) -> Self {
        Self {
            confirm: true,
            ..Self::new(options)
        }
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// The assembled job, from `Ready` until it is replaced or discarded.
    #[must_use]
    pub fn held_job(&self) -> Option<&Job> {
        self.held.as_ref().map(|h| &h.job)
    }

    fn shown_detail(&self) -> Option<String> {
        if self.stop_pending {
            return Some("Stopping".to_owned());
        }
        self.detail.clone().or_else(|| {
            self.word
                .as_ref()
                .map(|word| format!("Controller state: {word}"))
        })
    }

    /// The `Status` message for the current state.
    #[must_use]
    pub fn status(&self) -> Message {
        let carries_job = matches!(
            self.state,
            State::Ready
                | State::Running
                | State::Paused
                | State::AwaitingLocalStart
                | State::Done { .. }
        );
        let job = self
            .held
            .as_ref()
            .filter(|_| carries_job)
            .map(|h| Progress {
                job_id: h.job.job_id,
                line: h.line,
                lines: h.lines,
                elapsed_ms: h.elapsed_ms,
            });
        Message::Status {
            state: self.state,
            job,
            position: self.position,
            alarm: self.alarm.clone(),
            detail: self.shown_detail(),
        }
    }

    /// Applies one event.
    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        match event {
            Event::Message(message) => self.on_message(message, &mut out),
            Event::ClientDisconnected => {
                if self.state == State::Receiving {
                    self.discard();
                    self.set(State::Idle, &mut out);
                }
            }
            other => self.on_controller(other, &mut out),
        }
        out
    }

    /// Enters `state`; a detail stays only when it names a lost job and the state is
    /// `NoController` or `Idle` (the reconnect).
    fn set(&mut self, state: State, out: &mut Output) {
        let keep = self.sticky && matches!(state, State::NoController | State::Idle);
        if !keep {
            self.detail = None;
            self.sticky = false;
        }
        self.enter(state, out);
    }

    /// Enters `state` with `detail`.
    fn set_with(&mut self, state: State, detail: String, sticky: bool, out: &mut Output) {
        self.detail = Some(detail);
        self.sticky = sticky;
        self.enter(state, out);
    }

    fn enter(&mut self, state: State, out: &mut Output) {
        if !matches!(state, State::Running | State::Paused) {
            self.stop_pending = false;
        }
        self.state = state;
        out.messages.push(self.status());
    }

    fn error(out: &mut Output, code: ErrorCode, message: &str) {
        out.messages.push(Message::Error {
            code,
            message: message.to_owned(),
        });
    }

    fn wrong_state(out: &mut Output) {
        Self::error(
            out,
            ErrorCode::WrongState,
            "This does not fit the current state.",
        );
    }

    fn discard(&mut self) {
        self.assembly = None;
        self.held = None;
    }

    fn reject(out: &mut Output, job_id: Uuid, reason: RejectReason) {
        out.messages.push(Message::JobRejected { job_id, reason });
    }

    fn on_message(&mut self, message: Message, out: &mut Output) {
        match message {
            Message::GetStatus => out.messages.push(self.status()),
            Message::SubmitJob {
                job_id,
                name,
                kind,
                bytes,
                sha256,
            } => self.submit(job_id, name, kind, bytes, sha256, out),
            Message::JobChunk {
                job_id,
                index,
                text,
            } => self.chunk(job_id, index, &text, out),
            Message::Start { job_id } => self.start(job_id, out),
            Message::Pause | Message::Resume | Message::Stop => self.control(&message, out),
            // Outside `Alarm` nothing is written; the caller answers with the status (G33).
            Message::Unlock if self.state == State::Alarm => {
                out.commands.push(ControllerCommand::Unlock);
            }
            Message::Unlock => {}
            _ => Self::error(
                out,
                ErrorCode::UnexpectedMessage,
                "The connector does not accept this message.",
            ),
        }
    }

    fn submit(
        &mut self,
        job_id: Uuid,
        name: String,
        kind: JobKind,
        bytes: u64,
        sha256: String,
        out: &mut Output,
    ) {
        match self.state {
            State::NoController => Self::reject(out, job_id, RejectReason::NoController),
            State::Idle | State::Done { .. } if self.laser_off => {
                Self::reject(out, job_id, RejectReason::LaserModeOff);
            }
            State::Idle | State::Done { .. } if self.laser_unknown => {
                Self::reject(out, job_id, RejectReason::LaserModeUnknown);
            }
            State::Idle | State::Done { .. }
                if self
                    .word
                    .as_deref()
                    .is_some_and(|w| BUSY_WORDS.contains(&w)) =>
            {
                Self::reject(out, job_id, RejectReason::Busy);
            }
            State::Idle | State::Done { .. } => {
                match Assembly::begin(job_id, name, kind, bytes, sha256) {
                    Err(reason) => Self::reject(out, job_id, reason),
                    Ok(assembly) => {
                        self.held = None;
                        self.set(State::Receiving, out);
                        if assembly.is_full() {
                            self.complete(assembly, out);
                        } else {
                            self.assembly = Some(assembly);
                        }
                    }
                }
            }
            _ => Self::reject(out, job_id, RejectReason::Busy),
        }
    }

    fn chunk(&mut self, job_id: Uuid, index: u32, text: &str, out: &mut Output) {
        let Some(assembly) = self.assembly.as_mut() else {
            Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            return;
        };
        match assembly.push(job_id, index, text) {
            Ok(()) if assembly.is_full() => {
                if let Some(assembly) = self.assembly.take() {
                    self.complete(assembly, out);
                }
            }
            Ok(()) => {}
            Err(ChunkError::UnknownJob) => {
                Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            }
            Err(ChunkError::Reject(reason)) => {
                let held_id = assembly.job_id();
                self.assembly = None;
                Self::reject(out, held_id, reason);
                self.set(State::Idle, out);
            }
        }
    }

    fn complete(&mut self, assembly: Assembly, out: &mut Output) {
        let job_id = assembly.job_id();
        match assembly.finish() {
            Ok(job) => {
                let lines = u32::try_from(job.text.lines().count()).unwrap_or(u32::MAX);
                self.held = Some(Held {
                    job,
                    lines,
                    line: 0,
                    elapsed_ms: 0,
                });
                out.messages.push(Message::JobAccepted { job_id });
                self.set(State::Ready, out);
            }
            Err(reason) => {
                Self::reject(out, job_id, reason);
                self.set(State::Idle, out);
            }
        }
    }

    fn start(&mut self, job_id: Uuid, out: &mut Output) {
        let held_id = self.held.as_ref().map(|h| h.job.job_id);
        if self.state != State::Ready {
            Self::wrong_state(out);
        } else if held_id != Some(job_id) {
            Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
        } else if self.options.require_local_start {
            self.set(State::AwaitingLocalStart, out);
        } else {
            out.commands.push(ControllerCommand::Start { job_id });
            self.set(State::Running, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn counts_have_a_thousands_comma() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(12_345), "12,345");
        assert_eq!(count(1_234_567), "1,234,567");
    }
}
