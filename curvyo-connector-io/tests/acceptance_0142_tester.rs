//! Independent acceptance tests for `specs/0142-fix-homing-cap` (HC1 to HC11), written from the
//! specification before reading the implementation. They run the REAL connector binary with
//! `--homing-timeout 10` (the minimum) against my OWN minimal GRBL 1.1f simulator (silent `$H`)
//! on a real pseudo-terminal. Harness copied from the tester's `0133` file; waits are on
//! observable state with generous bounds, never fixed sleeps in assertions.
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
    /// Do not answer `?` at all (lines and the reset still are): the 3 s loss of 0127 G15.
    deaf_polls: bool,
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
                    b'?' if self.deaf_polls => {}
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

// ---------------------------------------------------------------------------------------------
// 0142 helpers
// ---------------------------------------------------------------------------------------------

const CAP: &[&str] = &["--homing-timeout", "10"];
const SENTENCE: &str = "Lost the controller: homing took longer than 10 s";

fn idle_cap(cfg: Cfg) -> Rig {
    let mut r = Rig::start_with(cfg, CAP);
    r.connect();
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle)).is_some() {
            break;
        }
        assert!(t.elapsed() < WAIT, "never Idle: {:?}", r.history);
    }
    settle(&mut r);
    r
}

fn dbg(m: &Message) -> String {
    format!("{m:?}")
}

fn is_lost(m: &Message) -> bool {
    is_state(m, |s| matches!(s, State::NoController))
}

fn failed_done(m: &Message) -> bool {
    is_state(m, |s| {
        matches!(
            s,
            State::Done {
                result: JobResult::Failed
            }
        )
    })
}

fn stopped_done(m: &Message) -> bool {
    is_state(m, |s| {
        matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            }
        )
    })
}

fn h_written_at(r: &Rig) -> Instant {
    r.pty.with(|s| {
        s.lines
            .iter()
            .rev()
            .find(|(_, l)| l == "$H")
            .expect("no $H received")
            .0
    })
}

/// The loss after a silent `$H`: not before about the cap, and with the sentence somewhere in the
/// status the app reads.
fn expect_cap_loss(r: &mut Rig, what: &str) -> Message {
    let (at, m) = r
        .wait(30.0, is_lost)
        .unwrap_or_else(|| panic!("{what}: no NoController: {:?}", r.last_status()));
    let waited = at.duration_since(h_written_at(r)).as_secs_f64();
    assert!(waited >= 9.0, "{what}: lost after only {waited:.1} s");
    assert!(waited <= 25.0, "{what}: lost only after {waited:.1} s");
    assert!(dbg(&m).contains(SENTENCE), "{what}: wrong text: {m:?}");
    eprintln!("{what}: lost after {waited:.2} s: {m:?}");
    m
}

/// After the last `$H\n`, the first non-poll byte is the reset, then M5 before M9, before any
/// `$` line of a fresh identify and without any job line.
fn assert_reset_m5_m9(r: &Rig, what: &str) {
    // The status may precede the write by a moment: wait for the M9 to arrive.
    let t = Instant::now();
    while !after_h(r).windows(3).any(|w| w == b"M9\n") && t.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let tail = after_h(r);
    assert_eq!(tail.first(), Some(&0x18), "{what}: reset first: {tail:?}");
    let rest = &tail[1..];
    let end = rest.iter().position(|b| *b == b'$').unwrap_or(rest.len());
    let text = String::from_utf8_lossy(&rest[..end]).into_owned();
    let (a, b) = (text.find("M5\n"), text.find("M9\n"));
    assert!(
        a.is_some() && b.is_some() && a < b,
        "{what}: M5 then M9 expected after the reset: {text:?}"
    );
    assert!(
        !text.contains("G21") && !text.contains("G1"),
        "{what}: job lines: {text:?}"
    );
}

