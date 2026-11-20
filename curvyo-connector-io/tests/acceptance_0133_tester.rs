//! Independent acceptance tests for `specs/0133-fix-pause-while-homing` (PH1 to PH9), written from
//! the specification and UX notes before reading the implementation. They run the REAL connector
//! binary against my OWN minimal GRBL 1.1f simulator (silent `$H`, `!` ignored while homing) on a
//! real pseudo-terminal. Harness copied from the tester's `0118` file; the homing ends when the
//! test says so, never after a fixed sleep.
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
                Some(e) if e.starts_with("error:") => {
                    self.ack(now, e);
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
            if !self.cfg.silent_homing
                && self.cfg.home_reply.is_some_and(|r| r.starts_with("error:"))
            {
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
// 0133 helpers
// ---------------------------------------------------------------------------------------------

const HJOB: &str = "$H\nG21\nG90\nM4 S0\nG1 X10 Y10 F600 S500\nG1 X20 Y20\nM5\n";

fn pending(m: &Message) -> bool {
    matches!(
        m,
        Message::Status {
            pause_pending: true,
            ..
        }
    )
}

fn is_paused(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::Paused))
}

fn is_running(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::Running))
}

fn done_completed(m: &Message) -> bool {
    is_state(m, |s| {
        matches!(
            s,
            State::Done {
                result: JobResult::Completed
            }
        )
    })
}

fn homing_cfg() -> Cfg {
    Cfg {
        homing_ms: LONG_HOME,
        silent_homing: true,
        ..Cfg::default()
    }
}

/// A job that starts with `$H` is running and GRBL is inside the silent homing cycle.
fn job_homing(cfg: Cfg) -> Rig {
    let mut r = idle(cfg);
    r.start_job(HJOB);
    r.wait_state(10.0, |s| matches!(s, State::Running))
        .expect("Running");
    cycle_running(&r);
    r
}

/// The homing cycle ends now (as the machine reaching its switches), with the cfg's reply.
fn finish_home(r: &Rig) {
    r.pty
        .with(|s| s.homing_until = Some(Instant::now() - Duration::from_millis(1)));
}

/// Raw bytes received after the last `$H\n`, without the `?` polls.
fn after_h(r: &Rig) -> Vec<u8> {
    r.pty.with(|s| {
        let b: Vec<u8> = s.raw.iter().map(|x| x.1).collect();
        let pos = b
            .windows(3)
            .rposition(|w| w == b"$H\n")
            .expect("no $H received");
        b[pos + 3..]
            .iter()
            .copied()
            .filter(|c| *c != b'?')
            .collect()
    })
}

fn bangs(r: &Rig) -> usize {
    r.pty.with(|s| s.raw.iter().filter(|x| x.1 == b'!').count())
}

/// Job lines received after `$H`, in order.
fn lines_after_h(r: &Rig) -> Vec<String> {
    r.pty.with(|s| {
        let p = s.lines.iter().rposition(|(_, l)| l == "$H").unwrap();
        s.lines[p + 1..].iter().map(|(_, l)| l.clone()).collect()
    })
}

fn press_pause_until_pending(r: &mut Rig) {
    r.send(Message::Pause);
    r.wait(10.0, pending).unwrap_or_else(|| {
        panic!(
            "no pause_pending: {:?}",
            r.history.iter().map(|h| &h.1).collect::<Vec<_>>()
        )
    });
}

fn never_paused(r: &Rig) -> bool {
    !r.history.iter().any(|(_, m)| is_paused(m))
}

fn final_status(r: &Rig) -> Message {
    r.last_status().cloned().expect("a status")
}

// ---------------------------------------------------------------------------------------------
// PH1, PH2
// ---------------------------------------------------------------------------------------------

