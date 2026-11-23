//! Independent acceptance tests for `specs/0118-machine-control` first slice (criteria 3 to 6),
//! written from the specification and the UX notes before reading the implementation. They run
//! the REAL connector binary against my OWN minimal GRBL 1.1 simulator on a real pseudo-terminal
//! (real time, no stepping), and against `--fake-controller`. The simulator is deliberately
//! independent of `tests/common/grbl_sim` (the harness is the tester's from `0127`, extended with
//! homing failures, a silent homing and the Check/Sleep words).
#![cfg(target_os = "linux")]
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    dead_code,
    missing_docs
)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, connect, open_paired};
use curvyo_connector_io::job_send::SendError;
use curvyo_connector_io::pin_verifier::PinMode;
use serialport::{SerialPort as _, TTYPort};

// ---------------------------------------------------------------------------------------------
// My own GRBL 1.1 simulator
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Cfg {
    /// `$32`; `None` leaves it out of `$$`.
    laser_mode: Option<bool>,
    startup_alarm: bool,
    hold1_ms: u64,
    banner_delay_ms: u64,
    /// Never print a banner after a reset that interrupts a job (the identify reset still answers).
    no_banner_after_reset: bool,
    /// GRBL build that leaves the laser on in a hold (reports `A:S`).
    laser_on_in_hold: bool,
    ignore_bang: bool,
    /// Planner time one motion line adds.
    motion_ms: u64,
    homing_ms: u64,
    /// `$10=1` style: report MPos with WCO.
    mpos_wco: Option<(f64, f64)>,
    /// Fail the n-th (1-based) G-code line received since the last reset with error:n.
    fail_gcode_line: Option<(usize, u32)>,
    garbage_banner: bool,
    silent_from_start: bool,
    /// How many ok per 1 ms tick: cycles through this list.
    ok_cycle: Vec<usize>,
    /// Report Door instead of Idle.
    door: bool,
    /// What `$H` ends with instead of `ok` ("error:5", "ALARM:9", "ALARM:8").
    home_reply: Option<&'static str>,
    /// Do not answer `?` while homing (a silent controller).
    silent_homing: bool,
    /// State word reported instead of Idle ("Check", "Sleep").
    word: Option<&'static str>,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            laser_mode: Some(true),
            startup_alarm: false,
            hold1_ms: 300,
            banner_delay_ms: 50,
            no_banner_after_reset: false,
            laser_on_in_hold: false,
            ignore_bang: false,
            motion_ms: 40,
            homing_ms: 600,
            mpos_wco: None,
            fail_gcode_line: None,
            garbage_banner: false,
            silent_from_start: false,
            ok_cycle: vec![1, 0, 2, 1, 3, 0, 1],
            door: false,
            home_reply: None,
            silent_homing: false,
            word: None,
        }
    }
}

#[derive(Default)]
struct Sim {
    cfg: Cfg,
    partial: Vec<u8>,
    queue: VecDeque<Vec<u8>>,
    buffered: usize,
    max_buffered: usize,
    overflow: bool,
    /// Every complete line received (text without newline), with time.
    lines: Vec<(Instant, String)>,
    /// Lines at the time they were completely received.
    rx_lines: Vec<(Instant, String)>,
    /// Real-time bytes in order: (time, byte, partial-line-pending).
    rt: Vec<(Instant, u8, bool)>,
    /// Every byte received in order (including real-time ones), for byte-level checks.
    raw: Vec<(Instant, u8)>,
    out: Vec<(Instant, Vec<u8>)>,
    alarm: Option<u32>,
    alarm_lock: bool,
    hold_since: Option<Instant>,
    stop_override: bool,
    spindle_on: bool,
    air_on: bool,
    s_value: f64,
    busy_until: Option<Instant>,
    homing_until: Option<Instant>,
    gcode_count: usize,
    tick_n: usize,
    mute: bool,
    last_answer: Option<Instant>,
    /// Times a status report was answered, with its state word.
    reports: Vec<(Instant, String)>,
    last_ok_at: Option<Instant>,
    banners: Vec<Instant>,
    error_sent_at: Option<Instant>,
    unlocks: usize,
    pos: (f64, f64),
    laser_on_when_hold0: Vec<Instant>,
    closed: bool,
    pending_off: Option<Instant>,
    banner_flushed: Vec<Instant>,
    /// GRBL reads no serial line before its banner after a reset.
    ready_at: Option<Instant>,
}

fn realtime(b: u8) -> bool {
    matches!(b, b'?' | b'!' | b'~' | 0x18 | 0x9E)
}

impl Sim {
    fn new(cfg: Cfg) -> Self {
        let mut s = Sim {
            mute: cfg.silent_from_start,
            alarm_lock: cfg.startup_alarm,
            alarm: None,
            cfg,
            ..Sim::default()
        };
        if s.cfg.startup_alarm {
            s.alarm = Some(0);
        }
        s
    }

    fn put(&mut self, at: Instant, text: &str) {
        if !self.mute {
            self.out.push((at, text.as_bytes().to_vec()));
        }
    }

    fn banner_text(&self) -> String {
        if self.cfg.garbage_banner {
            "\r\nHello from a modem, AT&F OK\r\n".to_owned()
        } else {
            "\r\nGrbl 1.1h ['$' for help]\r\n".to_owned()
        }
    }

    fn laser_out(&self, now: Instant) -> bool {
        if !self.spindle_on || self.s_value <= 0.0 || self.alarm.is_some() {
            return false;
        }
        if self.in_hold(now) {
            return self.cfg.laser_on_in_hold && !self.stop_override;
        }
        true
    }

    fn in_hold(&self, _now: Instant) -> bool {
        self.hold_since.is_some()
    }

    fn state_word(&self, now: Instant) -> String {
        if self.homing_until.is_some_and(|u| u > now) {
            return "Home".into();
        }
        if self.alarm.is_some() {
            return "Alarm".into();
        }
        if let Some(h) = self.hold_since {
            return if now.duration_since(h) >= Duration::from_millis(self.cfg.hold1_ms) {
                "Hold:0".into()
            } else {
                "Hold:1".into()
            };
        }
        if self.cfg.door {
            return "Door".into();
        }
        if let Some(w) = self.cfg.word {
            return w.into();
        }
        if self.busy_until.is_some_and(|u| u > now) {
            return "Run".into();
        }
        "Idle".into()
    }

    fn report(&self, now: Instant) -> String {
        let w = self.state_word(now);
        let (x, y) = self.pos;
        let mut s = format!("<{w}|MPos:{x:.3},{y:.3},0.000|FS:0,0");
        if let Some((ox, oy)) = self.cfg.mpos_wco {
            s += &format!("|WCO:{ox:.3},{oy:.3}");
        }
        if w.starts_with("Hold")
            && self.cfg.laser_on_in_hold
            && self.spindle_on
            && !self.stop_override
        {
            s += "|A:S";
        } else if !w.starts_with("Hold") && self.spindle_on {
            s += "|A:S";
        }
        s + ">\r\n"
    }

