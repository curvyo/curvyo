//! Acceptance tests of `specs/0118` slice 2e (criteria 38, 39) and `specs/0132` slice 2
//! (criteria 12, 13), written from the specification and the architect's R1 to R10 before the
//! implementation was read. The GRBL simulator is the controller: every byte the connector writes
//! is on the simulator's line log, so the laser-off order, the rebuilt state and the refusals
//! before the first byte are checked on the wire. Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use std::fmt::Write as _;

use common::grbl_bench::{Bench, DT};
use common::grbl_sim::SimConfig;
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobResult, ListEntry, Message, Peer, State,
};
use curvyo_connector_io::controller::Controller;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The job: a preamble of four lines, then forty burn moves whose feed and power alternate, then
/// `M5` and `M9`. Line 4 is `M8` (air) so the rebuild has an air word to restore.
fn job() -> String {
    let mut text = "G21\nG90\nM8\nM4 S0\n".to_owned();
    for i in 0..40 {
        let feed = if i % 2 == 0 { 600 } else { 800 };
        let power = if i % 2 == 0 { 500 } else { 300 };
        let _ = writeln!(text, "G1 X{} Y{} F{feed} S{power}", 10 + i, 5 + i % 3);
    }
    text + "M5\nM9\n"
}

/// A GRBL that reports its machine position (`$10=1`), so the connector knows where it stands,
/// with homing on (`$22=1`, locked at power-up until `$X`), so the Home command really homes, and
/// the work origin inside the travel (machine X and Y run from -400 and -300 to 0 after a Home).
fn rig_config() -> SimConfig {
    SimConfig {
        mpos_reports: true,
        homing_lock: true,
        homing_steps: 20,
        wco: [-300.0, -200.0, 0.0],
        ..SimConfig::default()
    }
}

fn owner() -> Peer {
    Peer::host()
}

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// The bench driven so that every message the machine pushes is kept, for the list and errors.
struct Rig {
    bench: Bench,
    seen: Vec<Message>,
}

impl Rig {
    fn new(config: SimConfig) -> Self {
        let mut rig = Self {
            bench: Bench::connected(config),
            seen: Vec::new(),
        };
        rig.ask(Message::Unlock, owner());
        assert!(rig.run_until(|r| r.state() == State::Idle, 2_000));
        rig
    }

    /// Applies an event and carries out the commands it gives, keeping the messages.
    fn event(&mut self, event: Event) {
        let out = self.bench.machine.handle(event);
        self.seen.extend(out.messages);
        for command in out.commands {
            let job = if matches!(command, ControllerCommand::Frame { .. }) {
                self.bench.machine.frame_job()
            } else {
                self.bench.machine.held_job()
            };
            let events = self.bench.ctrl.command(&command, job, self.bench.at);
            for e in events {
                self.event(e);
            }
        }
    }

    fn ask(&mut self, message: Message, peer: Peer) -> Vec<Message> {
        let before = self.seen.len();
        self.event(Event::Message(message, peer));
        self.seen[before..].to_vec()
    }

    /// One step of time for the controller and the machine; the same as the bench's step.
    fn step(&mut self) {
        self.bench.at += DT;
        let from_sim = self.bench.sim.step();
        let at = self.bench.at;
        let advance = self.bench.ctrl.advance(&from_sim, at);
        self.bench.sim.receive(&advance.write);
        for e in advance.events {
            self.event(e);
        }
    }

    fn run_until(&mut self, done: impl Fn(&Self) -> bool, max: usize) -> bool {
        for _ in 0..max {
            if done(self) {
                return true;
            }
            self.step();
        }
        done(self)
    }

    /// Idle, or the job that Stop ended (`Done { Stopped }`, as built and `0127` G12).
    fn settled(&self) -> bool {
        matches!(
            self.state(),
            State::Idle
                | State::Done {
                    result: JobResult::Stopped
                }
        )
    }

    fn state(&self) -> State {
        self.bench.state()
    }

    fn progress(&self) -> Option<curvyo_connector_core::Progress> {
        match self.bench.status() {
            Message::Status { job, .. } => job,
            _ => None,
        }
    }

