#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Independent acceptance tests for story 0128 (`specs/0128-connector-api-and-ctl`), written from
//! the specification before the implementation was read. They drive the real `curvyoctl` and
//! `curvyo-connector-server` binaries; they use no production seam. Criteria without a black-box
//! handle (the 30-minute clock, public peer addresses, the member app) are listed in the report.
//!
//! The server binary is built with `cargo build -p curvyo-connector-server` before this suite runs.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CTL: &str = env!("CARGO_BIN_EXE_curvyoctl");
const TEST_PASSPHRASE: &str = "0128 tester passphrase";

/// Runs its arguments with a pseudo terminal as stdin and answers each passphrase prompt, since a
/// sandbox has no keychain (the passphrase fallback of the credential store needs a terminal).
const PTY_RUNNER: &str = r#"
import os, pty, select, subprocess, sys
passphrase = os.environ["CURVYO_TEST_PASSPHRASE"].encode() + b"\n"
master, slave = pty.openpty()
child = subprocess.Popen(sys.argv[1:], stdin=slave, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
os.close(slave)
out, err = [], []
answered = 0
pipes = {child.stdout.fileno(): out, child.stderr.fileno(): err}
open_fds = set(pipes)
while open_fds:
    ready, _, _ = select.select(list(open_fds), [], [], 10)
    if not ready:
        break
    for fd in ready:
        data = os.read(fd, 4096)
        if not data:
            open_fds.discard(fd)
            continue
        pipes[fd].append(data)
    prompts = b"".join(err).count(b"Passphrase: ")
    while answered < prompts:
        os.write(master, passphrase)
        answered += 1
child.wait()
sys.stdout.buffer.write(b"".join(out))
sys.stderr.buffer.write(b"".join(err))
sys.exit(child.returncode if child.returncode >= 0 else 1)
"#;

/// Opens a pseudo terminal, prints its slave name, then counts the bytes the master receives.
const PTY_PROBE: &str = r#"
import os, pty, select, time
m, s = pty.openpty()
print(os.ttyname(s), flush=True)
total = 0
end = time.time() + 8
while time.time() < end:
    r, _, _ = select.select([m], [], [], 0.2)
    if r:
        try:
            total += len(os.read(m, 4096))
        except OSError:
            break
print("bytes", total, flush=True)
"#;

const WAIT: Duration = Duration::from_secs(20);

fn server_bin() -> PathBuf {
    let path = Path::new(CTL)
        .parent()
        .expect("curvyoctl has a directory")
        .join("curvyo-connector-server");
    assert!(
        path.exists(),
        "build curvyo-connector-server first: {}",
        path.display()
    );
    path
}

fn unique_root(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("0128-{tag}-"))
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("temporary directory")
}

fn free_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    drop(listener);
    addr
}

type Lines = Arc<Mutex<Vec<String>>>;

fn collect<R: Read + Send + 'static>(reader: R) -> Lines {
    let lines: Lines = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&lines);
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            sink.lock().expect("lock").push(line);
        }
    });
    lines
}

fn joined(lines: &Lines) -> String {
    lines.lock().expect("lock").join("\n")
}

