//! Acceptance tests for 0064 slice 2 (Q1 to Q4, Q5, Q8, Q9, Q13 to Q19), written from the
//! specification before reading the implementation. They drive the real binary on 127.0.0.1 with
//! `--fake-controller` and a temporary state directory, as a client of the protocol.
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use curvyo_connector_core::{ErrorCode, JobKind, JobResult, Message, RejectReason, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, Connection, connect, open_paired};
use curvyo_connector_io::frames::{Frame, IoFrame, RefusalReason};
use curvyo_connector_io::job_send::SendError;
use curvyo_connector_io::pairing::PairError;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use uuid::Uuid;

const BIN: &str = env!("CARGO_BIN_EXE_curvyo-connector-server");
const WAIT: Duration = Duration::from_secs(20);

struct Running {
    child: Child,
    lines: Receiver<String>,
    address: String,
    code: String,
    fingerprint: String,
    head: Vec<String>,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_on(dir: &std::path::Path, listen: &str, extra: &[&str]) -> Running {
    let mut child = Command::new(BIN)
        .args(["--fake-controller", "--listen", listen, "--state-dir"])
        .arg(dir)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let (sender, lines) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });
    let mut running = Running {
        child,
        lines,
        address: String::new(),
        code: String::new(),
        fingerprint: String::new(),
        head: Vec::new(),
    };
    while running.code.is_empty() {
        let line = running.lines.recv_timeout(WAIT).unwrap();
        if let Some(a) = line.strip_prefix("Listening on ") {
            a.clone_into(&mut running.address);
        }
        if let Some(f) = line.strip_prefix("Key fingerprint: ") {
            f.clone_into(&mut running.fingerprint);
        }
        if let Some(c) = line.strip_prefix("Pairing code: ") {
            c.split(',').next().unwrap().clone_into(&mut running.code);
        }
        running.head.push(line);
    }
    running
}

fn start(dir: &std::path::Path, extra: &[&str]) -> Running {
    start_on(dir, "127.0.0.1:0", extra)
}

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(BIN)
        .args(args)
        .arg("--state-dir")
        .arg(dir)
        .output()
        .unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn deadline() -> Instant {
    Instant::now() + WAIT
}

fn pair(server: &Running, name: &str) -> PairingRecord {
    let mut c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    c.pair(&server.code, name, deadline()).unwrap()
}

fn paired(server: &Running, record: &PairingRecord) -> (Connection, Message) {
    open_paired(&server.address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap()
}

fn job_text(lines: usize) -> String {
    "G1 X1 Y0\n".repeat(lines)
}

type Snapshot = (State, Option<(Uuid, u32, u32, u64)>, Option<String>);

fn snapshot(m: &Message) -> Snapshot {
    let Message::Status {
        state, job, alarm, ..
    } = m
    else {
        panic!("not a status: {m:?}");
    };
    (
        *state,
        job.as_ref()
            .map(|p| (p.job_id, p.line, p.lines, p.elapsed_ms)),
        alarm.clone(),
    )
}

fn get_status(c: &mut Connection) -> Snapshot {
    c.send(&Message::GetStatus).unwrap();
    let until = deadline();
    loop {
        let m = c.recv_message(until).unwrap();
        if matches!(m, Message::Status { .. }) {
            return snapshot(&m);
        }
    }
}

fn wait_for(c: &mut Connection, ok: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let until = deadline();
    loop {
        let m = c.recv_message(until).unwrap();
        if matches!(m, Message::Status { .. }) {
            let s = snapshot(&m);
            if ok(&s) {
                return s;
            }
        }
    }
}

fn submit(c: &mut Connection, name: &str, text: &str) -> Uuid {
    c.send_job(name, text, deadline()).unwrap().job_id
}

fn start_job(c: &mut Connection, id: Uuid) {
    c.send(&Message::Start { job_id: id }).unwrap();
}

fn is_running(s: &Snapshot) -> bool {
    s.0 == State::Running
}

fn is_done(s: &Snapshot) -> bool {
    matches!(s.0, State::Done { .. })
}

// ---------------------------------------------------------------- Q1

#[test]
fn q1_start_prints_address_code_and_fingerprint_and_listens() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    assert!(
        server.address.starts_with("127.0.0.1:"),
        "{:?}",
        server.head
    );
    let groups: Vec<&str> = server.fingerprint.split(' ').collect();
    assert_eq!(groups.len(), 8, "{}", server.fingerprint);
    assert!(
        groups
            .iter()
            .all(|g| g.len() == 4 && g.bytes().all(|b| b.is_ascii_hexdigit()))
    );
    let plain: String = server.code.chars().filter(|c| *c != '-').collect();
    assert_eq!(plain.len(), 8, "{}", server.code);
    // The printed fingerprint is the SHA-256 of the key the server presents on the wire.
    let c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let wire = c.spki.to_hex().to_uppercase();
    assert!(
        wire.starts_with(&server.fingerprint.replace(' ', "")),
        "{wire} vs {}",
        server.fingerprint
    );
}

