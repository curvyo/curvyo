//! The reference set and baseline of `specs/0089-trace-quality-and-colour` slice A (criteria 1 to
//! 4). `the_reference_set_is_complete` runs in the gate (metadata only); the rest is `#[ignore]`
//! and meant for a release build:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test reference -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `reference_table` traces every synthetic fixture at 1x, 0.5x and 0.25x and every committed real
//! fixture with each of the four presets, prints the per-group summary and fails on a miss of
//! the guard against the committed `reference` numbers in `tests/fixtures/trace/baseline.json`
//! (the visioncortex tracer of `c788b5a7`) or of the frozen targets (`report.rs`). `CURVYO_UPDATE_BASELINE=1` rewrites them. Peak memory is
//! measured per run in a child process of this binary (a counting allocator only ever rises).
//! It also writes every picture and a manifest to `target/trace-reference/` for the wasm timing
//! (`frontend/scripts/trace-reference-wasm.ts`), whose `wasm-times.json` it merges when present.
//! `CURVYO_TRACE_LOCAL=<dir>` adds the images of that directory as `local.<hash>`: only their
//! numbers are stored, never pixels or file names (Frage 82). For work on single images:
//! `CURVYO_TRACE_ONLY=a,b` keeps the cases whose key holds one of the names,
//! `CURVYO_TRACE_PRESETS=logo,photo` the presets, `CURVYO_TRACE_NO_PEAK=1` skips the
//! peak-memory child, and `CURVYO_TRACE_REPORT=<file>` writes the fresh numbers and the misses
//! as JSON without touching `baseline.json`.

// The harness measures peak memory and thread CPU time, which need Unix (every test in it is an `#[ignore]`d release run).
#![cfg(unix)]
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

use std::alloc::System;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cap::Cap;
use common::Picture;
use common::quality::real::{
    customer_images, customer_sidecar_complete, customer_sidecars, local_images, real_images,
    sidecar_complete, sidecars,
};
use common::quality::reference::{Edge, Feature, Group, SCALES};
use common::quality::report::{
    Miss, NODE_TARGET, SUMMARY_HEADER, baseline_text, geo_mean_of, misses, row_json, summary_line,
    target_misses,
};
use common::quality::run::{Case, Truth, case_params, measure, params_for};
use common::quality::shape::{corner_errors, measure_edge, self_intersections};
use common::quality::{reference_set, selection_set};
use curvyo_document_core::Affine;
use curvyo_vectorize_core::{PRESETS, Preset, Selection};
use kurbo::{BezPath, Point};
use serde_json::{Value, json};

#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, usize::MAX);

/// Every case: the synthetic fixtures at their three resolutions, the real and the local images.
fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    let colour = selection_set()
        .into_iter()
        .map(|s| (s.reference, Some(s.method)));
    for (reference, method) in reference_set().into_iter().map(|r| (r, None)).chain(colour) {
        for index in 0..SCALES.len() {
            let v = reference.variant(index);
            let selection = method.map_or(Selection::Brightness, |m| m.selection(&v.picture));
            out.push(Case {
                selection,
                key: v.key,
                group: Some(reference.group),
                picture: v.picture,
                mm_per_px: v.mm_per_px,
                truth: v.truth_4x.map(|raster_4x| Truth {
                    raster_4x,
                    paths: v.truth_paths,
                    corners: v.corners,
                    edges: v.edges,
                }),
            });
        }
    }
    for image in real_images()
        .into_iter()
        .chain(customer_images())
        .chain(local_images())
    {
        out.push(Case {
            selection: Selection::Brightness,
            key: image.id,
            group: image.group,
            mm_per_px: 25.4 / image.dpi,
            picture: image.picture,
            truth: None,
        });
    }
    // `CURVYO_TRACE_ONLY=a,b` keeps the cases whose key holds one of the names (a partial run
    // for work on one image; the guard still compares every traced run).
    if let Ok(only) = std::env::var("CURVYO_TRACE_ONLY") {
        let names: Vec<&str> = only.split(',').filter(|s| !s.is_empty()).collect();
        out.retain(|c| names.iter().any(|n| c.key.contains(n)));
    }
    out
}

/// The presets to run: all, or those named in `CURVYO_TRACE_PRESETS=logo,photo`.
fn presets() -> Vec<&'static Preset> {
    let wanted = std::env::var("CURVYO_TRACE_PRESETS").unwrap_or_default();
    PRESETS
        .iter()
        .filter(|p| wanted.is_empty() || wanted.split(',').any(|w| w == p.key))
        .collect()
}

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/trace-reference")
}

