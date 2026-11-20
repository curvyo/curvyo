//! Tester acceptance cases for `specs/0123-raster-engraving` in the laser core, written from the
//! specification before the implementation was read: the dot values and the seven dither modes
//! (criteria 6, 8 to 10) against the tester's own reference (`tests/tester_raster_0123`), the dot
//! map and G-code goldens of criterion 40 read back by the tester's own GRBL reader (burned length
//! equals dots times Interval, S per run, M3 and M4, bidirectional rows, overscan, passes, the
//! Engrave + Outline order, white writes nothing), and the material record format 3 of criterion
//! 45 (Frage 237 A).
#![allow(
    clippy::many_single_char_names,
    clippy::naive_bytecount,
    clippy::redundant_closure_for_method_calls,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::needless_range_loop,
    missing_docs
)]

mod job_fixtures;
mod tester_raster_0123;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{AssetHash, Color, GradientBox, GradientFill, PlacedGradient, Point};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::job::LeafGeometry;
use curvyo_laser_core::library::read_record;
use curvyo_laser_core::settings::{Dither, ImageSettings, Interval, OperationOptions, ScanAngle};
use job_fixtures::{circle, frame, plain};
use tester_raster_0123::{Mode, burned, dither, map_text, parse_map, read};

/// Superseded by the Dither on the picture (2026-11-20): the picture leaf holds the dither the
/// operation's `options` name (ADR 0072 amendment A4), so the dot maps keep their bytes.
fn raster_dot_map(leaf: &LeafGeometry, options: &OperationOptions, interval: Interval) -> String {
    curvyo_laser_core::job::raster_dot_map(
        &job_fixtures::with_dither(leaf.clone(), job_fixtures::dither_of(&options.image)),
        options,
        interval,
    )
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn image(w: u32, h: u32, mut px: impl FnMut(u32, u32) -> [u8; 4]) -> WorkingImage {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    }
}

fn grey_img(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> WorkingImage {
    image(w, h, |x, y| {
        let v = g(x, y);
        [v, v, v, 255]
    })
}

/// An image leaf one dot per pixel at Interval `i` mm: frame `w * i` by `h * i` at (10, 10).
fn leaf(img: WorkingImage, i: f64) -> LeafGeometry {
    let (w, h) = (f64::from(img.width) * i, f64::from(img.height) * i);
    LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(10.0, 10.0, w, h),
        asset: AssetHash::of(&img.pixels),
        pixels: Some(Arc::new(img)),
    }
}

fn opts(edit: impl FnOnce(&mut ImageSettings)) -> OperationOptions {
    let mut image = ImageSettings::DEFAULT;
    edit(&mut image);
    OperationOptions { image, ..plain() }
}

fn of(dither: Dither) -> OperationOptions {
    opts(|s| s.dither = dither)
}

fn dither_of(mode: Mode) -> Dither {
    match mode {
        Mode::Threshold(_) => Dither::Threshold,
        Mode::Ordered => Dither::Ordered,
        Mode::FloydSteinberg => Dither::FloydSteinberg,
        Mode::Jarvis => Dither::Jarvis,
        Mode::Atkinson => Dither::Atkinson,
        Mode::Stucki => Dither::Stucki,
    }
}

fn tenth() -> Interval {
    Interval::from_hundredths(10).unwrap()
}

fn map_of(l: &LeafGeometry, o: &OperationOptions) -> Vec<Vec<Option<bool>>> {
    parse_map(&raster_dot_map(l, o, tenth()))
}

fn grid_of(img: &WorkingImage) -> Vec<Vec<Option<u8>>> {
    (0..img.height)
        .map(|y| {
            (0..img.width)
                .map(|x| {
                    let i = ((y * img.width + x) * 4) as usize;
                    Some(img.pixels[i])
                })
                .collect()
        })
        .collect()
}

/// A small deterministic random source.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u8
    }
}

fn ramp16() -> WorkingImage {
    grey_img(16, 16, |x, _| (x * 17) as u8)
}

// ---- criterion 10: Threshold and Ordered are exact against the reference ---------------------

#[test]
fn c10_threshold_and_ordered_match_the_reference_on_random_greys() {
    let mut rng = Lcg(7);
    for round in 0..20 {
        let img = grey_img(24, 20, |_, _| rng.next());
        let g = grid_of(&img);
        let l = leaf(img, 0.1);
        for (mode, pct) in [
            (Mode::Threshold(50), 50u8),
            (Mode::Threshold(20), 20),
            (Mode::Threshold(83), 83),
            (Mode::Ordered, 50),
        ] {
            let o = opts(|s| {
                s.dither = dither_of(mode);
                s.threshold_pct = pct;
            });
            let got = map_of(&l, &o);
            let want = dither(&g, mode);
            assert_eq!(
                map_text(&got),
                map_text(&want),
                "round {round} {mode:?}: implementation vs reference"
            );
        }
    }
}