#[test]
fn q1_help_lists_every_flag() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, _) = run(dir.path(), &["--help"]);
    assert_eq!(code, 0);
    for flag in [
        "--fake-controller",
        "--listen",
        "--state-dir",
        "--pair",
        "--list-clients",
        "--revoke",
        "--require-local-start",
        "--fake-lines-per-second",
        "--fake-alarm-at-line",
    ] {
        assert!(out.contains(flag), "{flag} missing from --help:\n{out}");
    }
}

#[test]
fn q2_the_pair_flag_prints_a_code_and_codes_differ_between_starts() {
    let a = start(tempfile::tempdir().unwrap().path(), &["--pair"]);
    let b = start(tempfile::tempdir().unwrap().path(), &["--pair"]);
    assert_ne!(a.code, b.code);
}

// ---------------------------------------------------------------- Q2 over the wire

#[test]
fn q2_a_code_works_once() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let _ = pair(&server, "first");
    let mut c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let err = c.pair(&server.code, "second", deadline()).unwrap_err();
    assert!(
        matches!(
            err,
            PairError::Refused {
                reason: RefusalReason::NoCode,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn q2_q6_three_wrong_attempts_spend_the_code_and_count_down() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let wrong = if server.code.starts_with('2') {
        "3333-3333"
    } else {
        "2222-2222"
    };
    for left in [2u8, 1, 0] {
        let mut c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
        let err = c.pair(wrong, "x", deadline()).unwrap_err();
        assert_eq!(
            err,
            PairError::Refused {
                reason: RefusalReason::WrongCode,
                attempts_left: left
            }
        );
    }
    // The right code is now too late.
    let mut c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let err = c.pair(&server.code, "x", deadline()).unwrap_err();
    assert!(
        matches!(
            err,
            PairError::Refused {
                reason: RefusalReason::NoCode,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(run(dir.path(), &["--list-clients"]).1, "");
}

// ---------------------------------------------------------------- Q3

#[cfg(unix)]
#[test]
fn q3_key_file_is_0600_and_survives_a_restart_with_pairing_intact() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (record, fingerprint) = {
        let server = start(dir.path(), &[]);
        (pair(&server, "laptop"), server.fingerprint.clone())
    };
    let mut key_files = 0;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if std::fs::read_to_string(&path).is_ok_and(|t| t.contains("PRIVATE KEY")) {
            key_files += 1;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{path:?}");
        }
    }
    assert_eq!(key_files, 1);
    let second = start(dir.path(), &["--pair"]);
    assert_eq!(second.fingerprint, fingerprint);
    let (_, status) = paired(&second, &record);
    assert!(matches!(snapshot(&status).0, State::Idle));
}

#[test]
fn q3_a_damaged_key_file_stops_the_start_names_the_file_and_is_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    drop(start(dir.path(), &[]));
    let key = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| std::fs::read_to_string(p).is_ok_and(|t| t.contains("PRIVATE KEY")))
        .unwrap();
    std::fs::write(&key, "garbage").unwrap();
    let out = Command::new(BIN)
        .args([
            "--fake-controller",
            "--listen",
            "127.0.0.1:0",
            "--state-dir",
        ])
        .arg(dir.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    let name = key.file_name().unwrap().to_str().unwrap();
    assert!(err.contains(name), "{err}");
    assert_eq!(std::fs::read_to_string(&key).unwrap(), "garbage");
}

// ---------------------------------------------------------------- Q4

fn connect_raw(server: &Running) -> TcpStream {
    let s = TcpStream::connect(&server.address).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s
}

#[test]
fn q4_plain_http_and_plain_ws_get_no_message_and_the_server_lives_on() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    for request in [
        "GET /connector HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: x\r\n\r\n",
        "hello\r\n",
    ] {
        let mut s = connect_raw(&server);
        let _ = s.write_all(request.as_bytes());
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf);
        let text = String::from_utf8_lossy(&buf);
        assert!(!text.contains("HTTP/1.1 101"), "{text}");
        assert!(!text.contains("Hello"), "{text}");
        assert!(!text.contains("Welcome"), "{text}");
        assert!(!text.contains("\"type\""), "{text}");
    }
    assert!(connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).is_ok());
}

