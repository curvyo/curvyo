//! The raster goldens of `specs/0123-raster-engraving` criterion 40: dot maps in
//! `tests/fixtures/raster/` (16 x 16 PNG inputs and their expected maps as text) and GRBL files in
//! `tests/fixtures/output/grbl/raster_*.gcode`. Each G-code golden is built twice and compared
//! byte for byte, then read back: the burned length equals the burned dot count times the
//! Interval, and the file's total length equals the timeline's. `CURVYO_UPDATE_GOLDEN=1`
//! rewrites the files; review the diff.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AssetHash, Color, GradientBox, GradientFill, PlacedGradient, Point, Vec2,
};
use curvyo_image_core::{WorkingImage, decode_working, encode_png};
use curvyo_laser_core::job::{
    JobInput, LeafGeometry, LeafOutline, MoveKind, PlanCache, Timeline, build_job, write,
};
use curvyo_laser_core::operations::OperationKind::{self, Cut, Engrave, EngraveOutline, Image};
use curvyo_laser_core::settings::{
    Dither, ImageSettings, Interval, OperationOptions, Overscan, ScanAngle,
};
use curvyo_laser_core::{FeedRate, OriginCorner};
use job_fixtures::{Page, circle, frame, grey_image, line, own, plain, settings};
use serde_json::json;

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

fn updating() -> bool {
    std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some()
}

/// `bytes` against the stored file `path` (written first when updating).
fn check(path: &PathBuf, bytes: &[u8]) {
    if updating() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let stored =
        std::fs::read(path).unwrap_or_else(|_| panic!("missing golden {}", path.display()));
    assert!(
        stored == bytes,
        "{} differs from its golden",
        path.display()
    );
}

/// The PNG input `name` (16 x 16), written from `make` when updating, read back and decoded.
fn png(name: &str, make: impl Fn() -> WorkingImage) -> WorkingImage {
    let path = fixtures().join("raster").join(format!("{name}.png"));
    let expected = make();
    if updating() {
        // Tests run in parallel: write once, atomically, so no test reads a half-written file.
        let bytes = encode_png(&expected).unwrap();
        if std::fs::read(&path).ok().as_ref() != Some(&bytes) {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let temp = path.with_extension(format!("png.{}", std::process::id()));
            std::fs::write(&temp, bytes).unwrap();
            std::fs::rename(&temp, &path).unwrap();
        }
    }
    let decoded = decode_working(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        decoded.pixels, expected.pixels,
        "{name}.png holds the fixture"
    );
    decoded
}

/// The 16-step grey ramp, black at the left, white at the right.
fn ramp() -> WorkingImage {
    png("ramp_16", || {
        grey_image(16, 16, |x, _| u8::try_from(x * 17).unwrap())
    })
}

/// Black whose alpha ramps from opaque at the left to transparent at the right.
fn alpha_edge() -> WorkingImage {
    png("alpha_edge_16", || {
        let mut image = grey_image(16, 16, |_, _| 0);
        for (i, px) in image.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            px[3] = 255 - u8::try_from((i % 16) * 17).unwrap();
        }
        image
    })
}

fn picture(x: f64, y: f64, w: f64, h: f64, pixels: WorkingImage) -> LeafGeometry {
    LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(x, y, w, h),
        asset: AssetHash::of(&pixels.pixels),
        pixels: Some(Arc::new(pixels)),
    }
}

fn options(edit: impl FnOnce(&mut ImageSettings)) -> OperationOptions {
    let mut image = ImageSettings::DEFAULT;
    edit(&mut image);
    OperationOptions { image, ..plain() }
}

fn mode(dither: Dither) -> OperationOptions {
    options(|s| s.dither = dither)
}

// ---- dot maps -------------------------------------------------------------------------------

fn dot_map(name: &str, leaf: &LeafGeometry, options: &OperationOptions) -> String {
    let text = raster_dot_map(leaf, options, Interval::from_hundredths(10).unwrap());
    check(
        &fixtures().join("raster").join(format!("{name}.txt")),
        text.as_bytes(),
    );
    text
}

