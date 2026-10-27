//! The quality harness and the budgets (`specs/0025-raster-trace` criteria 21, 25 to 29, 36;
//! ADR 0023 §7 step 0). Every test here is `#[ignore]` and meant for a release build:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test quality -- --ignored --nocapture
//! ```
//!
//! `quality_table` traces the synthetic black and white set at the default settings, prints the
//! metrics of `research.md` section 5 per image, reports every miss of the criterion 28 targets,
//! and fails when a metric is more than 5 % worse than the committed run in
//! `tests/fixtures/trace/baseline.json` (the nightly job, criterion 36).
//! `CURVYO_UPDATE_BASELINE=1` rewrites our run in that file. With `TRACE_BASELINE_DIR` set it
//! also measures Inkscape's results found there as `<image>.inkscape-default.svg` and
//! `<image>.inkscape-best.svg` (`tools/trace-baseline/README.md`) and stores their numbers.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::collections::BTreeMap;
use std::ops::ControlFlow;

use common::quality::{Kind, Metrics, QualityImage, measure, quality_set};
use common::{params, run};
use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{
    DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, TraceParams, auto_threshold, trace,
};
use kurbo::{BezPath, PathEl};
use serde_json::{Value, json};

/// The default settings with the image's automatic Threshold.
fn defaults(image: &QualityImage) -> TraceParams {
    TraceParams {
        threshold: auto_threshold(image.picture.raster()),
        smooth: DEFAULT_SMOOTH,
        specks: Length::from_mm(DEFAULT_SPECKS_MM),
    }
}

fn mm_per_px(image: &QualityImage) -> f64 {
    25.4 / image.dpi
}

/// The luma of one straight-alpha pixel over white (Rec. 709, integers, rounded), as the tracer
/// computes it; the harness measures against its own copy.
fn luma(r: u8, g: u8, b: u8, a: u8) -> u8 {
    let over = |c: u8| (u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255;
    let weighted = 2126 * over(r) + 7152 * over(g) + 722 * over(b);
    u8::try_from((weighted + 5000) / 10_000).unwrap_or(u8::MAX)
}

/// The thresholded mask of `image`.
fn mask_of(image: &QualityImage, threshold: u8) -> Vec<bool> {
    image
        .picture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| luma(r, g, b, a) < threshold)
        .collect()
}

/// Our metrics on `image` at the defaults: outlines traced in mm, measured in pixels.
fn candidate(image: &QualityImage) -> Metrics {
    let p = defaults(image);
    let scale = mm_per_px(image);
    let result = run(&image.picture, Affine::scale(scale, scale), &p);
    let in_px: Vec<BezPath> = result
        .outlines
        .iter()
        .map(|o| kurbo::Affine::scale(1.0 / scale) * o.clone())
        .collect();
    measure(
        &in_px,
        &mask_of(image, p.threshold),
        image.picture.width as usize,
        image.picture.height as usize,
        scale,
    )
}

/// The outlines of a baseline SVG (in its page's millimetres), split per subpath, in pixels.
fn baseline_outlines(svg: &[u8], image: &QualityImage) -> Option<Vec<BezPath>> {
    let drawing = curvyo_svg_core::read_svg(svg).ok()?;
    let (page_w, _) = drawing.page?;
    let px_per_mm = f64::from(image.picture.width) / page_w;
    let mut paths = Vec::new();
    collect_paths(&drawing.nodes, &mut paths);
    let mut outlines = Vec::new();
    for path in paths {
        let mut current = BezPath::new();
        for el in (kurbo::Affine::scale(px_per_mm) * path).elements() {
            if matches!(el, PathEl::MoveTo(_)) && !current.elements().is_empty() {
                outlines.push(std::mem::take(&mut current));
            }
            current.push(*el);
        }
        if !current.elements().is_empty() {
            outlines.push(current);
        }
    }
    Some(outlines)
}

fn collect_paths(nodes: &[curvyo_svg_core::SvgNode], out: &mut Vec<BezPath>) {
    for node in nodes {
        match node {
            curvyo_svg_core::SvgNode::Group { children, .. } => collect_paths(children, out),
            curvyo_svg_core::SvgNode::Leaf { leaf, .. } => {
                if let curvyo_svg_core::LeafKind::Path { path, .. } = &leaf.kind {
                    out.push(path.clone());
                }
            }
            _ => {}
        }
    }
}

fn metrics_json(m: &Metrics) -> Value {
    let map: serde_json::Map<String, Value> = Metrics::NAMES
        .iter()
        .zip(m.values())
        .map(|(name, value)| ((*name).to_string(), json!((value * 1e4).round() / 1e4)))
        .collect();
    Value::Object(map)
}

