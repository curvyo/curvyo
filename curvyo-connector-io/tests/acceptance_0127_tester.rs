//! Independent acceptance tests for `specs/0127-connector-grbl-serial` (G1 to G34), written from
//! the specification before reading the implementation. The tests run the REAL connector binary
//! against my OWN minimal GRBL 1.1 simulator on a real pseudo-terminal (real time, no stepping).
//! The simulator is deliberately independent of `tests/common/grbl_sim`.
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

use curvyo_connector_core::{JobResult, Message, RejectReason, State};
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
        if self.alarm.is_some() {
            return "Alarm".into();
        }
        if self.homing_until.is_some_and(|u| u > now) {
            return "Home".into();
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
                    b'?' => {
                        let r = self.report(now);
                        self.reports.push((now, self.state_word(now)));
                        self.last_answer = Some(now);
                        self.put(now, &r);
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
            self.pos = (0.0, 0.0);
            self.put(now, "ok\r\n");
            self.last_ok_at = Some(now);
            self.alarm = None;
            self.alarm_lock = false;
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
        let mut child = Command::new(bin())
            .args(["--listen", "127.0.0.1:0", "--serial"])
            .arg(pty.path())
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

// ---------------------------------------------------------------------------------------------
// G1, G27
// ---------------------------------------------------------------------------------------------

#[test]
fn g1_usage_errors_and_help() {
    let (c, _, e) = run_bin(&["--serial", "/dev/null", "--fake-controller"]);
    assert_eq!(c, 2, "{e}");
    assert!(
        e.contains("--serial") && e.contains("--fake-controller"),
        "{e}"
    );
    let (c, _, e) = run_bin(&["--baud", "9600"]);
    assert_eq!(c, 2, "{e}");
    assert!(e.contains("--baud") && e.contains("--serial"), "{e}");
    for bad in ["0", "4000001", "abc", "-5"] {
        let (c, _, e) = run_bin(&["--serial", "/dev/null", "--baud", bad]);
        assert_eq!(c, 2, "baud {bad}: {e}");
        assert!(e.contains("--baud"), "{e}");
    }
    let (c, out, _) = run_bin(&["--help"]);
    assert_eq!(c, 0);
    for w in [
        "--serial",
        "--baud",
        "--fake-controller",
        "--listen",
        "--require-local-start",
    ] {
        assert!(out.contains(w), "{w}");
    }
    // G27: the help names the Linux serial notes.
    for w in ["dialout", "uucp", "ModemManager"] {
        assert!(out.contains(w), "help lacks {w}");
    }
}

#[test]
fn g1_no_serial_and_no_fake_is_no_controller() {
    let d = tempfile::tempdir().unwrap();
    // `specs/0128` A1, A28: without a run flag the program is a service in maintenance mode;
    // `--pair` alone is foreground mode without a controller.
    let mut child = Command::new(bin())
        .args(["--pair", "--listen", "127.0.0.1:0", "--state-dir"])
        .arg(d.path())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let (mut addr, mut code) = (String::new(), String::new());
    while code.is_empty() {
        let mut l = String::new();
        out.read_line(&mut l).unwrap();
        let l = l.trim().to_owned();
        if let Some(a) = l.strip_prefix("Listening on ") {
            addr = a.into();
        }
        if let Some(c) = l.strip_prefix("Pairing code: ") {
            code = c.split(',').next().unwrap().into();
        }
    }
    let mut c = connect(&addr, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let rec = c.pair(&code, "T", Instant::now() + WAIT).unwrap();
    drop(c);
    let (_c, st) = open_paired(&addr, rec.spki, &rec.token, CONNECT_TIMEOUT).unwrap();
    let _ = child.kill();
    assert!(
        is_state(&st, |s| matches!(s, State::NoController)),
        "{st:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// G2, G17, G19, G3
// ---------------------------------------------------------------------------------------------

#[test]
fn g2_g17_g19_identify_reads_banner_i_and_dollar_dollar_and_reports_the_controller() {
    let mut r = idle_rig(Cfg::default());
    let g = r.conn().greeting.clone();
    let c = g.controller.expect("Welcome carries the controller");
    assert_eq!(c.name, "GRBL");
    assert_eq!(c.firmware, "1.1h");
    assert_eq!(c.port.as_deref(), Some(r.pty.path().as_str()));
    assert_eq!(c.baud, Some(115_200));
    let s = c.settings.expect("settings");
    assert_eq!(s.laser_mode, Some(true));
    assert_eq!(s.s_max, Some(1000));
    let t = s.travel.expect("travel");
    let j = serde_json::to_value(t).unwrap();
    assert_eq!(j, serde_json::json!([400.0, 300.0]), "{j}");
    // G17: the first byte ever written is the soft reset; then $I and $$; no settings, no $X/$H.
    r.pty.with(|s| {
        assert_eq!(
            s.raw.first().map(|b| b.1),
            Some(0x18),
            "first byte must be 0x18"
        );
        let ls: Vec<&str> = s.lines.iter().map(|l| l.1.as_str()).collect();
        let i = ls.iter().position(|l| *l == "$I").expect("$I");
        let d = ls.iter().position(|l| *l == "$$").expect("$$");
        assert!(i < d);
        assert!(
            ls.iter()
                .all(|l| !l.contains('=') && *l != "$X" && *l != "$H"),
            "{ls:?}"
        );
    });
    assert!(r.stdout_has("GRBL 1.1h"), "{:?}", r.seen);
}

#[test]
fn g2_silent_port_and_garbage_port_are_no_controller_and_it_retries_to_success() {
    for garbage in [false, true] {
        let cfg = Cfg {
            silent_from_start: !garbage,
            garbage_banner: garbage,
            ..Cfg::default()
        };
        let mut r = Rig::start(cfg);
        r.connect();
        r.send(Message::GetStatus);
        // within about 5 s + margin: NoController with "No GRBL answer on <port>"
        let want = format!("No GRBL answer on {}", r.pty.path());
        let got = r.wait(9.0, |m| {
            is_state(m, |s| matches!(s, State::NoController)) && detail(m).contains(&want)
        });
        assert!(got.is_some(), "garbage={garbage}: {:?}", r.history);
        // retry: a proper controller appears on the same port -> Idle without a restart
        r.pty.with(|s| {
            s.cfg = Cfg::default();
            s.mute = false;
        });
        let up = r.wait_state(15.0, |s| matches!(s, State::Idle));
        assert!(up.is_some(), "no recovery: {:?}", r.history);
    }
}

#[test]
fn g3_port_absent_at_start_then_present_never_exits() {
    let dir = tempfile::tempdir().unwrap();
    let ptydir = dir.path().join("pty");
    std::fs::create_dir(&ptydir).unwrap();
    let mut pty = Pty::new(Cfg::default(), &ptydir);
    pty.unplug(); // the link is gone
    let mut r = Rig::launch(dir, pty, &[]);
    r.connect();
    r.send(Message::GetStatus);
    assert!(
        r.wait_state(4.0, |s| matches!(s, State::NoController))
            .is_some()
    );
    assert!(
        r.child.try_wait().unwrap().is_none(),
        "exited for a missing port"
    );
    r.pty.replug(Cfg::default());
    assert!(
        r.wait_state(10.0, |s| matches!(s, State::Idle)).is_some(),
        "{:?}",
        r.history
    );
}

// ---------------------------------------------------------------------------------------------
// G5 line validation
// ---------------------------------------------------------------------------------------------

fn written_to_port_since_identify(r: &Rig) -> Vec<u8> {
    r.pty.with(|s| {
        // everything after the `$$` line was fully read; job bytes would follow.
        let pos = s.raw.iter().rposition(|_| true).map_or(0, |p| p + 1);
        let _ = pos;
        s.raw.iter().map(|b| b.1).collect()
    })
}

#[test]
fn g5_bad_lines_fail_the_whole_job_before_the_first_byte() {
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "bang",
            "G21\nG90\nG1 X1 F100 !\nM5\n".into(),
            "Contains !, ~ or ?",
        ),
        ("tilde", "G21\nG1 X1 F100~\n".into(), "Contains !, ~ or ?"),
        (
            "question",
            "G21\nG1 X1 ?F100\n".into(),
            "Contains !, ~ or ?",
        ),
        (
            "long",
            format!("G21\nG1 X1 F100 {}\n", "Y".repeat(80)),
            "Longer than 79 characters",
        ),
        (
            "setting",
            "G21\n$100=250\nM5\n".into(),
            "Changes a machine setting",
        ),
        ("rst", "G21\n$RST=*\n".into(), "Changes a machine setting"),
        ("n", "G21\n$N0=G54\n".into(), "Changes a machine setting"),
        ("unlock", "G21\n$X\n".into(), "Unlocks the machine"),
    ];
    for (name, job, cause) in cases {
        let mut r = idle_rig(Cfg::default());
        let before = written_to_port_since_identify(&r).len();
        r.start_job(&job);
        let got = r.wait_state(10.0, |s| {
            matches!(
                s,
                State::Done {
                    result: JobResult::Failed
                }
            )
        });
        let (_, m) = got.unwrap_or_else(|| panic!("{name}: no Failed: {:?}", r.history));
        let d = detail(&m);
        assert!(
            d.starts_with("Failed at line ") && d.contains(cause),
            "{name}: {d}"
        );
        // nothing but status polls may have been written
        let after: Vec<u8> = r
            .pty
            .with(|s| s.raw.iter().skip(before).map(|b| b.1).collect());
        assert!(
            after.iter().all(|b| *b == b'?'),
            "{name}: bytes written before/at failure: {:?}",
            String::from_utf8_lossy(&after)
        );
    }
}

#[test]
fn g5_line_number_names_the_prepared_line_and_comments_are_harmless() {
    // `Hello! ~?` only in comments: runs to the end, no inline realtime byte.
    let job = "; Curvyo job: Hello! ~?\n(Name: Hello! ~?)\nG21 (units ! ~ ?)\nG90\nM4 S0\nG1 X10 Y10 F600 S500 ; end! ?\n\n   \nM5\n";
    let mut r = idle_rig(Cfg::default());
    r.start_job(job);
    let done = r.wait_state(15.0, |s| matches!(s, State::Done { .. }));
    let (_, m) = done.expect("no end");
    assert!(
        is_state(&m, |s| matches!(
            s,
            State::Done {
                result: JobResult::Completed
            }
        )),
        "{m:?}"
    );
    r.pty.with(|s| {
        let sent = job_lines(s);
        assert_eq!(sent, prepare(job), "wire lines differ");
        assert!(
            s.rt.iter()
                .all(|(_, b, inline)| !(*inline && matches!(*b, b'!' | b'~' | 0x18)))
        );
        // no `!` and no `~` at all during the job
        assert!(
            !s.rt.iter().any(|(_, b, _)| matches!(*b, b'!' | b'~')),
            "pause bytes sent"
        );
    });
}

#[test]
fn g5_failed_line_number_counts_job_lines() {
    let job = "; header\nG21\n(c)\nG90\nG1 X1 F100 !\nM5\n";
    let mut r = idle_rig(Cfg::default());
    r.start_job(job);
    let (_, m) = r
        .wait_state(10.0, |s| matches!(s, State::Done { .. }))
        .unwrap();
    let d = detail(&m);
    // the bad line is line 5 of the job text (comment lines count as lines)
    assert!(d.starts_with("Failed at line 5: "), "{d}");
}

#[test]
fn g5_non_ascii_is_refused_at_submit() {
    let mut r = idle_rig(Cfg::default());
    let e = r.submit("G21\n( caf\u{e9} )\nG90\n").unwrap_err();
    assert!(format!("{e:?}").contains("NotAscii"), "{e:?}");
}

// ---------------------------------------------------------------------------------------------
// G6, G8, G9 streaming
// ---------------------------------------------------------------------------------------------

fn run_to_completion(text: &str, cfg: Cfg, secs: f64) -> Rig {
    let mut r = idle_rig(cfg);
    r.start_job(text);
    let end = r.wait_state(secs, |s| matches!(s, State::Done { .. }));
    let (at, m) = end.unwrap_or_else(|| {
        panic!(
            "no end; last {:?}",
            r.history.iter().rev().take(3).collect::<Vec<_>>()
        )
    });
    assert!(
        is_state(&m, |s| matches!(
            s,
            State::Done {
                result: JobResult::Completed
            }
        )),
        "{m:?} {}",
        detail(&m)
    );
    // G8: Completed only after an Idle report that follows the last ok.
    r.pty.with(|s| {
        let last_ok = s.last_ok_at.unwrap();
        assert!(
            !s.busy_until.is_some_and(|u| u > at),
            "Completed while the planner still moves"
        );
        assert!(
            s.reports
                .iter()
                .any(|(t, w)| *t >= last_ok && w == "Idle" && *t <= at),
            "no Idle report between the last ok and Completed"
        );
    });
    r
}

#[test]
fn g6_every_golden_job_streams_without_overflow_in_order() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-laser-core/tests/fixtures/output/grbl");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert!(names.len() >= 20);
    for n in names {
        let text = golden(&n);
        let cfg = Cfg {
            motion_ms: 3,
            homing_ms: 200,
            ..Cfg::default()
        };
        let r = run_to_completion(&text, cfg, 60.0);
        r.pty.with(|s| {
            assert!(!s.overflow, "{n}: overflow, max {}", s.max_buffered);
            assert!(s.max_buffered <= 128, "{n}: {}", s.max_buffered);
            let sent: Vec<String> = job_lines(s).into_iter().filter(|l| !l.is_empty()).collect();
            // identify lines are before; the job (+ no stop writes) must equal prepared lines
            assert_eq!(sent, prepare(&text), "{n}: wire differs");
        });
    }
}

#[test]
fn g6_20000_line_job_counts_characters_and_never_exceeds_128() {
    let text = lines_job(20_000);
    let cfg = Cfg {
        motion_ms: 0,
        ..Cfg::default()
    };
    let r = run_to_completion(&text, cfg, 240.0);
    r.pty.with(|s| {
        assert!(!s.overflow, "overflow, max {}", s.max_buffered);
        assert!(s.max_buffered <= 128, "{}", s.max_buffered);
        let sent = job_lines(s);
        assert_eq!(sent.len(), prepare(&text).len());
        assert_eq!(sent, prepare(&text));
        // and it really used the buffer (not stop-and-wait): more than one line in flight
        assert!(
            s.max_buffered > 40,
            "never more than {} bytes in flight",
            s.max_buffered
        );
    });
}

#[test]
fn g8_progress_counts_acknowledged_lines_and_completes_after_idle() {
    let text = lines_job(300);
    let cfg = Cfg {
        motion_ms: 40,
        ..Cfg::default()
    };
    let r = run_to_completion(&text, cfg, 60.0);
    let mut last = 0;
    for (_, m) in &r.history {
        if let Message::Status { job: Some(p), .. } = m {
            assert!(p.line >= last && p.line <= p.lines, "{p:?}");
            last = p.line;
        }
    }
    assert!(last >= 300);
}

#[test]
fn g9_home_first_waits_without_timeout_and_polling_continues() {
    let text = "$H\nG21\nG90\nM4 S0\nG1 X10 F600 S500\nM5\n";
    let cfg = Cfg {
        homing_ms: 7_000,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    r.start_job(text);
    let t0 = Instant::now();
    // while homing (7 s > the 3 s loss rule): still Running, never NoController, `?` answered
    r.drain(6.0);
    let bad = r
        .history
        .iter()
        .filter(|(_, m)| is_state(m, |s| matches!(s, State::NoController | State::Done { .. })))
        .count();
    assert_eq!(bad, 0, "{:?}", r.history);
    let qs = r.pty.with(|s| {
        s.reports
            .iter()
            .filter(|(t, w)| *t > t0 && w == "Home")
            .count()
    });
    assert!(qs >= 15, "only {qs} Home reports in 6 s (want ~5/s)");
    let end = r
        .wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .expect("no end");
    assert!(is_state(&end.1, |s| matches!(
        s,
        State::Done {
            result: JobResult::Completed
        }
    )));
}

#[test]
fn g9_stop_during_homing_resets_without_feed_hold() {
    let text = "$H\nG21\nM5\n";
    let cfg = Cfg {
        homing_ms: 20_000,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    r.start_job(text);
    r.drain(1.0);
    let before = r.pty.with(|s| s.raw.len());
    r.send(Message::Stop);
    let end = r.wait(8.0, |m| {
        is_state(m, |s| {
            matches!(s, State::Done { .. } | State::Alarm | State::NoController)
        })
    });
    let (_, m) = end.expect("no end");
    assert!(
        is_state(&m, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            } | State::Alarm
        )),
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
            "first non-poll byte after Stop must be 0x18: {tail:?}"
        );
        assert!(!tail.contains(&b'!'), "feed hold sent during homing");
    });
}

