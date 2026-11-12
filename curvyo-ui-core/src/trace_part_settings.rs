//! The stored trace settings of one part with their ranges and defaults (ADR 0061 §5): a stored
//! map read key by key, a value of the wrong type or out of range read as its default, unknown keys
//! ignored on read and kept on write.

use curvyo_document_core::TracePartSettings;
use serde_json::Value;

/// What a setting holds.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    /// A number within `min..=max`; `nullable` also takes `null` (Auto).
    Number { min: f64, max: f64, nullable: bool },
    /// On or off.
    Flag,
    /// One of the texts.
    Choice(&'static [&'static str]),
    /// `#rrggbb`, `""` (transparent) or `null` (not read yet).
    Background,
    /// A list of `#rrggbb`; `nullable` also takes `null`.
    Colours { nullable: bool },
    /// Any text (a preset key).
    Text,
}

/// One setting: its key, what it holds and its default.
struct Spec {
    key: &'static str,
    kind: Kind,
    default: fn() -> Value,
}

/// The range of the vectrace setting `name`, as the bar's field has it.
fn range(name: &str) -> (f64, f64) {
    vectrace::PARAMS
        .iter()
        .chain(vectrace::ADVANCED_PARAMS)
        .chain(vectrace::METHOD_PARAMS)
        .find(|spec| spec.name == name)
        .map_or((0.0, 0.0), |spec| (spec.min, spec.max))
}

fn number(name: &str, nullable: bool) -> Kind {
    let (min, max) = range(name);
    Kind::Number { min, max, nullable }
}

fn default_of(name: &str) -> Value {
    vectrace::PARAMS
        .iter()
        .chain(vectrace::ADVANCED_PARAMS)
        .find(|spec| spec.name == name)
        .map_or(Value::Null, |spec| Value::from(spec.default))
}

/// The settings of ADR 0061 §5 in its order.
fn specs() -> Vec<Spec> {
    let mut all = value_specs();
    all.extend(method_specs());
    all
}

/// The bar's and Advanced's values.
fn value_specs() -> Vec<Spec> {
    vec![
        Spec {
            key: "threshold",
            kind: number("threshold", true),
            default: || Value::Null,
        },
        Spec {
            key: "smooth",
            kind: number("smooth", false),
            default: || default_of("smooth"),
        },
        Spec {
            key: "tolerance",
            kind: number("tolerance", false),
            default: || default_of("tolerance"),
        },
        Spec {
            key: "specks",
            kind: number("specks", false),
            default: || default_of("specks"),
        },
        Spec {
            key: "corners",
            kind: number("corners", false),
            default: || default_of("corners"),
        },
        Spec {
            key: "upscale",
            kind: number("upscale", false),
            default: || default_of("upscale"),
        },
        Spec {
            key: "denoise",
            kind: number("denoise", false),
            default: || default_of("denoise"),
        },
        Spec {
            key: "invert",
            kind: Kind::Flag,
            default: || Value::Bool(false),
        },
        Spec {
            key: "ignore_background",
            kind: Kind::Flag,
            default: || Value::Bool(false),
        },
        Spec {
            key: "recognise_text",
            kind: Kind::Flag,
            default: || Value::Bool(false),
        },
        Spec {
            key: "recognise_shapes",
            kind: Kind::Flag,
            default: || Value::Bool(false),
        },
    ]
}

/// The method's, the colours' and the preset's.
fn method_specs() -> Vec<Spec> {
    let colours_max = f64::from(u8::try_from(vectrace::MAX_COLOURS).unwrap_or(u8::MAX));
    vec![
        Spec {
            key: "method",
            kind: Kind::Choice(&["brightness", "remove_background", "pick_colours"]),
            default: || Value::from("brightness"),
        },
        Spec {
            key: "background_tolerance",
            kind: number("colour_tolerance", false),
            default: || Value::from(vectrace::DEFAULT_BACKGROUND_TOLERANCE),
        },
        Spec {
            key: "pick_tolerance",
            kind: number("colour_tolerance", false),
            default: || Value::from(vectrace::DEFAULT_PICK_TOLERANCE),
        },
        Spec {
            key: "background_auto",
            kind: Kind::Flag,
            default: || Value::Bool(true),
        },
        Spec {
            key: "background",
            kind: Kind::Background,
            default: || Value::Null,
        },
        Spec {
            key: "swatches",
            kind: Kind::Colours { nullable: false },
            default: || Value::Array(Vec::new()),
        },
        Spec {
            key: "output",
            kind: Kind::Choice(&["single", "multi"]),
            default: || Value::from("single"),
        },
        Spec {
            key: "colours",
            kind: Kind::Number {
                min: 2.0,
                max: colours_max,
                nullable: true,
            },
            default: || Value::Null,
        },
        Spec {
            key: "fixed",
            kind: Kind::Colours { nullable: false },
            default: || Value::Array(Vec::new()),
        },
        Spec {
            key: "excluded",
            kind: Kind::Colours { nullable: false },
            default: || Value::Array(Vec::new()),
        },
        Spec {
            key: "kept_palette",
            kind: Kind::Colours { nullable: true },
            default: || Value::Null,
        },
        Spec {
            key: "preset",
            kind: Kind::Text,
            default: || Value::from(""),
        },
        Spec {
            key: "preset_edited",
            kind: Kind::Flag,
            default: || Value::Bool(false),
        },
    ]
}