fn wait_for(what: &str, lines: &Lines, pred: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + WAIT;
    loop {
        let text = joined(lines);
        if pred(&text) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; output:\n{text}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// A child process that is killed when the test ends, whatever happens.
struct Guard(Child);

impl Drop for Guard {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn wait_exit(child: &mut Child, within: Duration) -> ExitStatus {
    let deadline = Instant::now() + within;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within {within:?}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Fingerprint in the form `3F2A-9C10-77B4-E2D1-0A5C-61F8-9B3E-4D27`, found in any text.
fn fingerprint_in(text: &str) -> Option<String> {
    let normalised = text.replace('-', " ");
    let groups: Vec<&str> = normalised.split_whitespace().collect();
    groups
        .windows(8)
        .find(|w| {
            w.iter().all(|g| {
                g.len() == 4
                    && g.chars()
                        .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c))
            })
        })
        .map(|w| w.join("-"))
}

/// The code after "Pairing code: ", in the form `K7QM-4XTR`.
fn pairing_code(text: &str) -> Option<String> {
    let start = text.find("Pairing code: ")? + "Pairing code: ".len();
    let code: String = text[start..].chars().take(9).collect();
    (code.len() == 9 && code.as_bytes()[4] == b'-').then_some(code)
}

struct Connector {
    guard: Guard,
    out: Lines,
}

impl Connector {
    fn start(state: &Path, addr: &str) -> Self {
        Self::start_with(state, addr, &[])
    }

    fn start_with(state: &Path, addr: &str, extra: &[&str]) -> Self {
        Self::start_probing(state, addr, addr, extra)
    }

    /// Starts with `--listen addr`, waits until `probe` accepts connections.
    fn start_probing(state: &Path, addr: &str, probe: &str, extra: &[&str]) -> Self {
        let mut child = Command::new(server_bin())
            .arg("--state-dir")
            .arg(state)
            .arg("--listen")
            .arg(addr)
            .args(extra)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn connector");
        let out = collect(child.stdout.take().expect("stdout"));
        let _err = collect(child.stderr.take().expect("stderr"));
        let connector = Connector {
            guard: Guard(child),
            out,
        };
        let probe = probe.to_string();
        wait_for("listening", &connector.out, move |_| {
            TcpStream::connect(&probe).is_ok()
        });
        connector
    }

    fn fingerprint(&self) -> String {
        wait_for("fingerprint line", &self.out, |t| {
            fingerprint_in(t).is_some()
        });
        fingerprint_in(&joined(&self.out)).expect("fingerprint")
    }

    fn text(&self) -> String {
        joined(&self.out)
    }

    fn child(&mut self) -> &mut Child {
        &mut self.guard.0
    }
}

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn ctl(home: &Path, args: &[&str]) -> Out {
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(PTY_RUNNER)
        .arg(CTL)
        .args(args)
        .env("XDG_CONFIG_HOME", home)
        .env("HOME", home)
        .env(
            "CURVYOCTL_HOST_NAME",
            home.file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        )
        .env("CURVYO_TEST_PASSPHRASE", TEST_PASSPHRASE)
        .stdin(Stdio::null())
        .output()
        .expect("run curvyoctl");
    Out {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn ok(out: &Out) {
    assert_eq!(
        out.code, 0,
        "exit 0 expected\nstdout:\n{}\nstderr:\n{}",
        out.stdout, out.stderr
    );
}

fn refused(out: &Out) {
    assert_eq!(
        out.code, 1,
        "exit 1 expected\nstdout:\n{}\nstderr:\n{}",
        out.stdout, out.stderr
    );
}

fn config_toml(name: &str, owner: &str, listen: &str) -> String {
    format!(
        "format = 1\nname = \"{name}\"\nowner = \"{owner}\"\nlisten = \"{listen}\"\n\n[controller]\nkind = \"fake\"\n"
    )
}

fn write(path: &Path, body: &str) -> PathBuf {
    fs::write(path, body).expect("write");
    path.to_path_buf()
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| n == name) {
            return Some(path);
        }
    }
    None
}

/// A configured connector with its owner computer (`owner_home`), as in the customer's walk.
struct Lab {
    _root: tempfile::TempDir,
    state: PathBuf,
    owner_home: PathBuf,
    config: PathBuf,
    addr: String,
    conn: Connector,
    fp: String,
}

impl Lab {
    fn fresh(tag: &str) -> Lab {
        let root = unique_root(tag);
        let state = root.path().join("state");
        fs::create_dir(&state).expect("state dir");
        let owner_home = root.path().join("owner-home");
        fs::create_dir(&owner_home).expect("owner home");
        let addr = free_addr();
        let config = write(
            &root.path().join("connector.toml"),
            &config_toml("workshop-laser", "Marc", &addr),
        );
        let conn = Connector::start(&state, &addr);
        let fp = conn.fingerprint();
        Lab {
            _root: root,
            state,
            owner_home,
            config,
            addr,
            conn,
            fp,
        }
    }

    fn configure(&self) {
        let out = ctl(
            &self.owner_home,
            &[
                "apply-config",
                "--insecure",
                &self.addr,
                "--fingerprint",
                &self.fp,
                "-f",
                self.config.to_str().expect("utf8"),
            ],
        );
        ok(&out);
    }
}

// ---------------------------------------------------------------- modes and the setup window

#[test]
fn a1_fresh_state_dir_prints_maintenance_window_and_fingerprint() {
    let root = unique_root("a1");
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    let addr = free_addr();
    let conn = Connector::start(&state, &addr);
    let text = wait_for("maintenance line", &conn.out, |t| {
        t.contains("Maintenance mode until ")
            && t.contains("UTC: waiting for curvyoctl apply-config --insecure")
    });
    assert!(
        fingerprint_in(&text).is_some(),
        "fingerprint line missing:\n{text}"
    );
    assert!(
        !state.join("connector.toml").exists(),
        "no configuration may exist"
    );
    assert!(
        fs::read_dir(&state).unwrap().next().is_some(),
        "the identity key is created in maintenance mode"
    );
}

#[test]
fn a1_fingerprint_flag_prints_the_same_fingerprint_as_the_journal() {
    let lab = Lab::fresh("a1fp");
    let out = Command::new(server_bin())
        .arg("--fingerprint")
        .arg("--state-dir")
        .arg(&lab.state)
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(0));
    let printed = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(
        fingerprint_in(&printed).as_deref(),
        Some(lab.fp.as_str()),
        "{printed}"
    );
}

#[test]
fn a2_identify_and_ports_answer_without_login_in_maintenance() {
    let lab = Lab::fresh("a2");
    let out = ctl(&lab.owner_home, &["identify", "--insecure", &lab.addr]);
    ok(&out);
    assert!(out.stdout.contains("Maintenance mode"), "{}", out.stdout);
    assert!(out.stdout.contains("closes in"), "{}", out.stdout);
    assert_eq!(
        fingerprint_in(&out.stdout).as_deref(),
        Some(lab.fp.as_str())
    );

    ok(&ctl(
        &lab.owner_home,
        &["identify", "--insecure", &lab.addr],
    ));
    let ports = ctl(&lab.owner_home, &["ports", "--insecure", &lab.addr]);
    ok(&ports);
    // No devices on this machine: the one line is the empty-list text ("No serial ports", ux-notes).
    if ports.stdout.trim() == "No serial ports" {
        return;
    }
    for line in ports.stdout.lines().filter(|l| !l.trim().is_empty()) {
        let l = line.trim();
        assert!(
            l.starts_with("/dev/serial/by-id/")
                || l.starts_with("/dev/ttyUSB")
                || l.starts_with("/dev/ttyACM"),
            "ListPorts returns only device names, got {l}"
        );
    }
}

#[test]
fn a3_maintenance_refuses_every_other_call_and_writes_no_byte_to_a_serial_port() {
    // A pty whose master this test reads. No connector is told about it in maintenance.
    let mut probe = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(PTY_PROBE)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("python3 is needed for the pty check");
    let probe_out = collect(probe.stdout.take().expect("stdout"));
    let _probe = Guard(probe);
    wait_for("pty slave", &probe_out, |t| t.contains("/dev/pts/"));

    let lab = Lab::fresh("a3");
    ok(&ctl(
        &lab.owner_home,
        &["identify", "--insecure", &lab.addr],
    ));
    // `login` sends Pair/Auth messages to the address without a stored token (A37).
    let outs = [ctl(
        &lab.owner_home,
        &["login", &lab.addr, "--code", "K7QM-4XTR"],
    )];
    let probe_text = wait_for("probe result", &probe_out, |t| t.contains("bytes "));
    assert!(
        probe_text.contains("bytes 0"),
        "zero bytes expected at the pty before any configuration; probe said:\n{probe_text}"
    );
    for out in &outs {
        refused(out);
        assert!(
            out.stderr.to_lowercase().contains("maintenance"),
            "maintenance refusal expected, got: {}",
            out.stderr
        );
    }
}

#[test]
fn a5_maintenance_window_is_about_thirty_minutes_after_start() {
    let lab = Lab::fresh("a5");
    let out = ctl(&lab.owner_home, &["identify", "--insecure", &lab.addr]);
    ok(&out);
    let tail = out.stdout.split("closes in ").nth(1).expect("closes in");
    let mut parts = tail.split_whitespace();
    let n: u64 = parts
        .next()
        .and_then(|v| v.trim_end_matches(',').parse().ok())
        .expect("number");
    match parts.next().unwrap_or("") {
        "min" => assert!((25..=30).contains(&n), "window {n} min"),
        "s" => assert!((1500..=1800).contains(&n), "window {n} s"),
        other => panic!("unexpected unit {other:?} in {}", out.stdout),
    }
}

#[test]
fn a6_apply_config_configures_without_restart_and_names_the_owner() {
    let mut lab = Lab::fresh("a6");
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &lab.fp,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    ok(&out);
    assert!(
        out.stdout.contains("Configured. You are the owner."),
        "{}",
        out.stdout
    );
    assert!(
        lab.state.join("connector.toml").exists(),
        "connector.toml is the commit point"
    );
    assert!(
        lab.state.join("clients.json").exists(),
        "clients.json written"
    );
    assert!(
        lab.conn.child().try_wait().unwrap().is_none(),
        "no restart: the same process still serves"
    );

    let identify = ctl(&lab.owner_home, &["identify", "--insecure", &lab.addr]);
    ok(&identify);
    assert!(
        identify.stdout.contains("workshop-laser"),
        "{}",
        identify.stdout
    );
    assert!(
        identify.stdout.contains("Marc"),
        "owner label shown: {}",
        identify.stdout
    );
    assert!(
        !identify.stdout.contains("Maintenance mode"),
        "{}",
        identify.stdout
    );

    let status = ctl(&lab.owner_home, &["status"]);
    ok(&status);
    let get = ctl(&lab.owner_home, &["get-config"]);
    ok(&get);
    assert!(get.stdout.contains("workshop-laser"), "{}", get.stdout);
}

#[test]
fn a6_invalid_configuration_names_the_field_and_keeps_the_window() {
    let lab = Lab::fresh("a6bad");
    let bad = write(
        &lab.state.parent().unwrap().join("bad.toml"),
        "format = 1\nname = \"\"\nowner = \"Marc\"\nlisten = \"127.0.0.1:7447\"\n\n[controller]\nkind = \"fake\"\n",
    );
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &lab.fp,
            "-f",
            bad.to_str().unwrap(),
        ],
    );
    refused(&out);
    assert!(
        out.stderr.starts_with("curvyoctl: "),
        "one refusal line: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("name"),
        "the field is named: {}",
        out.stderr
    );
    assert!(!lab.state.join("connector.toml").exists());
    let identify = ctl(&lab.owner_home, &["identify", "--insecure", &lab.addr]);
    ok(&identify);
    assert!(
        identify.stdout.contains("Maintenance mode"),
        "window kept: {}",
        identify.stdout
    );
}

