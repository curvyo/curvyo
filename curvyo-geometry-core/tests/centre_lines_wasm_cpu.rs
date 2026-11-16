//! The wasm budgets of Find centre lines (`specs/0117-centre-lines` criterion 18, ADR 0063 §10),
//! measured in Node on `wasm32-unknown-unknown` by the process's CPU time (`process.cpuUsage`): the traced whale tiled to 5,046 nodes (Apply ≤ 1.0 s, an option change ≤ 0.3 s) and to
//! 99,876 nodes (Apply ≤ 15 s). Release builds only:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!   cargo test --release --target wasm32-unknown-unknown -p curvyo-geometry-core \
//!   --test centre_lines_wasm_cpu -- --nocapture
//! ```

#![cfg(target_arch = "wasm32")]
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::{CentreOptions, CentreSkeleton, Outline};
use wasm_bindgen_test::wasm_bindgen_test as test;

const WHALE: &str = include_str!("fixtures/centre_lines/whale_traced.txt");

type Triple = (Point, Vec2, Vec2);

fn whale(dx: f64, dy: f64) -> Vec<Vec<(Vec<Triple>, bool)>> {
    let mut subjects: Vec<Vec<(Vec<Triple>, bool)>> = Vec::new();
    for line in WHALE.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => subjects
                .last_mut()
                .unwrap()
                .push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                subjects.last_mut().unwrap().last_mut().unwrap().0.push((
                    Point::new(v[0] + dx, v[1] + dy),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

fn herd(copies: usize) -> Vec<Vec<(Vec<Triple>, bool)>> {
    (0..copies)
        .flat_map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = ((i % 10) as f64 * 160.0, (i / 10) as f64 * 100.0);
            whale(x, y)
        })
        .collect()
}

fn build(subjects: &[Vec<(Vec<Triple>, bool)>]) -> Vec<CentreSkeleton> {
    subjects
        .iter()
        .map(|s| {
            let outlines: Vec<Outline<'_>> = s.iter().map(|(a, c)| Outline::new(a, *c)).collect();
            CentreSkeleton::build(&outlines).unwrap()
        })
        .collect()
}

fn finish(skeletons: &[CentreSkeleton], options: &CentreOptions) -> usize {
    skeletons
        .iter()
        .map(|s| s.finish(options, &[]).paths.len())
        .sum()
}

/// Node's CPU time of this process (user and system), in seconds: `process.cpuUsage()`.
fn cpu_seconds() -> f64 {
    use js_sys::{Function, Reflect};
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    let process = Reflect::get(&global, &"process".into()).unwrap();
    let usage: Function = Reflect::get(&process, &"cpuUsage".into())
        .unwrap()
        .dyn_into()
        .unwrap();
    let value = usage.call0(&process).unwrap();
    let part = |name: &str| {
        Reflect::get(&value, &name.into())
            .unwrap()
            .as_f64()
            .unwrap()
    };
    (part("user") + part("system")) / 1e6
}

fn seconds_of<T>(job: impl FnOnce() -> T) -> (T, f64) {
    let start = cpu_seconds();
    let out = job();
    (out, cpu_seconds() - start)
}

/// The wasm limit in seconds; a debug build only prints.
fn check(name: &str, seconds: f64, limit: f64) {
    web_sys_log(&format!("{name}: {seconds:.3} s (wasm limit {limit} s)"));
    if !cfg!(debug_assertions) {
        assert!(seconds <= limit, "{name}: {seconds} s");
    }
}

fn web_sys_log(text: &str) {
    wasm_bindgen_test::console_log!("{text}");
}

#[test]
fn criterion_18_wasm_apply_and_an_option_change_on_5000_nodes() {
    let subjects = herd(29);
    let ((skeletons, _), apply) = seconds_of(|| {
        let skeletons = build(&subjects);
        let paths = finish(&skeletons, &CentreOptions::default());
        (skeletons, paths)
    });
    let options = CentreOptions {
        overlap: -0.1,
        thick_parts: Some(Length::from_mm(5.0)),
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    let (_, change) = seconds_of(|| finish(&skeletons, &options));
    check("Apply, 5,046 nodes", apply, 1.0);
    check("Option change, 5,046 nodes", change, 0.3);
}

#[test]
fn criterion_18_wasm_apply_on_100000_nodes() {
    let subjects = herd(574);
    let (_, apply) = seconds_of(|| finish(&build(&subjects), &CentreOptions::default()));
    check("Apply, 99,876 nodes", apply, 15.0);
    web_sys_log(&format!(
        "wasm memory after: {} MB",
        memory_bytes() / 1_000_000.0
    ));
}

/// The size of this module's linear memory, in bytes (`memory.buffer.byteLength`): its peak.
fn memory_bytes() -> f64 {
    use js_sys::Reflect;
    let buffer = Reflect::get(&wasm_bindgen::memory(), &"buffer".into()).unwrap();
    Reflect::get(&buffer, &"byteLength".into())
        .unwrap()
        .as_f64()
        .unwrap()
}

/// Each slider of the bar on the staged memo the worker keeps (ADR 0063 amendment 2026-11-18, W
/// and C): one stage run per value change, wasm limit 0.3 s at 5,046 nodes.
#[test]
fn criterion_18_wasm_each_slider_on_the_staged_memo() {
    use curvyo_geometry_core::CentreStages;
    let mut stages: Vec<CentreStages> = build(&herd(29))
        .into_iter()
        .map(CentreStages::new)
        .collect();
    let mut run = |options: &CentreOptions| {
        stages
            .iter_mut()
            .map(|s| s.finish(options, &[]).paths.len())
            .sum::<usize>()
    };
    let base = CentreOptions::default();
    let (_, first) = seconds_of(|| run(&base));
    check("First finish, 5,046 nodes", first, 1.0);
    let steps = [
        (
            "Overlap",
            CentreOptions {
                overlap: 0.1,
                ..base
            },
        ),
        (
            "Thick parts",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                ..base
            },
        ),
        (
            "Optimise",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                ..base
            },
        ),
        (
            "Offset",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                inner_offset: Some(Length::from_mm(0.5)),
            },
        ),
    ];
    for (name, options) in steps {
        let (_, took) = seconds_of(|| run(&options));
        check(&format!("{name} change, 5,046 nodes"), took, 0.3);
    }
}
