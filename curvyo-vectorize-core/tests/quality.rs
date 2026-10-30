//! The quality harness and the budgets (`specs/0025-raster-trace` criteria 21, 25 to 29, 36;
//! ADR 0023 §7 step 0). Every test here is `#[ignore]` and meant for a release build:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test quality -- --ignored --nocapture
//! ```
//!
//! `quality_table` traces the synthetic black and white set at the default settings, prints the
//! metrics of `research.md` section 5 per image, prints the gap to Inkscape's default run, and
//! fails on a miss of the guard of `specs/0089` criterion 9a against the committed `candidate`
//! run in `tests/fixtures/trace/baseline.json` (nodes and wobble per image within +25 % or the
//! absolute floor, the true-shape area error within +5 % plus 0.5 points; the deviations from
//! the pixel border are reported on this synthetic set), or when a micro path or open outline
//! appears (criterion 28 as amended, the nightly job, criterion 36).
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

use common::quality::{
    Kind, Metrics, QualityImage, area_error_true_pct, measure, measure_micro, photograph,
    quality_set,
};
use common::{params, run};
use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{
    DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, PRESETS, Preset, TraceParams, auto_threshold, trace,
};
use kurbo::{BezPath, PathEl};
use serde_json::{Value, json};

/// The default settings with the image's automatic Threshold.
fn defaults(image: &QualityImage) -> TraceParams {
    TraceParams {
        threshold: auto_threshold(image.picture.raster()),
        smooth: DEFAULT_SMOOTH,
        specks: Length::from_mm(DEFAULT_SPECKS_MM),
        ..TraceParams::with_threshold(0)
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

/// Our metrics on `image` at the defaults: outlines traced in mm, measured in pixels against the
/// thresholded mask after the default Specks, as the preset runs measure (criterion 65 compares
/// the two; the trace is the same).
fn candidate(image: &QualityImage) -> Metrics {
    let p = defaults(image);
    let scale = mm_per_px(image);
    let result = run(&image.picture, Affine::scale(scale, scale), &p);
    let in_px: Vec<BezPath> = result
        .outlines
        .iter()
        .map(|o| kurbo::Affine::scale(1.0 / scale) * o.clone())
        .collect();
    let (width, height) = (image.picture.width as usize, image.picture.height as usize);
    let speck_px = DEFAULT_SPECKS_MM * DEFAULT_SPECKS_MM / (scale * scale);
    let reference = without_specks(&mask_of(image, p.threshold), width, height, speck_px);
    measure(&in_px, &reference, width, height, scale)
}

/// The photograph of criterion 61 as an image of the harness.
fn photo_image() -> QualityImage {
    QualityImage {
        name: "13-photograph",
        picture: photograph(),
        dpi: 96.0,
        kind: Kind::Other,
        truth_4x: None,
    }
}

/// One preset's run on one image (criterion 61).
struct PresetRun {
    metrics: Metrics,
    paths: usize,
    nodes: usize,
    /// Outlines wound as holes.
    holes: usize,
    /// The area error against the drawn shape, % (`None` for the photograph).
    area_error_true_pct: Option<f64>,
}

/// The micro-path area of a preset: 0.95 x its own Specks² (criterion 61, `adrs.md`
/// 2026-10-28 point 6).
fn micro_mm2(preset: &Preset) -> f64 {
    0.95 * preset.specks_mm * preset.specks_mm
}

/// `preset` on `image` at the image's automatic Threshold.
fn preset_run(preset: &Preset, image: &QualityImage) -> PresetRun {
    let threshold = auto_threshold(image.picture.raster());
    let p = preset.apply(TraceParams::with_threshold(threshold));
    let scale = mm_per_px(image);
    let result = run(&image.picture, Affine::scale(scale, scale), &p);
    let in_px: Vec<BezPath> = result
        .outlines
        .iter()
        .map(|o| kurbo::Affine::scale(1.0 / scale) * o.clone())
        .collect();
    let (width, height) = (image.picture.width as usize, image.picture.height as usize);
    let speck_px = preset.specks_mm * preset.specks_mm / (scale * scale);
    let reference = without_specks(&mask_of(image, threshold), width, height, speck_px);
    let metrics = measure_micro(&in_px, &reference, width, height, scale, micro_mm2(preset));
    // Outer outlines are clockwise on the canvas (positive in kurbo's sign), holes against.
    let holes = in_px.iter().filter(|o| common::area(o) < 0.0).count();
    PresetRun {
        metrics,
        paths: result.counts.paths,
        nodes: result.counts.nodes,
        holes,
        area_error_true_pct: image
            .truth_4x
            .as_ref()
            .map(|truth| area_error_true_pct(&in_px, truth, width, height)),
    }
}

/// Every preset on the set and the photograph, by preset key and image name.
fn preset_runs(set: &[QualityImage]) -> BTreeMap<&'static str, BTreeMap<&'static str, PresetRun>> {
    PRESETS
        .iter()
        .map(|preset| {
            let runs = set
                .iter()
                .map(|image| (image.name, preset_run(preset, image)))
                .collect();
            (preset.key, runs)
        })
        .collect()
}

/// The set of criterion 28 and the photograph of criterion 61.
fn set_with_photograph() -> Vec<QualityImage> {
    let mut set = quality_set();
    set.push(photo_image());
    set
}

/// The regions of `mask` equal to `value` (foreground 8-connected, background 4-connected, as
/// the tracer labels them): each region's pixels and whether it touches the border.
fn regions(mask: &[bool], width: usize, height: usize, value: bool) -> Vec<(Vec<usize>, bool)> {
    let mut seen = vec![false; mask.len()];
    let mut found = Vec::new();
    for start in 0..mask.len() {
        if mask[start] != value || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut pixels = Vec::new();
        seen[start] = true;
        let mut border = false;
        while let Some(i) = stack.pop() {
            pixels.push(i);
            let (x, y) = (i % width, i / width);
            border |= x == 0 || y == 0 || x + 1 == width || y + 1 == height;
            let mut visit = |j: usize| {
                if mask[j] == value && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < width {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - width);
            }
            if y + 1 < height {
                visit(i + width);
            }
            if value {
                for (dx, dy) in [(-1_i64, -1_i64), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                        visit(ny as usize * width + nx as usize);
                    }
                }
            }
        }
        found.push((pixels, border));
    }
    found
}

/// The holes of the thresholded picture: background regions (4-connected) not touching the
/// border.
fn reference_holes(mask: &[bool], width: usize, height: usize) -> usize {
    regions(mask, width, height, false)
        .iter()
        .filter(|(_, border)| !border)
        .count()
}

/// `mask` after the tracer's Specks step (criterion 7): foreground regions under `min_pixels`
/// become background, then holes under it are filled. The deviation of a preset is measured
/// against this, so a speck the preset drops on purpose is not counted as deviation.
fn without_specks(mask: &[bool], width: usize, height: usize, min_pixels: f64) -> Vec<bool> {
    let mut mask = mask.to_vec();
    for (value, holes_only) in [(true, false), (false, true)] {
        for (pixels, border) in regions(&mask, width, height, value) {
            if (pixels.len() as f64) < min_pixels && !(holes_only && border) {
                for i in pixels {
                    mask[i] = !value;
                }
            }
        }
    }
    mask
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

/// The misses of `ours` against the committed `candidate` numbers, with the guard of
/// `specs/0089` criterion 9a (which replaces criterion 28's +5 % on every metric for the
/// geometry metrics): any micro path or an outline that is not closed; nodes and wobble per
/// 100 mm above the larger of +25 % and +3 nodes or +2 flips; the deviations from the pixel
/// border (max, mean, area error against the thresholded mask) are reported only on this
/// synthetic set (9a guards them on real images and guards the true-shape error instead, see
/// `quality_table`).
fn misses(ours: &Metrics, committed: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    if ours.micro_paths > 0.0 {
        out.push(format!("micro_paths {} > 0", ours.micro_paths));
    }
    if ours.closed_pct < 100.0 {
        out.push(format!("closed_pct {:.1} < 100", ours.closed_pct));
    }
    let Some(committed) = committed else {
        return out;
    };
    for (name, floor) in [("node_density", 3.0), ("wobble", 2.0)] {
        let old = committed[name].as_f64().unwrap_or(f64::NAN);
        let value = match name {
            "node_density" => ours.node_density,
            _ => ours.wobble,
        };
        let limit = (old * 1.25).max(old + floor);
        if value > limit + 1e-9 {
            out.push(format!("{name}: {old} -> {value:.4} (limit {limit:.4})"));
        }
    }
    out
}

/// The true-shape miss of a preset run (`specs/0089` criterion 9a: at most the baseline plus
/// 5 % plus half a percentage point).
fn true_shape_miss(ours: Option<f64>, committed: Option<&Value>) -> Option<String> {
    let value = ours?;
    let old = committed?.get("area_error_true_pct")?.as_f64()?;
    let limit = old * 1.05 + 0.5;
    (value > limit + 1e-9)
        .then(|| format!("area_error_true_pct: {old} -> {value:.4} (limit {limit:.4})"))
}

/// Prints the gap to Inkscape's default run per metric; a gap is information, not a miss.
fn print_inkscape_gap(name: &str, ours: &Metrics, inkscape: &Value) {
    let gaps: Vec<String> = Metrics::NAMES
        .iter()
        .zip(ours.values())
        .filter_map(|(metric, value)| {
            let ink = inkscape[*metric].as_f64()?;
            Some(format!("{metric} {value:.2} vs {ink:.2}"))
        })
        .collect();
    println!("  Inkscape gap {name}: {}", gaps.join(", "));
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
        if let Some(ink) = inkscape {
            print_inkscape_gap(image.name, &ours, ink);
        }
        let committed = baseline.get("candidate").and_then(|c| c.get(image.name));
        for miss in misses(&ours, committed) {
            regressions.push(format!("{}: {miss}", image.name));
        }
        ours_all.insert(image.name.to_string(), metrics_json(&ours));
    }
    let n = set.len() as f64;
    let means: Vec<String> = sums.iter().map(|s| format!("{:>10.3}", s / n)).collect();
    println!("{:<22}{}", "mean", means.join(" "));
    // Criterion 61: every preset on the set and the photograph, guarded per preset (+5 %).
    let photo_set = set_with_photograph();
    let runs = preset_runs(&photo_set);
    let mut profiles = serde_json::Map::new();
    for preset in PRESETS {
        println!("preset {}", preset.key);
        let mut stored = serde_json::Map::new();
        for image in &photo_set {
            let run = &runs[preset.key][image.name];
            let values: Vec<String> = run
                .metrics
                .values()
                .iter()
                .map(|v| format!("{v:>10.3}"))
                .collect();
            println!(
                "  {:<20}{} {:>6} paths {:>6} nodes",
                image.name,
                values.join(" "),
                run.paths,
                run.nodes
            );
            let committed = baseline
                .get("candidate_profiles")
                .and_then(|c| c.get(preset.key))
                .and_then(|c| c.get(image.name));
            for miss in misses(&run.metrics, committed) {
                regressions.push(format!("{} {}: {miss}", preset.key, image.name));
            }
            if let Some(miss) = true_shape_miss(run.area_error_true_pct, committed) {
                regressions.push(format!("{} {}: {miss}", preset.key, image.name));
            }
            let mut values = metrics_json(&run.metrics);
            values["area_error_true_pct"] =
                json!(run.area_error_true_pct.map(|v| (v * 1e4).round() / 1e4));
            stored.insert(image.name.to_string(), values);
        }
        profiles.insert(preset.key.to_string(), Value::Object(stored));
    }
    // Criterion 65 (Logo / Line art equals the stored candidate) is lifted by `specs/0089`
    // (Frage 87 A: Logo may change); the candidate stays the measured reference of the guard.
    if std::env::var_os("CURVYO_UPDATE_BASELINE").is_some() || baseline_dir.is_some() {
        baseline["candidate_profiles"] = Value::Object(profiles);
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
        "misses of the guard of `specs/0089` criterion 9a against the committed candidate:\n{}",
        regressions.join("\n")
    );
}

/// The geometric mean of `values`.
fn geometric_mean(values: &[f64]) -> f64 {
    (values.iter().map(|v| v.ln()).sum::<f64>() / values.len() as f64).exp()
}

/// Criteria 62 to 64: the relations of the presets to Logo / Line art on the set, and Photo on
/// the photograph; prints the numbers the PR lists. The reduction bound of criterion 63 (within
/// 0.2 mm of the trace before it) is the `--lib` test
/// `silhouette_stays_within_the_smooth_tolerance_on_the_set`.
#[test]
#[ignore = "quality harness: run in release with --ignored --nocapture"]
fn preset_relations() {
    let photo_set = set_with_photograph();
    let runs = preset_runs(&photo_set);
    let (logo, engraving, silhouette, photo) = (
        &runs["logo"],
        &runs["engraving"],
        &runs["silhouette"],
        &runs["photo"],
    );
    let mut misses = Vec::new();
    let set = quality_set();
    println!(
        "image                  nodes L/E/S        density L/E/S          max dev L/E/S        area err true L/E"
    );
    let (mut max_logo, mut max_engraving) = (0.0, 0.0);
    let (mut area_logo, mut area_engraving) = (0.0, 0.0);
    let (mut engraving_ratios, mut silhouette_ratios) = (Vec::new(), Vec::new());
    for image in &set {
        let (l, e, s) = (
            &logo[image.name],
            &engraving[image.name],
            &silhouette[image.name],
        );
        let (true_l, true_e) = (
            l.area_error_true_pct.unwrap(),
            e.area_error_true_pct.unwrap(),
        );
        println!(
            "{:<22} {:>5}/{:>5}/{:>5}  {:>6.1}/{:>6.1}/{:>6.1}  {:>5.2}/{:>5.2}/{:>5.2}  {:>6.2}/{:>6.2}",
            image.name,
            l.nodes,
            e.nodes,
            s.nodes,
            l.metrics.node_density,
            e.metrics.node_density,
            s.metrics.node_density,
            l.metrics.max_deviation_px,
            e.metrics.max_deviation_px,
            s.metrics.max_deviation_px,
            true_l,
            true_e,
        );
        max_logo += l.metrics.max_deviation_px;
        max_engraving += e.metrics.max_deviation_px;
        area_logo += true_l;
        area_engraving += true_e;
        // Criterion 62: Engraving is finer.
        if e.metrics.max_deviation_px > l.metrics.max_deviation_px + 1e-3 {
            misses.push(format!(
                "62 {}: Engraving max deviation {:.3} > Logo {:.3}",
                image.name, e.metrics.max_deviation_px, l.metrics.max_deviation_px
            ));
        }
        if true_e > true_l + 0.5 {
            misses.push(format!(
                "62 {}: Engraving area error {true_e:.3} > Logo {true_l:.3} + 0.5",
                image.name
            ));
        }
        let ratio = e.metrics.node_density / l.metrics.node_density;
        engraving_ratios.push(ratio);
        if ratio > 4.0 {
            misses.push(format!(
                "62 {}: Engraving density {:.1} > 4 x Logo {:.1}",
                image.name, e.metrics.node_density, l.metrics.node_density
            ));
        }
        // Criterion 63: Silhouette is coarser where Logo has at least 10 nodes per 100 mm.
        if l.metrics.node_density >= 10.0 {
            silhouette_ratios.push(s.metrics.node_density / l.metrics.node_density);
            if s.metrics.node_density > l.metrics.node_density {
                misses.push(format!(
                    "63 {}: Silhouette density {:.1} > Logo {:.1}",
                    image.name, s.metrics.node_density, l.metrics.node_density
                ));
            }
        }
        if s.paths > l.paths {
            misses.push(format!(
                "63 {}: Silhouette {} outlines > Logo {}",
                image.name, s.paths, l.paths
            ));
        }
        // Criterion 64: nodes Engraving >= Logo >= Silhouette.
        if e.nodes < l.nodes || l.nodes < s.nodes {
            misses.push(format!(
                "64 {}: nodes {} / {} / {} not monotone",
                image.name, e.nodes, l.nodes, s.nodes
            ));
        }
        // Micro paths: below 0.95 x the preset's own Specks² (3.8 mm² for Silhouette).
        for (key, r) in [("engraving", e), ("silhouette", s)] {
            if r.metrics.closed_pct < 100.0 || r.metrics.micro_paths > 0.0 {
                misses.push(format!(
                    "{}: {key} closed {} %, {} micro paths",
                    image.name, r.metrics.closed_pct, r.metrics.micro_paths
                ));
            }
        }
    }
    let n = set.len() as f64;
    let (engraving_mean, silhouette_mean) = (
        geometric_mean(&engraving_ratios),
        geometric_mean(&silhouette_ratios),
    );
    let silhouette_max_dev = set
        .iter()
        .map(|i| silhouette[i.name].metrics.max_deviation_px)
        .fold(0.0, f64::max);
    println!(
        "mean max deviation: Logo {:.3} px, Engraving {:.3} px ({:.0} % lower); \
         mean area error true: Logo {:.2} %, Engraving {:.2} % ({:.0} % lower)",
        max_logo / n,
        max_engraving / n,
        (1.0 - max_engraving / max_logo) * 100.0,
        area_logo / n,
        area_engraving / n,
        (1.0 - area_engraving / area_logo) * 100.0,
    );
    println!(
        "density ratio to Logo, geometric mean: Engraving {engraving_mean:.2} (worst {:.2}), \
         Silhouette {silhouette_mean:.2} (worst {:.2}); Silhouette max deviation {silhouette_max_dev:.3} px",
        engraving_ratios.iter().copied().fold(0.0, f64::max),
        silhouette_ratios.iter().copied().fold(0.0, f64::max),
    );
    if max_engraving > 0.75 * max_logo {
        misses.push(format!(
            "62: mean max deviation Engraving {:.3} not 25 % below Logo {:.3}",
            max_engraving / n,
            max_logo / n
        ));
    }
    if area_engraving > 0.75 * area_logo {
        misses.push(format!(
            "62: mean area error Engraving {:.3} not 25 % below Logo {:.3}",
            area_engraving / n,
            area_logo / n
        ));
    }
    if engraving_mean > 2.5 {
        misses.push(format!(
            "62: Engraving density {engraving_mean:.2} x Logo as the geometric mean > 2.5"
        ));
    }
    if silhouette_mean > 0.6 {
        misses.push(format!(
            "63: Silhouette density {silhouette_mean:.2} x Logo as the geometric mean > 0.6"
        ));
    }
    // Criterion 62's test: the 300 dpi text keeps the holes of letters.
    let text = set.iter().find(|i| i.name == "03-text-300dpi").unwrap();
    let mask = mask_of(text, auto_threshold(text.picture.raster()));
    let reference = reference_holes(
        &mask,
        text.picture.width as usize,
        text.picture.height as usize,
    );
    let (holes_e, holes_l) = (engraving[text.name].holes, logo[text.name].holes);
    println!("03-text-300dpi holes: reference {reference}, Engraving {holes_e}, Logo {holes_l}");
    if (holes_e as f64) < 0.98 * reference as f64 {
        misses.push(format!(
            "62: Engraving keeps {holes_e} of {reference} letter holes"
        ));
    }
    // Criterion 63, Photo on the photograph: many shapes.
    let (p, l) = (&photo["13-photograph"], &logo["13-photograph"]);
    println!(
        "13-photograph outlines: Photo {}, Logo {} ({:.1} x); Photo max deviation {:.3} px",
        p.paths,
        l.paths,
        p.paths as f64 / l.paths.max(1) as f64,
        p.metrics.max_deviation_px
    );
    if p.paths < 3 * l.paths {
        misses.push(format!(
            "63: Photo {} outlines < 3 x Logo {}",
            p.paths, l.paths
        ));
    }
    if p.metrics.closed_pct < 100.0 || p.metrics.micro_paths > 0.0 {
        misses.push(format!(
            "63: Photo closed {} %, {} micro paths",
            p.metrics.closed_pct, p.metrics.micro_paths
        ));
    }
    assert!(
        misses.is_empty(),
        "preset relations missed:\n{}",
        misses.join("\n")
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
