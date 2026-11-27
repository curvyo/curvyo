//! The parameter schema and typed values (ADR 0034 §4) and their stored encoding (ADR 0033 §3).

use crate::extension::ClosedPath;
use curvyo_document_core::{FontKind, FontRef, Length};
use serde::Serialize;
use serde_json::{Map, Value};

/// The stored values of one fixture or manufacturing: key to encoded value, opaque to the
/// document model (ADR 0033 §3).
pub type StoredParams = Map<String, Value>;

/// The JSON key of a stored length: a number of millimetres tagged with its unit.
const COUNT: &str = "count";
const ON: &str = "on";
const CHOICE: &str = "choice";
const PATH: &str = "path";
const TEXT: &str = "text";
const PCT: &str = "pct";
const RATIO: &str = "ratio";
const FONT: &str = "font";
/// The most bytes a stored text may hold (a QR code at version 40 takes 2,953); longer text is
/// cut at a character boundary by [`Param::accept`].
const MAX_TEXT_BYTES: usize = 8192;
/// The most cubics a stored outline may hold, and the largest coordinate it may use (mm).
const MAX_PATH_CUBICS: usize = 2000;
const MAX_PATH_MM: f64 = 1e7;

/// One field of a schema-rendered block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Param {
    /// The key under which the value is stored.
    pub key: &'static str,
    /// The field label ("Spacing").
    pub label: &'static str,
    /// The tooltip text.
    pub tooltip: &'static str,
    /// What kind of value it holds, with its bounds.
    pub kind: ParamKind,
    /// The value of a new object and the value an invalid stored one reads as.
    pub default: ParamValue,
    /// The row is drawn only while this holds (UX notes of `0141` §3); `None` = always.
    pub shown_when: Option<ShownWhen>,
    /// The span of the slider track a number is drawn on (UX notes of `0141` §15.3); `None` = the
    /// whole range, linear.
    pub track: Option<Track>,
    /// The value a **new** object is written with when it differs from `default` (ADR 0089
    /// decision 2): a stored object without the key still reads `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<ParamValue>,
}

/// How a slider maps its position to a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    /// `v = from + (to - from) p`.
    Linear,
    /// Width's curve: `v = from + (to - from) (100^p - 1) / 99`.
    Log,
}

/// The track of a number's slider: positions 0 to 1 are the values `from` to `to`, a drag past
/// either end goes on up to the parameter's own bounds (the overshoot rule).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Track {
    /// The value at the left end.
    pub from: f64,
    /// The value at the right end.
    pub to: f64,
    /// The mapping between the ends.
    pub curve: Curve,
}

/// When a parameter row is drawn: a condition on another parameter's stored value (hidden, not
/// disabled; the hidden value is kept).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShownWhen {
    /// The switch `key` is on.
    On {
        /// The switch's key.
        key: &'static str,
    },
    /// Every one of the switches `keys` is on (a row that needs two switches, as Ring thickness
    /// needs Lid and Lid seat).
    AllOn {
        /// The switches' keys.
        keys: &'static [&'static str],
    },
    /// The choice `key` is `choice`.
    Is {
        /// The choice's key.
        key: &'static str,
        /// The option id it must hold.
        choice: &'static str,
    },
    /// The percentage `key` is at its maximum (a Module fill of 100 %).
    Full {
        /// The percentage's key.
        key: &'static str,
    },
    /// The length `key` is above zero.
    Positive {
        /// The length's key.
        key: &'static str,
    },
    /// Every one of the `conditions` holds (a row that hangs on a switch and a choice at once).
    All {
        /// The conditions, all of which must hold.
        conditions: &'static [ShownWhen],
    },
    /// At least one of the `conditions` holds.
    Any {
        /// The conditions, one of which must hold.
        conditions: &'static [ShownWhen],
    },
}

