//! `specs/0118` slice 2a against the GRBL simulator (adrs J6, J9): the `$J=` bytes per sign, the
//! laser guarantee, the feed cap, the clamp when homed, hold-to-move with its keep-alive, the
//! release, the cancel repeated until Idle, the stop of G12 when it does not settle, and
//! `error:15`. Time is stepped (10 ms a step), never the wall clock.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{
    ErrorCode, Event, JogAxis, JogDirection, JogStep, Message, Peer, State,
};
use curvyo_connector_io::controller::Controller;

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X-5 Y-5 F1000 S500\nM5\n";

fn config() -> SimConfig {
    SimConfig {
        mpos_reports: true,
        ..SimConfig::default()
    }
}

/// Homing is on: the machine wakes locked and a Home sets the origin.
fn homed_config() -> SimConfig {
    SimConfig {
        homing_lock: true,
        homing_steps: 20,
        ..config()
    }
}

fn press(axis: JogAxis, direction: JogDirection, step: JogStep, hold: bool) -> Message {
    Message::Jog {
        axis,
        direction,
        step,
        hold,
    }
}

fn x_plus(step: JogStep) -> Message {
    press(JogAxis::X, JogDirection::Plus, step, false)
}

/// One step of time for both sides and the machine's clock, as the hub's tick does.
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

fn say(bench: &mut Bench, message: Message) -> Vec<Message> {
    bench.apply(Event::Clock(bench.at));
    let out = bench.machine.handle(Event::Message(message, Peer::host()));
    let errors = out
        .messages
        .iter()
        .filter(|m| matches!(m, Message::Error { .. }))
        .cloned()
        .collect();
    for command in out.commands {
        let events = bench
            .ctrl
            .command(&command, bench.machine.held_job(), bench.at);
        for event in events {
            bench.apply(event);
        }
    }
    errors
}

fn homed(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    assert_eq!(bench.state(), State::Alarm);
    bench.app(Message::Home);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    assert!(matches!(
        bench.status(),
        Message::Status { homed: true, .. }
    ));
    // The first report after homing tells the connector where the head stands.
    bench.run_for(Duration::from_millis(300));
    bench
}

fn jog_lines(bench: &Bench, from: usize) -> Vec<String> {
    bench.sim.lines[from..]
        .iter()
        .filter(|l| l.starts_with("$J="))
        .cloned()
        .collect()
}

fn position(bench: &Bench) -> [f64; 2] {
    let Message::Status {
        machine_position: Some(m),
        ..
    } = bench.status()
    else {
        panic!("no machine position: {:?}", bench.status());
    };
    [m[0].0, m[1].0]
}

#[test]
fn j6_a_step_writes_m5_m9_then_one_jog_line_per_sign_and_ends_idle() {
    let cases = [
        (JogAxis::X, JogDirection::Plus, "$J=G91 G21 X1.000 F1000"),
        (JogAxis::X, JogDirection::Minus, "$J=G91 G21 X-1.000 F1000"),
        (JogAxis::Y, JogDirection::Plus, "$J=G91 G21 Y1.000 F1000"),
        (JogAxis::Y, JogDirection::Minus, "$J=G91 G21 Y-1.000 F1000"),
    ];
    for (axis, direction, line) in cases {
        let mut bench = Bench::connected(config());
        let mark = bench.sim.lines.len();
        bench.app(press(axis, direction, JogStep::One, false));
        assert_eq!(bench.state(), State::Jogging);
        assert!(until(&mut bench, |b| b.state() == State::Idle, 300));
        assert_eq!(bench.sim.lines[mark..], ["M5", "M9", line], "{line}");
        assert_eq!(bench.sim.faults, Vec::<String>::new());
        assert_eq!(bench.detail(), None);
        assert!(bench.closed.is_none());
    }
}

#[test]
fn j6_the_steps_and_their_feeds_are_the_constants_of_the_criterion() {
    for (step, line) in [
        (JogStep::Tenth, "$J=G91 G21 X0.100 F300"),
        (JogStep::One, "$J=G91 G21 X1.000 F1000"),
        (JogStep::Ten, "$J=G91 G21 X10.000 F3000"),
        (JogStep::Fifty, "$J=G91 G21 X50.000 F3000"),
    ] {
        let mut bench = Bench::connected(config());
        bench.app(x_plus(step));
        assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
        assert_eq!(jog_lines(&bench, 0), [line]);
    }
}

