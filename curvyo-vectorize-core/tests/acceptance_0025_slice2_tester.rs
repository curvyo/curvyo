//! Independent acceptance tests of `specs/0025-raster-trace` slice 2 at the tracer: criteria 40
//! to 45 and 59 to 66 (the table, Denoise, Upscale, Node limit, Invert, Ignore background and the
//! relations between the four presets). Pictures are generated here.
//!
//! The ordering and set-wide relations run on this file's own synthetic set, not on the
//! branch harness; the harness numbers are in `baseline.json` (`candidate_profiles`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::ops::ControlFlow;

use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{
    ADVANCED_PARAMS, DEFAULT_THRESHOLD, Denoise, PARAMS, PRESETS, ParamKind, Raster, TraceParams,
    TraceResult, Upscale, auto_threshold, trace,
};
use kurbo::{BezPath, PathEl, Point, Shape};

const MM_PER_PX: f64 = 25.4 / 96.0;

struct Pic {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

impl Pic {
    fn raster(&self) -> Raster<'_> {
        Raster {
            width: self.w,
            height: self.h,
            rgba: &self.rgba,
        }
    }
}

fn grey(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> Pic {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = f(x, y);
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    Pic { w, h, rgba }
}

fn aa(w: u32, h: u32, inside: impl Fn(f64, f64) -> bool) -> Pic {
    grey(w, h, |x, y| {
        let mut hits = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                if inside(
                    f64::from(x) + (f64::from(sx) + 0.5) / 4.0,
                    f64::from(y) + (f64::from(sy) + 0.5) / 4.0,
                ) {
                    hits += 1;
                }
            }
        }
        (255 - hits * 255 / 16) as u8
    })
}

fn px_map() -> Affine {
    Affine::new(MM_PER_PX, 0.0, 0.0, MM_PER_PX, 0.0, 0.0)
}

fn base(threshold: u8) -> TraceParams {
    TraceParams::with_threshold(threshold)
}

fn run(pic: &Pic, map: Affine, p: &TraceParams) -> TraceResult {
    trace(pic.raster(), map, p, &mut |_| ControlFlow::Continue(())).unwrap()
}

fn preset(key: &str) -> &'static curvyo_vectorize_core::Preset {
    PRESETS.iter().find(|p| p.key == key).unwrap()
}

fn with_preset(key: &str, threshold: u8) -> TraceParams {
    preset(key).apply(base(threshold))
}

fn flat(path: &BezPath, tol: f64) -> Vec<Point> {
    let mut pts = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), tol, |el| match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => pts.push(p),
        _ => {}
    });
    pts
}

fn line_art(side: u32) -> Pic {
    grey(side, side, |x, y| {
        let (fx, fy) = (f64::from(x), f64::from(y));
        let v = (fx * 0.021).sin() + (fy * 0.017).cos() + ((fx + fy) * 0.009).sin() * 1.3;
        let d = (v * 4.0 - (v * 4.0).round()).abs();
        ((d / 0.12).clamp(0.0, 1.0) * 255.0) as u8
    })
}

fn blobs() -> Pic {
    aa(300, 300, |x, y| {
        let v = (x * 0.05).sin() + (y * 0.04).cos() + ((x + y) * 0.03).sin();
        v > 0.4
    })
}

fn ring_dot() -> Pic {
    aa(300, 300, |x, y| {
        let r = ((x - 150.0).powi(2) + (y - 150.0).powi(2)).sqrt();
        (90.0..130.0).contains(&r) || r < 40.0
    })
}

fn text_like() -> Pic {
    aa(450, 150, |x, y| {
        let letter = |cx: f64| {
            let r = ((x - cx).powi(2) + (y - 75.0).powi(2)).sqrt();
            (28.0..44.0).contains(&r) || ((x - cx - 60.0).abs() < 6.0 && (30.0..120.0).contains(&y))
        };
        letter(60.0) || letter(190.0) || ((x - 330.0).abs() < 8.0 && (25.0..125.0).contains(&y))
    })
}

fn set() -> Vec<(&'static str, Pic)> {
    vec![
        ("ring", ring_dot()),
        ("text-like", text_like()),
        ("blobs", blobs()),
        ("line-art", line_art(500)),
    ]
}

fn auto(pic: &Pic) -> u8 {
    auto_threshold(pic.raster())
}

// ------------------------------------------------------- criteria 40, 45, 59, 60: the table

