//! `specs/0118` slice 2d against the GRBL simulator (criteria 9, 10, 35 b, 35 c; adrs P5, P6,
//! P12): the go to point writes `M5`/`M9`, one absolute `$J=`, then the pulse block in one write
//! only after Idle at the target; `S` per `$30`, no block with `$32=0` or `S` below 1; Stop
//! mid-move is the jog cancel, mid-pulse the stop of G12 with no alarm and homed kept; a door, a
//! pulse that does not end, a lost cable and `error:15`. Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{
    Event, Message, Millimetres, Peer, Pulse, PulseLength, PulsePower, State,
};

const RESET: u8 = 0x18;
const JOG_CANCEL: u8 = 0x85;

fn homed_config() -> SimConfig {
    SimConfig {
        mpos_reports: true,
        homing_lock: true,
        homing_steps: 20,
        travel: [400.0, 300.0],
        ..SimConfig::default()
    }
}

/// Identified with `settings` set first, then homed.
fn homed_with(config: SimConfig, settings: &[(u16, &str)]) -> Bench {
    let mut bench = Bench::new(config);
    for (n, v) in settings {
        bench.sim.set_setting(*n, v);
    }
    assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
    assert_eq!(bench.state(), State::Alarm);
    bench.app(Message::Home);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    run(&mut bench, Duration::from_millis(300));
    bench
}

fn homed() -> Bench {
    homed_with(homed_config(), &[])
}

fn tick(bench: &mut Bench) {
    bench.step();
    bench.apply(Event::Clock(bench.at));
}

fn run(bench: &mut Bench, time: Duration) {
    let end = bench.at + time;
    while bench.at < end {
        tick(bench);
    }
}

fn until(bench: &mut Bench, done: impl Fn(&Bench) -> bool, max: usize) -> bool {
    for _ in 0..max {
        if done(bench) {
            return true;
        }
        tick(bench);
    }
    done(bench)
}

fn pulse(power: u16, length: u16) -> Pulse {
    Pulse {
        power_permille: PulsePower::new(power).unwrap(),
        length_ms: PulseLength::new(length).unwrap(),
    }
}

fn go(bench: &mut Bench, x: f64, y: f64, pulse: Option<Pulse>) {
    bench.apply(Event::Clock(bench.at));
    bench.apply(Event::Message(
        Message::GoToPoint {
            x: Millimetres(x),
            y: Millimetres(y),
            pulse,
        },
        Peer::host(),
    ));
}

fn target(bench: &Bench) -> Option<[f64; 2]> {
    match bench.status() {
        Message::Status { target, .. } => target.map(|t| t.map(|m| m.0)),
        _ => None,
    }
}

fn homed_flag(bench: &Bench) -> bool {
    matches!(bench.status(), Message::Status { homed: true, .. })
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn golden() -> String {
    let path = format!(
        "{}/tests/fixtures/grbl/focus_pulse.gcode",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn p5_the_go_to_point_and_its_pulse_write_the_golden_bytes_in_order() {
    let mut bench = homed();
    let mark = bench.sim.lines.len();
    let log_mark = bench.sim.log.len();
    go(&mut bench, -100.5, -50.0, Some(pulse(10, 300)));
    assert_eq!(bench.state(), State::Focusing);
    assert_eq!(target(&bench), Some([-100.5, -50.0]));
    assert!(until(&mut bench, |b| b.state() == State::Pulsing, 1_000));
    let pos = bench.sim.rx_pos_for_tests();
    assert!((pos[0] + 100.5).abs() < 0.05 && (pos[1] + 50.0).abs() < 0.05);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    let mut text = bench.sim.lines[mark..].join("\n");
    text.push('\n');
    assert_eq!(text, golden());
    // The block went out in one piece: nothing in between, not even a `?`.
    let log = &bench.sim.log[log_mark..];
    assert!(contains(log, b"G1 M3 S10\nG4 P0.300\nM5 S0\n"));
    assert_eq!(bench.sim.dwells, [0.3]);
    // The planner takes the block's `M5` one step after its `ok`.
    run(&mut bench, Duration::from_millis(50));
    // The beam was lit for the dwell (30 steps of 10 ms) and the parsing of its two neighbours.
    assert!(
        (30..=40).contains(&bench.sim.lit_steps),
        "{}",
        bench.sim.lit_steps
    );
    assert!(!bench.sim.laser_lit());
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert_eq!(bench.detail(), None);
    assert_eq!(target(&bench), None);
    assert_eq!(bench.sim.faults, Vec::<String>::new());
}

#[test]
fn c9_nothing_before_the_block_carries_a_power() {
    let mut bench = homed();
    let mark = bench.sim.lines.len();
    go(&mut bench, -20.0, -20.0, Some(pulse(10, 50)));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    let lines = &bench.sim.lines[mark..];
    let block = lines.iter().position(|l| l.starts_with("G1 M3")).unwrap();
    assert_eq!(lines[..2], ["M5", "M9"]);
    for line in &lines[..block] {
        assert!(!line.contains("M3") && !line.contains("M4"), "{line}");
        assert!(!line.contains(" S") || line.contains("S0"), "{line}");
    }
}

#[test]
fn p5_s_follows_dollar_30_and_below_one_no_block_is_written() {
    let mut bench = homed_with(homed_config(), &[(30, "255")]);
    let mark = bench.sim.lines.len();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 300)));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    assert!(bench.sim.lines[mark..].contains(&"G1 M3 S2".to_owned()));

    let mark = bench.sim.lines.len();
    let lit = bench.sim.lit_steps;
    go(&mut bench, -12.0, -10.0, Some(pulse(1, 300)));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    let lines = &bench.sim.lines[mark..];
    assert!(lines.iter().all(|l| !l.starts_with("G1 M3")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("$J=G90")),
        "the head moved"
    );
    assert_eq!(bench.sim.lit_steps, lit);
    assert_eq!(bench.detail(), None);
}

