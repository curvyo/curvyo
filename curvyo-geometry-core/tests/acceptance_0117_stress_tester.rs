//! Tester stress for `specs/0117-centre-lines`: random noisy axis pieces (thin strokes along
//! random walks, the inputs that hung kurbo's fitter) alone and joined to the whale and the
//! dolphin, moved far from the origin and scaled, with random options, through the whole
//! pipeline (`CentreSkeleton::build` + `finish`). Each run has a 120 s watchdog; a hang or a
//! panic names its trial. Run by hand, in debug and in release:
//!
//! ```text
//! CENTRE_STRESS_SEED=1 CENTRE_STRESS_TRIALS=200 cargo test -p curvyo-geometry-core \
//!   --test acceptance_0117_stress_tester -- --ignored --nocapture
//! ```
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    missing_docs
)]

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, Markers, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    BooleanOp, CentreOptions, CentreSkeleton, Outline, StrokeShape, WidthProfile, boolean,
    variable_stroke_outline,
};

type Triple = (Point, Vec2, Vec2);
type Rings = Vec<Vec<Triple>>;

const WHALE: &str = include_str!("fixtures/centre_lines/whale_traced.txt");
const DOLPHIN: &str = include_str!("fixtures/centre_lines/dolphin_traced.txt");

/// xorshift64*: a reproducible stream per seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// Every outline of every subject of a traced fixture, as one region.
fn fixture(text: &str) -> Rings {
    let mut rings: Rings = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") | None => {}
            Some(&"outline") => rings.push(Vec::new()),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                rings.last_mut().unwrap().push((
                    Point::new(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
        }
    }
    rings
}

fn outlines(rings: &Rings) -> Vec<Outline<'_>> {
    rings.iter().map(|r| Outline::new(r, true)).collect()
}

/// The union of several regions (cleans a self-overlapping stroke outline too).
fn union(parts: &[Rings]) -> Option<Rings> {
    let ops: Vec<Vec<Outline<'_>>> = parts.iter().map(outlines).collect();
    let refs: Vec<&[Outline<'_>]> = ops.iter().map(Vec::as_slice).collect();
    let res = boolean(BooleanOp::Union, &refs, Tolerance::from_mm(0.003)).ok()?;
    Some(
        res.outlines()
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|q| (*q, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0)))
                    .collect()
            })
            .collect(),
    )
}

/// A noisy axis piece: a random walk of short steps and sharp turns from `start`, stroked
/// with round ends at `width`.
fn noisy_piece(rng: &mut Rng, start: Point, width: f64) -> Option<Rings> {
    let steps = 2 + (rng.next() % 30) as usize;
    let mut heading = 0.0_f64;
    let mut at = start;
    let mut anchors: Vec<Triple> = vec![(at, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0))];
    for _ in 0..steps {
        heading += rng.range(-3.2, 3.2);
        let length = rng.range(0.01, 0.5) * width.max(0.2) * 4.0;
        at = Point::new(at.x + heading.cos() * length, at.y + heading.sin() * length);
        anchors.push((at, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0)));
    }
    let shape = StrokeShape {
        width: Length::from_mm(width),
        join: LineJoin::Round,
        cap: LineCap::Round,
        dash: DashPattern::default(),
        markers: Markers::default(),
    };
    let profile = WidthProfile::new(Vec::new(), false)?;
    let out = variable_stroke_outline(
        Outline::new(&anchors, false),
        &shape,
        &profile,
        Tolerance::from_mm(0.002),
    )
    .ok()?;
    let rings: Rings = out
        .outlines()
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect();
    union(&[rings])
}

/// `rings` scaled about the origin, moved, and with every anchor jittered by up to `jitter`.
fn moved(rings: &Rings, scale: f64, offset: Vec2, jitter: f64, rng: &mut Rng) -> Rings {
    rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&(q, hin, hout)| {
                    let j = Vec2::new(rng.range(-jitter, jitter), rng.range(-jitter, jitter));
                    (
                        Point::new(q.x * scale + offset.x + j.x, q.y * scale + offset.y + j.y),
                        Vec2::new(hin.x * scale, hin.y * scale),
                        Vec2::new(hout.x * scale, hout.y * scale),
                    )
                })
                .collect()
        })
        .collect()
}

