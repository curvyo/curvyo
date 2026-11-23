//! The parameter schema and typed values (ADR 0034 §4) and their stored encoding (ADR 0033 §3).

use curvyo_document_core::Length;
use serde::Serialize;
use serde_json::{Map, Value};

/// The stored values of one fixture or manufacturing: key to encoded value, opaque to the
/// document model (ADR 0033 §3).
pub type StoredParams = Map<String, Value>;

/// The JSON key of a stored length: a number of millimetres tagged with its unit.
const MM: &str = "mm";
const DEG: &str = "deg";
const COUNT: &str = "count";
const ON: &str = "on";
const CHOICE: &str = "choice";

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
    /// The choice `key` is `choice`.
    Is {
        /// The choice's key.
        key: &'static str,
        /// The option id it must hold.
        choice: &'static str,
    },
    /// The length `key` is above zero.
    Positive {
        /// The length's key.
        key: &'static str,
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
            Self::Is { key, choice } => {
                matches!(value(key), Some(ParamValue::Choice(current)) if current == choice)
            }
            Self::Positive { key } => {
                matches!(value(key), Some(ParamValue::Length(length)) if length.as_mm() > 0.0)
            }
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
    },
    /// A whole number within `min..=max`.
    Count {
        /// The smallest value.
        min: u32,
        /// The largest value.
        max: u32,
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
}

impl ParamValue {
    /// The stored encoding: a one-key map naming the unit, `{"mm": n}`, `{"deg": n}`,
    /// `{"count": n}`, `{"on": b}` or `{"choice": "id"}` (ADR 0080 §2).
    #[must_use]
    pub fn to_stored(&self) -> Value {
        let (key, value) = match self {
            Self::Length(length) => (MM, Value::from(length.as_mm())),
            Self::Count(count) => (COUNT, Value::from(*count)),
            Self::Angle(degrees) => (DEG, Value::from(degrees.0)),
            Self::Switch(on) => (ON, Value::from(*on)),
            Self::Choice(id) => (CHOICE, Value::from(id.as_str())),
        };
        let mut map = Map::new();
        map.insert(key.to_owned(), value);
        Value::Object(map)
    }

    /// The value a stored encoding holds, `None` for any other shape or a number that is not
    /// finite.
    #[must_use]
    pub fn from_stored(value: &Value) -> Option<Self> {
        let Value::Object(map) = value else {
            return None;
        };
        if map.len() != 1 {
            return None;
        }
        let (key, inner) = map.iter().next()?;
        let finite = || inner.as_f64().filter(|number| number.is_finite());
        match key.as_str() {
            MM => Some(Self::Length(Length::from_mm(finite()?))),
            DEG => Some(Self::Angle(Degrees(finite()?))),
            COUNT => Some(Self::Count(u32::try_from(inner.as_u64()?).ok()?)),
            ON => Some(Self::Switch(inner.as_bool()?)),
            CHOICE => Some(Self::Choice(inner.as_str()?.to_owned())),
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

    /// The chosen option id this value holds, if it is a choice.
    #[must_use]
    pub fn choice(&self) -> Option<&str> {
        match self {
            Self::Choice(id) => Some(id),
            _ => None,
        }
    }
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
            (ParamKind::Count { min, max }, ParamValue::Count(count)) => {
                Some(ParamValue::Count(count.clamp(min, max)))
            }
            (ParamKind::Switch, switch @ ParamValue::Switch(_)) => Some(switch),
            (ParamKind::Choice { options }, ParamValue::Choice(id))
                if options.iter().any(|option| option.id == id) =>
            {
                Some(ParamValue::Choice(id))
            }
            _ => None,
        }
    }

    /// The stored values of a new object: every parameter at its default.
    #[must_use]
    pub fn defaults(params: &[Self]) -> StoredParams {
        params
            .iter()
            .map(|param| (param.key.to_owned(), param.default.to_stored()))
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
            },
            default: ParamValue::Length(Length::from_mm(10.0)),
            shown_when: None,
        }
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
            kind: ParamKind::Count { min: 6, max: 200 },
            default: ParamValue::Count(24),
            shown_when: None,
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
}