    fn do_reset(&mut self, now: Instant) {
        let moving = self.busy_until.is_some_and(|u| u > now) && self.hold_since.is_none();
        let homing = self.homing_until.is_some_and(|u| u > now);
        let mid_job = self.busy_until.is_some() || self.hold_since.is_some();
        self.queue.clear();
        self.buffered = 0;
        self.partial.clear();
        self.spindle_on = false;
        self.air_on = false;
        self.s_value = 0.0;
        self.hold_since = None;
        self.stop_override = false;
        self.busy_until = None;
        self.homing_until = None;
        self.pending_off = None;
        self.gcode_count = 0;
        let at = now + Duration::from_millis(self.cfg.banner_delay_ms);
        if !(self.cfg.no_banner_after_reset && mid_job) {
            let b = self.banner_text();
            self.banners.push(at);
            self.put(at, &b);
        }
        if self.alarm_lock && !moving && !homing {
            // GRBL with homing on: locked after every reset.
            self.alarm = Some(0);
        }
        if moving {
            self.alarm = Some(3);
            self.put(at, "ALARM:3\r\n[MSG:Reset while in motion]\r\n");
        } else if homing {
            self.alarm = Some(6);
            self.put(at, "ALARM:6\r\n");
        } else if self.alarm.is_some() && self.alarm != Some(0) {
            // keeps the numbered alarm locked
        }
        if self.alarm == Some(0) {
            self.put(at, "[MSG:'$H'|'$X' to unlock]\r\n");
        }
    }

    fn feed(&mut self, bytes: &[u8], now: Instant) {
        if self.mute || self.closed {
            return;
        }
        for &b in bytes {
            self.raw.push((now, b));
            if realtime(b) {
                self.rt.push((now, b, !self.partial.is_empty()));
                match b {
                    b'?' if self.cfg.silent_homing
                        && self.homing_until.is_some_and(|u| u > now) => {}
                    b'?' => {
                        let r = self.report(now);
                        self.reports.push((now, self.state_word(now)));
                        self.last_answer = Some(now);
                        // One serial stream: a report never overtakes the banner or the
                        // `ALARM:n` line a reset has queued but not yet written.
                        let at = self.out.iter().map(|o| o.0).fold(now, Instant::max);
                        self.put(at, &r);
                    }
                    b'!' => {
                        if !self.cfg.ignore_bang
                            && self.homing_until.is_none_or(|u| u <= now)
                            && self.hold_since.is_none()
                        {
                            self.hold_since = Some(now);
                        }
                    }
                    b'~' => {
                        if self.hold_since.is_some() {
                            self.hold_since = None;
                            self.stop_override = false;
                        }
                    }
                    0x9E => {
                        if self.hold_since.is_some() {
                            self.stop_override = !self.stop_override;
                        }
                    }
                    _ => self.do_reset(now),
                }
                continue;
            }
            self.partial.push(b);
            self.buffered += 1;
            if self.buffered > 128 {
                self.overflow = true;
            }
            self.max_buffered = self.max_buffered.max(self.buffered);
            if b == b'\n' {
                let line = std::mem::take(&mut self.partial);
                self.rx_lines
                    .push((now, String::from_utf8_lossy(&line).trim().to_owned()));
                self.queue.push_back(line);
            }
        }
    }

    fn tick(&mut self, now: Instant) {
        if self.mute || self.closed {
            return;
        }
        if let Some(u) = self.pending_off {
            if u <= now {
                self.spindle_on = false;
                self.pending_off = None;
            }
        }
        // Homing finished?
        if let Some(u) = self.homing_until {
            if u > now {
                return; // `$H` blocks the parser
            }
            self.homing_until = None;
            match self.cfg.home_reply {
                Some(a) if a.starts_with("ALARM:") => {
                    self.alarm = Some(a[6..].parse().unwrap());
                    self.put(now, &format!("{a}\r\n"));
                }
                _ => {
                    self.pos = (0.0, 0.0);
                    self.put(now, "ok\r\n");
                    self.last_ok_at = Some(now);
                    self.alarm = None;
                    self.alarm_lock = false;
                }
            }
        }
        let quota = self.cfg.ok_cycle[self.tick_n % self.cfg.ok_cycle.len()];
        self.tick_n += 1;
        for _ in 0..quota {
            let Some(line) = self.queue.pop_front() else {
                break;
            };
            self.buffered -= line.len();
            let text = String::from_utf8_lossy(&line).trim().to_owned();
            self.lines.push((now, text.clone()));
            self.handle_line(&text, now);
            if self.homing_until.is_some() {
                break;
            }
        }
    }

