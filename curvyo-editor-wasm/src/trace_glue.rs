//! The Trace tool's settings across the wasm boundary (`specs/0025-raster-trace` criteria 6,
//! 41, 45, 59 and 60): the host's settings JSON read into trace values, and the schema and the
//! presets written as JSON. Host-independent, so ordinary `cargo test` covers it.

use curvyo_document_core::Length;
use curvyo_ui_core::{
    TRACE_ADVANCED_PARAMS, TRACE_DEFAULT_BACKGROUND_TOLERANCE, TRACE_DEFAULT_PICK_TOLERANCE,
    TRACE_MAX_COLOURS, TRACE_METHOD_PARAMS, TRACE_MIN_COLOURS, TRACE_PARAMS, TRACE_PRESETS,
    TraceColourParams, TraceDenoise, TraceMethod, TraceParamKind, TraceParamUnit, TraceParams,
    TracePixelLength, TraceRgb, TraceSwatches, TraceUpscale,
};

/// `#RRGGBB` (or `RRGGBB`) as a colour; `None` for anything else.
#[must_use]
pub fn parse_hex(text: &str) -> Option<TraceRgb> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some(TraceRgb::new(channel(0)?, channel(2)?, channel(4)?))
}

/// A colour as `#RRGGBB`.
#[must_use]
pub fn hex(colour: TraceRgb) -> String {
    format!("#{:02X}{:02X}{:02X}", colour.r, colour.g, colour.b)
}

/// The method of `json` (`method` 0 Brightness, 1 Remove background, 2 Pick colours; `background`
/// `#RRGGBB` or null for a transparent one, the colour of both colour methods; `swatches` a list of `#RRGGBB`; `colour_tolerance`
/// 0 to 100, the method's default when missing), Brightness for anything unreadable.
fn method(json: &serde_json::Value) -> TraceMethod {
    let tolerance = |default: u8| {
        json.get("colour_tolerance")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u8::try_from(v.min(100)).ok())
            .unwrap_or(default)
    };
    let background = json
        .get("background")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_hex);
    match json.get("method").and_then(serde_json::Value::as_u64) {
        Some(1) => TraceMethod::RemoveBackground {
            background,
            tolerance: tolerance(TRACE_DEFAULT_BACKGROUND_TOLERANCE),
        },
        Some(2) => {
            let colours: Vec<TraceRgb> = json
                .get("swatches")
                .and_then(serde_json::Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(|v| v.as_str().and_then(parse_hex))
                        .collect()
                })
                .unwrap_or_default();
            TraceMethod::PickColours {
                swatches: TraceSwatches::from_slice(&colours),
                background,
                tolerance: tolerance(TRACE_DEFAULT_PICK_TOLERANCE),
            }
        }
        _ => TraceMethod::Brightness,
    }
}

/// `settings` (the host's JSON: `threshold`, `smooth`, `tolerance` in source px, `specks` in mm,
/// `corners`, `upscale` and `denoise` as choice indices, `invert`, `ignore_background`, and the
/// method's `method`, `background`, `swatches`, `colour_tolerance`) as trace values (criteria
/// 41, 60; `specs/0089` criterion 14); a missing or unreadable field takes its first-use value.
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
        selection: method(&json),
        smooth: byte("smooth").unwrap_or(first.smooth),
        tolerance: json
            .get("tolerance")
            .and_then(serde_json::Value::as_f64)
            .map_or(first.tolerance, TracePixelLength::from_px),
        specks: json
            .get("specks")
            .and_then(serde_json::Value::as_f64)
            .map_or(first.specks, Length::from_mm),
        corners: byte("corners").unwrap_or(first.corners),
        upscale: byte("upscale").map_or(first.upscale, TraceUpscale::from_index),
        denoise: byte("denoise").map_or(first.denoise, TraceDenoise::from_index),
        invert: flag("invert").unwrap_or(first.invert),
        ignore_background: flag("ignore_background").unwrap_or(first.ignore_background),
        ..first
    }
}

/// The colours of `list` (a JSON list of `#RRGGBB`), unreadable ones dropped.
fn colours(list: Option<&serde_json::Value>) -> Vec<TraceRgb> {
    list.and_then(serde_json::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().and_then(parse_hex))
                .collect()
        })
        .unwrap_or_default()
}

