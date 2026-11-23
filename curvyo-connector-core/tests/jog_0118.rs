//! `specs/0118` slice 2a, criteria 8, 10, 12, 13, 14, 18, 19, 20 (adrs J3 to J5): the jog rules of
//! the job machine per state in both machines, the dead-man by an injected clock, "homed since
//! connect" and the positions in `Status`. No wall time anywhere.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use std::time::Duration;

use common::{JOB_TEXT, chunk, job_id, submit};
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, Feed, JOG_DEAD_MAN, JobKind, JobResult, JogAxis,
    JogDirection, JogStep, Machine, Message, Millimetres, Options, Output, Peer, Role, StartRule,
    State,
};

const OWNER_RULE: Options = Options {
    start_rule: StartRule::OwnerConfirms,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn owner() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn member() -> Peer {
    Peer::new("Phone", Role::Member)
}

fn jog(step: JogStep, hold: bool) -> Message {
    Message::Jog {
        axis: JogAxis::X,
        direction: JogDirection::Plus,
        step,
        hold,
    }
}

fn plain() -> Message {
    jog(JogStep::One, false)
}

fn machine(confirming: bool, options: Options) -> Machine {
    let mut m = if confirming {
        Machine::confirming(options)
    } else {
        Machine::new(options)
    };
    m.handle(Event::ControllerConnected);
    m
}

fn say(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, owner()))
}

fn ready(m: &mut Machine) {
    say(m, submit(JOB_TEXT, JobKind::Job));
    say(m, chunk(0, JOB_TEXT));
    assert_eq!(m.state(), State::Ready);
}

fn error_of(out: &Output) -> Option<(ErrorCode, String)> {
    out.messages.iter().find_map(|m| match m {
        Message::Error { code, message } => Some((*code, message.clone())),
        _ => None,
    })
}

fn detail_of(m: &Machine) -> Option<String> {
    match m.status() {
        Message::Status { detail, .. } => detail,
        _ => None,
    }
}

#[test]
fn j3_a_jog_is_accepted_in_idle_ready_and_done_and_returns_to_where_it_came_from() {
    for confirming in [false, true] {
        for from in ["idle", "ready", "done"] {
            let mut m = machine(confirming, Options::default());
            match from {
                "ready" => ready(&mut m),
                "done" => {
                    ready(&mut m);
                    say(&mut m, Message::Start { job_id: job_id() });
                    m.handle(Event::JobFinished { elapsed_ms: 5 });
                }
                _ => {}
            }
            let before = m.state();
            let out = say(&mut m, plain());
            assert_eq!(
                out.commands,
                vec![ControllerCommand::Jog {
                    axis: JogAxis::X,
                    direction: JogDirection::Plus,
                    distance: Millimetres(1.0),
                    feed: Feed(1_000.0),
                    homed: false,
                }],
                "{from}"
            );
            assert_eq!(m.state(), State::Jogging);
            let held = matches!(from, "ready" | "done");
            assert_eq!(
                matches!(m.status(), Message::Status { job: Some(_), .. }),
                held,
                "a held job is carried along while jogging ({from})"
            );
            m.handle(Event::JogEnded);
            assert_eq!(m.state(), before, "{from} confirming {confirming}");
            assert_eq!(
                m.held_job().is_some(),
                held,
                "a kept job is never discarded"
            );
        }
    }
}

#[test]
fn j3_every_other_state_refuses_with_its_cause_and_writes_nothing() {
    let cases: [(&str, &str); 7] = [
        ("receiving", "A job is arriving."),
        ("running", "A job is running."),
        ("paused", "A job is running."),
        ("homing", "Homing."),
        ("alarm", "Alarm."),
        ("nocontroller", "No controller."),
        ("door", "Controller state: Door."),
    ];
    for confirming in [false, true] {
        for (name, words) in cases {
            let mut m = machine(confirming, Options::default());
            match name {
                "receiving" => {
                    say(&mut m, submit(JOB_TEXT, JobKind::Job));
                }
                "running" | "paused" => {
                    ready(&mut m);
                    say(&mut m, Message::Start { job_id: job_id() });
                    if name == "paused" {
                        say(&mut m, Message::Pause);
                        m.handle(Event::Held);
                    }
                }
                "homing" => {
                    say(&mut m, Message::Home);
                }
                "alarm" => {
                    m.handle(Event::Alarm { text: "x".into() });
                }
                "nocontroller" => {
                    m.handle(Event::ControllerLost);
                }
                _ => {
                    m.handle(Event::ControllerWord(Some("Door".into())));
                }
            }
            let state = m.state();
            let out = say(&mut m, plain());
            assert_eq!(out.commands, Vec::new(), "{name}");
            assert_eq!(
                error_of(&out),
                Some((ErrorCode::WrongState, words.to_owned())),
                "{name}"
            );
            assert_eq!(m.state(), state, "{name}");
        }
    }
}

