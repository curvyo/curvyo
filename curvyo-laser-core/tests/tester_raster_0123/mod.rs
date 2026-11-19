//! The tester's own reference for `specs/0123-raster-engraving`, written from the specification
//! only and independent of the generator: the dither modes of criterion 10 on a grid of grey
//! values (Threshold, Ordered 8 x 8 Bayer, Floyd-Steinberg, Jarvis, Atkinson, Stucki, Grey), the
//! luminance and alpha rule of criterion 6, and a GRBL raster reader for criteria 15 to 19 and 40
//! (burns per run with their S, M3 and M4, sweeps, travel, line counts) that knows nothing of the
//! implementation's types.
#![allow(
    clippy::match_same_arms,
    clippy::type_complexity,
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::needless_range_loop,
    clippy::float_cmp
)]

// ---- reference dither -------------------------------------------------------------------------

/// A dither mode of criterion 10.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Burns where grey is below `pct` % of 255.
    Threshold(u8),
    Ordered,
    FloydSteinberg,
    Jarvis,
    Atkinson,
    Stucki,
}

pub const ALL_SIX: [Mode; 6] = [
    Mode::Threshold(50),
    Mode::Ordered,
    Mode::FloydSteinberg,
    Mode::Jarvis,
    Mode::Atkinson,
    Mode::Stucki,
];

/// The classic 8 x 8 Bayer index matrix, built by the recursive rule.
#[must_use]
pub fn bayer8() -> [[u32; 8]; 8] {
    fn level(n: usize) -> Vec<Vec<u32>> {
        if n == 1 {
            return vec![vec![0]];
        }
        let k = n / 2;
        let m = level(k);
        let base = [[0, 2], [3, 1]];
        (0..n)
            .map(|y| {
                (0..n)
                    .map(|x| 4 * m[y % k][x % k] + base[y / k][x / k])
                    .collect()
            })
            .collect()
    }
    let m = level(8);
    let mut out = [[0; 8]; 8];
    for y in 0..8 {
        for x in 0..8 {
            out[y][x] = m[y][x];
        }
    }
    out
}

/// Error-diffusion kernels as (dx, dy, weight) and their divisor.
fn kernel(mode: Mode) -> (Vec<(i32, i32, f64)>, f64) {
    match mode {
        Mode::FloydSteinberg => (
            vec![(1, 0, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)],
            16.0,
        ),
        Mode::Jarvis => (
            vec![
                (1, 0, 7.0),
                (2, 0, 5.0),
                (-2, 1, 3.0),
                (-1, 1, 5.0),
                (0, 1, 7.0),
                (1, 1, 5.0),
                (2, 1, 3.0),
                (-2, 2, 1.0),
                (-1, 2, 3.0),
                (0, 2, 5.0),
                (1, 2, 3.0),
                (2, 2, 1.0),
            ],
            48.0,
        ),
        Mode::Stucki => (
            vec![
                (1, 0, 8.0),
                (2, 0, 4.0),
                (-2, 1, 2.0),
                (-1, 1, 4.0),
                (0, 1, 8.0),
                (1, 1, 4.0),
                (2, 1, 2.0),
                (-2, 2, 1.0),
                (-1, 2, 2.0),
                (0, 2, 4.0),
                (1, 2, 2.0),
                (2, 2, 1.0),
            ],
            42.0,
        ),
        Mode::Atkinson => (
            vec![
                (1, 0, 1.0),
                (2, 0, 1.0),
                (-1, 1, 1.0),
                (0, 1, 1.0),
                (1, 1, 1.0),
                (0, 2, 1.0),
            ],
            8.0,
        ),
        _ => (Vec::new(), 1.0),
    }
}

/// Dithers `grey` (rows of grey values; `None` outside the frame) in `mode`: `Some(true)` burns.
#[must_use]
pub fn dither(grey: &[Vec<Option<u8>>], mode: Mode) -> Vec<Vec<Option<bool>>> {
    let h = grey.len();
    let w = grey.first().map_or(0, Vec::len);
    let m = bayer8();
    match mode {
        Mode::Threshold(pct) => grey
            .iter()
            .map(|row| {
                row.iter()
                    .map(|g| g.map(|g| u32::from(g) * 100 < u32::from(pct) * 255))
                    .collect()
            })
            .collect(),
        Mode::Ordered => (0..h)
            .map(|r| {
                (0..w)
                    .map(|c| grey[r][c].map(|g| u32::from(g) < 4 * m[r % 8][c % 8] + 2))
                    .collect()
            })
            .collect(),
        _ => {
            let (k, div) = kernel(mode);
            let mut v: Vec<Vec<f64>> = grey
                .iter()
                .map(|row| row.iter().map(|g| g.map_or(0.0, f64::from)).collect())
                .collect();
            let mut out = vec![vec![None; w]; h];
            for r in 0..h {
                for c in 0..w {
                    if grey[r][c].is_none() {
                        continue;
                    }
                    let value = v[r][c];
                    let burn = value < 128.0;
                    let target = if burn { 0.0 } else { 255.0 };
                    let err = value - target;
                    out[r][c] = Some(burn);
                    for &(dx, dy, wgt) in &k {
                        let (x, y) = (c as i32 + dx, r as i32 + dy);
                        if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                            continue;
                        }
                        if grey[y as usize][x as usize].is_none() {
                            continue;
                        }
                        v[y as usize][x as usize] += err * wgt / div;
                    }
                }
            }
            out
        }
    }
}