/// The multicolour values of `settings` (`specs/0089` slice M: `output` 1 for Multicolour,
/// `colours` a count 2 to 16 or null for Auto, `fixed` and `excluded` lists of `#RRGGBB`,
/// `merged` a list of `[from, into]`), or `None` for Single colour (`output` missing or 0).
#[must_use]
pub fn trace_colour_settings(settings: &str) -> Option<TraceColourParams> {
    let json: serde_json::Value = serde_json::from_str(settings).ok()?;
    if json.get("output").and_then(serde_json::Value::as_u64) != Some(1) {
        return None;
    }
    let count = json
        .get("colours")
        .and_then(serde_json::Value::as_u64)
        .map(|n| {
            // invariant: clamped to 2 to 16 first.
            u8::try_from(n.clamp(TRACE_MIN_COLOURS as u64, TRACE_MAX_COLOURS as u64)).unwrap_or(2)
        });
    let merged = json
        .get("merged")
        .and_then(serde_json::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|pair| {
                    let pair = pair.as_array()?;
                    Some((
                        parse_hex(pair.first()?.as_str()?)?,
                        parse_hex(pair.get(1)?.as_str()?)?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(TraceColourParams {
        trace: trace_settings(settings),
        colours: count,
        fixed: colours(json.get("fixed")),
        excluded: colours(json.get("excluded")),
        merged,
    })
}

/// The presets as JSON (criteria 59 and 60), in menu order: a list of `{key, label, detail,
/// values: {smooth, tolerance, specks, corners, upscale, denoise}}`.
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
                    "tolerance": p.tolerance_px,
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
/// Advanced block are made from, the bar's first, then the Advanced block's in its order, then
/// the method's (`specs/0089` criterion 14).
#[must_use]
pub fn trace_params_json() -> String {
    let list: Vec<serde_json::Value> = TRACE_PARAMS
        .iter()
        .chain(TRACE_ADVANCED_PARAMS)
        .chain(TRACE_METHOD_PARAMS)
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "label": p.label,
                "kind": match p.kind { TraceParamKind::Integer => "integer", TraceParamKind::Length => "length", TraceParamKind::Choice(_) => "choice", TraceParamKind::Toggle => "toggle" },
                "unit": match p.unit { TraceParamUnit::None => "", TraceParamUnit::Millimetre => "mm", TraceParamUnit::Pixel => "px" },
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
            r#"{"threshold":90,"smooth":100,"tolerance":1.25,"specks":2,"corners":30,"upscale":0,"denoise":1,
                "invert":true,"ignore_background":true}"#,
        );
        assert_eq!((p.threshold, p.smooth, p.corners), (90, 100, 30));
        assert!((p.specks.as_mm() - 2.0).abs() < 1e-12);
        assert!((p.tolerance.as_px() - 1.25).abs() < 1e-12);
        assert_eq!(
            (p.upscale, p.denoise),
            (TraceUpscale::Auto, TraceDenoise::Low)
        );
        assert!(p.invert && p.ignore_background);
        assert_eq!(trace_settings("nonsense"), TraceParams::with_threshold(128));
        let partial = trace_settings(r#"{"threshold":40}"#);
        assert_eq!(partial, TraceParams::with_threshold(40));
    }

    /// `specs/0089` criterion 14: the method and its fields cross; a missing tolerance is the
    /// method's default; unreadable colours are dropped.
    #[test]
    fn the_method_crosses_as_json() {
        let p = trace_settings(r##"{"method":1,"background":"#3366cc","colour_tolerance":30}"##);
        assert_eq!(
            p.selection,
            TraceMethod::RemoveBackground {
                background: Some(TraceRgb::new(0x33, 0x66, 0xCC)),
                tolerance: 30
            }
        );
        let p = trace_settings(r#"{"method":1,"background":null}"#);
        assert_eq!(
            p.selection,
            TraceMethod::RemoveBackground {
                background: None,
                tolerance: TRACE_DEFAULT_BACKGROUND_TOLERANCE
            }
        );
        let p = trace_settings(
            r##"{"method":2,"background":"#FFFFFF","swatches":["#FFF2A8","nonsense","D9D9D9"]}"##,
        );
        assert_eq!(
            p.selection,
            TraceMethod::PickColours {
                swatches: TraceSwatches::from_slice(&[
                    TraceRgb::new(0xFF, 0xF2, 0xA8),
                    TraceRgb::new(0xD9, 0xD9, 0xD9)
                ]),
                background: Some(TraceRgb::new(0xFF, 0xFF, 0xFF)),
                tolerance: TRACE_DEFAULT_PICK_TOLERANCE
            }
        );
        assert_eq!(
            trace_settings(r#"{"method":7}"#).selection,
            TraceMethod::Brightness
        );
        assert_eq!(hex(TraceRgb::new(1, 0xAB, 255)), "#01ABFF");
        assert_eq!(parse_hex("#12345"), None);
    }

    /// `specs/0089` slice M: Multicolour and its palette edits cross; Single colour is `None`.
    #[test]
    fn the_multicolour_settings_cross_as_json() {
        assert_eq!(trace_colour_settings(r#"{"threshold":90}"#), None);
        assert_eq!(trace_colour_settings(r#"{"output":0}"#), None);
        let p = trace_colour_settings(
            r##"{"output":1,"colours":40,"fixed":["#FF0000","x"],"excluded":["#00FF00"],
                "merged":[["#0000FF","#FF0000"],["bad"]],"threshold":90}"##,
        )
        .unwrap();
        assert_eq!(p.trace.threshold, 90);
        assert_eq!(p.colours, Some(16));
        assert_eq!(p.fixed, [TraceRgb::new(255, 0, 0)]);
        assert_eq!(p.excluded, [TraceRgb::new(0, 255, 0)]);
        assert_eq!(
            p.merged,
            [(TraceRgb::new(0, 0, 255), TraceRgb::new(255, 0, 0))]
        );
        let auto = trace_colour_settings(r#"{"output":1,"colours":null}"#).unwrap();
        assert_eq!(auto.colours, None);
        assert_eq!(
            trace_colour_settings(r#"{"output":1,"colours":1}"#)
                .unwrap()
                .colours,
            Some(2)
        );
    }

    #[test]
    fn the_presets_and_the_schema_cross_as_json() {
        let presets: serde_json::Value = serde_json::from_str(&trace_presets_json()).unwrap();
        assert_eq!(presets.as_array().unwrap().len(), 4);
        assert_eq!(presets[2]["values"]["smooth"], 100);
        assert_eq!(presets[2]["values"]["tolerance"], 1.5);
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
                "tolerance",
                "specks",
                "invert",
                "ignore_background",
                "denoise",
                "upscale",
                "corners",
                "method",
                "colour_tolerance"
            ]
        );
        assert_eq!(params[5]["label"], "Drop border shapes");
        assert_eq!(
            params[6]["choices"],
            serde_json::json!(["Off", "Low", "High"])
        );
    }
}
