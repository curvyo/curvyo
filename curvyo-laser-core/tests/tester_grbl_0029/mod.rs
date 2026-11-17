//! The tester's own GRBL reader for `specs/0029-laser-job-preview-and-output` (criterion 31):
//! written from the specification only, independent of the generator and of the implementer's
//! reader. It accepts exactly the dialect criteria 24 to 26 and 38 allow and refuses anything
//! else (arcs, unknown words, wrong decimals, CR line ends, a burn with no feed), and reads back
//! the moves in whole micrometres, the burn and travel lengths, the extent and the burn runs.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::nonminimal_bool,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

/// What one straight move of the file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `G1` with the laser on and S above 0.
    Burn,
    /// `G0`.
    Travel,
    /// `G1` in X/Y with the laser off (overscan run-out).
    Off,
    /// `G1` in Z only.
    Z,
}

/// One move, in machine micrometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seg {
    pub from: (i64, i64),
    pub to: (i64, i64),
    pub z: i64,
    pub kind: Kind,
    /// The modal S when the move ran.
    pub s: u32,
    /// The modal F when the move ran (0 = none set).
    pub f: u32,
    /// 1-based line number.
    pub line: usize,
}

impl Seg {
    pub fn length_mm(&self) -> f64 {
        let dx = (self.to.0 - self.from.0) as f64 / 1000.0;
        let dy = (self.to.1 - self.from.1) as f64 / 1000.0;
        dx.hypot(dy)
    }
}

/// A run of consecutive burn moves at one S and F (a burn path).
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub s: u32,
    pub f: u32,
    pub points: Vec<(i64, i64)>,
}

/// The laser mode the file starts the spindle in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    M3,
    M4,
}

/// A read GRBL file.
#[derive(Debug, Clone)]
pub struct Program {
    /// The comment lines before the first command, without `; `.
    pub header: Vec<String>,
    /// The command lines in order (no comments).
    pub commands: Vec<String>,
    pub segs: Vec<Seg>,
    pub mode: Mode,
    pub relative: bool,
    pub homes: bool,
    /// Index into `commands` of `M8` / `M9`, if any.
    pub air_on: Option<usize>,
    pub air_off: Option<usize>,
    /// The head's last position.
    pub end: (i64, i64),
}

fn um(text: &str, line: usize) -> Result<i64, String> {
    let (neg, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    let (int, frac) = digits
        .split_once('.')
        .ok_or_else(|| format!("line {line}: coordinate {text} has no 3 decimals"))?;
    if int.is_empty()
        || frac.len() != 3
        || !int.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!(
            "line {line}: coordinate {text} is not <int>.<3 digits>"
        ));
    }
    if int.len() > 1 && int.starts_with('0') {
        return Err(format!("line {line}: leading zero in {text}"));
    }
    let v = int.parse::<i64>().unwrap() * 1000 + frac.parse::<i64>().unwrap();
    if neg && v == 0 {
        return Err(format!("line {line}: -0.000 written"));
    }
    Ok(if neg { -v } else { v })
}

fn whole(text: &str, line: usize) -> Result<u32, String> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("line {line}: {text} is not a whole number"));
    }
    text.parse::<u32>()
        .map_err(|e| format!("line {line}: {text}: {e}"))
}