    fn handle_line(&mut self, text: &str, now: Instant) {
        if text.is_empty() {
            self.ack(now, "ok");
            return;
        }
        let up = text.to_ascii_uppercase();
        if up == "$I" {
            self.put(now, "[VER:1.1h.20190825:]\r\n[OPT:V,15,128]\r\n");
            self.ack(now, "ok");
            return;
        }
        if up == "$$" {
            let mut s = String::from("$0=10\r\n$10=1\r\n$22=1\r\n$30=1000\r\n");
            if let Some(l) = self.cfg.laser_mode {
                s += &format!("$32={}\r\n", u8::from(l));
            }
            s += "$130=400.000\r\n$131=300.000\r\n";
            self.put(now, &s);
            self.ack(now, "ok");
            return;
        }
        if up == "$X" {
            self.unlocks += 1;
            self.put(now, "[MSG:Caution: Unlocked]\r\n");
            self.alarm = None;
            self.alarm_lock = false;
            self.ack(now, "ok");
            return;
        }
        if up == "$H" {
            if self.cfg.home_reply.is_some_and(|r| r.starts_with("error:")) {
                self.ack(now, self.cfg.home_reply.unwrap());
                return;
            }
            self.homing_until = Some(now + Duration::from_millis(self.cfg.homing_ms));
            return;
        }
        if up.starts_with('$') {
            self.ack(now, "ok");
            return;
        }
        if self.alarm.is_some() {
            self.ack(now, "error:9");
            return;
        }
        self.gcode_count += 1;
        if let Some((n, code)) = self.cfg.fail_gcode_line {
            if self.gcode_count == n {
                self.error_sent_at = Some(now);
                self.ack(now, &format!("error:{code}"));
                return;
            }
        }
        if up.contains("M3") || up.contains("M4") {
            self.spindle_on = true;
        }
        if up.contains("M5") {
            // synchronised with the planner: the spindle goes off when earlier motion is done
            match self.busy_until.filter(|u| *u > now) {
                Some(u) => self.pending_off = Some(u),
                None => self.spindle_on = false,
            }
        }
        if up.contains("M8") {
            self.air_on = true;
        }
        if up.contains("M9") {
            self.air_on = false;
        }
        if let Some(i) = up.find('S') {
            let n: String = up[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = n.parse::<f64>() {
                self.s_value = v;
            }
        }
        if up.contains("G0") || up.contains("G1") || up.contains("G2") || up.contains("G3") {
            if up.contains('X') || up.contains('Y') {
                let base = self.busy_until.filter(|u| *u > now).unwrap_or(now);
                self.busy_until = Some(base + Duration::from_millis(self.cfg.motion_ms));
            }
            let pick = |c: char| -> Option<f64> {
                let i = up.find(c)?;
                up[i + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect::<String>()
                    .parse()
                    .ok()
            };
            if let Some(x) = pick('X') {
                self.pos.0 = x;
            }
            if let Some(y) = pick('Y') {
                self.pos.1 = y;
            }
        }
        self.ack(now, "ok");
    }

    fn ack(&mut self, now: Instant, word: &str) {
        self.put(now, &format!("{word}\r\n"));
        self.last_ok_at = Some(now);
    }

    fn flush(&mut self, now: Instant) -> Vec<u8> {
        let mut out = Vec::new();
        self.out.retain(|(at, bytes)| {
            if *at <= now {
                if bytes.starts_with(b"\r\nGrbl") {
                    self.banner_flushed.push(now);
                }
                out.extend_from_slice(bytes);
                false
            } else {
                true
            }
        });
        out
    }
}

struct Pty {
    sim: Arc<Mutex<Sim>>,
    link: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    dir: PathBuf,
    n: usize,
}

fn drive(mut master: TTYPort, sim: Arc<Mutex<Sim>>, stop: Arc<AtomicBool>) {
    master.set_timeout(Duration::from_millis(1)).unwrap();
    let mut buf = [0_u8; 1024];
    // The banner a GRBL prints at power-on.
    {
        let mut s = sim.lock().unwrap();
        let b = s.banner_text();
        let at = Instant::now() + Duration::from_millis(20);
        if !s.cfg.silent_from_start {
            s.out.push((at, b.into_bytes()));
            if s.alarm_lock {
                s.out.push((at, b"[MSG:'$H'|'$X' to unlock]\r\n".to_vec()));
            }
        }
    }
    while !stop.load(Ordering::Relaxed) {
        let n = master.read(&mut buf).unwrap_or(0);
        let now = Instant::now();
        let out = {
            let mut s = sim.lock().unwrap();
            if n > 0 {
                s.feed(&buf[..n], now);
            }
            s.tick(now);
            s.flush(now)
        };
        if !out.is_empty() {
            let _ = master.write_all(&out);
        }
    }
}

impl Pty {
    fn new(cfg: Cfg, dir: &Path) -> Self {
        let mut p = Pty {
            sim: Arc::new(Mutex::new(Sim::new(cfg))),
            link: dir.join("ttyGRBL"),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            dir: dir.to_owned(),
            n: 0,
        };
        p.attach();
        p
    }

    fn attach(&mut self) {
        let (master, slave) = TTYPort::pair().unwrap();
        let name = slave.name().unwrap();
        drop(slave);
        let tmp = self.dir.join(format!("l{}", self.n));
        self.n += 1;
        let _ = std::fs::remove_file(&tmp);
        std::os::unix::fs::symlink(&name, &tmp).unwrap();
        std::fs::rename(&tmp, &self.link).unwrap();
        self.stop = Arc::new(AtomicBool::new(false));
        let (sim, stop) = (self.sim.clone(), self.stop.clone());
        self.thread = Some(std::thread::spawn(move || drive(master, sim, stop)));
    }

    /// The cable is pulled: the master side closes.
    fn unplug(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
        let _ = std::fs::remove_file(&self.link);
    }

    /// A fresh machine on a new pty (replug); previous logs are kept, state is reset.
    fn replug(&mut self, cfg: Cfg) {
        {
            let mut s = self.sim.lock().unwrap();
            let lines = std::mem::take(&mut s.lines);
            let raw = std::mem::take(&mut s.raw);
            *s = Sim::new(cfg);
            s.lines = lines;
            s.raw = raw;
        }
        self.attach();
    }

    fn path(&self) -> String {
        self.link.to_string_lossy().into_owned()
    }

    fn with<R>(&self, f: impl FnOnce(&mut Sim) -> R) -> R {
        f(&mut self.sim.lock().unwrap())
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The connector process and a client
// ---------------------------------------------------------------------------------------------

const WAIT: Duration = Duration::from_secs(25);

fn bin() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("curvyo-connector-server")
}

struct Rig {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
    address: String,
    code: String,
    dir: tempfile::TempDir,
    pty: Pty,
    conn: Option<Connection>,
    /// Every message received, with arrival time.
    history: Vec<(Instant, Message)>,
    welcome: Option<Message>,
    started_at: Instant,
    first_code_used: bool,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Rig {
    fn start(cfg: Cfg) -> Rig {
        Self::start_with(cfg, &[])
    }

    fn start_with(cfg: Cfg, extra: &[&str]) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let ptydir = dir.path().join("pty");
        std::fs::create_dir(&ptydir).unwrap();
        let pty = Pty::new(cfg, &ptydir);
        Self::launch(dir, pty, extra)
    }

    fn launch(dir: tempfile::TempDir, pty: Pty, extra: &[&str]) -> Rig {
        let mut cmd = Command::new(bin());
        cmd.args(["--listen", "127.0.0.1:0"]);
        if !extra.contains(&"--fake-controller") {
            cmd.arg("--serial").arg(pty.path());
        }
        let mut child = cmd
            .arg("--state-dir")
            .arg(dir.path().join("state"))
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (tx, lines) = channel();
        let out = child.stdout.take().unwrap();
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = tx2.send(l);
            }
        });
        let err = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            for l in BufReader::new(err).lines().map_while(Result::ok) {
                let _ = tx.send(format!("[stderr] {l}"));
            }
        });
        let mut rig = Rig {
            child,
            lines,
            seen: Vec::new(),
            address: String::new(),
            code: String::new(),
            dir,
            pty,
            conn: None,
            history: Vec::new(),
            welcome: None,
            started_at: Instant::now(),
            first_code_used: false,
        };
        while rig.code.is_empty() {
            let line = rig
                .lines
                .recv_timeout(WAIT)
                .expect("connector printed no pairing code");
            if let Some(a) = line.strip_prefix("Listening on ") {
                a.clone_into(&mut rig.address);
            }
            if let Some(c) = line.strip_prefix("Pairing code: ") {
                c.split(',').next().unwrap().clone_into(&mut rig.code);
            }
            rig.seen.push(line);
        }
        rig
    }

    /// Pairs a client and opens an authenticated connection. Returns (connection, Welcome-ish).
    fn pair_client(&mut self, name: &str) -> (Connection, curvyo_connector_io::client::Greeting) {
        let code = if self.first_code_used {
            std::thread::sleep(Duration::from_millis(300));
            while let Ok(l) = self.lines.try_recv() {
                self.seen.push(l);
            }
            self.child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(b"pair\n")
                .unwrap();
            let mut got = String::new();
            let quiet = Instant::now() + Duration::from_millis(800);
            while Instant::now() < quiet {
                if let Ok(l) = self.lines.recv_timeout(Duration::from_millis(100)) {
                    if let Some(c) = l.strip_prefix("Pairing code: ") {
                        got = c.split(',').next().unwrap().to_owned();
                    }
                    self.seen.push(l);
                }
            }
            assert!(!got.is_empty(), "no new pairing code: {:?}", self.seen);
            got
        } else {
            self.first_code_used = true;
            self.code_now()
        };
        let mut c = connect(&self.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
        let rec = c.pair(&code, name, Instant::now() + WAIT).unwrap();
        drop(c);
        let (conn, _status) =
            open_paired(&self.address, rec.spki, &rec.token, CONNECT_TIMEOUT).unwrap();
        let g = conn.greeting.clone();
        (conn, g)
    }

    fn code_now(&self) -> String {
        self.code.clone()
    }

    fn connect(&mut self) {
        let (c, g) = self.pair_client("Tester");
        self.welcome = Some(Message::Welcome {
            connector_name: g.name.clone(),
            controller: g.controller.clone(),
            mode: None,
        });
        self.conn = Some(c);
    }

    fn conn(&mut self) -> &mut Connection {
        self.conn.as_mut().unwrap()
    }

    /// Reads messages until `f` matches or `secs` pass; every message goes to `history`.
    fn wait(&mut self, secs: f64, f: impl Fn(&Message) -> bool) -> Option<(Instant, Message)> {
        let until = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            let m = match self.conn().try_recv(until) {
                Ok(Some(curvyo_connector_io::frames::Frame::Core(m))) => m,
                Ok(Some(_)) => continue,
                Ok(None) => return None,
                Err(e) => panic!("connection error: {e:?}"),
            };
            let now = Instant::now();
            self.history.push((now, m.clone()));
            if f(&m) {
                return Some((now, m));
            }
        }
    }

    fn wait_state(
        &mut self,
        secs: f64,
        pred: impl Fn(&State) -> bool,
    ) -> Option<(Instant, Message)> {
        self.wait(
            secs,
            |m| matches!(m, Message::Status { state, .. } if pred(state)),
        )
    }

    fn drain(&mut self, secs: f64) {
        let _ = self.wait(secs, |_| false);
    }

    fn send(&mut self, m: Message) {
        self.conn().send(&m).unwrap();
    }

    fn submit(&mut self, text: &str) -> Result<uuid::Uuid, SendError> {
        self.conn()
            .send_job("test.gcode", text, Instant::now() + WAIT)
            .map(|j| j.job_id)
    }

    fn start_job(&mut self, text: &str) -> uuid::Uuid {
        let id = self.submit(text).expect("job refused");
        self.send(Message::Start { job_id: id });
        id
    }

    fn last_status(&self) -> Option<&Message> {
        self.history
            .iter()
            .rev()
            .map(|(_, m)| m)
            .find(|m| matches!(m, Message::Status { .. }))
    }

    fn jobs_log(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("state").join("jobs.log")).unwrap_or_default()
    }

    /// Whether the terminal printed exactly `line` (UX notes section 10).
    fn seen_line(&mut self, line: &str) -> bool {
        // The connector prints on its own thread: poll briefly instead of one drain.
        let t = Instant::now();
        loop {
            let _ = self.stdout_has(line);
            if self.seen.iter().any(|l| l.trim_end() == line) {
                return true;
            }
            if t.elapsed() > Duration::from_secs(2) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn stdout_has(&mut self, needle: &str) -> bool {
        while let Ok(l) = self.lines.try_recv() {
            self.seen.push(l);
        }
        self.seen.iter().any(|l| l.contains(needle))
    }

    fn signal(&self, sig: &str) {
        let st = Command::new("kill")
            .arg(format!("-{sig}"))
            .arg(self.child.id().to_string())
            .status()
            .unwrap();
        assert!(st.success());
    }
}

fn run_bin(args: &[&str]) -> (i32, String, String) {
    let d = tempfile::tempdir().unwrap();
    let o = Command::new(bin())
        .args(args)
        .arg("--state-dir")
        .arg(d.path())
        .output()
        .unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn is_state(m: &Message, f: impl Fn(&State) -> bool) -> bool {
    matches!(m, Message::Status { state, .. } if f(state))
}

fn detail(m: &Message) -> String {
    match m {
        Message::Status { detail, .. } => detail.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn alarm_text(m: &Message) -> String {
    match m {
        Message::Status { alarm, .. } => alarm.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn lines_job(n: usize) -> String {
    let mut s = String::from("G21\nG90\nM4 S0\n");
    for i in 0..n {
        s += &format!("G1 X{}.000 Y{}.000 F6000 S500\n", i % 300, (i / 3) % 200);
    }
    s + "M5\n"
}

/// Expected lines on the wire: my own reading of G5.
fn prepare(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut s = String::new();
        let mut depth = 0;
        for c in raw.chars() {
            match c {
                ';' if depth == 0 => break,
                '(' => depth += 1,
                ')' if depth > 0 => depth -= 1,
                _ if depth > 0 => {}
                c if c.is_whitespace() => {}
                c => s.push(c),
            }
        }
        if !s.is_empty() {
            out.push(s);
        }
    }
    out
}

/// Job lines the simulator saw (all but the identify's `$I`/`$$` and the safety `M5`/`M9` that a
/// stop writes): everything after the first `$$`.
fn job_lines(sim: &Sim) -> Vec<String> {
    let pos = sim
        .lines
        .iter()
        .position(|(_, l)| l == "$$")
        .map_or(0, |p| p + 1);
    sim.lines[pos..].iter().map(|(_, l)| l.clone()).collect()
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-laser-core/tests/fixtures/output/grbl")
            .join(name),
    )
    .unwrap()
}

fn connected(cfg: Cfg) -> Rig {
    let mut r = Rig::start(cfg);
    r.connect();
    // wait for Idle after identify
    if !matches!(
        r.welcome,
        Some(Message::Welcome {
            controller: Some(_),
            ..
        })
    ) {
        r.wait_state(10.0, |s| matches!(s, State::Idle | State::Alarm));
    }
    r
}

fn idle_rig(cfg: Cfg) -> Rig {
    let mut r = Rig::start(cfg);
    // The connector may need a moment to identify; poll until Idle via fresh connections.
    r.connect();
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle)).is_some() {
            break;
        }
        assert!(t.elapsed() < WAIT, "never Idle: {:?}", r.history);
    }
    r
}

const CUT: &str = "G21\nG90\nM3 S500\nG1 X10 Y10 F600\nG1 X20 Y20\nM5\n";

fn start_fake(extra: &[&str]) -> Rig {
    let mut ex = vec!["--fake-controller"];
    ex.extend_from_slice(extra);
    let dir = tempfile::tempdir().unwrap();
    let ptydir = dir.path().join("pty");
    std::fs::create_dir(&ptydir).unwrap();
    let pty = Pty::new(Cfg::default(), &ptydir);
    Rig::launch(dir, pty, &ex)
}

// ---------------------------------------------------------------------------------------------
// 0118 helpers
// ---------------------------------------------------------------------------------------------

fn dollar_h(r: &Rig) -> usize {
    r.pty.with(|s| {
        s.lines
            .iter()
            .filter(|(_, l)| l.eq_ignore_ascii_case("$H"))
            .count()
    })
}

fn raw_dollar_h(r: &Rig) -> usize {
    r.pty.with(|s| {
        let b: Vec<u8> = s.raw.iter().map(|x| x.1).collect();
        b.windows(3).filter(|w| *w == b"$H\n").count()
    })
}

fn unlocks_sent(r: &Rig) -> usize {
    r.pty.with(|s| {
        s.lines
            .iter()
            .filter(|(_, l)| l.eq_ignore_ascii_case("$X"))
            .count()
    })
}

fn is_homing(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::Homing))
}