#[test]
fn ph1_pause_during_job_dollar_h_is_pending_and_nothing_is_sent() {
    let mut r = job_homing(homing_cfg());
    let b0 = bangs(&r);
    let n0 = r.pty.with(|s| s.raw.len());
    r.send(Message::Pause);
    let (_, m) = r
        .wait(10.0, pending)
        .expect("Pause during `$H` gave no pause_pending status");
    let Message::Status { state, job, .. } = &m else {
        unreachable!()
    };
    assert!(
        matches!(state, State::Running),
        "state must stay Running: {m:?}"
    );
    assert_eq!(job.as_ref().unwrap().line, 0, "{m:?}");
    r.drain(0.5);
    assert!(
        !r.history
            .iter()
            .any(|(_, m)| matches!(m, Message::Error { .. })),
        "a Pause during `$H` was refused: {:?}",
        r.history
    );
    assert_eq!(bangs(&r), b0, "no `!` while GRBL homes");
    // nothing but status polls reached GRBL since the press
    r.pty.with(|s| {
        let rest: Vec<u8> = s.raw[n0..]
            .iter()
            .map(|x| x.1)
            .filter(|c| *c != b'?')
            .collect();
        assert!(rest.is_empty(), "bytes sent while homing: {rest:?}");
    });
    // still pending later (a status poll or a second client would say the same)
    r.send(Message::GetStatus);
    assert!(r.wait(5.0, pending).is_some(), "pending lost on GetStatus");
}

#[test]
fn ph2_ok_for_dollar_h_writes_bang_before_any_line_then_paused_laser_off() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    finish_home(&r);
    let (_, m) = r.wait(10.0, is_paused).expect("never Paused after homing");
    let Message::Status {
        pause_pending, job, ..
    } = &m
    else {
        unreachable!()
    };
    assert!(!pause_pending, "{m:?}");
    assert_eq!(
        job.as_ref().unwrap().line,
        1,
        "line of `$H` acknowledged: {m:?}"
    );
    // the connector wrote `!` as the first byte after `$H\n`
    assert_eq!(
        after_h(&r).first(),
        Some(&b'!'),
        "after_h = {:?}",
        after_h(&r)
    );
    // wait for the simulator to be in Hold, then watch a quiet window
    let t = Instant::now();
    while r.pty.with(|s| s.hold_since.is_none()) {
        assert!(t.elapsed() < WAIT, "GRBL never held");
        std::thread::sleep(Duration::from_millis(10));
    }
    r.drain(1.5);
    assert!(is_paused(&final_status(&r)), "left Paused by itself");
    assert!(
        lines_after_h(&r).is_empty(),
        "lines streamed after `$H`: {:?}",
        lines_after_h(&r)
    );
    r.pty.with(|s| {
        assert!(!s.laser_out(Instant::now()), "laser on");
        assert!(!s.spindle_on && s.s_value == 0.0, "M3/M4/S sent");
        assert_eq!(s.lines.last().map(|l| l.1.as_str()), Some("$H"));
    });
    assert_eq!(bangs(&r), 1);
}

#[test]
fn ph2_resume_after_the_hold_continues_with_the_next_line_and_completes() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    finish_home(&r);
    r.wait(10.0, is_paused).expect("Paused");
    r.send(Message::Resume);
    r.wait(30.0, done_completed)
        .expect("job never completed after Resume");
    let ls = lines_after_h(&r);
    let want = prepare(HJOB)[1..].to_vec();
    assert!(
        ls.starts_with(&want),
        "next line after `$H` must be the job's own next line in order: {ls:?} vs {want:?}"
    );
    assert_eq!(dollar_h(&r), 1);
    // `~` after the `!`
    r.pty.with(|s| {
        let rt: Vec<u8> = s
            .raw
            .iter()
            .map(|x| x.1)
            .filter(|c| matches!(*c, b'!' | b'~'))
            .collect();
        assert_eq!(rt, b"!~");
    });
}

// ---------------------------------------------------------------------------------------------
// PH4
// ---------------------------------------------------------------------------------------------