/// The criterion 28 misses of `ours` (with Inkscape's default run when known).
fn misses(image: &QualityImage, ours: &Metrics, inkscape: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    let max_dev = 0.75;
    let area_limit = if image.kind == Kind::LogoOrText {
        1.0
    } else {
        3.0
    };
    if ours.micro_paths > 0.0 {
        out.push(format!("micro_paths {} > 0", ours.micro_paths));
    }
    if ours.max_deviation_px > max_dev {
        out.push(format!(
            "max_deviation_px {:.3} > {max_dev}",
            ours.max_deviation_px
        ));
    }
    if ours.mean_deviation_px > 0.3 {
        out.push(format!(
            "mean_deviation_px {:.3} > 0.3",
            ours.mean_deviation_px
        ));
    }
    if ours.area_error_pct >= area_limit {
        out.push(format!(
            "area_error_pct {:.3} >= {area_limit}",
            ours.area_error_pct
        ));
    }
    if ours.closed_pct < 100.0 {
        out.push(format!("closed_pct {:.1} < 100", ours.closed_pct));
    }
    if let Some(ink) = inkscape {
        let of = |key: &str| ink[key].as_f64().unwrap_or(f64::NAN);
        if ours.node_density > 0.6 * of("node_density") {
            out.push(format!(
                "node_density {:.2} > 60 % of Inkscape's {:.2}",
                ours.node_density,
                of("node_density")
            ));
        }
        if ours.wobble > 0.5 * of("wobble") {
            out.push(format!(
                "wobble {:.2} > 50 % of Inkscape's {:.2}",
                ours.wobble,
                of("wobble")
            ));
        }
    }
    out
}

/// Criteria 27, 28, 29 and 36: the table, the misses and the regression check.
#[test]
#[ignore = "quality harness: run in release with --ignored --nocapture"]
fn quality_table() {
    let path = common::fixture_dir().join("baseline.json");
    let mut baseline: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    let baseline_dir = std::env::var_os("TRACE_BASELINE_DIR").map(std::path::PathBuf::from);
    let mut ours_all = BTreeMap::new();
    let mut regressions = Vec::new();
    let mut sums = [0.0; 7];
    let set = quality_set();
    println!("image                 {}", Metrics::NAMES.join("  "));
    for image in &set {
        let ours = candidate(image);
        for (sum, value) in sums.iter_mut().zip(ours.values()) {
            *sum += value;
        }
        if let Some(dir) = &baseline_dir {
            let p = defaults(image);
            for run_name in ["inkscape-default", "inkscape-best"] {
                let file = dir.join(format!("{}.{run_name}.svg", image.name));
                let Ok(bytes) = std::fs::read(&file) else {
                    continue;
                };
                let Some(outlines) = baseline_outlines(&bytes, image) else {
                    continue;
                };
                let m = measure(
                    &outlines,
                    &mask_of(image, p.threshold),
                    image.picture.width as usize,
                    image.picture.height as usize,
                    mm_per_px(image),
                );
                baseline[run_name.replace('-', "_")][image.name] = metrics_json(&m);
            }
        }
        let values: Vec<String> = ours.values().iter().map(|v| format!("{v:>10.3}")).collect();
        println!("{:<22}{}", image.name, values.join(" "));
        let inkscape = baseline
            .get("inkscape_default")
            .and_then(|i| i.get(image.name));
        for miss in misses(image, &ours, inkscape) {
            println!("  MISS {}: {miss}", image.name);
        }
        if let Some(committed) = baseline.get("candidate").and_then(|c| c.get(image.name)) {
            for (name, value) in Metrics::NAMES.iter().zip(ours.values()) {
                let old = committed[*name].as_f64().unwrap_or(f64::NAN);
                // Higher is worse for every metric but closedness.
                let worse = if *name == "closed_pct" {
                    value < old * 0.95
                } else {
                    value > old * 1.05 + 1e-3
                };
                if worse {
                    regressions.push(format!("{} {name}: {old} -> {value:.4}", image.name));
                }
            }
        }
        ours_all.insert(image.name.to_string(), metrics_json(&ours));
    }
    let n = set.len() as f64;
    let means: Vec<String> = sums.iter().map(|s| format!("{:>10.3}", s / n)).collect();
    println!("{:<22}{}", "mean", means.join(" "));
    if std::env::var_os("CURVYO_UPDATE_BASELINE").is_some() || baseline_dir.is_some() {
        baseline["candidate"] = serde_json::to_value(&ours_all).unwrap();
        baseline["settings"] = json!({
            "smooth": DEFAULT_SMOOTH,
            "specks_mm": DEFAULT_SPECKS_MM,
            "threshold": "automatic",
        });
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&baseline).unwrap() + "\n",
        )
        .unwrap();
        return;
    }
    assert!(
        regressions.is_empty(),
        "regressions over 5 %:\n{}",
        regressions.join("\n")
    );
}