/// Error diffusion against the classic weights. The rounding of the spread error is left open by
/// the specification (fixed point, criterion 8), and error diffusion is chaotic under rounding, so
/// the implementation must match one fixed-point variant of the classic weights (any scale,
/// rounding and the 128 threshold) on every image, and the darkness of the exact one.
#[test]
fn c10_error_diffusions_follow_the_classic_weights() {
    let mut notes = Vec::new();
    for mode in [
        Mode::FloydSteinberg,
        Mode::Jarvis,
        Mode::Atkinson,
        Mode::Stucki,
    ] {
        let mut rng = Lcg(11);
        let mut dark_got = 0;
        let mut dark_want = 0;
        let mut total = 0;
        for round in 0..6 {
            let img = grey_img(32, 32, |_, _| rng.next());
            let g = grid_of(&img);
            let got = map_of(&leaf(img, 0.1), &of(dither_of(mode)));
            let (d, how) = tester_raster_0123::best_fixed_match(&g, mode, &got);
            notes.push(format!("{mode:?} #{round}: {d} ({how})"));
            assert!(
                d <= 10,
                "{mode:?} image {round}: {d} of 1024 dots differ at best ({how})"
            );
            total += 1024;
            dark_got += burned(&got);
            dark_want += burned(&dither(&g, mode));
        }
        let darkness = (dark_got as f64 - dark_want as f64).abs() / total as f64;
        assert!(
            darkness <= 0.01,
            "{mode:?}: {dark_got} vs {dark_want} burned"
        );
    }
    println!("{}", notes.join("\n"));
}

/// Uniform greys: the share of burned dots follows the darkness in the four diffusions and
/// Ordered, Atkinson drops a quarter of the error (lighter at light greys).
#[test]
fn c10_uniform_greys_burn_their_darkness_and_atkinson_drops_error() {
    for g in [32u8, 64, 96, 128, 160, 192, 224] {
        let l = leaf(grey_img(64, 64, |_, _| g), 0.1);
        let darkness = 1.0 - f64::from(g) / 255.0;
        for d in [
            Dither::Ordered,
            Dither::FloydSteinberg,
            Dither::Jarvis,
            Dither::Stucki,
        ] {
            let share = burned(&map_of(&l, &of(d))) as f64 / 4096.0;
            assert!(
                (share - darkness).abs() < 0.05,
                "{d:?} at grey {g}: {share} burned, darkness {darkness}"
            );
        }
    }
    let light = leaf(grey_img(64, 64, |_, _| 215), 0.1);
    let fs = burned(&map_of(&light, &of(Dither::FloydSteinberg)));
    let at = burned(&map_of(&light, &of(Dither::Atkinson)));
    assert!(
        at < fs,
        "Atkinson drops error: {at} vs Floyd-Steinberg {fs}"
    );
}

#[test]
fn c10_black_burns_fully_and_white_burns_nothing_in_every_mode() {
    let black = leaf(grey_img(20, 20, |_, _| 0), 0.1);
    let white = leaf(grey_img(20, 20, |_, _| 255), 0.1);
    for d in [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Atkinson,
        Dither::Stucki,
        Dither::Grey,
    ] {
        let b = raster_dot_map(&black, &of(d), tenth());
        let w = raster_dot_map(&white, &of(d), tenth());
        if d == Dither::Grey {
            assert!(
                b.lines()
                    .all(|l| l.len() == 40 && l.as_bytes().chunks(2).all(|c| c == b"ff")),
                "{d:?} black: {b}"
            );
            assert!(
                w.lines()
                    .all(|l| l.as_bytes().chunks(2).all(|c| c == b"00")),
                "{d:?} white: {w}"
            );
        } else {
            assert_eq!(burned(&parse_map(&b)), 400, "{d:?} black");
            assert_eq!(burned(&parse_map(&w)), 0, "{d:?} white");
        }
    }
    // Threshold just above 0 % still burns black fully.
    let t1 = opts(|s| {
        s.dither = Dither::Threshold;
        s.threshold_pct = 1;
    });
    assert_eq!(burned(&map_of(&black, &t1)), 400);
}

/// The spec's test: a 16-step ramp, more darkness gives at least as many burned dots.
#[test]
fn c10_a_16_step_ramp_is_monotonic_in_all_six_modes() {
    // Each step 8 dots wide, 64 rows, so every step sees whole 8 x 8 matrices.
    let l = leaf(grey_img(128, 64, |x, _| ((x / 8) * 17) as u8), 0.1);
    for d in [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Atkinson,
        Dither::Stucki,
    ] {
        let m = map_of(&l, &of(d));
        let counts: Vec<usize> = (0..16)
            .map(|s| {
                m.iter()
                    .map(|r| (s * 8..s * 8 + 8).filter(|&c| r[c] == Some(true)).count())
                    .sum()
            })
            .collect();
        for w in counts.windows(2) {
            assert!(w[0] >= w[1], "{d:?}: {counts:?}");
        }
        assert_eq!(counts[0], 512, "{d:?}: black step burns fully");
        assert_eq!(counts[15], 0, "{d:?}: white step burns nothing");
    }
}

/// Error diffusion always runs in the direction of increasing column: Bidirectional fill never
/// changes the dots.
#[test]
fn c10_bidirectional_never_changes_the_dots() {
    let mut rng = Lcg(3);
    let img = grey_img(30, 30, |_, _| rng.next());
    let l = leaf(img, 0.1);
    for d in [
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Atkinson,
        Dither::Stucki,
    ] {
        let on = raster_dot_map(&l, &opts(|s| s.dither = d), tenth());
        let mut o = opts(|s| s.dither = d);
        o.bidirectional = false;
        let off = raster_dot_map(&l, &o, tenth());
        assert_eq!(on, off, "{d:?}");
    }
}

