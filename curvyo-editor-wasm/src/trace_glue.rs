//! The Trace tool's settings across the wasm boundary (`specs/0025-raster-trace` criteria 6,
//! 41, 45, 59 and 60): the host's settings JSON read into trace values, and the schema and the
//! presets written as JSON. Host-independent, so ordinary `cargo test` covers it.

use curvyo_document_core::Length;
use curvyo_ui_core::{
    TRACE_ADVANCED_PARAMS, TRACE_PARAMS, TRACE_PRESETS, TraceDenoise, TraceParamKind,
    TraceParamUnit, TraceParams, TraceUpscale,
};

/// `settings` (the host's JSON: `threshold`, `smooth`, `specks` in mm, `corners`, `upscale` and
/// `denoise` as choice indices, `node_limit` or null, `invert`, `ignore_background`) as trace
/// values (criteria 41, 60); a missing or unreadable field takes its first-use value.
pub fn trace_settings(settings: &str) -> TraceParams {
    let json: serde_json::Value = serde_json::from_str(settings).unwrap_or_default();
    let byte = |name: &str| {
        json.get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u8::try_from(v).ok())
    };
    let flag = |name: &str| json.get(name).and_then(serde_json::Value::as_bool);
    let first = TraceParams::with_threshold(byte("threshold").unwrap_or(128));
    TraceParams {
        smooth: byte("smooth").unwrap_or(first.smooth),
        specks: json
            .get("specks")
            .and_then(serde_json::Value::as_f64)
            .map_or(first.specks, Length::from_mm),
        corners: byte("corners").unwrap_or(first.corners),
        upscale: byte("upscale").map_or(first.upscale, TraceUpscale::from_index),
        denoise: byte("denoise").map_or(first.denoise, TraceDenoise::from_index),
        node_limit: json
            .get("node_limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok()),
        invert: flag("invert").unwrap_or(first.invert),
        ignore_background: flag("ignore_background").unwrap_or(first.ignore_background),
        ..first
    }
}

/// The presets as JSON (criteria 59 and 60), in menu order: a list of `{key, label, detail,
/// values: {smooth, specks, corners, upscale, denoise}}`.
#[must_use]
pub fn trace_presets_json() -> String {
    let list: Vec<serde_json::Value> = TRACE_PRESETS
        .iter()
        .map(|p| {
            serde_json::json!({
                "key": p.key,
                "label": p.label,
                "detail": p.detail,
                "values": {
                    "smooth": p.smooth,
                    "specks": p.specks_mm,
                    "corners": p.corners,
                    "upscale": p.upscale.index(),
                    "denoise": p.denoise.index(),
                },
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

/// The settings schema as JSON (criteria 6 and 45): a list of `{name, label, kind, unit, min,
/// max, default, step, big_step, fine_step, group, advanced, choices}` the bar's fields and the
/// Advanced block are made from, the bar's first, then the Advanced block's in its order.
#[must_use]
pub fn trace_params_json() -> String {
    let list: Vec<serde_json::Value> = TRACE_PARAMS
        .iter()
        .chain(TRACE_ADVANCED_PARAMS)
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "label": p.label,
                "kind": match p.kind { TraceParamKind::Integer => "integer", TraceParamKind::Length => "length", TraceParamKind::Choice(_) => "choice", TraceParamKind::Toggle => "toggle" },
                "unit": match p.unit { TraceParamUnit::None => "", TraceParamUnit::Millimetre => "mm" },
                "min": p.min,
                "max": p.max,
                "default": p.default,
                "step": p.step,
                "big_step": p.big_step,
                "fine_step": p.fine_step,
                "group": p.group,
                "advanced": p.advanced,
                "choices": match p.kind { TraceParamKind::Choice(labels) => labels, _ => &[] },
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_json_reads_every_value_and_falls_back_to_the_first_use() {
        let p = trace_settings(
            r#"{"threshold":90,"smooth":100,"specks":2,"corners":30,"upscale":0,"denoise":1,
                "node_limit":500,"invert":true,"ignore_background":true}"#,
        );
        assert_eq!((p.threshold, p.smooth, p.corners), (90, 100, 30));
        assert!((p.specks.as_mm() - 2.0).abs() < 1e-12);
        assert_eq!(
            (p.upscale, p.denoise),
            (TraceUpscale::Auto, TraceDenoise::Low)
        );
        assert_eq!(p.node_limit, Some(500));
        assert!(p.invert && p.ignore_background);
        assert_eq!(trace_settings("nonsense"), TraceParams::with_threshold(128));
        let partial = trace_settings(r#"{"threshold":40}"#);
        assert_eq!(partial, TraceParams::with_threshold(40));
    }

    #[test]
    fn the_presets_and_the_schema_cross_as_json() {
        let presets: serde_json::Value = serde_json::from_str(&trace_presets_json()).unwrap();
        assert_eq!(presets.as_array().unwrap().len(), 4);
        assert_eq!(presets[2]["values"]["smooth"], 100);
        let params: serde_json::Value = serde_json::from_str(&trace_params_json()).unwrap();
        let names: Vec<&str> = params
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "threshold",
                "smooth",
                "specks",
                "denoise",
                "upscale",
                "corners",
                "node_limit",
                "invert",
                "ignore_background",
                "full_size_preview"
            ]
        );
        assert_eq!(
            params[3]["choices"],
            serde_json::json!(["Off", "Low", "High"])
        );
    }
}