#[test]
fn dot_maps_of_the_ramp_in_every_mode() {
    let leaf = picture(0.0, 0.0, 1.6, 1.6, ramp());
    for (name, dither) in [
        ("ramp_threshold_50", Dither::Threshold),
        ("ramp_ordered", Dither::Ordered),
        ("ramp_floyd_steinberg", Dither::FloydSteinberg),
        ("ramp_jarvis", Dither::Jarvis),
        ("ramp_atkinson", Dither::Atkinson),
        ("ramp_stucki", Dither::Stucki),
    ] {
        let text = dot_map(name, &leaf, &mode(dither));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 16, "{name}");
        assert!(
            lines.iter().all(|l| l.starts_with('#')),
            "{name}: black burns"
        );
        let burned = |column: usize| {
            lines
                .iter()
                .filter(|l| l.as_bytes().get(column) == Some(&b'#'))
                .count()
        };
        assert_eq!(burned(15), 0, "{name}: white never burns");
        assert!(burned(0) >= burned(14), "{name}: darker burns more");
    }
    let grey = dot_map("ramp_grey", &leaf, &mode(Dither::Grey));
    assert!(
        grey.lines().all(|l| l.starts_with("ff") && l.len() == 32),
        "{grey}"
    );
}

#[test]
fn dot_maps_of_the_adjustments_the_alpha_edge_and_the_scan_angle() {
    let leaf = picture(0.0, 0.0, 1.6, 1.6, ramp());
    let adjusted = options(|s| {
        s.dither = Dither::Threshold;
        s.brightness_pct = 20;
        s.gamma_hundredths = 150;
        s.invert = true;
    });
    let text = dot_map("ramp_brightness_gamma_invert", &leaf, &adjusted);
    assert!(
        text.lines().all(|l| l.starts_with('.') && l.ends_with('#')),
        "inverted:\n{text}"
    );
    let alpha = picture(0.0, 0.0, 1.6, 1.6, alpha_edge());
    let text = dot_map("alpha_edge", &alpha, &mode(Dither::Threshold));
    assert!(
        text.lines().all(|l| l.starts_with('#') && l.ends_with('.')),
        "{text}"
    );
    let mut turned = mode(Dither::FloydSteinberg);
    turned.scan_angle = ScanAngle::from_degrees(30.0).unwrap();
    let text = dot_map("scan_angle_30", &leaf, &turned);
    assert!(
        text.lines().count() > 16,
        "rows across the turned frame:\n{text}"
    );
}

#[test]
fn dot_maps_are_the_same_on_every_build() {
    let leaf = picture(0.0, 0.0, 1.6, 1.6, ramp());
    for dither in Dither::ALL {
        let o = mode(dither);
        let interval = Interval::from_hundredths(10).unwrap();
        assert_eq!(
            raster_dot_map(&leaf, &o, interval),
            raster_dot_map(&leaf, &o, interval)
        );
    }
}

// ---- GRBL -----------------------------------------------------------------------------------

/// The ramp at 8 x 8 mm, 16 x 16 dots of 0.5 mm.
fn ramp_leaf() -> LeafGeometry {
    picture(20.0, 20.0, 8.0, 8.0, ramp())
}

/// The Cut-out item of the image `leaf`: a Cut-out block takes outlines only, and the session
/// hands it an image as its frame, one closed ring (`curvyo-editor-wasm`
/// `session/output/input.rs`), so the ring is read from the image's own frame.
fn frame_ring(leaf: &LeafGeometry) -> LeafOutline {
    let LeafGeometry::Image { frame, .. } = leaf else {
        panic!("an image leaf");
    };
    LeafOutline {
        anchors: frame.iter().map(|p| (*p, Vec2::ZERO, Vec2::ZERO)).collect(),
        closed: true,
    }
}

/// An operation of `kind` at 60 %, 3,000 mm/min, 0.5 mm. Its name stays "Engrave 1" for every
/// kind, so the goldens built before the Image kind keep their bytes (ADR 0069 §13).
fn engrave(
    page: &mut Page,
    kind: OperationKind,
    passes: u8,
    options: OperationOptions,
    leaves: Vec<LeafGeometry>,
) {
    page.operation(
        kind,
        "Engrave 1",
        settings(
            kind,
            json!({"main": {"power_pct": 60, "speed_mm_min": 3000, "passes": passes, "interval_mm": 0.5}}),
        ),
        options,
        leaves,
    );
}

fn ramp_page(options: OperationOptions) -> Page {
    let mut page = Page::new();
    page.values().max_feed = FeedRate::new(12_000).unwrap();
    engrave(&mut page, Image, 1, options, vec![ramp_leaf()]);
    page
}