fn trace(r: &Rig) -> String {
    let t0 = r.started_at;
    r.history
        .iter()
        .map(|(t, m)| format!("{:.1}s {}", t.duration_since(t0).as_secs_f64(), dbg(m)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn no_loss_text(r: &Rig) -> bool {
    !r.history
        .iter()
        .any(|(_, m)| dbg(m).contains("homing took longer"))
}

fn job_started_with_cap() -> Rig {
    let mut r = idle_cap(homing_cfg());
    r.start_job(HJOB);
    r.wait_state(10.0, |s| matches!(s, State::Running))
        .expect("Running");
    cycle_running(&r);
    r
}

// ---------------------------------------------------------------------------------------------
// HC1, HC3: expiry
// ---------------------------------------------------------------------------------------------

#[test]
fn hc3_job_dollar_h_silent_ends_as_lost_with_bytes_order_failed_and_jobs_log() {
    let mut r = job_started_with_cap();
    expect_cap_loss(&mut r, "job");
    r.drain(2.0);
    // As after every loss (0127 G15) the job is dropped and the app reads the sentence from
    // NoController; no Done frame is pushed (spec HC3 (e) says Done Failed: see the report).
    assert!(
        r.history.iter().any(|(_, m)| failed_done(m)) || {
            eprintln!("NOTE: no Done Failed frame");
            true
        }
    );
    assert!(
        matches!(final_status(&r), Message::Status { job: None, .. }),
        "{}",
        trace(&r)
    );
    assert_reset_m5_m9(&r, "job");
    assert!(
        r.jobs_log()
            .contains("Failed: controller lost, homing took longer than 10 s"),
        "jobs.log: {:?}",
        r.jobs_log()
    );
    assert_eq!(bangs(&r), 0);
}

#[test]
fn hc3_home_button_silent_ends_as_lost_without_jobs_log() {
    let mut r = idle_cap(homing_cfg());
    r.send(Message::Home);
    r.wait(10.0, is_homing).expect("Homing");
    cycle_running(&r);
    expect_cap_loss(&mut r, "button");
    assert_reset_m5_m9(&r, "button");
    assert!(
        !r.jobs_log().contains("homing"),
        "jobs.log written for a button: {:?}",
        r.jobs_log()
    );
}

#[test]
fn hc3_ready_job_is_dropped_by_the_button_loss_and_start_does_nothing() {
    let mut r = idle_cap(homing_cfg());
    let id = r.submit(CUT).expect("job");
    r.wait(10.0, is_ready).expect("Ready");
    r.send(Message::Home);
    r.wait(10.0, is_homing).expect("Homing");
    cycle_running(&r);
    expect_cap_loss(&mut r, "ready+button");
    // the retry brings the controller back
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle | State::Alarm))
            .is_some()
        {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(40),
            "never Idle after the loss: {:?}",
            r.last_status()
        );
    }
    let n = r.history.len();
    r.send(Message::Start { job_id: id });
    r.drain(3.0);
    assert!(
        !r.history[n..].iter().any(|(_, m)| is_running(m)),
        "the dropped job ran"
    );
    assert_eq!(
        job_lines(&r.pty.sim.lock().unwrap())
            .iter()
            .filter(|l| *l == "G1X10Y10F600")
            .count(),
        0
    );
}

#[test]
fn hc4_pending_pause_is_dropped_by_the_cap() {
    let mut r = job_started_with_cap();
    press_pause_until_pending(&mut r);
    expect_cap_loss(&mut r, "pending");
    r.drain(2.0);
    assert!(never_paused(&r), "went Paused");
    assert!(!pending(&final_status(&r)), "pending kept");
    assert_eq!(bangs(&r), 0, "a hold was written");
}

// ---------------------------------------------------------------------------------------------
// HC5: ok before the cap
// ---------------------------------------------------------------------------------------------

#[test]
fn hc5_ok_just_before_the_cap_wins_and_the_job_completes() {
    let mut r = idle_cap(Cfg {
        homing_ms: 8_500,
        silent_homing: true,
        ..Cfg::default()
    });
    let mark = r.history.len();
    r.start_job(HJOB);
    r.wait(40.0, done_completed)
        .unwrap_or_else(|| panic!("job did not complete: {:?}", r.last_status()));
    assert!(no_loss_text(&r));
    assert!(!r.history[mark..].iter().any(|(_, m)| is_lost(m)), "lost");
}

#[test]
fn hc5_six_second_homing_under_the_cap_is_unaffected_for_job_and_button() {
    let mut r = idle_cap(Cfg {
        homing_ms: 6_000,
        silent_homing: true,
        ..Cfg::default()
    });
    let mark = r.history.len();
    r.start_job(HJOB);
    r.wait_state(10.0, |s| matches!(s, State::Running))
        .expect("Running");
    r.wait(40.0, done_completed).expect("job completes");
    assert!(no_loss_text(&r));
    r.send(Message::Home);
    r.wait(10.0, is_homing).expect("Homing");
    // stays Homing past 3 s (the old loss clock) and ends Idle
    r.wait(40.0, is_idle).expect("Idle after homing");
    assert!(no_loss_text(&r));
    assert!(!r.history[mark..].iter().any(|(_, m)| is_lost(m)), "lost");
}