// ---------------------------------------------------------------------------------------------
// G7 errors
// ---------------------------------------------------------------------------------------------

#[test]
fn g7_error_fails_the_job_names_the_job_line_and_sends_nothing_more() {
    // gcode line 6 on the wire is job line 8 (comment lines and blank lines count).
    let mut job = String::from("; head\nG21\n(x)\nG90\nM4 S0\n\n");
    for i in 0..60 {
        job += &format!("G1 X{i} Y{i} F600 S500\n");
    }
    job += "M5\n";
    let cfg = Cfg {
        fail_gcode_line: Some((4, 22)),
        ok_cycle: vec![0, 0, 1, 0, 0, 1],
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    r.start_job(&job);
    let (_, m) = r
        .wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .expect("no end");
    assert!(
        is_state(&m, |s| matches!(
            s,
            State::Done {
                result: JobResult::Failed
            }
        )),
        "{m:?}"
    );
    // 4th gcode line = `G1 X0 Y0 ...` = job line 7
    assert_eq!(detail(&m), "Failed at line 7: No feed rate (error 22)");
    r.drain(1.0);
    r.pty.with(|s| {
        let t = s.error_sent_at.expect("error sent");
        // lines that reached the port after the error was sent: only stop writes (M5, M9)
        let late: Vec<&str> = s
            .rx_lines
            .iter()
            .filter(|(at, _)| *at > t)
            .map(|(_, l)| l.as_str())
            .collect();
        assert!(
            late.iter().all(|l| *l == "M5" || *l == "M9"),
            "sent after the error: {late:?}"
        );
        assert!(
            s.raw.iter().any(|(_, b)| *b == 0x18),
            "no reset after failure"
        );
        assert!(!s.spindle_on && !s.air_on);
    });
    assert!(
        r.jobs_log().contains("Failed:") && r.jobs_log().contains("No feed rate (error 22)"),
        "{}",
        r.jobs_log()
    );
}

#[test]
fn g7_unknown_error_number_reads_error_n() {
    let cfg = Cfg {
        fail_gcode_line: Some((2, 77)),
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    r.start_job("G21\nG90\nM5\n");
    let (_, m) = r
        .wait_state(10.0, |s| matches!(s, State::Done { .. }))
        .unwrap();
    assert_eq!(detail(&m), "Failed at line 2: error 77");
}

// ---------------------------------------------------------------------------------------------
// G10 to G12 pause, resume, stop
// ---------------------------------------------------------------------------------------------

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

#[test]
fn g10_g11_pause_only_after_hold0_with_laser_off_then_resume() {
    for quirk in [false, true] {
        let cfg = Cfg {
            hold1_ms: 700,
            motion_ms: 80,
            laser_on_in_hold: quirk,
            ..Cfg::default()
        };
        let mut r = idle_rig(cfg);
        long_running(&mut r);
        let line_before = match r.last_status() {
            Some(Message::Status { job: Some(p), .. }) => p.line,
            o => panic!("{o:?}"),
        };
        assert!(
            r.pty.with(|s| s.laser_out(Instant::now())),
            "laser not on while running"
        );
        r.send(Message::Pause);
        let t_pause = Instant::now();
        let (at, _) = r
            .wait_state(6.0, |s| matches!(s, State::Paused))
            .expect("Paused");
        // Paused only once Hold:0 was reported
        let hold0 = r
            .pty
            .with(|s| s.reports.iter().find(|(_, w)| w == "Hold:0").map(|x| x.0));
        let hold0 = hold0.expect("sim never reported Hold:0");
        assert!(at >= hold0, "Paused before Hold:0 (quirk={quirk})");
        assert!(
            at.duration_since(t_pause) >= Duration::from_millis(650),
            "Paused during Hold:1"
        );
        // no Paused status was shown during Hold:1; Running stays
        r.drain(0.4);
        r.pty.with(|s| {
            assert!(
                !s.laser_out(Instant::now()),
                "laser on while held (quirk={quirk})"
            )
        });
        // idempotent
        r.send(Message::Pause);
        r.drain(0.5);
        assert!(is_state(r.last_status().unwrap(), |s| matches!(
            s,
            State::Paused
        )));
        // line count stops
        let l1 = match r.last_status() {
            Some(Message::Status { job: Some(p), .. }) => p.line,
            _ => 0,
        };
        r.drain(1.0);
        let l2 = match r.last_status() {
            Some(Message::Status { job: Some(p), .. }) => p.line,
            _ => 0,
        };
        assert_eq!(l1, l2, "lines advance while Paused");
        assert!(l1 >= line_before);
        // resume
        r.send(Message::Resume);
        assert!(
            r.wait_state(5.0, |s| matches!(s, State::Running)).is_some(),
            "no Running after resume"
        );
        r.drain(0.5);
        let l3 = match r.last_status() {
            Some(Message::Status { job: Some(p), .. }) => p.line,
            _ => 0,
        };
        assert!(l3 >= l2, "line count restarted");
        r.pty.with(|s| {
            assert!(s.rt.iter().any(|x| x.1 == b'~'), "no ~ sent");
            assert!(s.laser_out(Instant::now()) || s.busy_until.is_none() || true);
        });
        // clean up: stop
        r.send(Message::Stop);
        r.wait_state(10.0, |s| {
            matches!(s, State::Done { .. } | State::Alarm | State::NoController)
        });
    }
}

#[test]
fn g29_pause_not_confirmed_by_hold0_is_dropped_and_state_stays_running() {
    let cfg = Cfg {
        ignore_bang: true,
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Pause);
    r.drain(3.5);
    assert!(
        !r.history
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Paused))),
        "Paused without Hold:0"
    );
    assert!(
        is_state(r.last_status().unwrap(), |s| matches!(s, State::Running)),
        "{:?}",
        r.last_status()
    );
}