#[test]
fn c35c_with_laser_mode_off_the_head_moves_and_no_block_is_written() {
    let mut bench = homed_with(homed_config(), &[(32, "0")]);
    let mark = bench.sim.lines.len();
    go(&mut bench, -30.0, -30.0, Some(pulse(10, 300)));
    assert_eq!(bench.state(), State::Focusing);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    let lines = &bench.sim.lines[mark..];
    assert!(lines.iter().any(|l| l.starts_with("$J=G90")));
    assert!(lines.iter().all(|l| !l.contains("M3")), "{lines:?}");
    assert_eq!(bench.sim.lit_steps, 0);
}

#[test]
fn p4_a_target_beyond_the_travel_is_clamped_to_its_edge() {
    let mut bench = homed();
    let mark = bench.sim.lines.len();
    go(&mut bench, -450.0, 20.0, None);
    assert_eq!(target(&bench), Some([-400.0, 0.0]));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 2_000));
    assert!(bench.sim.lines[mark..].contains(&"$J=G90 G21 X-400.000 Y0.000 F3000".to_owned()));
    assert_eq!(bench.sim.faults, Vec::<String>::new());
}

#[test]
fn c10_stop_mid_move_cancels_the_jog_and_no_block_follows() {
    let mut bench = homed();
    let mark = bench.sim.lines.len();
    let log_mark = bench.sim.log.len();
    go(&mut bench, -300.0, -200.0, Some(pulse(10, 300)));
    run(&mut bench, Duration::from_millis(200));
    assert_eq!(bench.state(), State::Focusing);
    bench.app(Message::Stop);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    run(&mut bench, Duration::from_millis(500));
    assert!(bench.sim.log[log_mark..].contains(&JOG_CANCEL));
    assert!(!bench.sim.log[log_mark..].contains(&RESET));
    assert!(bench.sim.lines[mark..].iter().all(|l| !l.contains("M3")));
    assert_eq!(bench.sim.lit_steps, 0);
    assert!(homed_flag(&bench));
    assert_eq!(bench.alarm(), None);
}

#[test]
fn c10_stop_mid_pulse_resets_writes_m5_m9_raises_no_alarm_and_keeps_homed() {
    let mut bench = homed();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 2_000)));
    assert!(until(&mut bench, |b| b.state() == State::Pulsing, 1_000));
    assert!(until(&mut bench, |b| b.sim.dwell.is_some(), 100));
    run(&mut bench, Duration::from_millis(100));
    let lit = bench.sim.lit_steps;
    assert!(lit > 0);
    let mark = bench.sim.lines.len();
    let log_mark = bench.sim.log.len();
    bench.app(Message::Stop);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    run(&mut bench, Duration::from_millis(300));
    assert!(bench.sim.log[log_mark..].contains(&RESET));
    assert_eq!(bench.sim.lines[mark..], ["M5", "M9"]);
    assert!(bench.sim.lit_steps <= lit + 2, "the beam went off at once");
    assert!(!bench.sim.alarm);
    assert_eq!(bench.alarm(), None);
    assert!(
        homed_flag(&bench),
        "the reset of a standing head keeps homed"
    );
}

#[test]
fn c35c_an_open_door_mid_pulse_runs_the_stop() {
    let mut bench = homed();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 2_000)));
    assert!(until(&mut bench, |b| b.sim.dwell.is_some(), 1_000));
    let log_mark = bench.sim.log.len();
    bench.sim.door = true;
    assert!(until(&mut bench, |b| b.state() != State::Pulsing, 500));
    assert!(bench.sim.log[log_mark..].contains(&RESET));
    assert!(!bench.sim.laser_lit());
}

#[test]
fn c35c_a_pulse_that_does_not_end_within_its_length_and_one_second_is_stopped() {
    let mut bench = homed();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 300)));
    assert!(until(&mut bench, |b| b.sim.dwell.is_some(), 1_000));
    // The controller never ends its dwell.
    bench.sim.dwell = Some(1_000_000);
    let started = bench.at;
    assert!(until(&mut bench, |b| b.state() != State::Pulsing, 300));
    let took = bench.at.saturating_sub(started);
    assert!(
        took >= Duration::from_millis(1_250) && took <= Duration::from_millis(1_500),
        "{took:?}"
    );
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(bench.detail().as_deref(), Some("The pulse did not end"));
    assert!(!bench.sim.laser_lit());
}

#[test]
fn c35c_a_lost_cable_mid_pulse_leaves_the_block_to_end_itself() {
    let mut bench = homed();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 300)));
    assert!(until(&mut bench, |b| b.sim.dwell.is_some(), 1_000));
    bench.plugged = false;
    // The simulator, cut off, still runs its dwell and the block's own `M5 S0`.
    for _ in 0..40 {
        bench.sim.step();
    }
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert!(!bench.sim.laser_lit());
    assert!(until(&mut bench, |b| b.state() == State::NoController, 500));
    assert!(bench.last_write.starts_with(&[RESET]));
}

#[test]
fn j6_error_15_ends_the_move_with_its_text_and_no_pulse() {
    let mut bench = homed();
    let n = u32::try_from(bench.sim.lines.len()).unwrap() + 3;
    bench.sim.config.error_on_line = Some((n, 15));
    let mark = bench.sim.lines.len();
    go(&mut bench, -10.0, -10.0, Some(pulse(10, 300)));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    assert!(bench.detail().unwrap().contains("error 15"));
    assert!(bench.sim.lines[mark..].iter().all(|l| !l.contains("M3")));
}