/// A file name for a case key (keys hold `@` and `.`).
fn file_name(key: &str) -> String {
    format!("{}.pam", key.replace(['@', '.'], "_"))
}

/// Writes `picture` as PAM (`RGB_ALPHA`).
fn write_pam(path: &Path, picture: &Picture) {
    let mut bytes = format!(
        "P7\nWIDTH {}\nHEIGHT {}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n",
        picture.width, picture.height
    )
    .into_bytes();
    bytes.extend_from_slice(&picture.rgba);
    std::fs::write(path, bytes).unwrap();
}

/// The settings JSON the wasm `trace_job` takes for `preset` on `case`.
fn wasm_settings(preset: &Preset, case: &Case) -> Value {
    let p = case_params(preset, case);
    let hex = |c: curvyo_vectorize_core::Rgb8| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b);
    let (method, background, swatches, tolerance) = match p.selection {
        Selection::Brightness => (0, None, Vec::new(), 0),
        Selection::RemoveBackground {
            background,
            tolerance,
        } => (1, background.map(hex), Vec::new(), tolerance),
        Selection::PickColours {
            swatches,
            background,
            tolerance,
        } => (
            2,
            background.map(hex),
            swatches.as_slice().iter().map(|&c| hex(c)).collect(),
            tolerance,
        ),
    };
    json!({
        "method": method,
        "background": background,
        "swatches": swatches,
        "colour_tolerance": tolerance,
        "threshold": p.threshold,
        "smooth": p.smooth,
        "tolerance": p.tolerance.as_px(),
        "specks": preset.specks_mm,
        "corners": p.corners,
        "upscale": p.upscale.index(),
        "denoise": p.denoise.index(),
    })
}

/// Runs the peak-memory child for `file` and `preset`: the peak in bytes per pixel above what
/// was allocated before the trace.
fn peak_bytes_per_px(file: &Path, preset: &Preset, mm: f64) -> Option<f64> {
    let output = std::process::Command::new(std::env::current_exe().ok()?)
        .args([
            "peak_memory_child",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            "CURVYO_TRACE_PEAK",
            format!("{}|{}|{mm}", file.display(), preset.key),
        )
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // libtest prints the test's name on the same line.
    let line = text
        .lines()
        .find_map(|l| l.split("PEAK_BYTES_PER_PX=").nth(1))?;
    line.trim().parse().ok()
}

/// The child of [`peak_bytes_per_px`]: reads the picture (allocating only its bytes), traces it
/// and prints the rise of the allocator's high-water mark per pixel.
#[test]
#[ignore = "the peak-memory child of reference_table"]
fn peak_memory_child() {
    let Ok(spec) = std::env::var("CURVYO_TRACE_PEAK") else {
        return;
    };
    let mut parts = spec.split('|');
    let (file, key, mm) = (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap().parse::<f64>().unwrap(),
    );
    let preset = PRESETS.iter().find(|p| p.key == key).unwrap();
    let mut bytes = std::fs::read(file).unwrap();
    let end = bytes.windows(7).position(|w| w == b"ENDHDR\n").unwrap() + 7;
    let header = String::from_utf8_lossy(&bytes[..end]).to_string();
    let field = |k: &str| -> u32 {
        header
            .lines()
            .find_map(|l| l.strip_prefix(k))
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    };
    let (width, height) = (field("WIDTH "), field("HEIGHT "));
    bytes.drain(..end);
    let picture = Picture {
        width,
        height,
        rgba: bytes,
    };
    let p = params_for(preset, &picture);
    let before = ALLOCATOR.allocated();
    let result = common::run(&picture, Affine::scale(mm, mm), &p);
    let peak = ALLOCATOR.max_allocated().saturating_sub(before);
    drop(result);
    println!(
        "PEAK_BYTES_PER_PX={}",
        peak as f64 / (f64::from(width) * f64::from(height))
    );
}

/// Criteria 1 and 2 on the metadata: every case of criterion 2 is covered, every primary group
/// has at least three fixtures, the diagram and photo groups one each, every real fixture has a
/// complete sidecar, its file and an entry in `SOURCES.md`.
#[test]
fn the_reference_set_is_complete() {
    let mut set = reference_set();
    set.extend(selection_set().into_iter().map(|s| s.reference));
    for feature in Feature::ALL {
        assert!(
            set.iter().any(|r| r.covers.contains(&feature)),
            "{feature:?} is not covered"
        );
    }
    let mut names: Vec<&str> = set.iter().map(|r| r.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), set.len(), "fixture names are unique");
    let mut per_group: BTreeMap<Group, usize> = BTreeMap::new();
    for r in &set {
        assert!(!r.preserve.is_empty() && r.dpi > 0.0, "{}: note", r.name);
        *per_group.entry(r.group).or_default() += 1;
    }
    let sources =
        std::fs::read_to_string(common::quality::real::reference_dir().join("SOURCES.md")).unwrap();
    for (path, sidecar) in sidecars() {
        sidecar_complete(&sidecar).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let file = sidecar["file"].as_str().unwrap();
        assert!(path.with_file_name(file).exists(), "{file} exists");
        assert!(sources.contains(file), "{file} is listed in SOURCES.md");
        let group = Group::from_key(sidecar["group"].as_str().unwrap()).unwrap();
        *per_group.entry(group).or_default() += 1;
    }
    for (path, sidecar) in customer_sidecars() {
        customer_sidecar_complete(&sidecar).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let id = sidecar["id"].as_str().unwrap();
        assert!(sources.contains(id), "{id} is listed in SOURCES.md");
    }
    assert_eq!(
        customer_sidecars().len(),
        31,
        "the customer's 31 images (2026-10-29)"
    );
    for group in Group::PRIMARY {
        assert!(
            per_group.get(&group).copied().unwrap_or(0) >= 3,
            "{group:?}: {per_group:?}"
        );
    }
    for group in [Group::Diagram, Group::Photo] {
        assert!(
            per_group.get(&group).copied().unwrap_or(0) >= 1,
            "{group:?}"
        );
    }
}