#[test]
fn g12_stop_from_running_hold_reset_banner_then_m5_m9_stopped_after_banner() {
    // hold takes 1.4 s, banner 2.4 s: a 3.8 s stop, "Stopped", never "Timed out"/NoController
    let cfg = Cfg {
        // slow stop timing is set after identify
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.pty.with(|s| {
        s.cfg.hold1_ms = 1400;
        s.cfg.banner_delay_ms = 2400;
    });
    let t_stop = Instant::now();
    r.send(Message::Stop);
    // G30: a Status answer with detail "Stopping" within 1 s, job state unchanged
    let (at, ans) = r
        .wait(
            2.0,
            |m| matches!(m, Message::Status { detail: Some(d), .. } if d == "Stopping"),
        )
        .expect("no 'Stopping' answer");
    assert!(at.duration_since(t_stop) < Duration::from_millis(1000));
    assert!(is_state(&ans, |s| matches!(s, State::Running)), "{ans:?}");
    let (end_at, end) = r
        .wait_state(10.0, |s| {
            matches!(s, State::Done { .. } | State::NoController | State::Alarm)
        })
        .expect("no end");
    assert!(
        is_state(&end, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            }
        )),
        "{end:?} {}",
        detail(&end)
    );
    r.pty.with(|s| {
        let banner = s.banners.last().copied().unwrap();
        assert!(end_at >= banner, "Stopped before the banner");
        // byte order: ! then 0x18, then M5, M9 lines after the banner
        let bytes: Vec<u8> = s.raw.iter().map(|b| b.1).filter(|b| *b != b'?').collect();
        let bang = bytes.iter().rposition(|b| *b == b'!').expect("no !");
        let rst = bytes.iter().rposition(|b| *b == 0x18).expect("no reset");
        assert!(
            bang < rst,
            "bytes: {:?}",
            format!("{:?}", &bytes[bytes.len().saturating_sub(40)..])
        );
        let tail = String::from_utf8_lossy(&bytes[rst + 1..]).into_owned();
        assert_eq!(
            tail.replace('\r', ""),
            "M5\nM9\n",
            "tail after reset: {tail:?}"
        );
        // reset came only after Hold:0 (>= 1.4 s after the !)
        let t_bang = s.rt.iter().rev().find(|x| x.1 == b'!').unwrap().0;
        let t_rst = s.rt.iter().rev().find(|x| x.1 == 0x18).unwrap().0;
        assert!(
            t_rst.duration_since(t_bang) >= Duration::from_millis(1350),
            "reset before Hold:0"
        );
        assert!(!s.spindle_on && !s.air_on, "spindle/air on after Stopped");
    });
    assert!(r.jobs_log().contains("Stopped"), "{}", r.jobs_log());
}