impl ShownWhen {
    /// Whether the condition holds for `params` (a parameter of the schema `schema` that is missing
    /// reads as its default).
    #[must_use]
    pub fn holds(self, schema: &[Param], params: &StoredParams) -> bool {
        let value = |key: &str| {
            schema
                .iter()
                .find(|param| param.key == key)
                .map(|param| param.read(params))
        };
        match self {
            Self::On { key } => matches!(value(key), Some(ParamValue::Switch(true))),
            Self::AllOn { keys } => keys
                .iter()
                .all(|key| matches!(value(key), Some(ParamValue::Switch(true)))),
            Self::Is { key, choice } => {
                matches!(value(key), Some(ParamValue::Choice(current)) if current == choice)
            }
            Self::Full { key } => {
                let full = schema
                    .iter()
                    .find(|param| param.key == key)
                    .and_then(|param| match (param.kind, param.read(params)) {
                        (ParamKind::Percent { max, .. }, ParamValue::Percent(now)) => {
                            Some(now == max)
                        }
                        _ => None,
                    });
                full == Some(true)
            }
            Self::Positive { key } => {
                matches!(value(key), Some(ParamValue::Length(length)) if length.as_mm() > 0.0)
            }
            Self::All { conditions } => conditions
                .iter()
                .all(|condition| condition.holds(schema, params)),
            Self::Any { conditions } => conditions
                .iter()
                .any(|condition| condition.holds(schema, params)),
        }
    }
}

/// An angle in degrees, the unit of a stored angle parameter (`{"deg": n}`).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct Degrees(pub f64);

/// One named option of a choice parameter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ChoiceOption {
    /// The stored id.
    pub id: &'static str,
    /// The visible name.
    pub label: &'static str,
}

/// The kind of a parameter. Only the kinds a story uses exist (ADR 0034 §4, ADR 0080 §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParamKind {
    /// A length in mm within `min..=max`.
    Length {
        /// The smallest value.
        min: Length,
        /// The largest value.
        max: Length,
        /// One arrow-key step in mm (Shift x10, Ctrl x0.1).
        step: f64,
        /// The decimals shown, trailing zeros trimmed.
        decimals: u8,
        /// A stored 0 means automatic: the generator derives the value and reports the one it
        /// used (`Generated::automatic`); the schema minimum applies to every other value.
        automatic: bool,
    },
    /// A whole number within `min..=max`.
    Count {
        /// The smallest value.
        min: u32,
        /// The largest value.
        max: u32,
        /// A stored 0 means automatic, as for a length.
        automatic: bool,
    },
    /// An angle in degrees within `min..=max`.
    Angle {
        /// The smallest value.
        min: Degrees,
        /// The largest value.
        max: Degrees,
        /// One arrow-key step in degrees.
        step: f64,
        /// The decimals shown, trailing zeros trimmed.
        decimals: u8,
    },
    /// An on or off value.
    Switch,
    /// One of two to eight named options.
    Choice {
        /// The options, in order.
        options: &'static [ChoiceOption],
    },
    /// One closed outline in frame-local mm (ADR 0083 §2); the block draws no field for it.
    Path,
    /// A text of any length up to 8,192 bytes (ADR 0091 §3), one line or several.
    Text {
        /// Whether the field takes line breaks (Enter inserts one).
        multiline: bool,
    },
    /// A whole percentage within `min..=max`, signed (ADR 0091 §3, ADR 0092 §2: Bar width adjust
    /// runs from -30 to +30).
    Percent {
        /// The smallest value.
        min: i32,
        /// The largest value.
        max: i32,
    },
    /// A unitless number within `min..=max`, shown with its decimals (Wide:narrow, ADR 0092 §2).
    Ratio {
        /// The smallest value.
        min: f64,
        /// The largest value.
        max: f64,
        /// One arrow-key step (Shift x10, Ctrl x0.1).
        step: f64,
        /// The decimals shown, trailing zeros trimmed.
        decimals: u8,
    },
    /// An outline face of the maker's fonts, `None` for the included sans (ADR 0092 §3).
    Font,
}

