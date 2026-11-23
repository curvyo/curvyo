//! Independent acceptance tests for `specs/0118-machine-control` second slice 2a (criteria 8 to 14,
//! 17 to 19, 37 and the wire part of 12), written from the specification and `adrs.md` J1 to J9
//! before reading the implementation. The GRBL simulator of `tests/common/grbl_sim` runs in the
//! bench's stepped time (no sleeps); the dead-man is driven by an injected `Event::Clock` per
//! 10 ms step, as the hub's tick does. The service tests run the connector over TCP with
//! `--fake-controller`-equivalent settings and wait by condition only.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    missing_docs
)]

mod common;

use std::time::{Duration, Instant};

use common::grbl_bench::Bench;
use common::grbl_sim::SimConfig;
use common::service::{self, code_of};
use common::{WAIT, state_of, wait_for};
use curvyo_connector_core::{Event, Message, Role};
use curvyo_connector_io::client::Connection;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

const LAYOUT_JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

fn dbg<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn st(b: &Bench) -> String {
    dbg(&b.state())
}

fn wire(value: Value) -> Message {
    serde_json::from_value(value).expect("the wire shape of J1")
}

fn jog(axis: &str, direction: &str, step: &str) -> Message {
    wire(json!({"type": "Jog", "axis": axis, "direction": direction, "step": step}))
}

fn hold(axis: &str, direction: &str, step: &str) -> Message {
    wire(json!({"type": "Jog", "axis": axis, "direction": direction, "step": step, "hold": true}))
}

fn keep() -> Message {
    wire(json!({"type": "JogKeep"}))
}

fn jog_stop() -> Message {
    wire(json!({"type": "JogStop"}))
}

fn json_of(m: &Message) -> Value {
    serde_json::to_value(m).unwrap()
}