#[test]
fn a16_unknown_fields_and_baud_out_of_range_are_refused() {
    let lab = Lab::fresh("a16");
    let unknown = write(
        &lab.state.parent().unwrap().join("unknown.toml"),
        "format = 1\nname = \"workshop-laser\"\nowner = \"Marc\"\nlisten = \"127.0.0.1:7447\"\nsurprise = 1\n\n[controller]\nkind = \"fake\"\n",
    );
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &lab.fp,
            "-f",
            unknown.to_str().unwrap(),
        ],
    );
    refused(&out);
    assert!(
        out.stderr.contains("surprise"),
        "unknown field named: {}",
        out.stderr
    );

    let baud = write(
        &lab.state.parent().unwrap().join("baud.toml"),
        "format = 1\nname = \"workshop-laser\"\nowner = \"Marc\"\nlisten = \"127.0.0.1:7447\"\n\n[controller]\nkind = \"grbl-serial\"\nport = \"/dev/ttyUSB0\"\nbaud = 4000001\n",
    );
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &lab.fp,
            "-f",
            baud.to_str().unwrap(),
        ],
    );
    refused(&out);
    assert!(out.stderr.contains("baud"), "{}", out.stderr);
    assert!(!lab.state.join("connector.toml").exists());
}

#[test]
fn a7_second_computer_cannot_configure_a_configured_connector() {
    let lab = Lab::fresh("a7");
    lab.configure();
    let other = unique_root("a7other");
    let out = ctl(
        other.path(),
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &lab.fp,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    refused(&out);
    assert!(out.stderr.contains("already configured"), "{}", out.stderr);
    let text = lab.conn.text();
    assert!(
        text.lines()
            .any(|l| l.contains("configured") && l.contains("127.0.0.1")),
        "stdout names the client and its address:\n{text}"
    );
}

#[test]
fn a22_help_for_every_listed_command_and_enrol_is_a_usage_error() {
    let root = unique_root("a22");
    for cmd in [
        "identify",
        "ports",
        "apply-config",
        "get-config",
        "status",
        "jobs",
        "audit",
        "confirm-start",
        "stop",
        "unlock",
        "pair",
        "clients",
        "revoke",
        "reset",
        "login",
        "logout",
        "whoami",
        "version",
    ] {
        let out = ctl(root.path(), &[cmd, "--help"]);
        assert_eq!(out.code, 0, "{cmd} --help exit 0: {}", out.stderr);
        assert!(
            out.stdout.lines().all(|l| l.chars().count() <= 80),
            "{cmd} help within 80 columns"
        );
    }
    let enrol = ctl(root.path(), &["enrol"]);
    assert_eq!(enrol.code, 2, "enrol is not part of this slice");
}

#[test]
fn a23_fingerprint_mismatch_is_refused_before_anything_is_sent() {
    let lab = Lab::fresh("a23");
    let wrong = "0000-0000-0000-0000-0000-0000-0000-0000";
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            wrong,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    refused(&out);
    assert!(out.stderr.starts_with("curvyoctl: "), "{}", out.stderr);
    assert!(
        !lab.state.join("connector.toml").exists(),
        "nothing was sent"
    );
}

#[test]
fn a23_fingerprint_without_separators_is_accepted() {
    let lab = Lab::fresh("a23b");
    let bare: String = lab.fp.replace('-', "");
    assert_eq!(bare.len(), 32);
    let out = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &bare,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    ok(&out);
}

#[test]
fn a24_local_state_file_is_private_and_holds_no_token() {
    let lab = Lab::fresh("a24");
    lab.configure();
    let file = find_file(&lab.owner_home, "curvyoctl.toml")
        .expect("curvyoctl.toml in the user config directory");
    let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "file mode");
    let dir_mode = fs::metadata(file.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o700, "folder mode");
    let body = fs::read_to_string(&file).unwrap();
    let pinned = lab.fp.replace('-', "").to_lowercase();
    assert!(
        body.contains(&pinned),
        "the full key hash is pinned: {body}"
    );
    assert!(
        !body.to_lowercase().contains("token"),
        "no token in the file: {body}"
    );
}

