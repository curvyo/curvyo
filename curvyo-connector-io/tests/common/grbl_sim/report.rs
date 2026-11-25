//! What the simulator shows: its state word, its status report (with `Bf:` and `A:`) and whether
//! the beam is lit.

use super::{GrblSim, RX_SIZE, Spindle};

impl GrblSim {
    /// Whether a feed hold has completed (`Hold:0`).
    #[must_use]
    pub fn held(&self) -> bool {
        self.hold.is_some_and(|h| h.complete)
    }

    /// Whether the spindle output is powered, as the report's `A:` field shows it: off after
    /// Toggle Spindle Stop, and in a hold in laser mode unless the build leaves it on.
    pub(crate) fn spindle_powered(&self) -> bool {
        let in_hold = self.hold.is_some();
        self.spindle != Spindle::Off
            && !self.spindle_stop
            && !(in_hold && self.config.laser_mode && !self.config.laser_on_in_hold)
    }

    pub(crate) fn state_word(&self) -> &'static str {
        if self.homing.is_some() {
            "Home"
        } else if self.door {
            "Door:0"
        } else if self.alarm {
            "Alarm"
        } else if let Some(h) = self.hold {
            if h.complete { "Hold:0" } else { "Hold:1" }
        } else if self.planner.is_empty() {
            "Idle"
        } else if self.jogging() {
            "Jog"
        } else {
            "Run"
        }
    }

    pub(crate) fn report(&mut self) {
        let fmt = |p: [f64; 3]| format!("{:.3},{:.3},{:.3}", p[0], p[1], p[2]);
        let mut text = format!("<{}", self.state_word());
        if self.config.mpos_reports {
            text += &format!("|MPos:{}", fmt(self.pos));
            if self.reports.is_multiple_of(10) {
                text += &format!("|WCO:{}", fmt(self.config.wco));
            }
        } else {
            let w = self.config.wco;
            text += &format!(
                "|WPos:{}",
                fmt([self.pos[0] - w[0], self.pos[1] - w[1], self.pos[2] - w[2]])
            );
        }
        if self.config.buffer_reports {
            let blocks = self.config.planner_size.saturating_sub(self.planner.len());
            text += &format!("|Bf:{blocks},{}", RX_SIZE - self.rx.len());
        }
        text += "|FS:0,0";
        let mut accessories = String::new();
        match self.spindle {
            _ if !self.spindle_powered() => {}
            Spindle::Cw => accessories.push('S'),
            Spindle::Ccw => accessories.push('C'),
            Spindle::Off => {}
        }
        if self.air {
            accessories.push('F');
        }
        if !accessories.is_empty() {
            text += &format!("|A:{accessories}");
        }
        text.push('>');
        self.reports += 1;
        self.last_report = Some(self.clock);
        self.print(&text);
    }

    /// Whether the beam would be lit now: laser mode lights it while moving, and at standstill
    /// only for `M3` under motion mode `G1` (GRBL gives `M4` zero power at rest).
    #[must_use]
    pub fn laser_lit(&self) -> bool {
        let on = self.spindle_powered() && self.power > 0.0;
        let at_rest = self.spindle == Spindle::Cw && self.motion_g1 && self.hold.is_none();
        if self.config.laser_mode {
            on && (self.state_word() == "Run"
                || (self.held() && self.config.laser_on_in_hold)
                || at_rest)
        } else {
            on
        }
    }
}