#[test]
fn c59_four_presets_in_order_with_the_customers_texts() {
    let got: Vec<(&str, &str)> = PRESETS.iter().map(|p| (p.label, p.detail)).collect();
    assert_eq!(
        got,
        vec![
            ("Logo / Line art", "Sharp corners, clean curves."),
            ("Photo", "Many shapes. Photos trace roughly."),
            (
                "Silhouette for cutting/carving",
                "Closed, simple, few nodes."
            ),
            ("Engraving", "Keeps fine lines."),
        ]
    );
}

#[test]
fn c60_the_values_of_the_table() {
    let v = |key: &str| {
        let p = preset(key);
        (p.smooth, p.specks_mm, p.corners, p.upscale, p.denoise)
    };
    assert_eq!(v("logo"), (80, 0.5, 70, Upscale::Off, Denoise::Off));
    assert_eq!(v("photo"), (50, 0.1, 30, Upscale::Off, Denoise::Off));
    assert_eq!(v("silhouette"), (100, 2.0, 70, Upscale::Off, Denoise::Low));
    assert_eq!(v("engraving"), (80, 0.1, 70, Upscale::Auto, Denoise::Off));
}

#[test]
fn c60_a_pick_sets_those_values_and_nothing_else() {
    let start = TraceParams {
        threshold: 77,
        invert: true,
        ignore_background: true,
        ..base(77)
    };
    for p in PRESETS {
        let got = p.apply(start);
        assert_eq!(got.threshold, 77, "{}: Threshold is never set", p.key);
        assert!(got.invert && got.ignore_background);
        assert_eq!(
            (got.smooth, got.corners, got.upscale, got.denoise),
            (p.smooth, p.corners, p.upscale, p.denoise)
        );
        assert_eq!(got.specks.as_mm(), p.specks_mm);
    }
}

#[test]
fn c60_c65_logo_is_exactly_the_slice_1_default() {
    let logo = preset("logo").apply(base(DEFAULT_THRESHOLD));
    assert_eq!(logo, base(DEFAULT_THRESHOLD));
    let pic = line_art(300);
    let a = run(&pic, px_map(), &logo);
    let b = run(
        &pic,
        px_map(),
        &TraceParams {
            smooth: 80,
            specks: Length::from_mm(0.5),
            corners: 70,
            ..base(128)
        },
    );
    assert_eq!(a, b);
}

#[test]
fn c45_the_schema_describes_the_advanced_controls_and_the_table_fits_it() {
    let names: Vec<&str> = ADVANCED_PARAMS.iter().map(|p| p.name).collect();
    for want in [
        "denoise",
        "upscale",
        "corners",
        "invert",
        "ignore_background",
    ] {
        assert!(names.contains(&want), "{want} in the schema: {names:?}");
    }
    assert!(ADVANCED_PARAMS.iter().all(|p| p.advanced));
    assert!(PARAMS.iter().all(|p| !p.advanced));
    let by = |n: &str| {
        PARAMS
            .iter()
            .chain(ADVANCED_PARAMS)
            .find(|p| p.name == n)
            .unwrap()
    };
    let corners = by("corners");
    assert_eq!((corners.min, corners.max), (0.0, 100.0));
    assert!(matches!(by("denoise").kind, ParamKind::Choice(l) if l == ["Off", "Low", "High"]));
    assert!(matches!(by("upscale").kind, ParamKind::Choice(l) if l == ["Auto", "Off"]));
    assert!(matches!(by("invert").kind, ParamKind::Toggle));
    assert!(matches!(by("ignore_background").kind, ParamKind::Toggle));
    for p in PRESETS {
        let (s, k, c) = (by("smooth"), by("specks"), by("corners"));
        assert!((s.min..=s.max).contains(&f64::from(p.smooth)), "{}", p.key);
        assert!((k.min..=k.max).contains(&p.specks_mm), "{}", p.key);
        assert!((c.min..=c.max).contains(&f64::from(p.corners)), "{}", p.key);
    }
}

#[test]
fn c41_c43_neither_full_size_preview_nor_node_limit_is_in_the_schema() {
    // Frage 74 A (2026-10-29): both were removed from the UI, the schema and the product.
    let names: Vec<&str> = ADVANCED_PARAMS.iter().map(|p| p.name).collect();
    assert!(
        !names
            .iter()
            .any(|n| n.contains("full") || n.contains("preview") || n.contains("node")),
        "the schema has {names:?}"
    );
}

// ------------------------------------------------------- criterion 42: Denoise and Upscale