/// How a fixed-point error diffusion rounds each spread share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    Floor,
    Trunc,
    HalfAway,
}

/// The error diffusion of `mode` in fixed point: values scaled by `scale`, each share
/// `err * w / div` rounded by `round`, a dot burns when its value is below 128 (`below_128`) or
/// at most 127.5. The classic weights are the same in every variant; only the rounding of the
/// spread error differs, which the specification leaves open (criterion 8 asks for integers).
#[must_use]
pub fn dither_fixed(
    grey: &[Vec<Option<u8>>],
    mode: Mode,
    scale: i64,
    round: Round,
    below_128: bool,
) -> Vec<Vec<Option<bool>>> {
    let h = grey.len();
    let w = grey.first().map_or(0, Vec::len);
    let (k, div) = kernel(mode);
    let div = div as i64;
    let mut v: Vec<Vec<i64>> = grey
        .iter()
        .map(|row| row.iter().map(|g| g.map_or(0, i64::from) * scale).collect())
        .collect();
    let mut out = vec![vec![None; w]; h];
    for r in 0..h {
        for c in 0..w {
            if grey[r][c].is_none() {
                continue;
            }
            let value = v[r][c];
            let burn = if below_128 {
                value < 128 * scale
            } else {
                2 * value < 255 * scale
            };
            let err = value - if burn { 0 } else { 255 * scale };
            out[r][c] = Some(burn);
            for &(dx, dy, wgt) in &k {
                let (x, y) = (c as i32 + dx, r as i32 + dy);
                if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                    continue;
                }
                if grey[y as usize][x as usize].is_none() {
                    continue;
                }
                let n = err * wgt as i64;
                let q = match round {
                    Round::Floor => n.div_euclid(div),
                    Round::Trunc => n / div,
                    Round::HalfAway => {
                        if n >= 0 {
                            (n + div / 2) / div
                        } else {
                            -((-n + div / 2) / div)
                        }
                    }
                };
                v[y as usize][x as usize] += q;
            }
        }
    }
    out
}

/// The smallest number of differing dots between `got` and any fixed-point variant of the
/// classic error diffusion `mode` (scales 1 to 65,536, three roundings, two thresholds).
#[must_use]
pub fn best_fixed_match(
    grey: &[Vec<Option<u8>>],
    mode: Mode,
    got: &[Vec<Option<bool>>],
) -> (usize, String) {
    let mut best = (usize::MAX, String::new());
    for scale in [
        1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 4096, 65_536, 3, 6, 12, 21, 42, 48, 84, 168,
        336, 672, 1344, 2688, 10_752, 2016,
    ] {
        for round in [Round::Floor, Round::Trunc, Round::HalfAway] {
            for below in [true, false] {
                let d = differences(got, &dither_fixed(grey, mode, scale, round, below));
                if d < best.0 {
                    best = (d, format!("scale {scale} {round:?} below_128 {below}"));
                }
            }
        }
    }
    best
}

/// Grey of an sRGB pixel by the weighting of criterion 6, blended over white by its alpha.
#[must_use]
pub fn grey_of(r: u8, g: u8, b: u8, a: u8) -> f64 {
    let y = 0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b);
    let a = f64::from(a) / 255.0;
    y * a + 255.0 * (1.0 - a)
}

/// A dot map as text: one line per row, `#` burned, `.` not, space outside; trailing spaces cut.
#[must_use]
pub fn map_text(map: &[Vec<Option<bool>>]) -> String {
    let mut s = String::new();
    for row in map {
        let line: String = row
            .iter()
            .map(|d| match d {
                Some(true) => '#',
                Some(false) => '.',
                None => ' ',
            })
            .collect();
        s.push_str(line.trim_end());
        s.push('\n');
    }
    s
}

