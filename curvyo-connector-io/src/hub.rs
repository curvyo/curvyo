//! The state shared by all connections of one connector, under one lock: the job machine, its
//! controller, the mode, the pairing code, the paired clients and the fan-out of status messages.

mod journal;
mod link;
mod motion;
mod owner;
mod pairing;
mod setup;

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ControllerCommand, ControllerInfo, Event, Machine, Message, Options, Peer, StartRule, State,
};
use uuid::Uuid;

use crate::admin_log::{AdminEvent, AdminLog};
use crate::clients::Clients;
use crate::controller::Controller;
use crate::grbl::Timing;
use crate::job_log::{JobLog, LogPeople, LogResult};
use crate::job_store::{self, JobStore};
use crate::pairing_code::PairingCodes;
use crate::spki::SpkiHash;

use link::{fake_of, serial_port_of};
pub use setup::{ExitReason, HubMode};

/// Settings of the fake controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeSettings {
    /// Lines per second it streams.
    pub lines_per_second: u32,
    /// The line at which it raises an alarm.
    pub alarm_at_line: Option<u32>,
}

/// The serial port of a GRBL controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialSettings {
    /// The port, as given on the command line.
    pub port: String,
    /// Its baud rate.
    pub baud: u32,
    /// The limits (the tests shorten them).
    pub timing: Timing,
}

/// Which controller the connector has; one choice, so the flags cannot mix (addendum C2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerChoice {
    /// None: the state stays `NoController`.
    None,
    /// The fake controller.
    Fake(FakeSettings),
    /// GRBL on a serial port.
    Serial(SerialSettings),
}

/// What the hub is made of.
#[derive(Debug, Clone)]
pub struct HubSettings {
    /// The state directory (clients list and job log).
    pub state_dir: PathBuf,
    /// The controller.
    pub controller: ControllerChoice,
    /// `Start` waits for the connector's own terminal (foreground mode).
    pub require_local_start: bool,
    /// The name `Welcome` carries without a configuration (the host name).
    pub connector_name: String,
    /// Hash of the connector's own key.
    pub spki: SpkiHash,
}

/// One authenticated connection as the hub knows it: its client's name and its close flag.
struct SessionEntry {
    name: String,
    close: Weak<AtomicBool>,
}

pub(crate) struct Inner {
    pub(crate) machine: Machine,
    controller: Option<Box<dyn Controller>>,
    info: Option<ControllerInfo>,
    codes: PairingCodes,
    subscribers: Vec<Sender<Message>>,
    pub(crate) job_name: String,
    /// The jobs announced lately, by id, for the log line of a refusal.
    submitted: Vec<(Uuid, String)>,
    /// The job list's files in the state directory.
    store: JobStore,
    /// The texts on disk or on their way.
    stored_ids: HashSet<Uuid>,
    /// `list.json` as last written, and when its line was.
    last_stored: Option<(curvyo_connector_core::StoredList, Duration)>,
    /// The job list is not written any more (the connector is stopping a running job).
    frozen: bool,
    pub(crate) notices: Sender<String>,
    /// Bytes the controller produced outside the serial thread's turn.
    pending_write: Vec<u8>,
    mode: HubMode,
    /// Counts the switches out of maintenance mode; a maintenance session closes when it moves.
    epoch: u64,
    sessions: Vec<SessionEntry>,
    /// Where a controller change goes: the supervisor of the controller thread.
    supervisor: Option<Sender<curvyo_connector_core::ControllerConfig>>,
    /// Who configured the connector, from which address (A7).
    configured_by: Option<(String, std::net::IpAddr)>,
    /// The terminal was told that the setup window closed.
    window_told: bool,
    /// The serial port the current controller choice names; a serial thread of another port
    /// (one a new configuration replaced) attaches and loses nothing.
    serial_port: Option<String>,
}

/// One turn of the serial thread: the bytes to write and whether to close the port.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exchange {
    /// Bytes for the port.
    pub write: Vec<u8>,
    /// Close the port and open it again later.
    pub close: bool,
}

/// The shared state of one connector.
pub struct Hub {
    inner: Mutex<Inner>,
    started: Instant,
    clients: Clients,
    job_log: JobLog,
    admin_log: AdminLog,
    /// The client whose jogs the last `jobs.log` line named; any other line clears it.
    jog_run: Mutex<Option<String>>,
    settings: HubSettings,
    exit: Mutex<Option<Sender<ExitReason>>>,
}

/// A refused owner or setup call: the code and the sentence of its `Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The code.
    pub code: curvyo_connector_core::ErrorCode,
    /// The sentence.
    pub text: String,
}

impl Refused {
    /// The refusal in the access table's fixed words.
    #[must_use]
    pub fn fixed(code: curvyo_connector_core::ErrorCode) -> Self {
        match curvyo_connector_core::refusal(code) {
            Message::Error { code, message } => Self {
                code,
                text: message,
            },
            _ => Self {
                code,
                text: String::new(),
            },
        }
    }