fn is_idle(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::Idle))
}

fn is_ready(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::Ready))
}

fn job_of(m: &Message) -> bool {
    matches!(m, Message::Status { job: Some(_), .. })
}

fn until_error(r: &mut Rig, secs: f64) -> Option<(curvyo_connector_core::ErrorCode, String)> {
    r.wait(secs, |m| matches!(m, Message::Error { .. }))
        .map(|(_, m)| match m {
            Message::Error { code, message } => (code, message),
            _ => unreachable!(),
        })
}

/// The connector shows Idle before identify has read `$$` and before the reset's banner has
/// arrived; a command written then is a race of the simulator's start, not of the story.
fn settle(r: &mut Rig) {
    let t = Instant::now();
    while !r.pty.with(|s| s.lines.iter().any(|(_, l)| l == "$$")) {
        assert!(t.elapsed() < WAIT, "identify never read the settings");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
}

/// A rig whose controller has finished identify and shows Idle.
fn idle(cfg: Cfg) -> Rig {
    let mut r = idle_rig(cfg);
    settle(&mut r);
    r
}

fn alarm_rig(cfg: Cfg) -> Rig {
    let mut r = Rig::start(Cfg {
        startup_alarm: true,
        ..cfg
    });
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(12.0, |s| matches!(s, State::Alarm))
        .expect("startup alarm");
    settle(&mut r);
    r
}

fn long_running(r: &mut Rig) {
    let mut job = String::from("G21\nG90\nM3 S500\n");
    for i in 0..400 {
        job += &format!("G1 X{} Y{} F600\n", i % 100, i % 50);
    }
    job += "M5\n";
    r.start_job(&job);
    r.wait_state(10.0, |s| matches!(s, State::Running))
        .expect("Running");
    // let it run a bit
    r.drain(0.6);
}

/// The simulator has taken `$H` off its queue (the cycle runs).
fn cycle_running(r: &Rig) {
    let t = Instant::now();
    while !r.pty.with(|s| s.homing_until.is_some()) {
        assert!(t.elapsed() < WAIT, "the controller never started homing");
        std::thread::sleep(Duration::from_millis(10));
    }
}

const LONG_HOME: u64 = 20_000;

// ---------------------------------------------------------------------------------------------
// Criterion 3: Home from Idle, Ready, Done, Alarm
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_home_from_idle_writes_dollar_h_once_reports_homing_then_idle() {
    let mut r = idle(Cfg {
        homing_ms: 800,
        ..Cfg::default()
    });
    assert_eq!(dollar_h(&r), 0, "connector homed on its own");
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("no Homing status");
    r.wait(10.0, is_idle).expect("no Idle after homing");
    assert_eq!(dollar_h(&r), 1, "$H must be written exactly once");
    assert_eq!(raw_dollar_h(&r), 1);
    assert_eq!(unlocks_sent(&r), 0);
    // Idle -> no Homing again, nothing else written
    r.drain(2.0);
    assert_eq!(dollar_h(&r), 1);
    // the position read after homing is the origin the simulator reports
    let (_, m) = r
        .wait(1.0, |_| false)
        .unwrap_or((Instant::now(), Message::GetStatus));
    let _ = m;
}

#[test]
fn c3_every_client_sees_homing_at_once() {
    let mut r = idle(Cfg {
        homing_ms: 1500,
        ..Cfg::default()
    });
    let (mut c2, _) = r.pair_client("Second");
    r.send(Message::Home);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw = false;
    while let Ok(Some(f)) = c2.try_recv(deadline) {
        if let curvyo_connector_io::frames::Frame::Core(m) = f {
            if is_homing(&m) {
                saw = true;
                break;
            }
        }
    }
    assert!(saw, "the second client never saw Homing");
}

#[test]
fn c3_connector_never_homes_on_its_own() {
    // normal idle, startup alarm, after unlock, after a job and a stop
    let mut r = idle(Cfg::default());
    r.drain(3.0);
    assert_eq!(dollar_h(&r), 0);
    r.start_job(CUT);
    r.wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .unwrap();
    r.drain(1.0);
    assert_eq!(dollar_h(&r), 0, "homed after a job");
    let mut a = alarm_rig(Cfg::default());
    a.drain(3.0);
    a.send(Message::Unlock);
    a.wait_state(8.0, |s| matches!(s, State::Idle)).unwrap();
    a.drain(2.0);
    assert_eq!(dollar_h(&a), 0, "homed on its own around alarm and unlock");
}

#[test]
fn c3_home_with_a_ready_job_keeps_it_and_start_works_afterwards() {
    let mut r = idle(Cfg {
        homing_ms: 800,
        ..Cfg::default()
    });
    let id = r.submit(CUT).unwrap();
    r.wait_state(5.0, |s| matches!(s, State::Ready))
        .expect("Ready");
    r.send(Message::Home);
    let (_, h) = r.wait(5.0, is_homing).expect("Homing");
    assert!(job_of(&h), "Homing with a held job carries it: {h:?}");
    r.wait(10.0, is_ready).expect("back to Ready, job kept");
    assert_eq!(dollar_h(&r), 1);
    r.send(Message::Start { job_id: id });
    let end = r
        .wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .expect("job ran");
    assert!(is_state(&end.1, |s| matches!(
        s,
        State::Done {
            result: JobResult::Completed
        }
    )));
    assert_eq!(
        r.pty
            .with(|s| job_lines(s))
            .iter()
            .filter(|l| l.as_str() != "$H")
            .count()
            >= prepare(CUT).len(),
        true
    );
    assert_eq!(dollar_h(&r), 1, "Start must not home again");
}

#[test]
fn c3_home_from_done_works_for_completed() {
    let mut r = idle(Cfg {
        homing_ms: 500,
        ..Cfg::default()
    });
    r.start_job(CUT);
    r.wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .unwrap();
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing from Done");
    r.wait(10.0, is_idle).expect("Idle");
    assert_eq!(dollar_h(&r), 1);
}

#[test]
fn c3_home_from_done_stopped_and_failed() {
    // Stopped
    let mut r = idle(Cfg {
        homing_ms: 500,
        ..Cfg::default()
    });
    long_running(&mut r);
    r.send(Message::Stop);
    r.wait_state(15.0, |s| {
        matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            } | State::Alarm
        )
    })
    .unwrap();
    if r.last_status()
        .is_some_and(|m| is_state(m, |s| matches!(s, State::Done { .. })))
    {
        r.send(Message::Home);
        r.wait(5.0, is_homing).expect("Homing from Done(Stopped)");
        r.wait(10.0, is_idle).expect("Idle");
    }
    // Failed
    let mut f = idle(Cfg {
        homing_ms: 500,
        fail_gcode_line: Some((2, 22)),
        ..Cfg::default()
    });
    f.start_job(CUT);
    f.wait_state(15.0, |s| {
        matches!(
            s,
            State::Done {
                result: JobResult::Failed
            }
        )
    })
    .expect("failed");
    f.send(Message::Home);
    f.wait(5.0, is_homing).expect("Homing from Done(Failed)");
    f.wait(10.0, is_idle).expect("Idle after failed+home");
}