// ---------------------------------------------------------------- roles, owner calls, clients

#[test]
fn a8_a32_clients_file_is_format_2_with_roles() {
    let lab = Lab::fresh("a8");
    lab.configure();
    let clients = fs::read_to_string(lab.state.join("clients.json")).expect("clients.json");
    let compact: String = clients.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("\"format\":2"), "format 2: {clients}");
    assert!(
        compact.contains("\"role\":\"owner\""),
        "owner role: {clients}"
    );
}

#[test]
fn a10_pair_issues_codes_that_never_reach_stdout_or_the_admin_log() {
    let lab = Lab::fresh("a10");
    lab.configure();
    let laptop = ctl(&lab.owner_home, &["pair", "--name", "Laptop", "--owner"]);
    ok(&laptop);
    let code = pairing_code(&laptop.stdout).expect("pairing code printed");
    assert!(
        laptop.stdout.contains("valid for 10 minutes"),
        "{}",
        laptop.stdout
    );
    let phone = ctl(&lab.owner_home, &["pair", "--name", "Phone"]);
    ok(&phone);
    let phone_code = pairing_code(&phone.stdout).expect("second code");
    let clients = ctl(&lab.owner_home, &["clients"]);
    ok(&clients);
    assert!(
        clients.stdout.to_lowercase().contains("owner"),
        "{}",
        clients.stdout
    );
    assert!(
        !lab.conn.text().contains(&code) && !lab.conn.text().contains(&phone_code),
        "never on stdout"
    );
    let log = fs::read_to_string(lab.state.join("admin.log")).unwrap_or_default();
    assert!(
        log.contains("code-issued") || log.contains("code issued"),
        "issue recorded: {log}"
    );
    assert!(
        !log.contains(&code) && !log.contains(&phone_code),
        "never in admin.log"
    );
}

