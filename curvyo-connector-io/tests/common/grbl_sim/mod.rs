//! A GRBL 1.1 simulator for the tests and the `grbl_pty` example (`specs/0127` G22): a 128-byte
//! receive buffer that records an overflow as a fault, `ok` and `error:<n>`, status reports in
//! work or machine coordinates, the real-time bytes `?`, `!`, `~`, `0x18` and `0x9E` anywhere in
//! the stream, the banner, `$I`, `$$`, homing, spindle and air, alarms, a planner that keeps
//! moving after the last `ok`, the `Bf:` buffer field, a build that leaves the laser on in a hold,
//! the door, and silent, silent-after-Stop and garbage modes. Plain struct, test support only: its
//! time is a counter its driver steps, never the wall clock.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::struct_excessive_bools,
    clippy::single_match_else
)]

mod gcode;
mod report;

use std::collections::{BTreeMap, VecDeque};

/// GRBL's receive buffer.
pub const RX_SIZE: usize = 128;

/// What the simulated machine is like.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// The banner's version.
    pub version: &'static str,
    /// Steps one motion takes.
    pub steps_per_motion: u32,
    /// Steps from `!` to `Hold:0` while moving.
    pub decel_steps: u32,
    /// Steps a homing cycle takes.
    pub homing_steps: u32,
    /// Blocks the planner holds.
    pub planner_size: usize,
    /// The received line (counted from 1) that GRBL answers with `error:<code>`.
    pub error_on_line: Option<(u32, u16)>,
    /// `$10=1`: reports carry `MPos` and now and then `WCO`; otherwise `WPos`.
    pub mpos_reports: bool,
    /// `$22=1`: homing on, locked at power-up.
    pub homing_lock: bool,
    /// `$32`.
    pub laser_mode: bool,
    /// The work coordinate offset.
    pub wco: [f64; 3],
    /// `$10` asks for `Bf:` (planner blocks and receive bytes free) in every report.
    pub buffer_reports: bool,
    /// Built without `DISABLE_LASER_DURING_HOLD`: with `$32=1` the laser stays on in a hold.
    pub laser_on_in_hold: bool,
    /// GRBL 1.1f's homing loop: no `?` is answered while it runs; one `?` received meanwhile
    /// is answered (`<Home|...>`) when the cycle ends, before its `ok` (`specs/0129`).
    pub quiet_homing: bool,
    /// The homing cycle ends in this alarm (8 pull-off, 9 switch not found): `ALARM:<n>`, the
    /// `ok` of `$H`, then GRBL restarts itself locked (banner and unlock message).
    pub homing_fails: Option<u16>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            version: "1.1h",
            steps_per_motion: 3,
            decel_steps: 2,
            homing_steps: 50,
            planner_size: 15,
            error_on_line: None,
            mpos_reports: false,
            homing_lock: false,
            laser_mode: true,
            wco: [0.0; 3],
            buffer_reports: false,
            laser_on_in_hold: false,
            quiet_homing: false,
            homing_fails: None,
        }
    }
}