/// Reads `bytes` as GRBL G-code of criterion 24, or says what is wrong.
pub fn read(bytes: &[u8]) -> Result<Program, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    if !text.is_ascii() {
        return Err("not ASCII".into());
    }
    if text.contains('\r') {
        return Err("CR in line ends".into());
    }
    if !text.ends_with('\n') {
        return Err("the last line has no LF".into());
    }
    let mut header = Vec::new();
    let mut commands: Vec<String> = Vec::new();
    let mut segs = Vec::new();
    let mut seen_command = false;
    let (mut units, mut distance, mut plane) = (false, None::<bool>, false);
    let mut mode = None::<Mode>;
    let mut spindle_on = false;
    let mut s: u32 = 0;
    let mut f: u32 = 0;
    let (mut x, mut y, mut z) = (0_i64, 0_i64, 0_i64);
    let mut homes = false;
    let (mut air_on, mut air_off) = (None, None);
    let mut started = false; // M5 + M3/M4 S0 seen
    let mut last_was_m5 = false;
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        if raw.is_empty() {
            return Err(format!("line {n}: empty line"));
        }
        if raw != raw.trim() {
            return Err(format!("line {n}: stray whitespace in {raw:?}"));
        }
        if let Some(c) = raw.strip_prefix(';') {
            if seen_command {
                return Err(format!("line {n}: a comment after the header"));
            }
            header.push(c.trim_start().to_owned());
            continue;
        }
        seen_command = true;
        let idx = commands.len();
        commands.push(raw.to_owned());
        let words: Vec<&str> = raw.split(' ').collect();
        let head = words[0];
        let mut m5_now = false;
        match head {
            "$H" => {
                if idx != 0 {
                    return Err(format!("line {n}: $H is not the first command"));
                }
                homes = true;
            }
            "G21" => units = true,
            "G90" | "G91" => {
                if distance.is_some() {
                    return Err(format!("line {n}: distance mode set twice"));
                }
                distance = Some(head == "G91");
            }
            "G17" => plane = true,
            "M5" => {
                spindle_on = false;
                m5_now = true;
            }
            "M3" | "M4" => {
                if words.len() != 2 {
                    return Err(format!("line {n}: {raw}"));
                }
                let v = whole(
                    words[1]
                        .strip_prefix('S')
                        .ok_or(format!("line {n}: {raw}"))?,
                    n,
                )?;
                let m = if head == "M3" { Mode::M3 } else { Mode::M4 };
                if !started {
                    if v != 0 || !last_was_m5 {
                        return Err(format!("line {n}: the start is not M5 then {head} S0"));
                    }
                    if !(units && distance.is_some() && plane) {
                        return Err(format!("line {n}: laser start before G21/G90|G91/G17"));
                    }
                    started = true;
                    mode = Some(m);
                } else if mode != Some(m) {
                    return Err(format!("line {n}: laser mode changes to {head}"));
                }
                if m == Mode::M4 && started && idx > 0 && v != 0 {
                    return Err(format!("line {n}: M4 with S≠0 after the start"));
                }
                spindle_on = true;
                s = v;
            }
            "M8" => {
                if air_on.is_some() {
                    return Err(format!("line {n}: M8 twice"));
                }
                air_on = Some(idx);
            }
            "M9" => {
                if air_off.is_some() {
                    return Err(format!("line {n}: M9 twice"));
                }
                air_off = Some(idx);
            }
            "G0" | "G1" => {
                if !started {
                    return Err(format!("line {n}: a move before the laser start"));
                }
                let rel = distance.unwrap();
                let (mut nx, mut ny, mut nz) = (None, None, None);
                let mut nf = None;
                let mut ns = None;
                for w in &words[1..] {
                    let (letter, value) = w.split_at(1);
                    match letter {
                        "X" if nx.is_none() => nx = Some(um(value, n)?),
                        "Y" if ny.is_none() => ny = Some(um(value, n)?),
                        "Z" if nz.is_none() && head == "G1" => nz = Some(um(value, n)?),
                        "F" if nf.is_none() && head == "G1" => nf = Some(whole(value, n)?),
                        "S" if ns.is_none() && head == "G1" => ns = Some(whole(value, n)?),
                        _ => return Err(format!("line {n}: word {w} not allowed in {raw}")),
                    }
                }
                if let Some(v) = nf {
                    if v == f && nz.is_none() {
                        return Err(format!("line {n}: F{v} repeated though unchanged"));
                    }
                    f = v;
                }
                if let Some(v) = ns {
                    if v == s {
                        return Err(format!("line {n}: S{v} repeated though unchanged"));
                    }
                    s = v;
                }
                let from = (x, y);
                if let Some(dz) = nz {
                    if nx.is_some() || ny.is_some() {
                        return Err(format!("line {n}: Z combined with X/Y"));
                    }
                    if spindle_on && s > 0 {
                        return Err(format!("line {n}: Z move with the laser on"));
                    }
                    if f == 0 {
                        return Err(format!("line {n}: G1 Z with no feed"));
                    }
                    z = if rel { z + dz } else { dz };
                    segs.push(Seg {
                        from,
                        to: from,
                        z,
                        kind: Kind::Z,
                        s,
                        f,
                        line: n,
                    });
                } else {
                    if nx.is_none() && ny.is_none() {
                        return Err(format!("line {n}: a move with no axis"));
                    }
                    if rel {
                        x += nx.unwrap_or(0);
                        y += ny.unwrap_or(0);
                    } else {
                        x = nx.unwrap_or(x);
                        y = ny.unwrap_or(y);
                    }
                    let kind = if head == "G0" {
                        if mode == Some(Mode::M3) && spindle_on {
                            return Err(format!("line {n}: G0 in M3 without a preceding M5"));
                        }
                        Kind::Travel
                    } else if spindle_on && s > 0 {
                        if f == 0 {
                            return Err(format!("line {n}: a burn with no feed"));
                        }
                        Kind::Burn
                    } else {
                        if f == 0 {
                            return Err(format!("line {n}: G1 with no feed"));
                        }
                        Kind::Off
                    };
                    segs.push(Seg {
                        from,
                        to: (x, y),
                        z,
                        kind,
                        s,
                        f,
                        line: n,
                    });
                }
            }
            _ => return Err(format!("line {n}: unknown command {raw:?}")),
        }
        last_was_m5 = m5_now;
    }
    let mode = mode.ok_or("no laser start")?;
    // The end: M5, M9 when air is on, then a rapid back to the origin.
    let last = commands.len();
    if last < 2 {
        return Err("too short".into());
    }
    // The end may leave out the rapid back when the head already stands at the start (a closed
    // path in relative mode ends where it began).
    let back = commands[last - 1].starts_with("G0 ");
    if !back && (x, y) != (0, 0) {
        return Err(format!(
            "the file does not end with a G0 back, ends with {}",
            commands[last - 1]
        ));
    }
    let end_g0 = usize::from(back);
    let air = usize::from(air_on.is_some());
    let before = last - 1 - end_g0 - air;
    if commands[before] != "M5" {
        return Err(format!("no M5 before the end: {}", commands[before]));
    }
    if air_on.is_some() {
        if air_off != Some(last - 1 - end_g0) {
            return Err("M9 is not between the end M5 and G0".into());
        }
    } else if air_off.is_some() {
        return Err("M9 without M8".into());
    }
    if z != 0 {
        return Err(format!("Z ends at {z} µm, not 0"));
    }
    Ok(Program {
        header,
        commands,
        segs,
        mode,
        relative: distance.unwrap_or(false),
        homes,
        air_on,
        air_off,
        end: (x, y),
    })
}