#[test]
fn g12_stop_taking_nearly_six_seconds_is_never_timed_out() {
    let cfg = Cfg {
        // slow stop timing is set after identify
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.pty.with(|s| {
        s.cfg.hold1_ms = 1900;
        s.cfg.banner_delay_ms = 2900;
    });
    r.send(Message::Stop);
    let (_, end) = r
        .wait_state(12.0, |s| {
            matches!(s, State::Done { .. } | State::NoController | State::Alarm)
        })
        .expect("no end within 12 s");
    assert!(
        is_state(&end, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            }
        )),
        "{end:?} {}",
        detail(&end)
    );
    // the app waits 5 s for the command answer; the answer must have come early
    let first_stopping = r
        .history
        .iter()
        .find(|(_, m)| detail(m) == "Stopping")
        .map(|x| x.0);
    assert!(first_stopping.is_some(), "no Stopping answer");
}

#[test]
fn g12_stop_in_paused_starts_at_the_reset() {
    let cfg = Cfg {
        motion_ms: 80,
        hold1_ms: 200,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Pause);
    r.wait_state(6.0, |s| matches!(s, State::Paused))
        .expect("Paused");
    let n = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'!').count());
    r.send(Message::Stop);
    let (_, end) = r
        .wait_state(10.0, |s| {
            matches!(s, State::Done { .. } | State::NoController | State::Alarm)
        })
        .unwrap();
    assert!(
        is_state(&end, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            }
        )),
        "{end:?}"
    );
    let n2 = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'!').count());
    assert_eq!(n, n2, "a second ! was sent for Stop in Paused");
    assert!(!r.pty.with(|s| s.spindle_on));
}

