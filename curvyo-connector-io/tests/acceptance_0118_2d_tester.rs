//! Independent acceptance tests for `specs/0118-machine-control` slice 2d (criteria 10, 12, 34,
//! 35 a to e, 36; `adrs.md` P5, P6, P9, P11), written from the specification before reading the
//! implementation. The GRBL simulator runs in the bench's stepped time (no sleeps); the service
//! tests run the connector over TCP with owner and member. Every wait is by condition.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    missing_docs
)]

mod common;

use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::SimConfig;
use common::service::{code_of, deadline, service};
use common::wait_for;
use curvyo_connector_core::{ErrorCode, Message, Role, State};

/// Unlocked and homed, `Idle`, on a bench with the simulator's `settings` applied before the
/// connector identified the controller.
fn homed(config: SimConfig, settings: &[(u16, &str)]) -> Bench {
    let mut b = bench(config, settings);
    b.app(Message::Unlock);
    assert!(
        b.run_until(|b| b.state() == State::Idle, 500),
        "unlock: {:?}",
        b.state()
    );
    b.app(Message::Home);
    assert!(
        b.run_until(|b| homed_flag(b) && b.state() == State::Idle, 50_000),
        "homing did not end: {:?}",
        b.state()
    );
    b
}

/// Identified with the simulator's settings applied first.
fn bench(config: SimConfig, settings: &[(u16, &str)]) -> Bench {
    let mut b = Bench::new(config);
    for (number, value) in settings {
        b.sim.set_setting(*number, value);
    }
    assert!(b.run_until(|b| b.state() != State::NoController, 1_000));
    b
}

fn homed_flag(b: &Bench) -> bool {
    serde_json::to_value(b.status()).unwrap()["homed"] == serde_json::json!(true)
}

fn travel_config() -> SimConfig {
    SimConfig {
        travel: [380.0, 300.0],
        soft_limits: true,
        homing_lock: true,
        mpos_reports: true,
        ..SimConfig::default()
    }
}

/// A `GoToPoint` as the app sends it (the wire form of `command_go_to_point*.json`).
fn go_to(x: f64, y: f64, pulse: Option<(u16, u16)>) -> Message {
    let mut value = serde_json::json!({ "type": "GoToPoint", "x": x, "y": y });
    if let Some((power_permille, length_ms)) = pulse {
        value["pulse"] = serde_json::json!({
            "power_permille": power_permille,
            "length_ms": length_ms,
        });
    }
    serde_json::from_value(value).unwrap()
}

/// The simulator's bytes received so far, without real-time bytes and spaces.
fn squeezed(b: &Bench) -> String {
    b.lines_after(0).replace(' ', "")
}

/// The one-write pulse block of criterion 35 b, as the simulator receives it.
fn pulse_block(s: u32, dwell: &str) -> String {
    format!("G1M3S{s}\nG4P{dwell}\nM5S0")
}

fn jog_lines(b: &Bench) -> Vec<String> {
    b.sim
        .lines
        .iter()
        .filter(|l| l.starts_with("$J="))
        .cloned()
        .collect()
}

fn lines_with(b: &Bench, needle: &str) -> Vec<String> {
    b.sim
        .lines
        .iter()
        .filter(|l| l.replace(' ', "").contains(needle))
        .cloned()
        .collect()
}