#[test]
fn c3_home_in_alarm_sends_dollar_h_and_never_dollar_x() {
    let mut r = alarm_rig(Cfg {
        homing_ms: 800,
        ..Cfg::default()
    });
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing in Alarm");
    r.wait(10.0, is_idle).expect("Idle after Home in Alarm");
    assert_eq!(dollar_h(&r), 1);
    assert_eq!(unlocks_sent(&r), 0, "$X must not be sent for Home");
    assert_eq!(r.pty.with(|s| s.unlocks), 0);
}

// ---------------------------------------------------------------------------------------------
// Criterion 5 and the refusal table: WrongState and its cause, nothing written
// ---------------------------------------------------------------------------------------------

fn refused(r: &mut Rig, what: &str) {
    let before = dollar_h(r);
    r.send(Message::Home);
    let (code, message) = until_error(r, 5.0).unwrap_or_else(|| panic!("{what}: no refusal"));
    assert_eq!(
        code,
        curvyo_connector_core::ErrorCode::WrongState,
        "{what}: {message}"
    );
    assert!(
        !message.trim().is_empty(),
        "{what}: refusal carries no cause"
    );
    println!("REFUSAL {what}: {message}");
    r.drain(0.5);
    assert_eq!(dollar_h(r), before, "{what}: $H was written");
    assert!(
        !r.history.iter().rev().take(5).any(|(_, m)| is_homing(m)),
        "{what}: Homing reported"
    );
}