#[test]
fn j3_a_waiting_start_or_home_refuses_a_jog() {
    let mut m = machine(
        true,
        Options {
            start_rule: StartRule::LocalPress,
        },
    );
    ready(&mut m);
    say(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(m.state(), State::AwaitingLocalStart);
    let out = say(&mut m, plain());
    assert_eq!(
        error_of(&out),
        Some((
            ErrorCode::WrongState,
            "A start waits at the machine.".into()
        ))
    );
    let mut m = machine(true, OWNER_RULE);
    m.handle(Event::Message(Message::Home, member()));
    assert_eq!(m.state(), State::AwaitingLocalStart);
    let out = say(&mut m, plain());
    assert_eq!(error_of(&out).map(|e| e.0), Some(ErrorCode::WrongState));
    assert_eq!(out.commands, Vec::new());
}

#[test]
fn j3_require_local_start_does_not_hold_a_jog() {
    let mut m = machine(
        false,
        Options {
            start_rule: StartRule::LocalPress,
        },
    );
    let out = say(&mut m, plain());
    assert_eq!(m.state(), State::Jogging);
    assert_eq!(out.commands.len(), 1);
}

#[test]
fn j3_one_more_step_is_taken_once_and_later_ones_are_dropped() {
    let mut m = machine(true, Options::default());
    assert_eq!(say(&mut m, plain()).commands.len(), 1);
    assert_eq!(say(&mut m, plain()).commands.len(), 1, "queued");
    let third = say(&mut m, plain());
    assert_eq!(third.commands, Vec::new(), "dropped");
    assert_eq!(error_of(&third), None, "dropped without a word");
    assert_eq!(m.state(), State::Jogging);
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Idle);
    // The next press starts afresh.
    assert_eq!(say(&mut m, plain()).commands.len(), 1);
}