fn gcode(name: &str, input: &JobInput) -> String {
    let built = build_job(input, &[], &mut PlanCache::default());
    let format = input.machine.values.output_format;
    let bytes = write(format, &built.job);
    assert_eq!(
        bytes,
        write(
            format,
            &build_job(input, &[], &mut PlanCache::default()).job
        ),
        "{name}: byte-identical"
    );
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.is_ascii() && !text.contains('\r'));
    let timeline = Timeline::of(&built.job);
    // The burn the file holds: the timeline's burn with each sweep counted by its burned dots.
    let mut expected_burn = timeline.burn_mm();
    let mut from = built.job.home;
    for m in &built.job.moves {
        if let MoveKind::Sweep { sweep, .. } = m.kind {
            let sweep = built.job.sweep(sweep).unwrap();
            let interval =
                f64::from(built.job.blocks[m.block].numbers.interval.hundredths()) / 100.0;
            #[allow(clippy::cast_precision_loss)]
            let dots = sweep.burned_dots() as f64;
            expected_burn -= from.distance_mm(m.to) - dots * interval;
        }
        from = m.to;
    }
    let read = reader::read(&text);
    assert!(
        (read.burn_mm - expected_burn).abs() < 0.01,
        "{name}: burn {} vs {expected_burn}",
        read.burn_mm
    );
    let total = timeline.burn_mm() + timeline.travel_mm();
    assert!(
        (read.burn_mm + read.travel_mm - total).abs() < 0.01,
        "{name}: length"
    );
    assert!(
        text.lines().count() < 2_000,
        "{name}: never one line per dot"
    );
    check(&fixtures().join("output/grbl").join(name), text.as_bytes());
    text
}

#[test]
fn raster_dither_modes() {
    for (name, dither) in [
        ("raster_threshold.gcode", Dither::Threshold),
        ("raster_ordered.gcode", Dither::Ordered),
        ("raster_floyd_steinberg.gcode", Dither::FloydSteinberg),
        ("raster_jarvis.gcode", Dither::Jarvis),
        ("raster_atkinson.gcode", Dither::Atkinson),
        ("raster_stucki.gcode", Dither::Stucki),
    ] {
        let text = gcode(name, &ramp_page(mode(dither)).input);
        assert!(text.contains(" S600"), "{name}: the set's power");
        assert!(
            !text
                .lines()
                .any(|l| l.starts_with("G1") && l.contains(" X28.000") && l.contains(" S600")),
            "{name}: the white column never burns"
        );
    }
}

#[test]
fn raster_grey_m4() {
    let text = gcode(
        "raster_grey_m4.gcode",
        &ramp_page(options(|s| {
            s.dither = Dither::Grey;
            s.lowest_power_tenths = 100;
        }))
        .input,
    );
    let burned: Vec<u32> = text
        .lines()
        .filter_map(|l| {
            l.split_whitespace()
                .find_map(|w| w.strip_prefix('S')?.parse().ok())
        })
        .filter(|&s| s > 0)
        .collect();
    assert_eq!(burned.iter().max(), Some(&600), "black at the set's power");
    let lowest = *burned.iter().min().unwrap();
    assert!(
        (100..=140).contains(&lowest),
        "the lightest burned dot near Lowest power: {lowest}"
    );
}

#[test]
fn raster_unidirectional_scan_angle_and_overscan() {
    let mut o = mode(Dither::Threshold);
    o.bidirectional = false;
    gcode("raster_unidirectional.gcode", &ramp_page(o).input);
    let mut o = mode(Dither::Threshold);
    o.scan_angle = ScanAngle::from_degrees(30.0).unwrap();
    gcode("raster_scan_angle.gcode", &ramp_page(o).input);
    let mut o = mode(Dither::Threshold);
    o.overscan = Overscan::new(20).unwrap();
    gcode("raster_overscan.gcode", &ramp_page(o).input);
}

#[test]
fn raster_gradient_shape() {
    let frame = GradientBox::axis_aligned(Point::new(20.0, 20.0), Point::new(30.0, 30.0));
    let leaf = LeafGeometry::Gradient {
        outlines: vec![circle(25.0, 25.0, 5.0)],
        gradient: PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &frame),
    };
    let mut page = Page::new();
    engrave(
        &mut page,
        Image,
        1,
        mode(Dither::FloydSteinberg),
        vec![leaf],
    );
    gcode("raster_gradient_shape.gcode", &page.input);
}

#[test]
fn raster_passes() {
    let mut page = Page::new();
    engrave(
        &mut page,
        Image,
        2,
        mode(Dither::Threshold),
        vec![ramp_leaf()],
    );
    gcode("raster_passes.gcode", &page.input);
}