fn openssl(server: &Running, version: &str) -> (bool, String) {
    let mut child = Command::new("openssl")
        .args(["s_client", "-connect", &server.address, version])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (
        text.contains("Cipher is") && !text.contains("Cipher is (NONE)"),
        text,
    )
}

#[test]
fn q4_tls_1_2_and_lower_are_refused_and_tls_1_3_works() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let (ok13, text13) = openssl(&server, "-tls1_3");
    assert!(ok13, "TLS 1.3 handshake failed:\n{text13}");
    let (ok12, text12) = openssl(&server, "-tls1_2");
    assert!(!ok12, "TLS 1.2 was accepted:\n{text12}");
    assert!(connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).is_ok());
}

// ---------------------------------------------------------------- Q5, Q9

#[test]
fn q5_q9_only_the_hash_of_the_token_is_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let record = pair(&server, "Studio laptop");
    let token = record.token.to_hex().to_string();
    let mut files = 0;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        files += 1;
        assert!(
            !String::from_utf8_lossy(&bytes).contains(&token),
            "the plain token is on disk"
        );
    }
    assert!(files >= 2);
    let all: String = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap_or_default())
        .collect();
    assert!(all.contains(&record.token.sha256_hex()), "{all}");
    assert!(all.contains("Studio laptop"));
}

#[test]
fn q5_pairing_stores_the_seen_key_and_the_connector_name() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let record = pair(&server, "laptop");
    assert!(
        record
            .spki
            .to_hex()
            .to_uppercase()
            .starts_with(&server.fingerprint.replace(' ', ""))
    );
    assert_ne!(record.connector_name, "");
}

#[test]
fn q9_an_unpaired_client_gets_pairing_required_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let record = pair(&server, "laptop");
    let mut anon = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let id = Uuid::from_u128(0x0064_0000_0000_4000_8000_0000_0000_0001);
    let attempts = [
        Message::GetStatus,
        Message::SubmitJob {
            job_id: id,
            name: "evil".into(),
            kind: JobKind::Job,
            bytes: 0,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        },
        Message::Start { job_id: id },
        Message::Pause,
        Message::Resume,
        Message::Stop,
    ];
    for m in attempts {
        anon.send(&m).unwrap();
        let reply = anon.recv_message(deadline()).unwrap();
        assert!(
            matches!(
                reply,
                Message::Error {
                    code: ErrorCode::PairingRequired,
                    ..
                }
            ),
            "{m:?} -> {reply:?}"
        );
    }
    // An unknown token is no authentication either.
    let bogus = anon.auth(
        &curvyo_connector_io::token::Token::generate().unwrap(),
        deadline(),
    );
    assert_eq!(bogus.unwrap_err(), ClientError::NoLongerPaired);
    let (mut real, _) = paired(&server, &record);
    assert_eq!(get_status(&mut real).0, State::Idle);
}

