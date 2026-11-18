//! The state shared by all connections of one connector: the job machine, its controller, the
//! pairing code, the paired apps and the fan-out of status messages, under one lock.

mod pairing;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ControllerCommand, ControllerInfo, Event, Machine, Message, Options, State,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::clients::Clients;
use crate::controller::Controller;
use crate::fake_controller::FakeController;
use crate::grbl::{GrblController, Timing};
use crate::hub_log::{logged_outcome, rejection_line, terminal_alarm};
use crate::job_log::{JobLog, LogResult};
use crate::pairing_code::PairingCodes;
use crate::spki::SpkiHash;

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
    /// `Start` waits for the connector's own terminal.
    pub require_local_start: bool,
    /// The name `Welcome` carries.
    pub connector_name: String,
    /// Hash of the connector's own key.
    pub spki: SpkiHash,
}

pub(crate) struct Inner {
    pub(crate) machine: Machine,
    controller: Option<Box<dyn Controller>>,
    info: Option<ControllerInfo>,
    codes: PairingCodes,
    subscribers: Vec<Sender<Message>>,
    pub(crate) job_name: String,
    submitted: Option<(Uuid, String)>,
    pub(crate) notices: Sender<String>,
    /// Bytes the controller produced outside the serial thread's turn.
    pending_write: Vec<u8>,
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
    settings: HubSettings,
}