// ---- criterion 10 Grey, criterion 9 adjustments ----------------------------------------------

#[test]
fn c10_grey_levels_follow_darkness() {
    let m = raster_dot_map(&leaf(ramp16(), 0.1), &of(Dither::Grey), tenth());
    let first = m.lines().next().unwrap();
    let levels: Vec<u8> = first
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect();
    let want: Vec<u8> = (0..16).map(|x| 255 - (x * 17) as u8).collect();
    assert_eq!(levels, want);
}

#[test]
fn c9_brightness_plus_100_engraves_nothing_in_any_mode() {
    let l = leaf(grey_img(16, 16, |x, y| ((x * 16 + y) % 256) as u8), 0.1);
    for d in [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Atkinson,
        Dither::Stucki,
        Dither::Grey,
    ] {
        let o = opts(|s| {
            s.dither = d;
            s.brightness_pct = 100;
        });
        let t = raster_dot_map(&l, &o, tenth());
        assert!(!t.contains('#'), "{d:?}: {t}");
        if d == Dither::Grey {
            assert!(
                t.lines()
                    .all(|l| l.as_bytes().chunks(2).all(|c| c == b"00")),
                "{t}"
            );
        }
    }
}

#[test]
fn c9_invert_black_engraves_nothing_and_white_burns_whole() {
    let black = leaf(grey_img(10, 10, |_, _| 0), 0.1);
    let white = leaf(grey_img(10, 10, |_, _| 255), 0.1);
    for d in [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Stucki,
    ] {
        let o = opts(|s| {
            s.dither = d;
            s.invert = true;
        });
        assert_eq!(burned(&map_of(&black, &o)), 0, "{d:?}");
        assert_eq!(burned(&map_of(&white, &o)), 100, "{d:?}");
    }
}

#[test]
fn c9_gamma_above_one_lightens_and_contrast_spreads() {
    let l = leaf(grey_img(64, 64, |_, _| 110), 0.1);
    let base = burned(&map_of(&l, &of(Dither::FloydSteinberg)));
    let g2 = burned(&map_of(&l, &opts(|s| s.gamma_hundredths = 200)));
    let g05 = burned(&map_of(&l, &opts(|s| s.gamma_hundredths = 50)));
    assert!(
        g2 < base && base < g05,
        "gamma 2: {g2}, 1: {base}, 0.5: {g05}"
    );
    // Contrast +100 pushes a dark-of-middle grey toward black, -100 toward the middle.
    let c_hi = burned(&map_of(&l, &opts(|s| s.contrast_pct = 100)));
    let c_lo = burned(&map_of(&l, &opts(|s| s.contrast_pct = -100)));
    assert!(c_hi > base, "contrast up {c_hi} vs {base}");
    assert!(c_lo < base, "contrast down {c_lo} vs {base}");
    // Brightness -50 darkens.
    let b = burned(&map_of(&l, &opts(|s| s.brightness_pct = -50)));
    assert!(b > base, "{b} vs {base}");
}

// ---- criterion 6: luminance, alpha, resampling ---------------------------------------------

#[test]
fn c6_luminance_weights_colour_pixels() {
    // Pure red/green/blue in Grey mode: levels 255 - luma.
    for (px, luma) in [
        ([255, 0, 0, 255], 0.2126 * 255.0),
        ([0, 255, 0, 255], 0.7152 * 255.0),
        ([0, 0, 255, 255], 0.0722 * 255.0),
    ] {
        let l = leaf(image(4, 4, |_, _| px), 0.1);
        let m = raster_dot_map(&l, &of(Dither::Grey), tenth());
        let level = u8::from_str_radix(&m[0..2], 16).unwrap();
        let want = 255.0 - luma;
        assert!(
            (f64::from(level) - want).abs() <= 1.5,
            "{px:?}: level {level}, want {want}"
        );
    }
}

#[test]
fn c6_alpha_blends_over_white() {
    let clear = leaf(image(8, 8, |_, _| [0, 0, 0, 0]), 0.1);
    for d in [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Grey,
    ] {
        let t = raster_dot_map(&clear, &of(d), tenth());
        assert!(!t.contains('#') && !t.contains("ff"), "{d:?}: {t}");
    }
    let half = leaf(image(8, 8, |_, _| [0, 0, 0, 128]), 0.1);
    let m = raster_dot_map(&half, &of(Dither::Grey), tenth());
    let level = u8::from_str_radix(&m[0..2], 16).unwrap();
    assert!(
        (i32::from(level) - 128).abs() <= 2,
        "half transparent black: {level}"
    );
}