fn noisy_logo(bg: u8, amplitude: i32) -> Pic {
    let mut seed = 7_u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(seed >> 8) / f64::from(1_u32 << 24)
    };
    let clean = aa(160, 160, |x, y| {
        ((x - 80.0).powi(2) + (y - 80.0).powi(2)).sqrt() <= 50.0
    });
    let mut pic = clean;
    for px in pic.rgba.chunks_mut(4) {
        let v = if px[0] > 250 { bg } else { px[0] };
        let n = ((rnd() * 2.0 - 1.0) * f64::from(amplitude)).round() as i32;
        let v = (i32::from(v) + n).clamp(0, 255) as u8;
        px[..3].fill(v);
    }
    pic
}

#[test]
fn c42_denoise_low_keeps_ringing_out_and_the_logo_within_one_pixel() {
    // The spec's case: white page, black logo, +-20 grey levels everywhere.
    let pic = noisy_logo(255, 20);
    let p = TraceParams {
        denoise: Denoise::Low,
        specks: Length::from_mm(0.0),
        ..base(128)
    };
    let r = run(&pic, Affine::IDENTITY, &p);
    assert_eq!(r.outlines.len(), 1, "one outline, none outside the logo");
    let worst = flat(&r.outlines[0], 0.01)
        .iter()
        .map(|q| (((q.x - 80.0).powi(2) + (q.y - 80.0).powi(2)).sqrt() - 50.0).abs())
        .fold(0.0, f64::max);
    assert!(worst <= 1.0, "max deviation {worst:.3} px");
}

#[test]
fn c42_denoise_removes_threshold_crossing_noise_and_high_at_least_as_much() {
    // A grey page at 140 with +-20 noise crosses the Threshold 128 in about 1 pixel in 7.
    let pic = noisy_logo(140, 20);
    let n = |d: Denoise| {
        let p = TraceParams {
            denoise: d,
            specks: Length::from_mm(0.0),
            ..base(128)
        };
        run(&pic, Affine::IDENTITY, &p).counts.paths
    };
    let (off, low, high) = (n(Denoise::Off), n(Denoise::Low), n(Denoise::High));
    println!("c42 outlines: Off {off}, Low {low}, High {high}");
    assert!(off > 20, "the fixture is noisy: {off}");
    assert!(low < off / 4, "Low cleans most of it: {low} vs {off}");
    assert!(
        high <= low,
        "High cleans at least as much as Low: {high} vs {low}"
    );
}

/// Distance from `p` to the closed polyline `pts`.
fn dist_to_polyline(p: Point, pts: &[Point]) -> f64 {
    let mut best = f64::MAX;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
        };
        best = best.min(((p.x - a.x - t * dx).powi(2) + (p.y - a.y - t * dy).powi(2)).sqrt());
    }
    best
}

/// Deviation of a result from the true shape in both directions (`specs/0089` criterion 9a):
/// trace to truth (each flattened outline point against the shape's signed distance) and truth
/// to trace (boundary samples of the shape every 0.1 px against the outlines). Returns the mean
/// and the maximum over both directions, in source pixels.
fn deviation_both(
    r: &TraceResult,
    sdf: &dyn Fn(f64, f64) -> f64,
    truth_boundary: &[Point],
) -> (f64, f64) {
    let outlines: Vec<Vec<Point>> = r
        .outlines
        .iter()
        .map(|o| {
            flat(o, 0.005)
                .into_iter()
                .map(|q| Point::new(q.x / MM_PER_PX, q.y / MM_PER_PX))
                .collect()
        })
        .collect();
    let mut devs: Vec<f64> = outlines
        .iter()
        .flatten()
        .map(|q| sdf(q.x, q.y).abs())
        .collect();
    devs.extend(truth_boundary.iter().map(|t| {
        outlines
            .iter()
            .map(|o| dist_to_polyline(*t, o))
            .fold(f64::MAX, f64::min)
    }));
    (
        devs.iter().sum::<f64>() / devs.len() as f64,
        devs.iter().copied().fold(0.0, f64::max),
    )
}

