//! `specs/0118` slice 2b, core: the frame (criteria 22 to 28; adrs F2 to F5): its wire, the
//! rules per state in both machines, a member's wait, and the words a frame may hold.
#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    clippy::assert_is_empty
)]

mod common;

use common::{JOB_TEXT, chunk, fixture, job_id, sha256_hex, submit};
use curvyo_connector_core::{
    Confirms, ControllerCommand, ErrorCode, Event, JobKind, JobResult, Machine, Message, Options,
    Output, Peer, Progress, RejectReason, Role, StartRule, State, check_frame, decode, encode,
};
use uuid::Uuid;

const FRAME: &str = "; Curvyo frame: a\nG21\nG90\nG17\nM5\nM9\nG1 X10.000 Y10.000 F3000 S0\nG1 X60.000 Y10.000 F3000 S0\nG1 X10.000 Y10.000 F3000 S0\n";
const OWNER_RULE: Options = Options {
    start_rule: StartRule::OwnerConfirms,
};
const LOCAL_RULE: Options = Options {
    start_rule: StartRule::LocalPress,
};

fn frame_id() -> Uuid {
    "11111111-2222-4333-8444-555555555555".parse().unwrap()
}

fn owner() -> Peer {
    Peer::new("Marc", Role::Owner)
}

fn member() -> Peer {
    Peer::new("Phone", Role::Member)
}

fn from(m: &mut Machine, peer: &Peer, message: Message) -> Output {
    m.handle(Event::Message(message, peer.clone()))
}

fn app(m: &mut Machine, message: Message) -> Output {
    from(m, &Peer::host(), message)
}

fn frame_submit(text: &str) -> Message {
    Message::SubmitJob {
        job_id: frame_id(),
        name: "frame".to_owned(),
        kind: JobKind::Frame,
        bytes: text.len() as u64,
        sha256: sha256_hex(text),
    }
}

/// A frame sent by `peer`: submit and its one chunk; the output of the chunk.
fn frame_from(m: &mut Machine, peer: &Peer, text: &str) -> Output {
    let mut all = from(m, peer, frame_submit(text));
    let last = from(
        m,
        peer,
        Message::JobChunk {
            job_id: frame_id(),
            index: 0,
            text: text.to_owned(),
        },
    );
    all.messages.extend(last.messages);
    all.commands.extend(last.commands);
    all.confirmed = last.confirmed;
    all
}

fn both(options: Options) -> [Machine; 2] {
    [Machine::new(options), Machine::confirming(options)]
}

fn idle(mut m: Machine) -> Machine {
    m.handle(Event::ControllerConnected);
    m
}

fn ready(mut m: Machine) -> Machine {
    m = idle(m);
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    app(&mut m, chunk(0, JOB_TEXT));
    assert_eq!(m.state(), State::Ready);
    m
}

fn done(mut m: Machine) -> Machine {
    m = ready(m);
    app(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::JobFinished { elapsed_ms: 5 });
    assert!(matches!(m.state(), State::Done { .. }));
    m
}

fn rejected(out: &Output) -> Option<RejectReason> {
    out.messages.iter().find_map(|m| match m {
        Message::JobRejected { reason, .. } => Some(*reason),
        _ => None,
    })
}

fn accepted(out: &Output) -> bool {
    out.messages
        .iter()
        .any(|m| matches!(m, Message::JobAccepted { position: None, .. }))
}

fn wrong_state(out: &Output) -> Option<String> {
    out.messages.iter().find_map(|m| match m {
        Message::Error {
            code: ErrorCode::WrongState,
            message,
        } => Some(message.clone()),
        _ => None,
    })
}

fn ready_progress() -> Option<Progress> {
    Some(Progress {
        job_id: job_id(),
        line: 0,
        lines: 6,
        elapsed_ms: 0,
        from_line: None,
        entering: false,
    })
}