/// One trial's input and options, described for the log.
struct Trial {
    what: String,
    rings: Rings,
    options: CentreOptions,
}

fn trial(seed: u64, index: u64, whale: &Rings, dolphin: &Rings) -> Option<Trial> {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (index + 1).wrapping_mul(0xd1b5));
    for _ in 0..4 {
        rng.next();
    }
    let offsets = [0.0, 100.0, 1_000.0, 2_000.0, 5_000.0];
    let off = |rng: &mut Rng| {
        let o = offsets[(rng.next() % offsets.len() as u64) as usize];
        o * if rng.next().is_multiple_of(2) {
            1.0
        } else {
            -1.0
        } + rng.range(0.0, 50.0)
    };
    let offset = Vec2::new(off(&mut rng), off(&mut rng));
    let scale = 10_f64.powf(rng.range(-1.0, 1.0));
    let kind = rng.next() % 3;
    let jitter = if rng.next().is_multiple_of(2) {
        0.0
    } else {
        rng.range(0.0, 0.05)
    };
    let width = rng.range(0.1, 3.0);
    let pieces = 1 + (rng.next() % 4) as usize;
    let (base, name) = match kind {
        0 => (None, "pieces"),
        1 => (Some(whale), "whale"),
        _ => (Some(dolphin), "dolphin"),
    };
    let mut parts: Vec<Rings> = Vec::new();
    if let Some(b) = base {
        parts.push(moved(b, scale, offset, jitter, &mut rng));
    }
    for _ in 0..pieces {
        let start = if base.is_some() {
            // On the drawing, so the pieces join its bands.
            let ring = &parts[0][(rng.next() % parts[0].len() as u64) as usize];
            ring[(rng.next() % ring.len() as u64) as usize].0
        } else {
            Point::new(
                offset.x + rng.range(0.0, 20.0),
                offset.y + rng.range(0.0, 20.0),
            )
        };
        parts.push(noisy_piece(&mut rng, start, width)?);
    }
    let rings = if parts.len() == 1 {
        parts.pop()?
    } else {
        union(&parts)?
    };
    let options = CentreOptions {
        overlap: rng.range(-0.5, 0.5),
        thick_parts: if rng.next().is_multiple_of(3) {
            None
        } else {
            Some(Length::from_mm(rng.range(0.5, 100.0)))
        },
        inner_offset: if rng.next().is_multiple_of(2) {
            None
        } else {
            Some(Length::from_mm(rng.range(-1.0, 1.0)))
        },
        optimise: [0.0, 0.25, 0.5, 0.75, 1.0, rng.unit()][(rng.next() % 6) as usize],
    };
    Some(Trial {
        what: format!(
            "seed {seed} trial {index}: {name} + {pieces} pieces w {width:.3} scale {scale:.3} \
             offset ({:.1}, {:.1}) jitter {jitter:.3} {options:?}",
            offset.x, offset.y
        ),
        rings,
        options,
    })
}

fn env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The outcome of one run under a watchdog.
enum Outcome {
    Built(usize),
    Refused,
    Hang,
    Panic,
}