/// Parses a dot map text (`#`/`.`/space) into rows.
#[must_use]
pub fn parse_map(text: &str) -> Vec<Vec<Option<bool>>> {
    text.lines()
        .map(|l| {
            l.chars()
                .map(|c| match c {
                    '#' => Some(true),
                    '.' => Some(false),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// The number of differing dots between two maps of the same shape (missing cells count as
/// outside).
#[must_use]
pub fn differences(a: &[Vec<Option<bool>>], b: &[Vec<Option<bool>>]) -> usize {
    let h = a.len().max(b.len());
    let mut n = 0;
    for r in 0..h {
        let ra = a.get(r).cloned().unwrap_or_default();
        let rb = b.get(r).cloned().unwrap_or_default();
        for c in 0..ra.len().max(rb.len()) {
            let x = ra.get(c).copied().flatten();
            let y = rb.get(c).copied().flatten();
            if x != y {
                n += 1;
            }
        }
    }
    n
}

#[must_use]
pub fn burned(map: &[Vec<Option<bool>>]) -> usize {
    map.iter().flatten().filter(|d| **d == Some(true)).count()
}

// ---- GRBL reader ------------------------------------------------------------------------------

/// One straight move of a GRBL file in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mv {
    pub from: (f64, f64),
    pub to: (f64, f64),
    /// `G0`.
    pub rapid: bool,
    /// The laser emits: `G1`, a spindle state M3 or M4 on, and S above 0.
    pub burn: bool,
    /// The modal S when it ran.
    pub s: u32,
    /// The modal F when it ran.
    pub f: f64,
    /// The spindle word in force: 3, 4 or 5.
    pub spindle: u8,
    /// 1-based line.
    pub line: usize,
}

impl Mv {
    #[must_use]
    pub fn len(&self) -> f64 {
        (self.to.0 - self.from.0).hypot(self.to.1 - self.from.1)
    }
}

/// A file read back.
#[derive(Debug, Clone, Default)]
pub struct Gcode {
    pub moves: Vec<Mv>,
    /// Lines that are not comments or empty.
    pub code_lines: usize,
    pub lines: usize,
    /// Every spindle word in file order (3, 4, 5).
    pub spindle_words: Vec<u8>,
    /// `F` words in file order with their line.
    pub feeds: Vec<(usize, f64)>,
    /// `S` words in file order.
    pub s_words: Vec<(usize, u32)>,
}

fn words(line: &str) -> Result<Vec<(char, f64)>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            i += 1;
            continue;
        }
        if !c.is_ascii_uppercase() {
            return Err(format!("unexpected {c:?} in {line:?}"));
        }
        let mut j = i + 1;
        while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.' || chars[j] == '-') {
            j += 1;
        }
        let num: String = chars[i + 1..j].iter().collect();
        let v: f64 = num
            .parse()
            .map_err(|_| format!("bad number {num:?} in {line:?}"))?;
        if !v.is_finite() {
            return Err(format!("not finite in {line:?}"));
        }
        out.push((c, v));
        i = j;
    }
    Ok(out)
}

/// Reads a GRBL file. Refuses CR, tabs, arcs, unknown words, a `G1` burn without a feed.
pub fn read(bytes: &[u8]) -> Result<Gcode, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    if text.contains('\r') || text.contains('\t') {
        return Err("CR or tab".into());
    }
    let mut g = Gcode::default();
    let (mut x, mut y) = (0.0_f64, 0.0_f64);
    let mut motion: Option<u8> = None;
    let mut s: u32 = 0;
    let mut f: f64 = 0.0;
    let mut spindle: u8 = 5;
    let mut metric = false;
    let mut absolute = false;
    for (n, raw) in text.lines().enumerate() {
        let line_no = n + 1;
        g.lines += 1;
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        g.code_lines += 1;
        let ws = words(line)?;
        let mut nx = None;
        let mut ny = None;
        let mut moved = false;
        for &(c, v) in &ws {
            match c {
                'G' => match v as i64 {
                    0 => {
                        motion = Some(0);
                        moved = true;
                    }
                    1 => {
                        motion = Some(1);
                        moved = true;
                    }
                    21 => metric = true,
                    90 => absolute = true,
                    17 => {}
                    28 => {}
                    92 => {}
                    other => return Err(format!("unknown G{other} at {line_no}")),
                },
                'M' => match v as i64 {
                    3 => {
                        spindle = 3;
                        g.spindle_words.push(3);
                    }
                    4 => {
                        spindle = 4;
                        g.spindle_words.push(4);
                    }
                    5 => {
                        spindle = 5;
                        g.spindle_words.push(5);
                    }
                    7 | 8 | 9 | 2 | 30 => {}
                    other => return Err(format!("unknown M{other} at {line_no}")),
                },
                'X' => {
                    nx = Some(v);
                    moved = true;
                }
                'Y' => {
                    ny = Some(v);
                    moved = true;
                }
                'Z' => {}
                'S' => {
                    if v < 0.0 || v.fract() != 0.0 {
                        return Err(format!("S must be whole at {line_no}"));
                    }
                    s = v as u32;
                    g.s_words.push((line_no, s));
                }
                'F' => {
                    if v <= 0.0 {
                        return Err(format!("F must be positive at {line_no}"));
                    }
                    f = v;
                    g.feeds.push((line_no, v));
                }
                '$' => {}
                other => return Err(format!("unknown word {other} at {line_no}")),
            }
        }
        if moved && (nx.is_some() || ny.is_some()) {
            if !metric || !absolute {
                return Err(format!("move before G21/G90 at {line_no}"));
            }
            let to = (nx.unwrap_or(x), ny.unwrap_or(y));
            let m = motion.ok_or_else(|| format!("move without G0/G1 at {line_no}"))?;
            let burn = m == 1 && spindle != 5 && s > 0;
            if burn && f <= 0.0 {
                return Err(format!("burn without feed at {line_no}"));
            }
            g.moves.push(Mv {
                from: (x, y),
                to,
                rapid: m == 0,
                burn,
                s,
                f,
                spindle,
                line: line_no,
            });
            x = to.0;
            y = to.1;
        }
    }
    Ok(g)
}