#[test]
fn c5_home_refused_while_running() {
    let mut r = idle(Cfg::default());
    long_running(&mut r);
    refused(&mut r, "running");
    // and the job was not disturbed
    r.send(Message::Stop);
    r.wait_state(15.0, |s| matches!(s, State::Done { .. } | State::Alarm))
        .unwrap();
}

#[test]
fn c5_home_refused_while_paused() {
    let mut r = idle(Cfg::default());
    long_running(&mut r);
    r.send(Message::Pause);
    r.wait_state(10.0, |s| matches!(s, State::Paused))
        .expect("Paused");
    refused(&mut r, "paused");
    r.send(Message::Stop);
    r.wait_state(15.0, |s| matches!(s, State::Done { .. } | State::Alarm))
        .unwrap();
}

#[test]
fn c5_home_refused_while_stopping() {
    let mut r = idle(Cfg {
        banner_delay_ms: 2500,
        ..Cfg::default()
    });
    long_running(&mut r);
    r.send(Message::Stop);
    refused(&mut r, "stopping");
    r.wait_state(20.0, |s| matches!(s, State::Done { .. } | State::Alarm))
        .unwrap();
}

#[test]
fn c5_home_refused_while_receiving() {
    use sha2::{Digest, Sha256};
    let mut r = idle(Cfg::default());
    let id = uuid::Uuid::from_u128(0x1180_0000_0000_4000_8000_0000_0000_0001);
    let text = CUT.to_owned();
    let hash: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    r.send(Message::SubmitJob {
        job_id: id,
        name: "t.gcode".into(),
        kind: curvyo_connector_core::JobKind::Job,
        bytes: text.len() as u64,
        sha256: hash,
    });
    r.send(Message::JobChunk {
        job_id: id,
        index: 0,
        text: text[..10].to_owned(),
    });
    r.wait_state(5.0, |s| matches!(s, State::Receiving))
        .expect("Receiving");
    refused(&mut r, "receiving");
}

#[test]
fn c5_home_refused_while_awaiting_local_start() {
    let mut r = Rig::start_with(Cfg::default(), &["--require-local-start"]);
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .expect("Idle");
    settle(&mut r);
    let id = r.submit(CUT).unwrap();
    r.send(Message::Start { job_id: id });
    r.wait_state(5.0, |s| matches!(s, State::AwaitingLocalStart))
        .expect("awaiting");
    refused(&mut r, "awaiting local start");
}

#[test]
fn c5_home_refused_without_controller() {
    let mut r = Rig::start(Cfg {
        silent_from_start: true,
        ..Cfg::default()
    });
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(15.0, |s| matches!(s, State::NoController))
        .expect("NoController");
    refused(&mut r, "no controller");
}

fn controller_word(cfg: Cfg, word: &str) -> Rig {
    let mut r = Rig::start(cfg);
    r.connect();
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait(1.0, |m| detail(m).contains(word)).is_some() {
            break;
        }
        assert!(
            t.elapsed() < WAIT,
            "never reported {word}: {:?}",
            r.last_status()
        );
    }
    r
}

#[test]
fn c5_home_refused_in_door_check_and_sleep() {
    let mut r = controller_word(
        Cfg {
            door: true,
            ..Cfg::default()
        },
        "Door",
    );
    refused(&mut r, "door");
    let mut r = controller_word(
        Cfg {
            word: Some("Check"),
            ..Cfg::default()
        },
        "Check",
    );
    refused(&mut r, "check");
    let mut r = controller_word(
        Cfg {
            word: Some("Sleep"),
            ..Cfg::default()
        },
        "Sleep",
    );
    refused(&mut r, "sleep");
}