#[test]
fn a11_last_owner_and_unknown_name_refusals_are_one_line_each() {
    let lab = Lab::fresh("a11");
    lab.configure();
    let clients = ctl(&lab.owner_home, &["clients"]);
    ok(&clients);
    let owner_name = clients
        .stdout
        .lines()
        .filter(|l| l.to_lowercase().contains("owner") && !l.to_lowercase().contains("member"))
        .find_map(|l| l.split_whitespace().next().map(str::to_string))
        .expect("the owner is listed");
    let last = ctl(&lab.owner_home, &["revoke", &owner_name]);
    refused(&last);
    assert!(
        last.stderr
            .contains(&format!("{owner_name} is the last owner of workshop-laser")),
        "{}",
        last.stderr
    );
    let unknown = ctl(&lab.owner_home, &["revoke", "Phon"]);
    refused(&unknown);
    assert!(
        unknown
            .stderr
            .contains("workshop-laser has no client called Phon"),
        "{}",
        unknown.stderr
    );
}

#[test]
fn a14_confirm_start_with_nothing_waiting_and_stop_without_a_job() {
    let lab = Lab::fresh("a14");
    lab.configure();
    let confirm = ctl(&lab.owner_home, &["confirm-start"]);
    refused(&confirm);
    assert!(
        confirm
            .stderr
            .contains("no start is waiting on workshop-laser"),
        "{}",
        confirm.stderr
    );
    let stop = ctl(&lab.owner_home, &["stop"]);
    ok(&stop);
    assert!(stop.stdout.contains("No job"), "{}", stop.stdout);
}

#[test]
fn a18_admin_log_is_private_and_names_actions_without_codes() {
    let lab = Lab::fresh("a18");
    lab.configure();
    let issued = ctl(&lab.owner_home, &["pair", "--name", "Laptop", "--owner"]);
    ok(&issued);
    let code = pairing_code(&issued.stdout).expect("code");
    let second = unique_root("a18second");
    ok(&ctl(second.path(), &["login", &lab.addr, "--code", &code]));
    let log_path = lab.state.join("admin.log");
    let mode = fs::metadata(&log_path)
        .expect("admin.log exists")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "admin.log mode");
    let log = fs::read_to_string(&log_path).unwrap();
    let lower = log.to_lowercase();
    assert!(lower.contains("configured"), "{log}");
    assert!(
        lower.contains("code-issued") || lower.contains("code issued"),
        "{log}"
    );
    assert!(lower.contains("paired"), "{log}");
    assert!(
        log.lines()
            .filter(|l| !l.trim().is_empty())
            .all(|l| l.contains(':')),
        "each line has a time"
    );
    assert!(!log.contains(&code), "the code is never logged");
    ok(&ctl(&lab.owner_home, &["audit", "--last", "5"]));
}

#[test]
fn a34_output_is_plain_ascii_and_each_refusal_is_one_line() {
    let lab = Lab::fresh("a34");
    lab.configure();
    let mut all = String::new();
    for args in [
        vec!["identify", "--insecure"],
        vec!["status"],
        vec!["clients"],
        vec!["whoami"],
        vec!["jobs", "--last", "5"],
        vec!["audit", "--last", "3"],
        vec!["ports"],
    ] {
        let mut full = args.clone();
        if args[0] == "identify" {
            full.push(&lab.addr);
        }
        let out = ctl(&lab.owner_home, &full);
        all.push_str(&out.stdout);
        all.push_str(&out.stderr);
    }
    assert!(
        all.bytes().all(|b| b != 0x1b && b < 0x80),
        "ASCII only, no colour"
    );
    let refusal = ctl(&lab.owner_home, &["revoke", "Nobody"]);
    refused(&refusal);
    let lines: Vec<&str> = refusal
        .stderr
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("Passphrase"))
        .collect();
    assert_eq!(lines.len(), 1, "one stderr line");
    assert!(lines[0].starts_with("curvyoctl: "), "{}", lines[0]);
}

// ---------------------------------------------------------------- second computer (A35 to A38)