impl Gcode {
    #[must_use]
    pub fn burn_mm(&self) -> f64 {
        self.moves.iter().filter(|m| m.burn).map(Mv::len).sum()
    }

    #[must_use]
    pub fn rapid_mm(&self) -> f64 {
        self.moves.iter().filter(|m| m.rapid).map(Mv::len).sum()
    }

    /// Laser-off `G1` length (gaps and overscan inside sweeps).
    #[must_use]
    pub fn off_mm(&self) -> f64 {
        self.moves
            .iter()
            .filter(|m| !m.rapid && !m.burn)
            .map(Mv::len)
            .sum()
    }

    #[must_use]
    pub fn burns(&self) -> Vec<Mv> {
        self.moves.iter().copied().filter(|m| m.burn).collect()
    }

    /// The extent of the burns: (min x, min y, max x, max y).
    #[must_use]
    pub fn burn_extent(&self) -> Option<(f64, f64, f64, f64)> {
        let b = self.burns();
        if b.is_empty() {
            return None;
        }
        let mut e = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for m in b {
            for p in [m.from, m.to] {
                e.0 = e.0.min(p.0);
                e.1 = e.1.min(p.1);
                e.2 = e.2.max(p.0);
                e.3 = e.3.max(p.1);
            }
        }
        Some(e)
    }

    /// Horizontal burn segments grouped by their Y (rounded to 1 µm), in file order of first
    /// appearance: (y, [(x0, x1, s)]) with x0 < x1.
    #[must_use]
    pub fn rows(&self) -> Vec<(f64, Vec<(f64, f64, u32)>)> {
        let mut out: Vec<(f64, Vec<(f64, f64, u32)>)> = Vec::new();
        for m in self.burns() {
            if (m.from.1 - m.to.1).abs() > 1e-6 {
                continue;
            }
            let yk = (m.from.1 * 1000.0).round() / 1000.0;
            let seg = (m.from.0.min(m.to.0), m.from.0.max(m.to.0), m.s);
            if let Some(r) = out.iter_mut().find(|r| (r.0 - yk).abs() < 1e-6) {
                r.1.push(seg);
            } else {
                out.push((yk, vec![seg]));
            }
        }
        out
    }

    /// The burned dot map of horizontal rows: frame left `x0`, top row centre `y_top` (machine
    /// Y, rows go down by `interval`), `cols` x `rows_n` dots. A dot burns when a burn segment
    /// covers its centre. Returns `(map, power)` where power is the S of the covering segment.
    #[must_use]
    pub fn dot_map(
        &self,
        x0: f64,
        y_top: f64,
        interval: f64,
        cols: usize,
        rows_n: usize,
    ) -> Vec<Vec<Option<u32>>> {
        let mut map = vec![vec![None; cols]; rows_n];
        for (y, segs) in self.rows() {
            let r = ((y_top - y) / interval).round();
            if r < 0.0 || r as usize >= rows_n {
                continue;
            }
            let r = r as usize;
            for (a, b, s) in segs {
                for c in 0..cols {
                    let cx = x0 + (c as f64 + 0.5) * interval;
                    if cx > a + 1e-6 && cx < b - 1e-6 {
                        map[r][c] = Some(s);
                    }
                }
            }
        }
        map
    }
}

/// Burned dots of a power map.
#[must_use]
pub fn burned_power(map: &[Vec<Option<u32>>]) -> usize {
    map.iter().flatten().filter(|d| d.is_some()).count()
}

/// A boolean map from a power map.
#[must_use]
pub fn to_bool(map: &[Vec<Option<u32>>]) -> Vec<Vec<Option<bool>>> {
    map.iter()
        .map(|r| r.iter().map(|d| Some(d.is_some())).collect())
        .collect()
}