/// Every status the connector pushed while it was `Focusing`, as JSON.
fn focusing_statuses(b: &Bench) -> Vec<serde_json::Value> {
    b.pushed
        .iter()
        .map(|p| serde_json::to_value(&p.status).unwrap())
        .filter(|v| v["state"]["name"] == "Focusing")
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Criterion 34 and 9: the move is laser off; M5 and M9 come before the first motion byte
// ---------------------------------------------------------------------------------------------

#[test]
fn a_go_to_point_moves_laser_off_with_an_absolute_jog_at_ten_mm_feed() {
    let mut b = homed(travel_config(), &[]);
    b.app(go_to(-50.0, -20.0, None));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            50_000
        ),
        "the move did not end: {:?}",
        b.state()
    );
    let lines = b.sim.lines.clone();
    let jog = lines
        .iter()
        .position(|l| l.starts_with("$J="))
        .expect("a jog line");
    assert!(
        lines[jog].starts_with("$J=G90"),
        "absolute jog: {}",
        lines[jog]
    );
    assert!(
        lines[jog].contains("X-50") && lines[jog].contains("Y-20"),
        "{}",
        lines[jog]
    );
    assert!(
        lines[jog].contains("F3000"),
        "10 mm jog feed: {}",
        lines[jog]
    );
    let laser_off = lines.iter().position(|l| l == "M5").expect("an M5 line");
    let m9 = lines.iter().position(|l| l == "M9").expect("an M9 line");
    assert!(
        laser_off < jog && m9 < jog,
        "M5 and M9 before the jog: {lines:?}"
    );
    for line in &lines[..jog] {
        assert!(!line.contains("M3") && !line.contains("M4"), "{line}");
    }
    assert!(
        lines_with(&b, "M3").is_empty() && lines_with(&b, "M4").is_empty(),
        "no M3 or M4 in a move without a pulse: {lines:?}"
    );
    for line in lines.iter().filter(|l| l.starts_with("G1 X")) {
        assert!(line.ends_with("S0"), "no S above 0 in a motion: {line}");
    }
    assert!(b.alarm().is_none(), "no alarm: {:?}", b.alarm());
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 b and P5: the pulse block after arrival, one write, M3 and bare G1, M5 S0 last
// ---------------------------------------------------------------------------------------------

#[test]
fn the_pulse_block_follows_the_arrival_as_one_block_with_s_from_dollar_30() {
    let mut b = homed(travel_config(), &[]);
    b.app(go_to(-50.0, -20.0, Some((10, 300))));
    assert!(
        b.run_until(
            |b| !lines_with(b, "M5S0").is_empty() && b.state() == State::Idle,
            50_000
        ),
        "the pulse did not end: {:?} {:?} {:?}",
        b.state(),
        b.sim.lines,
        b.detail()
    );
    let text = squeezed(&b);
    // $30 is 1000 in the simulator: S = floor(10 permille of 1000) = 10.
    assert!(
        text.contains(&format!("{}\n", pulse_block(10, "0.300"))),
        "the block, one write, M5 S0 last: {text}"
    );
    let jog = text.find("$J=").expect("the move");
    let block = text.find("G1M3S10").expect("the block");
    assert!(jog < block, "the block only after the arrival");
    assert_eq!(
        text.matches("M3").count(),
        1,
        "one M3, in the block: {text}"
    );
    assert!(
        lines_with(&b, "M4").is_empty(),
        "never M4 for a pulse (35 b): {:?}",
        b.sim.lines
    );
    assert!(b.alarm().is_none(), "no alarm: {:?}", b.alarm());
}

#[test]
fn the_pulse_power_is_a_share_of_dollar_30_rounded_down_and_capped_at_five_percent() {
    // $30 = 255: 10 permille gives floor(2.55) = 2; 50 permille gives floor(12.75) = 12.
    let mut b = homed(travel_config(), &[(30, "255")]);
    b.app(go_to(-10.0, -10.0, Some((10, 300))));
    assert!(
        b.run_until(
            |b| !lines_with(b, "M5S0").is_empty() && b.state() == State::Idle,
            50_000
        ),
        "{:?}",
        b.state()
    );
    assert!(
        squeezed(&b).contains("G1M3S2\nG4P0.300\nM5S0"),
        "{}",
        squeezed(&b)
    );

    let mut c = homed(travel_config(), &[(30, "255")]);
    c.app(go_to(-10.0, -10.0, Some((50, 500))));
    assert!(
        c.run_until(
            |c| !lines_with(c, "M5S0").is_empty() && c.state() == State::Idle,
            50_000
        ),
        "{:?}",
        c.state()
    );
    assert!(
        squeezed(&c).contains("G1M3S12\nG4P0.500\nM5S0"),
        "{}",
        squeezed(&c)
    );
}

#[test]
fn a_pulse_below_s_one_is_left_out_and_the_head_still_arrives() {
    // $30 = 255: 1 permille gives floor(0.255) = 0, below 1, so no block is written.
    let mut b = homed(travel_config(), &[(30, "255")]);
    b.app(go_to(-30.0, -15.0, Some((1, 300))));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            50_000
        ),
        "{:?}",
        b.state()
    );
    assert!(
        lines_with(&b, "M3").is_empty(),
        "no pulse below S 1: {:?}",
        b.sim.lines
    );
    assert!(
        lines_with(&b, "G4P").is_empty(),
        "no dwell: {:?}",
        b.sim.lines
    );
}