#[test]
fn ph4_resume_while_pending_cancels_and_the_job_streams_on_without_a_hold() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    r.send(Message::Resume);
    let (_, m) = r
        .wait(10.0, |m| is_running(m) && !pending(m))
        .expect("Resume did not clear pause_pending");
    let _ = m;
    finish_home(&r);
    r.wait(30.0, done_completed).expect("job did not run on");
    assert_eq!(bangs(&r), 0, "a cancelled pause still held the machine");
    assert!(never_paused(&r), "went Paused");
    assert_eq!(lines_after_h(&r)[..2], prepare(HJOB)[1..3]);
}

#[test]
fn ph4_second_pause_is_idempotent_and_the_pause_still_holds() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    r.send(Message::Pause);
    r.send(Message::Pause);
    r.drain(0.7);
    assert!(
        !r.history
            .iter()
            .any(|(_, m)| matches!(m, Message::Error { .. })),
        "a second Pause was refused: {:?}",
        r.history.iter().map(|h| &h.1).collect::<Vec<_>>()
    );
    assert!(pending(&final_status(&r)), "pending lost");
    finish_home(&r);
    r.wait(10.0, is_paused).expect("Paused");
    assert_eq!(bangs(&r), 1, "exactly one hold");
}

// ---------------------------------------------------------------------------------------------
// PH5
// ---------------------------------------------------------------------------------------------

#[test]
fn ph5_stop_while_pending_resets_without_a_hold_and_clears_the_pause() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    let before = r.pty.with(|s| s.raw.len());
    r.send(Message::Stop);
    // GRBL answers the reset during homing with ALARM:6; the job ends either way
    let (_, m) = r
        .wait(15.0, |m| {
            is_state(m, |s| {
                matches!(s, State::Alarm | State::Done { .. } | State::Idle)
            })
        })
        .expect("Stop did not end the job");
    assert!(!pending(&m), "pause_pending survived the Stop: {m:?}");
    r.pty.with(|s| {
        let tail: Vec<u8> = s.raw[before..]
            .iter()
            .map(|b| b.1)
            .filter(|b| *b != b'?')
            .collect();
        assert_eq!(tail.first(), Some(&0x18), "reset must come first: {tail:?}");
        assert!(!tail.contains(&b'!'), "feed hold while homing: {tail:?}");
    });
    r.drain(1.5);
    assert!(never_paused(&r));
    assert!(!pending(&final_status(&r)));
    // a second job afterwards must not inherit the pause
    r.send(Message::Unlock);
    r.drain(0.5);
}

// ---------------------------------------------------------------------------------------------
// PH6
// ---------------------------------------------------------------------------------------------

fn failing(reply: &'static str) -> Rig {
    let mut r = job_homing(Cfg {
        home_reply: Some(reply),
        homing_ms: LONG_HOME,
        silent_homing: true,
        ..Cfg::default()
    });
    press_pause_until_pending(&mut r);
    finish_home(&r);
    r
}

fn assert_dropped(r: &mut Rig, what: &str) {
    let (_, m) = r
        .wait(15.0, |m| {
            is_state(m, |s| matches!(s, State::Alarm | State::Done { .. }))
        })
        .unwrap_or_else(|| {
            panic!(
                "{what}: job never ended: {:?}",
                r.history.iter().map(|h| &h.1).collect::<Vec<_>>()
            )
        });
    assert!(!pending(&m), "{what}: pending kept: {m:?}");
    r.drain(1.5);
    assert!(never_paused(r), "{what}: went Paused");
    assert!(!pending(&final_status(r)), "{what}: pending at the end");
    assert_eq!(bangs(r), 0, "{what}: a hold was written");
    // only the connector's own safety lines (laser off, air off) may follow a failed `$H`
    assert!(
        lines_after_h(r).iter().all(|l| l == "M5" || l == "M9"),
        "{what}: job lines after failed `$H`: {:?}",
        lines_after_h(r)
    );
    eprintln!("{what}: ended as {m:?}");
}

#[test]
fn ph6_alarm_9_drops_the_pending_pause() {
    let mut r = failing("ALARM:9");
    assert_dropped(&mut r, "ALARM:9");
}

#[test]
fn ph6_alarm_8_drops_the_pending_pause() {
    let mut r = failing("ALARM:8");
    assert_dropped(&mut r, "ALARM:8");
}

