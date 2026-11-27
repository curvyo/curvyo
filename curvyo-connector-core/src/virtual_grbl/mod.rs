//! A GRBL 1.1 controller in pure Rust (`specs/0161-cnc-output` criteria 15 to 18, `adrs.md`
//! CN5): it takes the bytes a sender writes and the time the caller lets pass, and returns what
//! GRBL would print. A 128-byte character-counting receive buffer, a 15-block planner, `ok` and
//! `error:<n>`, status reports, feed hold, resume, soft reset, `$` settings, homing, alarms and
//! the moves it ran with their times. No I/O and no clock of its own: it builds for wasm32.

pub mod codes;
mod parse;
mod planner;
mod settings;
mod status;
mod system;

use std::collections::VecDeque;

pub use parse::{Action, Modal, Spindle, parse_line};
pub use planner::{Block, PlanTime, Seconds, block, durations, junction_max_sq, plan_time};
pub use settings::GrblSettings;

use crate::gcode_line::{MAX_LINE, strip};

/// GRBL's receive buffer in bytes.
pub const RX_SIZE: usize = 128;
/// The blocks GRBL's planner holds (`[OPT:V,15,128]`).
pub const PLANNER_BLOCKS: usize = 15;
/// The version the banner names.
pub const VERSION: &str = "1.1h";
/// How long the virtual homing cycle takes.
pub const HOMING_SECONDS: f64 = 1.0;

/// What runs now.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Running {
    from: [f64; 3],
    block: Block,
    jog: bool,
    duration: f64,
    elapsed: f64,
    exit_sq: f64,
}

/// One move the virtual GRBL finished (criterion 17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Executed {
    /// Where it ended (machine mm).
    pub target: [f64; 3],
    /// Whether the spindle turned while it ran.
    pub spindle_on: bool,
    /// When it ended, on the machine's run clock.
    pub at: Seconds,
}

/// The virtual GRBL 1.1.
#[derive(Debug, Clone)]
pub struct VirtualGrbl {
    settings: GrblSettings,
    rx: Vec<u8>,
    out: Vec<u8>,
    modal: Modal,
    /// The parser's position: where the last queued block ends.
    planned: [f64; 3],
    /// The queued moves and whether each is a jog.
    queue: VecDeque<(Block, bool)>,
    running: Option<Running>,
    pos: [f64; 3],
    speed_sq: f64,
    spindle: (Spindle, f64),
    locked: bool,
    critical: bool,
    /// Seconds left until a feed hold is complete.
    hold: Option<f64>,
    homing: Option<f64>,
    dwell: Option<f64>,
    reports: u32,
    busy: f64,
    /// Every move finished, in order.
    pub executed: Vec<Executed>,
    /// What a sender did wrong: an overflowed buffer, a real-time byte inside a line.
    pub faults: Vec<String>,
    /// Every line received, without its newline.
    pub lines: Vec<String>,
}

impl VirtualGrbl {
    /// A machine just powered up with `settings`: it prints its banner, locked when homing is
    /// on.
    #[must_use]
    pub fn new(settings: GrblSettings) -> Self {
        let mut grbl = Self {
            locked: settings.homing(),
            settings,
            rx: Vec::new(),
            out: Vec::new(),
            modal: Modal::default(),
            planned: [0.0; 3],
            queue: VecDeque::new(),
            running: None,
            pos: [0.0; 3],
            speed_sq: 0.0,
            spindle: (Spindle::Off, 0.0),
            critical: false,
            hold: None,
            homing: None,
            dwell: None,
            reports: 0,
            busy: 0.0,
            executed: Vec::new(),
            faults: Vec::new(),
            lines: Vec::new(),
        };
        grbl.banner();
        grbl
    }

    /// The settings.
    #[must_use]
    pub fn settings(&self) -> &GrblSettings {
        &self.settings
    }

    /// The head's machine position.
    #[must_use]
    pub fn position(&self) -> [f64; 3] {
        status::position_of(self)
    }

    /// The spindle as it stands.
    #[must_use]
    pub fn spindle(&self) -> (Spindle, f64) {
        self.spindle
    }

    /// Whether the machine is in an alarm.
    #[must_use]
    pub fn alarmed(&self) -> bool {
        self.locked || self.critical
    }

    /// The seconds the machine spent moving or dwelling.
    #[must_use]
    pub fn busy_time(&self) -> Seconds {
        Seconds::new(self.busy)
    }

