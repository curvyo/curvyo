//! The job state machine of one connector: events in, new state and messages out, no clock.

mod arrival;
mod confirm;
mod controls;
mod event;
mod frame;
mod home;
mod interrupt;
mod jog;
mod list;
mod restart;
mod store;

pub use confirm::{Confirmation, Confirms, Peer, People, StartRule};
pub use event::{ControllerCommand, Event};
pub use list::ListLog;

use std::time::Duration;

use uuid::Uuid;

use crate::assembly::{Assembly, Job};
pub(crate) use crate::count::count;
use crate::job_list::JobList;
use crate::message::{ErrorCode, JobKind, Message, Progress, RejectReason, State};
use crate::millimetres::Millimetres;
use frame::FrameRun;
use jog::JogRun;
use list::Transfer;
use restart::Plan;

/// Settings of the machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Who has to agree before a `Start` runs (architect addendum D5).
    pub start_rule: StartRule,
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
    /// A member's frame (`specs/0118` 2b, adrs F5); the frame is in its own slot.
    Frame,
    /// A restart from a line (`specs/0118` 2e, adrs R8).
    Restart,
}

#[derive(Debug, Clone)]
struct Held {
    job: Job,
    sha256: String,
    lines: u32,
    line: u32,
    elapsed_ms: u64,
    people: People,
    /// A restart from a line: the line it began at.
    from_line: Option<u32>,
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
    /// The frame that arrives, waits or runs, beside the held job (`specs/0118` 2b).
    frame: Option<FrameRun>,
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
    /// The state a homing or a jog started from (`Idle`, `Ready`, `Done` or `Alarm`), and returns to.
    motion_from: State,
    /// The controller has been homed since it was identified (`specs/0118` 13).
    homed: bool,
    /// The jog the controller runs.
    jog: Option<JogRun>,
    /// The last `Clock` of the caller, for the jog's dead-man.
    now: Duration,
    /// The head's machine position and the work offset, as the controller reported them.
    mpos: Option<[Millimetres; 3]>,
    wco: Option<[Millimetres; 3]>,
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
    /// GRBL's planner blocks, for the restart line (`specs/0118` 39).
    planner: u32,
    /// The restart each interrupted entry offers.
    plans: Vec<Plan>,
    /// A restart that has not reached its line: the entry it returns to the list.
    back: Option<crate::job_list::Item>,
    /// The restart moves the head to its line.
    entering: bool,
    /// Counts the breaks of trust in the position (reset, alarm, loss, reconnect).
    breaks: u64,
    /// Entries a Stop interrupted, with the break count then (`specs/0118` adrs R4, Frage 390 C).
    unbroken: Vec<(Uuid, u64)>,
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
            frame: None,
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
            motion_from: State::Idle,
            homed: false,
            jog: None,
            now: Duration::ZERO,
            mpos: None,
            wco: None,
            requester: None,
            sender: String::new(),
            list: JobList::default(),
            transfers: Vec::new(),
            note: None,
            last_list: None,
            planner: crate::restart::PLANNER_BLOCKS,
            plans: Vec::new(),
            back: None,
            entering: false,
            breaks: 0,
            unbroken: Vec::new(),
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
                | State::Jogging
                | State::Framing
                | State::Done { .. }
        );
        // A waiting Home or frame starts no job, so it names none.
        let waits_home = self.state == State::AwaitingLocalStart
            && matches!(self.waiting, Waiting::Home | Waiting::Frame);
        let job = self
            .held
            .as_ref()
            .filter(|_| carries_job && !waits_home)
            .map(|h| Progress {
                job_id: h.job.job_id,
                line: h.line,
                lines: h.lines,
                elapsed_ms: h.elapsed_ms,
                from_line: h.from_line,
                entering: self.entering,
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
            homed: self.homed,
            machine_position: self.mpos,
            work_offset: self.wco,
            frame_id: self.shown_frame_id(),
        }
    }

    /// Applies one event.
    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        if self.before_line(&event, &mut out) {
            self.announce_list(&mut out);
            return out;
        }
        match event {
            Event::Message(message, peer) => self.on_message(message, &peer, &mut out),
            Event::ClientDisconnected => {
                if self.state == State::Receiving {
                    self.discard();
                    self.set(State::Idle, &mut out);
                }
            }
            Event::Clock(now) => self.on_clock(now, &mut out),
            Event::TransferAbandoned { job_id } => self.abandon(job_id, &mut out),
            Event::RunNext { job_id } => self.run_next(job_id, &mut out),
            Event::Planner { blocks } => {
                if blocks > 0 && blocks != self.planner {
                    self.planner = blocks;
                    self.plan_all();
                }
            }
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
        if !matches!(
            state,
            State::Running | State::Paused | State::Homing | State::Jogging | State::Framing
        ) {
            self.stop_pending = false;
        }
        if state != State::Jogging {
            self.jog = None;
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
        self.frame = None;
        self.requester = None;
    }

    fn reject(out: &mut Output, job_id: Uuid, reason: RejectReason) {
        out.messages.push(Message::JobRejected { job_id, reason });
    }

    fn on_message(&mut self, message: Message, peer: &Peer, out: &mut Output) {
        if self.takes_as_request(&message, peer, out) || self.frame_refuses(&message, out) {
            return;
        }
        match message {
            Message::GetStatus => out.messages.push(self.status()),
            Message::SubmitJob {
                job_id,
                name,
                kind: JobKind::Frame,
                bytes,
                sha256,
            } => self.submit_frame((job_id, name), (bytes, sha256), peer, out),
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
            Message::RestartFromLine { job_id, line } => {
                self.restart_from_line(job_id, line, peer, out);
            }
            Message::Pause | Message::Resume | Message::Stop => self.control(&message, out),
            // Outside `Alarm` nothing is written; the caller answers with the status (G33).
            Message::Unlock if self.state == State::Alarm => {
                out.commands.push(ControllerCommand::Unlock);
            }
            Message::Unlock => {}
            Message::Home => self.home(peer, out),
            Message::Jog { .. } | Message::JogKeep | Message::JogStop => {
                self.jog_message(&message, out);
            }
            _ => Self::error(
                out,
                ErrorCode::UnexpectedMessage,
                "The connector does not accept this message.",
            ),
        }
    }
}
