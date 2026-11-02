//! Thread CPU time of pattern generation (`specs/0071-pattern-fill` criterion 18, `adrs.md`
//! decision 6, acceptance report finding 2, criterion 29): every generator at its defaults in both
//! modes, and every generator in both modes (and with whole pieces only, where that applies) at
//! the finest setting that is still made (found by bisection
//! against the work limit) on a circle and on an 8,000-edge star, which is where the work limit
//! must hold. The numbers are printed (`--nocapture`) and recorded in `plan.md`; the budgets are
//! asserted in a native release build only (the `performance-budgets` CI job); in a debug build
//! the bisection tests are skipped.
//!
//! ```text
//! cargo test --release -p curvyo-geometry-core --test pattern_cpu_time -- --nocapture --test-threads=1
//! ```

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, missing_docs)]

use std::time::Duration;

use curvyo_document_core::{
    GeneratorKind, Length, PatternCount, PatternFill, PatternGenerator, PatternMode, PatternParam,
    PatternSeed, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    PATTERN_MAX_PIECES, PATTERN_NOTE_PIECES, PatternError, PatternOutline, PatternPiece,
    pattern_pieces,
};

const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

/// One generator at its defaults on a 100 mm square, both modes (measured worst case about 40 ms
/// on the build VM, Sea waves in Shapes mode; recorded in `plan.md`).
const DEFAULT_BUDGET: Duration = Duration::from_millis(100);
/// Anything that is made, however fine: the work limit is at most about 70 ms on the build VM,
/// measured as an upper envelope; the rest is room for a CI runner up to about 1.9 times slower.
const WORST_BUDGET: Duration = Duration::from_millis(150);
/// A refusal before any work.
const REFUSAL_BUDGET: Duration = Duration::from_millis(5);

type Outlines = Vec<PatternOutline>;

fn polygon(points: &[(f64, f64)]) -> Outlines {
    let anchors = points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect();
    vec![(anchors, true)]
}

fn square() -> Outlines {
    polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)])
}

fn circle() -> Outlines {
    let (c, r) = (50.0, 50.0);
    let k = 0.552_284_749_830_793_6 * r;
    let a = |x: f64, y: f64, h_in: (f64, f64), h_out: (f64, f64)| {
        (
            Point::new(c + x, c + y),
            Vec2::new(h_in.0, h_in.1),
            Vec2::new(h_out.0, h_out.1),
        )
    };
    vec![(
        vec![
            a(r, 0.0, (0.0, -k), (0.0, k)),
            a(0.0, r, (k, 0.0), (-k, 0.0)),
            a(-r, 0.0, (0.0, k), (0.0, -k)),
            a(0.0, -r, (-k, 0.0), (k, 0.0)),
        ],
        true,
    )]
}

/// A star of 4,000 needle points 10 mm deep on a 100 mm circle: 8,000 edges, a boundary that
/// crosses every cell of a ring and every line through it many times.
fn star() -> Outlines {
    let n = 4000_u32;
    let points: Vec<(f64, f64)> = (0..2 * n)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::PI / f64::from(n);
            let r = if i % 2 == 0 { 50.0 } else { 40.0 };
            (50.0 + r * libm::cos(a), 50.0 + r * libm::sin(a))
        })
        .collect();
    polygon(&points)
}

#[cfg(unix)]
fn thread_time() -> Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    Duration::new(
        u64::try_from(now.tv_sec).unwrap(),
        u32::try_from(now.tv_nsec).unwrap(),
    )
}

#[cfg(not(unix))]
fn thread_time() -> Duration {
    // Off Unix the budgets are not asserted; wall time stands in for the printout.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}

fn timed_on(
    outlines: &Outlines,
    fill: &PatternFill,
) -> (Result<Vec<PatternPiece>, PatternError>, Duration) {
    let started = thread_time();
    let result = pattern_pieces(outlines, fill, Length::from_mm(0.25), TOLERANCE);
    (result, thread_time().saturating_sub(started))
}

fn timed(fill: &PatternFill) -> (Result<Vec<PatternPiece>, PatternError>, Duration) {
    timed_on(&square(), fill)
}

fn fill(generator: PatternGenerator, mode: PatternMode) -> PatternFill {
    PatternFill {
        generator,
        margin: Length::from_mm(0.0),
        mode,
        seed: PatternSeed::new(1),
        whole_pieces: false,
    }
}

/// The settings measured for `kind`: cut pieces, and whole pieces only where that applies.
fn wholes(kind: GeneratorKind) -> &'static [bool] {
    if kind.has_whole_pieces() {
        &[false, true]
    } else {
        &[false]
    }
}