/// The spindle (the laser).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spindle {
    Off,
    Cw,
    Ccw,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Block {
    Move { target: [f64; 3], steps: u32 },
    Spindle(Spindle, f64),
    Air(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hold {
    complete: bool,
    decel: u32,
}

/// The simulator.
#[derive(Debug, Clone)]
pub struct GrblSim {
    pub config: SimConfig,
    rx: Vec<u8>,
    /// Faults the connector caused: an overflow, a real-time byte inside a line.
    pub faults: Vec<String>,
    out: Vec<u8>,
    /// Every byte received.
    pub log: Vec<u8>,
    /// Every line received, in order, without its newline.
    pub lines: Vec<String>,
    planner: VecDeque<Block>,
    parse_wait: u32,
    pub(crate) settings: BTreeMap<u16, String>,
    pub spindle: Spindle,
    pub power: f64,
    pub air: bool,
    spindle_stop: bool,
    pub(crate) pos: [f64; 3],
    /// Where the planner's last block ends.
    pub(crate) planned: [f64; 3],
    pub(crate) relative: bool,
    pub alarm: bool,
    critical: bool,
    homing: Option<u32>,
    /// A `?` came during a quiet homing cycle.
    report_owed: bool,
    hold: Option<Hold>,
    reports: u32,
    /// Answers nothing and takes nothing (a dead controller on a live port).
    pub silent: bool,
    /// Prints other text instead of a banner (not GRBL).
    pub garbage: bool,
    /// Goes silent when it receives the feed hold `!` (dies at the Stop).
    pub silent_on_hold: bool,
    /// The safety door is open: the state reads `Door:0`.
    pub door: bool,
    /// Steps since the start.
    pub clock: u64,
    /// The clock at the last status report.
    pub last_report: Option<u64>,
}

impl GrblSim {
    /// A machine just powered up.
    #[must_use]
    pub fn new(config: SimConfig) -> Self {
        let mut settings = BTreeMap::new();
        for (n, v) in [
            (0, "10"),
            (1, "25"),
            (
                10,
                match (config.mpos_reports, config.buffer_reports) {
                    (false, false) => "0",
                    (true, false) => "1",
                    (false, true) => "2",
                    (true, true) => "3",
                },
            ),
            (13, "0"),
            (22, if config.homing_lock { "1" } else { "0" }),
            (30, "1000"),
            (31, "0"),
            (32, if config.laser_mode { "1" } else { "0" }),
            (130, "400.000"),
            (131, "300.000"),
            (132, "0.000"),
        ] {
            settings.insert(n, v.to_owned());
        }
        Self {
            alarm: config.homing_lock,
            config,
            rx: Vec::new(),
            faults: Vec::new(),
            out: Vec::new(),
            log: Vec::new(),
            lines: Vec::new(),
            planner: VecDeque::new(),
            parse_wait: 0,
            settings,
            spindle: Spindle::Off,
            power: 0.0,
            air: false,
            spindle_stop: false,
            pos: [0.0; 3],
            planned: [0.0; 3],
            relative: false,
            critical: false,
            homing: None,
            report_owed: false,
            hold: None,
            reports: 0,
            silent: false,
            garbage: false,
            silent_on_hold: false,
            door: false,
            clock: 0,
            last_report: None,
        }
    }

    /// Sets a `$` setting as the maker would have.
    pub fn set_setting(&mut self, number: u16, value: &str) {
        self.settings.insert(number, value.to_owned());
        if number == 32 {
            self.config.laser_mode = value != "0";
        }
    }

    pub(crate) fn print(&mut self, text: &str) {
        self.out.extend_from_slice(text.as_bytes());
        self.out.extend_from_slice(b"\r\n");
    }

    fn partial_line(&self) -> bool {
        self.rx.last().is_some_and(|b| *b != b'\n')
    }

    fn moving(&self) -> bool {
        self.homing.is_some()
            || (!self.planner.is_empty() && !self.hold.is_some_and(|h| h.complete))
    }

    /// Bytes from the connector.
    pub fn receive(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.log.push(b);
            if self.silent {
                continue;
            }
            if b == b'!' && self.silent_on_hold {
                self.silent = true;
                continue;
            }
            if matches!(b, b'?' | b'!' | b'~' | 0x18 | 0x9E) && self.partial_line() {
                self.faults
                    .push(format!("real-time byte {b:#04x} inside a line"));
            }
            match b {
                b'?' if self.homing.is_some() && self.config.quiet_homing => {
                    self.report_owed = true;
                }
                b'?' => self.report(),
                b'!' => self.feed_hold(),
                b'~' => self.cycle_start(),
                0x18 => self.reset(),
                0x9E => {
                    if self.hold.is_some_and(|h| h.complete) {
                        self.spindle_stop = !self.spindle_stop;
                    }
                }
                _ if self.rx.len() >= RX_SIZE => {
                    self.faults.push(format!(
                        "receive buffer overflow at byte {}",
                        self.log.len()
                    ));
                }
                _ => self.rx.push(b),
            }
        }
    }

    fn feed_hold(&mut self) {
        if self.homing.is_some() || self.alarm || self.hold.is_some() {
            return;
        }
        let moving = !self.planner.is_empty();
        self.hold = Some(Hold {
            complete: !moving,
            decel: self.config.decel_steps,
        });
    }

    fn cycle_start(&mut self) {
        if self.hold.is_some_and(|h| h.complete) {
            self.hold = None;
            self.spindle_stop = false;
        }
    }

    /// A soft reset, from `0x18` or the machine's reset button.
    pub fn reset(&mut self) {
        let raised = if self.homing.is_some() {
            Some(6)
        } else if self.moving() {
            Some(3)
        } else {
            None
        };
        self.rx.clear();
        self.planner.clear();
        self.planned = self.pos;
        self.hold = None;
        self.homing = None;
        self.report_owed = false;
        self.spindle = Spindle::Off;
        self.power = 0.0;
        self.air = false;
        self.spindle_stop = false;
        self.parse_wait = 0;
        self.critical = false;
        if let Some(code) = raised {
            self.alarm = true;
            self.print(&format!("ALARM:{code}"));
        }
        if self.garbage {
            self.print("AT+CGMI");
            self.print("NO CARRIER");
            return;
        }
        self.print("");
        let banner = format!("Grbl {} ['$' for help]", self.config.version);
        self.print(&banner);
        if self.alarm {
            self.print("[MSG:'$H'|'$X' to unlock]");
        }
    }

    /// An alarm as the machine raises it (`1` hard limit, `2` soft limit need a reset).
    pub fn inject_alarm(&mut self, code: u16) {
        self.planner.clear();
        self.planned = self.pos;
        self.rx.clear();
        self.hold = None;
        self.spindle = Spindle::Off;
        self.air = false;
        self.alarm = true;
        self.print(&format!("ALARM:{code}"));
        if code <= 2 {
            self.critical = true;
            self.print("[MSG:Reset to continue]");
        }
    }

    /// An `ok` nobody asked for (another program on the port).
    pub fn inject_ok(&mut self) {
        self.print("ok");
    }

    /// Bytes waiting in the receive buffer.
    #[must_use]
    pub fn rx_used(&self) -> usize {
        self.rx.len()
    }

    /// Whether the planner still holds blocks.
    #[must_use]
    pub fn planner_busy(&self) -> bool {
        !self.planner.is_empty()
    }

    /// One step of the machine's time; the bytes it printed.
    pub fn step(&mut self) -> Vec<u8> {
        self.clock += 1;
        if self.silent {
            self.out.clear();
            return Vec::new();
        }
        self.run_planner();
        self.parse_next();
        std::mem::take(&mut self.out)
    }

    fn run_planner(&mut self) {
        if let Some(n) = self.homing.as_mut() {
            *n = n.saturating_sub(1);
            if *n == 0 {
                if std::mem::take(&mut self.report_owed) {
                    self.report();
                }
                self.homing = None;
                if let Some(code) = self.config.homing_fails {
                    self.alarm = true;
                    self.print(&format!("ALARM:{code}"));
                    self.print("ok");
                    self.rx.clear();
                    let banner = format!("Grbl {} ['$' for help]", self.config.version);
                    self.print("");
                    self.print(&banner);
                    self.print("[MSG:'$H'|'$X' to unlock]");
                    return;
                }
                self.alarm = false;
                self.pos = [0.0; 3];
                self.planned = self.pos;
                self.print("ok");
            }
            return;
        }
        if let Some(h) = self.hold.as_mut() {
            if !h.complete {
                h.decel = h.decel.saturating_sub(1);
                h.complete = h.decel == 0;
            }
            return;
        }
        if self.alarm {
            return;
        }
        while let Some(front) = self.planner.front_mut() {
            match front {
                Block::Move { target, steps } => {
                    *steps = steps.saturating_sub(1);
                    if *steps == 0 {
                        self.pos = *target;
                        self.planner.pop_front();
                    }
                    break;
                }
                Block::Spindle(s, p) => {
                    (self.spindle, self.power) = (*s, *p);
                    self.planner.pop_front();
                }
                Block::Air(on) => {
                    self.air = *on;
                    self.planner.pop_front();
                }
            }
        }
    }

    fn parse_next(&mut self) {
        if self.parse_wait > 0 {
            self.parse_wait -= 1;
            return;
        }
        if self.homing.is_some() || self.planner.len() >= self.config.planner_size {
            return;
        }
        let Some(end) = self.rx.iter().position(|b| *b == b'\n') else {
            return;
        };
        let line: Vec<u8> = self.rx.drain(..=end).collect();
        let line = String::from_utf8_lossy(&line[..end])
            .trim_end_matches('\r')
            .to_owned();
        if self.critical || self.garbage {
            return;
        }
        self.lines.push(line.clone());
        let n = u32::try_from(self.lines.len()).unwrap();
        // A varying, line-count-driven delay before the next line is taken (G6).
        self.parse_wait = n % 3;
        self.execute(n, &line);
    }

    pub(crate) fn push_block(&mut self, block: Block) {
        self.planner.push_back(block);
    }

    pub(crate) fn start_homing(&mut self) -> bool {
        if self.settings.get(&22).is_some_and(|v| v == "1") {
            self.homing = Some(self.config.homing_steps);
            true
        } else {
            false
        }
    }
}
