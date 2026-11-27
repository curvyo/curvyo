//! The virtual GRBL's `$` commands (`specs/0161-cnc-output` criterion 15): `$$`, `$I`, `$G`,
//! `$#`, `$X`, `$H`, `$C`, `$<n>=<value>` and the `$J=` jog.

use super::codes;
use super::parse::{Action, parse_line};
use super::planner::block;
use super::{HOMING_SECONDS, VERSION, VirtualGrbl};

fn error(g: &mut VirtualGrbl, code: u16) {
    g.print(&format!("error:{code}"));
}

/// Answers the `$` line `line` (stripped, upper case); `false` when it must wait.
pub(super) fn run(g: &mut VirtualGrbl, line: &str) -> bool {
    match line {
        "$$" => {
            for l in g.settings.listing() {
                g.print(&l);
            }
        }
        "$I" => {
            g.print(&format!("[VER:{VERSION}.20190825:Curvyo virtual GRBL]"));
            g.print("[OPT:V,15,128]");
        }
        "$G" => {
            let motion = if g.modal.rapid { "G0" } else { "G1" };
            let text = format!(
                "[GC:{motion} G54 G17 G21 {} G94 M5 M9 T0 F{:.0} S{:.0}]",
                if g.modal.absolute { "G90" } else { "G91" },
                g.modal.feed.unwrap_or(0.0),
                g.modal.rpm
            );
            g.print(&text);
        }
        "$#" => {
            for name in [
                "G54", "G55", "G56", "G57", "G58", "G59", "G28", "G30", "G92",
            ] {
                g.print(&format!("[{name}:0.000,0.000,0.000]"));
            }
            g.print("[TLO:0.000]");
            g.print("[PRB:0.000,0.000,0.000:0]");
        }
        "$C" | "$N" | "$SLP" => {}
        "$X" => {
            if g.critical {
                error(g, codes::GCODE_LOCK);
                return true;
            }
            if g.locked {
                g.locked = false;
                g.print("[MSG:Caution: Unlocked]");
            }
        }
        "$H" => {
            if !g.settings.homing() {
                error(g, codes::SETTING_DISABLED);
            } else if g.moving() || g.hold.is_some() || g.critical {
                error(g, codes::NOT_IDLE);
            } else {
                g.homing = Some(HOMING_SECONDS);
            }
            // The `ok` of a homing cycle comes when it ends.
            return true;
        }
        _ if line.starts_with("$J=") => return jog(g, &line[3..]),
        _ => {
            let set = line
                .strip_prefix('$')
                .and_then(|s| s.split_once('='))
                .and_then(|(n, v)| Some((n.parse::<u16>().ok()?, v.to_owned())));
            let Some((n, value)) = set else {
                error(g, codes::INVALID_STATEMENT);
                return true;
            };
            if g.moving() {
                error(g, codes::NOT_IDLE);
                return true;
            }
            if let Err(code) = g.settings.set(n, &value) {
                error(g, code);
                return true;
            }
        }
    }
    g.print("ok");
    true
}

/// `$J=<words>`: one move that leaves the parser's state as it was, refused in an alarm, without
/// a feed, or beyond the soft limits.
fn jog(g: &mut VirtualGrbl, words: &str) -> bool {
    if g.alarmed() {
        error(g, codes::GCODE_LOCK);
        return true;
    }
    let mut modal = g.modal;
    modal.rapid = false;
    modal.feed = None;
    let actions = parse_line(&format!("G1{words}"), &mut modal, g.planned).unwrap_or_default();
    let target = actions.iter().find_map(|a| {
        if let Action::Move { target, .. } = *a {
            Some(target)
        } else {
            None
        }
    });
    let Some(target) = target else {
        error(g, codes::INVALID_JOG);
        return true;
    };
    if g.outside_soft_limits(target) {
        error(g, codes::TRAVEL_EXCEEDED);
        return true;
    }
    let feed = modal.feed.unwrap_or(0.0);
    if let Some(b) = block(g.planned, target, false, feed, &g.settings) {
        g.queue.push_back((b, true));
        g.planned = target;
    }
    g.print("ok");
    true
}