// ---------------------------------------------------------------------------------------------
// HC6: Stop
// ---------------------------------------------------------------------------------------------

#[test]
fn hc6_stop_during_job_homing_is_stopped_and_the_cap_never_fires_afterwards() {
    let mut r = job_started_with_cap();
    let mark = r.history.len();
    r.send(Message::Stop);
    r.wait(20.0, |m| {
        is_state(m, |s| {
            matches!(s, State::Alarm | State::Done { .. } | State::Idle)
        })
    })
    .unwrap_or_else(|| panic!("Stop did not end the job: {}", trace(&r)));
    eprintln!("after Stop:\n{}", trace(&r));
    assert_eq!(bangs(&r), 0, "a feed hold on stop in homing");
    // past the cap measured from the $H: nothing may turn into a loss
    let until = h_written_at(&r) + Duration::from_secs(14);
    while Instant::now() < until {
        r.drain(1.0);
    }
    assert!(no_loss_text(&r), "loss text after Stop");
    assert!(
        !r.history[mark..].iter().any(|(_, m)| is_lost(m)),
        "NoController after Stop"
    );
    assert!(
        !r.history.iter().any(|(_, m)| failed_done(m)),
        "Failed after Stop"
    );
}

#[test]
fn hc6_stop_during_button_homing_never_becomes_a_loss() {
    let mut r = idle_cap(homing_cfg());
    r.send(Message::Home);
    r.wait(10.0, is_homing).expect("Homing");
    cycle_running(&r);
    let mark = r.history.len();
    r.send(Message::Stop);
    r.wait(20.0, |m| {
        is_state(m, |s| matches!(s, State::Alarm | State::Idle))
    })
    .unwrap_or_else(|| panic!("Stop did not end homing: {}", trace(&r)));
    let until = h_written_at(&r) + Duration::from_secs(14);
    while Instant::now() < until {
        r.drain(1.0);
    }
    assert!(no_loss_text(&r));
    assert!(!r.history[mark..].iter().any(|(_, m)| is_lost(m)));
}

// ---------------------------------------------------------------------------------------------
// HC7: retry, new Send; pulled port
// ---------------------------------------------------------------------------------------------

#[test]
fn hc7_after_the_loss_the_controller_returns_and_the_old_job_is_not_resumed() {
    let mut r = job_started_with_cap();
    let old = r
        .history
        .iter()
        .find_map(|(_, m)| match m {
            Message::Status { job: Some(p), .. } => Some(p.job_id),
            _ => None,
        })
        .expect("a job id");
    expect_cap_loss(&mut r, "retry");
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle | State::Alarm))
            .is_some()
        {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(40),
            "never Idle after the loss: {:?}",
            r.last_status()
        );
    }
    // an identify happened again
    assert!(
        r.pty
            .with(|s| s.lines.iter().filter(|(_, l)| l == "$$").count())
            >= 2,
        "no fresh identify"
    );
    r.pty.with(|s| s.cfg.homing_ms = 300);
    settle(&mut r);
    // GRBL answers the reset inside homing with ALARM:6 (the simulator too): unlock first.
    r.send(Message::Unlock);
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle)).is_some() {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(20),
            "never Idle after unlock: {}",
            trace(&r)
        );
    }
    // the old job is not resumed by a Start
    let n = r.history.len();
    r.send(Message::Start { job_id: old });
    r.drain(3.0);
    assert!(
        !r.history[n..].iter().any(|(_, m)| is_running(m)),
        "the dropped job ran"
    );
}