#[test]
fn j6_the_feed_is_capped_at_the_controllers_maximum_rate() {
    // The settings are read at identify, so `$110` is set before the controller connects.
    let mut bench = Bench::new(config());
    bench.sim.set_setting(110, "600.000");
    assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
    bench.app(x_plus(JogStep::Ten));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
    assert_eq!(jog_lines(&bench, 0), ["$J=G91 G21 X10.000 F600"]);
}

#[test]
fn c9_no_motion_line_carries_m3_m4_or_a_power_and_m5_leads_after_a_job() {
    let mut bench = Bench::connected(config());
    bench.send(JOB);
    let id = bench.machine.held_job().unwrap().job_id;
    bench.app(Message::Start { job_id: id });
    assert!(until(
        &mut bench,
        |b| matches!(b.state(), State::Done { .. }),
        2_000
    ));
    let mark = bench.sim.lines.len();
    bench.app(x_plus(JogStep::One));
    assert!(until(&mut bench, |b| b.state() != State::Jogging, 500));
    assert!(
        matches!(bench.state(), State::Done { .. }),
        "a done job stays done after a jog"
    );
    let lines = &bench.sim.lines[mark..];
    // The job ran a spindle word, so the guarantee writes M5 and M9 first.
    assert_eq!(lines[..2], ["M5", "M9"]);
    for line in lines {
        let upper = line.to_ascii_uppercase();
        assert!(!upper.contains("M3") && !upper.contains("M4"), "{line}");
        assert!(!upper.contains('S') || upper.contains("S0"), "{line}");
    }
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert!(!bench.sim.laser_lit());
    // A second jog repeats nothing: M5 was the last spindle word.
    let mark = bench.sim.lines.len();
    bench.app(x_plus(JogStep::One));
    assert!(until(&mut bench, |b| b.state() != State::Jogging, 500));
    assert_eq!(bench.sim.lines[mark..].len(), 1);
}

#[test]
fn c17_homed_the_jog_is_clamped_to_the_travel_and_at_the_edge_writes_nothing() {
    let mut config = homed_config();
    config.travel = [30.0, 20.0];
    config.soft_limits = true;
    let mut bench = homed(config);
    assert_eq!(position(&bench), [0.0, 0.0]);
    let mark = bench.sim.lines.len();
    // At the origin the box ends at 0: a jog towards + writes nothing and ends.
    bench.app(x_plus(JogStep::Ten));
    assert_eq!(bench.state(), State::Idle, "nothing to wait for");
    assert_eq!(jog_lines(&bench, mark), Vec::<String>::new());
    // Towards - it goes the whole 10, then 10 again, and the third is cut at the edge.
    for expected in ["X-10.000", "X-10.000", "X-10.000"] {
        bench.app(press(JogAxis::X, JogDirection::Minus, JogStep::Ten, false));
        assert!(until(&mut bench, |b| b.state() == State::Idle, 1_000));
        bench.run_for(Duration::from_millis(300));
        let last = jog_lines(&bench, mark).pop().unwrap();
        assert!(last.contains(expected), "{last}");
    }
    assert!((position(&bench)[0] + 30.0).abs() < 0.01);
    let mark = bench.sim.lines.len();
    bench.app(press(JogAxis::X, JogDirection::Minus, JogStep::One, false));
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(jog_lines(&bench, mark), Vec::<String>::new());
    // A jog past the edge from nearby is shortened: 50 mm from -30 towards + stops at 0.
    bench.app(press(JogAxis::X, JogDirection::Plus, JogStep::Fifty, false));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 2_000));
    assert!(jog_lines(&bench, mark).pop().unwrap().contains("X30.000"));
    assert_eq!(
        bench.detail(),
        None,
        "soft limits never fired: error 15 stayed unused"
    );
}

#[test]
fn c17_not_homed_the_jog_is_not_clamped_and_error_15_is_named() {
    let mut config = config();
    config.travel = [30.0, 20.0];
    config.soft_limits = true;
    let mut bench = Bench::connected(config);
    assert!(matches!(
        bench.status(),
        Message::Status { homed: false, .. }
    ));
    let mark = bench.sim.lines.len();
    bench.app(x_plus(JogStep::Ten));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 600));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Jog beyond the travel (error 15)")
    );
    assert_eq!(
        jog_lines(&bench, mark),
        ["$J=G91 G21 X10.000 F3000"],
        "GRBL's own limit decided, the connector wrote the whole step"
    );
    assert_eq!(bench.state(), State::Idle);
    assert!(bench.closed.is_none());
    assert_eq!(bench.sim.faults, Vec::<String>::new());
}