#[test]
fn q9_list_clients_then_revoke_and_a_revoked_token_gets_pairing_required() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let record = pair(&server, "Studio laptop");
    let (code, out, _) = run(dir.path(), &["--list-clients"]);
    assert_eq!(code, 0);
    assert!(out.contains("Studio laptop"), "{out}");
    assert!(out.contains("20"), "a date is expected: {out}");
    assert_eq!(run(dir.path(), &["--revoke", "Studio laptop"]).0, 0);
    assert!(!run(dir.path(), &["--list-clients"]).1.contains("Studio"));
    let err = open_paired(&server.address, record.spki, &record.token, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(err, ClientError::NoLongerPaired);
}

// ---------------------------------------------------------------- Q8

#[test]
fn q8_a_changed_key_is_refused_with_both_fingerprints_before_the_token_is_sent() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let listen = format!("127.0.0.1:{port}");
    let first_dir = tempfile::tempdir().unwrap();
    let record = {
        let a = start_on(first_dir.path(), &listen, &[]);
        pair(&a, "laptop")
    };
    let second_dir = tempfile::tempdir().unwrap();
    let b = start_on(second_dir.path(), &listen, &[]);
    let err = open_paired(&b.address, record.spki, &record.token, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    let ClientError::KeyChanged { stored, presented } = err else {
        panic!("{err:?}");
    };
    assert_eq!(stored, record.spki);
    assert_ne!(presented, stored);
    assert!(
        presented
            .to_hex()
            .to_uppercase()
            .starts_with(&b.fingerprint.replace(' ', ""))
    );
    // The impostor learned nothing: no client and no token was ever offered to it.
    assert_eq!(run(second_dir.path(), &["--list-clients"]).1, "");
}

// ---------------------------------------------------------------- Q13

#[test]
fn q13_send_job_ends_in_ready_with_the_job_id_and_line_count() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let sent = c
        .send_job("square.gcode", &job_text(7), deadline())
        .unwrap();
    assert_eq!(sent.lines, 7);
    let (state, job, _) = get_status(&mut c);
    assert_eq!(state, State::Ready);
    let (id, line, lines, elapsed) = job.unwrap();
    assert_eq!((id, line, lines, elapsed), (sent.job_id, 0, 7, 0));
}

#[test]
fn q13_refusals_name_the_reason_hold_nothing_and_are_logged() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    // not ASCII
    let err = c
        .send_job("umlaut.gcode", "G1 X1\n; Grüße\n", deadline())
        .unwrap_err();
    assert_eq!(err, SendError::Rejected(RejectReason::NotAscii));
    assert_eq!(get_status(&mut c).0, State::Idle);
    // over 64 MiB, announced only
    let err = c
        .announce_oversized("big.txt", 64 * 1024 * 1024 + 1, deadline())
        .unwrap_err();
    assert_eq!(err, SendError::Rejected(RejectReason::TooLarge));
    assert_eq!(get_status(&mut c).0, State::Idle);
    // wrong hash
    let id = Uuid::from_u128(0x0064_0000_0000_4000_8000_0000_0000_0001);
    c.send(&Message::SubmitJob {
        job_id: id,
        name: "hash.gcode".into(),
        kind: JobKind::Job,
        bytes: 3,
        sha256: "0".repeat(64),
    })
    .unwrap();
    c.send(&Message::JobChunk {
        job_id: id,
        index: 0,
        text: "G1\n".into(),
    })
    .unwrap();
    let until = deadline();
    loop {
        let m = c.recv_message(until).unwrap();
        if let Message::JobRejected { reason, .. } = m {
            assert_eq!(reason, RejectReason::HashMismatch);
            break;
        }
    }
    assert_eq!(get_status(&mut c).0, State::Idle);
    // busy
    let held = submit(&mut c, "held.gcode", &job_text(3));
    let err = c.send_job("second.gcode", "G1\n", deadline()).unwrap_err();
    assert_eq!(err, SendError::Rejected(RejectReason::Busy));
    let (state, job, _) = get_status(&mut c);
    assert_eq!(state, State::Ready);
    assert_eq!(job.unwrap().0, held, "the held job is untouched");
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    for needle in [
        "umlaut.gcode\tRejected: NotAscii",
        "big.txt\tRejected: TooLarge",
        "hash.gcode\tRejected: HashMismatch",
        "second.gcode\tRejected: Busy",
    ] {
        assert!(log.contains(needle), "{needle} missing in\n{log}");
    }
}