/// A typed parameter value.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ParamValue {
    /// A length.
    Length(Length),
    /// A whole number.
    Count(u32),
    /// An angle in degrees.
    Angle(Degrees),
    /// On or off.
    Switch(bool),
    /// The id of the chosen option.
    Choice(String),
    /// One closed outline, as `{"path": [x0, y0, c1x, c1y, c2x, c2y, x, y, ...]}`.
    Path(ClosedPath),
    /// A text, as `{"text": "..."}`.
    Text(String),
    /// A whole percentage, as `{"pct": n}`.
    Percent(i32),
    /// A unitless number, as `{"ratio": x}`.
    Ratio(f64),
    /// A font face, as `{"font": {"family", "style", "face_hash", "face_index"}}` or `{"font":
    /// null}` for the included sans.
    Font(Option<FontRef>),
}

impl ParamValue {
    /// The stored encoding: a one-key map naming the unit, `{"mm": n}`, `{"deg": n}`,
    /// `{"count": n}`, `{"on": b}` or `{"choice": "id"}` (ADR 0080 §2).
    #[must_use]
    pub fn to_stored(&self) -> Value {
        let (key, value) = match self {
            Self::Length(length) => {
                return curvyo_document_core::encode_number(
                    curvyo_document_core::StoredUnit::Millimetres,
                    length.as_mm(),
                );
            }
            Self::Angle(degrees) => {
                return curvyo_document_core::encode_number(
                    curvyo_document_core::StoredUnit::Degrees,
                    degrees.0,
                );
            }
            Self::Count(count) => (COUNT, Value::from(*count)),
            Self::Switch(on) => (ON, Value::from(*on)),
            Self::Choice(id) => (CHOICE, Value::from(id.as_str())),
            Self::Path(path) => (PATH, Value::from(path.flat())),
            Self::Text(text) => (TEXT, Value::from(text.as_str())),
            Self::Percent(percent) => (PCT, Value::from(*percent)),
            Self::Ratio(ratio) => (RATIO, Value::from(*ratio)),
            Self::Font(face) => (FONT, face.as_ref().map_or(Value::Null, font_to_stored)),
        };
        let mut map = Map::new();
        map.insert(key.to_owned(), value);
        Value::Object(map)
    }

    /// The value a stored encoding holds, `None` for any other shape or a number that is not
    /// finite.
    #[must_use]
    pub fn from_stored(value: &Value) -> Option<Self> {
        if let Some((unit, number)) = curvyo_document_core::decode_number(value) {
            return Some(match unit {
                curvyo_document_core::StoredUnit::Millimetres => {
                    Self::Length(Length::from_mm(number))
                }
                curvyo_document_core::StoredUnit::Degrees => Self::Angle(Degrees(number)),
            });
        }
        let Value::Object(map) = value else {
            return None;
        };
        if map.len() != 1 {
            return None;
        }
        let (key, inner) = map.iter().next()?;
        match key.as_str() {
            COUNT => Some(Self::Count(u32::try_from(inner.as_u64()?).ok()?)),
            ON => Some(Self::Switch(inner.as_bool()?)),
            CHOICE => Some(Self::Choice(inner.as_str()?.to_owned())),
            TEXT => Some(Self::Text(inner.as_str()?.to_owned())),
            PCT => Some(Self::Percent(i32::try_from(inner.as_i64()?).ok()?)),
            RATIO => inner.as_f64().filter(|n| n.is_finite()).map(Self::Ratio),
            FONT => match inner {
                Value::Null => Some(Self::Font(None)),
                other => font_from_stored(other).map(|face| Self::Font(Some(face))),
            },
            PATH => {
                let numbers: Option<Vec<f64>> = inner
                    .as_array()?
                    .iter()
                    .map(|number| number.as_f64().filter(|n| n.is_finite()))
                    .collect();
                Some(Self::Path(ClosedPath::from_flat(&numbers?)?))
            }
            _ => None,
        }
    }

    /// The length this value holds, if it is one.
    #[must_use]
    pub const fn length(&self) -> Option<Length> {
        match self {
            Self::Length(length) => Some(*length),
            _ => None,
        }
    }