// ---------------------------------------------------------------------------------------------
// Criterion 3: Stop during homing; criterion 4: failures
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_stop_during_homing_resets_first_then_alarm_6_and_ready_job_is_dropped_on_grbl() {
    let mut r = idle(Cfg {
        homing_ms: LONG_HOME,
        ..Cfg::default()
    });
    let id = r.submit(CUT).unwrap();
    r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing");
    cycle_running(&r);
    r.drain(0.6);
    let before = r.pty.with(|s| s.raw.len());
    r.send(Message::Stop);
    let (_, m) = r
        .wait_state(10.0, |s| matches!(s, State::Alarm))
        .expect("alarm 6 after the reset");
    assert!(
        alarm_text(&m).contains("Reset during homing (alarm 6)"),
        "{m:?}"
    );
    r.pty.with(|s| {
        let tail: Vec<u8> = s.raw[before..]
            .iter()
            .map(|b| b.1)
            .filter(|b| *b != b'?')
            .collect();
        assert_eq!(
            tail.first(),
            Some(&0x18),
            "first non-poll byte must be the reset: {tail:?}"
        );
        assert!(!tail.contains(&b'!'), "feed hold sent while homing");
    });
    // the job is gone after an alarm; the maker unlocks and the old job id is unknown
    r.send(Message::Unlock);
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .expect("Idle after unlock");
    r.send(Message::Start { job_id: id });
    assert!(
        until_error(&mut r, 5.0).is_some(),
        "the dropped job could be started"
    );
    assert_eq!(dollar_h(&r), 1);
}

#[test]
fn c3_stop_during_homing_from_idle_ends_in_alarm_6_with_unlock() {
    let mut r = idle(Cfg {
        homing_ms: LONG_HOME,
        ..Cfg::default()
    });
    r.send(Message::Home);
    r.wait(5.0, is_homing).unwrap();
    cycle_running(&r);
    r.send(Message::Stop);
    let (_, m) = r
        .wait_state(10.0, |s| matches!(s, State::Alarm))
        .expect("alarm");
    assert!(
        alarm_text(&m).contains("Reset during homing (alarm 6)"),
        "{m:?}"
    );
    r.send(Message::Unlock);
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .expect("unlock");
    assert_eq!(unlocks_sent(&r), 1);
}

#[test]
fn c3_stop_during_homing_a_second_stop_is_harmless() {
    let mut r = idle(Cfg {
        homing_ms: LONG_HOME,
        ..Cfg::default()
    });
    r.send(Message::Home);
    r.wait(5.0, is_homing).unwrap();
    cycle_running(&r);
    r.send(Message::Stop);
    r.send(Message::Stop);
    r.wait_state(10.0, |s| matches!(s, State::Alarm))
        .expect("alarm");
    r.drain(1.0);
    let resets = r.pty.with(|s| s.raw.iter().filter(|b| b.1 == 0x18).count());
    assert!(resets >= 1);
}

#[test]
fn c4_homing_not_enabled_error_5_words_and_machine_left_idle_job_kept() {
    let mut r = idle(Cfg {
        home_reply: Some("error:5"),
        ..Cfg::default()
    });
    let id = r.submit(CUT).unwrap();
    r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
    r.send(Message::Home);
    let (_, m) = r
        .wait(8.0, |m| {
            detail(m).contains("Homing is not enabled (error 5)")
        })
        .unwrap_or_else(|| {
            panic!(
                "no words: {:?}",
                r.history.iter().map(|h| &h.1).collect::<Vec<_>>()
            )
        });
    assert!(
        !is_state(&m, |s| matches!(s, State::Alarm)),
        "error 5 must not alarm: {m:?}"
    );
    // job kept after error:5 and startable
    r.send(Message::GetStatus);
    r.drain(1.0);
    let last = r.last_status().cloned().unwrap();
    assert!(
        is_ready(&last) || is_state(&last, |s| matches!(s, State::Idle)),
        "{last:?}"
    );
    r.send(Message::Start { job_id: id });
    r.wait_state(15.0, |s| matches!(s, State::Done { .. } | State::Running))
        .expect("kept job did not start");
    assert_eq!(dollar_h(&r), 1);
}

#[test]
fn c4_homing_not_enabled_idle_machine_stays_idle_and_second_press_gives_same_words() {
    let mut r = idle(Cfg {
        home_reply: Some("error:5"),
        ..Cfg::default()
    });
    for _ in 0..2 {
        r.send(Message::Home);
        r.wait(8.0, |m| {
            detail(m).contains("Homing is not enabled (error 5)")
        })
        .expect("words");
    }
    assert!(r.pty.with(|s| s.alarm.is_none()));
    assert_eq!(dollar_h(&r), 2);
}

#[test]
fn c4_alarm_9_and_8_words_unlock_and_job_dropped() {
    for (reply, words) in [
        ("ALARM:9", "Homing did not find the switch (alarm 9)"),
        (
            "ALARM:8",
            "Homing pull-off did not clear the switch (alarm 8)",
        ),
    ] {
        let mut r = idle(Cfg {
            home_reply: Some(reply),
            homing_ms: 400,
            ..Cfg::default()
        });
        let id = r.submit(CUT).unwrap();
        r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
        r.send(Message::Home);
        let (_, m) = r
            .wait_state(10.0, |s| matches!(s, State::Alarm))
            .unwrap_or_else(|| panic!("{reply}: no alarm"));
        assert!(alarm_text(&m).contains(words), "{reply}: {m:?}");
        r.send(Message::Unlock);
        r.wait_state(8.0, |s| matches!(s, State::Idle))
            .expect("unlock");
        assert_eq!(unlocks_sent(&r), 1);
        r.send(Message::Start { job_id: id });
        assert!(
            until_error(&mut r, 5.0).is_some(),
            "{reply}: Ready job survived the alarm"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 0129 loss clock
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_a_9_second_silent_homing_completes_without_losing_the_controller() {
    let mut r = idle(Cfg {
        homing_ms: 9_000,
        silent_homing: true,
        ..Cfg::default()
    });
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing");
    let mark = r.history.len();
    let t = Instant::now();
    let end = r.wait(20.0, |m| {
        is_idle(m) || is_state(m, |s| matches!(s, State::NoController))
    });
    let (_, m) = end.expect("no end");
    assert!(is_idle(&m), "silent homing lost the controller: {m:?}");
    assert!(
        t.elapsed() > Duration::from_secs(5),
        "homing ended too early"
    );
    assert!(
        !r.history[mark..]
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::NoController))),
        "NoController during homing"
    );
}

// A port error during homing cannot be made with a pty: closing the master gives the connector
// neither a read nor a write error (the idle case ends only by the 3 s silence rule, which homing
// pauses by design). Not exercised here; see the report.

// ---------------------------------------------------------------------------------------------
// --require-local-start
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_require_local_start_home_waits_for_the_press_at_the_connector() {
    let mut r = Rig::start_with(
        Cfg {
            homing_ms: 500,
            ..Cfg::default()
        },
        &["--require-local-start"],
    );
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .expect("Idle");
    r.send(Message::Home);
    r.wait_state(5.0, |s| matches!(s, State::AwaitingLocalStart))
        .expect("Home must wait");
    r.drain(2.0);
    assert_eq!(dollar_h(&r), 0, "$H written before the press");
    assert!(!r.history.iter().any(|(_, m)| is_homing(m)));
    r.child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    r.wait(5.0, is_homing).expect("Homing after the press");
    r.wait(10.0, is_idle).expect("Idle");
    assert_eq!(dollar_h(&r), 1);
    // Unlock stays free under require-local-start (0127 G33) and Home needs the press every time
    r.send(Message::Home);
    r.wait_state(5.0, |s| matches!(s, State::AwaitingLocalStart))
        .expect("waits again");
}

