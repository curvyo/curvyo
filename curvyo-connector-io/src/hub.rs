//! The state shared by all connections of one connector: the job machine, the fake controller,
//! the pairing code, the paired apps and the fan-out of status messages, under one lock.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ControllerCommand, Event, JobResult, Machine, Message, Options, State,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::clients::{Clients, ClientsError};
use crate::fake_controller::FakeController;
use crate::job_log::{JobLog, LogResult};
use crate::pairing_code::{PairingCodes, Password, Refusal};
use crate::spki::SpkiHash;

/// Settings of the fake controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeSettings {
    /// Lines per second it streams.
    pub lines_per_second: u32,
    /// The line at which it raises an alarm.
    pub alarm_at_line: Option<u32>,
}

/// What the hub is made of.
#[derive(Debug, Clone)]
pub struct HubSettings {
    /// The state directory (clients list and job log).
    pub state_dir: PathBuf,
    /// The fake controller, if one is attached.
    pub fake: Option<FakeSettings>,
    /// `Start` waits for the connector's own terminal.
    pub require_local_start: bool,
    /// The name `Welcome` carries.
    pub connector_name: String,
    /// Hash of the connector's own key.
    pub spki: SpkiHash,
}

struct Inner {
    machine: Machine,
    fake: Option<FakeController>,
    codes: PairingCodes,
    subscribers: Vec<Sender<Message>>,
    job_name: String,
    submitted: Option<(Uuid, String)>,
    notices: Sender<String>,
}

/// The shared state of one connector.
pub struct Hub {
    inner: Mutex<Inner>,
    started: Instant,
    clients: Clients,
    job_log: JobLog,
    settings: HubSettings,
}