// ---------------------------------------------------------------- Q14, Q15

#[test]
fn q14_start_runs_the_lines_to_completion_and_status_is_rate_limited() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "1500"]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "burst.gcode", &job_text(3000));
    let began = Instant::now();
    start_job(&mut c, id);
    let mut statuses = 0u32;
    let last;
    let until = Instant::now() + WAIT;
    loop {
        let m = c.recv_message(until).unwrap();
        if matches!(m, Message::Status { .. }) {
            statuses += 1;
            let s = snapshot(&m);
            if is_done(&s) {
                last = Some(s);
                break;
            }
        }
    }
    let seconds = began.elapsed().as_secs_f64();
    let (state, job, _) = last.unwrap();
    assert_eq!(
        state,
        State::Done {
            result: JobResult::Completed
        }
    );
    let (_, line, lines, _) = job.unwrap();
    assert_eq!((line, lines), (3000, 3000));
    assert!(statuses >= 2, "statuses {statuses}");
    let budget = 5.0 * seconds + 4.0;
    assert!(
        f64::from(statuses) <= budget,
        "{statuses} statuses in {seconds:.2} s exceed 5 per second"
    );
    assert!(seconds > 1.0, "3000 lines at 1500/s cannot take {seconds}");
}

#[test]
fn q14_the_default_pace_is_20_lines_per_second() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "pace.gcode", &job_text(40));
    start_job(&mut c, id);
    let done = wait_for(&mut c, is_done);
    let elapsed = done.1.unwrap().3;
    assert!(
        (1500..=3500).contains(&elapsed),
        "40 lines at 20/s took {elapsed} ms"
    );
}

#[test]
fn q15_pause_holds_the_line_resume_continues_stop_ends_as_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "100"]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "p.gcode", &job_text(5000));
    start_job(&mut c, id);
    wait_for(&mut c, |s| is_running(s) && s.1.is_some_and(|j| j.1 >= 5));
    c.send(&Message::Pause).unwrap();
    let paused = wait_for(&mut c, |s| s.0 == State::Paused);
    let held = paused.1.unwrap().1;
    std::thread::sleep(Duration::from_millis(600));
    let (state, job, _) = get_status(&mut c);
    assert_eq!(state, State::Paused);
    assert_eq!(job.unwrap().1, held, "the line count moved while paused");
    c.send(&Message::Resume).unwrap();
    let resumed = wait_for(&mut c, |s| is_running(s) && s.1.is_some_and(|j| j.1 > held));
    assert!(resumed.1.unwrap().1 >= held);
    c.send(&Message::Stop).unwrap();
    let stopped = wait_for(&mut c, is_done);
    assert_eq!(
        stopped.0,
        State::Done {
            result: JobResult::Stopped
        }
    );
    let at = stopped.1.unwrap().1;
    assert!(at > 0 && at < 5000);
    // a second Stop changes nothing
    c.send(&Message::Stop).unwrap();
    assert_eq!(get_status(&mut c).0, stopped.0);
}

// ---------------------------------------------------------------- Q16