// ---------------------------------------------------------------------------------------------
// The fake controller (criterion 6)
// ---------------------------------------------------------------------------------------------

#[test]
fn c6_fake_home_from_idle_and_ready_keeps_the_job() {
    let mut r = start_fake(&[]);
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .expect("Idle");
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("fake Homing");
    r.wait(10.0, is_idle).expect("fake Idle");
    let id = r.submit(CUT).unwrap();
    r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("fake Homing with a job");
    r.wait(10.0, is_ready).expect("job kept");
    r.send(Message::Start { job_id: id });
    r.wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .expect("ran");
}

#[test]
fn c6_fake_stop_during_homing_keeps_the_ready_job() {
    let mut r = start_fake(&[]);
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(10.0, |s| matches!(s, State::Idle)).unwrap();
    let id = r.submit(CUT).unwrap();
    r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing");
    r.send(Message::Stop);
    // the homing ends (stopped) without discarding the job; wait for the end of Homing
    r.wait(10.0, |m| {
        !is_homing(m) && matches!(m, Message::Status { .. })
    })
    .expect("homing ended");
    r.drain(1.0);
    let last = r.last_status().cloned().unwrap();
    assert!(
        is_ready(&last) || is_state(&last, |s| matches!(s, State::Alarm)),
        "after Stop in fake homing: {last:?}"
    );
    if is_ready(&last) {
        r.send(Message::Start { job_id: id });
        r.wait_state(15.0, |s| matches!(s, State::Done { .. }))
            .expect("kept job runs");
    }
}

#[test]
fn c6_fake_home_while_running_is_refused_and_in_alarm_works() {
    let mut r = start_fake(&[]);
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(10.0, |s| matches!(s, State::Idle)).unwrap();
    let mut job = String::from("G21\nG90\nM4 S0\n");
    for i in 0..400 {
        job += &format!("G1 X{} Y{} F600 S500\n", i % 100, i % 50);
    }
    r.start_job(&job);
    if r.wait_state(5.0, |s| matches!(s, State::Running)).is_some() {
        r.send(Message::Home);
        let (code, msg) = until_error(&mut r, 5.0).expect("refusal while running");
        assert_eq!(code, curvyo_connector_core::ErrorCode::WrongState, "{msg}");
        r.send(Message::Stop);
    }
}

// ---------------------------------------------------------------------------------------------
// Goldens
// ---------------------------------------------------------------------------------------------

fn msg_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-connector-core/tests/fixtures/messages")
}

#[test]
fn golden_the_three_new_messages_have_exactly_these_bytes() {
    let d = msg_fixtures();
    let read = |n: &str| std::fs::read_to_string(d.join(n)).unwrap();
    assert_eq!(read("command_home.json").trim_end(), r#"{"type":"Home"}"#);
    assert_eq!(
        serde_json::to_string(&Message::Home).unwrap(),
        r#"{"type":"Home"}"#
    );
    let homing = Message::Status {
        state: State::Homing,
        job: None,
        position: None,
        alarm: None,
        detail: None,
        pause_pending: false,
        requested_by: None,
        job_name: None,
        sent_by: None,
        homed: false,
        machine_position: None,
        work_offset: None,
    };
    assert_eq!(
        read("status_homing.json").trim_end(),
        r#"{"type":"Status","state":{"name":"Homing"},"job":null,"position":null,"alarm":null}"#
    );
    assert_eq!(
        serde_json::to_string(&homing).unwrap(),
        read("status_homing.json").trim_end()
    );
    let held = read("status_homing_ready.json");
    let parsed: Message = serde_json::from_str(held.trim_end()).unwrap();
    assert!(
        matches!(&parsed, Message::Status { state: State::Homing, job: Some(j), .. } if j.line == 0 && j.lines == 6)
    );
    assert_eq!(serde_json::to_string(&parsed).unwrap(), held.trim_end());
}

#[test]
fn golden_old_fixtures_are_unchanged_since_main_and_round_trip() {
    let out = Command::new("git")
        .args([
            "diff",
            "--name-status",
            "main...HEAD",
            "--",
            "curvyo-connector-core/tests/fixtures",
            "curvyo-connector-io/tests/fixtures",
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .output();
    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        for l in text.lines() {
            // jobs.log is a log whose new Home line is appended; every golden message is new or untouched
            assert!(
                l.starts_with('A') || l.ends_with("jobs.log"),
                "an existing fixture changed: {l}"
            );
        }
        let log = Command::new("git")
            .args([
                "diff",
                "main...HEAD",
                "--",
                "curvyo-connector-io/tests/fixtures/jobs.log",
            ])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
            .output()
            .unwrap();
        for l in String::from_utf8_lossy(&log.stdout).lines() {
            assert!(
                !l.starts_with('-') || l.starts_with("---"),
                "a jobs.log line was removed or changed: {l}"
            );
        }
    }
    for e in std::fs::read_dir(msg_fixtures()).unwrap() {
        let p = e.unwrap().path();
        let text = std::fs::read_to_string(&p).unwrap();
        let t = text.trim_end();
        let m: Message = serde_json::from_str(t).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            t,
            "{p:?} does not round-trip byte-identically"
        );
    }
}

#[test]
fn golden_an_old_peer_status_without_homing_still_reads() {
    // additive: a v0 client that never heard of Home reads every old state
    let m: Message = serde_json::from_str(
        r#"{"type":"Status","state":{"name":"Idle"},"job":null,"position":null,"alarm":null}"#,
    )
    .unwrap();
    assert!(is_idle(&m));
}

/// Manual: my simulator on a pty for driving the real app (`HOMING_MS`, `HOME_REPLY`, `STARTUP_ALARM`).
/// Prints `SIM_PATH <slave>` and answers until killed.
#[test]
#[ignore = "manual: drives the real desktop app"]
fn manual_pty_sim() {
    let dir = tempfile::tempdir().unwrap();
    let ms = std::env::var("HOMING_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6_000);
    let reply = std::env::var("HOME_REPLY")
        .ok()
        .filter(|v| !v.is_empty())
        .map(|s| &*Box::leak(s.into_boxed_str()));
    let cfg = Cfg {
        homing_ms: ms,
        home_reply: reply,
        startup_alarm: std::env::var("STARTUP_ALARM").is_ok_and(|v| !v.is_empty()),
        ..Cfg::default()
    };
    let pty = Pty::new(cfg, dir.path());
    println!("SIM_PATH {}", pty.path());
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