#[test]
fn ph6_error_5_drops_the_pending_pause() {
    let mut r = failing("error:5");
    assert_dropped(&mut r, "error:5");
}

// ---------------------------------------------------------------------------------------------
// PH3
// ---------------------------------------------------------------------------------------------

fn pause_refused_in_button_homing(with_job: bool) {
    let mut r = idle(homing_cfg());
    if with_job {
        r.submit(CUT).unwrap();
        r.wait_state(5.0, |s| matches!(s, State::Ready)).unwrap();
    }
    r.send(Message::Home);
    r.wait(5.0, is_homing).expect("Homing");
    cycle_running(&r);
    let b0 = bangs(&r);
    r.send(Message::Pause);
    let (code, text) = until_error(&mut r, 5.0).expect("Pause during Home answered nothing");
    assert!(
        matches!(code, curvyo_connector_core::ErrorCode::WrongState),
        "{code:?} {text}"
    );
    assert!(text.contains("No job to pause"), "{text}");
    assert_eq!(bangs(&r), b0);
    // nothing remembered: homing ends, no hold, no pending, state Idle/Ready
    finish_home(&r);
    r.wait(10.0, |m| is_idle(m) || is_ready(m))
        .expect("homing end");
    r.drain(1.0);
    assert!(!r.history.iter().any(|(_, m)| pending(m)));
    assert!(never_paused(&r));
    assert_eq!(bangs(&r), b0);
}

#[test]
fn ph3_pause_in_button_homing_is_no_job_to_pause() {
    pause_refused_in_button_homing(false);
}

#[test]
fn ph3_pause_in_button_homing_with_a_ready_job_is_the_same() {
    pause_refused_in_button_homing(true);
}

// ---------------------------------------------------------------------------------------------
// PH7: normal run unchanged, 2 s drop for a motion line
// ---------------------------------------------------------------------------------------------

#[test]
fn ph7_normal_pause_still_holds() {
    let mut r = idle(Cfg::default());
    long_running(&mut r);
    r.send(Message::Pause);
    r.wait(10.0, is_paused).expect("normal Pause");
    assert!(
        !r.history.iter().any(|(_, m)| pending(m)),
        "pending in a normal run"
    );
    assert_eq!(bangs(&r), 1);
}

#[test]
fn ph7_g29_drop_still_applies_on_a_motion_line() {
    let mut r = idle(Cfg {
        ignore_bang: true,
        ..Cfg::default()
    });
    long_running(&mut r);
    r.send(Message::Pause);
    let t = Instant::now();
    while bangs(&r) == 0 {
        assert!(t.elapsed() < WAIT, "no `!` for a normal Pause");
        std::thread::sleep(Duration::from_millis(10));
    }
    let sent = Instant::now();
    // GRBL never reports Hold:0; the connector must drop the pause within its hold limit
    r.drain(5.0);
    assert!(never_paused(&r), "Paused although GRBL never held");
    assert!(
        !r.history.iter().any(|(_, m)| pending(m)),
        "a motion-line Pause must not become pending"
    );
    let last = final_status(&r);
    assert!(is_running(&last), "{last:?}");
    assert!(sent.elapsed() >= Duration::from_secs(5));
}

// ---------------------------------------------------------------------------------------------
// PH9: unplug while pending
// ---------------------------------------------------------------------------------------------