#[test]
fn with_laser_mode_off_the_head_moves_and_no_pulse_is_written() {
    // $32 = 0 (laser mode off): the head still moves, the pulse is left out (ux-notes W55).
    let mut b = homed(travel_config(), &[(32, "0")]);
    b.app(go_to(-40.0, -25.0, Some((10, 300))));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            50_000
        ),
        "{:?}",
        b.state()
    );
    assert!(
        lines_with(&b, "M3").is_empty(),
        "no M3 with $32=0: {:?}",
        b.sim.lines
    );
    assert!(
        lines_with(&b, "G4P").is_empty(),
        "no dwell with $32=0: {:?}",
        b.sim.lines
    );
}

#[test]
fn a_move_without_a_pulse_writes_no_pulse_at_all() {
    let mut b = homed(travel_config(), &[]);
    b.app(go_to(-20.0, -10.0, None));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            50_000
        ),
        "{:?}",
        b.state()
    );
    assert!(lines_with(&b, "M3").is_empty() && lines_with(&b, "G4P").is_empty());
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 c: refusals, nothing written
// ---------------------------------------------------------------------------------------------

#[test]
fn a_point_with_the_controller_not_homed_is_refused_and_nothing_moves() {
    let mut b = bench(
        SimConfig {
            homing_lock: false,
            ..SimConfig::default()
        },
        &[],
    );
    assert!(!homed_flag(&b));
    b.app(go_to(-10.0, -10.0, Some((10, 300))));
    b.run_for(Duration::from_millis(300));
    assert!(
        jog_lines(&b).is_empty(),
        "no jog when not homed: {:?}",
        b.sim.lines
    );
    assert!(lines_with(&b, "M3").is_empty(), "no pulse when not homed");
    assert_ne!(b.state(), State::Focusing);
}

#[test]
fn a_point_in_alarm_is_refused_and_the_laser_stays_off() {
    // Locked at power-up: the connector is in Alarm and GRBL refuses motion (criterion 8).
    let mut b = bench(travel_config(), &[]);
    assert_eq!(b.state(), State::Alarm);
    b.app(go_to(-10.0, -10.0, Some((10, 300))));
    b.run_for(Duration::from_millis(300));
    assert!(
        jog_lines(&b).is_empty(),
        "no jog in Alarm: {:?}",
        b.sim.lines
    );
    assert!(lines_with(&b, "M3").is_empty(), "no pulse in Alarm");
    assert_eq!(b.state(), State::Alarm);
}

#[test]
fn out_of_range_values_are_refused_before_any_byte_is_written() {
    let refused = [
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":0,"length_ms":300}}"#,
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":51,"length_ms":300}}"#,
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":10,"length_ms":49}}"#,
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":10,"length_ms":2001}}"#,
        r#"{"type":"GoToPoint","x":10001,"y":1}"#,
    ];
    for text in refused {
        assert!(
            serde_json::from_str::<Message>(text).is_err(),
            "decoded but must be refused: {text}"
        );
    }
    let accepted = [
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":1,"length_ms":50}}"#,
        r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":50,"length_ms":2000}}"#,
    ];
    for text in accepted {
        assert!(serde_json::from_str::<Message>(text).is_ok(), "{text}");
    }
    // The refused values never reach the connector: a homed bench with no motion so far.
    let b = homed(travel_config(), &[]);
    assert!(jog_lines(&b).is_empty(), "nothing moved: {:?}", b.sim.lines);
    assert!(lines_with(&b, "M3").is_empty(), "no pulse written");
}