    /// The last list the machine pushed, as `(waiting)`.
    fn list(&self) -> Vec<ListEntry> {
        self.seen
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::JobList { waiting, .. } => Some(waiting.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn interrupted(&self) -> Option<ListEntry> {
        self.list().into_iter().find(|e| e.interrupted_at.is_some())
    }

    fn home(&mut self) {
        self.ask(Message::Home, owner());
        assert!(self.run_until(|r| r.settled() && r.homed(), 20_000));
    }

    fn homed(&self) -> bool {
        matches!(self.bench.status(), Message::Status { homed: true, .. })
    }

    fn detail(&self) -> Option<String> {
        self.bench.detail()
    }

    /// Sends the job, starts it and stops it once it has acknowledged `at_line` lines.
    fn run_and_stop(&mut self, text: &str, at_line: u32) -> Uuid {
        let id = self.bench.send(text);
        self.ask(Message::Start { job_id: id }, owner());
        assert!(self.run_until(|r| r.bench.line() >= at_line, 20_000));
        self.ask(Message::Stop, owner());
        let settled = self.run_until(Rig::settled, 20_000);
        assert!(
            settled,
            "after Stop: state {:?}, status {:?}",
            self.state(),
            self.bench.status()
        );
        id
    }

    fn restart(&mut self, job_id: Uuid, line: u32) -> Vec<Message> {
        self.ask(Message::RestartFromLine { job_id, line }, owner())
    }

    fn sim_lines(&self) -> Vec<String> {
        self.bench.sim.lines.clone()
    }
}

fn error_of(answers: &[Message]) -> Option<(ErrorCode, String)> {
    answers.iter().find_map(|m| match m {
        Message::Error { code, message } => Some((*code, message.clone())),
        _ => None,
    })
}

/// The simulator keeps its lines without spaces; the comparisons are on that form.
fn norm(line: &str) -> String {
    line.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Every `S` value of a normalised line.
fn s_values(line: &str) -> Vec<f64> {
    let mut values = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == 'S' {
            let digits: String =
                std::iter::from_fn(|| chars.next_if(|d| d.is_ascii_digit() || *d == '.')).collect();
            values.push(digits.parse().unwrap_or(f64::NAN));
        }
    }
    values
}

/// The travel to the start of line n: a `G1` at the jog feed with `S0`.
fn is_travel(line: &str) -> bool {
    line.starts_with("G1X") && line.ends_with("F3000S0")
}

#[test]
fn criterion_38_entry_move_is_laser_off_then_rebuilt_then_line_n() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let text = job();
    let id = rig.run_and_stop(&text, 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    let mark = rig.sim_lines().len();
    let answers = rig.restart(id, n);
    assert!(error_of(&answers).is_none(), "{answers:?}");
    let travelled = rig.run_until(
        |r| r.sim_lines().iter().skip(mark).any(|l| is_travel(&norm(l))),
        20_000,
    );
    assert!(travelled, "no travel line");
    // Wait until line n has been sent too.
    let line_n = norm(text.lines().nth(n as usize - 1).unwrap());
    assert!(rig.run_until(
        |r| r.sim_lines().iter().skip(mark).any(|l| norm(l) == line_n),
        20_000
    ));
    let lines: Vec<String> = rig.sim_lines().iter().skip(mark).map(|l| norm(l)).collect();
    // `$32` is read again right before the entry (adrs R8, the laser-mode check); before it only
    // the Stop's own `M5` and `M9` may still be on their way.
    let read = lines
        .iter()
        .position(|l| l == "$$")
        .expect("the laser-mode read");
    assert!(
        lines[..read].iter().all(|l| l == "M5" || l == "M9"),
        "only laser-off words before the read: {lines:?}"
    );
    let lines = &lines[read + 1..];
    // The entry starts laser off: M5 and M9 first, then the modal reset, then the travel.
    let modal = lines
        .iter()
        .position(|l| l == "G21G90G17G94")
        .expect("the modal reset");
    assert!(
        lines[..modal].iter().all(|l| l == "M5" || l == "M9"),
        "only laser-off words before the modal reset: {lines:?}"
    );
    assert!(lines[..modal].contains(&"M5".to_owned()), "{lines:?}");
    let travel = lines
        .iter()
        .position(|l| is_travel(l))
        .expect("a G1 travel at F3000 S0");
    // Nothing before the travel turns the laser on or sets an S above 0, and no G0.
    for l in &lines[..travel] {
        assert!(
            !l.contains("M3") && !l.contains("M4"),
            "laser before the travel: {l}"
        );
        assert!(
            s_values(l).iter().all(|v| *v == 0.0),
            "S above 0 before the travel: {l}"
        );
        assert!(!l.starts_with("G0"), "G0 ignores the feed: {l}");
    }
    // After the travel: the rebuilt state (air M8 from line 3), then S with M3 or M4, then line n.
    let after = &lines[travel + 1..];
    let air = after
        .iter()
        .position(|l| l == "M8")
        .expect("M8 rebuilt after the travel");
    let spindle = after
        .iter()
        .position(|l| l.contains("M3") || l.contains("M4"))
        .expect("the spindle word after the rebuild");
    assert!(air < spindle, "M8 comes before the spindle: {after:?}");
    let pos_n = after
        .iter()
        .position(|l| *l == line_n)
        .unwrap_or_else(|| panic!("line {n} {line_n} not sent after the rebuild: {after:?}"));
    assert!(spindle < pos_n, "the spindle comes before line n");
}

#[test]
fn criterion_39_restart_line_is_acknowledged_minus_planner_never_later() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    rig.run_and_stop(&job(), 30);
    let shown = rig.interrupted().unwrap();
    let acked = shown.interrupted_at.unwrap();
    let n = shown.restart_line.unwrap();
    assert!(n >= 1, "never before the first line");
    assert!(n <= acked, "never later than the acknowledged line");
    // The simulator reports a planner of 15 blocks in $I: n is 15 motion lines back.
    assert_eq!(
        n,
        acked - 14,
        "acked {acked}, n {n}: first of the 15 motion lines that may be unburned (adrs R3)"
    );
}

#[test]
fn criterion_39_a_stop_without_home_at_the_same_position_is_accepted_per_390c() {
    // Frage 390 C: a Stop that broke nothing proves the position by the same machine position.
    let mut rig = Rig::new(rig_config());
    let text = job();
    let id = rig.run_and_stop(&text, 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    let answers = rig.restart(id, n);
    assert!(
        error_of(&answers).is_none(),
        "refused after a Stop at the same position: {answers:?}"
    );
}

#[test]
fn criterion_38_stop_before_line_n_returns_the_entry_unchanged() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let id = rig.run_and_stop(&job(), 30);
    let before = rig.interrupted().unwrap();
    let n = before.restart_line.unwrap();
    let mark = rig.sim_lines().len();
    rig.restart(id, n);
    assert!(rig.run_until(|r| r.progress().is_some_and(|p| p.entering), 20_000));
    rig.ask(Message::Stop, owner());
    assert!(rig.run_until(Rig::settled, 20_000));
    let after = rig.interrupted().expect("back in the list");
    assert_eq!(after.restart_line, Some(n), "the same restart line");
    assert_eq!(
        after.interrupted_at, before.interrupted_at,
        "the same interruption"
    );
    let line_n = norm(job().lines().nth(n as usize - 1).unwrap());
    let written = rig.sim_lines();
    assert!(
        !written.iter().skip(mark).any(|l| norm(l) == line_n),
        "line n was never sent"
    );
}

#[test]
fn criterion_38_stop_after_line_n_interrupts_again_never_before_n() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let id = rig.run_and_stop(&job(), 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    rig.restart(id, n);
    assert!(rig.run_until(
        |r| r.progress().is_some_and(|p| !p.entering && p.line > n + 3),
        20_000
    ));
    rig.ask(Message::Stop, owner());
    assert!(rig.run_until(Rig::settled, 20_000));
    let again = rig
        .interrupted()
        .expect("Frage 390 C: a Stop interrupts again");
    assert!(
        again.restart_line.unwrap() >= n,
        "never before the run's first line"
    );
    assert!(again.interrupted_at.unwrap() >= n);
}

#[test]
fn criterion_38_laser_mode_off_is_refused_before_the_first_byte() {
    // `$32` set to 0 on the controller after connect (its own panel, another program): the
    // connector reads `$$` again right before the entry, so no byte of the entry goes out.
    let mut rig = Rig::new(rig_config());
    rig.home();
    let id = rig.run_and_stop(&job(), 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    for _ in 0..50 {
        // The Stop's own `M5` and `M9` reach the simulator first.
        rig.step();
    }
    rig.bench.sim.set_setting(32, "0");
    let written = rig.sim_lines().len();
    let answers = rig.restart(id, n);
    assert!(
        error_of(&answers).is_none(),
        "the cached $32=1 lets the read go out: {answers:?}"
    );
    assert!(rig.run_until(|r| r.state() == State::Idle, 2_000));
    assert_eq!(
        rig.sim_lines()[written..],
        ["$$".to_owned()],
        "only the read of $32, no byte of the entry"
    );
    let detail = rig.detail().unwrap_or_default();
    assert!(detail.contains("Laser mode is off"), "{detail}");
    let back = rig.interrupted().expect("the entry is back, unchanged");
    assert_eq!(back.restart_line, Some(n));
    assert_eq!(
        back.restart_cause.as_deref(),
        Some("Laser mode is off ($32=0)")
    );
    // The cached value is fresh now: the next press is refused at once, with no byte at all.
    let written = rig.sim_lines().len();
    let answers = rig.restart(id, n);
    let (code, text) = error_of(&answers).expect("refused");
    assert_eq!(code, ErrorCode::WrongState);
    assert!(text.contains("Laser mode is off"), "{text}");
    for _ in 0..50 {
        rig.step();
    }
    assert_eq!(rig.sim_lines().len(), written, "no byte reached the port");
}

#[test]
fn criterion_38_a_head_position_job_is_refused_before_the_first_byte() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let mut text = "G21\nG91\nM4 S0\n".to_owned();
    for i in 0..40 {
        let _ = writeln!(text, "G1 X1 Y{} F600 S500", 1 + i % 2);
    }
    text += "M5\n";
    let id = rig.run_and_stop(&text, 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    let answers = rig.restart(id, n);
    let (code, message) = error_of(&answers).expect("refused");
    assert_eq!(code, ErrorCode::WrongState);
    assert!(message.contains("Starts at the head position"), "{message}");
}

#[test]
fn criterion_12_a_stale_line_answers_wrong_state_with_a_fresh_list() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let id = rig.run_and_stop(&job(), 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    let answers = rig.restart(id, n + 1);
    let (code, _) = error_of(&answers).expect("refused");
    assert_eq!(code, ErrorCode::WrongState);
    let fresh = answers.iter().any(|m| matches!(m, Message::JobList { .. }));
    assert!(fresh, "a fresh list comes with the refusal: {answers:?}");
    assert!(rig.settled());
}

#[test]
fn criterion_12_a_unknown_job_is_refused() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let answers = rig.restart(Uuid::from_u128(0xdead), 5);
    let (code, _) = error_of(&answers).expect("refused");
    assert_eq!(code, ErrorCode::UnknownJob);
}

#[test]
fn criterion_38_progress_reads_entering_then_from_line() {
    let mut rig = Rig::new(rig_config());
    rig.home();
    let id = rig.run_and_stop(&job(), 30);
    let n = rig.interrupted().unwrap().restart_line.unwrap();
    rig.restart(id, n);
    assert!(rig.run_until(|r| r.progress().is_some_and(|p| p.entering), 20_000));
    let p = rig.progress().unwrap();
    assert_eq!(p.from_line, Some(n), "from_line names the restart line");
    assert!(rig.run_until(|r| r.progress().is_some_and(|p| !p.entering), 20_000));
    let p = rig.progress().unwrap();
    assert_eq!(p.from_line, Some(n));
    assert!(
        p.line >= n - 1,
        "the count runs in the whole text from n - 1"
    );
}

// ---- The hub: rights, the logs, the fake controller ------------------------------------------

mod hub_part {
    use std::time::{Duration, Instant};