fn fill_whole(generator: PatternGenerator, mode: PatternMode, whole: bool) -> PatternFill {
    PatternFill {
        whole_pieces: whole,
        ..fill(generator, mode)
    }
}

fn assert_budget(name: &str, elapsed: Duration, budget: Duration) {
    if cfg!(all(unix, not(debug_assertions))) {
        assert!(elapsed < budget, "{name}: {elapsed:?} over {budget:?}");
    }
}

fn with(kind: GeneratorKind, values: &[(PatternParam, f64)]) -> PatternGenerator {
    values
        .iter()
        .fold(PatternGenerator::defaults(kind), |g, &(param, value)| {
            g.with(param, value).unwrap()
        })
}

#[test]
fn ac18_every_generator_at_its_defaults_is_fast() {
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            for &whole in wholes(kind) {
                let defaults = PatternGenerator::defaults(kind);
                let (result, elapsed) = timed(&fill_whole(defaults, mode, whole));
                let count = result.unwrap().len();
                println!(
                    "{:<20} {mode:?} whole {whole}: {count:>6} pieces, {elapsed:?} CPU",
                    kind.id()
                );
                assert_budget(kind.id(), elapsed, DEFAULT_BUDGET);
            }
        }
    }
}

/// The generator `kind` at fineness `s` (smaller is finer and more work), every size it has tied
/// to `s`.
fn at_scale(kind: GeneratorKind, s: f64) -> PatternGenerator {
    use PatternParam as P;
    let least = |v: f64| v.max(0.05);
    match kind {
        GeneratorKind::Hatch | GeneratorKind::Crosshatch | GeneratorKind::Grid => {
            with(kind, &[(P::Spacing, least(s))])
        }
        GeneratorKind::Dots => with(
            kind,
            &[(P::Spacing, least(s)), (P::Diameter, least(s / 3.0))],
        ),
        GeneratorKind::Honeycomb => {
            with(kind, &[(P::Wall, 0.05), (P::CellSize, least(s).max(0.1))])
        }
        GeneratorKind::BrickWall => with(
            kind,
            &[
                (P::Joint, 0.05),
                (P::BrickWidth, least(s).max(0.1)),
                (P::BrickHeight, least(s).max(0.1)),
            ],
        ),
        GeneratorKind::LeafForest => with(kind, &[(P::Density, 1.0), (P::LeafSize, least(s))]),
        GeneratorKind::SeaWaves => with(
            kind,
            &[(P::RingSpacing, least(s / 4.0)), (P::ScaleRadius, least(s))],
        ),
        GeneratorKind::Waves => with(
            kind,
            &[
                (P::Wavelength, least(s)),
                (P::Amplitude, s / 4.0),
                (P::Spacing, least(s / 2.0)),
            ],
        ),
        GeneratorKind::InterpolationWaves => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let lines = (1000.0 / s).clamp(2.0, 1_000_000.0) as u32;
            PatternGenerator::InterpolationWaves {
                lines: PatternCount::new(lines).unwrap(),
            }
        }
    }
}

/// The finest scale of `kind` that is still made on `outlines`, by bisection in log scale between
/// 0.05 and 50; `None` when even the coarsest is refused.
fn finest_made(
    outlines: &Outlines,
    kind: GeneratorKind,
    mode: PatternMode,
    whole: bool,
) -> Option<f64> {
    let is_made = |s: f64| {
        pattern_pieces(
            outlines,
            &fill_whole(at_scale(kind, s), mode, whole),
            Length::from_mm(0.25),
            TOLERANCE,
        )
        .is_ok()
    };
    let (mut fine, mut coarse) = (0.05_f64, 50.0_f64);
    if is_made(fine) {
        return Some(fine);
    }
    if !is_made(coarse) {
        return None;
    }
    for _ in 0..12 {
        let middle = (fine * coarse).sqrt();
        if is_made(middle) {
            coarse = middle;
        } else {
            fine = middle;
        }
    }
    Some(coarse)
}

/// The work limit holds for every generator at the finest setting it still makes: nothing that is
/// made takes longer than [`WORST_BUDGET`], and what is refused is refused in about that time.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release only: bisection of thread CPU budgets"
)]
fn ac18_every_generator_at_its_finest_accepted_setting_stays_within_the_budget() {
    for (shape, outlines) in [("circle", circle()), ("star", star())] {
        for kind in GeneratorKind::ALL {
            for mode in [PatternMode::Lines, PatternMode::Shapes] {
                for &whole in wholes(kind) {
                    finest_within_budget(shape, &outlines, kind, mode, whole);
                }
            }
        }
    }
}

