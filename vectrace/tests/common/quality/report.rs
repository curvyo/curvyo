//! The reference report (`specs/0089-trace-quality-and-colour` criteria 3, 4, 9): the guard
//! against the committed `reference` numbers and the frozen targets, the per-group summary,
//! the compact JSON form of the `reference` section of `baseline.json`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value, json};

/// The metrics of one run, by name (finite numbers; an infinite or missing value is `null`).
pub type Row = BTreeMap<&'static str, f64>;

/// Topology counts no image may have more of than its baseline (`specs/0089` criterion 9a,
/// hard: lost only; the extra counts are reported, lead 2026-10-29).
pub const TOPOLOGY: &[&str] = &["components_lost", "connections_lost", "holes_lost"];

/// Deviations from the true shape: at most the baseline plus 5 % plus 0.05 px.
pub const TRUE_SHAPE: &[&str] = &[
    "dev_trace_max_px",
    "dev_trace_mean_px",
    "dev_truth_max_px",
    "dev_truth_mean_px",
];

/// Logo / Line art's nodes per 100 mm as a geometric mean over the set: at most this share of
/// the baseline's (criterion 9, frozen 2026-10-29).
pub const NODE_TARGET: f64 = 0.75;

fn value(row: &Value, name: &str) -> Option<f64> {
    row.get(name).and_then(Value::as_f64)
}

/// One miss of the guard: the metric (`name`), the measured `value` (none when missing) and the
/// sentence for the report. The accepted state (`accepted_reference.json`) is keyed by `name`.
pub struct Miss {
    pub name: String,
    pub value: Option<f64>,
    pub text: String,
}

/// The guard's misses of one image (`ours` against `committed`, criterion 9 as frozen on
/// 2026-10-29): nothing lost beyond the baseline, no self-intersection, the true-shape deviation within
/// the baseline plus 5 % plus 0.05 px, the deviation from the pixel border on real images
/// within the larger of the baseline plus 5 % and the tolerance plus 1.5 px, nodes and wobble
/// per 100 mm within the larger of the baseline plus 25 % and the baseline plus 3 nodes or
/// 2 flips.
pub fn misses(ours: &Value, committed: &Value, tolerance_px: f64, real: bool) -> Vec<Miss> {
    let mut out = Vec::new();
    let mut check = |name: &str, limit: f64| match value(ours, name) {
        Some(new) if new <= limit + 1e-9 => {}
        new => out.push(Miss {
            name: name.to_string(),
            value: new,
            text: format!(
                "{name}: {:?} -> {new:?} (limit {limit:.4})",
                value(committed, name)
            ),
        }),
    };
    for name in TOPOLOGY {
        if let Some(old) = value(committed, name) {
            check(name, old);
        }
    }
    check("self_intersections", 0.0);
    for name in TRUE_SHAPE {
        if let Some(old) = value(committed, name) {
            check(name, old * 1.05 + 0.05);
        }
    }
    if real && let Some(old) = value(committed, "max_deviation_px") {
        check("max_deviation_px", (old * 1.05).max(tolerance_px + 1.5));
    }
    if let Some(old) = value(committed, "node_density") {
        check("node_density", (old * 1.25).max(old + 3.0));
    }
    if let Some(old) = value(committed, "wobble") {
        check("wobble", (old * 1.25).max(old + 2.0));
    }
    out
}

/// The geometric mean of the positive values of `name` over `rows`.
pub fn geo_mean_of(rows: &[&Value], name: &str) -> f64 {
    geo_mean(
        &rows
            .iter()
            .filter_map(|r| value(r, name))
            .collect::<Vec<_>>(),
    )
}

/// The frozen targets of criteria 5 to 7 and 10 a run of Logo / Line art misses
/// (`specs/0089`, frozen 2026-10-29), by fixture and resolution; axis-aligned features on
/// half pixels are measured apart and reported only.
pub fn target_misses(key: &str, row: &Value) -> Vec<Miss> {
    let mut out = Vec::new();
    let Some((fixture, resolution)) = key.split_once('@') else {
        return out;
    };
    if !matches!(resolution, "1x" | "0.5x") {
        return out;
    }
    let mut over = |what: &str, name: &str, limit: f64| {
        if let Some(v) = value(row, name).filter(|v| *v > limit + 1e-9) {
            out.push(Miss {
                name: format!("criterion {what}: {name}"),
                value: Some(v),
                text: format!("criterion {what}: {name} {v:.4} > {limit}"),
            });
        }
    };
    if fixture == "geo-corners" {
        over("5", "corner_err_max_px", 0.5);
    }
    if matches!(
        fixture,
        "geo-corners" | "geo-round" | "line-angles" | "logo-shapes"
    ) {
        over("6", "straight_segments_max", 2.0);
        over("6", "straight_dev_max_px", 0.35);
        over("6", "straight_flips", 0.0);
        over("6", "straight_lost", 0.0);
    }
    if matches!(fixture, "geo-round" | "sketch-bowed") {
        over("7", "smooth_dev_max_px", 0.5);
        over("7", "false_corners", 0.0);
    }
    out
}

