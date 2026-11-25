//! The go to point jog and its pulse block for GRBL, from the command to Idle (`specs/0118` 2d,
//! adrs P5, P6): the target clamped into the travel, one absolute `$J=` with the laser off, and
//! the pulse block `G1 M3 S<s>` / `G4 P<seconds>` / `M5 S0` in one write that ends itself.

use std::time::Duration;

use curvyo_connector_core::{Event, Millimetres, PulseLength, PulsePower, clamp_target};

use crate::grbl::controller::GrblController;
use crate::grbl::home::Owed;
use crate::grbl::texts;

/// The pulse block that is in GRBL's hands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PulseRun {
    /// The sentence of a line GRBL refused before the block's end.
    failure: Option<String>,
}

/// A coordinate with three decimals and never `-0.000`.
fn coordinate(value: f64) -> String {
    let rounded = (value * 1_000.0).round() / 1_000.0;
    format!("{:.3}", if rounded == 0.0 { 0.0 } else { rounded })
}

/// The absolute jog to `target` (job file coordinates) at `feed` mm/min.
#[must_use]
pub fn go_to_line(target: [Millimetres; 2], feed: f64) -> String {
    format!(
        "$J=G90 G21 X{} Y{} F{feed:.0}\n",
        coordinate(target[0].0),
        coordinate(target[1].0)
    )
}

/// The `S` of a pulse of `power` per mille of `s_max` (`$30`), rounded down.
#[must_use]
pub fn pulse_s(power: PulsePower, s_max: u32) -> u64 {
    u64::from(power.get()) * u64::from(s_max) / 1_000
}

/// The three lines of the pulse block: the power at standstill under `G1`, the dwell in seconds
/// with three decimals (GRBL's `G4 P`), the beam off.
#[must_use]
pub fn pulse_block(s: u64, length: PulseLength) -> [String; 3] {
    let ms = length.get();
    [
        format!("G1 M3 S{s}\n"),
        format!("G4 P{}.{:03}\n", ms / 1_000, ms % 1_000),
        "M5 S0\n".to_owned(),
    ]
}

impl GrblController {
    /// `ControllerCommand::GoTo`: names the clamped target (`GoToTarget`) and writes the jog; a
    /// move that cannot be written ends at once (`JogEnded`).
    pub(super) fn go_to(&mut self, target: [Millimetres; 2], feed: f64, at: Duration) {
        let wco = if self.wco_seen {
            [Millimetres(self.wco[0]), Millimetres(self.wco[1])]
        } else {
            [Millimetres(0.0); 2]
        };
        let within = self.travel_box();
        let clamped = clamp_target(target, wco, within.as_ref());
        self.events.push(Event::GoToTarget { target: clamped });
        let feed = self.capped_both(feed);
        let distance = self
            .position
            .map_or(0.0, |p| (clamped[0].0 - p[0]).hypot(clamped[1].0 - p[1]));
        let line = go_to_line(clamped, feed);
        if !self.jog_absolute(&line, distance / (feed / 60.0), at) {
            self.events.push(Event::JogEnded);
        }
    }

    /// `ControllerCommand::Pulse`: the block in one write, only with laser mode on, `$30` known,
    /// `S` at least 1 and nothing else in GRBL's hands; otherwise `PulseSkipped`.
    pub(super) fn pulse(&mut self, power: PulsePower, length: PulseLength) {
        let busy = self.stream.is_some()
            || self.stop.is_some()
            || self.home_in_flight()
            || self.alarmed
            || self.jogging()
            || self.pulse.is_some();
        let s = match (self.settings.laser_mode(), self.settings.s_max()) {
            (Some(true), Some(s_max)) => pulse_s(power, s_max),
            _ => 0,
        };
        let lines = pulse_block(s, length);
        let bytes: usize = lines.iter().map(String::len).sum();
        if busy || s < 1 || self.owed_bytes() + bytes > self.rx_size {
            self.events.push(Event::PulseSkipped);
            return;
        }
        for (i, line) in lines.iter().enumerate() {
            self.out.extend_from_slice(line.as_bytes());
            self.owed.push_back(Owed::Pulse {
                bytes: line.len(),
                last: i == 2,
            });
        }
        // The block ends in `M5`: the laser guarantee holds for the next motion.
        self.spindle_off = true;
        self.pulse = Some(PulseRun::default());
    }

    /// GRBL's answer to a line of the pulse block: an error before the end is named when the
    /// block's `M5 S0` is answered; an error on `M5 S0` itself runs the stop of G12.
    pub(super) fn on_pulse_answer(&mut self, code: Option<u16>, last: bool, at: Duration) {
        let Some(run) = self.pulse.as_mut() else {
            return;
        };
        if !last {
            if let Some(code) = code {
                run.failure.get_or_insert_with(|| texts::error_text(code));
            }
            return;
        }
        if code.is_some() {
            self.begin_stop(None, at);
            return;
        }
        let failure = self.pulse.take().and_then(|run| run.failure);
        self.events.push(match failure {
            Some(detail) => Event::PulseFailed { detail },
            None => Event::PulseEnded,
        });
    }

    /// A report while the block runs: a hold or an open door ends it with the stop of G12, from
    /// its reset, since GRBL already holds.
    pub(super) fn pulse_report(&mut self, state: &str, at: Duration) {
        let holds = state.starts_with("Hold") || state.starts_with("Door");
        if self.pulse.is_some() && self.stop.is_none() && holds {
            self.start_stop((true, false, false), None, at);
        }
    }

    /// Whether the pulse block is in GRBL's hands.
    pub(super) fn pulsing(&self) -> bool {
        self.pulse.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jog_line_is_absolute_in_millimetres_without_negative_zero() {
        let line = go_to_line([Millimetres(12.5), Millimetres(-0.0001)], 3_000.0);
        assert_eq!(line, "$J=G90 G21 X12.500 Y0.000 F3000\n");
    }

    #[test]
    fn s_is_rounded_down_from_the_per_mille_of_s_max() {
        let p = |n| PulsePower::new(n).unwrap();
        assert_eq!(pulse_s(p(10), 1_000), 10);
        assert_eq!(pulse_s(p(50), 1_000), 50);
        assert_eq!(pulse_s(p(10), 255), 2);
        assert_eq!(pulse_s(p(1), 255), 0);
        assert_eq!(pulse_s(p(3), 999), 2);
    }

    #[test]
    fn the_dwell_is_in_seconds_with_three_decimals() {
        let l = |n| PulseLength::new(n).unwrap();
        assert_eq!(
            pulse_block(10, l(300)),
            [
                "G1 M3 S10\n".to_owned(),
                "G4 P0.300\n".to_owned(),
                "M5 S0\n".to_owned()
            ]
        );
        assert_eq!(pulse_block(1, l(2_000))[1], "G4 P2.000\n");
        assert_eq!(pulse_block(1, l(50))[1], "G4 P0.050\n");
    }
}