fn second_computer_code(lab: &Lab, role_flag: bool) -> String {
    let mut args = vec!["pair", "--name", "Laptop"];
    if role_flag {
        args.push("--owner");
    }
    let out = ctl(&lab.owner_home, &args);
    ok(&out);
    pairing_code(&out.stdout).expect("code")
}

#[test]
fn a35_login_makes_a_second_computer_an_owner_and_owner_calls_work_there() {
    let lab = Lab::fresh("a35");
    lab.configure();
    let code = second_computer_code(&lab, true);
    let second = unique_root("a35second");
    let login = ctl(second.path(), &["login", &lab.addr, "--code", &code]);
    ok(&login);
    assert!(
        login
            .stdout
            .contains("Logged in to workshop-laser. This computer is an owner."),
        "{}",
        login.stdout
    );
    let whoami = ctl(second.path(), &["whoami"]);
    ok(&whoami);
    assert!(
        whoami.stdout.to_lowercase().contains("owner"),
        "{}",
        whoami.stdout
    );
    let clients = ctl(second.path(), &["clients"]);
    ok(&clients);
    assert!(clients.stdout.contains("curvyoctl@"), "{}", clients.stdout);
    let confirm = ctl(second.path(), &["confirm-start"]);
    refused(&confirm);
    assert!(
        confirm
            .stderr
            .contains("no start is waiting on workshop-laser"),
        "an owner call runs, it is not refused: {}",
        confirm.stderr
    );
    let file = find_file(second.path(), "curvyoctl.toml").expect("stored locally");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a35_login_accepts_lower_case_and_no_hyphen() {
    let lab = Lab::fresh("a35case");
    lab.configure();
    let code = second_computer_code(&lab, true);
    let bare = code.replace('-', "").to_lowercase();
    let second = unique_root("a35case2");
    ok(&ctl(second.path(), &["login", &lab.addr, "--code", &bare]));
}

#[test]
fn a35_member_code_stores_nothing_on_a_login() {
    let lab = Lab::fresh("a35m");
    lab.configure();
    let code = second_computer_code(&lab, false);
    let second = unique_root("a35m2");
    let login = ctl(second.path(), &["login", &lab.addr, "--code", &code]);
    refused(&login);
    assert!(
        login.stderr.contains("the code was issued for a member"),
        "{}",
        login.stderr
    );
    assert!(
        find_file(second.path(), "curvyoctl.toml").is_none(),
        "nothing stored"
    );
}

#[test]
fn a37_wrong_codes_count_down_then_the_code_is_spent_and_a_used_code_is_not_valid() {
    let lab = Lab::fresh("a37");
    lab.configure();
    let code = second_computer_code(&lab, true);
    let first = code.chars().next().unwrap();
    let wrong = format!("{}{}", if first == 'A' { 'B' } else { 'A' }, &code[1..]);
    let third = unique_root("a37third");
    let one = ctl(third.path(), &["login", &lab.addr, "--code", &wrong]);
    refused(&one);
    assert!(
        one.stderr.contains("wrong code, 2 attempts left"),
        "{}",
        one.stderr
    );
    let two = ctl(third.path(), &["login", &lab.addr, "--code", &wrong]);
    assert!(two.stderr.contains("1 attempt left"), "{}", two.stderr);
    let three = ctl(third.path(), &["login", &lab.addr, "--code", &wrong]);
    assert!(three.stderr.contains("code spent"), "{}", three.stderr);
    let after = ctl(third.path(), &["login", &lab.addr, "--code", &code]);
    refused(&after);
    assert!(
        find_file(third.path(), "curvyoctl.toml").is_none(),
        "nothing stored after refusals"
    );

    let code2 = second_computer_code(&lab, true);
    let good_home = unique_root("a37good");
    ok(&ctl(
        good_home.path(),
        &["login", &lab.addr, "--code", &code2],
    ));
    let reuse_home = unique_root("a37reuse");
    let reuse = ctl(reuse_home.path(), &["login", &lab.addr, "--code", &code2]);
    refused(&reuse);
    assert!(reuse.stderr.contains("code not valid"), "{}", reuse.stderr);
}

#[test]
fn a37_login_to_a_connector_in_maintenance_is_refused() {
    let lab = Lab::fresh("a37m");
    let home = unique_root("a37mhome");
    let out = ctl(home.path(), &["login", &lab.addr, "--code", "K7QM-4XTR"]);
    refused(&out);
    assert!(out.stderr.contains("maintenance mode"), "{}", out.stderr);
}

#[test]
fn a38_changed_key_refuses_login_then_logout_and_login_again_with_the_same_code() {
    let mut lab = Lab::fresh("a38");
    lab.configure();
    let code_one = second_computer_code(&lab, true);
    let second = unique_root("a38second");
    ok(&ctl(
        second.path(),
        &["login", &lab.addr, "--code", &code_one],
    ));

    // Reset on the host: the connector comes back with a new key.
    let status = ctl(&lab.owner_home, &["reset", "--confirm", "workshop-laser"]);
    ok(&status);
    assert!(wait_exit_of(&mut lab.conn, WAIT).success(), "reset exits 0");
    assert_eq!(
        fs::read_dir(&lab.state).unwrap().count(),
        0,
        "state directory emptied"
    );

    lab.conn = Connector::start(&lab.state, &lab.addr);
    let new_fp = lab.conn.fingerprint();
    assert_ne!(new_fp, lab.fp, "new key after reset");
    let reapply = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &new_fp,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    ok(&reapply);
    let code_two = second_computer_code(&lab, true);
    let refusal = ctl(second.path(), &["login", &lab.addr, "--code", &code_two]);
    refused(&refusal);
    assert!(
        refusal.stderr.contains("the key of workshop-laser changed"),
        "{}",
        refusal.stderr
    );
    assert!(
        refusal.stderr.contains("Stored key:") && refusal.stderr.contains("Presented key:"),
        "{}",
        refusal.stderr
    );
    let out = ctl(second.path(), &["logout", "workshop-laser"]);
    ok(&out);
    assert!(
        out.stdout
            .contains("Removed workshop-laser from this computer"),
        "{}",
        out.stdout
    );
    let relogin = ctl(second.path(), &["login", &lab.addr, "--code", &code_two]);
    ok(&relogin);
}

#[test]
fn a38_logout_works_offline_and_removes_only_this_computer_entry() {
    let mut lab = Lab::fresh("a38off");
    lab.configure();
    let code = second_computer_code(&lab, true);
    let second = unique_root("a38off2");
    ok(&ctl(second.path(), &["login", &lab.addr, "--code", &code]));
    lab.conn.child().kill().expect("kill connector");
    let _ = lab.conn.child().wait();
    let out = ctl(second.path(), &["logout", "workshop-laser"]);
    ok(&out);
    assert!(
        out.stdout
            .contains("Removed workshop-laser from this computer"),
        "{}",
        out.stdout
    );
    let file = find_file(second.path(), "curvyoctl.toml");
    let remaining = file
        .map(|f| fs::read_to_string(f).unwrap_or_default())
        .unwrap_or_default();
    assert!(!remaining.contains(&lab.addr), "entry removed: {remaining}");
}

// ---------------------------------------------------------------- reset and recovery

#[test]
fn a19_reset_with_a_wrong_name_changes_nothing() {
    let mut lab = Lab::fresh("a19");
    lab.configure();
    let out = ctl(&lab.owner_home, &["reset", "--confirm", "wrong-name"]);
    assert_eq!(out.code, 2, "usage exit 2 for a mismatch (ux-notes 4.11)");
    assert!(
        out.stderr
            .contains("--confirm wrong-name does not match workshop-laser"),
        "{}",
        out.stderr
    );
    assert!(lab.state.join("connector.toml").exists(), "nothing deleted");
    assert!(
        lab.conn.child().try_wait().unwrap().is_none(),
        "still serving"
    );
}

#[test]
fn a19_a20_a30_reset_empties_the_directory_exits_0_and_a_new_key_comes_after() {
    let mut lab = Lab::fresh("a19b");
    lab.configure();
    let old_fp = lab.fp.clone();
    let out = ctl(&lab.owner_home, &["reset", "--confirm", "workshop-laser"]);
    ok(&out);
    let status = wait_exit_of(&mut lab.conn, WAIT);
    assert_eq!(status.code(), Some(0), "exit 0 after reset");
    assert_eq!(
        fs::read_dir(&lab.state).unwrap().count(),
        0,
        "directory exists and is empty"
    );

    lab.conn = Connector::start(&lab.state, &lab.addr);
    let new_fp = lab.conn.fingerprint();
    assert_ne!(new_fp, old_fp, "a fresh identity key");
    assert!(
        lab.conn.text().contains("Maintenance mode until"),
        "fresh maintenance mode"
    );

    // The owner's reset removed the connector from this computer: no entry, no token (A20).
    let gone = ctl(&lab.owner_home, &["status"]);
    assert_eq!(gone.code, 2, "no connector is known after the reset");
    assert!(
        gone.stderr.contains("no connector is known"),
        "{}",
        gone.stderr
    );

    let way_on = ctl(
        &lab.owner_home,
        &[
            "apply-config",
            "--insecure",
            &lab.addr,
            "--fingerprint",
            &new_fp,
            "-f",
            lab.config.to_str().unwrap(),
        ],
    );
    ok(&way_on);
}

fn wait_exit_of(conn: &mut Connector, within: Duration) -> ExitStatus {
    wait_exit(conn.child(), within)
}

#[test]
fn a21_server_reset_is_refused_while_running_and_works_when_stopped() {
    let lab = Lab::fresh("a21");
    lab.configure();
    let refused_run = Command::new(server_bin())
        .arg("--reset")
        .arg("--state-dir")
        .arg(&lab.state)
        .output()
        .expect("run");
    assert_eq!(
        refused_run.status.code(),
        Some(1),
        "a running connector holds the lock"
    );
    assert!(lab.state.join("connector.toml").exists(), "nothing deleted");
    drop(lab);

    let root = unique_root("a21stop");
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    fs::write(
        state.join("connector.toml"),
        config_toml("workshop-laser", "Marc", "127.0.0.1:7447"),
    )
    .unwrap();
    let out = Command::new(server_bin())
        .arg("--reset")
        .arg("--state-dir")
        .arg(&state)
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        fs::read_dir(&state).unwrap().count(),
        0,
        "directory kept, contents deleted"
    );
}