    use curvyo_connector_core::{ErrorCode, JobKind, ListEntry, Message, Peer, Role};
    use curvyo_connector_io::hub::{ControllerChoice, FakeSettings, Hub, HubSettings};
    use curvyo_connector_io::spki::SpkiHash;
    use uuid::Uuid;

    use super::{common, error_of, job, sha};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn hub(dir: &tempfile::TempDir, started: Instant) -> Hub {
        let (hub, _notices) = Hub::new(
            HubSettings {
                state_dir: dir.path().to_owned(),
                controller: ControllerChoice::Fake(FakeSettings {
                    lines_per_second: 20,
                    alarm_at_line: None,
                }),
                require_local_start: false,
                connector_name: "Test".into(),
                spki: SpkiHash::of_spki_der(b"test"),
            },
            started,
        );
        hub
    }

    fn id() -> Uuid {
        Uuid::from_u128(0x2e)
    }

    /// Homes, runs the job for two seconds at 20 lines a second, and stops it.
    fn stopped(hub: &Hub, t0: Instant) -> Instant {
        let owner = Peer::new("Laptop", Role::Owner);
        hub.home(&owner, t0);
        hub.tick(t0 + ms(2_500));
        let text = job();
        hub.handle_message(
            Message::SubmitJob {
                job_id: id(),
                name: "sign.gcode".into(),
                kind: JobKind::Job,
                bytes: text.len() as u64,
                sha256: sha(&text),
            },
            &owner,
            t0 + ms(2_500),
        );
        hub.handle_message(
            Message::JobChunk {
                job_id: id(),
                index: 0,
                text,
            },
            &owner,
            t0 + ms(2_500),
        );
        hub.handle_message(Message::Start { job_id: id() }, &owner, t0 + ms(2_500));
        hub.tick(t0 + ms(4_500));
        hub.handle_message(Message::Stop, &owner, t0 + ms(4_500));
        t0 + ms(4_500)
    }