#[test]
fn a_pulse_of_the_capped_length_and_power_is_accepted_without_a_clamp_error() {
    let mut b = homed(travel_config(), &[(30, "1000")]);
    b.app(go_to(-10.0, -10.0, Some((50, 2000))));
    assert!(
        b.run_until(
            |b| !lines_with(b, "M5S0").is_empty() && b.state() == State::Idle,
            80_000
        ),
        "{:?}",
        b.state()
    );
    assert!(
        squeezed(&b).contains("G1M3S50\nG4P2.000\nM5S0"),
        "{}",
        squeezed(&b)
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 10 and 35 c: Stop during the move cancels the jog; Stop during the pulse runs G12
// ---------------------------------------------------------------------------------------------

#[test]
fn stop_during_the_move_cancels_the_jog_and_no_pulse_follows() {
    let mut b = homed(
        SimConfig {
            steps_per_motion: 400,
            ..travel_config()
        },
        &[],
    );
    b.app(go_to(-300.0, -200.0, Some((10, 300))));
    assert!(
        b.run_until(|b| !jog_lines(b).is_empty(), 1_000),
        "the jog did not start"
    );
    b.run_for(Duration::from_millis(50));
    assert_eq!(b.state(), State::Focusing, "still moving");
    b.app(Message::Stop);
    assert!(
        b.run_until(|b| b.state() != State::Focusing, 5_000),
        "Stop did not end the move: {:?}",
        b.state()
    );
    assert!(
        b.sim.log.contains(&0x85),
        "the jog cancel 0x85 reached the controller"
    );
    b.run_for(Duration::from_millis(500));
    assert!(
        lines_with(&b, "G4P").is_empty(),
        "no pulse after a stopped move"
    );
    assert!(
        lines_with(&b, "M3").is_empty(),
        "no M3 after a stopped move"
    );
    assert!(b.alarm().is_none(), "no alarm: {:?}", b.alarm());
    assert_ne!(b.state(), State::Alarm);
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 c: a pulse is never sent with a job, after another motion, or in a stop
// ---------------------------------------------------------------------------------------------

#[test]
fn no_pulse_follows_a_jog_or_a_home() {
    let mut b = homed(travel_config(), &[]);
    let jog: Message =
        serde_json::from_str(r#"{"type":"Jog","axis":"X","direction":"Plus","step":"1"}"#).unwrap();
    b.app(jog);
    assert!(
        b.run_until(|b| b.state() == State::Idle, 50_000),
        "{:?}",
        b.state()
    );
    b.app(Message::Home);
    assert!(b.run_until(|b| b.state() == State::Idle && homed_flag(b), 50_000));
    assert!(
        lines_with(&b, "G4P").is_empty(),
        "no dwell after a jog or a home"
    );
    assert!(
        lines_with(&b, "M3").is_empty(),
        "no M3 after a jog or a home"
    );
}

#[test]
fn a_point_while_a_job_runs_is_refused_and_no_pulse_is_written() {
    let mut b = homed(travel_config(), &[]);
    let text = "G21\nG90\nM4 S0\nG1 X200 Y200 F1000 S500\nG1 X0 Y0 F1000 S500\nM5\n";
    b.send(text);
    b.app(Message::Start {
        job_id: common::grbl_bench::job_id(1),
    });
    assert!(
        b.run_until(|b| b.state() == State::Running, 5_000),
        "{:?}",
        b.state()
    );
    b.app(go_to(-10.0, -10.0, Some((10, 300))));
    b.run_for(Duration::from_millis(200));
    assert!(jog_lines(&b).is_empty(), "no move during a job");
    assert!(lines_with(&b, "G4P").is_empty(), "no pulse during a job");
}

// ---------------------------------------------------------------------------------------------
// Criterion 34: the target is the connector's clamped target, reported in Status.target
// ---------------------------------------------------------------------------------------------

#[test]
fn a_target_beyond_the_travel_is_clamped_and_reported_as_status_target() {
    let mut b = homed(
        SimConfig {
            steps_per_motion: 300,
            ..travel_config()
        },
        &[],
    );
    b.app(go_to(-1000.0, -1000.0, None));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            80_000
        ),
        "{:?}",
        b.state()
    );
    let statuses = focusing_statuses(&b);
    let target = statuses
        .iter()
        .rev()
        .find_map(|s| s.get("target").cloned())
        .expect("Status.target while Focusing");
    let x = target[0].as_f64().unwrap();
    let y = target[1].as_f64().unwrap();
    assert!(
        x.abs() <= 380.0 + 1e-6 && x.abs() > 1.0,
        "clamped x inside the travel: {x}"
    );
    assert!(
        y.abs() <= 300.0 + 1e-6 && y.abs() > 1.0,
        "clamped y inside the travel: {y}"
    );
    for line in jog_lines(&b) {
        let value = line
            .split('X')
            .nth(1)
            .and_then(|rest| rest.split('Y').next())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .expect("an X word");
        assert!(
            value.abs() <= 380.0 + 1e-6,
            "the head never leaves the travel: {line}"
        );
    }
    let after = serde_json::to_value(b.status()).unwrap();
    assert!(
        after.get("target").is_none(),
        "target left out once the move ended: {after}"
    );
}

#[test]
fn a_point_inside_the_travel_is_reported_at_its_own_place() {
    let mut b = homed(travel_config(), &[]);
    b.app(go_to(-60.0, -40.0, None));
    assert!(b.run_until(
        |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
        50_000
    ));
    let statuses = focusing_statuses(&b);
    let target = statuses
        .iter()
        .find_map(|s| s.get("target").cloned())
        .expect("Status.target while Focusing");
    assert!((target[0].as_f64().unwrap() + 60.0).abs() < 1e-9);
    assert!((target[1].as_f64().unwrap() + 40.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------------------------
// Criterion 12 and 380 A: owner only; the log lines; the member is refused over TCP
// ---------------------------------------------------------------------------------------------

#[test]
fn a_member_is_refused_go_to_point_and_the_owner_is_not() {
    let svc = service();
    let (mut setup, token) = svc.setup(1000);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    owner.send(&Message::Home).unwrap();
    wait_for(&mut owner, |m| {
        serde_json::to_value(m).unwrap()["homed"] == serde_json::json!(true)
    });
    let answer = member
        .request(&go_to(-10.0, -10.0, Some((10, 300))), deadline())
        .unwrap();
    assert_eq!(
        code_of(&answer),
        Some(ErrorCode::OwnerRequired),
        "a member's Go to point is refused: {answer:?}"
    );
    owner.send(&go_to(-15.0, -5.0, Some((10, 300)))).unwrap();
    wait_for(&mut owner, |m| {
        serde_json::to_value(m).unwrap()["state"]["name"] == "Focusing"
    });
    wait_for(&mut owner, |m| {
        serde_json::to_value(m).unwrap()["state"]["name"] == "Idle"
    });
    let log = svc.file("jobs.log");
    assert_eq!(
        log.matches("Focus point:").count(),
        1,
        "one Focus point line for the owner's move, none for the refused member: {log}"
    );
    assert_eq!(
        log.matches("Focus pulse:").count(),
        1,
        "one Focus pulse line for the pulse block: {log}"
    );
}

#[test]
fn every_focusing_status_carries_the_clamped_target_not_the_typed_one() {
    // criterion 34: the canvas marks the connector's clamped target; no status of the move may
    // carry the typed point beyond the travel (the first status is pushed at once, before the
    // connector has clamped it, so this is strict).
    let mut b = homed(
        SimConfig {
            steps_per_motion: 300,
            ..travel_config()
        },
        &[],
    );
    b.app(go_to(-1000.0, -1000.0, None));
    assert!(
        b.run_until(
            |b| b.state() == State::Idle && !jog_lines(b).is_empty(),
            80_000
        ),
        "{:?}",
        b.state()
    );
    for status in focusing_statuses(&b) {
        if let Some(target) = status.get("target") {
            let x = target[0].as_f64().unwrap();
            let y = target[1].as_f64().unwrap();
            assert!(
                x.abs() <= 380.0 + 1e-6 && y.abs() <= 300.0 + 1e-6,
                "a Focusing status with the typed target outside the travel: {status}"
            );
        }
    }
}

#[test]
fn stop_during_the_pulse_runs_the_g12_sequence_and_raises_no_alarm() {
    let mut b = homed(travel_config(), &[]);
    b.app(go_to(-20.0, -10.0, Some((10, 2000))));
    assert!(
        b.run_until(|b| !lines_with(b, "G4P2.000").is_empty(), 50_000),
        "the pulse block was not written: {:?}",
        b.state()
    );
    let mark = b.sim.log.len();
    b.app(Message::Stop);
    assert!(
        b.run_until(
            |b| b.state() != State::Pulsing && b.state() != State::Focusing,
            5_000
        ),
        "Stop did not end the pulse: {:?}",
        b.state()
    );
    let after = &b.sim.log[mark..];
    assert!(
        after.contains(&b'!') && after.contains(&0x18),
        "hold and reset after the block (G12): after={after:?} lines={:?} state={:?}",
        b.sim.lines,
        b.state()
    );
    assert!(
        b.alarm().is_none(),
        "no alarm after a Stop in a pulse: {:?}",
        b.alarm()
    );
    assert_ne!(b.state(), State::Alarm);
}