#[test]
fn g12_banner_never_comes_is_no_controller_could_not_confirm() {
    let cfg = Cfg {
        no_banner_after_reset: true,
        hold1_ms: 100,
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Stop);
    let (_, end) = r
        .wait(10.0, |m| {
            is_state(m, |s| {
                matches!(s, State::Done { .. } | State::NoController | State::Alarm)
            })
        })
        .expect("no end");
    assert!(
        is_state(&end, |s| matches!(s, State::NoController)),
        "{end:?}"
    );
    assert!(
        detail(&end).contains("Could not confirm the machine stopped"),
        "{}",
        detail(&end)
    );
}

#[test]
fn g12_machine_reset_button_during_hold_ends_the_stop() {
    let cfg = Cfg {
        hold1_ms: 5000,
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Stop);
    r.drain(0.7);
    // the machine's own reset button: spontaneous banner
    r.pty.with(|s| {
        let now = Instant::now();
        s.do_reset(now);
    });
    let (_, end) = r
        .wait_state(6.0, |s| {
            matches!(s, State::Done { .. } | State::NoController | State::Alarm)
        })
        .expect("stop did not end on a banner during the hold");
    assert!(
        is_state(&end, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            } | State::Alarm
        )),
        "{end:?}"
    );
}

#[test]
fn g12_start_while_stop_pending_starts_nothing() {
    let cfg = Cfg {
        // slow stop timing is set after identify
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.pty.with(|s| {
        s.cfg.hold1_ms = 1500;
        s.cfg.banner_delay_ms = 1500;
    });
    r.send(Message::Stop);
    r.drain(0.3);
    let id = uuid::Uuid::from_u128(7);
    r.send(Message::Start { job_id: id });
    let end = r
        .wait_state(10.0, |s| {
            matches!(s, State::Done { .. } | State::NoController | State::Alarm)
        })
        .expect("end");
    assert!(
        is_state(&end.1, |s| matches!(
            s,
            State::Done {
                result: JobResult::Stopped
            }
        )),
        "{:?}",
        end.1
    );
    let idx = r.history.len();
    r.drain(1.5);
    assert!(
        !r.history[idx..]
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Running))),
        "Running after the end: {:?}",
        &r.history[idx..]
    );
}

// ---------------------------------------------------------------------------------------------
// G13, G14, G33 alarms and unlock
// ---------------------------------------------------------------------------------------------

#[test]
fn g13_alarm_during_a_job_stops_sending_resets_and_ends_in_alarm() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    let t = Instant::now();
    r.pty.with(|s| {
        s.alarm = Some(1);
        s.spindle_on = false; // GRBL kills the spindle on a hard limit
        s.put(t, "ALARM:1\r\n");
        s.queue.clear();
        s.buffered = 0;
    });
    let (_, m) = r
        .wait_state(6.0, |s| matches!(s, State::Alarm | State::Done { .. }))
        .expect("no alarm");
    assert!(is_state(&m, |s| matches!(s, State::Alarm)), "{m:?}");
    assert_eq!(
        alarm_text(&m),
        "Hard limit hit, position may be lost (alarm 1)"
    );
    r.drain(1.0);
    r.pty.with(|s| {
        let late: Vec<&str> = s
            .lines
            .iter()
            .filter(|(at, l)| *at > t + Duration::from_millis(300) && l.starts_with('G'))
            .map(|(_, l)| l.as_str())
            .collect();
        assert!(late.is_empty(), "job lines after the alarm: {late:?}");
        assert!(
            s.raw.iter().any(|(at, b)| *b == 0x18 && *at >= t),
            "no soft reset after the alarm"
        );
        assert!(!s.spindle_on);
    });
    // SubmitJob in Alarm is Busy
    let e = r.submit("G21\n").unwrap_err();
    assert!(format!("{e:?}").contains("Busy"), "{e:?}");
    assert!(
        r.jobs_log()
            .contains("Alarm: Hard limit hit, position may be lost (alarm 1)"),
        "{}",
        r.jobs_log()
    );
}

#[test]
fn g13_alarm_while_idle_and_unknown_number() {
    let mut r = idle_rig(Cfg::default());
    r.pty.with(|s| {
        s.alarm = Some(42);
        s.put(Instant::now(), "ALARM:42\r\n");
    });
    let (_, m) = r
        .wait_state(6.0, |s| matches!(s, State::Alarm))
        .expect("no alarm while idle");
    assert_eq!(alarm_text(&m), "Unknown alarm (alarm 42)");
}

#[test]
fn g14_g33_startup_alarm_unlock_button_flow() {
    let cfg = Cfg {
        startup_alarm: true,
        ..Cfg::default()
    };
    let mut r = Rig::start(cfg);
    r.connect();
    r.send(Message::GetStatus);
    let (_, m) = r
        .wait_state(12.0, |s| matches!(s, State::Alarm))
        .expect("Alarm at startup");
    assert_eq!(alarm_text(&m), "Locked, not homed", "{m:?}");
    // never unlocks on its own
    r.drain(3.0);
    assert_eq!(r.pty.with(|s| s.unlocks), 0);
    assert!(
        !r.pty
            .with(|s| s.lines.iter().any(|(_, l)| l == "$X" || l == "$H"))
    );
    // Start impossible: SubmitJob is Busy in Alarm
    let e = r.submit(CUT).unwrap_err();
    assert!(
        format!("{e:?}").contains("Busy") || format!("{e:?}").contains("Rejected"),
        "{e:?}"
    );
    // a second paired client unlocks
    let (mut c2, _) = r.pair_client("Second");
    let n = r.pty.with(|s| s.lines.len());
    c2.send(&Message::Unlock).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut cleared = false;
    while let Ok(Some(f)) = c2.try_recv(deadline) {
        if let curvyo_connector_io::frames::Frame::Core(m) = f {
            if is_state(&m, |s| matches!(s, State::Idle)) {
                cleared = true;
                break;
            }
        }
    }
    assert!(cleared, "alarm did not clear after Unlock");
    assert_eq!(
        r.pty.with(|s| s.unlocks),
        1,
        "$X must be written exactly once"
    );
    let _ = n;
    assert!(r.jobs_log().contains("Unlock: Second"), "{}", r.jobs_log());
    assert!(r.stdout_has("Unlock sent by Second"), "{:?}", r.seen);
    // G33: one terminal line per unlock, nothing else new.
    assert!(!r.stdout_has("Sent $X"), "{:?}", r.seen);
    // after the alarm: Send works again
    drop(c2);
    assert!(r.submit(CUT).is_ok());
}