    /// Whether moves are queued or running.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.running.is_some() || !self.queue.is_empty() || self.dwell.is_some()
    }

    fn print(&mut self, text: &str) {
        self.out.extend_from_slice(text.as_bytes());
        self.out.extend_from_slice(b"\r\n");
    }

    fn banner(&mut self) {
        self.print("");
        self.print(&format!("Grbl {VERSION} ['$' for help]"));
        if self.locked {
            self.print("[MSG:'$H'|'$X' to unlock]");
        }
    }

    /// Takes what GRBL printed since the last call.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Bytes from the sender: real-time bytes act at once, the rest fills the receive buffer.
    pub fn receive(&mut self, bytes: &[u8]) {
        for &b in bytes {
            let partial = self.rx.last().is_some_and(|last| *last != b'\n');
            if matches!(b, b'?' | b'!' | b'~' | 0x18 | 0x85) && partial {
                self.faults
                    .push(format!("real-time byte {b:#04x} inside a line"));
            }
            match b {
                b'?' => status::report(self),
                b'!' => self.feed_hold(),
                b'~' => self.cycle_start(),
                0x18 => self.reset(),
                0x85 => self.jog_cancel(),
                _ if self.rx.len() >= RX_SIZE => {
                    self.faults.push("receive buffer overflow".to_owned());
                }
                _ => self.rx.push(b),
            }
        }
    }

    fn feed_hold(&mut self) {
        if self.hold.is_none() && self.homing.is_none() && self.moving() {
            let accel = self.running.map_or(1.0, |r| r.block.accel);
            self.hold = Some(self.speed_now() / accel);
        }
    }

    fn cycle_start(&mut self) {
        if self.hold.is_some_and(|left| left <= 0.0) {
            self.hold = None;
        }
    }

    fn jog_cancel(&mut self) {
        if self.running.is_some_and(|r| r.jog) {
            self.pos = status::position_of(self);
            self.running = None;
            self.queue.clear();
            self.planned = self.pos;
            self.speed_sq = 0.0;
        }
    }

    /// A soft reset: everything stops; moving loses position (`ALARM:3`).
    pub fn reset(&mut self) {
        let raised = if self.homing.is_some() {
            Some(6)
        } else if self.moving() && !self.running.is_some_and(|r| r.jog) {
            Some(codes::ALARM_RESET_IN_MOTION)
        } else {
            None
        };
        self.pos = status::position_of(self);
        self.rx.clear();
        self.queue.clear();
        self.running = None;
        self.planned = self.pos;
        self.speed_sq = 0.0;
        self.hold = None;
        self.homing = None;
        self.dwell = None;
        self.spindle = (Spindle::Off, 0.0);
        self.modal = Modal::default();
        self.critical = false;
        if let Some(code) = raised {
            self.locked = true;
            self.print(&format!("ALARM:{code}"));
        }
        self.banner();
    }

    fn alarm(&mut self, code: u16) {
        self.pos = status::position_of(self);
        self.queue.clear();
        self.running = None;
        self.planned = self.pos;
        self.spindle = (Spindle::Off, 0.0);
        self.critical = true;
        self.print(&format!("ALARM:{code}"));
        self.print("[MSG:Reset to continue]");
    }

    fn speed_now(&self) -> f64 {
        self.running.map_or(0.0, |r| r.block.nominal)
    }

    fn planner_free(&self) -> bool {
        self.queue.len() + usize::from(self.running.is_some()) < PLANNER_BLOCKS
    }

    /// Lets `seconds` pass: lines are taken while the planner has room, the moves run.
    pub fn advance(&mut self, seconds: f64) -> Vec<u8> {
        self.parse_ready();
        let mut left = seconds.max(0.0);
        while left > 0.0 && self.run_for(&mut left) {
            self.parse_ready();
        }
        self.parse_ready();
        self.take_output()
    }

    /// Takes every complete line the planner has room for.
    fn parse_ready(&mut self) {
        while self.homing.is_none() && self.dwell.is_none() && self.planner_free() {
            let Some(end) = self.rx.iter().position(|b| *b == b'\n') else {
                return;
            };
            let raw: Vec<u8> = self.rx.drain(..=end).collect();
            let text = String::from_utf8_lossy(&raw[..end])
                .trim_end_matches('\r')
                .to_owned();
            self.lines.push(text.clone());
            let line = strip(&text).to_ascii_uppercase();
            if !self.execute(&line) {
                // A line that waits for the planner to run empty goes back to the front.
                let mut back = text.into_bytes();
                back.push(b'\n');
                self.rx.splice(0..0, back);
                self.lines.pop();
                return;
            }
        }
    }

    /// Answers one line; `false` when it must wait until the planner is empty.
    fn execute(&mut self, line: &str) -> bool {
        if line.is_empty() {
            self.print("ok");
            return true;
        }
        if line.len() > MAX_LINE {
            self.print(&format!("error:{}", codes::LINE_OVERFLOW));
            return true;
        }
        if line.starts_with('$') {
            return system::run(self, line);
        }
        if self.alarmed() {
            self.print(&format!("error:{}", codes::GCODE_LOCK));
            return true;
        }
        let mut modal = self.modal;
        match parse_line(line, &mut modal, self.planned) {
            Err(code) => {
                self.print(&format!("error:{code}"));
                true
            }
            Ok(actions) => self.queue_actions(&actions, modal),
        }
    }

    fn queue_actions(&mut self, actions: &[Action], modal: Modal) -> bool {
        let syncs = actions
            .iter()
            .any(|a| matches!(a, Action::Spindle(..) | Action::Dwell(_)));
        if syncs && self.moving() {
            return false;
        }
        for action in actions {
            match *action {
                Action::Spindle(s, rpm) => self.spindle = (s, rpm),
                Action::Dwell(s) => self.dwell = Some(s.max(0.0)),
                Action::Move {
                    target,
                    rapid,
                    feed_mm_min,
                } => {
                    if self.outside_soft_limits(target) {
                        self.modal = modal;
                        self.alarm(codes::ALARM_SOFT_LIMIT);
                        return true;
                    }
                    if let Some(b) = block(self.planned, target, rapid, feed_mm_min, &self.settings)
                    {
                        self.queue.push_back((b, false));
                    }
                    self.planned = target;
                }
            }
        }
        self.modal = modal;
        self.print("ok");
        true
    }

    /// Whether `target` lies outside GRBL's soft-limit box `[-travel, 0]` with `$20=1`.
    fn outside_soft_limits(&self, target: [f64; 3]) -> bool {
        self.settings.soft_limits()
            && (0..3).any(|i| target[i] > 1e-6 || target[i] < -self.settings.travel(i) - 1e-6)
    }

    /// Runs the machine for up to `left` seconds; `false` when nothing more happens now.
    fn run_for(&mut self, left: &mut f64) -> bool {
        if self.critical || self.locked && self.homing.is_none() {
            return false;
        }
        if let Some(h) = self.homing.as_mut() {
            let step = left.min(*h);
            *h -= step;
            *left -= step;
            if *h <= 0.0 {
                self.homing = None;
                self.locked = false;
                self.pos = [0.0; 3];
                self.planned = self.pos;
                self.print("ok");
            }
            return true;
        }
        if let Some(h) = self.hold.as_mut() {
            *h -= *left;
            *left = 0.0;
            return false;
        }
        if let Some(d) = self.dwell.as_mut() {
            let step = left.min(*d);
            *d -= step;
            *left -= step;
            self.busy += step;
            if *d <= 0.0 {
                self.dwell = None;
            }
            return true;
        }
        if self.running.is_none() && !self.start_next() {
            self.speed_sq = 0.0;
            return false;
        }
        let Some(r) = self.running.as_mut() else {
            return false;
        };
        let step = left.min(r.duration - r.elapsed);
        r.elapsed += step;
        *left -= step;
        self.busy += step;
        if r.elapsed >= r.duration {
            let done = *r;
            self.running = None;
            self.pos = done.block.target;
            self.speed_sq = done.exit_sq;
            self.executed.push(Executed {
                target: done.block.target,
                spindle_on: self.spindle.0 != Spindle::Off,
                at: Seconds::new(self.busy),
            });
        }
        true
    }

    /// Starts the queue's front block, planned over the queued moves (GRBL replans each block).
    fn start_next(&mut self) -> bool {
        let Some((first, jog)) = self.queue.front().copied() else {
            return false;
        };
        let blocks: Vec<Block> = self.queue.iter().map(|q| q.0).collect();
        let plan = durations(&blocks, self.speed_sq, &self.settings);
        self.queue.pop_front();
        let (duration, exit_sq) = plan.first().copied().unwrap_or((0.0, 0.0));
        self.running = Some(Running {
            from: self.pos,
            block: first,
            jog,
            duration,
            elapsed: 0.0,
            exit_sq,
        });
        true
    }
}

#[cfg(test)]
mod tests;