#[test]
fn c6_a_finer_source_is_averaged_and_a_coarser_one_is_smooth() {
    // A 1-pixel checker 4x finer than the dots reads as mid grey (box average), not as an
    // alias of black or white.
    let fine = image(64, 64, |x, y| {
        let v = if (x + y) % 2 == 0 { 0 } else { 255 };
        [v, v, v, 255]
    });
    let l = LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(0.0, 0.0, 1.6, 1.6),
        asset: AssetHash::of(&fine.pixels),
        pixels: Some(Arc::new(fine)),
    };
    let m = raster_dot_map(&l, &of(Dither::Grey), tenth());
    for line in m.lines() {
        for c in line.as_bytes().chunks(2) {
            let v = u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap();
            assert!((i32::from(v) - 128).abs() <= 8, "checker reads {v}: {m}");
        }
    }
    // A 2 x 1 picture (black, white) over 40 dots: a smooth ramp, many levels, not 2 blocks.
    let coarse = image(2, 1, |x, _| if x == 0 { [0, 0, 0, 255] } else { [255; 4] });
    let l = LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(0.0, 0.0, 4.0, 0.1),
        asset: AssetHash::of(&coarse.pixels),
        pixels: Some(Arc::new(coarse)),
    };
    let m = raster_dot_map(&l, &of(Dither::Grey), tenth());
    let row = m.lines().next().unwrap();
    let levels: std::collections::BTreeSet<&[u8]> = row.as_bytes().chunks(2).collect();
    assert!(levels.len() >= 10, "bilinear gives a shade: {row}");
}

#[test]
fn c5_a_1x1_image_is_one_value_over_its_whole_frame() {
    let one = image(1, 1, |_, _| [0, 0, 0, 255]);
    let l = LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(0.0, 0.0, 2.0, 1.0),
        asset: AssetHash::of(&one.pixels),
        pixels: Some(Arc::new(one)),
    };
    let m = map_of(&l, &of(Dither::Threshold));
    assert_eq!(m.len(), 10);
    assert!(
        m.iter()
            .all(|r| r.len() == 20 && r.iter().all(|d| *d == Some(true)))
    );
}

/// No dot outside the frame: a 30 degree scan angle keeps every dot centre inside.
#[test]
fn c5_scan_angle_dots_stay_in_the_frame_and_count_matches_the_area() {
    let black = grey_img(16, 16, |_, _| 0);
    let l = LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(0.0, 0.0, 1.6, 1.6),
        asset: AssetHash::of(&black.pixels),
        pixels: Some(Arc::new(black)),
    };
    let mut o = of(Dither::Threshold);
    o.scan_angle = ScanAngle::from_degrees(30.0).unwrap();
    let n = burned(&map_of(&l, &o));
    // 2.56 mm² / 0.01 mm² = 256 dots, within an edge row of dots.
    assert!((200..=300).contains(&n), "{n} dots for 256");
}

/// Criterion 8: same input, same output; the map has no state between calls.
#[test]
fn c8_dot_maps_are_identical_on_every_call() {
    let mut rng = Lcg(99);
    let img = grey_img(40, 40, |_, _| rng.next());
    let l = leaf(img, 0.1);
    for d in [
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Grey,
        Dither::Ordered,
    ] {
        let a = raster_dot_map(&l, &of(d), tenth());
        let b = raster_dot_map(&l, &of(d), tenth());
        assert_eq!(a, b);
    }
}

// ---- gradient shape (criteria 3, 5, 24) -----------------------------------------------------

#[test]
fn c24_gradient_black_to_white_in_grey_mode_runs_full_to_none() {
    let b = GradientBox::axis_aligned(Point::new(0.0, 0.0), Point::new(10.0, 2.0));
    let l = LeafGeometry::Gradient {
        outlines: vec![job_fixtures::rect(0.0, 0.0, 10.0, 2.0)],
        gradient: PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &b),
    };
    let m = raster_dot_map(&l, &of(Dither::Grey), tenth());
    let row: Vec<u8> = m
        .lines()
        .next()
        .unwrap()
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(row.len(), 100);
    assert!(row[0] >= 250, "leftmost full: {}", row[0]);
    assert!(
        (i32::from(row[50]) - 128).abs() <= 8,
        "middle half: {}",
        row[50]
    );
    assert!(row[99] <= 5, "rightmost none: {}", row[99]);
    for w in row.windows(2) {
        assert!(w[0] >= w[1], "{row:?}");
    }
}

#[test]
fn c5_gradient_dots_only_inside_the_outline() {
    let b = GradientBox::axis_aligned(Point::new(0.0, 0.0), Point::new(4.0, 4.0));
    let l = LeafGeometry::Gradient {
        outlines: vec![circle(2.0, 2.0, 2.0)],
        gradient: PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &b),
    };
    let mut o = of(Dither::Threshold);
    o.image.threshold_pct = 100;
    let m = map_of(&l, &o);
    // Every dot of the grid (inside the circle) burns at threshold 100 % but the white end.
    let n = burned(&m);
    let area = std::f64::consts::PI * 4.0 / 0.01;
    assert!(
        n as f64 <= area * 1.05 && n as f64 > area * 0.85,
        "{n} vs circle {area}"
    );
    // Corners of the box are outside the frame: no dot there.
    assert!(
        m[0].first().copied().flatten().is_none(),
        "{}",
        map_text(&m)
    );
}

// ---- criterion 40: the dot map goldens agree with the reference -----------------------------

fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures().join("raster").join(name)).unwrap()
}