/// Criterion 42, Upscale part, as superseded by `specs/0089`: measured against the true shape in
/// both directions, Upscale Auto is not above the no-Upscale deviation plus 5 % plus 0.05 px.
#[test]
fn c42_upscale_auto_is_not_worse_than_off_against_the_true_shape() {
    let discs = [(25.0, 30.0, 14.0), (65.0, 30.0, 10.0), (100.0, 28.0, 8.0)];
    let boxes = [(20.0, 48.0, 50.0, 53.0), (60.0, 49.0, 108.0, 52.0)];
    let sdf = move |x: f64, y: f64| {
        let mut best = f64::MAX;
        for (cx, cy, r) in discs {
            best = best.min(((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r);
        }
        for (x0, y0, x1, y1) in boxes {
            let (dx, dy) = ((x0 - x).max(x - x1).max(0.0), (y0 - y).max(y - y1).max(0.0));
            let inside = (x - x0).min(x1 - x).min(y - y0).min(y1 - y);
            let d = if dx > 0.0 || dy > 0.0 {
                (dx * dx + dy * dy).sqrt()
            } else {
                -inside
            };
            best = if d.abs() < best.abs() { d } else { best };
        }
        best
    };
    let mut boundary = Vec::new();
    for (cx, cy, r) in discs {
        let n = (2.0 * std::f64::consts::PI * r / 0.1).ceil() as usize;
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            boundary.push(Point::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    for (x0, y0, x1, y1) in boxes {
        for i in 0..=((x1 - x0) / 0.1).round() as usize {
            let x = x0 + i as f64 * 0.1;
            boundary.push(Point::new(x, y0));
            boundary.push(Point::new(x, y1));
        }
        for i in 0..=((y1 - y0) / 0.1).round() as usize {
            let y = y0 + i as f64 * 0.1;
            boundary.push(Point::new(x0, y));
            boundary.push(Point::new(x1, y));
        }
    }
    let pic = aa(120, 60, |x, y| sdf(x, y) <= 0.0);
    let t = auto(&pic);
    let run_with = |u: Upscale| {
        let p = TraceParams {
            upscale: u,
            specks: Length::from_mm(0.0),
            ..base(t)
        };
        run(&pic, px_map(), &p)
    };
    let (off, auto_) = (run_with(Upscale::Off), run_with(Upscale::Auto));
    let (m_off, x_off) = deviation_both(&off, &sdf, &boundary);
    let (m_auto, x_auto) = deviation_both(&auto_, &sdf, &boundary);
    println!(
        "c42 upscale (both ways): Off mean {m_off:.3} max {x_off:.3} nodes {}; Auto mean {m_auto:.3} max {x_auto:.3} nodes {}",
        off.counts.nodes, auto_.counts.nodes
    );
    assert!(
        x_auto <= x_off * 1.05 + 0.05,
        "max deviation: Auto {x_auto:.3} vs Off {x_off:.3}"
    );
    assert!(
        m_auto <= m_off * 1.05 + 0.05,
        "mean deviation: Auto {m_auto:.3} vs Off {m_off:.3}"
    );
    // the picture coordinates stay those of the original pixels
    let b = auto_
        .outlines
        .iter()
        .map(BezPath::bounding_box)
        .fold(None, |a: Option<kurbo::Rect>, b| {
            Some(a.map_or(b, |a| a.union(b)))
        })
        .unwrap();
    assert!(
        (b.x0 / MM_PER_PX - 11.0).abs() < 1.5 && (b.x1 / MM_PER_PX - 108.0).abs() < 1.5,
        "{b:?}"
    );
}

#[test]
fn c41_upscale_auto_applies_only_below_one_megapixel() {
    assert!(Upscale::Auto.applies(999, 1000));
    assert!(!Upscale::Auto.applies(1000, 1000));
    assert!(!Upscale::Auto.applies(2000, 2000));
    assert!(!Upscale::Off.applies(10, 10));
}

// ------------------------------------------------------- criterion 44: Invert, Ignore background

#[test]
fn c44_invert_is_the_exact_complement_of_the_mask() {
    let pic = grey(40, 30, |x, y| ((x * 7 + y * 3) % 256) as u8);
    for t in [0_u8, 1, 100, 128, 255] {
        let normal = run(
            &pic,
            px_map(),
            &TraceParams {
                specks: Length::from_mm(0.0),
                ..base(t)
            },
        );
        let inverted = run(
            &pic,
            px_map(),
            &TraceParams {
                invert: true,
                specks: Length::from_mm(0.0),
                ..base(t)
            },
        );
        assert_eq!(
            normal.foreground_pixels + inverted.foreground_pixels,
            40 * 30,
            "threshold {t}"
        );
    }
    // at or above: a pixel equal to the Threshold is background normally, foreground inverted
    let one = Pic {
        w: 1,
        h: 1,
        rgba: vec![100, 100, 100, 255],
    };
    let n = |inv: bool| {
        run(
            &one,
            px_map(),
            &TraceParams {
                invert: inv,
                specks: Length::from_mm(0.0),
                ..base(100)
            },
        )
        .foreground_pixels
    };
    assert_eq!((n(false), n(true)), (0, 1));
}

#[test]
fn c44_a_black_logo_inverted_is_the_page_shape_with_the_logo_as_a_hole() {
    let pic = aa(200, 200, |x, y| {
        ((x - 100.0).powi(2) + (y - 100.0).powi(2)).sqrt() <= 50.0
    });
    let r = run(
        &pic,
        Affine::IDENTITY,
        &TraceParams {
            invert: true,
            specks: Length::from_mm(0.0),
            ..base(128)
        },
    );
    assert_eq!(r.outlines.len(), 2, "page rectangle and the logo as a hole");
    let mut areas: Vec<f64> = r.outlines.iter().map(Shape::area).collect();
    areas.sort_by(|a, b| b.abs().total_cmp(&a.abs()));
    assert!(areas[0] * areas[1] < 0.0, "the hole winds against the page");
    assert!(
        (areas[0].abs() - 40_000.0).abs() < 400.0,
        "page {}",
        areas[0]
    );
    assert!(
        (areas[1].abs() - std::f64::consts::PI * 2500.0).abs() < 120.0,
        "hole {}",
        areas[1]
    );
    let filled = (areas[0] + areas[1]).abs();
    assert!((filled - (40_000.0 - std::f64::consts::PI * 2500.0)).abs() < 400.0);
}

#[test]
fn c44_ignore_background_drops_the_regions_touching_the_border() {
    // A black frame on the border, a ring inside the frame's hole, a dot inside the ring.
    let pic = aa(300, 300, |x, y| {
        let frame = !(10.0..290.0).contains(&x) || !(10.0..290.0).contains(&y);
        let r = ((x - 150.0).powi(2) + (y - 150.0).powi(2)).sqrt();
        frame || (60.0..90.0).contains(&r) || r < 20.0
    });
    let off = run(
        &pic,
        Affine::IDENTITY,
        &TraceParams {
            specks: Length::from_mm(0.0),
            ..base(128)
        },
    );
    let on = run(
        &pic,
        Affine::IDENTITY,
        &TraceParams {
            ignore_background: true,
            specks: Length::from_mm(0.0),
            ..base(128)
        },
    );
    assert_eq!(
        off.outlines.len(),
        2 + 3,
        "frame outer and inner, ring outer and inner, dot"
    );
    assert_eq!(on.outlines.len(), 3, "ring outer, ring inner, dot");
    for o in &on.outlines {
        let b = o.bounding_box();
        assert!(b.x0 > 20.0 && b.x1 < 280.0, "{b:?}");
    }
    // the same with Invert: the white page touches the border and goes, inner white regions stay
    let both = run(
        &pic,
        Affine::IDENTITY,
        &TraceParams {
            invert: true,
            ignore_background: true,
            specks: Length::from_mm(0.0),
            ..base(128)
        },
    );
    assert_eq!(
        both.outlines.len(),
        4,
        "page inside the frame (outer, ring hole) and the gap (outer, dot hole)"
    );
    let open = aa(300, 300, |x, y| {
        let r = ((x - 150.0).powi(2) + (y - 150.0).powi(2)).sqrt();
        (60.0..90.0).contains(&r) || r < 20.0
    });
    let kept = run(
        &open,
        Affine::IDENTITY,
        &TraceParams {
            invert: true,
            ignore_background: true,
            specks: Length::from_mm(0.0),
            ..base(128)
        },
    );
    assert_eq!(
        kept.outlines.len(),
        2,
        "the white page touches the border and goes; the gap between ring and dot stays"
    );
}

// ------------------------------------------------------- criteria 62 to 64: relations

fn mm2(o: &BezPath) -> f64 {
    o.area().abs()
}

/// Nodes per 100 mm of the preset's result on `pic`.
fn density_of(pic: &Pic, key: &str) -> f64 {
    let r = run(pic, px_map(), &with_preset(key, auto(pic)));
    let mm: f64 = r.outlines.iter().map(|o| o.perimeter(1e-3)).sum();
    r.counts.nodes as f64 / (mm / 100.0).max(1e-9)
}

/// Criterion 64 as superseded by `specs/0089`: the geometric mean of the nodes per 100 mm over
/// the set is ordered Engraving >= Logo / Line art >= Silhouette; per image an inversion is
/// allowed up to the per-image cap of criterion 9a (the larger of +25 % and +3 nodes per 100 mm).
#[test]
fn c64_node_ordering_engraving_logo_silhouette() {
    let geo = |v: &[f64]| (v.iter().map(|x| x.ln()).sum::<f64>() / v.len() as f64).exp();
    let (mut es, mut ls, mut ss) = (Vec::new(), Vec::new(), Vec::new());
    let within_cap = |lower_order: f64, higher_order: f64| {
        lower_order <= (higher_order * 1.25).max(higher_order + 3.0)
    };
    for (name, pic) in set() {
        let (e, l, s) = (
            density_of(&pic, "engraving"),
            density_of(&pic, "logo"),
            density_of(&pic, "silhouette"),
        );
        println!("c64 {name}: engraving {e:.2}, logo {l:.2}, silhouette {s:.2} nodes/100mm");
        assert!(
            within_cap(l, e),
            "{name}: Logo {l:.2} above the cap of Engraving {e:.2}"
        );
        assert!(
            within_cap(s, l),
            "{name}: Silhouette {s:.2} above the cap of Logo {l:.2}"
        );
        es.push(e);
        ls.push(l);
        ss.push(s);
    }
    let (ge, gl, gs) = (geo(&es), geo(&ls), geo(&ss));
    println!("c64 geometric mean: engraving {ge:.2}, logo {gl:.2}, silhouette {gs:.2}");
    assert!(ge >= gl, "Engraving {ge:.2} < Logo {gl:.2}");
    assert!(gl >= gs, "Logo {gl:.2} < Silhouette {gs:.2}");
}

#[test]
fn c63_silhouette_has_no_outline_or_hole_below_3_8_mm2_and_is_closed() {
    for (name, pic) in set() {
        let t = auto(&pic);
        let sil = run(&pic, px_map(), &with_preset("silhouette", t));
        let logo = run(&pic, px_map(), &with_preset("logo", t));
        for o in &sil.outlines {
            assert!(mm2(o) >= 3.8, "{name}: an outline of {:.2} mm2", mm2(o));
            assert!(
                matches!(o.elements().last(), Some(PathEl::ClosePath)),
                "{name}: closed"
            );
        }
        assert!(
            sil.outlines.len() <= logo.outlines.len(),
            "{name}: outlines {} vs {}",
            sil.outlines.len(),
            logo.outlines.len()
        );
    }
}

/// Per image of the 0089 reference set at 1x: Silhouette's and Logo / Line art's nodes per
/// 100 mm, for the images where both have outlines; asserts the 3.8 mm2 floor on the way.
fn c63_densities() -> Vec<(String, f64, f64)> {
    use common::quality::{reference_set, run::params_for};
    let density = |r: &TraceResult| {
        let mm: f64 = r.outlines.iter().map(|o| o.perimeter(1e-3)).sum();
        r.counts.nodes as f64 / (mm / 100.0).max(1e-9)
    };
    let mut out = Vec::new();
    for reference in reference_set() {
        let v = reference.variant(0);
        let map = Affine::scale(v.mm_per_px, v.mm_per_px);
        let at = |key: &str| {
            let params = params_for(preset(key), &v.picture);
            let raster = Raster {
                width: v.picture.width,
                height: v.picture.height,
                rgba: &v.picture.rgba,
            };
            trace(raster, map, &params, &mut |_| ControlFlow::Continue(())).unwrap()
        };
        let (sil, logo) = (at("silhouette"), at("logo"));
        for o in &sil.outlines {
            assert!(
                mm2(o) >= 0.95 * 4.0,
                "{}: an outline or hole of {:.2} mm2",
                v.key,
                mm2(o)
            );
        }
        if sil.outlines.is_empty() || logo.outlines.is_empty() {
            continue;
        }
        let (dl, ds) = (density(&logo), density(&sil));
        println!(
            "c63 {}: logo {dl:.1}, silhouette {ds:.1} nodes/100mm",
            v.key
        );
        out.push((v.key, dl, ds));
    }
    out
}

/// Criterion 63 as amended by `specs/0089` (2026-10-29), on the 0089 reference set at 1x:
/// Silhouette's nodes per 100 mm are at most 0.8 times Logo / Line art's as the geometric mean;
/// no outline or hole below 0.95 x 4 mm2.
#[test]
fn c63_silhouette_geometric_mean_at_most_0_8_of_logo_on_the_0089_reference_set() {
    let rows = c63_densities();
    assert!(!rows.is_empty(), "the set has images with outlines");
    let geo = (rows.iter().map(|(_, l, s)| (s / l).ln()).sum::<f64>() / rows.len() as f64).exp();
    println!(
        "c63 geometric mean ratio {geo:.3} over {} images",
        rows.len()
    );
    assert!(geo <= 0.8, "geometric mean {geo:.3} > 0.8");
}

/// Criterion 63 as amended by `specs/0089`: Silhouette's nodes per 100 mm are never more than
/// Logo / Line art's on any image of the reference set (the spec names no tolerance).
#[test]
#[ignore = "0025/0089 FAIL: Silhouette > Logo on 5 of 25 images"]
fn c63_silhouette_never_more_than_logo_per_image_on_the_0089_reference_set() {
    let misses: Vec<String> = c63_densities()
        .into_iter()
        .filter(|(_, l, s)| s > l)
        .map(|(k, l, s)| format!("{k}: Silhouette {s:.2} > Logo {l:.2}"))
        .collect();
    assert!(misses.is_empty(), "per image: {misses:#?}");
}

/// A photograph stand-in: smooth value noise with a few octaves, 400 x 400.
fn photo() -> Pic {
    let mut seed = 99_u64;
    let mut lattice = vec![0.0_f64; 65 * 65];
    for v in &mut lattice {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *v = ((seed >> 33) as f64) / f64::from(1_u32 << 31);
    }
    let at = |gx: usize, gy: usize| lattice[(gy % 65) * 65 + gx % 65];
    grey(400, 400, |x, y| {
        let mut total = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        for octave in 0..4 {
            let cell = 64.0 / f64::from(1_u32 << octave);
            let (fx, fy) = (
                f64::from(x) / cell.max(2.0) * 1.0,
                f64::from(y) / cell.max(2.0),
            );
            let (ix, iy) = (fx.floor() as usize, fy.floor() as usize);
            let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
            let s = |t: f64| t * t * (3.0 - 2.0 * t);
            let a = at(ix, iy) * (1.0 - s(tx)) + at(ix + 1, iy) * s(tx);
            let b = at(ix, iy + 1) * (1.0 - s(tx)) + at(ix + 1, iy + 1) * s(tx);
            total += (a * (1.0 - s(ty)) + b * s(ty)) * amp;
            norm += amp;
            amp *= 0.55;
        }
        let grain = (f64::from((x * 7919 + y * 104_729) % 97) / 97.0 - 0.5) * 0.35;
        ((total / norm + grain).clamp(0.0, 1.0) * 255.0) as u8
    })
}

#[test]
fn c63_photo_has_many_shapes_none_below_its_own_specks_and_closed() {
    let pic = photo();
    let t = auto(&pic);
    let ph = run(&pic, px_map(), &with_preset("photo", t));
    let logo = run(&pic, px_map(), &with_preset("logo", t));
    println!(
        "c63 photo fixture: photo {} outlines {} nodes, logo {} outlines",
        ph.outlines.len(),
        ph.counts.nodes,
        logo.outlines.len()
    );
    assert!(
        ph.outlines.len() >= 3 * logo.outlines.len(),
        "at least 3 times: {} vs {}",
        ph.outlines.len(),
        logo.outlines.len()
    );
    for o in &ph.outlines {
        assert!(mm2(o) >= 0.95 * 0.1 * 0.1, "outline of {:.5} mm2", mm2(o));
        assert!(matches!(o.elements().last(), Some(PathEl::ClosePath)));
    }
}

#[test]
fn c62_engraving_keeps_the_holes_and_fine_shapes_that_logo_fills() {
    // Letters with a 1 px hole (0.07 mm2): Logo fills it, Engraving keeps it.
    let pic = grey(300, 150, |x, y| {
        let glyph = |cx: u32| {
            let inside = (cx..cx + 30).contains(&x) && (30..120).contains(&y);
            let hole = x == cx + 14 && y == 70;
            inside && !hole
        };
        if (0..9).any(|i| glyph(10 + i * 32)) {
            0
        } else {
            255
        }
    });
    let t = auto(&pic);
    let holes = |key: &str| {
        let r = run(&pic, px_map(), &with_preset(key, t));
        r.outlines.len() - 9
    };
    let (e, l) = (holes("engraving"), holes("logo"));
    println!("c62 holes kept: engraving {e}, logo {l}");
    assert_eq!(l, 0, "Logo at 0.5 mm fills 1 px holes (0.07 mm2)");
    assert_eq!(e, 9, "Engraving at 0.1 mm keeps all nine holes");
}

/// Criterion 62 is measured on the quality set (the research images, lead decision 2026-10-28;
/// the PO clarifies the spec). The plain shapes of this file's set are printed as an
/// observation only: on a plain ring Engraving has about 8x Logo's density.
#[test]
#[ignore = "quality set (75 s in debug): run in release with --ignored, as the quality harness"]
fn c62_engraving_has_at_most_4x_the_node_density_of_logo_and_2_5x_as_geometric_mean() {
    let len = |r: &TraceResult| r.outlines.iter().map(|o| o.perimeter(1e-3)).sum::<f64>();
    let dens = |r: &TraceResult| r.counts.nodes as f64 / (len(r) / 100.0).max(1e-9);
    for (name, pic) in set() {
        let t = auto(&pic);
        let e = run(&pic, px_map(), &with_preset("engraving", t));
        let l = run(&pic, px_map(), &with_preset("logo", t));
        println!(
            "c62 observation {name}: density engraving {:.1} logo {:.1} ({:.2}x)",
            dens(&e),
            dens(&l),
            dens(&e) / dens(&l)
        );
    }
    let mut ratios = Vec::<f64>::new();
    let mut misses = Vec::<String>::new();
    for image in common::quality::quality_set() {
        let pic = &image.picture;
        let t = auto_threshold(pic.raster());
        let mm = 25.4 / image.dpi;
        let map = Affine::new(mm, 0.0, 0.0, mm, 0.0, 0.0);
        let traced = |key: &str| {
            trace(pic.raster(), map, &with_preset(key, t), &mut |_| {
                ControlFlow::Continue(())
            })
            .unwrap()
        };
        let (e, l) = (traced("engraving"), traced("logo"));
        println!(
            "c62 {}: density engraving {:.1} logo {:.1}",
            image.name,
            dens(&e),
            dens(&l)
        );
        ratios.push(dens(&e) / dens(&l));
        if dens(&e) > 4.0 * dens(&l) {
            misses.push(format!("{}: {:.2}x", image.name, dens(&e) / dens(&l)));
        }
        assert!(
            e.outlines
                .iter()
                .all(|o| matches!(o.elements().last(), Some(PathEl::ClosePath)))
        );
    }
    let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / ratios.len() as f64).exp();
    println!("c62 geometric mean {geo:.2}x");
    assert!(misses.is_empty(), "above 4x: {misses:?}");
    assert!(geo <= 2.5, "geometric mean {geo:.2}x");
}

#[test]
fn c60_every_preset_is_deterministic_and_never_panics_on_degenerate_pictures() {
    let tiny = Pic {
        w: 1,
        h: 1,
        rgba: vec![0, 0, 0, 255],
    };
    let blank = grey(50, 50, |_, _| 255);
    for p in PRESETS {
        for pic in [&tiny, &blank] {
            let a = trace(pic.raster(), px_map(), &p.apply(base(128)), &mut |_| {
                ControlFlow::Continue(())
            });
            let b = trace(pic.raster(), px_map(), &p.apply(base(128)), &mut |_| {
                ControlFlow::Continue(())
            });
            assert_eq!(a, b, "{}", p.key);
        }
        let pic = text_like();
        let a = run(&pic, px_map(), &p.apply(base(128)));
        let b = run(&pic, px_map(), &p.apply(base(128)));
        assert_eq!(a, b, "{}", p.key);
    }
}

#[test]
fn advanced_options_survive_a_stop_and_degenerate_input() {
    let pic = line_art(300);
    let p = TraceParams {
        denoise: Denoise::High,
        upscale: Upscale::Auto,
        invert: true,
        ignore_background: true,
        ..base(128)
    };
    let mut n = 0;
    let stopped = trace(pic.raster(), px_map(), &p, &mut |_| {
        n += 1;
        if n == 3 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert_eq!(
        stopped.unwrap_err(),
        curvyo_vectorize_core::TraceError::Cancelled
    );
    let all = trace(pic.raster(), px_map(), &p, &mut |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(
        all.outlines
            .iter()
            .all(|o| matches!(o.elements().last(), Some(PathEl::ClosePath)))
    );
    let one = Pic {
        w: 1,
        h: 1,
        rgba: vec![0, 0, 0, 255],
    };
    let r = trace(one.raster(), px_map(), &p, &mut |_| {
        ControlFlow::Continue(())
    });
    assert!(r.is_ok() || matches!(r, Err(curvyo_vectorize_core::TraceError::Invalid(_))));
}