#[test]
fn q16_the_job_survives_a_cut_connection_and_stop_works_from_a_fresh_one() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "100"]);
    let record = pair(&server, "laptop");
    let id = {
        let (mut c, _) = paired(&server, &record);
        let id = submit(&mut c, "survive.gcode", &job_text(5000));
        start_job(&mut c, id);
        wait_for(&mut c, |s| is_running(s) && s.1.is_some_and(|j| j.1 >= 3));
        id
    };
    std::thread::sleep(Duration::from_millis(500));
    let (mut fresh, status) = paired(&server, &record);
    let (state, job, _) = snapshot(&status);
    assert_eq!(state, State::Running, "the job stopped with the app");
    let (job_id, first, _, _) = job.unwrap();
    assert_eq!(job_id, id);
    std::thread::sleep(Duration::from_millis(400));
    let (_, job, _) = get_status(&mut fresh);
    assert!(job.unwrap().1 > first, "the line count does not advance");
    fresh.send(&Message::Stop).unwrap();
    let done = wait_for(&mut fresh, is_done);
    assert_eq!(
        done.0,
        State::Done {
            result: JobResult::Stopped
        }
    );
}

#[test]
fn q16_start_works_from_a_fresh_connection_with_the_id_of_the_ready_status() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "2000"]);
    let record = pair(&server, "laptop");
    let id = {
        let (mut c, _) = paired(&server, &record);
        submit(&mut c, "later.gcode", &job_text(50))
    };
    let (mut fresh, status) = paired(&server, &record);
    let (state, job, _) = snapshot(&status);
    assert_eq!(state, State::Ready);
    let from_status = job.unwrap().0;
    assert_eq!(from_status, id);
    start_job(&mut fresh, from_status);
    let done = wait_for(&mut fresh, is_done);
    assert_eq!(
        done.0,
        State::Done {
            result: JobResult::Completed
        }
    );
    assert_eq!(done.1.unwrap().1, 50);
}

// ---------------------------------------------------------------- Q17

#[test]
fn q17_local_start_waits_for_enter_and_stop_discards_the_job() {
    let dir = tempfile::tempdir().unwrap();
    let mut server = start(
        dir.path(),
        &["--require-local-start", "--fake-lines-per-second", "2000"],
    );
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "etch.gcode", &job_text(10));
    start_job(&mut c, id);
    wait_for(&mut c, |s| s.0 == State::AwaitingLocalStart);
    let line = loop {
        let l = server.lines.recv_timeout(WAIT).unwrap();
        if l.contains("Press Enter to start") {
            break l;
        }
    };
    assert!(line.contains("etch.gcode"), "{line}");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(get_status(&mut c).0, State::AwaitingLocalStart);
    server
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let done = wait_for(&mut c, is_done);
    assert_eq!(
        done.0,
        State::Done {
            result: JobResult::Completed
        }
    );
    // Stop in AwaitingLocalStart discards the job.
    let id = submit(&mut c, "second.gcode", &job_text(10));
    start_job(&mut c, id);
    wait_for(&mut c, |s| s.0 == State::AwaitingLocalStart);
    c.send(&Message::Stop).unwrap();
    wait_for(&mut c, |s| s.0 == State::Idle);
    assert!(
        get_status(&mut c).1.is_none(),
        "a discarded job still has progress"
    );
}

#[test]
fn q17_without_the_flag_start_runs_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "30"]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "now.gcode", &job_text(100));
    start_job(&mut c, id);
    let s = wait_for(&mut c, |s| {
        matches!(s.0, State::Running | State::AwaitingLocalStart)
    });
    assert_eq!(s.0, State::Running);
}

// ---------------------------------------------------------------- Q18

fn log_lines(dir: &std::path::Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(dir.join("jobs.log"))
        .unwrap()
        .lines()
        .map(|l| l.split('\t').map(str::to_owned).collect())
        .collect()
}

fn is_rfc3339_utc(t: &str) -> bool {
    let b = t.as_bytes();
    t.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && t.bytes()
            .enumerate()
            .all(|(i, c)| [4, 7, 10, 13, 16, 19].contains(&i) || c.is_ascii_digit())
}