#[test]
fn c18_a_hold_moves_in_short_increments_on_keep_alives_and_the_release_stops_it() {
    let mut bench = Bench::connected(config());
    let mark = bench.sim.log.len();
    say(
        &mut bench,
        press(JogAxis::X, JogDirection::Minus, JogStep::Ten, true),
    );
    assert_eq!(bench.state(), State::Jogging);
    // 1 s of holding, a keep-alive every 100 ms.
    for _ in 0..10 {
        run(&mut bench, Duration::from_millis(100));
        say(&mut bench, Message::JogKeep);
    }
    assert_eq!(bench.state(), State::Jogging);
    let lines = jog_lines(&bench, 0);
    assert!(lines.len() >= 8, "{lines:?}");
    assert!(
        lines.iter().all(|l| l == "$J=G91 G21 X-5.000 F3000"),
        "increments of a tenth of a second at the step's feed: {lines:?}"
    );
    // Never more than a quarter of a second of motion in front of the head.
    let moved = -bench.sim.rx_pos_for_tests()[0];
    assert!(moved > 30.0, "{moved}");
    // The release.
    let before = bench.sim.log.len();
    say(&mut bench, Message::JogStop);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 100));
    assert!(bench.sim.log[before..].contains(&0x85));
    assert!(
        !bench.sim.log[mark..].contains(&b'!'),
        "a release is no hold"
    );
    assert!(!bench.sim.log[mark..].contains(&0x18), "and no reset");
    let at_release = bench.sim.rx_pos_for_tests();
    run(&mut bench, Duration::from_secs(1));
    assert_eq!(bench.sim.rx_pos_for_tests(), at_release, "the head stands");
    assert_eq!(bench.alarm(), None);
    assert_eq!(bench.sim.faults, Vec::<String>::new());
}

#[test]
fn c18_without_a_keep_alive_the_hold_is_cancelled_within_350_ms() {
    let mut bench = Bench::connected(config());
    say(
        &mut bench,
        press(JogAxis::Y, JogDirection::Minus, JogStep::Ten, true),
    );
    let started = bench.at;
    assert!(until(&mut bench, |b| b.sim.log.contains(&0x85), 100));
    let waited = bench.at.saturating_sub(started);
    assert!(waited >= Duration::from_millis(300), "{waited:?}");
    assert!(waited <= Duration::from_millis(350), "{waited:?}");
    assert!(until(&mut bench, |b| b.state() == State::Idle, 100));
    let there = bench.sim.rx_pos_for_tests();
    run(&mut bench, Duration::from_secs(1));
    assert_eq!(bench.sim.rx_pos_for_tests(), there);
}

#[test]
fn j6_a_cancel_that_meets_a_late_jog_line_is_repeated_until_the_head_stands() {
    let mut bench = Bench::connected(config());
    // The line and the cancel leave in one turn: GRBL ignores `0x85` outside a jog, then runs
    // the line.
    bench.app(x_plus(JogStep::Fifty));
    bench.app(Message::Stop);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 400));
    let cancels = bench
        .sim
        .log
        .iter()
        .copied()
        .map(usize::from)
        .filter(|b| *b == 0x85)
        .count();
    assert!(cancels >= 2, "repeated at the reports: {cancels}");
    let there = bench.sim.rx_pos_for_tests();
    assert!(there[0] < 49.0, "the 50 mm line was cut short: {there:?}");
    run(&mut bench, Duration::from_secs(2));
    assert_eq!(bench.sim.rx_pos_for_tests(), there);
    assert_eq!(bench.alarm(), None);
}

#[test]
fn c10_stop_during_a_jog_is_the_cancel_not_the_stop_and_keeps_the_ready_job() {
    let mut bench = Bench::connected(config());
    let id = bench.send(JOB);
    let mark = bench.sim.log.len();
    bench.app(x_plus(JogStep::Fifty));
    run(&mut bench, Duration::from_millis(100));
    bench.app(Message::Stop);
    assert_eq!(bench.detail().as_deref(), Some("Stopping"));
    assert!(until(&mut bench, |b| b.state() == State::Ready, 300));
    assert_eq!(bench.detail(), None);
    assert_eq!(bench.machine.held_job().map(|j| j.job_id), Some(id));
    let sent = &bench.sim.log[mark..];
    assert!(sent.contains(&0x85));
    assert!(!sent.contains(&b'!') && !sent.contains(&0x18));
    assert_eq!(bench.alarm(), None);
}