    /// The whole number this value holds, if it is one.
    #[must_use]
    pub const fn count(&self) -> Option<u32> {
        match self {
            Self::Count(count) => Some(*count),
            _ => None,
        }
    }

    /// The angle in degrees this value holds, if it is one.
    #[must_use]
    pub const fn degrees(&self) -> Option<f64> {
        match self {
            Self::Angle(degrees) => Some(degrees.0),
            _ => None,
        }
    }

    /// The switch this value holds, if it is one.
    #[must_use]
    pub const fn switch(&self) -> Option<bool> {
        match self {
            Self::Switch(on) => Some(*on),
            _ => None,
        }
    }

    /// The outline this value holds, if it is one.
    #[must_use]
    pub const fn path(&self) -> Option<&ClosedPath> {
        match self {
            Self::Path(path) => Some(path),
            _ => None,
        }
    }

    /// The text this value holds, if it is one.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The percentage this value holds, if it is one.
    #[must_use]
    pub const fn percent(&self) -> Option<i32> {
        match self {
            Self::Percent(percent) => Some(*percent),
            _ => None,
        }
    }

    /// The unitless number this value holds, if it is one.
    #[must_use]
    pub const fn ratio(&self) -> Option<f64> {
        match self {
            Self::Ratio(ratio) => Some(*ratio),
            _ => None,
        }
    }

    /// The face this value names (`None` inside: the included sans), if it is a font value.
    #[must_use]
    pub const fn font(&self) -> Option<Option<&FontRef>> {
        match self {
            Self::Font(face) => Some(face.as_ref()),
            _ => None,
        }
    }

    /// The chosen option id this value holds, if it is a choice.
    #[must_use]
    pub fn choice(&self) -> Option<&str> {
        match self {
            Self::Choice(id) => Some(id),
            _ => None,
        }
    }
}

/// The stored form of a face: the four fields of a text's `FontRef` (an outline face).
fn font_to_stored(face: &FontRef) -> Value {
    let mut map = Map::new();
    map.insert("family".to_owned(), Value::from(face.family.as_str()));
    map.insert("style".to_owned(), Value::from(face.style.as_str()));
    map.insert("face_hash".to_owned(), Value::from(face.face_hash.as_str()));
    map.insert("face_index".to_owned(), Value::from(face.face_index));
    Value::Object(map)
}

/// The face a stored `{"family", "style", "face_hash", "face_index"}` map names.
fn font_from_stored(value: &Value) -> Option<FontRef> {
    let map = value.as_object()?;
    let text = |key: &str| map.get(key)?.as_str().map(str::to_owned);
    Some(FontRef {
        family: text("family")?,
        style: text("style")?,
        face_hash: text("face_hash")?,
        face_index: u32::try_from(map.get("face_index")?.as_u64()?).ok()?,
        kind: FontKind::Outline,
    })
}