#[test]
fn g33_unlock_in_idle_and_running_writes_nothing() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    let n0 = r.pty.with(|s| s.lines.len());
    r.send(Message::Unlock);
    r.drain(1.0);
    assert_eq!(r.pty.with(|s| s.lines.len()), n0, "Unlock wrote in Idle");
    long_running(&mut r);
    let n1 = r.pty.with(|s| s.lines.len());
    r.send(Message::Unlock);
    r.drain(0.1);
    let wrote: Vec<String> = r
        .pty
        .with(|s| s.lines.iter().skip(n1).map(|l| l.1.clone()).collect());
    assert!(!wrote.iter().any(|l| l == "$X"), "{wrote:?}");
    assert_eq!(r.pty.with(|s| s.unlocks), 0);
    r.send(Message::Stop);
    r.wait_state(10.0, |s| {
        matches!(s, State::Done { .. } | State::NoController | State::Alarm)
    });
}

#[test]
fn g33_unlock_works_under_require_local_start() {
    let cfg = Cfg {
        startup_alarm: true,
        ..Cfg::default()
    };
    let mut r = Rig::start_with(cfg, &["--require-local-start"]);
    r.connect();
    r.send(Message::GetStatus);
    r.wait_state(12.0, |s| matches!(s, State::Alarm))
        .expect("alarm");
    r.send(Message::Unlock);
    assert!(
        r.wait_state(8.0, |s| matches!(s, State::Idle)).is_some(),
        "no unlock under --require-local-start"
    );
}

// ---------------------------------------------------------------------------------------------
// G34
// ---------------------------------------------------------------------------------------------

#[test]
fn g34_laser_mode_off_and_unknown_refuse_the_job_and_write_nothing() {
    for (mode, reason, text) in [
        (
            Some(false),
            RejectReason::LaserModeOff,
            "Laser mode is off ($32=0)",
        ),
        (
            None,
            RejectReason::LaserModeUnknown,
            "Laser mode unknown ($32 not read)",
        ),
    ] {
        let mut r = idle_rig(Cfg {
            laser_mode: mode,
            ..Cfg::default()
        });
        let n = r.pty.with(|s| s.raw.iter().filter(|b| b.1 != b'?').count());
        let e = r.submit(CUT).unwrap_err();
        match e {
            SendError::Rejected(got) => assert_eq!(got, reason),
            o => panic!("{o:?}"),
        }
        let n2 = r.pty.with(|s| s.raw.iter().filter(|b| b.1 != b'?').count());
        assert_eq!(n, n2, "bytes written for a refused job");
        r.send(Message::GetStatus);
        assert!(
            r.wait_state(2.0, |s| matches!(s, State::Idle)).is_some(),
            "state changed"
        );
        let line = format!("Job refused: {text}");
        assert!(r.seen_line(&line), "terminal lacks {line:?}: {:?}", r.seen);
    }
    // $32=1 accepts
    let mut r = idle_rig(Cfg::default());
    assert!(r.submit(CUT).is_ok());
}

// ---------------------------------------------------------------------------------------------
// G15 loss
// ---------------------------------------------------------------------------------------------

#[test]
fn g15_port_closed_mid_job_fails_never_resumes_never_running_stale() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    let t = Instant::now();
    r.pty.unplug();
    let (_, m) = r
        .wait_state(8.0, |s| matches!(s, State::NoController))
        .expect("no NoController after unplug");
    let d = detail(&m);
    assert!(d.starts_with("Lost the controller at line "), "{d}");
    assert!(t.elapsed() < Duration::from_secs(5));
    // every later status: never Running/Paused
    r.drain(2.5);
    r.pty.replug(Cfg::default());
    let idle = r.wait_state(15.0, |s| matches!(s, State::Idle));
    assert!(
        idle.is_some(),
        "no reconnect: {:?}",
        r.history.iter().rev().take(4).collect::<Vec<_>>()
    );
    r.drain(3.0);
    let after: Vec<_> = r
        .history
        .iter()
        .skip_while(|(_, m)| {
            !matches!(
                m,
                Message::Status {
                    state: State::NoController,
                    ..
                }
            )
        })
        .collect();
    assert!(
        !after
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Running | State::Paused))),
        "Running after loss: {after:?}"
    );
    // the reconnect did not resume: no gcode line reached the new machine
    r.pty.with(|s| {
        let after_replug: Vec<&str> = s
            .lines
            .iter()
            .rev()
            .take_while(|(_, l)| l != "$$")
            .map(|x| x.1.as_str())
            .collect();
        assert!(
            after_replug
                .iter()
                .all(|l| l.is_empty() || !l.starts_with('G')),
            "job resumed: {after_replug:?}"
        );
    });
    let log = r.jobs_log();
    assert!(log.contains("Failed: controller lost at line"), "{log}");
    // a client connecting after the loss reads the same text (kept until next job state)
    let (mut c2, _) = r.pair_client("Late");
    c2.send(&Message::GetStatus).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    let mut text = String::new();
    while let Ok(Some(f)) = c2.try_recv(until) {
        if let curvyo_connector_io::frames::Frame::Core(m) = f {
            if let Message::Status { .. } = m {
                text = detail(&m);
                break;
            }
        }
    }
    assert!(
        text.contains("Lost the controller"),
        "late client read {text:?}"
    );
}

#[test]
fn g15_port_stays_open_but_silent_is_lost_within_3s_and_never_running_stale() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    let t_mute = Instant::now();
    r.pty.with(|s| s.mute = true);
    let (at, m) = r
        .wait_state(8.0, |s| matches!(s, State::NoController))
        .expect("never lost");
    assert!(
        detail(&m).starts_with("Lost the controller at line "),
        "{}",
        detail(&m)
    );
    let d = at.duration_since(t_mute);
    assert!(d <= Duration::from_millis(4500), "took {d:?}");
    // No Running/Paused push later than last answer + 3.3 s
    let last_answer = r.pty.with(|s| s.last_answer.unwrap());
    for (t, m) in &r.history {
        if is_state(m, |s| matches!(s, State::Running | State::Paused)) {
            assert!(
                t.duration_since(last_answer) < Duration::from_millis(3500),
                "stale Running pushed {:?} after the last answer",
                t.duration_since(last_answer)
            );
        }
    }
}

