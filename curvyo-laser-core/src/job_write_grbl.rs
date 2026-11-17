//! The GRBL G-code bytes of a job (ADR 0064 §7, §12; criteria 24 to 26, 38): a pure function of
//! the job, so the same job always gives the same bytes.

use std::fmt::Write as _;

use crate::job_input::PointUm;
use crate::job_moves::{Job, Move, MoveKind};
use crate::operation_settings::Role;

/// The longest comment line, in characters (Proposal).
const MAX_COMMENT: usize = 80;

/// The GRBL G-code of `job`, ASCII with LF line ends.
#[must_use]
pub fn write_grbl(job: &Job) -> Vec<u8> {
    let mut out = Writer::new(job);
    out.header();
    let (last, body) = match job.moves.split_last() {
        Some((last, body)) if last.kind == MoveKind::Travel => (Some(last), body),
        _ => (None, job.moves.as_slice()),
    };
    for m in body {
        out.step(m);
    }
    out.line("M5");
    out.laser_on = false;
    if job.machine.air_assist {
        out.line("M9");
    }
    if let Some(last) = last {
        out.travel(last.to);
    }
    out.text.into_bytes()
}

#[allow(clippy::struct_excessive_bools)]
// four independent modal states of the controller
struct Writer<'a> {
    job: &'a Job,
    text: String,
    relative: bool,
    /// The last written machine point (the job's home at the start).
    at: PointUm,
    z: i64,
    feed: Option<u32>,
    s: u32,
    m3: bool,
    laser_on: bool,
    air_on: bool,
}

impl<'a> Writer<'a> {
    fn new(job: &'a Job) -> Self {
        Self {
            job,
            text: String::new(),
            relative: job.start.relative(),
            at: job.frame.machine_point(job.home),
            z: 0,
            feed: None,
            s: 0,
            m3: !job.machine.laser_mode,
            laser_on: false,
            air_on: false,
        }
    }

    fn line(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }

    fn comment(&mut self, text: &str) {
        let ascii: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_graphic() || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .take(MAX_COMMENT - 2)
            .collect();
        let line = format!("; {ascii}");
        self.line(line.trim_end());
    }

    fn header(&mut self) {
        let job = self.job;
        self.comment(&format!("Curvyo job: {}", job.name));
        self.comment(&format!("Machine: {}", job.machine.name));
        for (index, block) in job.blocks.iter().enumerate() {
            self.comment(&format!("{} {}", index + 1, block.name));
        }
        if job.start.homes() {
            self.line("$H");
        }
        self.line("G21");
        self.line(if self.relative { "G91" } else { "G90" });
        self.line("G17");
        self.line("M5");
        if self.m3 {
            self.line("M3 S0");
            self.laser_on = true;
        } else {
            self.line("M4 S0");
        }
    }

    /// The `X.. Y..` words of a move to page point `to`, absolute or relative.
    fn xy(&mut self, to: PointUm) -> String {
        let machine = self.job.frame.machine_point(to);
        let shown = if self.relative {
            PointUm::new(machine.x - self.at.x, machine.y - self.at.y)
        } else {
            machine
        };
        self.at = machine;
        format!("X{} Y{}", coordinate(shown.x), coordinate(shown.y))
    }

    fn feed_word(&mut self, feed: u32, always: bool) -> String {
        if always || self.feed != Some(feed) {
            self.feed = Some(feed);
            format!(" F{feed}")
        } else {
            String::new()
        }
    }

    fn s_word(&mut self, s: u32) -> String {
        if self.s == s {
            String::new()
        } else {
            self.s = s;
            format!(" S{s}")
        }
    }

    fn travel(&mut self, to: PointUm) {
        if self.m3 && self.laser_on {
            self.line("M5");
            self.laser_on = false;
        }
        let xy = self.xy(to);
        self.line(&format!("G0 {xy}"));
    }

    fn step(&mut self, m: &Move) {
        let Some(block) = self.job.blocks.get(m.block) else {
            if m.kind == MoveKind::Travel {
                self.travel(m.to);
            }
            return;
        };
        match m.kind {
            MoveKind::Travel => self.travel(m.to),
            MoveKind::Burn { role, .. } => {
                if self.job.machine.air_assist && !self.air_on {
                    self.line("M8");
                    self.air_on = true;
                }
                let numbers = block.of(role);
                let s = s_value(numbers.power.tenths(), self.job.machine.max_power.get());
                if self.m3 && !self.laser_on {
                    self.line(&format!("M3 S{s}"));
                    self.s = s;
                    self.laser_on = true;
                }
                let xy = self.xy(m.to);
                let f = self.feed_word(numbers.feed.get(), false);
                let sw = self.s_word(s);
                self.line(&format!("G1 {xy}{f}{sw}"));
            }
            MoveKind::RunOut(role) => {
                let xy = self.xy(m.to);
                let f = self.feed_word(block.of(role).feed.get(), false);
                let sw = self.s_word(0);
                self.line(&format!("G1 {xy}{f}{sw}"));
            }
            MoveKind::Z(z) => {
                let shown = if self.relative { z - self.z } else { z };
                self.z = z;
                let f = self.feed_word(block.of(Role::Main).feed.get(), true);
                let sw = self.s_word(0);
                self.line(&format!("G1 Z{}{f}{sw}", coordinate(shown)));
            }
        }
    }
}

/// GRBL's S of a power in tenths of a percent: `round(power × S-max / 100)` in integers.
#[must_use]
pub fn s_value(power_tenths: u16, max_power: u16) -> u32 {
    (u32::from(power_tenths) * u32::from(max_power) + 500) / 1000
}

/// `um` as millimetres with three decimals, never `-0.000`.
#[must_use]
pub fn coordinate(um: i64) -> String {
    let sign = if um < 0 { "-" } else { "" };
    let abs = um.unsigned_abs();
    let mut out = String::with_capacity(12);
    let _ = write!(out, "{sign}{}.{:03}", abs / 1000, abs % 1000);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_have_three_decimals_and_no_negative_zero() {
        assert_eq!(coordinate(0), "0.000");
        assert_eq!(coordinate(1_250), "1.250");
        assert_eq!(coordinate(-5), "-0.005");
        assert_eq!(coordinate(-12_000), "-12.000");
    }

    #[test]
    fn s_is_the_rounded_share_of_s_max() {
        assert_eq!(s_value(1000, 1000), 1000);
        assert_eq!(s_value(355, 1000), 355);
        assert_eq!(s_value(355, 255), 91, "35.5 % of 255 = 90.5, rounds up");
        assert_eq!(s_value(10, 1), 0);
    }
}