#[test]
fn c40_dot_map_goldens_match_the_reference() {
    let g = grid_of(&ramp16());
    for (file, mode) in [
        ("ramp_threshold_50.txt", Mode::Threshold(50)),
        ("ramp_ordered.txt", Mode::Ordered),
    ] {
        assert_eq!(golden(file), map_text(&dither(&g, mode)), "{file}");
    }
    for (file, mode) in [
        ("ramp_floyd_steinberg.txt", Mode::FloydSteinberg),
        ("ramp_jarvis.txt", Mode::Jarvis),
        ("ramp_atkinson.txt", Mode::Atkinson),
        ("ramp_stucki.txt", Mode::Stucki),
    ] {
        let (d, how) = tester_raster_0123::best_fixed_match(&g, mode, &parse_map(&golden(file)));
        assert_eq!(
            d, 0,
            "{file}: {d} of 256 dots differ from the classic weights ({how})"
        );
    }
    let grey = golden("ramp_grey.txt");
    for line in grey.lines() {
        assert_eq!(line, "ffeeddccbbaa99887766554433221100");
    }
    for name in [
        "ramp_brightness_gamma_invert.txt",
        "alpha_edge.txt",
        "scan_angle_30.txt",
    ] {
        assert_ne!(golden(name), "", "{name}");
    }
    // Alpha edge: black with alpha ramping to 0 at the right never burns its transparent end.
    for line in golden("alpha_edge.txt").lines() {
        assert!(line.len() <= 16);
        assert_ne!(line.as_bytes().get(15).copied(), Some(b'#'), "{line}");
    }
}

// ---- criterion 40: the GRBL goldens read back by the tester's reader ------------------------