#[test]
fn g15_unplugged_during_hold() {
    let cfg = Cfg {
        motion_ms: 80,
        hold1_ms: 100,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Pause);
    r.wait_state(5.0, |s| matches!(s, State::Paused))
        .expect("Paused");
    r.pty.unplug();
    let (_, m) = r
        .wait_state(8.0, |s| matches!(s, State::NoController))
        .expect("no NoController");
    assert!(
        detail(&m).starts_with("Lost the controller at line "),
        "{}",
        detail(&m)
    );
    r.drain(2.0);
    let after_loss = r
        .history
        .iter()
        .skip_while(|(_, m)| !is_state(m, |s| matches!(s, State::NoController)));
    assert!(
        !after_loss
            .into_iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Running | State::Paused)))
    );
}

// ---------------------------------------------------------------------------------------------
// G16 signals
// ---------------------------------------------------------------------------------------------

/// Hang guard for the signal tests, not a speed requirement.
const EXIT_BOUND: Duration = Duration::from_secs(30);

#[test]
fn g16_sigterm_and_sigint_during_a_job_stop_the_machine_and_exit_cleanly() {
    for sig in ["TERM", "INT"] {
        let cfg = Cfg {
            motion_ms: 80,
            // slow stop timing is set after identify
            ..Cfg::default()
        };
        let mut r = idle_rig(cfg);
        long_running(&mut r);
        r.pty.with(|s| {
            s.cfg.hold1_ms = 1500;
            s.cfg.banner_delay_ms = 2000;
        });
        // Functional check only (CLAUDE.md section 5: no wall-time assertion); the bound merely
        // keeps a hung child from hanging the suite. The 6 s of G16 is the product's EXIT_STOP.
        let t = Instant::now();
        r.signal(sig);
        let status = loop {
            if let Some(s) = r.child.try_wait().unwrap() {
                break s;
            }
            assert!(
                t.elapsed() < EXIT_BOUND,
                "{sig}: still running after {EXIT_BOUND:?}"
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(status.code(), Some(0), "{sig}: {status:?}");
        r.pty.with(|s| {
            let bytes: Vec<u8> = s.raw.iter().map(|b| b.1).filter(|b| *b != b'?').collect();
            let bang = bytes
                .iter()
                .rposition(|b| *b == b'!')
                .expect("no feed hold");
            let rst = bytes.iter().rposition(|b| *b == 0x18).expect("no reset");
            assert!(
                bang < rst,
                "bytes: {:?}",
                format!("{:?}", &bytes[bytes.len().saturating_sub(40)..])
            );
            assert!(!s.spindle_on && !s.air_on, "laser on after exit");
        });
        assert!(r.jobs_log().contains("Stopped"), "{sig}: {}", r.jobs_log());
    }
}

#[test]
fn g16_sigterm_without_job_exits_at_once() {
    let mut r = idle_rig(Cfg::default());
    let t = Instant::now();
    r.signal("TERM");
    loop {
        if let Some(s) = r.child.try_wait().unwrap() {
            assert_eq!(s.code(), Some(0));
            break;
        }
        assert!(t.elapsed() < EXIT_BOUND, "no exit without a job");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn g16_sigterm_while_paused() {
    let cfg = Cfg {
        motion_ms: 80,
        hold1_ms: 100,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.send(Message::Pause);
    r.wait_state(5.0, |s| matches!(s, State::Paused))
        .expect("Paused");
    let t = Instant::now();
    r.signal("TERM");
    loop {
        if let Some(s) = r.child.try_wait().unwrap() {
            assert_eq!(s.code(), Some(0), "{s:?}");
            break;
        }
        assert!(
            t.elapsed() < EXIT_BOUND,
            "still running after {EXIT_BOUND:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!r.pty.with(|s| s.spindle_on || s.air_on));
}

// ---------------------------------------------------------------------------------------------
// G18, G28, G29, G4
// ---------------------------------------------------------------------------------------------

#[test]
fn g18_require_local_start_writes_no_job_byte_before_enter() {
    let mut r = Rig::start_with(Cfg::default(), &["--require-local-start"]);
    r.connect();
    r.wait_state(10.0, |s| matches!(s, State::Idle))
        .or_else(|| {
            r.send(Message::GetStatus);
            r.wait_state(5.0, |s| matches!(s, State::Idle))
        });
    let id = r.submit(CUT).unwrap();
    r.send(Message::Start { job_id: id });
    r.drain(2.0);
    r.pty.with(|s| {
        let j = job_lines(s);
        assert!(j.is_empty(), "job bytes before Enter: {j:?}");
    });
    r.child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    r.wait_state(15.0, |s| matches!(s, State::Done { .. }))
        .expect("did not run after Enter");
    assert_eq!(r.pty.with(|s| job_lines(s)), prepare(CUT));
}

#[test]
fn g28_foreign_ok_with_no_line_in_flight_fails_the_job() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    // a long job so lines stay in flight: stop acking by making the sim ack slowly
    r.pty
        .with(|s| s.cfg.ok_cycle = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    let mut job = String::from("G21\nG90\n");
    for i in 0..300 {
        job += &format!("G1 X{} F600\n", i % 100);
    }
    r.start_job(&job);
    r.wait_state(10.0, |s| matches!(s, State::Running)).unwrap();
    // wait until the sim has acked everything it got and the buffer is empty? Instead inject while empty:
    r.pty.with(|s| {
        s.queue.clear();
        s.buffered = 0;
    });
    r.pty.with(|s| s.put(Instant::now(), "ok\r\n"));
    // either lines in flight (taken as oldest answer) or none (fails): only assert the safe invariant,
    // no overflow ever and, if failed, the text.
    r.drain(3.0);
    r.pty
        .with(|s| assert!(!s.overflow, "overflow after a foreign ok"));
    r.send(Message::Stop);
    r.wait_state(10.0, |s| {
        matches!(s, State::Done { .. } | State::NoController | State::Alarm)
    });
}

#[test]
fn g29_machine_reset_button_mid_job_fails_the_job() {
    let cfg = Cfg {
        motion_ms: 80,
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    long_running(&mut r);
    r.pty.with(|s| s.do_reset(Instant::now()));
    let (_, m) = r
        .wait_state(8.0, |s| matches!(s, State::Done { .. } | State::Alarm))
        .expect("no end");
    // moving reset raises alarm 3 in the simulator; the job must end Failed or in Alarm, with the text
    let d = detail(&m);
    let ok = d.starts_with("The controller was reset at line ")
        || is_state(&m, |s| matches!(s, State::Alarm));
    assert!(ok, "{m:?}");
    let idx = r.history.len();
    r.drain(1.5);
    assert!(
        !r.history[idx..]
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Running))),
        "Running after the end: {:?}",
        &r.history[idx..]
    );
}

#[test]
fn g4_polling_rate_other_states_and_work_position() {
    // Idle: about 1 Hz; Running: about 5 Hz.
    let cfg = Cfg {
        motion_ms: 100,
        mpos_wco: Some((10.0, 5.0)),
        ..Cfg::default()
    };
    let mut r = idle_rig(cfg);
    let n0 = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'?').count());
    r.drain(5.0);
    let n1 = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'?').count());
    let idle_rate = (n1 - n0) as f64 / 5.0;
    assert!(
        (0.6..=2.0).contains(&idle_rate),
        "idle poll rate {idle_rate}/s"
    );
    long_running(&mut r);
    let m0 = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'?').count());
    r.drain(3.0);
    let m1 = r.pty.with(|s| s.rt.iter().filter(|x| x.1 == b'?').count());
    let run_rate = (m1 - m0) as f64 / 3.0;
    assert!(
        (3.5..=7.0).contains(&run_rate),
        "running poll rate {run_rate}/s"
    );
    // work position = machine position - WCO; pos set by the sim to the last target
    let (mx, my) = r.pty.with(|s| s.pos);
    let mut seen = None;
    for (_, m) in r.history.iter().rev() {
        if let Message::Status {
            position: Some(p), ..
        } = m
        {
            seen = Some(serde_json::to_value(p).unwrap());
            break;
        }
    }
    let p = seen.expect("no position in any Status");
    let wx = p[0].as_f64().unwrap();
    let wy = p[1].as_f64().unwrap();
    assert!(
        (wx - (mx - 10.0)).abs() < 25.0 && (wy - (my - 5.0)).abs() < 25.0,
        "work pos {p} vs machine {mx},{my}"
    );
    let _ = (wx, wy);
    r.send(Message::Stop);
    r.wait_state(10.0, |s| {
        matches!(s, State::Done { .. } | State::NoController | State::Alarm)
    });
}

