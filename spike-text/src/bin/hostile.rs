//! Hostile font input: fixed fixtures plus a time-boxed random-mutation run.
//! Usage: hostile <seconds for the random run> <font files...>
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct Outcome {
    hr_parsed: bool,
    hr_shaped: bool,
    rb_parsed: bool,
    panic_hr: bool,
    panic_rb: bool,
}

fn run_case(data: &[u8], text: &str) -> Outcome {
    let mut o = Outcome::default();
    let r = catch_unwind(AssertUnwindSafe(|| {
        if let Ok(face) = spike_text::Face::new(data.to_vec(), 0) {
            o.hr_parsed = true;
            o.hr_shaped = face.shape_and_outline(text, 20.0).is_ok();
            std::hint::black_box(face.outline_all());
        }
    }));
    o.panic_hr = r.is_err();
    let r = catch_unwind(AssertUnwindSafe(|| {
        if let Some(face) = rustybuzz::Face::from_slice(data, 0) {
            o.rb_parsed = true;
            std::hint::black_box(spike_text::rb::shape_and_outline(&face, text));
            for g in 0..face.number_of_glyphs() {
                struct N;
                impl ttf_parser::OutlineBuilder for N {
                    fn move_to(&mut self, _: f32, _: f32) {}
                    fn line_to(&mut self, _: f32, _: f32) {}
                    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
                    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
                    fn close(&mut self) {}
                }
                let _ = face.outline_glyph(ttf_parser::GlyphId(g), &mut N);
            }
        }
    }));
    o.panic_rb = r.is_err();
    o
}