fn gcode(name: &str) -> tester_raster_0123::Gcode {
    let bytes = std::fs::read(fixtures().join("output/grbl").join(name)).unwrap();
    read(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The ramp goldens: 16 x 16 dots of 0.5 mm at (20, 20.25) page, bottom-left origin on a
/// 300 mm high bed, so the first row centre is at Y 279.75.
fn ramp_map(g: &tester_raster_0123::Gcode) -> Vec<Vec<Option<u32>>> {
    g.dot_map(20.0, 279.75, 0.5, 16, 16)
}

#[test]
fn c40_dither_goldens_burn_exactly_the_dots_of_their_dot_map() {
    let ramp = grid_of(&ramp16());
    for (file, map, mode) in [
        (
            "raster_threshold.gcode",
            "ramp_threshold_50.txt",
            Mode::Threshold(50),
        ),
        ("raster_ordered.gcode", "ramp_ordered.txt", Mode::Ordered),
        (
            "raster_floyd_steinberg.gcode",
            "ramp_floyd_steinberg.txt",
            Mode::FloydSteinberg,
        ),
        ("raster_jarvis.gcode", "ramp_jarvis.txt", Mode::Jarvis),
        ("raster_atkinson.gcode", "ramp_atkinson.txt", Mode::Atkinson),
        ("raster_stucki.gcode", "ramp_stucki.txt", Mode::Stucki),
    ] {
        let g = gcode(file);
        let dots = burned(&parse_map(&golden(map)));
        assert!(
            (g.burn_mm() - dots as f64 * 0.5).abs() < 0.01,
            "{file}: burn {} mm vs {dots} dots x 0.5",
            g.burn_mm()
        );
        let from_file = tester_raster_0123::to_bool(&ramp_map(&g));
        assert_eq!(
            map_text(&from_file),
            golden(map),
            "{file}: the file burns the dot map"
        );
        let _ = (&ramp, mode);
        // One G1 per run, never one per dot.
        let runs: usize = g.rows().iter().map(|r| r.1.len()).sum();
        assert!(
            g.code_lines < dots + 40,
            "{file}: {} lines for {dots} dots",
            g.code_lines
        );
        assert!(runs <= dots);
        // M4 dynamic: every burn under M4, S 600 (60 % of 1000).
        assert!(
            g.burns().iter().all(|m| m.spindle == 4 && m.s == 600),
            "{file}"
        );
        // Bidirectional: consecutive burned rows alternate direction.
        let dirs: Vec<bool> = row_directions(&g);
        for w in dirs.windows(2) {
            assert_ne!(w[0], w[1], "{file}: rows alternate");
        }
        // No burn outside the frame (20..28 x 272..280).
        let (x0, y0, x1, y1) = g.burn_extent().unwrap();
        assert!(x0 >= 20.0 - 1e-6 && x1 <= 28.0 + 1e-6 && y0 >= 272.0 && y1 <= 280.0);
    }
}

/// Per row: true when its first burn runs in +X.
fn row_directions(g: &tester_raster_0123::Gcode) -> Vec<bool> {
    let mut out = Vec::new();
    let mut last_y: Option<f64> = None;
    for m in g.burns() {
        if last_y.is_none_or(|y| (m.from.1 - y).abs() > 1e-6) {
            out.push(m.to.0 > m.from.0);
            last_y = Some(m.from.1);
        }
    }
    out
}

#[test]
fn c19_grey_m4_golden_has_s_per_run_following_darkness() {
    let g = gcode("raster_grey_m4.gcode");
    let rows = g.rows();
    assert_eq!(rows.len(), 16);
    for (_, segs) in &rows {
        let mut segs = segs.clone();
        segs.sort_by(|a, b| a.0.total_cmp(&b.0));
        // 15 burned dots (the white one does not burn), powers falling to the right.
        assert_eq!(segs.len(), 15, "{segs:?}");
        for w in segs.windows(2) {
            assert!(w[0].2 > w[1].2, "{segs:?}");
        }
        assert_eq!(segs[0].2, 600, "black runs at the set's power");
    }
    assert!(g.burns().iter().all(|m| m.spindle == 4));
    assert!((g.burn_mm() - 16.0 * 15.0 * 0.5).abs() < 0.01);
}

#[test]
fn c19_m3_golden_turns_the_laser_on_and_off_per_run() {
    let g = gcode("raster_m3_constant_power.gcode");
    assert!(g.burns().iter().all(|m| m.spindle == 3));
    // Every G0 runs with the spindle off.
    assert!(g.moves.iter().filter(|m| m.rapid).all(|m| m.spindle == 5));
    let runs: usize = g.rows().iter().map(|r| r.1.len()).sum();
    let on = g.spindle_words.iter().filter(|&&w| w == 3).count();
    assert_eq!(on, runs + 1, "one M3 per run plus the header's");
    assert!(
        (g.burn_mm() - burned(&parse_map(&golden("ramp_threshold_50.txt"))) as f64 * 0.5).abs()
            < 0.01
    );
}

#[test]
fn c16_unidirectional_golden_runs_every_row_one_way() {
    let g = gcode("raster_unidirectional.gcode");
    let d = row_directions(&g);
    assert!(d.len() == 16 && d.iter().all(|&x| x == d[0]), "{d:?}");
    assert!((g.burn_mm() - 64.0).abs() < 0.01, "{}", g.burn_mm());
}

#[test]
fn c16_overscan_golden_extends_rows_with_the_laser_off() {
    let g = gcode("raster_overscan.gcode");
    let plain = gcode("raster_threshold.gcode");
    assert!((g.burn_mm() - plain.burn_mm()).abs() < 0.01, "same burn");
    assert!(g.off_mm() > 0.0, "laser-off run-out");
    // Every off G1 lies on a burned row's line and outside the burn extent in X.
    let (x0, _, x1, _) = g.burn_extent().unwrap();
    for m in g.moves.iter().filter(|m| !m.rapid && !m.burn) {
        assert!((m.from.1 - m.to.1).abs() < 1e-6);
        let lo = m.from.0.min(m.to.0);
        let hi = m.from.0.max(m.to.0);
        assert!(hi <= x0 + 1e-6 || lo >= x1 - 1e-6, "{m:?}");
    }
}

#[test]
fn c17_passes_golden_burns_the_same_rows_again() {
    let g = gcode("raster_passes.gcode");
    let one = gcode("raster_threshold.gcode");
    let k = (g.burn_mm() / one.burn_mm()).round();
    assert!(k >= 2.0, "passes: {k}");
    assert!((g.burn_mm() - k * one.burn_mm()).abs() < 0.01);
}

// Superseded by the Image kind rework (2026-11-19): `raster_engrave_outline.gcode` is removed
// (Engrave + Outline hatches an image's frame and traces it, criterion 1); the dots-then-frame
// order is now an image in an Image and a Cut operation, `raster_image_and_cut.gcode`.
#[test]
fn c2_image_and_cut_golden_runs_dots_then_the_frame_with_the_cut_set() {
    let g = gcode("raster_image_and_cut.gcode");
    let burns = g.burns();
    let raster_s = burns[0].s;
    let first_other = burns.iter().position(|m| m.s != raster_s).expect("a frame");
    assert!(
        burns[first_other..].iter().all(|m| m.s != raster_s),
        "frame after all dots"
    );
    let outline: f64 = burns[first_other..].iter().map(|m| m.len()).sum();
    assert!((outline - 32.0).abs() < 0.05, "the 8 x 8 frame: {outline}");
}

#[test]
fn c15_white_golden_writes_no_burn() {
    // A white 8 x 8 mm image in Engrave (S600) and a Cut line at S900 on Y 260.
    let g = gcode("raster_white_skipped.gcode");
    assert!(
        g.burns()
            .iter()
            .all(|m| m.s == 900 && (m.from.1 - 260.0).abs() < 1e-6)
    );
    assert!(
        g.moves.iter().all(|m| m.burn || m.rapid),
        "no sweep and no laser-off G1 over white"
    );
    assert!(!g.s_words.iter().any(|&(_, s)| s == 600));
}

#[test]
fn c40_scan_angle_origin_gradient_goldens_read_back() {
    let a = gcode("raster_scan_angle.gcode");
    assert!(
        a.burns().iter().any(|m| (m.from.1 - m.to.1).abs() > 1e-3),
        "turned rows"
    );
    let tl = gcode("raster_origin_corner_tl.gcode");
    assert!((tl.burn_mm() - 64.0).abs() < 0.01);
    let (_, y0, _, y1) = tl.burn_extent().unwrap();
    assert!(
        y0 >= 20.0 - 1e-6 && y1 <= 28.0 + 1e-6,
        "top-left origin: Y as on the page"
    );
    let e = gcode("raster_gradient_shape.gcode");
    let (x0, y0, x1, y1) = e.burn_extent().unwrap();
    assert!(x0 >= 20.0 - 1e-6 && x1 <= 30.0 + 1e-6, "{x0} {x1}");
    assert!(y1 - y0 <= 10.0 + 1e-6);
    // Ellipse with a black-to-white gradient: the left half burns more than the right half.
    let mid = 25.0;
    let left: f64 = e
        .burns()
        .iter()
        .filter(|m| m.from.0.max(m.to.0) <= mid + 1e-6)
        .map(|m| m.len())
        .sum();
    let right: f64 = e
        .burns()
        .iter()
        .filter(|m| m.from.0.min(m.to.0) >= mid - 1e-6)
        .map(|m| m.len())
        .sum();
    assert!(left > right, "{left} vs {right}");
}

#[test]
fn c19_goldens_have_no_date_path_or_random_number() {
    for entry in std::fs::read_dir(fixtures().join("output/grbl")).unwrap() {
        let p = entry.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("raster_") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for bad in ["2026", "/home", "\\", "tmp", ":\\"] {
            assert!(!text.contains(bad), "{name} holds {bad}");
        }
    }
}

// ---- criterion 45: material record format 3 --------------------------------------------------

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

fn material(name: &str) -> String {
    std::fs::read_to_string(fixtures().join("materials").join(name)).unwrap()
}

// Superseded by the Image kind rework (2026-11-19): the Fill choice left the recipe; the dither
// is an Image test's (criterion 44), and `format3_gradient_test.json` became
// `format3_image_test.json`.
#[test]
fn c45_format_3_keeps_the_dither_of_an_image_test() {
    let m = read_record(&format!("{ID}.json"), &material("format3_image_test.json")).unwrap();
    let dithers: Vec<Option<Dither>> = m.tests.iter().map(|t| t.recipe.dither()).collect();
    assert!(dithers.contains(&Some(Dither::Ordered)), "{dithers:?}");
}

#[test]
fn c45_format_2_reads_with_no_image_test() {
    let m = read_record(&format!("{ID}.json"), &material("format2_reads_solid.json")).unwrap();
    assert_ne!(m.tests.len(), 0);
    assert!(m.tests.iter().all(|t| t.recipe.dither().is_none()));
}

#[test]
fn c45_an_image_recipe_in_format_2_or_a_grey_dither_is_damage() {
    let three = material("format3_image_test.json");
    let two = three.replacen("\"format_version\": 3", "\"format_version\": 2", 1);
    assert_ne!(two, three);
    let r2 = read_record(&format!("{ID}.json"), &two);
    let damaged_2 = r2.as_ref().map_or(true, |m| {
        m.tests.iter().all(|t| t.recipe.dither().is_none()) && m.tests.len() < 3
    });
    assert!(damaged_2, "an Image test in format 2 is damage: {r2:?}");
    let grey = three.replacen("\"dither\": \"ordered\"", "\"dither\": \"grey\"", 1);
    assert_ne!(grey, three);
    let r = read_record(&format!("{ID}.json"), &grey);
    let refused = r.as_ref().map_or(true, |m| {
        !m.tests
            .iter()
            .any(|t| t.recipe.dither() == Some(Dither::Grey))
    });
    assert!(refused, "Grey is never a grid dither");
}

// ---- Image kind rework (2026-11-19): goldens, the Image set and the Image test --------------

/// `raster_image_frame_kinds.gcode` (criteria 1, 40): the 8 x 8 mm image at (20, 20) in Cut,
/// Score, Cut-out, Engrave and Engrave + Outline writes frame lines and hatch, never a dot row
/// beyond the hatch of the frame: each kind's S burns exactly its rectangle.
#[test]
fn c1_c40_image_frame_kinds_golden_is_frames_and_hatch_only() {
    let g = gcode("raster_image_frame_kinds.gcode");
    let by_s = |s: u32| -> f64 { g.burns().iter().filter(|m| m.s == s).map(|m| m.len()).sum() };
    // Cut 90 %, Score 20 %, Cut-out 80 %: one closed 8 x 8 line each.
    for (s, what) in [
        (900, "Cut"),
        (200, "Score"),
        (800, "Cut-out"),
        (300, "the outline"),
    ] {
        assert!((by_s(s) - 32.0).abs() < 0.01, "{what}: {}", by_s(s));
        let lines: Vec<_> = g.burns().into_iter().filter(|m| m.s == s).collect();
        assert_eq!(lines.len(), 4, "{what}: one rectangle of four sides");
        assert!(
            (lines[0].from.0 - lines[3].to.0).abs() < 1e-6
                && (lines[0].from.1 - lines[3].to.1).abs() < 1e-6,
            "{what}: closed"
        );
    }
    // Engrave and Engrave + Outline at 60 %, 0.5 mm: 16 hatch lines of 8 mm each.
    assert!(
        (by_s(600) - 2.0 * 16.0 * 8.0).abs() < 0.01,
        "two hatches of 16 x 8 mm: {}",
        by_s(600)
    );
    for m in g.burns().into_iter().filter(|m| m.s == 600) {
        assert!((m.from.1 - m.to.1).abs() < 1e-6, "a horizontal hatch line");
        assert!(
            (m.len() - 8.0).abs() < 1e-6,
            "a hatch line spans the frame, not a dot run: {m:?}"
        );
        assert!(m.from.0.min(m.to.0) >= 20.0 - 1e-6 && m.from.0.max(m.to.0) <= 28.0 + 1e-6);
    }
    let (x0, y0, x1, y1) = g.burn_extent().unwrap();
    assert!(
        (x0 - 20.0).abs() < 1e-6 && (x1 - 28.0).abs() < 1e-6,
        "inside the frame: {x0} {x1}"
    );
    assert!(
        (y0 - 272.0).abs() < 1e-6 && (y1 - 280.0).abs() < 1e-6,
        "{y0} {y1}"
    );
}

/// `raster_image_and_cut.gcode`: the dots' burned length is the burned dot count times the
/// Interval (criterion 40) and the frame follows.
#[test]
fn c40_image_and_cut_golden_burns_whole_dots_then_the_frame() {
    let g = gcode("raster_image_and_cut.gcode");
    let dots: usize = burned_power(&ramp_map(&g));
    let burned: f64 = g
        .burns()
        .iter()
        .filter(|m| m.s == 600)
        .map(|m| m.len())
        .sum();
    assert!(dots > 0);
    assert!(
        (burned - dots as f64 * 0.5).abs() < 0.01,
        "{burned} mm = {dots} dots x 0.5"
    );
    let frame: f64 = g
        .burns()
        .iter()
        .filter(|m| m.s == 900)
        .map(|m| m.len())
        .sum();
    assert!((frame - 32.0).abs() < 0.01, "{frame}");
}

fn burned_power(map: &[Vec<Option<u32>>]) -> usize {
    tester_raster_0123::burned_power(map)
}

/// Frage 270 B (criteria 13, 41): format 3 holds the Image set; a format-2 record that holds
/// one is damage; a record without one reads as no Image set.
#[test]
fn c41_format_3_holds_the_image_set_and_format_2_holding_one_is_damage() {
    use curvyo_laser_core::settings::SetKind;
    let three = material("format3_image_test.json");
    let m = read_record(&format!("{ID}.json"), &three).unwrap();
    let image = m.lasers[0]
        .sets
        .get(&SetKind::Image)
        .expect("the Image set");
    assert_eq!(image.numbers.interval.unwrap().hundredths(), 10);
    let engrave = m.lasers[0].sets.get(&SetKind::Engrave).unwrap();
    assert_ne!(
        engrave.numbers.interval.unwrap().hundredths(),
        10,
        "the two sets are their own"
    );
    let two = material("format2_image_set.json");
    assert!(
        read_record(&format!("{ID}.json"), &two).is_err(),
        "a format-2 record holding an Image set is damage"
    );
    // Without the Image set, a format-3 record reads with no Image set.
    let start = three.find("        \"image\": {").unwrap();
    let end = start + three[start..].find("        }\n      }").unwrap() + "        }\n".len();
    let mut none = three.clone();
    none.replace_range(start..end, "");
    let none = none.replacen(
        "\"source\": \"typed\"\n        },\n",
        "\"source\": \"typed\"\n        }\n",
        1,
    );
    let none = none.replacen(
        "\"kind\": \"image\",\n      \"set\"",
        "\"kind\": \"engrave\",\n      \"set\"",
        1,
    );
    match read_record(&format!("{ID}.json"), &none) {
        Ok(m) => assert!(!m.lasers[0].sets.contains_key(&SetKind::Image)),
        Err(e) => panic!("a record without an Image set is valid: {e:?}\n{none}"),
    }
}

/// Criterion 44: an Engrave test's recipe with a Dither is damage (Dither is the Image grid's).
#[test]
fn c44_an_engrave_recipe_with_a_dither_is_damage() {
    let three = material("format3_image_test.json");
    let engrave = three.replacen(
        "\"crosshatch\": false\n      },\n      \"lasered\": null",
        "\"crosshatch\": false,\n        \"dither\": \"ordered\"\n      },\n      \"lasered\": null",
        1,
    );
    assert_ne!(engrave, three, "the Engrave test got a dither");
    let r = read_record(&format!("{ID}.json"), &engrave);
    let refused = r.as_ref().map_or(true, |m| {
        !m.tests
            .iter()
            .any(|t| t.recipe.dither().is_some() && t.id.to_string() == "5SL1D")
    });
    assert!(refused, "an Engrave recipe never keeps a Dither: {r:?}");
    // An Image test without a dither reads Floyd-Steinberg or is damage, never an Engrave.
    let no_dither = three.replacen(",\n        \"dither\": \"ordered\"", "", 1);
    assert_ne!(no_dither, three);
    if let Ok(m) = read_record(&format!("{ID}.json"), &no_dither) {
        let t = m
            .tests
            .iter()
            .find(|t| t.id.to_string() == "9GR4D")
            .unwrap();
        assert_eq!(t.recipe.kind(), curvyo_laser_core::settings::SetKind::Image);
    }
}