impl Hub {
    /// A hub, and the receiver of its terminal notices (one line each).
    #[must_use]
    pub fn new(settings: HubSettings, started: Instant) -> (Self, Receiver<String>) {
        let (notices, receiver) = channel();
        let options = Options {
            require_local_start: settings.require_local_start,
        };
        let mut machine = match settings.controller {
            ControllerChoice::Serial(_) => Machine::confirming(options),
            _ => Machine::new(options),
        };
        let mut controller: Option<Box<dyn Controller>> = None;
        if let ControllerChoice::Fake(f) = settings.controller {
            controller = Some(Box::new(FakeController::new(
                f.lines_per_second,
                f.alarm_at_line,
            )));
            machine.handle(Event::ControllerConnected);
        }
        let info = controller.as_ref().and_then(|c| c.info());
        let hub = Self {
            inner: Mutex::new(Inner {
                machine,
                controller,
                info,
                codes: PairingCodes::default(),
                subscribers: Vec::new(),
                job_name: String::new(),
                submitted: None,
                notices,
                pending_write: Vec::new(),
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

    fn at(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    /// Hash of the connector's key.
    #[must_use]
    pub fn spki(&self) -> SpkiHash {
        self.settings.spki
    }

    fn welcome_of(&self, info: Option<ControllerInfo>) -> Message {
        Message::Welcome {
            connector_name: self.settings.connector_name.clone(),
            controller: info,
        }
    }

    /// The `Welcome` for a greeted client.
    #[must_use]
    pub fn welcome(&self) -> Message {
        let info = self.lock().info.clone();
        self.welcome_of(info)
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

    /// `Unlock` from the app `client` (G33): `$X` goes to the controller only in `Alarm`; the
    /// answer is the status.
    pub fn unlock(&self, client: &str, now: Instant) -> Message {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
        if inner.machine.state() == State::Alarm {
            let _ = inner.notices.send(format!("Unlock sent by {client}"));
            let at = OffsetDateTime::now_utc();
            let result = LogResult::Unlock(client.to_owned());
            if let Err(e) = self.job_log.append(at, "", &result) {
                let _ = inner.notices.send(format!("Cannot write the job log: {e}"));
            }
        }
        self.apply(
            &mut inner,
            Event::Message(Message::Unlock),
            now,
            &mut Vec::new(),
        );
        inner.machine.status()
    }

    /// A message of a paired app; the answers for that app alone (statuses go to everyone
    /// subscribed, `GetStatus` is answered directly).
    pub fn handle_message(&self, message: Message, now: Instant) -> Vec<Message> {
        let mut inner = self.lock();
        // A stale controller is judged before the command, so no command acts on a lost link
        // and no answer shows a dead link as `Running` (G15).
        self.turn(&mut inner, &[], now);
        if message == Message::GetStatus {
            return vec![inner.machine.status()];
        }
        let mut direct = Vec::new();
        self.apply(&mut inner, Event::Message(message), now, &mut direct);
        direct
    }

    /// A connection that sent a job ended: a job still being received is dropped.
    pub fn client_disconnected(&self, now: Instant) {
        let mut inner = self.lock();
        self.apply(&mut inner, Event::ClientDisconnected, now, &mut Vec::new());
    }

    /// Lets the controller run up to `now` with nothing read (the fake controller's pace).
    pub fn tick(&self, now: Instant) {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
    }

    /// One turn of the serial thread: `read` from the port; what to write and whether to close.
    pub fn exchange(&self, read: &[u8], now: Instant) -> Exchange {
        let mut inner = self.lock();
        let close = self.turn(&mut inner, read, now);
        Exchange {
            write: std::mem::take(&mut inner.pending_write),
            close,
        }
    }

    /// Runs the controller once; true when it gave the link up.
    fn turn(&self, inner: &mut Inner, read: &[u8], now: Instant) -> bool {
        let at = self.at(now);
        let Some(controller) = inner.controller.as_mut() else {
            return false;
        };
        let advance = controller.advance(read, at);
        inner.pending_write.extend_from_slice(&advance.write);
        for notice in advance.notices {
            let _ = inner.notices.send(notice);
        }
        self.publish_info(inner);
        for event in advance.events {
            self.apply(inner, event, now, &mut Vec::new());
        }
        let Some(notice) = advance.close else {
            return false;
        };
        let _ = inner.notices.send(notice);
        self.drop_controller(inner, now);
        true
    }

    fn drop_controller(&self, inner: &mut Inner, now: Instant) {
        inner.controller = None;
        self.publish_info(inner);
        self.apply(inner, Event::ControllerLost, now, &mut Vec::new());
    }

    /// Sends a `Welcome` to every subscriber when the controller's info changed (addendum C6).
    fn publish_info(&self, inner: &mut Inner) {
        let info = inner.controller.as_ref().and_then(|c| c.info());
        if info != inner.info {
            inner.info.clone_from(&info);
            let welcome = self.welcome_of(info);
            inner
                .subscribers
                .retain(|s| s.send(welcome.clone()).is_ok());
        }
    }

    /// The serial thread opened the port at `now`: a fresh GRBL controller identifies (G3).
    pub fn attach_serial(&self, serial: &SerialSettings, now: Instant) {
        let controller =
            GrblController::new(&serial.port, serial.baud, serial.timing, self.at(now));
        self.lock().controller = Some(Box::new(controller));
    }

    /// The port failed (read or write error): the controller is lost (G15).
    pub fn lose(&self, notice: String, now: Instant) {
        let mut inner = self.lock();
        if inner.controller.is_some() {
            let _ = inner.notices.send(notice);
            inner.pending_write.clear();
            self.drop_controller(&mut inner, now);
        }
    }

    /// The port could not be opened: why, for the app (G2, G3).
    pub fn open_failed(&self, detail: String, now: Instant) {
        let mut inner = self.lock();
        self.apply(
            &mut inner,
            Event::NoControllerDetail { detail },
            now,
            &mut Vec::new(),
        );
    }

    /// A line for the connector's terminal.
    pub fn notice(&self, line: String) {
        let _ = self.lock().notices.send(line);
    }

    /// Ends the terminal's notices: the receiver reads what was sent, then its end.
    pub fn close_notices(&self) {
        self.lock().notices = channel().0;
    }

    /// Stops a running or paused job as a client's Stop would (G16); the job's name if one ran.
    pub fn stop_for_exit(&self, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        if !matches!(inner.machine.state(), State::Running | State::Paused) {
            return None;
        }
        let name = inner.job_name.clone();
        self.apply(
            &mut inner,
            Event::Message(Message::Stop),
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
        let name = inner.job_name.clone();
        self.apply(&mut inner, Event::LocalStartPressed, now, &mut Vec::new());
        let _ = inner.notices.send(format!("Started {name}"));
        Some(name)
    }

    /// Applies `event` and every event the controller answers its commands with, in order.
    fn apply(&self, inner: &mut Inner, event: Event, now: Instant, direct: &mut Vec<Message>) {
        let at = self.at(now);
        let mut queue = VecDeque::from([event]);
        while let Some(event) = queue.pop_front() {
            if let Event::Message(Message::SubmitJob { job_id, name, .. }) = &event {
                inner.submitted = Some((*job_id, name.clone()));
            }
            let before = inner.machine.state();
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
            for message in output.messages {
                self.route(inner, message, direct);
            }
            self.log_outcome(inner, before);
        }
    }

    fn route(&self, inner: &mut Inner, message: Message, direct: &mut Vec<Message>) {
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
                let result = LogResult::Rejected(format!("{reason:?}"));
                match rejection_line(reason) {
                    // The settings refusal is its own terminal line (UX notes section 10).
                    Some(line) => self.log_with_line(inner, &name, &result, line),
                    None => self.log(inner, &name, &result, None),
                }
                direct.push(Message::JobRejected { job_id, reason });
            }
            other => direct.push(other),
        }
    }

    fn log_outcome(&self, inner: &mut Inner, before: State) {
        let after = inner.machine.state();
        if after == before {
            return;
        }
        let status = inner.machine.status();
        if let Some((result, shown)) = logged_outcome(before, after, &status) {
            let name = inner.job_name.clone();
            self.log(inner, &name, &result, shown);
        } else if let Some(line) = terminal_alarm(before, after, &status) {
            let _ = inner.notices.send(line);
        }
        if after == State::AwaitingLocalStart {
            let _ = inner
                .notices
                .send(format!("Press Enter to start {}", inner.job_name));
        }
    }

    fn log(&self, inner: &Inner, name: &str, result: &LogResult, shown: Option<String>) {
        let text = shown.unwrap_or_else(|| result.text());
        self.log_with_line(inner, name, result, format!("Job {name}: {text}"));
    }

    /// Appends `result` to `jobs.log` and prints `line` on the terminal.
    fn log_with_line(&self, inner: &Inner, name: &str, result: &LogResult, line: String) {
        let at = OffsetDateTime::now_utc();
        if let Err(e) = self.job_log.append(at, name, result) {
            let _ = inner.notices.send(format!("Cannot write the job log: {e}"));
        }
        let _ = inner.notices.send(line);
    }
}

/// The streamer's tick period.
pub const TICK: Duration = Duration::from_millis(50);