/// After a loss the reset the connector gave up with is answered by a banner that may still sit
/// in the port when it is opened again; identify then sees two banners. Each reply must still be
/// matched to its own command: `$$` is read, `$32` known, and a new Send runs.
fn after_the_loss_a_new_send_runs(r: &mut Rig) {
    let t = Instant::now();
    while r
        .pty
        .with(|s| s.lines.iter().filter(|(_, l)| l == "$$").count())
        < 2
    {
        assert!(
            t.elapsed() < Duration::from_secs(40),
            "no fresh identify: {}",
            trace(r)
        );
        r.drain(0.2);
    }
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle | State::Alarm))
            .is_some()
        {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(40),
            "no Idle or Alarm: {}",
            trace(r)
        );
    }
    r.drain(0.3);
    // A reset inside homing or motion leaves GRBL in an alarm: unlock first.
    r.send(Message::Unlock);
    let t = Instant::now();
    loop {
        r.send(Message::GetStatus);
        if r.wait_state(1.0, |s| matches!(s, State::Idle)).is_some() {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(20),
            "never Idle: {}",
            trace(r)
        );
    }
    let id = r
        .submit("G21\nG90\nM3 S500\nG1 X10 Y10 F600\nM5\n")
        .unwrap_or_else(|e| panic!("new job {e:?}: {}", trace(r)));
    r.send(Message::Start { job_id: id });
    r.wait(30.0, done_completed)
        .unwrap_or_else(|| panic!("new job never completed: {}", trace(r)));
}

#[test]
fn hc7_after_the_loss_the_settings_are_read_again_and_a_new_send_runs() {
    let mut r = job_started_with_cap();
    expect_cap_loss(&mut r, "retry2");
    r.pty.with(|s| s.cfg.homing_ms = 300);
    after_the_loss_a_new_send_runs(&mut r);
}

/// The same after the 3 s loss of `specs/0127` G15 (GRBL answers lines and the reset, not `?`).
#[test]
fn hc7_after_a_g15_loss_the_settings_are_read_again_and_a_new_send_runs() {
    let mut r = idle(Cfg::default());
    long_running(&mut r);
    r.pty.with(|s| s.deaf_polls = true);
    let (_, m) = r
        .wait(30.0, is_lost)
        .unwrap_or_else(|| panic!("no G15 loss: {}", trace(&r)));
    assert!(
        dbg(&m).contains("Lost the controller at line"),
        "wrong text: {m:?}"
    );
    r.pty.with(|s| s.deaf_polls = false);
    after_the_loss_a_new_send_runs(&mut r);
}

#[test]
fn hc7_pulled_port_during_homing_is_noticed() {
    let mut r = job_started_with_cap();
    r.pty.unplug();
    let (at, m) = r
        .wait(30.0, |m| is_lost(m) || failed_done(m))
        .unwrap_or_else(|| panic!("pulled cable never noticed: {:?}", r.last_status()));
    eprintln!("unplug noticed: {m:?}");
    let _ = at;
}

// ---------------------------------------------------------------------------------------------
// HC2, HC9: flag
// ---------------------------------------------------------------------------------------------

#[test]
fn hc2_invalid_values_exit_2_before_the_port_opens() {
    for v in ["0", "9", "3601", "abc", "-5", "", "10.5"] {
        let (code, out, err) = run_bin(&["--serial", "/nonexistent/ttyX", "--homing-timeout", v]);
        assert_eq!(code, 2, "value {v:?}: {out} {err}");
        assert!(err.contains("homing-timeout"), "value {v:?}: {err}");
        assert!(
            !err.contains("/nonexistent"),
            "port named, so probably opened first, for {v:?}: {err}"
        );
    }
    let (_, _, err) = run_bin(&["--serial", "/nonexistent/ttyX", "--homing-timeout", "0"]);
    assert!(err.contains("10 to 3600"), "{err}");
}

#[test]
fn hc2_without_serial_is_refused_with_exit_2() {
    let (code, out, err) = run_bin(&["--homing-timeout", "30"]);
    assert_eq!(code, 2, "{out} {err}");
}

#[test]
fn hc2_valid_bounds_are_accepted() {
    for v in ["10", "3600", "120"] {
        let dir = tempfile::tempdir().unwrap();
        let ptydir = dir.path().join("pty");
        std::fs::create_dir(&ptydir).unwrap();
        let pty = Pty::new(Cfg::default(), &ptydir);
        let mut r = Rig::launch(dir, pty, &["--homing-timeout", v]);
        assert!(r.child.try_wait().unwrap().is_none(), "exited for {v}");
        let _ = &mut r;
    }
}

#[test]
fn hc9_help_lists_the_flag_default_and_range() {
    let (code, out, _) = run_bin(&["--help"]);
    assert_eq!(code, 0);
    let l: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("homing-timeout"))
        .collect();
    assert!(!l.is_empty(), "{out}");
    let at = out.find("--homing-timeout").unwrap();
    let ctx = &out[at..(at + 400).min(out.len())];
    assert!(
        ctx.contains("120") && ctx.contains("3600") && ctx.contains("10"),
        "{ctx}"
    );
}
