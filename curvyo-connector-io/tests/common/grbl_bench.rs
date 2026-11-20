//! The GRBL controller, the job machine and the simulator in one loop over an in-memory byte
//! channel, with stepped time (`specs/0127` G22): every status the machine pushes is recorded
//! with the age of the newest status report the controller had then.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::format_collect
)]

use curvyo_connector_core::Peer;
use std::time::Duration;

use curvyo_connector_core::{Event, JobKind, Machine, Message, Options, State};
use curvyo_connector_io::controller::Controller;
use curvyo_connector_io::grbl::{GrblController, Timing};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::grbl_sim::{GrblSim, SimConfig};

/// The port name the bench reports.
pub const PORT: &str = "/dev/ttyBench";
/// One step of simulated time.
pub const DT: Duration = Duration::from_millis(10);

/// One status the machine pushed.
#[derive(Debug, Clone)]
pub struct Pushed {
    pub at: Duration,
    pub status: Message,
    /// Time since the controller last read a status report; `None` before the first.
    pub report_age: Option<Duration>,
}

pub struct Bench {
    pub sim: GrblSim,
    pub ctrl: GrblController,
    pub machine: Machine,
    pub at: Duration,
    pub pushed: Vec<Pushed>,
    pub notices: Vec<String>,
    pub closed: Option<String>,
    /// The bytes the controller wrote when it gave the link up.
    pub last_write: Vec<u8>,
    /// False once the cable is pulled: nothing goes either way.
    pub plugged: bool,
    last_report: Option<Duration>,
    job: u128,
}

pub fn job_id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Bench {
    pub fn new(config: SimConfig) -> Self {
        Self::with(config, Options::default())
    }

    /// A bench whose controller uses `timing` (a short homing cap, `specs/0142`).
    pub fn with_timing(config: SimConfig, timing: Timing) -> Self {
        let mut bench = Self::new(config);
        bench.ctrl = GrblController::new(PORT, 115_200, timing, Duration::ZERO);
        bench
    }

    pub fn with(config: SimConfig, options: Options) -> Self {
        Self {
            sim: GrblSim::new(config),
            ctrl: GrblController::new(PORT, 115_200, Timing::default(), Duration::ZERO),
            machine: Machine::confirming(options),
            at: Duration::ZERO,
            pushed: Vec::new(),
            notices: Vec::new(),
            closed: None,
            last_write: Vec::new(),
            plugged: true,
            last_report: None,
            job: 0,
        }
    }

    /// Identified, `Idle` (or `Alarm`).
    pub fn connected(config: SimConfig) -> Self {
        let mut bench = Self::new(config);
        assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
        bench
    }

    pub fn state(&self) -> State {
        self.machine.state()
    }

    pub fn status(&self) -> Message {
        self.machine.status()
    }

    pub fn detail(&self) -> Option<String> {
        match self.machine.status() {
            Message::Status { detail, .. } => detail,
            _ => None,
        }
    }

    pub fn alarm(&self) -> Option<String> {
        match self.machine.status() {
            Message::Status { alarm, .. } => alarm,
            _ => None,
        }
    }

    pub fn line(&self) -> u32 {
        match self.machine.status() {
            Message::Status { job: Some(j), .. } => j.line,
            _ => 0,
        }
    }

    fn record(&mut self, messages: Vec<Message>) {
        for status in messages {
            if matches!(status, Message::Status { .. }) {
                let report_age = self.last_report.map(|r| self.at.saturating_sub(r));
                self.pushed.push(Pushed {
                    at: self.at,
                    status,
                    report_age,
                });
            }
        }
    }

    /// Applies `event` and the controller's answers to its commands.
    pub fn apply(&mut self, event: Event) {
        let out = self.machine.handle(event);
        self.record(out.messages);
        for command in out.commands {
            let events = self
                .ctrl
                .command(&command, self.machine.held_job(), self.at);
            for e in events {
                self.apply(e);
            }
        }
    }

    pub fn app(&mut self, message: Message) {
        self.apply(Event::Message(message, Peer::host()));
    }

    /// Sends `text` as a job and waits for `Ready`.
    pub fn send(&mut self, text: &str) -> Uuid {
        self.job += 1;
        let id = job_id(self.job);
        self.app(Message::SubmitJob {
            job_id: id,
            name: "bench.gcode".into(),
            kind: JobKind::Job,
            bytes: text.len() as u64,
            sha256: sha256_hex(text),
        });
        let mut index = 0;
        let mut rest = text;
        while !rest.is_empty() {
            let (part, tail) = rest.split_at(rest.len().min(60_000));
            self.app(Message::JobChunk {
                job_id: id,
                index,
                text: part.to_owned(),
            });
            index += 1;
            rest = tail;
        }
        assert_eq!(self.state(), State::Ready);
        id
    }

    /// Sends and starts `text`.
    pub fn start(&mut self, text: &str) {
        let id = self.send(text);
        self.app(Message::Start { job_id: id });
    }

    /// One step of time for both sides.
    pub fn step(&mut self) {
        self.at += DT;
        let from_sim = if self.plugged {
            self.sim.step()
        } else {
            Vec::new()
        };
        if from_sim.contains(&b'<') {
            self.last_report = Some(self.at);
        }
        let advance = self.ctrl.advance(&from_sim, self.at);
        if self.plugged {
            self.sim.receive(&advance.write);
        }
        self.notices.extend(advance.notices);
        for event in advance.events {
            self.apply(event);
        }
        if let Some(reason) = advance.close {
            self.closed = Some(reason);
            self.last_write = advance.write;
            self.apply(Event::ControllerLost);
        }
    }

    /// Steps until `done` or `max` steps; whether `done` came.
    pub fn run_until(&mut self, done: impl Fn(&Self) -> bool, max: usize) -> bool {
        for _ in 0..max {
            if done(self) {
                return true;
            }
            self.step();
        }
        done(self)
    }

    /// Steps for `time`.
    pub fn run_for(&mut self, time: Duration) {
        let end = self.at + time;
        while self.at < end {
            self.step();
        }
    }

    /// Whether a status was pushed that shows `Running` or `Paused` with no report younger than
    /// `limit`.
    pub fn stale_running_push(&self, limit: Duration) -> Option<&Pushed> {
        self.pushed.iter().find(|p| {
            matches!(
                p.status,
                Message::Status {
                    state: State::Running | State::Paused,
                    ..
                }
            ) && p.report_age.is_none_or(|age| age >= limit)
        })
    }

    /// The bytes the simulator received after byte `from`, without real-time bytes.
    pub fn lines_after(&self, from: usize) -> String {
        String::from_utf8_lossy(
            &self.sim.log[from..]
                .iter()
                .copied()
                .filter(|b| !matches!(b, b'?' | b'!' | b'~' | 0x18 | 0x9E))
                .collect::<Vec<u8>>(),
        )
        .into_owned()
    }
}