    /// The `Error` message.
    #[must_use]
    pub fn message(self) -> Message {
        Message::Error {
            code: self.code,
            message: self.text,
        }
    }
}

/// The machine for a controller choice: a serial controller confirms Pause, Resume and Stop.
fn machine_for(choice: &ControllerChoice, start_rule: StartRule) -> Machine {
    let options = Options { start_rule };
    match choice {
        ControllerChoice::Serial(_) => Machine::confirming(options),
        _ => Machine::new(options),
    }
}

impl Hub {
    /// A hub in foreground mode, and the receiver of its terminal notices (one line each).
    #[must_use]
    pub fn new(settings: HubSettings, started: Instant) -> (Self, Receiver<String>) {
        let (notices, receiver) = channel();
        let rule = if settings.require_local_start {
            StartRule::LocalPress
        } else {
            StartRule::Immediate
        };
        let mut machine = machine_for(&settings.controller, rule);
        let controller = fake_of(&settings.controller);
        if controller.is_some() {
            machine.handle(Event::ControllerConnected);
        }
        let info = controller.as_ref().and_then(|c| c.info());
        let loaded = job_store::load(&settings.state_dir);
        machine.restore(loaded.restored.items.clone());
        let hub = Self {
            inner: Mutex::new(Inner {
                machine,
                controller,
                info,
                codes: PairingCodes::default(),
                subscribers: Vec::new(),
                job_name: String::new(),
                submitted: Vec::new(),
                store: JobStore::new(&settings.state_dir),
                stored_ids: HashSet::new(),
                last_stored: None,
                frozen: false,
                notices,
                pending_write: Vec::new(),
                mode: HubMode::Foreground,
                epoch: 0,
                sessions: Vec::new(),
                supervisor: None,
                configured_by: None,
                window_told: false,
                serial_port: serial_port_of(&settings.controller),
            }),
            started,
            clients: Clients::new(&settings.state_dir),
            job_log: JobLog::new(&settings.state_dir),
            admin_log: AdminLog::new(&settings.state_dir),
            jog_run: Mutex::new(None),
            settings,
            exit: Mutex::new(None),
        };
        hub.log_restore(&loaded);
        {
            let mut inner = hub.lock();
            inner.stored_ids = inner
                .machine
                .stored_texts()
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            // What a restart found is on disk already; nothing is written until it changes.
            inner.last_stored = Some((inner.machine.stored(), Duration::ZERO));
        }
        (hub, receiver)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn at(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    /// Hash of the connector's key.
    #[must_use]
    pub fn spki(&self) -> SpkiHash {
        self.settings.spki
    }

    /// The current `Status`.
    #[must_use]
    pub fn status(&self) -> Message {
        self.lock().machine.status()
    }

    /// The current job state.
    #[must_use]
    pub fn state(&self) -> State {
        self.lock().machine.state()
    }

    /// Statuses of every change from now on, and a `Welcome` when the controller changes; drop
    /// the receiver to unsubscribe.
    pub fn subscribe(&self) -> Receiver<Message> {
        let (sender, receiver) = channel();
        self.lock().subscribers.push(sender);
        receiver
    }

    /// A message of a paired client `peer`; the answers for that client alone (statuses go to
    /// everyone subscribed, `GetStatus` is answered directly).
    pub fn handle_message(&self, message: Message, peer: &Peer, now: Instant) -> Vec<Message> {
        let mut inner = self.lock();
        // A stale controller is judged before the command, so no command acts on a lost link
        // and no answer shows a dead link as `Running` (G15).
        self.turn(&mut inner, &[], now);
        if message == Message::GetStatus {
            return vec![inner.machine.status()];
        }
        let mut direct = Vec::new();
        self.apply(
            &mut inner,
            Event::Message(message, peer.clone()),
            now,
            &mut direct,
        );
        direct
    }

    /// `Unlock` from the client `peer` (G33): `$X` goes to the controller only in `Alarm`; the
    /// answer is the status. Logged in `jobs.log` (G33) and `admin.log` (A18).
    pub fn unlock(&self, peer: &Peer, now: Instant) -> Message {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
        if inner.machine.state() == State::Alarm {
            let client = peer.name.as_str();
            let _ = inner.notices.send(format!("Unlock sent by {client}"));
            let result = LogResult::Unlock(client.to_owned());
            self.append_job_line(&inner, "", &result, &LogPeople::default());
            self.append_admin(&inner, client, &AdminEvent::Unlocked);
        }
        self.apply(
            &mut inner,
            Event::Message(Message::Unlock, peer.clone()),
            now,
            &mut Vec::new(),
        );
        inner.machine.status()
    }

    /// A connection that sent the jobs `job_ids` ended: those still being received are dropped,
    /// no other.
    pub fn client_disconnected(&self, job_ids: &[Uuid], now: Instant) {
        let mut inner = self.lock();
        for job_id in job_ids {
            let event = Event::TransferAbandoned { job_id: *job_id };
            self.apply(&mut inner, event, now, &mut Vec::new());
        }
    }

    /// The `JobList` as a connection gets it before its first `Status`.
    #[must_use]
    pub fn job_list(&self) -> Message {
        self.lock().machine.job_list()
    }

    /// A line for the connector's terminal.
    pub fn notice(&self, line: String) {
        let _ = self.lock().notices.send(line);
    }

    /// Before a wipe: everything queued for the job store is on disk and nothing is written after
    /// it, so the deletion is the last word on the directory.
    pub fn freeze_store(&self) {
        let mut inner = self.lock();
        inner.frozen = true;
        inner.store.flush();
    }

    /// Ends the terminal's notices: the receiver reads what was sent, then its end.
    pub fn close_notices(&self) {
        self.lock().notices = channel().0;
    }

    /// Stops a running or paused job as a client's Stop would (G16, A19); the job's name if one
    /// ran. A paused job whose resume waits for an owner counts as paused.
    pub fn stop_for_exit(&self, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        let paused = inner.machine.waits_for_resume();
        if !paused && !matches!(inner.machine.state(), State::Running | State::Paused) {
            return None;
        }
        let name = inner.job_name.clone();
        // The job is written as interrupted at its line, then the list stays as it is.
        inner.sync_store(self.at(now), true);
        inner.store.flush();
        inner.frozen = true;
        self.apply(
            &mut inner,
            Event::Message(Message::Stop, Peer::host()),
            now,
            &mut Vec::new(),
        );
        Some(name)
    }

    /// The press of the start button on the connector's host; the job's name if it started.
    pub fn local_start(&self, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        if inner.machine.state() != State::AwaitingLocalStart {
            return None;
        }
        let homing = matches!(inner.machine.status(), Message::Status { job: None, .. });
        let name = if homing {
            "Home".to_owned()
        } else {
            inner.job_name.clone()
        };
        self.apply(&mut inner, Event::LocalStartPressed, now, &mut Vec::new());
        if inner.machine.state() == State::AwaitingLocalStart {
            return None;
        }
        let _ = inner.notices.send(format!("Started {name}"));
        Some(name)
    }

    /// Applies `event` and every event the controller answers its commands with, in order.
    fn apply(&self, inner: &mut Inner, event: Event, now: Instant, direct: &mut Vec<Message>) {
        let at = self.at(now);
        let mut queue = VecDeque::from([event]);
        while let Some(event) = queue.pop_front() {
            if let Event::Message(Message::SubmitJob { job_id, name, .. }, _) = &event {
                inner.submitted.retain(|(id, _)| id != job_id);
                inner.submitted.push((*job_id, name.clone()));
                if inner.submitted.len() > 64 {
                    inner.submitted.remove(0);
                }
            }

            let before = inner.machine.state();
            let people = inner.machine.people().cloned();
            let output = inner.machine.handle(event);
            for command in &output.commands {
                if matches!(command, ControllerCommand::Start { .. }) {
                    inner.job_name = inner
                        .machine
                        .held_job()
                        .map(|j| j.name.clone())
                        .unwrap_or_default();
                }
                let job = inner.machine.held_job();
                if let Some(controller) = inner.controller.as_mut() {
                    // Its bytes go out with the next turn of the serial thread.
                    queue.extend(controller.command(command, job, at));
                }
            }
            if let Some(confirmation) = &output.confirmed {
                self.log_confirmation(inner, confirmation);
            }
            for log in &output.list_log {
                self.log_list_change(inner, log);
            }
            for message in output.messages {
                self.route(inner, message, direct);
            }
            self.log_outcome(inner, before, people);
            if let Some(next) = output.follow_up {
                queue.push_front(next);
            }
            inner.sync_store(at, false);
        }
    }

    fn route(&self, inner: &mut Inner, message: Message, direct: &mut Vec<Message>) {
        match message {
            Message::Status { .. } | Message::JobList { .. } => inner
                .subscribers
                .retain(|s| s.send(message.clone()).is_ok()),
            Message::JobAccepted { .. } => {
                inner.job_name = inner
                    .machine
                    .held_job()
                    .map(|j| j.name.clone())
                    .unwrap_or_default();
                direct.push(message);
            }
            Message::JobRejected { job_id, reason } => {
                self.log_rejection(inner, job_id, reason);
                direct.push(Message::JobRejected { job_id, reason });
            }
            other => direct.push(other),
        }
    }
}

/// The streamer's tick period.
pub const TICK: Duration = Duration::from_millis(50);