    fn interrupted(hub: &Hub) -> Option<ListEntry> {
        match hub.job_list() {
            Message::JobList { waiting, .. } => {
                waiting.into_iter().find(|e| e.interrupted_at.is_some())
            }
            _ => None,
        }
    }

    /// The rights are the session's (`admit`), so the member goes over a real connection.
    #[test]
    fn criterion_12_a_member_is_refused_owner_only_on_the_wire_and_an_owner_is_not() {
        let svc = common::service::service();
        let (mut owner, token) = svc.setup(20);
        let (_member_token, mut member) = svc.pair(&mut owner, "Phone", Role::Member);
        let _ = token;
        let order = Message::RestartFromLine {
            job_id: Uuid::from_u128(0xabc),
            line: 5,
        };
        let answer = member.request(&order, common::service::deadline()).unwrap();
        assert_eq!(
            common::service::code_of(&answer),
            Some(ErrorCode::OwnerRequired),
            "member: {answer:?}"
        );
        let answer = owner.request(&order, common::service::deadline()).unwrap();
        assert_eq!(
            common::service::code_of(&answer),
            Some(ErrorCode::UnknownJob),
            "owner reaches the job check: {answer:?}"
        );
    }

    #[test]
    fn criterion_13_an_owner_restart_writes_both_logs_with_the_owner_job_and_line() {
        let dir = tempfile::tempdir().unwrap();
        let t0 = Instant::now();
        let hub = hub(&dir, t0);
        let at = stopped(&hub, t0);
        let shown = interrupted(&hub).expect("listed as interrupted");
        let n = shown.restart_line.unwrap();
        let owner = Peer::new("Laptop", Role::Owner);
        let answers = hub.handle_message(
            Message::RestartFromLine {
                job_id: shown.job_id,
                line: n,
            },
            &owner,
            at,
        );
        assert!(error_of(&answers).is_none(), "{answers:?}");
        let jobs = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap_or_default();
        assert!(
            jobs.contains(&format!("Restart from line {n}")),
            "jobs.log: {jobs:?}"
        );
        assert!(
            jobs.contains("Laptop"),
            "the client name is in the job's log: {jobs:?}"
        );
        let admin = std::fs::read_to_string(dir.path().join("admin.log")).unwrap_or_default();
        assert!(
            admin.contains("sign.gcode"),
            "admin_restart.log names the job: {admin:?}"
        );
        assert!(
            admin.contains(&n.to_string()),
            "admin_restart.log names the line: {admin:?}"
        );
        assert!(
            admin.contains("Laptop"),
            "admin_restart.log names the owner: {admin:?}"
        );
    }
}
