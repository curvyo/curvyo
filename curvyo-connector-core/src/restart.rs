//! Where a restart from a line begins: its line, the position proof, the lines that take a
//! stopped machine there with the laser off (`specs/0118` 2e, adrs R3, R4, R6).

use crate::gcode_words::{strip, words};
use crate::job_list::StopPoint;
use crate::jog::JogStep;
use crate::millimetres::Millimetres;
use crate::modal::{Modal, Spindle};

/// GRBL's planner blocks until `$I` names them (`[OPT:V,15,128]` names 15).
pub const PLANNER_BLOCKS: u32 = 16;

/// Two work offsets within this distance per axis are the same (mm).
const SAME: f64 = 0.01;

/// Whether a line moves an axis: a word X, Y or Z (an arc counts one).
fn moves(line: &str) -> bool {
    let stripped = strip(line);
    words(&stripped).is_some_and(|w| w.iter().any(|(l, _)| matches!(l, 'X' | 'Y' | 'Z')))
}

/// The line a restart begins at: from the acknowledged line `acked` back over `blocks` motion
/// lines (GRBL's planner may hold them unburned), never before `run_from`; so a few segments may
/// burn twice and none is skipped.
#[must_use]
pub fn restart_line(text: &str, acked: u32, run_from: u32, blocks: u32) -> u32 {
    let floor = run_from.max(1);
    if acked < floor || blocks == 0 {
        return floor;
    }
    let lines: Vec<&str> = text
        .lines()
        .take(usize::try_from(acked).unwrap_or(usize::MAX))
        .collect();
    let mut counted = 0;
    let mut line = acked;
    while line >= floor {
        let at = usize::try_from(line - 1).unwrap_or(usize::MAX);
        if lines.get(at).is_some_and(|l| moves(l)) {
            counted += 1;
            if counted == blocks {
                return line;
            }
        }
        line -= 1;
    }
    floor
}

/// What the connector knows of the machine now, for the position proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Now {
    /// Homed since the connector identified the controller.
    pub(crate) homed: bool,
    /// The machine position the controller last reported.
    pub(crate) machine: Option<[Millimetres; 3]>,
    /// The work offset the controller last reported.
    pub(crate) offset: Option<[Millimetres; 3]>,
    /// No break of trust since a Stop interrupted the job (Frage 390 C): GRBL kept its count.
    pub(crate) unbroken: bool,
}

/// Rule a of `specs/0132` criterion 12: the position is trusted. The work offset must be the
/// stored one also after a Home (Curvyo writes no `G10`/`G92`), then the machine is homed, or
/// nothing broke since a Stop and the head stands where it stopped.
pub(crate) fn position(stop: Option<&StopPoint>, now: &Now) -> Result<(), &'static str> {
    let stored = stop.and_then(|s| s.work_offset);
    let (Some(stored), Some(offset), Some(_)) = (stored, now.offset, now.machine) else {
        return Err("No position from the machine");
    };
    if stored
        .iter()
        .zip(offset)
        .any(|(a, b)| (a.0 - b.0).abs() > SAME)
    {
        return Err("Work offset changed");
    }
    let same = |a: Option<[Millimetres; 3]>, b: Option<[Millimetres; 3]>| {
        a.zip(b)
            .is_some_and(|(a, b)| a.iter().zip(b).all(|(a, b)| (a.0 - b.0).abs() <= SAME))
    };
    let stood = stop.and_then(|s| s.machine_position);
    if now.homed || (now.unbroken && same(stood, now.machine)) {
        Ok(())
    } else {
        Err("Not homed")
    }
}

/// A coordinate with three decimals, never `-0.000`.
fn coordinate(value: f64) -> String {
    let text = format!("{value:.3}");
    if text == "-0.000" {
        "0.000".to_owned()
    } else {
        text
    }
}