fn status_fields(m: &Machine) -> (State, Option<Progress>, Option<Uuid>, Option<String>) {
    match m.status() {
        Message::Status {
            state,
            job,
            frame_id,
            requested_by,
            ..
        } => (state, job, frame_id, requested_by),
        _ => unreachable!(),
    }
}

// ---- the wire (F2) ---------------------------------------------------------------------------

#[test]
fn the_wire_additions_match_their_golden_files() {
    let cases = [
        ("submit_frame", frame_submit(FRAME)),
        (
            "reject_not_a_frame",
            Message::JobRejected {
                job_id: job_id(),
                reason: RejectReason::NotAFrame,
            },
        ),
    ];
    for (name, message) in cases {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
    let mut m = idle(Machine::new(Options::default()));
    app(&mut m, frame_submit(FRAME));
    app(
        &mut m,
        Message::JobChunk {
            job_id: frame_id(),
            index: 0,
            text: FRAME.to_owned(),
        },
    );
    let framing = m.status();
    assert_eq!(
        encode(&framing).unwrap(),
        fixture("messages/status_framing.json")
    );
    let mut owner_rule = idle(Machine::new(OWNER_RULE));
    frame_from(&mut owner_rule, &member(), FRAME);
    assert_eq!(
        encode(&owner_rule.status()).unwrap(),
        fixture("messages/status_awaiting_frame.json")
    );
    // Old statuses stay as they were: no `frame_id` key without a frame.
    assert!(
        !encode(&idle(Machine::new(Options::default())).status())
            .unwrap()
            .contains("frame_id")
    );
}

// ---- F4: accepted per state, a Ready job kept, never listed -----------------------------------

#[test]
fn a_frame_runs_from_idle_ready_and_done_and_returns_where_it_came_from() {
    for start in [idle, ready, done] {
        for m in both(Options::default()) {
            let mut m = start(m);
            let before = m.state();
            let list_before = m.job_list();
            let out = frame_from(&mut m, &Peer::host(), FRAME);
            assert!(accepted(&out), "{out:?}");
            assert_eq!(
                out.commands,
                [ControllerCommand::Frame {
                    job_id: frame_id(),
                    homed: false
                }]
            );
            assert_eq!(m.state(), State::Framing);
            assert_eq!(m.frame_job().unwrap().text.as_ref(), FRAME);
            assert_eq!(m.frame_job().unwrap().kind, JobKind::Frame);
            let (_, job, id, _) = status_fields(&m);
            assert_eq!(id, Some(frame_id()));
            assert_eq!(job.is_some(), before != State::Idle, "{before:?}");
            assert_eq!(m.job_list(), list_before, "a frame is never listed");
            m.handle(Event::Progress {
                line: 3,
                elapsed_ms: 10,
            });
            assert_eq!(
                status_fields(&m).1.map(|p| p.line),
                job.map(|p| p.line),
                "the frame's progress touches no job"
            );
            m.handle(Event::JobFinished { elapsed_ms: 20 });
            assert_eq!(m.state(), before, "no Done, no result");
            assert!(m.frame_job().is_none());
            assert_eq!(status_fields(&m).2, None);
            assert_eq!(m.job_list(), list_before);
        }
    }
}

#[test]
fn a_frame_in_small_chunks_assembles_to_the_same_text() {
    let mut m = ready(Machine::new(Options::default()));
    app(&mut m, frame_submit(FRAME));
    for (i, piece) in FRAME.as_bytes().chunks(10).enumerate() {
        let text = std::str::from_utf8(piece).unwrap().to_owned();
        let message = Message::JobChunk {
            job_id: frame_id(),
            index: i.try_into().unwrap(),
            text,
        };
        app(&mut m, message);
    }
    assert_eq!(m.state(), State::Framing);
    assert_eq!(m.frame_job().unwrap().text.as_ref(), FRAME);
}

#[test]
fn the_next_start_runs_the_held_job_not_the_frame() {
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::JobFinished { elapsed_ms: 20 });
    assert_eq!(status_fields(&m).1, ready_progress());
    let out = app(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(
        out.commands,
        [ControllerCommand::Start { job_id: job_id() }]
    );
    assert_eq!(m.held_job().unwrap().kind, JobKind::Job);
    assert_eq!(m.state(), State::Running);
}

#[test]
fn a_done_job_stays_done_and_its_result_is_not_written_again() {
    let mut m = done(Machine::new(Options::default()));
    let out = frame_from(&mut m, &Peer::host(), FRAME);
    assert!(out.list_log.is_empty());
    let out = m.handle(Event::JobFinished { elapsed_ms: 20 });
    assert_eq!(
        m.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    assert!(out.follow_up.is_none(), "no RunNext: a frame is no job");
}

#[test]
fn a_frame_is_busy_in_every_other_state() {
    // Receiving, Running, Paused, a waiting start, Homing, Jogging, Alarm, a controller word.
    for m in both(Options::default()) {
        let mut m = idle(m);
        app(&mut m, submit(JOB_TEXT, JobKind::Job));
        app(&mut m, chunk(0, &JOB_TEXT[..5]));
        assert_eq!(m.state(), State::Receiving);
        assert_eq!(
            rejected(&app(&mut m, frame_submit(FRAME))),
            Some(RejectReason::Busy)
        );
    }
    for m in both(Options::default()) {
        let mut m = ready(m);
        app(&mut m, Message::Start { job_id: job_id() });
        assert_eq!(
            rejected(&app(&mut m, frame_submit(FRAME))),
            Some(RejectReason::Busy)
        );
        app(&mut m, Message::Pause);
        m.handle(Event::Held);
        assert_eq!(
            rejected(&app(&mut m, frame_submit(FRAME))),
            Some(RejectReason::Busy)
        );
    }
    let mut m = ready(Machine::new(LOCAL_RULE));
    app(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(m.state(), State::AwaitingLocalStart);
    assert_eq!(
        rejected(&app(&mut m, frame_submit(FRAME))),
        Some(RejectReason::Busy)
    );
    let mut homing = idle(Machine::new(Options::default()));
    app(&mut homing, Message::Home);
    assert_eq!(
        rejected(&app(&mut homing, frame_submit(FRAME))),
        Some(RejectReason::Busy)
    );
    let mut m = idle(Machine::new(Options::default()));
    m.handle(Event::Alarm {
        text: "Hard limit".into(),
    });
    assert_eq!(
        rejected(&app(&mut m, frame_submit(FRAME))),
        Some(RejectReason::Busy)
    );
    let mut m = idle(Machine::new(Options::default()));
    m.handle(Event::ControllerWord(Some("Door".into())));
    assert_eq!(
        rejected(&app(&mut m, frame_submit(FRAME))),
        Some(RejectReason::Busy)
    );
    let mut m = Machine::new(Options::default());
    assert_eq!(
        rejected(&app(&mut m, frame_submit(FRAME))),
        Some(RejectReason::NoController)
    );
}

#[test]
fn a_frame_arriving_blocks_a_second_and_a_start_that_ran_meanwhile_refuses_it() {
    let mut m = ready(Machine::new(Options::default()));
    let two_parts = (&FRAME[..20], &FRAME[20..]);
    app(&mut m, frame_submit(FRAME));
    assert_eq!(
        m.state(),
        State::Ready,
        "the state does not change while it arrives"
    );
    assert_eq!(
        rejected(&app(&mut m, frame_submit(FRAME))),
        Some(RejectReason::Busy)
    );
    assert!(
        rejected(&app(&mut m, chunk(0, two_parts.0))).is_none(),
        "a chunk of the job id is for the job, not the frame"
    );
    let mut m = ready(Machine::new(Options::default()));
    app(&mut m, frame_submit(FRAME));
    app(&mut m, Message::Start { job_id: job_id() });
    let out = app(
        &mut m,
        Message::JobChunk {
            job_id: frame_id(),
            index: 0,
            text: FRAME.to_owned(),
        },
    );
    assert_eq!(
        rejected(&out),
        Some(RejectReason::Busy),
        "the state is checked again"
    );
    assert_eq!(m.state(), State::Running);
    assert!(m.frame_job().is_none());
}

#[test]
fn a_frame_refuses_its_checks_before_anything_moves() {
    for (text, reason) in [
        ("G21\nG90\nM5\nM3 S100\n", RejectReason::NotAFrame),
        ("G21\nG90\nG1 X1 Y1 F100 S0\n", RejectReason::NotAFrame),
        ("G21\nG90\nM5\nG1 X1 Y1 F100 S1\n", RejectReason::NotAFrame),
        ("; nothing\n", RejectReason::NotAFrame),
        (
            "G21\nG90\nM5\nG1 X1 Y1 F100 S0\u{e9}\n",
            RejectReason::NotAscii,
        ),
    ] {
        for m in both(Options::default()) {
            let mut m = ready(m);
            let out = frame_from(&mut m, &Peer::host(), text);
            assert_eq!(rejected(&out), Some(reason), "{text:?}");
            assert!(out.commands.is_empty());
            assert_eq!(m.state(), State::Ready, "the Ready job is untouched");
            assert!(m.frame_job().is_none());
        }
    }
    let mut m = ready(Machine::new(Options::default()));
    let wrong_hash = Message::SubmitJob {
        job_id: frame_id(),
        name: "f".into(),
        kind: JobKind::Frame,
        bytes: FRAME.len() as u64,
        sha256: sha256_hex("other"),
    };
    app(&mut m, wrong_hash);
    let out = app(
        &mut m,
        Message::JobChunk {
            job_id: frame_id(),
            index: 0,
            text: FRAME.to_owned(),
        },
    );
    assert_eq!(rejected(&out), Some(RejectReason::HashMismatch));
    let huge = Message::SubmitJob {
        job_id: frame_id(),
        name: "f".into(),
        kind: JobKind::Frame,
        bytes: 300 * 1024,
        sha256: sha256_hex(""),
    };
    assert_eq!(
        rejected(&app(&mut m, huge)),
        Some(RejectReason::TooLarge),
        "one chunk at most"
    );
}

#[test]
fn the_laser_mode_of_the_controller_does_not_refuse_a_frame() {
    for event in [Event::LaserModeOff(true), Event::LaserModeUnknown(true)] {
        let mut m = idle(Machine::new(Options::default()));
        m.handle(event);
        assert!(accepted(&frame_from(&mut m, &Peer::host(), FRAME)));
        assert_eq!(m.state(), State::Framing);
    }
}

// ---- Framing: what is refused, Stop, alarm, loss ----------------------------------------------

#[test]
fn while_framing_start_home_jog_pause_resume_and_new_jobs_are_refused() {
    for m in both(Options::default()) {
        let mut m = ready(m);
        frame_from(&mut m, &Peer::host(), FRAME);
        let refused = [
            Message::Start { job_id: job_id() },
            Message::Home,
            Message::Pause,
            Message::Resume,
            Message::Jog {
                axis: curvyo_connector_core::JogAxis::X,
                direction: curvyo_connector_core::JogDirection::Plus,
                step: curvyo_connector_core::JogStep::One,
                hold: false,
            },
        ];
        for message in refused {
            let out = app(&mut m, message.clone());
            assert_eq!(
                wrong_state(&out).as_deref(),
                Some("Framing."),
                "{message:?}"
            );
            assert!(out.commands.is_empty());
        }
        let out = app(&mut m, submit(JOB_TEXT, JobKind::Job));
        assert_eq!(rejected(&out), Some(RejectReason::Busy));
        assert_eq!(
            rejected(&app(&mut m, frame_submit(FRAME))),
            Some(RejectReason::Busy)
        );
        assert_eq!(m.state(), State::Framing);
    }
}

#[test]
fn stop_ends_a_frame_and_keeps_the_ready_job() {
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    let out = app(&mut m, Message::Stop);
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert_eq!(m.state(), State::Ready);
    assert_eq!(status_fields(&m).1, ready_progress());
    assert_eq!(status_fields(&m).2, None);
    let out = app(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(
        out.commands,
        [ControllerCommand::Start { job_id: job_id() }]
    );
}

#[test]
fn a_confirming_machine_waits_for_the_controller_to_stop() {
    let mut m = ready(Machine::confirming(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    let out = app(&mut m, Message::Stop);
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert_eq!(m.state(), State::Framing);
    match m.status() {
        Message::Status { detail, .. } => assert_eq!(detail.as_deref(), Some("Stopping")),
        _ => unreachable!(),
    }
    assert!(
        app(&mut m, Message::Stop).commands.is_empty(),
        "a second Stop is idempotent"
    );
    m.handle(Event::Stopped);
    assert_eq!(m.state(), State::Ready);
    assert_eq!(status_fields(&m).1, ready_progress());
    match m.status() {
        Message::Status { detail, .. } => assert_eq!(detail, None),
        _ => unreachable!(),
    }
}

#[test]
fn a_stop_from_idle_returns_to_idle_and_from_done_to_done() {
    let mut m = idle(Machine::confirming(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    app(&mut m, Message::Stop);
    m.handle(Event::Stopped);
    assert_eq!(m.state(), State::Idle);
    let mut m = done(Machine::confirming(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    app(&mut m, Message::Stop);
    m.handle(Event::Stopped);
    assert!(matches!(m.state(), State::Done { .. }));
}

#[test]
fn a_failed_frame_returns_with_its_detail() {
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::JobFailedWith {
        elapsed_ms: 0,
        detail: "Outside the travel by 12 mm".into(),
    });
    assert_eq!(m.state(), State::Ready);
    match m.status() {
        Message::Status { detail, .. } => {
            assert_eq!(detail.as_deref(), Some("Outside the travel by 12 mm"));
        }
        _ => unreachable!(),
    }
    // A GRBL answer names no line: the frame's lines are nobody's job lines.
    let mut m = idle(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::JobFailedWith {
        elapsed_ms: 0,
        detail: "Failed at line 3: No feed rate (error 22)".into(),
    });
    match m.status() {
        Message::Status { detail, .. } => {
            assert_eq!(detail.as_deref(), Some("No feed rate (error 22)"));
        }
        _ => unreachable!(),
    }
}

#[test]
fn an_alarm_drops_the_ready_job_and_a_lost_controller_the_frame() {
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::Alarm {
        text: "Hard limit".into(),
    });
    assert_eq!(m.state(), State::Alarm);
    assert!(m.held_job().is_none() && m.frame_job().is_none());
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::ControllerLost);
    assert_eq!(m.state(), State::NoController);
    assert!(m.frame_job().is_none());
    let mut m = ready(Machine::new(Options::default()));
    frame_from(&mut m, &Peer::host(), FRAME);
    m.handle(Event::ControllerReset);
    assert_eq!(
        m.state(),
        State::Ready,
        "a reset ends the frame, the job is kept"
    );
}

#[test]
fn the_homed_flag_travels_with_the_command() {
    let mut m = idle(Machine::new(Options::default()));
    app(&mut m, Message::Home);
    m.handle(Event::Homed);
    let out = frame_from(&mut m, &Peer::host(), FRAME);
    assert_eq!(
        out.commands,
        [ControllerCommand::Frame {
            job_id: frame_id(),
            homed: true
        }]
    );
}

// ---- F5: a member's frame ----------------------------------------------------------------------

#[test]
fn a_members_frame_waits_for_an_owner_who_confirms_it() {
    for m in both(OWNER_RULE) {
        let mut m = ready(m);
        let out = frame_from(&mut m, &member(), FRAME);
        assert!(accepted(&out));
        assert!(
            out.commands.is_empty(),
            "nothing moves before the owner agrees"
        );
        assert_eq!(m.state(), State::AwaitingLocalStart);
        let (_, job, id, requested_by) = status_fields(&m);
        assert_eq!(
            (job, id, requested_by),
            (None, Some(frame_id()), Some("Phone".into()))
        );
        // Nobody else frames, jogs, homes or starts meanwhile; a member cannot confirm.
        assert_eq!(
            rejected(&app(&mut m, frame_submit(FRAME))),
            Some(RejectReason::Busy)
        );
        assert!(wrong_state(&from(&mut m, &owner(), Message::Home)).is_some());
        assert!(
            wrong_state(&from(
                &mut m,
                &member(),
                Message::Start { job_id: frame_id() }
            ))
            .is_some()
        );
        assert!(
            wrong_state(&from(&mut m, &owner(), Message::Start { job_id: job_id() })).is_none(),
            "{:?}",
            m.state()
        );
        assert_eq!(
            m.state(),
            State::AwaitingLocalStart,
            "the held job's id confirms nothing"
        );
        let out = from(&mut m, &owner(), Message::Start { job_id: frame_id() });
        assert_eq!(
            out.commands,
            [ControllerCommand::Frame {
                job_id: frame_id(),
                homed: false
            }]
        );
        let confirmed = out.confirmed.unwrap();
        assert_eq!(confirmed.what, Confirms::Frame);
        assert_eq!(confirmed.requested_by, "Phone");
        assert_eq!(confirmed.confirmed_by.as_deref(), Some("Marc"));
        assert_eq!(confirmed.job_id, Some(frame_id()));
        assert_eq!(m.state(), State::Framing);
        m.handle(Event::JobFinished { elapsed_ms: 1 });
        assert_eq!(m.state(), State::Ready);
        assert_eq!(status_fields(&m).1, ready_progress());
    }
}

#[test]
fn stop_from_anyone_discards_a_waiting_frame_and_keeps_the_job() {
    for who in [owner(), member(), Peer::host()] {
        let mut m = ready(Machine::new(OWNER_RULE));
        frame_from(&mut m, &member(), FRAME);
        let out = from(&mut m, &who, Message::Stop);
        assert!(out.commands.is_empty());
        assert_eq!(m.state(), State::Ready);
        assert_eq!(status_fields(&m).1, ready_progress());
        assert!(m.frame_job().is_none());
    }
    let mut m = idle(Machine::new(OWNER_RULE));
    frame_from(&mut m, &member(), FRAME);
    from(&mut m, &owner(), Message::Stop);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn an_owners_own_frame_runs_at_once_under_the_owner_rule() {
    let mut m = ready(Machine::new(OWNER_RULE));
    let out = frame_from(&mut m, &owner(), FRAME);
    assert_eq!(out.commands.len(), 1);
    assert_eq!(m.state(), State::Framing);
    assert!(out.confirmed.is_none());
}

#[test]
fn under_the_local_press_rule_every_frame_waits_for_enter() {
    let mut m = ready(Machine::new(LOCAL_RULE));
    let out = frame_from(&mut m, &Peer::host(), FRAME);
    assert!(out.commands.is_empty());
    assert_eq!(m.state(), State::AwaitingLocalStart);
    let out = m.handle(Event::LocalStartPressed);
    assert_eq!(out.commands.len(), 1);
    assert_eq!(out.confirmed.unwrap().what, Confirms::Frame);
    assert_eq!(m.state(), State::Framing);
}

#[test]
fn a_lost_controller_drops_a_waiting_frame() {
    let mut m = ready(Machine::new(OWNER_RULE));
    frame_from(&mut m, &member(), FRAME);
    m.handle(Event::ControllerLost);
    assert!(m.frame_job().is_none());
    assert_eq!(m.state(), State::NoController);
}

// ---- F3: the check itself is the connector's, not the app's -----------------------------------

#[test]
fn the_check_is_reachable_for_the_controller_side() {
    let moves = check_frame(FRAME).unwrap();
    assert_eq!(moves.len(), 3);
}