fn is_hex(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Whether `value` is a valid value of `kind`.
fn accepts(kind: Kind, value: &Value) -> bool {
    match (kind, value) {
        (Kind::Number { nullable, .. } | Kind::Colours { nullable, .. }, Value::Null) => nullable,
        (Kind::Number { min, max, .. }, Value::Number(n)) => n
            .as_f64()
            .is_some_and(|n| n.is_finite() && (min..=max).contains(&n)),
        (Kind::Flag, Value::Bool(_))
        | (Kind::Text, Value::String(_))
        | (Kind::Background, Value::Null) => true,
        (Kind::Choice(choices), Value::String(text)) => choices.contains(&text.as_str()),
        (Kind::Background, Value::String(text)) => text.is_empty() || is_hex(text),
        (Kind::Colours { .. }, Value::Array(items)) => {
            items.iter().all(|item| item.as_str().is_some_and(is_hex))
        }
        _ => false,
    }
}

/// Every setting of a part: the stored value where it is valid, else its default. Unknown keys
/// are not read.
#[must_use]
pub fn read_part_settings(stored: &TracePartSettings) -> TracePartSettings {
    specs()
        .into_iter()
        .map(|spec| {
            let value = stored
                .get(spec.key)
                .filter(|value| accepts(spec.kind, value))
                .cloned()
                .unwrap_or_else(spec.default);
            (spec.key.to_owned(), value)
        })
        .collect()
}

/// The map a settings gesture writes: the valid known values of `values` over `stored`, every
/// other stored key (a newer build's) kept.
#[must_use]
pub fn written_part_settings(
    stored: &TracePartSettings,
    values: &TracePartSettings,
) -> TracePartSettings {
    let mut out = stored.clone();
    for spec in specs() {
        if let Some(value) = values.get(spec.key).filter(|v| accepts(spec.kind, v)) {
            out.insert(spec.key.to_owned(), value.clone());
        }
    }
    out
}

/// `settings` without the picked colours: what Add part gives the new part (`0094` criterion 19,
/// customer 2026-11-10).
#[must_use]
pub fn without_picked_colours(settings: &TracePartSettings) -> TracePartSettings {
    let mut out = settings.clone();
    for key in ["swatches", "fixed", "excluded"] {
        out.insert(key.to_owned(), Value::Array(Vec::new()));
    }
    out.insert("kept_palette".to_owned(), Value::Null);
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn map(value: Value) -> TracePartSettings {
        serde_json::from_value(value).unwrap()
    }

    /// ADR 0061 §5: a wrong type or a value out of range reads as the default; unknown keys are
    /// not read.
    #[test]
    fn a_damaged_value_reads_as_its_default() {
        let read = read_part_settings(&map(json!({
            "threshold": 300, "smooth": 40, "tolerance": "big", "method": "magic",
            "background": "#12345", "swatches": ["#ff0000"], "colours": 1, "future": 1,
        })));
        assert_eq!(read["threshold"], Value::Null, "out of range: Auto");
        assert_eq!(read["smooth"], json!(40));
        assert_eq!(read["tolerance"], default_of("tolerance"));
        assert_eq!(read["method"], json!("brightness"));
        assert_eq!(read["background"], Value::Null);
        assert_eq!(read["swatches"], json!(["#ff0000"]));
        assert_eq!(read["colours"], Value::Null);
        assert!(!read.contains_key("future"));
        assert_eq!(read.len(), 24);
        assert_eq!(
            read_part_settings(&TracePartSettings::new())["smooth"],
            json!(80.0)
        );
    }

    /// ADR 0061 §5: a write keeps a newer build's keys and refuses invalid values.
    #[test]
    fn a_write_keeps_unknown_keys_and_drops_invalid_values() {
        let stored = map(json!({"smooth": 10, "future": "kept"}));
        let written = written_part_settings(&stored, &map(json!({"smooth": 20, "corners": 900})));
        assert_eq!(written, map(json!({"smooth": 20, "future": "kept"})));
    }

    #[test]
    fn a_new_part_starts_without_picked_colours() {
        let out = without_picked_colours(&map(json!({"swatches": ["#ff0000"], "smooth": 3})));
        assert_eq!(out["swatches"], json!([]));
        assert_eq!(out["smooth"], json!(3));
        assert_eq!(out["kept_palette"], Value::Null);
    }
}