impl Hub {
    /// A hub, and the receiver of its terminal notices (one line each).
    #[must_use]
    pub fn new(settings: HubSettings, started: Instant) -> (Self, Receiver<String>) {
        let (notices, receiver) = channel();
        let mut machine = Machine::new(Options {
            require_local_start: settings.require_local_start,
        });
        let fake = settings
            .fake
            .map(|f| FakeController::new(f.lines_per_second, f.alarm_at_line));
        if fake.is_some() {
            machine.handle(Event::ControllerConnected);
        }
        let hub = Self {
            inner: Mutex::new(Inner {
                machine,
                fake,
                codes: PairingCodes::default(),
                subscribers: Vec::new(),
                job_name: String::new(),
                submitted: None,
                notices,
            }),
            started,
            clients: Clients::new(&settings.state_dir),
            job_log: JobLog::new(&settings.state_dir),
            settings,
        };
        (hub, receiver)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Hash of the connector's key.
    #[must_use]
    pub fn spki(&self) -> SpkiHash {
        self.settings.spki
    }

    /// The `Welcome` for a greeted client.
    #[must_use]
    pub fn welcome(&self) -> Message {
        Message::Welcome {
            connector_name: self.settings.connector_name.clone(),
            controller: self.settings.fake.map(|_| FakeController::info()),
        }
    }

    /// The current `Status`.
    #[must_use]
    pub fn status(&self) -> Message {
        self.lock().machine.status()
    }

    /// Statuses of every change from now on; drop the receiver to unsubscribe.
    pub fn subscribe(&self) -> Receiver<Message> {
        let (sender, receiver) = channel();
        self.lock().subscribers.push(sender);
        receiver
    }

    /// Whether `token` belongs to a paired app. The list is read from the file every time.
    #[must_use]
    pub fn accepts(&self, token: &str) -> bool {
        self.clients.accepts(token).unwrap_or(false)
    }

    /// Whether any app is paired.
    ///
    /// # Errors
    /// [`ClientsError`] for a damaged list.
    pub fn has_clients(&self) -> Result<bool, ClientsError> {
        Ok(!self.clients.list()?.is_empty())
    }

    /// A message of a paired app; the answers for that app alone (statuses go to everyone
    /// subscribed, `GetStatus` is answered directly).
    pub fn handle_message(&self, message: Message, now: Instant) -> Vec<Message> {
        if message == Message::GetStatus {
            return vec![self.status()];
        }
        let mut inner = self.lock();
        let mut direct = Vec::new();
        self.apply(&mut inner, Event::Message(message), now, &mut direct);
        direct
    }

    /// A connection that sent a job ended: a job still being received is dropped.
    pub fn client_disconnected(&self, now: Instant) {
        let mut inner = self.lock();
        self.apply(&mut inner, Event::ClientDisconnected, now, &mut Vec::new());
    }

    /// Lets the fake controller run up to `now`.
    pub fn tick(&self, now: Instant) {
        let mut inner = self.lock();
        let at = now.saturating_duration_since(self.started);
        let events = inner
            .fake
            .as_mut()
            .map(|f| f.advance(at))
            .unwrap_or_default();
        let mut ignored = Vec::new();
        for event in events {
            self.apply(&mut inner, event, now, &mut ignored);
        }
    }

    /// The press of the start button on the connector's host; the job's name if it started.
    pub fn local_start(&self, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        if inner.machine.state() != State::AwaitingLocalStart {
            return None;
        }
        let name = inner.job_name.clone();
        self.apply(&mut inner, Event::LocalStartPressed, now, &mut Vec::new());
        let _ = inner.notices.send(format!("Started {name}"));
        Some(name)
    }

    /// Issues a new code, replacing a valid one.
    pub fn issue_code(&self, now: Instant) -> Option<String> {
        self.lock().codes.issue(now)
    }

    /// The valid code as printed.
    #[must_use]
    pub fn current_code(&self, now: Instant) -> Option<String> {
        self.lock().codes.current(now)
    }

    /// Begins a pairing attempt.
    ///
    /// # Errors
    /// [`Refusal`] when there is no valid code or an exchange is running.
    pub fn begin_pair(&self, now: Instant) -> Result<Password, Refusal> {
        self.lock().codes.begin(now)
    }

    /// The running attempt failed or was abandoned: the attempts left.
    pub fn pair_failed(&self) -> u8 {
        self.lock().codes.failed()
    }

    /// The running attempt verified: the code is spent and the app is stored.
    ///
    /// # Errors
    /// [`ClientsError`] when the list cannot be written; the code stays spent.
    pub fn pair_succeeded(&self, name: &str, token_sha256: &str) -> Result<(), ClientsError> {
        self.lock().codes.succeeded();
        self.clients
            .add(name, token_sha256, OffsetDateTime::now_utc())
    }

    fn apply(&self, inner: &mut Inner, event: Event, now: Instant, direct: &mut Vec<Message>) {
        if let Event::Message(Message::SubmitJob { job_id, name, .. }) = &event {
            inner.submitted = Some((*job_id, name.clone()));
        }
        let before = inner.machine.state();
        let output = inner.machine.handle(event);
        let at = now.saturating_duration_since(self.started);
        let lines = inner
            .machine
            .held_job()
            .map_or(0, |j| j.text.lines().count());
        let lines = u32::try_from(lines).unwrap_or(u32::MAX);
        for command in &output.commands {
            if let Some(fake) = inner.fake.as_mut() {
                fake.command(command, lines, at);
            }
            if matches!(command, ControllerCommand::Start { .. }) {
                inner.job_name = inner
                    .machine
                    .held_job()
                    .map(|j| j.name.clone())
                    .unwrap_or_default();
            }
        }
        for message in output.messages {
            match message {
                Message::Status { .. } => inner
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
                    let name = inner
                        .submitted
                        .as_ref()
                        .filter(|(id, _)| *id == job_id)
                        .map(|(_, name)| name.clone())
                        .unwrap_or_default();
                    self.log(inner, &name, &LogResult::Rejected(format!("{reason:?}")));
                    direct.push(Message::JobRejected { job_id, reason });
                }
                other => direct.push(other),
            }
        }
        self.log_outcome(inner, before);
    }

    fn log_outcome(&self, inner: &mut Inner, before: State) {
        let after = inner.machine.state();
        if after == before {
            return;
        }
        let was_active = matches!(
            before,
            State::Running | State::Paused | State::AwaitingLocalStart
        );
        let result = match after {
            State::Done { result } => Some(match result {
                JobResult::Completed => LogResult::Completed,
                JobResult::Stopped => LogResult::Stopped,
                JobResult::Failed => LogResult::Failed,
            }),
            State::Alarm if was_active => match inner.machine.status() {
                Message::Status { alarm, .. } => Some(LogResult::Alarm(alarm.unwrap_or_default())),
                _ => None,
            },
            _ => None,
        };
        if let Some(result) = result {
            let name = inner.job_name.clone();
            self.log(inner, &name, &result);
        }
        if after == State::AwaitingLocalStart {
            let _ = inner
                .notices
                .send(format!("Press Enter to start {}", inner.job_name));
        }
    }

    fn log(&self, inner: &Inner, name: &str, result: &LogResult) {
        let at = OffsetDateTime::now_utc();
        if let Err(e) = self.job_log.append(at, name, result) {
            let _ = inner.notices.send(format!("Cannot write the job log: {e}"));
        }
        let text = result.text();
        let _ = inner.notices.send(format!("Job {name}: {text}"));
    }
}

/// The streamer's tick period.
pub const TICK: Duration = Duration::from_millis(50);