#[test]
fn j3_while_jogging_home_start_and_submit_are_refused_and_pause_has_no_job() {
    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    let out = say(&mut m, Message::Home);
    assert_eq!(
        error_of(&out),
        Some((ErrorCode::WrongState, "Jogging.".into()))
    );
    let out = say(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(error_of(&out).map(|e| e.0), Some(ErrorCode::WrongState));
    assert_eq!(out.commands, Vec::new());
    let out = say(&mut m, Message::Pause);
    assert_eq!(
        error_of(&out),
        Some((ErrorCode::WrongState, "No job to pause".into()))
    );
    let out = say(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert!(
        out.messages
            .iter()
            .any(|m| matches!(m, Message::JobRejected { .. }))
    );
    assert_eq!(m.state(), State::Jogging);
}

#[test]
fn j3_stop_is_a_jog_cancel_not_a_stop_and_keeps_the_ready_job() {
    // Without confirmation the machine is back at once.
    let mut m = machine(false, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    let out = say(&mut m, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(m.state(), State::Ready);
    assert!(m.held_job().is_some());
    // With confirmation it reads "Stopping" until the controller stands.
    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    let out = say(&mut m, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(m.state(), State::Jogging);
    assert_eq!(detail_of(&m).as_deref(), Some("Stopping"));
    // A second Stop writes nothing more.
    assert_eq!(say(&mut m, Message::Stop).commands, Vec::new());
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Ready);
    assert_eq!(detail_of(&m), None);
}

#[test]
fn j3_a_confirmed_stop_that_needed_the_reset_ends_the_jog_too() {
    let mut m = machine(true, Options::default());
    say(&mut m, plain());
    say(&mut m, Message::Stop);
    m.handle(Event::Stopped);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn j3_jog_stop_is_not_stop_and_outside_a_jog_it_does_nothing() {
    for confirming in [false, true] {
        let mut m = machine(confirming, Options::default());
        ready(&mut m);
        let out = say(&mut m, Message::JogStop);
        assert_eq!(out.commands, Vec::new());
        assert_eq!(
            m.state(),
            State::Ready,
            "a release after the jog ended keeps the job"
        );
        assert!(m.held_job().is_some());
        // The same in Done, Running and Homing.
        say(&mut m, Message::Start { job_id: job_id() });
        assert_eq!(say(&mut m, Message::JogStop).commands, Vec::new());
        assert_eq!(m.state(), State::Running);
    }
    let mut m = machine(false, Options::default());
    say(&mut m, jog(JogStep::One, true));
    let out = say(&mut m, Message::JogStop);
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(m.state(), State::Idle);
    assert_eq!(say(&mut m, Message::JogStop).commands, Vec::new());
}

#[test]
fn j3_a_failed_jog_names_its_cause_and_returns() {
    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    m.handle(Event::JogFailed {
        detail: "Jog beyond the travel (error 15)".into(),
    });
    assert_eq!(m.state(), State::Ready);
    assert_eq!(
        detail_of(&m).as_deref(),
        Some("Jog beyond the travel (error 15)")
    );
    assert!(m.held_job().is_some());
}

#[test]
fn j3_an_alarm_a_loss_or_a_reset_ends_the_jog_and_an_alarm_drops_a_ready_job() {
    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    m.handle(Event::Alarm {
        text: "Hard limit".into(),
    });
    assert_eq!(m.state(), State::Alarm);
    assert!(m.held_job().is_none());

    let mut m = machine(true, Options::default());
    say(&mut m, plain());
    m.handle(Event::ControllerLost);
    assert_eq!(m.state(), State::NoController);
    assert_eq!(detail_of(&m).as_deref(), Some("Lost the controller"));

    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, plain());
    m.handle(Event::ControllerReset);
    assert_eq!(m.state(), State::Ready);
}

#[test]
fn j4_a_hold_lives_300_ms_on_the_injected_clock_and_a_keep_extends_it() {
    assert_eq!(JOG_DEAD_MAN, ms(300));
    let mut m = machine(false, Options::default());
    m.handle(Event::Clock(ms(1_000)));
    let out = say(&mut m, jog(JogStep::Ten, true));
    assert_eq!(
        out.commands,
        vec![ControllerCommand::JogHold {
            axis: JogAxis::X,
            direction: JogDirection::Plus,
            feed: Feed(3_000.0),
            homed: false,
        }]
    );
    assert_eq!(m.state(), State::Jogging);
    // 299 ms after the start the hold lives.
    assert_eq!(m.handle(Event::Clock(ms(1_299))).commands, Vec::new());
    assert_eq!(m.state(), State::Jogging);
    // A keep-alive at 1,299 ms moves the deadline to 1,599 ms.
    m.handle(Event::Clock(ms(1_299)));
    say(&mut m, Message::JogKeep);
    assert_eq!(m.handle(Event::Clock(ms(1_598))).commands, Vec::new());
    assert_eq!(m.state(), State::Jogging);
    // At exactly 300 ms without one it is cancelled.
    let out = m.handle(Event::Clock(ms(1_599)));
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(
        m.state(),
        State::Idle,
        "a machine without confirmation is back at once"
    );
    // Late keep-alives and later clocks do nothing.
    assert_eq!(say(&mut m, Message::JogKeep).commands, Vec::new());
    assert_eq!(m.handle(Event::Clock(ms(9_000))).commands, Vec::new());
}

#[test]
fn j4_on_a_confirming_machine_the_cancel_is_given_once_and_the_end_is_the_controllers() {
    let mut m = machine(true, Options::default());
    ready(&mut m);
    say(&mut m, jog(JogStep::One, true));
    let out = m.handle(Event::Clock(ms(300)));
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(m.state(), State::Jogging, "until the controller stands");
    assert_eq!(detail_of(&m), None, "a release is no Stop");
    assert_eq!(m.handle(Event::Clock(ms(900))).commands, Vec::new());
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Ready);
}

#[test]
fn j4_a_step_has_no_deadline_and_a_second_press_cannot_become_a_hold() {
    let mut m = machine(true, Options::default());
    say(&mut m, plain());
    assert_eq!(m.handle(Event::Clock(ms(60_000))).commands, Vec::new());
    assert_eq!(m.state(), State::Jogging);
    assert_eq!(say(&mut m, jog(JogStep::One, true)).commands, Vec::new());
}

#[test]
fn j5_homed_since_connect_is_set_by_homed_and_cleared_by_what_loses_the_position() {
    fn homed(m: &Machine) -> bool {
        matches!(m.status(), Message::Status { homed: true, .. })
    }
    let mut m = machine(true, Options::default());
    assert!(!homed(&m));
    // The `ok` of a job's own `$H`, in a state of its own.
    ready(&mut m);
    say(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::Homed);
    assert!(homed(&m));
    assert_eq!(m.state(), State::Running);
    m.handle(Event::JobFinished { elapsed_ms: 1 });
    assert!(homed(&m), "a job's end keeps it");
    // The Home command.
    for lost in 0..5 {
        let mut m = machine(true, Options::default());
        say(&mut m, Message::Home);
        m.handle(Event::Homed);
        assert!(homed(&m));
        // A jog and its cancel keep it, a stop's own reset too.
        say(&mut m, plain());
        say(&mut m, Message::Stop);
        m.handle(Event::JogEnded);
        assert!(homed(&m));
        say(&mut m, Message::Start { job_id: job_id() });
        match lost {
            0 => drop(m.handle(Event::Alarm { text: "x".into() })),
            1 => drop(m.handle(Event::ControllerLost)),
            2 => drop(m.handle(Event::StopUnconfirmed)),
            3 => drop(m.handle(Event::ControllerReset)),
            _ => {
                m.handle(Event::ControllerLost);
                m.handle(Event::ControllerConnected);
            }
        }
        assert!(!homed(&m), "case {lost}");
    }
    let mut m = machine(true, Options::default());
    say(&mut m, Message::Home);
    m.handle(Event::Homed);
    let out = say(&mut m, plain());
    assert!(matches!(
        out.commands.first(),
        Some(ControllerCommand::Jog { homed: true, .. })
    ));
}

#[test]
fn c14_status_carries_the_machine_position_and_the_work_offset() {
    let mut m = machine(true, Options::default());
    let moved = [-112.5, -130.0, 0.0].map(Millimetres);
    let shifted = [100.0, 100.0, 0.0].map(Millimetres);
    let out = m.handle(Event::MachinePosition {
        machine: Some(moved),
        offset: None,
    });
    assert!(matches!(
        out.messages.last(),
        Some(Message::Status { machine_position: Some(p), work_offset: None, .. }) if *p == moved
    ));
    m.handle(Event::MachinePosition {
        machine: Some(moved),
        offset: Some(shifted),
    });
    // A report without the offset keeps the last one; an unchanged report pushes nothing.
    let out = m.handle(Event::MachinePosition {
        machine: Some(moved),
        offset: None,
    });
    assert_eq!(out.messages, Vec::new());
    assert!(matches!(
        m.status(),
        Message::Status { machine_position: Some(p), work_offset: Some(o), .. } if p == moved && o == shifted
    ));
    // A lost controller forgets both.
    m.handle(Event::ControllerLost);
    assert!(matches!(
        m.status(),
        Message::Status {
            machine_position: None,
            work_offset: None,
            ..
        }
    ));
}

#[test]
fn j3_a_done_job_stays_done_after_a_jog() {
    let mut m = machine(false, Options::default());
    ready(&mut m);
    say(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::JobFinished { elapsed_ms: 5 });
    let done = State::Done {
        result: JobResult::Completed,
    };
    assert_eq!(m.state(), done);
    say(&mut m, plain());
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), done);
}
