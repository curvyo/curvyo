//! `specs/0118` slice 2e against the GRBL simulator (adrs R6, R9, R11; `specs/0132` 12): a
//! restart reads `$32` again (`$$`), then writes the entry line by line with the laser off until
//! its last line, a Stop during the travel returns the entry unchanged, the position proof
//! refuses before a byte is written (not homed after a connector restart, a work offset changed
//! after Home, `$32=0`, a Stop whose reset came during motion), and a target outside the travel
//! writes nothing. Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use std::fmt::Write as _;
use std::sync::Arc;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{
    Entry, Item, JobResult, ListEntry, Message, Millimetres, State, StopPoint,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Lines 1 to 3 set the state, 4 to 53 are fifty moves along Y -20, 54 ends.
fn text(x0: f64) -> String {
    let mut text = "G21\nG90\nM4 S0\n".to_owned();
    for i in 0..50 {
        let _ = writeln!(text, "G1 X{} Y-20 F1000 S500", x0 - f64::from(i));
    }
    text + "M5\n"
}

fn id() -> Uuid {
    Uuid::from_u128(0x77)
}

fn item(text: &str, exact: bool) -> Item {
    let sha256 = Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
    Item {
        entry: Entry {
            job_id: id(),
            name: "sign.gcode".into(),
            lines: u32::try_from(text.lines().count()).unwrap(),
            bytes: text.len() as u64,
            sha256,
            sent_by: "Phone".into(),
            run_next_by: None,
            interrupted_at: Some(40),
            stop: Some(StopPoint {
                machine_position: Some([-40.0, -20.0, 0.0].map(Millimetres)),
                work_offset: Some([0.0; 3].map(Millimetres)),
                run_from: 1,
                exact,
            }),
        },
        text: Arc::from(text),
    }
}

fn config() -> SimConfig {
    SimConfig {
        mpos_reports: true,
        homing_lock: true,
        homing_steps: 20,
        travel: [400.0, 300.0],
        ..SimConfig::default()
    }
}

/// A connector that came back after a restart with the interrupted job in its list.
fn restored(config: SimConfig, text: &str, exact: bool) -> Bench {
    let mut bench = Bench::connected(config);
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench.machine.restore(vec![item(text, exact)]);
    bench
}

fn home(bench: &mut Bench) {
    bench.app(Message::Home);
    assert!(bench.run_until(|b| b.state() == State::Idle && b.homed(), 600));
    bench.run_for(std::time::Duration::from_millis(300));
}

fn row(bench: &Bench) -> Option<ListEntry> {
    match bench.machine.job_list() {
        Message::JobList { waiting, .. } => waiting.into_iter().find(|e| e.position == 0),
        _ => None,
    }
}

fn restart(bench: &mut Bench) {
    bench.app(Message::RestartFromLine {
        job_id: id(),
        line: 26,
    });
}

fn entering(bench: &Bench) -> bool {
    matches!(bench.status(), Message::Status { job: Some(p), .. } if p.entering)
}

trait Homed {
    fn homed(&self) -> bool;
}

impl Homed for Bench {
    fn homed(&self) -> bool {
        matches!(self.status(), Message::Status { homed: true, .. })
    }
}

#[test]
fn r6_the_entry_goes_out_line_by_line_dark_until_its_last_line_then_the_job_runs() {
    let text = text(-10.0);
    let mut bench = restored(config(), &text, true);
    home(&mut bench);
    let shown = row(&bench).unwrap();
    assert_eq!(
        shown.restart_line,
        Some(26),
        "15 blocks from `[OPT:V,15,128]`"
    );
    assert_eq!(shown.restart_cause, None);
    let mark = bench.sim.lines.len();
    restart(&mut bench);
    assert_eq!(bench.state(), State::Running);
    assert!(entering(&bench));
    // Dark until the head stands at the start of line 26 (machine X -31, Y -20).
    for _ in 0..3_000 {
        if !entering(&bench) {
            break;
        }
        bench.step();
        if bench.sim.laser_lit() {
            let [x, y] = bench.sim.rx_pos_for_tests();
            assert!(x <= -30.95 && (y + 20.0).abs() < 1e-6, "lit at {x} {y}");
        }
    }
    assert!(!entering(&bench), "the entry reached its line");
    assert!(bench.run_until(|b| matches!(b.state(), State::Done { .. }), 6_000));
    assert_eq!(
        bench.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    let lines = &bench.sim.lines[mark..];
    assert_eq!(
        lines[..9],
        [
            "$$",
            "M5",
            "M9",
            "G21G90G17G94",
            "G1X-31.000Y-20.000F3000S0",
            "G21G90G17G94",
            "G1F1000",
            "S500M4",
            "G1X-32Y-20F1000S500",
        ]
    );
    assert_eq!(lines.len(), 9 + 28, "lines 27 to 54 follow");
    match bench.status() {
        Message::Status { job: Some(p), .. } => {
            assert_eq!((p.from_line, p.line, p.lines), (Some(26), 54, 54));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(row(&bench), None, "a completed restart leaves no row");
    assert_eq!(bench.sim.faults, Vec::<String>::new());
}

#[test]
fn r8_stop_during_the_travel_returns_the_entry_unchanged() {
    let text = text(-10.0);
    let mut config = config();
    config.steps_per_motion = 200;
    let mut bench = restored(config, &text, true);
    home(&mut bench);
    restart(&mut bench);
    bench.run_for(std::time::Duration::from_millis(200));
    assert!(entering(&bench), "still on the travel");
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == State::Idle, 2_000));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Not started: Restart from line 26")
    );
    let shown = row(&bench).unwrap();
    assert_eq!(
        (shown.interrupted_at, shown.restart_line),
        (Some(40), Some(26))
    );
    assert_eq!(bench.machine.stored().waiting[0], item(&text, true).entry);
    assert_eq!(shown.restart_cause, None, "the asked reset keeps homed");
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert!(bench.alarm().is_none());
}

#[test]
fn r4_after_a_connector_restart_it_is_not_homed_until_home() {
    let text = text(-10.0);
    let mut bench = restored(config(), &text, true);
    bench.run_for(std::time::Duration::from_millis(300));
    assert_eq!(
        row(&bench).unwrap().restart_cause.as_deref(),
        Some("Not homed")
    );
    let mark = bench.sim.lines.len();
    restart(&mut bench);
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(bench.sim.lines.len(), mark, "no line");
    home(&mut bench);
    assert_eq!(row(&bench).unwrap().restart_cause, None);
    restart(&mut bench);
    assert_eq!(bench.state(), State::Running);
}

#[test]
fn r4_a_work_offset_changed_after_home_refuses() {
    let text = text(-10.0);
    let mut bench = restored(config(), &text, true);
    home(&mut bench);
    // As `G10 L20 P1 X5` on the controller would.
    bench.sim.config.wco = [5.0, 0.0, 0.0];
    assert!(bench.run_until(|b| row(b).unwrap().restart_cause.is_some(), 2_000));
    assert_eq!(
        row(&bench).unwrap().restart_cause.as_deref(),
        Some("Work offset changed")
    );
    let mark = bench.sim.lines.len();
    restart(&mut bench);
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(bench.sim.lines.len(), mark);
}

#[test]
fn r9_a_target_outside_the_travel_writes_nothing_and_keeps_the_entry() {
    // Line 25 stands at X -460: 60 mm beyond the 400 mm of X.
    let text = text(-439.0);
    let mut bench = restored(config(), &text, true);
    home(&mut bench);
    let log_mark = bench.sim.log.len();
    restart(&mut bench);
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(
        bench.detail().as_deref(),
        Some("Outside the travel by 60 mm")
    );
    assert_eq!(bench.lines_after(log_mark), "");
    assert_eq!(row(&bench).unwrap().interrupted_at, Some(40));
}

#[test]
fn laser_mode_off_refuses_the_restart() {
    let text = text(-10.0);
    let mut config = config();
    config.laser_mode = false;
    let mut bench = restored(config, &text, true);
    home(&mut bench);
    assert_eq!(
        row(&bench).unwrap().restart_cause.as_deref(),
        Some("Laser mode is off ($32=0)")
    );
}

#[test]
fn a_crash_stores_the_line_approximately_and_reads_near() {
    let text = text(-10.0);
    let mut bench = restored(config(), &text, false);
    home(&mut bench);
    let shown = row(&bench).unwrap();
    assert!(shown.line_approximate);
    assert_eq!(shown.restart_cause, None);
}

/// A connector with homing unlocked and no job list: the bench of the stop tests.
fn unlocked(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench
}

/// Starts `text(-10.0)` and stops it at line 30 of 54; the stopped job's row.
fn run_and_stop(bench: &mut Bench) -> ListEntry {
    bench.start(&text(-10.0));
    assert!(bench.run_until(|b| b.line() >= 30, 6_000));
    bench.app(Message::Stop);
    assert!(bench.run_until(
        |b| matches!(b.state(), State::Done { .. } | State::Alarm | State::Idle),
        6_000
    ));
    bench.run_for(std::time::Duration::from_millis(300));
    row(bench).expect("Frage 390 C: the stopped job is listed as interrupted")
}

#[test]
fn r4_a_stop_whose_reset_followed_hold_zero_keeps_homed_and_the_position() {
    // Feed hold, `Hold:0` (motion complete), then the reset: GRBL keeps its position, no alarm.
    let mut bench = unlocked(config());
    home(&mut bench);
    let shown = run_and_stop(&mut bench);
    assert!(bench.alarm().is_none(), "no alarm after a completed hold");
    assert!(
        bench.homed(),
        "the asked reset after Hold:0 keeps homed (criterion 13)"
    );
    assert_eq!(shown.restart_cause, None);
    // Without a Home: the Stop broke nothing and the head stands where it stopped (390 C).
    let mut bench = unlocked(config());
    let shown = run_and_stop(&mut bench);
    assert!(!bench.homed());
    assert_eq!(shown.restart_cause, None, "proved by the unbroken Stop");
}

#[test]
fn r4_a_stop_whose_reset_came_during_motion_raises_alarm_3_and_needs_a_home() {
    // The hold never completes within the limit: the reset lands while moving, `ALARM:3`.
    let mut config = config();
    config.decel_steps = 1_000_000;
    let mut bench = unlocked(config);
    home(&mut bench);
    let shown = run_and_stop(&mut bench);
    assert_eq!(bench.state(), State::Alarm, "{:?}", bench.status());
    assert!(!bench.homed());
    assert_eq!(shown.restart_cause.as_deref(), Some("Alarm"));
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 2_000));
    bench.run_for(std::time::Duration::from_millis(300));
    assert_eq!(
        row(&bench).unwrap().restart_cause.as_deref(),
        Some("Not homed"),
        "the position is not trusted after a reset during motion"
    );
}