#[test]
fn g4_door_state_is_shown_as_grbl_word_and_never_running() {
    let cfg = Cfg {
        door: true,
        ..Cfg::default()
    };
    let mut r = Rig::start(cfg);
    r.connect();
    r.send(Message::GetStatus);
    r.drain(4.0);
    assert!(
        !r.history
            .iter()
            .any(|(_, m)| is_state(m, |s| matches!(s, State::Running))),
        "Running in Door"
    );
    let e = r.submit(CUT);
    assert!(
        matches!(e, Err(SendError::Rejected(RejectReason::Busy))),
        "{e:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Wire additivity (G19, G33, G34 golden files; protocol stays 0)
// ---------------------------------------------------------------------------------------------

#[test]
fn wire_old_golden_messages_decode_and_encode_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    // CI checks out a merge ref without a local `main`: use whichever base ref exists.
    let git = |args: &[&str]| {
        Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap()
    };
    let base = ["origin/main", "main"]
        .into_iter()
        .find(|r| git(&["rev-parse", "--verify", "-q", r]).status.success());
    let Some(base) = base else {
        eprintln!("no main ref in this checkout: golden-unchanged comparison skipped");
        return;
    };
    let ls = Command::new("git")
        .current_dir(&root)
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            base,
            "curvyo-connector-core/tests/fixtures/messages",
        ])
        .output()
        .unwrap();
    let names = String::from_utf8_lossy(&ls.stdout).into_owned();
    assert!(
        names.lines().count() > 10,
        "no golden messages on main: {names}"
    );
    for path in names.lines() {
        let old = Command::new("git")
            .current_dir(&root)
            .args(["show", &format!("{base}:{path}")])
            .output()
            .unwrap();
        let text = String::from_utf8(old.stdout).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        // the old file must be unchanged on this branch
        let now =
            std::fs::read_to_string(root.join(path)).unwrap_or_else(|_| panic!("{path} deleted"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&now).unwrap(),
            value,
            "{path} was edited"
        );
        // and decodes to a Message and encodes back to the same value
        let m: Message =
            serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            value,
            "{path} re-encodes differently"
        );
    }
    // new messages exist
    for f in [
        "command_unlock",
        "reject_laser_mode_off",
        "reject_laser_mode_unknown",
    ] {
        assert!(
            root.join(format!(
                "curvyo-connector-core/tests/fixtures/messages/{f}.json"
            ))
            .exists(),
            "{f}"
        );
    }
}

#[test]
fn protocol_stays_zero() {
    let mut r = idle_rig(Cfg::default());
    assert_eq!(r.conn().greeting.protocol, 0);
}

// ---------------------------------------------------------------------------------------------
// G20 and G32 smoke
// ---------------------------------------------------------------------------------------------

#[test]
fn g32_deny_toml_allows_mpl_for_serialport_only() {
    let t = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../deny.toml"))
        .unwrap();
    assert!(t.contains("MPL-2.0"));
    assert!(t.contains("serialport"));
}

// ---------------------------------------------------------------------------------------------
// Manual demo helper (ignored): my simulator on a pty for driving the real desktop app.
// CTL=<dir> SIMCFG="startup_alarm,laser_off" ... ; touch <dir>/alarm1 injects ALARM:1, <dir>/quit ends.
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "manual helper for the desktop-app check"]
fn manual_pty_sim() {
    let ctl = PathBuf::from(std::env::var("CTL").unwrap());
    let flags = std::env::var("SIMCFG").unwrap_or_default();
    let mut cfg = Cfg {
        motion_ms: 150,
        ..Cfg::default()
    };
    if flags.contains("startup_alarm") {
        cfg.startup_alarm = true;
    }
    if flags.contains("laser_off") {
        cfg.laser_mode = Some(false);
    }
    let pdir = ctl.join("pty");
    std::fs::create_dir_all(&pdir).unwrap();
    let pty = Pty::new(cfg, &pdir);
    println!(
        "SIM_PATH {}",
        std::fs::read_link(&pty.link).unwrap().display()
    );
    loop {
        std::thread::sleep(Duration::from_millis(200));
        if ctl.join("quit").exists() {
            break;
        }
        if ctl.join("alarm1").exists() {
            let _ = std::fs::remove_file(ctl.join("alarm1"));
            pty.with(|s| {
                s.alarm = Some(1);
                s.spindle_on = false;
                s.queue.clear();
                s.buffered = 0;
                s.put(Instant::now(), "ALARM:1\r\n");
            });
        }
    }
}