/// Runs `t` through the pipeline in a thread and waits at most `limit`; a hanging thread is
/// left behind.
fn run(t: &Trial, limit: Duration) -> Outcome {
    let (tx, rx) = mpsc::channel();
    let rings = t.rings.clone();
    let options = t.options;
    let worker = thread::spawn(move || {
        let result = CentreSkeleton::build(&outlines(&rings))
            .map(|skeleton| skeleton.finish(&options, &[]).paths.len());
        let _ = tx.send(result.ok());
    });
    match rx.recv_timeout(limit) {
        Ok(Some(n)) => {
            worker.join().unwrap();
            Outcome::Built(n)
        }
        Ok(None) => {
            worker.join().unwrap();
            Outcome::Refused
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Outcome::Hang,
        Err(mpsc::RecvTimeoutError::Disconnected) => Outcome::Panic,
    }
}

/// Whole-pipeline stress: no trial hangs (120 s watchdog) or panics. A hang is logged and the
/// run goes on (at most four hangs per run, each leaves a busy thread).
#[test]
#[ignore = "stress, run by hand with CENTRE_STRESS_SEED, CENTRE_STRESS_TRIALS, CENTRE_STRESS_FROM"]
fn noisy_axis_pieces_far_and_scaled_never_hang_or_panic() {
    let seed = env("CENTRE_STRESS_SEED", 1);
    let trials = env("CENTRE_STRESS_TRIALS", 50);
    let from = env("CENTRE_STRESS_FROM", 0);
    let whale = fixture(WHALE);
    let dolphin = fixture(DOLPHIN);
    let (mut built, mut refused, mut paths) = (0_usize, 0_usize, 0_usize);
    let mut bad: Vec<String> = Vec::new();
    for index in from..trials {
        let Some(t) = trial(seed, index, &whale, &dolphin) else {
            continue;
        };
        match run(&t, Duration::from_secs(120)) {
            Outcome::Built(n) => {
                built += 1;
                paths += n;
            }
            Outcome::Refused => refused += 1,
            Outcome::Hang => {
                eprintln!("HANG (> 120 s): {}", t.what);
                bad.push(format!("HANG {}", t.what));
            }
            Outcome::Panic => {
                eprintln!("PANIC: {}", t.what);
                bad.push(format!("PANIC {}", t.what));
            }
        }
        if bad.iter().filter(|b| b.starts_with("HANG")).count() >= 4 {
            break;
        }
    }
    eprintln!(
        "seed {seed}: trials {from}..{trials}, {built} built ({paths} paths), {refused} refused, \
         {} bad",
        bad.len()
    );
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Writes the input of the settle-corner hang (seed 5, trial 19 of the stress above: one noisy
/// piece) as `tests/fixtures/centre_lines/settle_corner_loop.txt`, with its options.
#[test]
#[ignore = "writes a fixture; run by hand"]
fn write_the_settle_corner_fixture() {
    use std::fmt::Write as _;
    let t = trial(5, 19, &fixture(WHALE), &fixture(DOLPHIN)).unwrap();
    let mut out = format!("# {}\nsubject\n", t.what);
    for ring in &t.rings {
        out.push_str("outline true\n");
        for (q, hin, hout) in ring {
            let _ = writeln!(
                out,
                "{:?} {:?} {:?} {:?} {:?} {:?}",
                q.x, q.y, hin.x, hin.y, hout.x, hout.y
            );
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/centre_lines/settle_corner_loop.txt"
    );
    std::fs::write(path, out).unwrap();
}

const SETTLE_CORNER_LOOP: &str = include_str!("fixtures/centre_lines/settle_corner_loop.txt");

/// The whole pipeline on one noisy axis piece (`settle_corner_loop.txt`, seed 5 trial 19 of the
/// stress above) finishes. It hung for ever in `medial_axis::spurs::settle_corner`: the walk
/// along the degree-2 run from a settled junction has no visited set or step bound, so a loop
/// that stays within `CORNER_RUN` radii of the junction spins (5 of 5 stress hangs, debug and
/// release, through `prune_spurs` and `prune_short_subtrees`, on pieces, whale and dolphin).
#[test]
fn a_small_axis_loop_at_a_settled_corner_does_not_hang() {
    let t = Trial {
        what: "settle_corner_loop.txt".to_owned(),
        rings: fixture(SETTLE_CORNER_LOOP),
        options: CentreOptions {
            overlap: 0.286_841_202_996_590_1,
            thick_parts: Some(Length::from_mm(47.082_248_227_493_5)),
            inner_offset: Some(Length::from_mm(0.415_713_456_280_607_3)),
            optimise: 0.75,
        },
    };
    assert!(
        !matches!(
            run(&t, Duration::from_secs(60)),
            Outcome::Hang | Outcome::Panic
        ),
        "hang or panic: {}",
        t.what
    );
}

/// A random closed ring: radius noise around a centre, with optional tiny kinks (a loop-like
/// spike of a few steps with a very small radius), as straight segments.
fn wobbly_ring(rng: &mut Rng, c: Point, radius: f64, kinks: bool) -> Vec<Triple> {
    let n = 8 + (rng.next() % 60) as usize;
    let mut ring: Vec<Triple> = Vec::new();
    for i in 0..n {
        let a = std::f64::consts::TAU * i as f64 / n as f64 + rng.range(-0.02, 0.02);
        let mut r = radius * rng.range(0.55, 1.0);
        if kinks && rng.next().is_multiple_of(5) {
            r = radius * rng.range(0.02, 0.15);
        }
        ring.push((
            Point::new(c.x + a.cos() * r, c.y + a.sin() * r),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
        ));
    }
    ring
}

/// Random closed rings with tiny loops and holes (a region with up to four holes, several
/// rings with kinks), scaled and moved, with random options: no hang, no panic.
#[test]
#[ignore = "stress, run by hand with CENTRE_STRESS_SEED, CENTRE_STRESS_TRIALS"]
fn random_rings_with_tiny_loops_and_holes_never_hang_or_panic() {
    let seed = env("CENTRE_STRESS_SEED", 1);
    let trials = env("CENTRE_STRESS_TRIALS", 50);
    let (mut built, mut refused, mut skipped) = (0_usize, 0_usize, 0_usize);
    let mut bad: Vec<String> = Vec::new();
    for index in 0..trials {
        let mut rng =
            Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (index + 7).wrapping_mul(0xa5a5));
        for _ in 0..4 {
            rng.next();
        }
        let scale = 10_f64.powf(rng.range(-1.0, 1.5));
        let off = Vec2::new(rng.range(-3000.0, 3000.0), rng.range(-3000.0, 3000.0));
        let kinks = !rng.next().is_multiple_of(3);
        let mut rings: Rings = Vec::new();
        let outer = wobbly_ring(&mut rng, Point::new(off.x, off.y), 20.0 * scale, kinks);
        rings.push(outer);
        for _ in 0..(rng.next() % 5) {
            let c = Point::new(
                off.x + rng.range(-8.0, 8.0) * scale,
                off.y + rng.range(-8.0, 8.0) * scale,
            );
            let r = rng.range(1.0, 6.0) * scale;
            rings.push(wobbly_ring(&mut rng, c, r, kinks));
        }
        // Even-odd-ish clean-up through the boolean (holes overlapping the outer are fine).
        let Some(cleaned) = union(&[rings]) else {
            skipped += 1;
            continue;
        };
        if cleaned.is_empty() {
            skipped += 1;
            continue;
        }
        let options = CentreOptions {
            overlap: rng.range(-0.5, 0.5),
            thick_parts: if rng.next().is_multiple_of(3) {
                None
            } else {
                Some(Length::from_mm(rng.range(0.5, 100.0)))
            },
            inner_offset: if rng.next().is_multiple_of(2) {
                None
            } else {
                Some(Length::from_mm(rng.range(-1.0, 1.0)))
            },
            optimise: rng.unit(),
        };
        let t = Trial {
            what: format!("rings seed {seed} trial {index} scale {scale:.2} {options:?}"),
            rings: cleaned,
            options,
        };
        match run(&t, Duration::from_secs(120)) {
            Outcome::Built(_) => built += 1,
            Outcome::Refused => refused += 1,
            Outcome::Hang => bad.push(format!("HANG {}", t.what)),
            Outcome::Panic => bad.push(format!("PANIC {}", t.what)),
        }
        if bad.len() >= 4 {
            break;
        }
    }
    eprintln!(
        "rings seed {seed}: {trials} trials, {built} built, {refused} refused, {skipped} skipped, {} bad",
        bad.len()
    );
    assert!(bad.is_empty(), "{bad:#?}");
}