#[test]
fn r4_a_reset_without_hold_zero_distrusts_the_position_even_without_alarm_3() {
    // The same stop with the `ALARM:3` line lost: no `Hold:0` before the reset is reason enough.
    let mut config = config();
    config.decel_steps = 1_000_000;
    config.quiet_reset_alarm = true;
    let mut bench = unlocked(config);
    home(&mut bench);
    let shown = run_and_stop(&mut bench);
    assert!(!bench.homed(), "a reset during motion clears homed");
    assert_eq!(shown.restart_cause.as_deref(), Some("Not homed"));
    let mark = bench.sim.lines.len();
    bench.app(Message::RestartFromLine {
        job_id: shown.job_id,
        line: shown.restart_line.unwrap(),
    });
    assert_eq!(bench.sim.lines.len(), mark, "no line");
    assert!(!matches!(bench.state(), State::Running));
}

#[test]
fn r8_laser_mode_is_read_again_right_before_the_entry() {
    let text = text(-10.0);
    let mut bench = restored(config(), &text, true);
    home(&mut bench);
    bench.sim.set_setting(32, "0");
    let mark = bench.sim.lines.len();
    restart(&mut bench);
    assert!(bench.run_until(|b| b.state() == State::Idle, 2_000));
    assert_eq!(
        bench.sim.lines[mark..],
        ["$$".to_owned()],
        "no byte of the entry"
    );
    assert_eq!(bench.detail().as_deref(), Some("Laser mode is off ($32=0)"));
    assert_eq!(
        row(&bench).unwrap().restart_cause.as_deref(),
        Some("Laser mode is off ($32=0)"),
        "the cached value is fresh"
    );
    assert_eq!(bench.sim.spindle, Spindle::Off);
    // Set back on, the next restart reads it again and runs.
    bench.sim.set_setting(32, "1");
    restart(&mut bench);
    assert_eq!(bench.state(), State::Idle, "refused on the cached $32=0");
}