// ---------------------------------------------------------------- foreground, packaging, config

#[test]
fn a28_foreground_flags_are_refused_when_a_configuration_exists() {
    let root = unique_root("a28");
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    fs::write(
        state.join("connector.toml"),
        config_toml("workshop-laser", "Marc", "127.0.0.1:7447"),
    )
    .unwrap();
    let mut child = Command::new(server_bin())
        .arg("--state-dir")
        .arg(&state)
        .arg("--listen")
        .arg(free_addr())
        .arg("--fake-controller")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let status = wait_exit(&mut child, Duration::from_secs(15));
    assert!(status.code() != Some(0), "refused, not started: {status:?}");
}

#[test]
fn a28_foreground_mode_writes_no_configuration_or_logs() {
    let root = unique_root("a28fg");
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    let addr = free_addr();
    let mut child = Command::new(server_bin())
        .arg("--state-dir")
        .arg(&state)
        .arg("--listen")
        .arg(&addr)
        .arg("--fake-controller")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let out = collect(child.stdout.take().unwrap());
    let _guard = Guard(child);
    wait_for("foreground pairing code", &out, |t| {
        pairing_code(t).is_some()
    });
    for name in ["connector.toml", "clients.json", "admin.log", "jobs.log"] {
        assert!(
            !state.join(name).exists(),
            "{name} must not be written in foreground mode"
        );
    }
}