/// A feed or power: whole numbers without decimals, others with up to three.
fn short(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// The lines that take a stopped machine to the position and state before the restart line,
/// laser off until the last one: `M5`, `M9`; `G21 G90 G17 G94` (and a work system other than
/// `G54`); the travel `G1 … S0` at the 10 mm jog feed (Z first when it goes up from `z_now`, the
/// work Z now); the rebuilt state (an arc mode as `G1`, line n names its own); `S` with `M3`/`M4` when the laser was on.
pub(crate) fn entry(modal: &Modal, z_now: Option<f64>) -> Vec<String> {
    let feed = short(JogStep::Ten.feed().0);
    let mut lines = vec!["M5".to_owned(), "M9".to_owned()];
    lines.push(if modal.work == 54 {
        "G21 G90 G17 G94".to_owned()
    } else {
        format!("G21 G90 G17 G94 G{}", modal.work)
    });
    let [x, y, z] = modal.position;
    let travel = format!(
        "G1 X{} Y{} F{feed} S0",
        coordinate(x.unwrap_or_default()),
        coordinate(y.unwrap_or_default())
    );
    let lift = z.map(|z| format!("G1 Z{} F{feed} S0", coordinate(z)));
    let up = matches!((z, z_now), (Some(z), Some(now)) if z > now);
    match lift {
        Some(lift) if up => lines.extend([lift, travel]),
        Some(lift) => lines.extend([travel, lift]),
        None => lines.push(travel),
    }
    let mut state = vec![
        if modal.inches { "G20" } else { "G21" },
        if modal.incremental { "G91" } else { "G90" },
    ];
    if modal.arc_incremental {
        state.push("G91.1");
    }
    state.push(match modal.plane {
        18 => "G18",
        19 => "G19",
        _ => "G17",
    });
    state.push(if modal.inverse_time { "G93" } else { "G94" });
    lines.push(state.join(" "));
    // An arc mode is set by line n itself (`modal::before` checked it names its motion).
    let motion = match modal.motion {
        2 | 3 => "G1".to_owned(),
        other => format!("G{other}"),
    };
    lines.push(match modal.feed {
        Some(f) => format!("{motion} F{}", short(f)),
        None => motion,
    });
    if modal.mist {
        lines.push("M7".to_owned());
    }
    if modal.flood {
        lines.push("M8".to_owned());
    }
    let power = format!("S{}", short(modal.power));
    lines.push(match modal.spindle {
        Spindle::Constant => format!("{power} M3"),
        Spindle::Dynamic => format!("{power} M4"),
        Spindle::Off => power,
    });
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal::{Unrebuildable, before};
    use std::fmt::Write as _;

    fn moves_text(n: u32) -> String {
        (0..n).fold("G21\nG90\nM4 S0\n".to_owned(), |mut text, i| {
            let _ = writeln!(text, "G1 X{i} Y0 F600 S500");
            text
        })
    }

    #[test]
    fn restart_line_table() {
        // Lines 1 to 3 hold no motion; 4 to 43 are 40 moves.
        let text = moves_text(40);
        // (acked, run_from, blocks) -> line
        let table = [
            (43, 1, 15, 29),
            (43, 1, 16, 28),
            (20, 1, 15, 6),
            (18, 1, 15, 4),
            (17, 1, 15, 1),
            (10, 1, 16, 1),
            (0, 1, 16, 1),
            (43, 35, 16, 35),
            (43, 30, 15, 30),
            (43, 29, 15, 29),
            (20, 25, 16, 25),
            (43, 1, 1, 43),
        ];
        for (acked, run_from, blocks, line) in table {
            assert_eq!(
                restart_line(&text, acked, run_from, blocks),
                line,
                "{acked} {run_from} {blocks}"
            );
        }
    }

    #[test]
    fn an_arc_counts_one_and_lines_without_axes_count_none() {
        let text = "G0 X0 Y0\nG2 X10 Y0 I5 J0 F600\nS200\n; note\nM8\nG1 Y5\nG1 X1\n";
        assert_eq!(restart_line(text, 7, 1, 2), 6);
        assert_eq!(restart_line(text, 7, 1, 3), 2);
        assert_eq!(restart_line(text, 7, 1, 4), 1);
        assert_eq!(
            restart_line(text, 5, 1, 1),
            2,
            "S, comment and M8 are no moves"
        );
    }

    #[allow(clippy::unnecessary_wraps)] // the table's cells are options
    fn mm(v: [f64; 3]) -> Option<[Millimetres; 3]> {
        Some(v.map(Millimetres))
    }

    fn stop(offset: Option<[Millimetres; 3]>) -> StopPoint {
        StopPoint {
            machine_position: mm([-100.0, -100.0, 0.0]),
            work_offset: offset,
            run_from: 1,
            exact: true,
        }
    }

    #[test]
    fn position_table() {
        let zero = mm([0.0; 3]);
        let now = |homed, machine, offset| Now {
            homed,
            machine,
            offset,
            unbroken: false,
        };
        let here = mm([-50.0, -50.0, 0.0]);
        let table = [
            (Some(stop(zero)), now(true, here, zero), Ok(())),
            (Some(stop(zero)), now(false, here, zero), Err("Not homed")),
            (
                Some(stop(zero)),
                now(true, here, mm([0.005, -0.01, 0.0])),
                Ok(()),
            ),
            (
                Some(stop(zero)),
                now(true, here, mm([0.02, 0.0, 0.0])),
                Err("Work offset changed"),
            ),
            (
                Some(stop(zero)),
                now(false, here, mm([5.0, 0.0, 0.0])),
                Err("Work offset changed"),
            ),
            (
                Some(stop(zero)),
                now(true, None, zero),
                Err("No position from the machine"),
            ),
            (
                Some(stop(zero)),
                now(true, here, None),
                Err("No position from the machine"),
            ),
            (
                Some(stop(None)),
                now(true, here, zero),
                Err("No position from the machine"),
            ),
            (
                None,
                now(true, here, zero),
                Err("No position from the machine"),
            ),
        ];
        for (i, (stop, now, expected)) in table.into_iter().enumerate() {
            assert_eq!(position(stop.as_ref(), &now), expected, "row {i}");
        }
        // Frage 390 C: no break since a Stop and the head where it stopped also prove it.
        let stopped = Now {
            homed: false,
            machine: mm([-100.0, -100.005, 0.0]),
            offset: zero,
            unbroken: true,
        };
        assert_eq!(position(Some(&stop(zero)), &stopped), Ok(()));
        let moved = Now {
            machine: here,
            ..stopped
        };
        assert_eq!(position(Some(&stop(zero)), &moved), Err("Not homed"));
        let broken = Now {
            unbroken: false,
            ..stopped
        };
        assert_eq!(position(Some(&stop(zero)), &broken), Err("Not homed"));
    }

    #[test]
    fn numbers_are_written_plainly() {
        assert_eq!(coordinate(-0.0001), "0.000");
        assert_eq!(coordinate(12.3456), "12.346");
        assert_eq!(short(3000.0), "3000");
        assert_eq!(short(12.5), "12.5");
        assert_eq!(short(0.0), "0");
    }

    /// (case, restart line, work Z now)
    const GOLDEN: [(&str, u32, Option<f64>); 8] = [
        ("curvyo_m4", 10, Some(0.0)),
        ("curvyo_m3_air", 14, Some(0.0)),
        ("feed_and_s_change", 9, Some(0.0)),
        ("inch_g20", 7, Some(0.0)),
        ("z_up", 8, Some(-3.0)),
        ("z_down", 8, Some(0.0)),
        ("arc_g2", 7, Some(0.0)),
        ("home_first_line", 11, Some(0.0)),
    ];

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/tests/fixtures/restart/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn every_entry_matches_its_golden_file() {
        for (case, line, z_now) in GOLDEN {
            let text = fixture(&format!("{case}.gcode"));
            let modal = before(&text, line).unwrap_or_else(|e| panic!("{case}: {e:?}"));
            let entry = entry(&modal, z_now);
            let golden = fixture(&format!("{case}.entry"));
            assert_eq!(entry.join("\n") + "\n", golden, "{case}");
            // The laser stays off until the last line.
            for l in &entry[..entry.len() - 1] {
                assert!(!l.contains("M3") && !l.contains("M4"), "{case}: {l}");
                assert!(
                    l.split(' ').all(|w| !w.starts_with('S') || w == "S0"),
                    "{case}: {l}"
                );
            }
            assert_eq!(entry[..2], ["M5", "M9"], "{case}");
        }
    }

    #[test]
    fn refused_jobs_name_their_cause() {
        let table = [
            ("head_position_g91", 10, Unrebuildable::HeadPosition),
            ("unknown_g28", 8, Unrebuildable::Word("G28".into())),
            ("program_end_m2", 9, Unrebuildable::Word("M2".into())),
        ];
        for (case, line, cause) in table {
            let text = fixture(&format!("{case}.gcode"));
            assert_eq!(before(&text, line), Err(cause), "{case}");
        }
    }
}