/// One case of the test above.
fn finest_within_budget(
    shape: &str,
    outlines: &Outlines,
    kind: GeneratorKind,
    mode: PatternMode,
    whole: bool,
) {
    let case = format!("{shape:<6} {:<20} {mode:?} whole {whole}", kind.id());
    let Some(s) = finest_made(outlines, kind, mode, whole) else {
        let (_, elapsed) = timed_on(outlines, &fill_whole(at_scale(kind, 50.0), mode, whole));
        println!("{case}: refused even at its coarsest, in {elapsed:?} CPU");
        assert_budget(kind.id(), elapsed, WORST_BUDGET);
        return;
    };
    let (result, elapsed) = timed_on(outlines, &fill_whole(at_scale(kind, s), mode, whole));
    let pieces = result.unwrap().len();
    println!("{case}: finest {s:.3}, {pieces:>6} pieces, {elapsed:?} CPU");
    assert_budget(kind.id(), elapsed, WORST_BUDGET);
    let (refused, elapsed) = timed_on(outlines, &fill_whole(at_scale(kind, s * 0.9), mode, whole));
    if refused.is_err() {
        println!("{case}: refused at {:.3} in {elapsed:?} CPU", s * 0.9);
        assert_budget(kind.id(), elapsed, WORST_BUDGET);
    }
}

/// Leaf forest's worst case (`review-architecture.md` change 4): Density 100 % on a box just
/// under the hard limit, where random throwing saturates and every further throw fails.
#[test]
fn ac18_leaf_forest_at_full_density_near_the_hard_limit_is_bounded() {
    #[allow(clippy::cast_precision_loss)]
    let r = (100.0 * 100.0 / (std::f64::consts::PI * (PATTERN_MAX_PIECES as f64) * 0.95)).sqrt();
    let generator = with(
        GeneratorKind::LeafForest,
        &[
            (PatternParam::Density, 1.0),
            (PatternParam::LeafSize, 2.0 * r),
        ],
    );
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let (result, elapsed) = timed(&fill(generator, mode));
        println!(
            "leaf forest at density 100 % near the hard limit {mode:?}: {:?}, {elapsed:?} CPU",
            result.as_ref().map(Vec::len)
        );
        assert_budget("leaf forest", elapsed, WORST_BUDGET);
    }
}

/// The spacing that gives about `pieces` dots on the square.
fn dots_for(pieces: usize) -> PatternGenerator {
    #[allow(clippy::cast_precision_loss)]
    let spacing = 100.0 / (pieces as f64).sqrt();
    with(
        GeneratorKind::Dots,
        &[
            (PatternParam::Diameter, spacing / 2.0),
            (PatternParam::Spacing, spacing),
        ],
    )
}

#[test]
fn ac18_the_note_limit_is_generated_and_the_hard_limit_is_refused_at_once() {
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let (result, elapsed) = timed(&fill(dots_for(PATTERN_NOTE_PIECES), mode));
        let count = result.unwrap().len();
        println!("dots near the note limit {mode:?}: {count} pieces, {elapsed:?} CPU");
        assert_budget("dots", elapsed, WORST_BUDGET);
    }
    let million = with(
        GeneratorKind::Dots,
        &[(PatternParam::Diameter, 0.05), (PatternParam::Spacing, 0.1)],
    );
    let (result, elapsed) = timed(&fill(million, PatternMode::Lines));
    println!("dots over the hard limit: {result:?}, {elapsed:?} CPU");
    assert!(matches!(result, Err(PatternError::TooMany { .. })));
    assert_budget("refusal", elapsed, REFUSAL_BUDGET);
    // The acceptance report's case: Interpolation waves with many lines, refused before any work.
    let lines = PatternGenerator::InterpolationWaves {
        lines: PatternCount::new(100_000).unwrap(),
    };
    let (result, elapsed) = timed(&fill(lines, PatternMode::Lines));
    println!("interpolation waves, 100,000 lines: {result:?}, {elapsed:?} CPU");
    assert_eq!(result, Err(PatternError::TooDetailed));
    assert_budget("refusal", elapsed, REFUSAL_BUDGET);
}

/// The measurement behind the hard limit: dots at the hard limit itself (printed only).
#[test]
fn ac18_measure_the_hard_limit() {
    let (result, elapsed) = timed(&fill(
        dots_for(PATTERN_MAX_PIECES * 9 / 10),
        PatternMode::Lines,
    ));
    println!(
        "dots near the hard limit: {:?}, {elapsed:?} CPU",
        result.map(|p| p.len())
    );
}