/// `text` cut to [`MAX_TEXT_BYTES`] at a character boundary.
fn cut_text(mut text: String) -> String {
    if text.len() > MAX_TEXT_BYTES {
        let mut end = MAX_TEXT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// The shape check of an outline (ADR 0083 §2): at least two cubics, at most 2,000, every number
/// finite within 1e7 mm. Whether the outline can be built is the generator's check.
fn path_is_usable(path: &ClosedPath) -> bool {
    let finite = |x: f64, y: f64| x.abs() <= MAX_PATH_MM && y.abs() <= MAX_PATH_MM;
    (2..=MAX_PATH_CUBICS).contains(&path.segments.len())
        && finite(path.start.x, path.start.y)
        && path.segments.iter().all(|cubic| {
            [cubic.c1, cubic.c2, cubic.to]
                .iter()
                .all(|point| finite(point.x, point.y))
        })
}

impl Param {
    /// The value `stored` holds for this parameter; a missing, mistyped or out-of-range value
    /// reads as the default and is never rewritten on read (ADR 0033 §3).
    #[must_use]
    pub fn read(&self, stored: &StoredParams) -> ParamValue {
        stored
            .get(self.key)
            .and_then(ParamValue::from_stored)
            .filter(|value| self.accept(value.clone()).as_ref() == Some(value))
            .unwrap_or_else(|| self.default.clone())
    }

    /// An edited value as it is written: a number clamped into the bounds (UX notes §4, permit
    /// within bounds), a count rounded half up first; `None` for a value of the wrong kind, a
    /// non-finite number or a choice that is not an option.
    #[must_use]
    pub fn accept(&self, value: ParamValue) -> Option<ParamValue> {
        match (self.kind, value) {
            (
                ParamKind::Length {
                    automatic: true, ..
                },
                ParamValue::Length(length),
            ) if length.as_mm() == 0.0 => Some(ParamValue::Length(length)),
            (ParamKind::Length { min, max, .. }, ParamValue::Length(length))
                if length.as_mm().is_finite() =>
            {
                Some(ParamValue::Length(Length::from_mm(
                    length.as_mm().clamp(min.as_mm(), max.as_mm()),
                )))
            }
            (ParamKind::Angle { min, max, .. }, ParamValue::Angle(degrees))
                if degrees.0.is_finite() =>
            {
                Some(ParamValue::Angle(Degrees(degrees.0.clamp(min.0, max.0))))
            }
            (
                ParamKind::Count {
                    automatic: true, ..
                },
                ParamValue::Count(0),
            ) => Some(ParamValue::Count(0)),
            (ParamKind::Count { min, max, .. }, ParamValue::Count(count)) => {
                Some(ParamValue::Count(count.clamp(min, max)))
            }
            (ParamKind::Switch, switch @ ParamValue::Switch(_)) => Some(switch),
            (ParamKind::Choice { options }, ParamValue::Choice(id))
                if options.iter().any(|option| option.id == id) =>
            {
                Some(ParamValue::Choice(id))
            }
            (ParamKind::Path, ParamValue::Path(path)) if path_is_usable(&path) => {
                Some(ParamValue::Path(path))
            }
            (ParamKind::Percent { min, max }, ParamValue::Percent(percent)) => {
                Some(ParamValue::Percent(percent.clamp(min, max)))
            }
            (ParamKind::Ratio { min, max, .. }, ParamValue::Ratio(ratio)) if ratio.is_finite() => {
                Some(ParamValue::Ratio(ratio.clamp(min, max)))
            }
            (ParamKind::Font, font @ ParamValue::Font(_)) => Some(font),
            (ParamKind::Text { .. }, ParamValue::Text(text)) => {
                Some(ParamValue::Text(cut_text(text)))
            }
            _ => None,
        }
    }

    /// The stored values of a new object: every parameter at its default.
    #[must_use]
    pub fn defaults(params: &[Self]) -> StoredParams {
        params
            .iter()
            .map(|param| {
                let value = param.initial.as_ref().unwrap_or(&param.default);
                (param.key.to_owned(), value.to_stored())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spacing() -> Param {
        Param {
            key: "spacing",
            label: "Spacing",
            tooltip: "Spacing",
            kind: ParamKind::Length {
                min: Length::from_mm(0.5),
                max: Length::from_mm(1000.0),
                step: 1.0,
                decimals: 2,
                automatic: false,
            },
            default: ParamValue::Length(Length::from_mm(10.0)),
            shown_when: None,
            track: None,
            initial: None,
        }
    }

    #[test]
    fn a_row_shown_when_all_its_switches_are_on() {
        let switches = [
            Param {
                key: "a",
                label: "A",
                tooltip: "A",
                kind: ParamKind::Switch,
                default: ParamValue::Switch(true),
                shown_when: None,
                track: None,
                initial: None,
            },
            Param {
                key: "b",
                label: "B",
                tooltip: "B",
                kind: ParamKind::Switch,
                default: ParamValue::Switch(true),
                shown_when: None,
                track: None,
                initial: None,
            },
        ];
        let both = ShownWhen::AllOn { keys: &["a", "b"] };
        let mut stored = StoredParams::new();
        assert!(both.holds(&switches, &stored), "defaults are on");
        stored.insert("a".into(), ParamValue::Switch(false).to_stored());
        assert!(!both.holds(&switches, &stored));
    }

    #[test]
    fn all_and_any_combine_conditions_and_a_missing_key_is_false() {
        let switch = |key: &'static str, on: bool| Param {
            key,
            label: key,
            tooltip: key,
            kind: ParamKind::Switch,
            default: ParamValue::Switch(on),
            shown_when: None,
            track: None,
            initial: None,
        };
        let schema = [switch("a", true), switch("b", false)];
        let (a, b) = (ShownWhen::On { key: "a" }, ShownWhen::On { key: "b" });
        let none = StoredParams::new();
        for (conditions, all, any) in [
            (&[a, b][..], false, true),
            (&[a, a][..], true, true),
            (&[b, b][..], false, false),
            (&[a, ShownWhen::On { key: "missing" }][..], false, true),
        ] {
            // The slices are leaked only in this test, to get the `'static` the variants need.
            let conditions: &'static [ShownWhen] =
                Box::leak(conditions.to_vec().into_boxed_slice());
            assert_eq!(ShownWhen::All { conditions }.holds(&schema, &none), all);
            assert_eq!(ShownWhen::Any { conditions }.holds(&schema, &none), any);
        }
    }

    #[test]
    fn an_automatic_zero_passes_accept_and_any_other_value_keeps_the_minimum() {
        let mut param = spacing();
        param.kind = ParamKind::Length {
            min: Length::from_mm(0.5),
            max: Length::from_mm(1000.0),
            step: 1.0,
            decimals: 2,
            automatic: true,
        };
        let zero = ParamValue::Length(Length::from_mm(0.0));
        assert_eq!(param.accept(zero.clone()), Some(zero.clone()));
        assert_eq!(
            param.accept(ParamValue::Length(Length::from_mm(0.1))),
            Some(ParamValue::Length(Length::from_mm(0.5))),
            "a typed value below the minimum is clamped"
        );
        let mut stored = StoredParams::new();
        stored.insert("spacing".into(), zero.to_stored());
        assert_eq!(param.read(&stored), zero, "a stored 0 reads as automatic");
        let count = ParamKind::Count {
            min: 1,
            max: 10,
            automatic: true,
        };
        param.kind = count;
        assert_eq!(
            param.accept(ParamValue::Count(0)),
            Some(ParamValue::Count(0))
        );
        assert_eq!(
            param.accept(ParamValue::Count(99)),
            Some(ParamValue::Count(10))
        );
    }

    #[test]
    fn a_track_is_part_of_the_serialised_row() {
        let mut param = spacing();
        param.track = Some(Track {
            from: 1.0,
            to: 100.0,
            curve: Curve::Log,
        });
        let json = serde_json::to_value(&param).unwrap();
        assert_eq!(
            json["track"],
            serde_json::json!({"from": 1.0, "to": 100.0, "curve": "log"})
        );
        assert!(serde_json::to_value(spacing()).unwrap()["track"].is_null());
    }

    #[test]
    fn a_length_is_stored_as_millimetres_tagged_with_the_unit() {
        let value = ParamValue::Length(Length::from_mm(2.5));
        assert_eq!(value.to_stored(), serde_json::json!({"mm": 2.5}));
        assert_eq!(ParamValue::from_stored(&value.to_stored()), Some(value));
    }

    #[test]
    fn an_invalid_stored_value_reads_as_the_default() {
        let param = spacing();
        let mut stored = StoredParams::new();
        assert_eq!(param.read(&stored), param.default);
        stored.insert("spacing".into(), serde_json::json!({"mm": 0.1}));
        assert_eq!(param.read(&stored), param.default, "below the minimum");
        stored.insert("spacing".into(), serde_json::json!("ten"));
        assert_eq!(param.read(&stored), param.default, "wrong kind");
        stored.insert("spacing".into(), serde_json::json!({"mm": 4.0}));
        assert_eq!(
            param.read(&stored),
            ParamValue::Length(Length::from_mm(4.0))
        );
    }

    #[test]
    fn an_edit_is_clamped_into_the_bounds_and_a_wrong_kind_is_refused() {
        let param = spacing();
        assert_eq!(
            param.accept(ParamValue::Length(Length::from_mm(0.1))),
            Some(ParamValue::Length(Length::from_mm(0.5)))
        );
        assert_eq!(
            param.accept(ParamValue::Length(Length::from_mm(f64::NAN))),
            None
        );
    }

    #[test]
    fn every_kind_round_trips_and_clamps() {
        let count = Param {
            key: "teeth",
            label: "Teeth",
            tooltip: "Teeth",
            kind: ParamKind::Count {
                min: 6,
                max: 200,
                automatic: false,
            },
            default: ParamValue::Count(24),
            shown_when: None,
            track: None,
            initial: None,
        };
        assert_eq!(
            count.accept(ParamValue::Count(3)),
            Some(ParamValue::Count(6))
        );
        for value in [
            ParamValue::Count(7),
            ParamValue::Angle(Degrees(20.0)),
            ParamValue::Switch(true),
            ParamValue::Choice("spokes".into()),
        ] {
            assert_eq!(ParamValue::from_stored(&value.to_stored()), Some(value));
        }
        let choice = Param {
            key: "web",
            label: "Web",
            tooltip: "Web",
            kind: ParamKind::Choice {
                options: &[
                    ChoiceOption {
                        id: "solid",
                        label: "Solid",
                    },
                    ChoiceOption {
                        id: "spokes",
                        label: "Spokes",
                    },
                ],
            },
            default: ParamValue::Choice("solid".into()),
            shown_when: None,
            track: None,
            initial: None,
        };
        assert_eq!(choice.accept(ParamValue::Choice("lattice".into())), None);
        assert!(
            !ShownWhen::Is {
                key: "web",
                choice: "spokes"
            }
            .holds(std::slice::from_ref(&choice), &StoredParams::new())
        );
    }

    #[test]
    fn text_and_percent_round_trip_clamp_and_cut() {
        let text = Param {
            key: "content",
            label: "Content",
            tooltip: "Content",
            kind: ParamKind::Text { multiline: true },
            default: ParamValue::Text("a".into()),
            shown_when: None,
            track: None,
            initial: None,
        };
        let percent = Param {
            key: "fill",
            label: "Fill",
            tooltip: "Fill",
            kind: ParamKind::Percent { min: 50, max: 100 },
            default: ParamValue::Percent(100),
            shown_when: None,
            track: None,
            initial: None,
        };
        for value in [
            ParamValue::Text("two\nlines \u{e4}".into()),
            ParamValue::Percent(70),
        ] {
            assert_eq!(ParamValue::from_stored(&value.to_stored()), Some(value));
        }
        assert_eq!(
            ParamValue::Text("x".into()).to_stored(),
            serde_json::json!({"text": "x"})
        );
        assert_eq!(
            ParamValue::Percent(7).to_stored(),
            serde_json::json!({"pct": 7})
        );
        assert_eq!(
            percent.accept(ParamValue::Percent(10)),
            Some(ParamValue::Percent(50))
        );
        assert_eq!(percent.accept(ParamValue::Text("1".into())), None);
        let long = "\u{e4}".repeat(5000);
        let kept = text.accept(ParamValue::Text(long)).unwrap();
        assert!(kept.text().unwrap().len() <= 8192);
        assert_ne!(kept.text().unwrap().len(), 0);
        let schema = [percent];
        let mut stored = StoredParams::new();
        assert!(
            ShownWhen::Full { key: "fill" }.holds(&schema, &stored),
            "default is full"
        );
        stored.insert("fill".into(), ParamValue::Percent(90).to_stored());
        assert!(!ShownWhen::Full { key: "fill" }.holds(&schema, &stored));
    }

    #[test]
    fn a_signed_percent_a_ratio_and_a_font_round_trip_through_the_stored_form() {
        let adjust = ParamKind::Percent { min: -30, max: 30 };
        let minus = ParamValue::Percent(-5);
        assert_eq!(minus.to_stored(), serde_json::json!({"pct": -5}));
        assert_eq!(ParamValue::from_stored(&minus.to_stored()), Some(minus));
        let param = Param {
            kind: adjust,
            default: ParamValue::Percent(0),
            ..spacing()
        };
        assert_eq!(
            param.accept(ParamValue::Percent(-90)),
            Some(ParamValue::Percent(-30))
        );
        let ratio = Param {
            kind: ParamKind::Ratio {
                min: 2.0,
                max: 3.0,
                step: 0.1,
                decimals: 1,
            },
            default: ParamValue::Ratio(3.0),
            ..spacing()
        };
        assert_eq!(
            ratio.accept(ParamValue::Ratio(7.5)),
            Some(ParamValue::Ratio(3.0))
        );
        assert_eq!(ratio.accept(ParamValue::Ratio(f64::NAN)), None);
        assert_eq!(
            ParamValue::from_stored(&serde_json::json!({"ratio": 2.5})),
            Some(ParamValue::Ratio(2.5))
        );
        let none = ParamValue::Font(None);
        assert_eq!(none.to_stored(), serde_json::json!({"font": null}));
        assert_eq!(ParamValue::from_stored(&none.to_stored()), Some(none));
        let face = ParamValue::Font(Some(curvyo_document_core::FontRef {
            family: "Source Sans 3".into(),
            style: "Regular".into(),
            face_hash: "ab".repeat(32),
            face_index: 0,
            kind: curvyo_document_core::FontKind::Outline,
        }));
        assert_eq!(ParamValue::from_stored(&face.to_stored()), Some(face));
        assert_eq!(
            ParamValue::from_stored(&serde_json::json!({"font": {"family": "x"}})),
            None,
            "a face without its hash is no face"
        );
    }

    fn square(side: f64) -> ClosedPath {
        use crate::extension::Cubic;
        use curvyo_document_core::Point;
        let corners = [(side, 0.0), (side, side), (0.0, side), (0.0, 0.0)];
        ClosedPath {
            start: Point::new(0.0, 0.0),
            segments: corners
                .iter()
                .map(|&(x, y)| Cubic {
                    c1: Point::new(x, y),
                    c2: Point::new(x, y),
                    to: Point::new(x, y),
                })
                .collect(),
        }
    }

    fn outline() -> Param {
        Param {
            key: "outline",
            label: "Outline",
            tooltip: "Outline",
            kind: ParamKind::Path,
            default: ParamValue::Path(square(10.0)),
            shown_when: None,
            track: None,
            initial: None,
        }
    }

    #[test]
    fn a_path_is_stored_as_the_flat_numbers_and_reads_back_unchanged() {
        let value = ParamValue::Path(square(20.0));
        let stored = value.to_stored();
        assert_eq!(stored["path"].as_array().map(Vec::len), Some(2 + 6 * 4));
        assert_eq!(ParamValue::from_stored(&stored), Some(value.clone()));
        let mut map = StoredParams::new();
        map.insert("outline".into(), stored);
        assert_eq!(outline().read(&map), value, "read compares accept(v) == v");
    }

    #[test]
    fn a_path_of_the_wrong_shape_reads_as_the_default() {
        let mut map = StoredParams::new();
        for bad in [
            serde_json::json!({"path": [0.0, 0.0, 1.0]}),
            serde_json::json!({"path": [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 5.0, 5.0]}),
            serde_json::json!({"path": "x"}),
            serde_json::json!({"path": [0.0, 0.0, 1.0e9, 1.0, 2.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]}),
        ] {
            map.insert("outline".into(), bad);
            assert_eq!(outline().read(&map), outline().default);
        }
        let many = ClosedPath {
            segments: vec![square(1.0).segments[0]; 2001],
            ..square(1.0)
        };
        assert_eq!(outline().accept(ParamValue::Path(many)), None);
    }
}