/// sfnt table records: (tag, offset, length, record position).
fn tables(d: &[u8]) -> Vec<([u8; 4], usize, usize, usize)> {
    let be16 = |p: usize| {
        d.get(p..p + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as usize)
    };
    let be32 = |p: usize| {
        d.get(p..p + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let n = be16(4).unwrap_or(0);
    (0..n)
        .filter_map(|i| {
            let r = 12 + 16 * i;
            let tag: [u8; 4] = d.get(r..r + 4)?.try_into().ok()?;
            Some((tag, be32(r + 8)?, be32(r + 12)?, r))
        })
        .collect()
}

fn table(d: &[u8], t: &[u8; 4]) -> Option<(usize, usize)> {
    tables(d)
        .into_iter()
        .find(|x| &x.0 == t)
        .map(|x| (x.1, x.2))
}

/// glyf glyph byte ranges from loca.
fn glyph_ranges(d: &[u8]) -> Vec<(usize, usize)> {
    let (Some((head, _)), Some((loca, ll)), Some((glyf, _))) =
        (table(d, b"head"), table(d, b"loca"), table(d, b"glyf"))
    else {
        return vec![];
    };
    let long = d.get(head + 50..head + 52) == Some(&[0, 1]);
    let mut offs = vec![];
    if long {
        for p in (loca..loca + ll).step_by(4) {
            if let Some(b) = d.get(p..p + 4) {
                offs.push(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
            }
        }
    } else {
        for p in (loca..loca + ll).step_by(2) {
            if let Some(b) = d.get(p..p + 2) {
                offs.push(u16::from_be_bytes([b[0], b[1]]) as usize * 2)
            }
        }
    }
    offs.windows(2)
        .map(|w| (glyf + w[0], glyf + w[1]))
        .collect()
}

fn fixtures(name: &str, real: &[u8], rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = vec![];
    // Truncations: every byte up to 512, then 300 spread lengths.
    for len in (0..512.min(real.len())).chain((0..300).map(|i| real.len() * i / 300)) {
        v.push((format!("{name}: truncated to {len}"), real[..len].to_vec()));
    }
    // Per table: zeroed data, truncated just inside it, length set huge, offset past the end.
    for (tag, off, len, rec) in tables(real) {
        let t = String::from_utf8_lossy(&tag).into_owned();
        let mut z = real.to_vec();
        for b in z.iter_mut().skip(off).take(len) {
            *b = 0
        }
        v.push((format!("{name}: {t} zeroed"), z));
        if off + len / 2 <= real.len() {
            v.push((
                format!("{name}: cut inside {t}"),
                real[..off + len / 2].to_vec(),
            ));
        }
        let mut h = real.to_vec();
        h[rec + 12..rec + 16].copy_from_slice(&u32::MAX.to_be_bytes());
        v.push((format!("{name}: {t} length 0xffffffff"), h));
        let mut p = real.to_vec();
        p[rec + 8..rec + 12].copy_from_slice(&((real.len() as u32) + 16).to_be_bytes());
        v.push((format!("{name}: {t} offset past end"), p));
        let mut r = real.to_vec();
        for b in r.iter_mut().skip(off).take(len) {
            *b = rng.next() as u8
        }
        v.push((format!("{name}: {t} random content"), r));
    }
    // glyf: composite loops and a simple glyph claiming 65,536 points.
    let ranges = glyph_ranges(real);
    let composites: Vec<(usize, usize)> = ranges
        .iter()
        .enumerate()
        .filter(|(_, (s, e))| e > s && real.get(*s..s + 2) == Some(&[0xff, 0xff]))
        .map(|(g, (s, _))| (g, *s))
        .collect();
    if let Some(&(g, s)) = composites.first() {
        let mut d = real.to_vec();
        d[s + 12..s + 14].copy_from_slice(&(g as u16).to_be_bytes());
        v.push((format!("{name}: composite glyph {g} refers to itself"), d));
    }
    if composites.len() >= 2 {
        let (a, sa) = composites[0];
        let (b, sb) = composites[1];
        let mut d = real.to_vec();
        d[sa + 12..sa + 14].copy_from_slice(&(b as u16).to_be_bytes());
        d[sb + 12..sb + 14].copy_from_slice(&(a as u16).to_be_bytes());
        v.push((format!("{name}: composites {a} <-> {b} loop"), d));
        // Every composite points to every composite's successor: one long cycle.
        let mut d = real.to_vec();
        for (i, &(_, s)) in composites.iter().enumerate() {
            let next = composites[(i + 1) % composites.len()].0 as u16;
            d[s + 12..s + 14].copy_from_slice(&next.to_be_bytes());
        }
        v.push((
            format!("{name}: {} composites in one cycle", composites.len()),
            d,
        ));
    }
    if let Some((g, (s, _))) = ranges.iter().enumerate().find(|(_, (s, e))| {
        e > s
            && real
                .get(*s..s + 2)
                .is_some_and(|b| i16::from_be_bytes([b[0], b[1]]) > 0)
    }) {
        let nc = i16::from_be_bytes([real[*s], real[s + 1]]) as usize;
        let mut d = real.to_vec();
        let p = s + 10 + 2 * (nc - 1);
        d[p..p + 2].copy_from_slice(&0xffffu16.to_be_bytes());
        v.push((format!("{name}: glyph {g} claims 65536 points"), d));
    }
    // Random flips.
    for flips in [1usize, 10, 100, 1000] {
        for seed in 0..25 {
            let mut d = real.to_vec();
            for _ in 0..flips {
                let i = rng.below(d.len());
                d[i] = rng.next() as u8
            }
            v.push((format!("{name}: {flips} random bytes, seed {seed}"), d));
        }
    }
    v
}

fn generic(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut v = vec![("empty".to_string(), vec![])];
    for n in [1usize, 4, 12, 64, 1024, 65_536, 1 << 20] {
        v.push((format!("zeroed {n} bytes"), vec![0; n]));
        let mut z = vec![0; n.max(12)];
        z[..4].copy_from_slice(&[0, 1, 0, 0]);
        v.push((format!("sfnt header then zeros, {n} bytes"), z));
    }
    for i in 0..200 {
        let n = rng.below(1 << 16) + 1;
        let mut d: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        if i % 2 == 0 && n >= 12 {
            d[..4].copy_from_slice(if i % 4 == 0 { b"OTTO" } else { &[0, 1, 0, 0] });
            d[4..6].copy_from_slice(&((rng.below(40) as u16).to_be_bytes()));
        }
        v.push((format!("random {n} bytes #{i}"), d));
    }
    for i in 0..50 {
        let mut d = b"ttcf\0\x01\0\0".to_vec();
        d.extend((0..rng.below(4096)).map(|_| rng.next() as u8));
        v.push((format!("random ttc #{i}"), d));
    }
    v
}

fn mutate(real: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut d = real.to_vec();
    match rng.below(5) {
        0 => {
            for _ in 0..1 + rng.below(64) {
                let i = rng.below(d.len());
                d[i] = rng.next() as u8
            }
        }
        1 => {
            let t = tables(real);
            if !t.is_empty() {
                let (_, off, len, _) = t[rng.below(t.len())];
                for _ in 0..1 + rng.below(16) {
                    let i = off + rng.below(len.max(1));
                    if i < d.len() {
                        d[i] = rng.next() as u8
                    }
                }
            }
        }
        2 => {
            let t = tables(real);
            if !t.is_empty() {
                let (_, _, _, rec) = t[rng.below(t.len())];
                let i = rec + 8 + rng.below(8);
                d[i] = rng.next() as u8
            }
        }
        3 => d.truncate(rng.below(d.len() + 1)),
        _ => {
            let a = rng.below(d.len());
            let b = rng.below(d.len());
            let n = rng.below(256).min(d.len() - a.max(b));
            d.copy_within(a..a + n, b)
        }
    }
    d
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut args = std::env::args().skip(1);
    let secs: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let fonts: Vec<(String, Vec<u8>)> = args
        .map(|p| {
            (
                std::path::Path::new(&p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    let text =
        "AVATAR Toffee: the office waffle flavour fits five fine fjords. Typewriters, LAVA, Yacht.";
    let crash_dir = std::path::Path::new("target/spike-fonts/crashes");
    std::fs::create_dir_all(crash_dir).unwrap();

    let (job_tx, job_rx) = mpsc::channel::<Vec<u8>>();
    let (res_tx, res_rx) = mpsc::channel::<Outcome>();
    let t2 = text.to_string();
    std::thread::spawn(move || {
        for d in job_rx {
            let _ = res_tx.send(run_case(&d, &t2));
        }
    });
    #[derive(Default)]
    struct St {
        n: usize,
        hr_parse: usize,
        hr_shape: usize,
        rb_parse: usize,
        p_hr: usize,
        p_rb: usize,
        worst: (Duration, String),
    }
    let mut st = St::default();
    let check = |label: &str, d: Vec<u8>, st: &mut St| {
        let keep = d.clone();
        let t = Instant::now();
        job_tx.send(d).unwrap();
        let Ok(o) = res_rx.recv_timeout(TIMEOUT) else {
            std::fs::write(crash_dir.join(format!("hang-{}.bin", st.n)), &keep).unwrap();
            println!("HANG > {TIMEOUT:?}: {label}");
            std::process::exit(2);
        };
        let el = t.elapsed();
        if el > st.worst.0 {
            st.worst = (el, label.to_string())
        }
        st.n += 1;
        if label.contains("compos") || label.contains("65536") {
            println!("  {label}: {o:?}, {:.1} ms", el.as_secs_f64() * 1e3)
        }
        st.hr_parse += usize::from(o.hr_parsed);
        st.hr_shape += usize::from(o.hr_shaped);
        st.rb_parse += usize::from(o.rb_parsed);
        if o.panic_hr {
            st.p_hr += 1;
            std::fs::write(crash_dir.join(format!("panic-hr-{}.bin", st.n)), &keep).unwrap();
            println!("PANIC harfrust/skrifa: {label}")
        }
        if o.panic_rb {
            st.p_rb += 1;
            std::fs::write(crash_dir.join(format!("panic-rb-{}.bin", st.n)), &keep).unwrap();
            println!("PANIC rustybuzz/ttf-parser: {label}")
        }
    };
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for (l, d) in generic(&mut rng) {
        check(&l, d, &mut st)
    }
    for (name, real) in &fonts {
        for (l, d) in fixtures(name, real, &mut rng) {
            check(&l, d, &mut st)
        }
    }
    println!(
        "fixtures: {} cases, harfrust parsed {}, shaped {}, rustybuzz parsed {}, panics hr {} rb {}, slowest {:.1} ms ({})",
        st.n,
        st.hr_parse,
        st.hr_shape,
        st.rb_parse,
        st.p_hr,
        st.p_rb,
        st.worst.0.as_secs_f64() * 1e3,
        st.worst.1
    );
    let fixed = st.n;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        let (name, real) = &fonts[rng.below(fonts.len())];
        let d = mutate(real, &mut rng);
        let label = format!("{name}: random mutation #{}", st.n);
        check(&label, d, &mut st);
    }
    println!(
        "random run: {} cases in {secs} s; totals: panics hr {} rb {}, slowest {:.1} ms ({})",
        st.n - fixed,
        st.p_hr,
        st.p_rb,
        st.worst.0.as_secs_f64() * 1e3,
        st.worst.1
    );
}