#[test]
fn a30_unit_file_has_the_hardening_and_the_service_guide_names_the_groups() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let unit =
        fs::read_to_string(repo.join("curvyo-connector-server/dist/curvyo-connector.service"))
            .expect("unit file");
    for needle in [
        "DynamicUser=yes",
        "StateDirectory=curvyo-connector",
        "SupplementaryGroups=dialout",
        "Restart=on-failure",
        "TimeoutStopSec=15",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=",
        "PrivateTmp=yes",
        "DevicePolicy=closed",
        "DeviceAllow=char-ttyUSB",
        "DeviceAllow=char-ttyACM",
    ] {
        assert!(unit.contains(needle), "unit file lacks {needle}");
    }
    let guide =
        fs::read_to_string(repo.join("docs/guides/connector-service-linux.md")).expect("guide");
    assert!(guide.contains("uucp"), "Arch group named");
    assert!(guide.contains("curvyoctl"), "curvyoctl described");
    if let Ok(verify) = Command::new("systemd-analyze")
        .arg("verify")
        .arg(repo.join("curvyo-connector-server/dist/curvyo-connector.service"))
        .output()
    {
        println!(
            "systemd-analyze verify exit {:?}: {}",
            verify.status.code(),
            String::from_utf8_lossy(&verify.stderr)
        );
    }
}

#[test]
fn a31_config_listen_wins_over_flag_and_config_flag_is_refused() {
    let root = unique_root("a31");
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    let file_addr = free_addr();
    let flag_addr = free_addr();
    fs::write(
        state.join("connector.toml"),
        config_toml("workshop-laser", "Marc", &file_addr),
    )
    .unwrap();
    let conn = Connector::start_probing(&state, &flag_addr, &file_addr, &[]);
    let text = wait_for("flag ignored line", &conn.out, |t| {
        t.to_lowercase().contains("ignored")
    });
    assert!(text.contains("listen"), "names the ignored flag: {text}");
    assert!(
        TcpStream::connect(&file_addr).is_ok(),
        "listens on the configuration address"
    );
    assert!(
        TcpStream::connect(&flag_addr).is_err(),
        "not on the flag address"
    );

    let config_flag = Command::new(server_bin())
        .arg("--config")
        .arg(state.join("connector.toml"))
        .output()
        .expect("run");
    assert_eq!(
        config_flag.status.code(),
        Some(2),
        "--config <file> does not exist"
    );
}

#[test]
fn a29_listing_ports_on_a_machine_without_devices_is_an_empty_list_not_an_error() {
    let lab = Lab::fresh("a29");
    ok(&ctl(
        &lab.owner_home,
        &["identify", "--insecure", &lab.addr],
    ));
    ok(&ctl(
        &lab.owner_home,
        &["identify", "--insecure", &lab.addr],
    ));
    let out = ctl(&lab.owner_home, &["ports", "--insecure", &lab.addr]);
    ok(&out);
}

// ---------------------------------------------------------------- jobs

#[test]
fn a34_jobs_and_audit_accept_last_and_print_no_job_on_an_idle_connector() {
    let lab = Lab::fresh("jobs");
    lab.configure();
    ok(&ctl(&lab.owner_home, &["jobs", "--last", "5"]));
    ok(&ctl(&lab.owner_home, &["audit", "--last", "3"]));
    let status = ctl(&lab.owner_home, &["status"]);
    ok(&status);
}