/// Criterion 1 (Frage 271 A): one image in Cut, Score, Cut-out, Engrave and Engrave + Outline
/// writes its frame lines and hatch, no dot.
#[test]
fn raster_image_frame_kinds() {
    let mut page = Page::new();
    let leaf = || ramp_leaf();
    page.operation(Cut, "Cut 1", own(Cut, 90.0, 300, 1), plain(), vec![leaf()]);
    page.operation(
        OperationKind::Score,
        "Score 1",
        own(OperationKind::Score, 20.0, 1000, 1),
        plain(),
        vec![leaf()],
    );
    page.cut_out(
        "Cut-out 1",
        own(OperationKind::CutOut, 80.0, 400, 1),
        0.0,
        vec![vec![vec![frame_ring(&leaf())]]],
    );
    engrave(&mut page, Engrave, 1, plain(), vec![leaf()]);
    page.operation(
        EngraveOutline,
        "Engrave + Outline 1",
        settings(
            EngraveOutline,
            json!({"main": {"power_pct": 60, "speed_mm_min": 3000, "passes": 1, "interval_mm": 0.5},
                   "outline": {"power_pct": 30, "speed_mm_min": 1000, "passes": 1}}),
        ),
        plain(),
        vec![leaf()],
    );
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    assert!(built.job.sweeps.is_empty(), "no dot outside Image");
    let text = gcode("raster_image_frame_kinds.gcode", &page.input);
    for s in [" S900", " S200", " S800", " S600", " S300"] {
        assert!(text.contains(s), "{s}: every kind writes");
    }
}

/// Criterion 1: an image in an Image and a Cut operation: the dots, then the frame.
#[test]
fn raster_image_and_cut() {
    let mut page = Page::new();
    engrave(
        &mut page,
        Image,
        1,
        mode(Dither::Threshold),
        vec![ramp_leaf()],
    );
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![ramp_leaf()],
    );
    let text = gcode("raster_image_and_cut.gcode", &page.input);
    assert!(
        text.rfind(" S600").unwrap() < text.find(" S900").unwrap(),
        "dots in the Marking section first"
    );
}

#[test]
fn raster_m3_constant_power_and_origin_corner() {
    let mut page = ramp_page(mode(Dither::Threshold));
    page.values().laser_mode = false;
    let text = gcode("raster_m3_constant_power.gcode", &page.input);
    assert!(text.lines().filter(|l| *l == "M5").count() > 16);
    let page = ramp_page(mode(Dither::Threshold)).corner(OriginCorner::TopLeft);
    gcode("raster_origin_corner_tl.gcode", &page.input);
}

#[test]
fn raster_white_skipped() {
    let mut page = Page::new();
    engrave(
        &mut page,
        Image,
        1,
        mode(Dither::FloydSteinberg),
        vec![picture(
            20.0,
            20.0,
            8.0,
            8.0,
            grey_image(16, 16, |_, _| 255),
        )],
    );
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![job_fixtures::leaf(vec![line(
            Point::new(20.0, 40.0),
            Point::new(40.0, 40.0),
        )])],
    );
    let text = gcode("raster_white_skipped.gcode", &page.input);
    assert!(!text.contains(" S600"), "white writes nothing");
    assert!(text.contains(" S900"));
}

mod reader {
    pub struct Read {
        pub burn_mm: f64,
        pub travel_mm: f64,
    }

    fn word(line: &str, letter: char) -> Option<f64> {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse().ok()))
    }

    /// Burn is a laser-on `G1` with `S` above 0; everything else that moves X and Y is travel.
    pub fn read(text: &str) -> Read {
        let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
        let (mut m3, mut on) = (false, true);
        let mut out = Read {
            burn_mm: 0.0,
            travel_mm: 0.0,
        };
        for line in text.lines().filter(|l| !l.starts_with(';')) {
            match line.split_whitespace().next() {
                Some("M3") => {
                    m3 = true;
                    on = true;
                    if let Some(v) = word(line, 'S') {
                        s = v;
                    }
                }
                Some("M5") => on = !m3 && on,
                Some(g @ ("G0" | "G1")) => {
                    if let Some(v) = word(line, 'S') {
                        s = v;
                    }
                    let (Some(nx), Some(ny)) = (word(line, 'X'), word(line, 'Y')) else {
                        continue;
                    };
                    let length = (nx - x).hypot(ny - y);
                    if g == "G1" && s > 0.0 && on {
                        out.burn_mm += length;
                    } else {
                        out.travel_mm += length;
                    }
                    (x, y) = (nx, ny);
                }
                _ => {}
            }
        }
        out
    }
}