impl Program {
    pub fn burn_mm(&self) -> f64 {
        self.segs
            .iter()
            .filter(|s| s.kind == Kind::Burn)
            .map(Seg::length_mm)
            .sum()
    }
    pub fn travel_mm(&self) -> f64 {
        self.segs
            .iter()
            .filter(|s| s.kind == Kind::Travel)
            .map(Seg::length_mm)
            .sum()
    }
    pub fn off_mm(&self) -> f64 {
        self.segs
            .iter()
            .filter(|s| s.kind == Kind::Off)
            .map(Seg::length_mm)
            .sum()
    }
    /// The box of every burn point `(min x, min y, max x, max y)` in µm.
    pub fn burn_box(&self) -> Option<(i64, i64, i64, i64)> {
        let mut b: Option<(i64, i64, i64, i64)> = None;
        for s in self.segs.iter().filter(|s| s.kind == Kind::Burn) {
            for p in [s.from, s.to] {
                b = Some(match b {
                    None => (p.0, p.1, p.0, p.1),
                    Some((a, c, d, e)) => (a.min(p.0), c.min(p.1), d.max(p.0), e.max(p.1)),
                });
            }
        }
        b
    }
    /// The box of every point the head visits.
    pub fn all_box(&self) -> (i64, i64, i64, i64) {
        let mut b = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for s in &self.segs {
            for p in [s.from, s.to] {
                b = (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1));
            }
        }
        b
    }
    /// The burn runs: consecutive burn moves (a travel, run-out or Z breaks a run; a change of S
    /// or F starts a new run).
    pub fn runs(&self) -> Vec<Run> {
        let mut out: Vec<Run> = Vec::new();
        let mut open = false;
        for s in &self.segs {
            if s.kind != Kind::Burn {
                open = false;
                continue;
            }
            match out.last_mut() {
                Some(r)
                    if open && r.s == s.s && r.f == s.f && *r.points.last().unwrap() == s.from =>
                {
                    r.points.push(s.to);
                }
                _ => out.push(Run {
                    s: s.s,
                    f: s.f,
                    points: vec![s.from, s.to],
                }),
            }
            open = true;
        }
        out
    }
    /// The distinct (S, F) of the burn runs in order of first appearance.
    pub fn settings_order(&self) -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = Vec::new();
        for r in self.runs() {
            if v.last() != Some(&(r.s, r.f)) {
                v.push((r.s, r.f));
            }
        }
        v
    }
    /// The header lines that name a run: `(<n>, <name>)`.
    pub fn header_runs(&self) -> Vec<(u32, String)> {
        self.header
            .iter()
            .filter_map(|h| {
                let (n, name) = h.split_once(' ')?;
                Some((n.parse().ok()?, name.to_owned()))
            })
            .collect()
    }
}