/// A row as JSON, numbers rounded to 4 decimals, plus its group.
pub fn row_json(row: &Row, group: &str) -> Value {
    let mut map = Map::new();
    map.insert("group".into(), json!(group));
    for (name, value) in row {
        let v = if value.is_finite() {
            json!((value * 1e4).round() / 1e4)
        } else {
            Value::Null
        };
        map.insert((*name).to_string(), v);
    }
    Value::Object(map)
}

/// The `reference` section as compact JSON text: one line per run.
pub fn reference_text(reference: &Map<String, Value>) -> String {
    let mut text = String::from("{\n");
    let presets: Vec<_> = reference.iter().collect();
    for (pi, (preset, runs)) in presets.iter().enumerate() {
        let _ = writeln!(text, "    {}: {{", json!(preset));
        let runs = runs.as_object().cloned().unwrap_or_default();
        let n = runs.len();
        for (ri, (key, row)) in runs.iter().enumerate() {
            let comma = if ri + 1 < n { "," } else { "" };
            let _ = writeln!(text, "      {}: {}{comma}", json!(key), row);
        }
        let comma = if pi + 1 < presets.len() { "," } else { "" };
        let _ = writeln!(text, "    }}{comma}");
    }
    text.push_str("  }");
    text
}

/// `baseline` as text: pretty JSON with the `reference` section compact and last.
pub fn baseline_text(baseline: &Value) -> String {
    let mut rest = baseline.clone();
    let reference = rest.as_object_mut().and_then(|m| m.remove("reference"));
    let mut text = serde_json::to_string_pretty(&rest).unwrap_or_default();
    if let Some(Value::Object(reference)) = reference {
        text.truncate(text.trim_end().len() - 1);
        let text_trimmed = text.trim_end().to_string();
        text = text_trimmed;
        let _ = write!(
            text,
            ",\n  \"reference\": {}\n}}",
            reference_text(&reference)
        );
    }
    text.push('\n');
    text
}

fn geo_mean(values: &[f64]) -> f64 {
    let positive: Vec<f64> = values.iter().copied().filter(|v| *v > 0.0).collect();
    if positive.is_empty() {
        return 0.0;
    }
    (positive.iter().map(|v| v.ln()).sum::<f64>() / positive.len() as f64).exp()
}

fn get(row: &Value, name: &str) -> Option<f64> {
    row.get(name).and_then(Value::as_f64)
}

/// One line of the per-group summary for the runs `rows` (one preset, one resolution).
pub fn summary_line(label: &str, rows: &[&Value]) -> String {
    let all = |name: &str| -> Vec<f64> { rows.iter().filter_map(|r| get(r, name)).collect() };
    let max = |name: &str| all(name).into_iter().fold(f64::NAN, f64::max);
    let mean = |name: &str| {
        let v = all(name);
        if v.is_empty() {
            f64::NAN
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    // `+ 0.0` turns the empty sum's -0 into 0.
    let sum = |name: &str| all(name).iter().sum::<f64>() + 0.0;
    let missing_corners = rows
        .iter()
        .filter(|r| get(r, "corners").unwrap_or(0.0) > 0.0 && get(r, "corner_err_max_px").is_none())
        .count();
    format!(
        "| {label} | {} | {:.1} | {:.2} | {:.2} | {:.2} | {:.2}{} | {:.2} | {:.1} | {:.0} | {:.0} / {:.0} | {:.0} | {:.0} | {:.0} |",
        rows.len(),
        geo_mean(&all("node_density")),
        mean("dev_trace_max_px"),
        mean("dev_truth_max_px"),
        mean("dev_truth_mean_px"),
        max("corner_err_max_px"),
        if missing_corners > 0 { "*" } else { "" },
        max("straight_dev_max_px"),
        mean("straight_segments_max"),
        sum("straight_flips"),
        sum("components_lost") + sum("connections_lost") + sum("holes_lost"),
        sum("components_extra") + sum("connections_extra") + sum("holes_extra"),
        sum("false_corners"),
        sum("self_intersections"),
        sum("cpu_native_ms"),
    )
}

/// The header of the summary table.
pub const SUMMARY_HEADER: &str = "| group | n | nodes/100mm (geo) | dev trace max | dev truth max | dev truth mean | corner err max | straight dev max | segs/edge | flips | topo lost / extra | false corners | self-int | CPU ms |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";