/// The slice A metrics on shapes with known answers: a square traced exactly has its corners as
/// nodes, one segment per side and no flips; a figure eight crosses itself once; a staircase
/// edge flips and deviates by its step.
#[test]
fn the_metrics_measure_known_shapes() {
    let square = common::quality::render::polygon(&[
        Point::new(10.0, 10.0),
        Point::new(50.0, 10.0),
        Point::new(50.0, 50.0),
        Point::new(10.0, 50.0),
    ]);
    let corners = [Point::new(10.0, 10.0), Point::new(50.0, 50.0)];
    assert_eq!(
        corner_errors(std::slice::from_ref(&square), &corners),
        vec![0.0, 0.0]
    );
    let side = Edge {
        points: vec![Point::new(10.0, 10.0), Point::new(50.0, 10.0)],
        straight: true,
        clearance: 10.0,
    };
    let r = measure_edge(std::slice::from_ref(&square), &side);
    assert!(
        !r.lost && r.deviation < 1e-9 && r.segments == 1 && r.flips == 0,
        "{r:?}"
    );
    let mut stairs = vec![Point::new(10.0, 10.0)];
    for i in 0..20 {
        let x = 10.0 + 2.0 * f64::from(i);
        stairs.extend([Point::new(x + 2.0, 10.0 + 0.5 * f64::from(i % 2))]);
    }
    stairs.extend([Point::new(50.0, 50.0), Point::new(10.0, 50.0)]);
    let r = measure_edge(&[common::quality::render::polygon(&stairs)], &side);
    assert!(r.flips > 5 && (r.deviation - 0.5).abs() < 1e-9, "{r:?}");
    let mut eight = BezPath::new();
    eight.move_to((0.0, 0.0));
    eight.line_to((20.0, 20.0));
    eight.line_to((20.0, 0.0));
    eight.line_to((0.0, 20.0));
    eight.close_path();
    assert_eq!(self_intersections(&[eight], 30, 30), 1);
    assert_eq!(self_intersections(&[square], 60, 60), 0);
}

/// The accepted misses in `tests/fixtures/trace/accepted_reference.json`, by
/// `<preset> <case>: <metric>` to the accepted value (`specs/0089` criterion 9a, customer
/// acceptance 2026-10-30). The nightly fails only on a miss that is not listed or on a listed
/// one more than 5 % worse. `CURVYO_UPDATE_ACCEPTED=1` rewrites the file from the current misses.
struct Accepted {
    stored: BTreeMap<String, f64>,
    recorded: BTreeMap<String, f64>,
}

