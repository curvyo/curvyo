//! Helpers shared by the tests: fixtures, a hand-written job and the fake controller.
#![allow(dead_code, clippy::unwrap_used)]

use curvyo_connector_core::Peer;
use curvyo_connector_core::{
    ControllerCommand, Event, JobKind, Machine, Message, Options, Output, State,
};
use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const JOB_TEXT: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

pub fn job_id() -> Uuid {
    "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap()
}

pub fn fixture(path: &str) -> String {
    let full = format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(full).unwrap()
}

pub fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

pub fn submit(text: &str, kind: JobKind) -> Message {
    Message::SubmitJob {
        job_id: job_id(),
        name: "coaster.gcode".to_owned(),
        kind,
        bytes: text.len() as u64,
        sha256: sha256_hex(text),
    }
}

pub fn chunk(index: u32, text: &str) -> Message {
    Message::JobChunk {
        job_id: job_id(),
        index,
        text: text.to_owned(),
    }
}

/// The machine side as the tests see it: it receives commands and produces controller events.
#[derive(Debug, Default)]
pub struct FakeController {
    pub plugged_in: bool,
    pub streaming: Option<Uuid>,
    pub paused: bool,
    pub line: u32,
}

impl FakeController {
    pub fn plug_in(&mut self) -> Event {
        self.plugged_in = true;
        Event::ControllerConnected
    }

    pub fn unplug(&mut self) -> Event {
        self.plugged_in = false;
        self.streaming = None;
        Event::ControllerLost
    }

    /// Takes the commands the machine produced, as a real controller would.
    pub fn obey(&mut self, out: &Output) {
        for command in &out.commands {
            match command {
                ControllerCommand::Start { job_id } | ControllerCommand::StartAt { job_id, .. } => {
                    self.streaming = Some(*job_id);
                    self.line = 0;
                    self.paused = false;
                }
                ControllerCommand::Pause => self.paused = true,
                ControllerCommand::Resume => self.paused = false,
                ControllerCommand::Stop => self.streaming = None,
                ControllerCommand::Unlock
                | ControllerCommand::Home
                | ControllerCommand::Frame { .. }
                | ControllerCommand::Jog { .. }
                | ControllerCommand::JogHold { .. }
                | ControllerCommand::JogCancel
                | ControllerCommand::GoTo { .. }
                | ControllerCommand::Pulse { .. } => {}
            }
        }
    }

    pub fn advance(&mut self, lines: u32) -> Event {
        self.line += lines;
        Event::Progress {
            line: self.line,
            elapsed_ms: u64::from(self.line) * 250,
        }
    }

    pub fn finish(&mut self) -> Event {
        self.streaming = None;
        Event::JobFinished { elapsed_ms: 1500 }
    }

    pub fn fail(&mut self) -> Event {
        self.streaming = None;
        Event::JobFailed { elapsed_ms: 900 }
    }

    pub fn alarm(&mut self, text: &str) -> Event {
        self.streaming = None;
        Event::Alarm {
            text: text.to_owned(),
        }
    }
}

/// A machine with a controller and the driver around it.
pub struct Rig {
    pub machine: Machine,
    pub controller: FakeController,
}

impl Rig {
    pub fn new(options: Options) -> Self {
        Self {
            machine: Machine::new(options),
            controller: FakeController::default(),
        }
    }

    pub fn send(&mut self, event: Event) -> Output {
        let out = self.machine.handle(event);
        self.controller.obey(&out);
        out
    }

    pub fn app(&mut self, message: Message) -> Output {
        self.send(Event::Message(message, Peer::host()))
    }

    pub fn plugged() -> Self {
        let mut rig = Self::new(Options::default());
        let e = rig.controller.plug_in();
        rig.send(e);
        rig
    }

    /// Submits `JOB_TEXT` in two chunks; ends in `Ready`.
    pub fn ready(options: Options) -> Self {
        let mut rig = Self::new(options);
        let e = rig.controller.plug_in();
        rig.send(e);
        rig.app(submit(JOB_TEXT, JobKind::Job));
        let (a, b) = JOB_TEXT.split_at(20);
        rig.app(chunk(0, a));
        rig.app(chunk(1, b));
        assert_eq!(rig.machine.state(), State::Ready);
        rig
    }
}

pub fn last_status(out: &Output) -> &Message {
    out.messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::Status { .. }))
        .unwrap()
}