/// Step 0 for criterion 4: the lowest Smooth at which the disc of criterion 21 holds.
#[test]
#[ignore = "step 0: prints the disc deviation per Smooth"]
fn smooth_scan_for_the_default() {
    let disc = common::read_picture("disc.pgm");
    for smooth in (0..=100).step_by(10) {
        let result = run(&disc, Affine::IDENTITY, &params(128, smooth, 0.5));
        let (mut max, mut sum, mut n) = (0.0_f64, 0.0, 0.0);
        for seg in result.outlines[0].segments() {
            for i in 0..100 {
                let p = kurbo::ParamCurve::eval(&seg, f64::from(i) / 100.0);
                let d = (p.distance(kurbo::Point::new(250.0, 250.0)) - 100.0).abs();
                max = max.max(d);
                sum += d;
                n += 1.0;
            }
        }
        println!(
            "smooth {smooth:>3}: {:>3} nodes, max {max:.3} px, mean {:.3} px",
            result.counts.nodes,
            sum / n
        );
    }
}

/// Writes the quality set as PGM to `target/trace-quality/` for the offline baselines.
#[test]
#[ignore = "writes the quality set for Inkscape and potrace"]
fn write_quality_set() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/trace-quality");
    std::fs::create_dir_all(&dir).unwrap();
    for image in quality_set() {
        let p = &image.picture;
        let mut bytes = format!("P5\n{} {}\n255\n", p.width, p.height).into_bytes();
        bytes.extend(p.levels());
        std::fs::write(dir.join(format!("{}.pgm", image.name)), bytes).unwrap();
        println!("{} {} dpi", image.name, image.dpi);
    }
    // The budget's picture, for the wasm timing of criterion 25 in a worker or Node.
    let art = line_art_2000();
    let mut bytes = format!("P5\n{} {}\n255\n", art.width, art.height).into_bytes();
    bytes.extend(art.levels());
    std::fs::write(dir.join("line-art-2000.pgm"), bytes).unwrap();
}

/// The line-art picture of the budget: 2000 x 2000 px of strokes, circles and noise specks.
fn line_art_2000() -> common::Picture {
    let mut noise = common::quality::Noise::new(42);
    let circles: Vec<(f64, f64, f64)> = (0..400)
        .map(|_| {
            (
                1000.0 + 950.0 * noise.next(),
                1000.0 + 950.0 * noise.next(),
                8.0 + 30.0 * noise.next().abs(),
            )
        })
        .collect();
    let mut levels = vec![255_u8; 2000 * 2000];
    for &(cx, cy, r) in &circles {
        let (x0, x1) = (
            (cx - r - 3.0).max(0.0) as usize,
            ((cx + r + 3.0) as usize).min(1999),
        );
        let (y0, y1) = (
            (cy - r - 3.0).max(0.0) as usize,
            ((cy + r + 3.0) as usize).min(1999),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
                if (d - r).abs() <= 2.0 {
                    levels[y * 2000 + x] = 0;
                }
            }
        }
    }
    for _ in 0..20_000 {
        let x = ((noise.next() + 1.0) * 999.5) as usize;
        let y = ((noise.next() + 1.0) * 999.5) as usize;
        levels[y * 2000 + x] = 0;
    }
    common::Picture::grey(2000, 2000, &levels)
}

/// Criterion 25: a full trace of the 2000 x 2000 px line art under 4 s of thread CPU time;
/// criterion 17: no stretch between two `poll` calls longer than 250 ms.
#[cfg(unix)]
#[test]
#[ignore = "budget: run in release"]
fn line_art_2000_budget() {
    let picture = line_art_2000();
    let map = Affine::scale(25.4 / 300.0, 25.4 / 300.0);
    let p = params(128, DEFAULT_SMOOTH, DEFAULT_SPECKS_MM);
    let mut last = common::thread_cpu_seconds();
    let mut longest = 0.0_f64;
    let start = last;
    let mut poll = |_| {
        let now = common::thread_cpu_seconds();
        longest = longest.max(now - last);
        last = now;
        ControlFlow::Continue(())
    };
    let result = trace(picture.raster(), map, &p, &mut poll).unwrap();
    let total = common::thread_cpu_seconds() - start;
    println!(
        "2000 x 2000 px: {total:.3} s thread CPU, {} paths, {} nodes, {} specks, longest poll gap {longest:.3} s",
        result.counts.paths, result.counts.nodes, result.counts.specks_removed
    );
    // The preview's reduced copy (1000 x 1000 px) of the same picture.
    let levels = picture.levels();
    let reduced = common::render(1000, 1000, 1, |x, y| {
        levels[(y as usize * 2) * 2000 + x as usize * 2] < 128
    });
    let t0 = common::thread_cpu_seconds();
    let small = run(&reduced, Affine::scale(25.4 / 150.0, 25.4 / 150.0), &p);
    println!(
        "reduced copy 1000 x 1000 px: {:.3} s thread CPU, {} paths",
        common::thread_cpu_seconds() - t0,
        small.counts.paths
    );
    assert!(total < 4.0, "{total} s");
    assert!(longest < 0.25, "{longest} s between two polls");
}