impl Accepted {
    fn load() -> Self {
        let stored = std::fs::read_to_string(common::fixture_dir().join("accepted_reference.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            stored,
            recorded: BTreeMap::new(),
        }
    }

    /// Adds the miss `text` of `key` to `regressions` unless it is accepted and not worse.
    fn check(&mut self, regressions: &mut Vec<String>, key: &str, value: Option<f64>, text: &str) {
        let Some(value) = value else {
            regressions.push(text.to_string());
            return;
        };
        self.recorded
            .insert(key.to_string(), (value * 1e4).round() / 1e4);
        match self.stored.get(key) {
            Some(accepted) if value <= accepted * 1.05 + 1e-9 => {
                println!("accepted miss {text} (accepted {accepted})");
            }
            Some(accepted) => {
                regressions.push(format!("{text}; accepted {accepted} + 5 % exceeded"));
            }
            None => regressions.push(format!("{text}; not an accepted miss")),
        }
    }

    /// The same for a miss of the per-run guard.
    fn check_miss(&mut self, regressions: &mut Vec<String>, run: &str, miss: &Miss) {
        self.check(
            regressions,
            &format!("{run}: {}", miss.name),
            miss.value,
            &format!("{run}: {}", miss.text),
        );
    }

    fn write_if_asked(&self) -> bool {
        if std::env::var_os("CURVYO_UPDATE_ACCEPTED").is_none() {
            return false;
        }
        std::fs::write(
            common::fixture_dir().join("accepted_reference.json"),
            serde_json::to_string_pretty(&self.recorded).unwrap() + "\n",
        )
        .unwrap();
        true
    }
}

/// Criteria 3 and 4: the reference table, the summary and the +5 % guard.
#[cfg(unix)]
#[test]
#[ignore = "reference harness: run in release with --ignored --nocapture --test-threads=1"]
fn reference_table() {
    let path = common::fixture_dir().join("baseline.json");
    let mut baseline: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let dir = out_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let wasm: Value = std::fs::read_to_string(dir.join("wasm-times.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    let mut manifest = Vec::new();
    let mut fresh: BTreeMap<&str, serde_json::Map<String, Value>> = BTreeMap::new();
    let mut groups: BTreeMap<(String, &str, String), Vec<Value>> = BTreeMap::new();
    let mut regressions = Vec::new();
    let mut accepted = Accepted::load();
    for case in cases() {
        let file = dir.join(file_name(&case.key));
        write_pam(&file, &case.picture);
        let group = case.group.map_or("local", Group::key);
        for preset in presets() {
            let mut row = measure(&case, preset);
            // `CURVYO_TRACE_NO_PEAK=1` skips the peak-memory child (a quick partial run).
            if std::env::var_os("CURVYO_TRACE_NO_PEAK").is_none()
                && let Some(peak) = peak_bytes_per_px(&file, preset, case.mm_per_px)
            {
                row.insert("peak_bytes_per_px", peak);
            }
            let run_key = format!("{}|{}", case.key, preset.key);
            if let Some(ms) = wasm.get(&run_key).and_then(Value::as_f64) {
                row.insert("cpu_wasm_ms", ms);
            }
            manifest.push(json!({
                "key": run_key,
                "file": file_name(&case.key),
                "map": [case.mm_per_px, 0.0, 0.0, case.mm_per_px, 0.0, 0.0],
                "settings": wasm_settings(preset, &case),
            }));
            let value = row_json(&row, group);
            if let Some(old) = baseline
                .get("reference")
                .and_then(|r| r.get(preset.key))
                .and_then(|r| r.get(&case.key))
            {
                let real = !case.key.contains('@');
                for miss in misses(&value, old, preset.tolerance_px, real) {
                    accepted.check_miss(
                        &mut regressions,
                        &format!("{} {}", preset.key, case.key),
                        &miss,
                    );
                }
            }
            if preset.key == "logo" {
                for miss in target_misses(&case.key, &value) {
                    accepted.check_miss(
                        &mut regressions,
                        &format!("{} {}", preset.key, case.key),
                        &miss,
                    );
                }
            }
            let resolution = case.key.split('@').nth(1).unwrap_or("real").to_string();
            groups
                .entry((group.to_string(), preset.key, resolution))
                .or_default()
                .push(value.clone());
            fresh
                .entry(preset.key)
                .or_default()
                .insert(case.key.clone(), value);
        }
        println!("{} traced", case.key);
    }
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    for preset in presets() {
        println!("\npreset {}\n{SUMMARY_HEADER}", preset.key);
        for ((group, key, resolution), rows) in &groups {
            if *key == preset.key {
                let refs: Vec<&Value> = rows.iter().collect();
                println!("{}", summary_line(&format!("{group} {resolution}"), &refs));
            }
        }
    }
    // Nodes and wobble by geometric mean over the set: Logo / Line art's nodes at most
    // `NODE_TARGET` of the baseline's; no preset's wobble above its baseline's.
    for preset in presets() {
        let Some(runs) = fresh.get(preset.key) else {
            continue;
        };
        let Some(old) = baseline.get("reference").and_then(|r| r.get(preset.key)) else {
            continue;
        };
        let ours: Vec<&Value> = runs.values().collect();
        let theirs: Vec<&Value> = runs.keys().filter_map(|k| old.get(k)).collect();
        let (n_new, n_old) = (
            geo_mean_of(&ours, "node_density"),
            geo_mean_of(&theirs, "node_density"),
        );
        let (w_new, w_old) = (geo_mean_of(&ours, "wobble"), geo_mean_of(&theirs, "wobble"));
        println!(
            "{}: nodes/100 mm (geo) {n_old:.2} -> {n_new:.2}, wobble (geo) {w_old:.2} -> {w_new:.2}",
            preset.key
        );
        if preset.key == "logo" && n_new > NODE_TARGET * n_old {
            accepted.check(
                &mut regressions,
                "logo: nodes (geo)",
                Some(n_new),
                &format!("logo: nodes (geo) {n_new:.2} > {NODE_TARGET} x {n_old:.2}"),
            );
        }
        if w_new > w_old {
            accepted.check(
                &mut regressions,
                &format!("{}: wobble (geo)", preset.key),
                Some(w_new),
                &format!("{}: wobble (geo) {w_new:.2} > {w_old:.2}", preset.key),
            );
        }
    }
    // `CURVYO_TRACE_REPORT=<file>` writes the fresh numbers and the misses (a before/after
    // table for the PR without touching `baseline.json`).
    if let Ok(report) = std::env::var("CURVYO_TRACE_REPORT") {
        let fresh_json: serde_json::Map<String, Value> = fresh
            .iter()
            .map(|(preset, runs)| ((*preset).to_string(), Value::Object(runs.clone())))
            .collect();
        let text = serde_json::to_string_pretty(&json!({
            "reference": fresh_json,
            "misses": regressions,
        }))
        .unwrap();
        std::fs::write(&report, text).unwrap();
    }
    // 0025 criterion 63 as superseded (lead, 2026-10-29): Silhouette's nodes per 100 mm are at
    // most 0.8 x Logo / Line art's as the geometric mean over the set, never more per image.
    if let (Some(logo), Some(silhouette)) = (fresh.get("logo"), fresh.get("silhouette")) {
        let mut ratios = Vec::new();
        for (key, l) in logo {
            let Some(s) = silhouette.get(key) else {
                continue;
            };
            let (dl, ds) = (
                l.get("node_density").and_then(Value::as_f64).unwrap_or(0.0),
                s.get("node_density").and_then(Value::as_f64).unwrap_or(0.0),
            );
            if dl > 0.0 && ds > 0.0 {
                ratios.push(ds / dl);
            }
            if ds > dl + 1e-9 {
                accepted.check(
                    &mut regressions,
                    &format!("silhouette {key}: nodes/100 mm above Logo / Line art"),
                    Some(ds),
                    &format!(
                        "silhouette {key}: nodes/100 mm {ds:.2} > Logo / Line art {dl:.2} (0025 criterion 63)"
                    ),
                );
            }
        }
        let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / ratios.len().max(1) as f64).exp();
        println!("silhouette / logo nodes per 100 mm (geo): {geo:.3}");
        if geo > 0.8 {
            accepted.check(
                &mut regressions,
                "silhouette: nodes/100 mm (geo) to Logo / Line art",
                Some(geo),
                &format!(
                    "silhouette: nodes/100 mm (geo) {geo:.3} x Logo / Line art > 0.8 (0025 criterion 63)"
                ),
            );
        }
    }
    if accepted.write_if_asked() {
        return;
    }
    if std::env::var_os("CURVYO_UPDATE_BASELINE").is_some() {
        let reference = baseline
            .as_object_mut()
            .unwrap()
            .entry("reference")
            .or_insert_with(|| json!({}));
        for (preset, runs) in fresh {
            let stored = reference
                .as_object_mut()
                .unwrap()
                .entry(preset)
                .or_insert_with(|| json!({}));
            for (key, value) in runs {
                stored[key] = value;
            }
        }
        std::fs::write(&path, baseline_text(&baseline)).unwrap();
        return;
    }
    assert!(
        regressions.is_empty(),
        "misses of the guard or the frozen targets (`specs/0089` criteria 5 to 10):\n{}",
        regressions.join("\n")
    );
}