// FINDING: with a job's silent `$H` running the connector never notices a pulled cable (90 s
// observed, also without any Pause), so the pending pause stays pending. Run with --ignored.
#[test]
#[ignore = "finding: a pulled cable during a job's `$H` is never detected (PH9)"]
fn ph9_unplug_while_pending_drops_the_pause_and_a_new_job_is_not_paused() {
    let mut r = job_homing(homing_cfg());
    press_pause_until_pending(&mut r);
    r.pty.unplug();
    let (_, m) = r
        .wait(30.0, |m| matches!(m, Message::Status { .. }) && !pending(m))
        .unwrap_or_else(|| {
            panic!(
                "pending never cleared after the cable was pulled: {:?}",
                r.history
                    .iter()
                    .rev()
                    .take(6)
                    .map(|h| &h.1)
                    .collect::<Vec<_>>()
            )
        });
    eprintln!("after unplug: {m:?}");
    assert!(never_paused(&r), "Paused on a dead link");
    // replug a fresh machine; the next job with `$H` must stream on
    r.pty.replug(Cfg {
        homing_ms: 300,
        ..Cfg::default()
    });
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle)).is_some() {
            break;
        }
        assert!(
            t.elapsed() < WAIT,
            "never Idle again: {:?}",
            final_status(&r)
        );
    }
    settle(&mut r);
    let b0 = bangs(&r);
    r.start_job(HJOB);
    r.wait(30.0, done_completed)
        .expect("second job did not complete");
    assert_eq!(bangs(&r), b0, "stale pending pause held the next job");
    assert!(!r.history.iter().rev().take(20).any(|(_, m)| pending(m)));
}

// ---------------------------------------------------------------------------------------------
// Goldens
// ---------------------------------------------------------------------------------------------

fn msg_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-connector-core/tests/fixtures/messages")
}

#[test]
fn golden_status_pause_pending_and_old_messages_byte_identical() {
    let text = std::fs::read_to_string(msg_fixtures().join("status_pause_pending.json")).unwrap();
    let t = text.trim_end();
    let m: Message = serde_json::from_str(t).unwrap();
    assert!(pending(&m));
    assert_eq!(serde_json::to_string(&m).unwrap(), t);
    // false is left out: an old Status is byte-identical
    for old in [
        r#"{"type":"Status","state":{"name":"Idle"},"job":null,"position":null,"alarm":null}"#,
        r#"{"type":"Status","state":{"name":"Homing"},"job":null,"position":null,"alarm":null}"#,
    ] {
        let m: Message = serde_json::from_str(old).unwrap();
        assert!(!pending(&m));
        assert_eq!(serde_json::to_string(&m).unwrap(), old);
    }
    // every golden round-trips byte-identically, none modified or deleted since main
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
        .output()
        .unwrap();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        assert!(
            l.starts_with('A') || l.ends_with("jobs.log"),
            "an existing fixture changed: {l}"
        );
    }
    for e in std::fs::read_dir(msg_fixtures()).unwrap() {
        let p = e.unwrap().path();
        let text = std::fs::read_to_string(&p).unwrap();
        let t = text.trim_end();
        let m: Message = serde_json::from_str(t).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        assert_eq!(serde_json::to_string(&m).unwrap(), t, "{p:?}");
    }
}

/// Manual: my simulator with a silent `$H` on a pty for driving the real app (`HOMING_MS`,
/// `HOME_REPLY`). Prints `SIM_PATH <slave>` and answers until killed.
#[test]
#[ignore = "manual: drives the real desktop app"]
fn manual_pty_sim() {
    let dir = tempfile::tempdir().unwrap();
    let ms = std::env::var("HOMING_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15_000);
    let reply = std::env::var("HOME_REPLY")
        .ok()
        .filter(|v| !v.is_empty())
        .map(|s| &*Box::leak(s.into_boxed_str()));
    let pty = Pty::new(
        Cfg {
            homing_ms: ms,
            home_reply: reply,
            silent_homing: true,
            ..Cfg::default()
        },
        dir.path(),
    );
    println!("SIM_PATH {}", pty.path());
    let t0 = Instant::now();
    let mut seen = 0;
    let mut seen_rt = 0;
    loop {
        std::thread::sleep(Duration::from_millis(100));
        pty.with(|s| {
            for (_, l) in &s.lines[seen..] {
                if l.len() < 12 {
                    println!("{:6.1}s line {l}", t0.elapsed().as_secs_f32());
                }
            }
            seen = s.lines.len();
            for (_, b, _) in &s.rt[seen_rt..] {
                if *b != b'?' {
                    println!("{:6.1}s rt 0x{b:02x}", t0.elapsed().as_secs_f32());
                }
            }
            seen_rt = s.rt.len();
        });
    }
}
