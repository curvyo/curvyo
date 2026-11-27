//! What the virtual GRBL shows (`specs/0161-cnc-output` criterion 15): its state word, the head's
//! position between two block ends, and the `?` status report as `$10` asks for it.

use std::fmt::Write as _;

use super::parse::Spindle;
use super::{PLANNER_BLOCKS, RX_SIZE, VirtualGrbl};

/// The head's machine position: along the running block by the time it has run.
pub(super) fn position_of(g: &VirtualGrbl) -> [f64; 3] {
    let Some(r) = g.running else {
        return g.pos;
    };
    let t = if r.duration > 0.0 {
        (r.elapsed / r.duration).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let mut p = r.from;
    for (axis, slot) in p.iter_mut().enumerate() {
        *slot += (r.block.target[axis] - r.from[axis]) * t;
    }
    p
}

/// The state word of the report.
pub(super) fn state_word(g: &VirtualGrbl) -> &'static str {
    if g.homing.is_some() {
        "Home"
    } else if g.alarmed() {
        "Alarm"
    } else if let Some(left) = g.hold {
        if left <= 0.0 { "Hold:0" } else { "Hold:1" }
    } else if g.running.is_some_and(|r| r.jog) {
        "Jog"
    } else if g.moving() {
        "Run"
    } else {
        "Idle"
    }
}

fn axes(p: [f64; 3]) -> String {
    format!("{:.3},{:.3},{:.3}", p[0], p[1], p[2])
}

/// Prints the status report: state, `MPos` (or `WPos` when `$10` bit 0 is off), `Bf` when bit 1
/// is on, `FS`, `WCO` now and then with `MPos`, and `A` while the spindle turns.
pub(super) fn report(g: &mut VirtualGrbl) {
    let mask = g.settings.report_mask();
    let p = position_of(g);
    let mut text = format!("<{}", state_word(g));
    if mask & 1 == 1 {
        let _ = write!(text, "|MPos:{}", axes(p));
    } else {
        let _ = write!(text, "|WPos:{}", axes(p));
    }
    if mask & 2 == 2 {
        let used = g.queue.len() + usize::from(g.running.is_some());
        let _ = write!(
            text,
            "|Bf:{},{}",
            PLANNER_BLOCKS.saturating_sub(used),
            RX_SIZE - g.rx.len()
        );
    }
    let moving = g.running.is_some() && g.hold.is_none();
    let feed = if moving { g.speed_now() * 60.0 } else { 0.0 };
    let rpm = if g.spindle.0 == Spindle::Off {
        0.0
    } else {
        g.spindle.1
    };
    let _ = write!(text, "|FS:{feed:.0},{rpm:.0}");
    if mask & 1 == 1 && g.reports.is_multiple_of(10) {
        text += "|WCO:0.000,0.000,0.000";
    }
    match g.spindle.0 {
        Spindle::Cw => text += "|A:S",
        Spindle::Ccw => text += "|A:C",
        Spindle::Off => {}
    }
    text.push('>');
    g.reports = g.reports.wrapping_add(1);
    g.print(&text);
}
