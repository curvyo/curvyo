//! The job state machine of one connector: events in, new state and messages out, no clock.

mod arrival;
mod confirm;
mod controls;
mod home;
mod list;

pub use confirm::{Confirmation, Peer, People, StartRule};
pub use list::ListLog;

use uuid::Uuid;

use crate::assembly::{Assembly, Job};
pub(crate) use crate::count::count;
use crate::job_list::JobList;
use crate::message::{ErrorCode, Message, Progress, RejectReason, State};
use crate::millimetres::Millimetres;
use list::Transfer;

/// Settings of the machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Who has to agree before a `Start` runs (architect addendum D5).
    pub start_rule: StartRule,
}

/// Something that happened: a message from the app or an event from the controller side.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A message from a paired, greeted client, or from the host itself.
    Message(Message, Peer),
    /// A controller was attached.
    ControllerConnected,
    /// The controller was lost; a job in flight is dropped and named in the detail.
    ControllerLost,
    /// The controller was lost for the reason in `detail`, which the app shows in place of the
    /// line count; a job in flight is dropped (`specs/0142`, the homing cap).
    ControllerLostWith {
        /// The sentence ("Lost the controller: homing took longer than 120 s").
        detail: String,
    },
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
    /// The connection that sent the job `job_id` ended: that job's transfer is dropped if it is
    /// still arriving, and no other (`specs/0125` S19, `specs/0132` L2).
    TransferAbandoned {
        /// The job.
        job_id: Uuid,
    },
    /// The chain's decision when a job ended (made inside the machine, carried out by the
    /// caller as the next event): the first waiting job `job_id` starts.
    RunNext {
        /// The job.
        job_id: Uuid,
    },
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
    /// The controller ended its homing cycle (GRBL's `ok` to `$H`, `specs/0118` M2).
    Homed,
    /// The controller refused the homing cycle and stays where it was (GRBL `error:<n>` to `$H`).
    HomingFailed {
        /// The sentence the app shows ("Homing is not enabled (error 5)").
        detail: String,
    },
    /// The controller cannot hold now (a job's `$H` runs) and holds at its first safe moment;
    /// `Running` keeps the pause pending until `Held` (`specs/0133` P3).
    PauseDeferred,
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
    /// Run the controller's homing cycle.
    Home,
}

/// The result of one event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    /// Messages for the app, in order.
    pub messages: Vec<Message>,
    /// Commands for the controller, in order.
    pub commands: Vec<ControllerCommand>,
    /// A start or resume that waited ran: who asked and who agreed, for `admin.log`.
    pub confirmed: Option<Confirmation>,
    /// Changes of the job list that `admin.log` records.
    pub list_log: Vec<ListLog>,
    /// An event the caller applies next, with the same rules as any other (the chain).
    pub follow_up: Option<Event>,
}

/// What `AwaitingLocalStart` waits to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Waiting {
    /// The held job.
    Start,
    /// The paused job's resume (owner rule only).
    Resume,
    /// A homing cycle (`specs/0118` M5).
    Home,
}

#[derive(Debug, Clone)]
struct Held {
    job: Job,
    sha256: String,
    lines: u32,
    line: u32,
    elapsed_ms: u64,
    people: People,
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
    /// A pause was taken and waits for the controller's first moment to hold (`specs/0133`).
    pause_pending: bool,
    /// The controller's laser mode is off: `SubmitJob` is refused.
    laser_off: bool,
    /// The controller's laser mode is unknown: `SubmitJob` is refused.
    laser_unknown: bool,
    /// What `AwaitingLocalStart` waits to start.
    waiting: Waiting,
    /// The state a homing started from (`Idle`, `Ready` or `Alarm`), and returns to.
    homed_from: State,
    /// Who pressed the start or resume that waits in `AwaitingLocalStart`.
    requester: Option<String>,
    /// Who sent the job being received.
    sender: String,
    /// The jobs behind the current one.
    list: JobList,
    /// Jobs for the list whose chunks are still arriving.
    transfers: Vec<Transfer>,
    /// "Chain stopped: `<state>`" until the next start.
    note: Option<String>,
    /// The last `JobList` pushed.
    last_list: Option<Message>,
}

/// Controller states in which no job may start (`specs/0127` G4): the door, check mode, sleep.
const BUSY_WORDS: [&str; 3] = ["Door", "Check", "Sleep"];

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
            pause_pending: false,
            laser_off: false,
            laser_unknown: false,
            waiting: Waiting::Start,
            homed_from: State::Idle,
            requester: None,
            sender: String::new(),
            list: JobList::default(),
            transfers: Vec::new(),
            note: None,
            last_list: None,
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
        if self.state == State::Homing {
            // The state says it; the controller's `Home` word is not repeated (`specs/0118` M3).
            return None;
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
                | State::Homing
                | State::Done { .. }
        );
        // A waiting Home starts no job, so it names none.
        let waits_home = self.state == State::AwaitingLocalStart && self.waiting == Waiting::Home;
        let job = self
            .held
            .as_ref()
            .filter(|_| carries_job && !waits_home)
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
            alarm: self.alarm.clone().filter(|_| self.state != State::Homing),
            detail: self.shown_detail(),
            pause_pending: self.pause_pending,
            requested_by: self.shown_requester(),
            job_name: self.shown_job().map(|h| h.job.name.clone()),
            sent_by: self.shown_job().map(|h| h.people.sent_by.clone()),
        }
    }

    /// Applies one event.
    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        match event {
            Event::Message(message, peer) => self.on_message(message, &peer, &mut out),
            Event::ClientDisconnected => {
                if self.state == State::Receiving {
                    self.discard();
                    self.set(State::Idle, &mut out);
                }
            }
            Event::TransferAbandoned { job_id } => self.abandon(job_id, &mut out),
            Event::RunNext { job_id } => self.run_next(job_id, &mut out),
            other => self.on_controller(other, &mut out),
        }
        self.announce_list(&mut out);
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
        if !matches!(state, State::Running | State::Paused | State::Homing) {
            self.stop_pending = false;
        }
        self.pause_pending &= state == State::Running;
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
        self.requester = None;
    }

    fn reject(out: &mut Output, job_id: Uuid, reason: RejectReason) {
        out.messages.push(Message::JobRejected { job_id, reason });
    }

    fn on_message(&mut self, message: Message, peer: &Peer, out: &mut Output) {
        if self.takes_as_request(&message, peer, out) {
            return;
        }
        match message {
            Message::GetStatus => out.messages.push(self.status()),
            Message::SubmitJob {
                job_id,
                name,
                kind,
                bytes,
                sha256,
            } => self.submit(job_id, name, kind, (bytes, sha256), &peer.name, out),
            Message::MoveJob { .. } | Message::RemoveJob { .. } | Message::SetRunNext { .. } => {
                self.list_message(&message, peer, out);
            }
            Message::JobChunk {
                job_id,
                index,
                text,
            } => self.chunk(job_id, index, &text, out),
            Message::Start { job_id } => self.start(job_id, peer, out),
            Message::Pause | Message::Resume | Message::Stop => self.control(&message, out),
            // Outside `Alarm` nothing is written; the caller answers with the status (G33).
            Message::Unlock if self.state == State::Alarm => {
                out.commands.push(ControllerCommand::Unlock);
            }
            Message::Unlock => {}
            Message::Home => self.home(out),
            _ => Self::error(
                out,
                ErrorCode::UnexpectedMessage,
                "The connector does not accept this message.",
            ),
        }
    }
}