#[test]
fn c10_a_cancel_the_controller_ignores_ends_in_the_stop_of_g12_after_two_seconds() {
    // A slow axis (60 mm/min): the 50 mm step is still under way after the 2 s of the limit.
    let mut bench = Bench::new(config());
    bench.sim.set_setting(110, "60.000");
    assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
    bench.sim.no_jog_cancel = true;
    bench.app(x_plus(JogStep::Fifty));
    run(&mut bench, Duration::from_millis(100));
    bench.app(Message::Stop);
    let (stopped_at, mark) = (bench.at, bench.sim.log.len());
    run(&mut bench, Duration::from_millis(1_900));
    assert!(
        !bench.sim.log[mark..].contains(&0x18),
        "not before the limit"
    );
    assert!(until(
        &mut bench,
        |b| b.sim.log[mark..].contains(&0x18),
        200
    ));
    assert!(
        bench.sim.log[mark..].contains(&b'!'),
        "hold first, then the reset"
    );
    assert!(bench.at.saturating_sub(stopped_at) >= Duration::from_secs(2));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    assert_eq!(bench.alarm(), None, "a reset in a jog raises no alarm");
    // The laser guarantee starts afresh after the reset.
    let mark = bench.sim.lines.len();
    bench.app(x_plus(JogStep::One));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    assert_eq!(bench.sim.lines[mark..].len(), 3, "M5 M9 and the line");
}

#[test]
fn c8_a_jog_is_refused_while_a_job_runs_or_it_is_locked_and_nothing_is_written() {
    let mut bench = Bench::connected(config());
    bench.start(&JOB.repeat(40));
    run(&mut bench, Duration::from_millis(200));
    assert_eq!(bench.state(), State::Running);
    let mark = bench.sim.log.len();
    let refused = say(&mut bench, x_plus(JogStep::One));
    assert!(matches!(
        refused.as_slice(),
        [Message::Error { code: ErrorCode::WrongState, message }] if message == "A job is running."
    ));
    assert_eq!(
        bench.sim.log[mark..]
            .iter()
            .copied()
            .filter(|b| *b == b'$')
            .fold(0, |n, _| n + 1),
        0
    );
    let mut locked = Bench::connected(homed_config());
    assert_eq!(locked.state(), State::Alarm);
    let mark = locked.sim.log.len();
    let refused = say(&mut locked, x_plus(JogStep::One));
    assert!(matches!(
        refused.as_slice(),
        [Message::Error { code: ErrorCode::WrongState, message }] if message == "Alarm."
    ));
    assert_eq!(locked.sim.log.len(), mark);
}

#[test]
fn j3_the_jog_word_is_no_foreign_controller_state() {
    let mut bench = Bench::connected(config());
    bench.app(x_plus(JogStep::Fifty));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 600));
    assert!(
        !bench.notices.iter().any(|n| n.contains("Controller state")),
        "{:?}",
        bench.notices
    );
}

#[test]
fn j5_homed_since_connect_and_the_positions_reach_the_status() {
    let mut config = homed_config();
    config.wco = [5.0, 6.0, 0.0];
    let mut bench = homed(config);
    let Message::Status {
        homed,
        machine_position,
        work_offset,
        ..
    } = bench.status()
    else {
        panic!()
    };
    assert!(homed);
    assert_eq!(machine_position.map(|p| [p[0].0, p[1].0]), Some([0.0, 0.0]));
    assert_eq!(work_offset.map(|p| [p[0].0, p[1].0]), Some([5.0, 6.0]));
    // A jog and its cancel keep it; a reset nobody asked for clears it.
    bench.app(x_plus(JogStep::One));
    assert!(until(&mut bench, |b| b.state() == State::Idle, 300));
    assert!(matches!(
        bench.status(),
        Message::Status { homed: true, .. }
    ));
    bench.sim.reset();
    run(&mut bench, Duration::from_millis(300));
    assert!(matches!(
        bench.status(),
        Message::Status { homed: false, .. }
    ));
}

#[test]
fn j5_a_jobs_own_dollar_h_sets_homed() {
    let mut bench = Bench::connected(homed_config());
    bench.app(Message::Unlock);
    assert!(until(&mut bench, |b| b.state() == State::Idle, 500));
    assert!(matches!(
        bench.status(),
        Message::Status { homed: false, .. }
    ));
    bench.start("$H\nG21\nG90\nG0 X-1 Y-1 F1000\n");
    assert!(until(
        &mut bench,
        |b| matches!(b.state(), State::Done { .. }),
        2_000
    ));
    assert!(matches!(
        bench.status(),
        Message::Status { homed: true, .. }
    ));
}