/// `x` and `y` of a position given as an object or as an array.
fn xy(v: &Value) -> (f64, f64) {
    if let Some(o) = v.as_object() {
        (o["x"].as_f64().unwrap(), o["y"].as_f64().unwrap())
    } else {
        (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.01
}

fn mpos(b: &Bench) -> (f64, f64) {
    xy(&json_of(&b.status())["machine_position"])
}

fn homed_flag(b: &Bench) -> bool {
    json_of(&b.status())["homed"] == json!(true)
}

fn jog_lines(b: &Bench) -> Vec<String> {
    b.sim
        .lines
        .iter()
        .filter(|l| l.starts_with("$J="))
        .cloned()
        .collect()
}

/// Whether a `$J=` line is `$J=G91 G21 <axis><distance> F<feed>` with these numbers (GRBL's own
/// spelling of the number, `0.1` or `0.100`, does not matter).
fn is_jog(line: &str, axis: char, distance: f64, feed: f64) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() != 4 || words[0] != "$J=G91" || words[1] != "G21" {
        return false;
    }
    let (Some(a), Some(f)) = (words[2].strip_prefix(axis), words[3].strip_prefix('F')) else {
        return false;
    };
    let (Ok(a), Ok(f)) = (a.parse::<f64>(), f.parse::<f64>()) else {
        return false;
    };
    near(a, distance) && near(f, feed)
}

#[allow(clippy::naive_bytecount)] // the simulator log is small; a count is all that is needed
fn cancel_bytes(b: &Bench) -> usize {
    b.sim.log.iter().filter(|x| **x == 0x85).count()
}

/// One step of both sides, with the injected clock the hub would send.
fn tick(b: &mut Bench) {
    b.step();
    b.apply(Event::Clock(b.at));
}

fn ticks_for(b: &mut Bench, time: Duration) {
    let end = b.at + time;
    while b.at < end {
        tick(b);
    }
}

fn until_idle(b: &mut Bench, max: usize) -> bool {
    for _ in 0..max {
        if st(b) == "Idle" {
            return true;
        }
        tick(b);
    }
    st(b) == "Idle"
}

/// A message at the current injected time.
fn send(b: &mut Bench, m: Message) {
    b.apply(Event::Clock(b.at));
    b.app(m);
}

/// One press of a step: the move, then the head at rest again.
fn press(b: &mut Bench, axis: &str, direction: &str, step: &str) {
    send(b, jog(axis, direction, step));
    assert!(
        until_idle(b, 50_000),
        "the step did not end: {} detail {:?} alarm {:?} sim alarm {}",
        st(b),
        b.detail(),
        b.alarm(),
        b.sim.alarm
    );
}

fn homed_bench(config: SimConfig) -> Bench {
    let mut b = Bench::connected(config);
    b.app(Message::Unlock);
    assert!(b.run_until(|b| st(b) == "Idle", 500), "unlock: {}", st(&b));
    send(&mut b, Message::Home);
    assert!(
        b.run_until(|b| homed_flag(b) && st(b) == "Idle", 50_000),
        "homing did not end: {}",
        st(&b)
    );
    b
}

// ---------------------------------------------------------------------------------------------
// Criterion 9: the laser is off for the whole motion
// ---------------------------------------------------------------------------------------------

#[test]
fn m5_and_m9_come_before_the_first_jog_byte_and_no_jog_line_powers_the_laser() {
    let mut b = Bench::connected(SimConfig::default());
    send(&mut b, jog("X", "Plus", "1"));
    assert!(
        b.run_until(|b| !jog_lines(b).is_empty(), 1_000),
        "no jog line was written"
    );
    let lines = b.sim.lines.clone();
    let first = lines.iter().position(|l| l.starts_with("$J=")).unwrap();
    assert!(
        lines[..first].iter().any(|l| l == "M5"),
        "M5 first: {lines:?}"
    );
    assert!(
        lines[..first].iter().any(|l| l == "M9"),
        "M9 first: {lines:?}"
    );
    for line in jog_lines(&b) {
        for word in line.split_whitespace() {
            assert!(
                !(word.starts_with('M') || word.starts_with('S')),
                "a jog line carries no M or S word: {line}"
            );
            assert!(!word.starts_with("M3") && !word.starts_with("M4"), "{line}");
        }
    }
    assert!(until_idle(&mut b, 50_000));
}

// ---------------------------------------------------------------------------------------------
// Criteria 15, 16: steps, feeds and the direction on the machine axes
// ---------------------------------------------------------------------------------------------

#[test]
fn each_step_writes_its_jog_line_with_its_feed_and_sign() {
    // (axis, direction, step, distance on the machine axis, feed)
    let cases = [
        ("X", "Plus", "0.1", 0.1, 300.0),
        ("X", "Minus", "1", -1.0, 1000.0),
        ("Y", "Plus", "10", 10.0, 3000.0),
        ("Y", "Minus", "50", -50.0, 3000.0),
    ];
    let mut b = Bench::connected(SimConfig::default());
    for (axis, direction, step, distance, feed) in cases {
        press(&mut b, axis, direction, step);
        let axis_char = axis.chars().next().unwrap();
        assert!(
            jog_lines(&b)
                .iter()
                .any(|l| is_jog(l, axis_char, distance, feed)),
            "{axis} {direction} {step}: no line with {distance} at F{feed}: {:?}",
            jog_lines(&b)
        );
    }
}

#[test]
fn a_held_jog_repeats_only_steps_of_its_own_feed() {
    let mut b = Bench::connected(SimConfig::default());
    send(&mut b, hold("X", "Plus", "10"));
    ticks_for(&mut b, Duration::from_millis(200));
    send(&mut b, keep());
    ticks_for(&mut b, Duration::from_millis(100));
    send(&mut b, jog_stop());
    assert!(until_idle(&mut b, 50_000));
    let lines = jog_lines(&b);
    assert_ne!(lines.len(), 0, "the held jog wrote no line");
    // A held jog streams increments of feed x 0.1 s (adrs J6), at the step's feed.
    for line in lines {
        let words: Vec<&str> = line.split_whitespace().collect();
        let distance: f64 = words[2][1..].parse().unwrap();
        assert!(
            words[3] == "F3000" && distance > 0.0 && distance <= 10.0,
            "{line}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 17, 18: the travel clamp, unhomed motion, error:15
// ---------------------------------------------------------------------------------------------

fn travel_config() -> SimConfig {
    SimConfig {
        travel: [380.0, 300.0],
        soft_limits: true,
        homing_lock: true,
        mpos_reports: true,
        ..SimConfig::default()
    }
}

#[test]
fn homed_jog_clamps_at_the_travel_box_and_writes_nothing_at_the_edge() {
    let mut b = homed_bench(travel_config());
    let (x0, _) = mpos(&b);
    assert!(
        near(x0, 0.0),
        "the home position is the origin of the test: {x0}"
    );

    let before = jog_lines(&b).len();
    press(&mut b, "X", "Plus", "1");
    assert_eq!(
        jog_lines(&b).len(),
        before,
        "a jog past the origin edge writes nothing"
    );

    for _ in 0..7 {
        press(&mut b, "X", "Minus", "50");
    }
    assert!(
        near(mpos(&b).0, -350.0),
        "seven steps of 50: {:?}",
        mpos(&b)
    );

    press(&mut b, "X", "Minus", "50");
    assert!(
        jog_lines(&b).iter().any(|l| is_jog(l, 'X', -30.0, 3000.0)),
        "the last step is cut to the edge: {:?}",
        jog_lines(&b)
    );
    assert!(near(mpos(&b).0, -380.0), "at the edge: {:?}", mpos(&b));

    let at_edge = jog_lines(&b).len();
    press(&mut b, "X", "Minus", "1");
    assert_eq!(
        jog_lines(&b).len(),
        at_edge,
        "a jog at the edge writes nothing"
    );
}

#[test]
fn unhomed_jog_is_not_clamped_and_the_controllers_error_15_is_shown() {
    let mut b = Bench::connected(SimConfig {
        homing_lock: false,
        ..travel_config()
    });
    assert!(!homed_flag(&b));
    for _ in 0..8 {
        press(&mut b, "X", "Minus", "50");
    }
    let written = jog_lines(&b)
        .iter()
        .filter(|l| is_jog(l, 'X', -50.0, 3000.0))
        .count();
    assert_eq!(
        written,
        8,
        "no position is trusted, so no clamp: {:?}",
        jog_lines(&b)
    );
    let detail = b.detail().unwrap_or_default().to_lowercase();
    assert!(
        detail.contains("error:15") || detail.contains("travel"),
        "error:15 is shown in the state line: {detail:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 10, 18: Stop and release end the motion; the dead-man (injected clock)
// ---------------------------------------------------------------------------------------------

#[test]
fn stop_during_a_held_jog_writes_the_jog_cancel_keeps_the_position_and_is_no_alarm() {
    let mut b = Bench::connected(SimConfig {
        mpos_reports: true,
        ..SimConfig::default()
    });
    send(&mut b, hold("X", "Plus", "1"));
    for i in 0..50 {
        if i % 10 == 0 {
            send(&mut b, keep());
        }
        tick(&mut b);
    }
    assert!(
        mpos(&b).0 > 0.0,
        "the head moved while held: {:?}",
        mpos(&b)
    );
    assert_eq!(cancel_bytes(&b), 0, "no cancel while the keep-alive flows");

    send(&mut b, Message::Stop);
    assert!(
        b.run_until(|b| cancel_bytes(b) > 0, 500),
        "0x85 was not written"
    );
    assert!(
        until_idle(&mut b, 50_000),
        "the controller is not Idle: {}",
        st(&b)
    );
    assert!(b.alarm().is_none(), "no alarm: {:?}", b.alarm());
    let rest = mpos(&b);
    ticks_for(&mut b, Duration::from_millis(300));
    assert!(near(mpos(&b).0, rest.0), "the position is kept");
}

#[test]
fn losing_the_keep_alive_cancels_between_300_and_350_ms_after_the_last_keep() {
    let mut b = Bench::connected(SimConfig::default());
    send(&mut b, hold("X", "Plus", "1"));
    let mut last_keep = Duration::ZERO;
    for i in 0..30 {
        if i % 10 == 0 {
            send(&mut b, keep());
            last_keep = b.at;
        }
        tick(&mut b);
        assert_eq!(cancel_bytes(&b), 0, "no cancel while keeps flow");
    }
    let mut cancelled_at = None;
    for _ in 0..200 {
        tick(&mut b);
        if cancel_bytes(&b) > 0 {
            cancelled_at = Some(b.at);
            break;
        }
    }
    let at = cancelled_at.expect("no cancel after the keep-alive was lost");
    let gap = at.saturating_sub(last_keep);
    assert!(
        gap >= Duration::from_millis(300) && gap <= Duration::from_millis(350),
        "the cancel came {gap:?} after the last keep"
    );
}

#[test]
fn release_cancels_within_100_ms() {
    let mut b = Bench::connected(SimConfig::default());
    send(&mut b, hold("X", "Plus", "1"));
    ticks_for(&mut b, Duration::from_millis(100));
    send(&mut b, keep());
    let released = b.at;
    send(&mut b, jog_stop());
    let before = cancel_bytes(&b);
    assert!(
        b.run_until(|b| cancel_bytes(b) > before, 100),
        "no cancel within 100 ms"
    );
    assert!(b.at.saturating_sub(released) <= Duration::from_millis(100));
}

#[test]
fn a_release_never_discards_a_ready_job() {
    let mut b = Bench::connected(SimConfig::default());
    b.send(LAYOUT_JOB);
    assert_eq!(st(&b), "Ready");
    send(&mut b, hold("Y", "Plus", "1"));
    ticks_for(&mut b, Duration::from_millis(100));
    send(&mut b, jog_stop());
    ticks_for(&mut b, Duration::from_millis(500));
    assert_eq!(st(&b), "Ready", "the held job survives the jog");
}

// ---------------------------------------------------------------------------------------------
// Criteria 8, 11, 20: refusals by controller state; nothing reaches the port
// ---------------------------------------------------------------------------------------------

#[test]
fn a_door_refuses_a_jog_and_no_byte_of_it_reaches_the_port() {
    let mut b = Bench::connected(SimConfig::default());
    b.sim.door = true;
    let read = b.run_until(
        |b| st(b).contains("Door") || b.detail().is_some_and(|d| d.contains("Door")),
        500,
    );
    assert!(
        read,
        "the door is not read: state {} detail {:?}",
        st(&b),
        b.detail()
    );
    let before = jog_lines(&b).len();
    send(&mut b, jog("X", "Plus", "1"));
    ticks_for(&mut b, Duration::from_millis(200));
    assert_eq!(jog_lines(&b).len(), before, "no $J= while the door is open");
}

#[test]
fn an_alarm_refuses_a_jog_but_home_is_still_allowed() {
    let mut b = Bench::connected(SimConfig {
        homing_lock: true,
        ..SimConfig::default()
    });
    assert!(st(&b).contains("Alarm"), "locked at power-up: {}", st(&b));
    send(&mut b, jog("X", "Plus", "1"));
    ticks_for(&mut b, Duration::from_millis(200));
    assert!(jog_lines(&b).is_empty(), "no jog while in alarm");
    send(&mut b, Message::Home);
    ticks_for(&mut b, Duration::from_millis(50));
    assert!(
        b.sim.lines.iter().any(|l| l == "$H"),
        "Home reaches the controller: {:?}",
        b.sim.lines
    );
}

#[test]
fn a_running_job_refuses_a_jog_and_writes_no_jog_line() {
    let mut b = Bench::connected(SimConfig::default());
    let moves: String = (0..200)
        .map(|i| format!("G1 X{} Y0 F1000 S500\n", i % 50))
        .collect();
    let job = format!("G21\nG90\n{moves}M5\n");
    b.start(&job);
    assert!(
        b.run_until(|b| st(b) == "Running", 5_000),
        "the job does not run: {}",
        st(&b)
    );
    send(&mut b, jog("X", "Plus", "1"));
    ticks_for(&mut b, Duration::from_millis(100));
    assert!(jog_lines(&b).is_empty(), "no jog line while a job runs");
}

#[test]
fn nothing_moves_without_a_press() {
    let mut b = Bench::connected(SimConfig::default());
    ticks_for(&mut b, Duration::from_secs(2));
    assert!(jog_lines(&b).is_empty(), "no $J= without a press");
    assert_eq!(cancel_bytes(&b), 0, "no cancel without a press");
    assert_eq!(st(&b), "Idle");
}

// ---------------------------------------------------------------------------------------------
// Criteria 13, 14: homed since connect, machine position and work offset in Status
// ---------------------------------------------------------------------------------------------

#[test]
fn status_carries_homed_the_machine_position_and_the_work_offset() {
    let mut b = Bench::connected(SimConfig {
        mpos_reports: true,
        wco: [5.0, 6.0, 0.0],
        homing_lock: true,
        ..SimConfig::default()
    });
    assert!(!homed_flag(&b), "not homed after connect");
    b.app(Message::Unlock);
    assert!(b.run_until(|b| st(b) == "Idle", 500));
    send(&mut b, Message::Home);
    assert!(b.run_until(|b| homed_flag(b) && st(b) == "Idle", 50_000));
    assert!(
        b.run_until(|b| !json_of(&b.status())["work_offset"].is_null(), 20_000),
        "the work offset is read from WCO"
    );
    let offset = xy(&json_of(&b.status())["work_offset"]);
    assert!(
        near(offset.0, 5.0) && near(offset.1, 6.0),
        "WCO is 5, 6: {offset:?}"
    );

    let before = mpos(&b).0;
    press(&mut b, "X", "Minus", "10");
    assert!(
        near(mpos(&b).0, before - 10.0),
        "machine position follows the jog"
    );
}

// ---------------------------------------------------------------------------------------------
// Service tests: owner-only motion, a second app sees Jogging, a member's Home (37), the log
// ---------------------------------------------------------------------------------------------

fn is_state(m: &Message, want: &str) -> bool {
    matches!(m, Message::Status { .. }) && dbg(&state_of(m)) == want
}

/// Reads for `time` and returns every message seen.
fn drain(connection: &mut Connection, time: Duration) -> Vec<Message> {
    let until = Instant::now() + time;
    let mut seen = Vec::new();
    while Instant::now() < until {
        if let Ok(m) = connection.recv_message(Instant::now() + Duration::from_millis(20)) {
            seen.push(m);
        }
    }
    seen
}

fn errors(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .filter(|m| matches!(m, Message::Error { .. }))
        .map(|m| dbg(&code_of(m)))
        .collect()
}

fn wait_file(svc: &service::Service, name: &str, needle: &str) -> String {
    let until = Instant::now() + WAIT;
    loop {
        let text = svc.file(name);
        if text.contains(needle) || Instant::now() > until {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_member_is_refused_jog_and_keep_and_the_owner_holds_a_jog() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    member.send(&hold("X", "Plus", "1")).unwrap();
    member.send(&keep()).unwrap();
    member.send(&jog("Y", "Plus", "1")).unwrap();
    let seen = drain(&mut member, Duration::from_millis(400));
    let refused = errors(&seen);
    assert!(
        refused
            .iter()
            .filter(|c| c.contains("OwnerRequired"))
            .count()
            >= 2,
        "a member's hold and keep are refused: {refused:?}"
    );
    assert!(
        !seen.iter().any(|m| is_state(m, "Jogging")),
        "a member starts no jog"
    );

    member.send(&jog_stop()).unwrap();
    assert!(
        errors(&drain(&mut member, Duration::from_millis(200))).is_empty(),
        "JogStop is allowed for all"
    );

    owner.send(&hold("X", "Plus", "1")).unwrap();
    wait_for(&mut owner, |m| is_state(m, "Jogging"));
    owner.send(&jog_stop()).unwrap();
    wait_for(&mut owner, |m| is_state(m, "Idle"));
}

#[test]
fn a_second_app_sees_the_jog_as_jogging() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    let (_t, mut observer) = svc.pair(&mut setup, "Phone", Role::Member);

    owner.send(&hold("X", "Plus", "1")).unwrap();
    let until = Instant::now() + WAIT;
    loop {
        owner.send(&keep()).unwrap();
        if matches!(observer.recv_message(Instant::now() + Duration::from_millis(50)), Ok(m) if is_state(&m, "Jogging"))
        {
            break;
        }
        assert!(Instant::now() < until, "the second app never saw Jogging");
    }
    owner.send(&jog_stop()).unwrap();
}

#[test]
fn closing_the_connection_ends_the_hold() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    let (_t, mut observer) = svc.pair(&mut setup, "Phone", Role::Member);

    owner.send(&hold("X", "Plus", "1")).unwrap();
    let until = Instant::now() + WAIT;
    loop {
        owner.send(&keep()).unwrap();
        if matches!(observer.recv_message(Instant::now() + Duration::from_millis(50)), Ok(m) if is_state(&m, "Jogging"))
        {
            break;
        }
        assert!(Instant::now() < until, "no Jogging seen");
    }
    owner.close();
    wait_for(&mut observer, |m| !is_state(m, "Jogging"));
}

#[test]
fn a_pad_session_writes_one_jog_line_in_the_job_log() {
    let svc = service::service();
    let (_setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    owner.send(&hold("X", "Plus", "1")).unwrap();
    wait_for(&mut owner, |m| is_state(m, "Jogging"));
    for _ in 0..3 {
        owner.send(&keep()).unwrap();
        drain(&mut owner, Duration::from_millis(60));
    }
    owner.send(&jog_stop()).unwrap();
    wait_for(&mut owner, |m| is_state(m, "Idle"));
    let text = wait_file(&svc, "jobs.log", "Jog: ");
    let lines = text.lines().filter(|l| l.contains("Jog: ")).count();
    assert_eq!(lines, 1, "one line per pad session, not per step: {text:?}");
}

#[test]
fn a_member_home_waits_and_the_owners_home_confirms_it_and_logs_home() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    member.send(&Message::Home).unwrap();
    let answers = errors(&drain(&mut member, Duration::from_millis(300)));
    assert!(
        answers.is_empty(),
        "a member's Home is not refused any more: {answers:?}"
    );
    wait_for(&mut owner, |m| {
        serde_json::to_string(m).unwrap().contains("Phone")
    });

    owner.send(&Message::Home).unwrap();
    wait_for(&mut owner, |m| json_of(m)["homed"] == json!(true));
    let log = wait_file(&svc, "admin.log", "start-confirmed");
    assert!(
        log.lines()
            .any(|l| l.contains("start-confirmed") && l.contains("Home")),
        "admin.log names Home: {log:?}"
    );
}

#[test]
fn an_owners_stop_discards_a_waiting_member_home() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(50);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    member.send(&Message::Home).unwrap();
    wait_for(&mut owner, |m| {
        serde_json::to_string(m).unwrap().contains("Phone")
    });
    owner.send(&Message::Stop).unwrap();
    wait_for(&mut owner, |m| {
        !serde_json::to_string(m).unwrap().contains("Phone")
    });
    let seen = drain(&mut owner, Duration::from_millis(300));
    assert!(
        !seen.iter().any(|m| is_state(m, "Homing")),
        "the discarded Home never runs"
    );
}

#[test]
fn a_second_member_home_while_one_waits_is_wrong_state() {
    let svc = service::service();
    let (mut setup, _token) = svc.setup(50);
    let (_t1, mut first) = svc.pair(&mut setup, "Phone", Role::Member);
    let (_t2, mut second) = svc.pair(&mut setup, "Watch", Role::Member);

    first.send(&Message::Home).unwrap();
    wait_for(&mut first, |m| {
        serde_json::to_string(m).unwrap().contains("Phone")
    });
    second.send(&Message::Home).unwrap();
    let refused = errors(&drain(&mut second, Duration::from_millis(500)));
    assert!(
        refused.iter().any(|c| c.contains("WrongState")),
        "{refused:?}"
    );
}