#[test]
fn q18_every_outcome_appends_one_time_name_result_line() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &["--fake-lines-per-second", "2000"]);
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let a = submit(&mut c, "done.gcode", &job_text(5));
    start_job(&mut c, a);
    wait_for(&mut c, is_done);
    start_slow_stop(&server, &mut c);
    let rows = log_lines(dir.path());
    assert!(rows.len() >= 2, "{rows:?}");
    // `specs/0128` A15 (addendum D9): time, name, result, then sent by, started by, confirmed by.
    for row in &rows {
        assert_eq!(row.len(), 6, "{row:?}");
        assert!(is_rfc3339_utc(&row[0]), "{}", row[0]);
    }
    assert_eq!(
        rows[0][1..],
        ["done.gcode", "Completed", "laptop", "laptop", ""]
    );
    assert_eq!(
        rows[1][1..],
        ["stopme.gcode", "Stopped", "laptop", "laptop", ""]
    );
    // nothing else is a file of this program but the key, the clients and the log
    let mut names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    // `specs/0128` adds `admin.log` (A18) and `lock` (A21).
    assert_eq!(
        names,
        [
            "admin.log",
            "clients.json",
            "identity.pem",
            "jobs.log",
            "lock"
        ],
        "unexpected extra files: {names:?}"
    );
}

/// Submits a job, stops it right after the start. Returns nothing the caller needs.
fn start_slow_stop(_server: &Running, c: &mut Connection) {
    // The server of this test runs fast; a 200000 line job is still running when Stop arrives.
    let id = submit(c, "stopme.gcode", &job_text(200_000));
    start_job(c, id);
    wait_for(c, is_running);
    c.send(&Message::Stop).unwrap();
    wait_for(c, is_done);
}

// ---------------------------------------------------------------- Q19

#[test]
fn q19_the_alarm_stops_the_stream_is_reported_logged_and_busy_answers_a_submit() {
    let dir = tempfile::tempdir().unwrap();
    let server = start(
        dir.path(),
        &[
            "--fake-alarm-at-line",
            "6",
            "--fake-lines-per-second",
            "500",
        ],
    );
    let (mut c, _) = paired(&server, &pair(&server, "laptop"));
    let id = submit(&mut c, "alarm.gcode", &job_text(100));
    start_job(&mut c, id);
    let alarm = wait_for(&mut c, |s| s.0 == State::Alarm);
    let text = alarm.2.clone().unwrap();
    assert_ne!(text, "");
    let line = alarm.1.map(|j| j.1);
    std::thread::sleep(Duration::from_millis(500));
    let (state, job, again) = get_status(&mut c);
    assert_eq!(state, State::Alarm);
    assert_eq!(again.as_deref(), Some(text.as_str()));
    assert_eq!(job.map(|j| j.1), line, "the stream went on after the alarm");
    assert!(line.is_none_or(|l| l <= 6 + 1), "stopped late at {line:?}");
    let err = c.send_job("next.gcode", "G1\n", deadline()).unwrap_err();
    assert_eq!(err, SendError::Rejected(RejectReason::Busy));
    let rows = log_lines(dir.path());
    assert!(
        rows.iter()
            .any(|r| r[1] == "alarm.gcode" && r[2] == format!("Alarm: {text}")),
        "{rows:?}"
    );
}

#[test]
fn q9_pairing_frames_are_not_a_way_around_the_token() {
    // Pairing answers with PairRefused and no token for a client that sends garbage in round 2.
    let dir = tempfile::tempdir().unwrap();
    let server = start(dir.path(), &[]);
    let mut c = connect(&server.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    c.send_frame(&Frame::Io(IoFrame::PairSpake {
        round: 2,
        client_name: None,
        message: "00".into(),
    }))
    .unwrap();
    let reply = c.recv(deadline());
    assert!(
        !matches!(reply, Ok(Frame::Core(Message::Paired { .. }))),
        "{reply:?}"
    );
    assert_eq!(run(dir.path(), &["--list-clients"]).1, "");
}
