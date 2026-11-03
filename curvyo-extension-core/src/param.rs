//! The parameter schema and typed values (ADR 0034 §4) and their stored encoding (ADR 0033 §3).

use curvyo_document_core::Length;
use serde::Serialize;
use serde_json::{Map, Value};

/// The stored values of one fixture or manufacturing: key to encoded value, opaque to the
/// document model (ADR 0033 §3).
pub type StoredParams = Map<String, Value>;

/// The JSON key of a stored length: a number of millimetres tagged with its unit.
const MM: &str = "mm";

/// One field of a schema-rendered block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Param {
    /// The key under which the value is stored; a [`HostKey`] spelling binds a value the host
    /// owns (the machine name, the work area) instead.
    pub key: &'static str,
    /// The field label ("Spacing").
    pub label: &'static str,
    /// The tooltip text.
    pub tooltip: &'static str,
    /// What kind of value it holds, with its bounds.
    pub kind: ParamKind,
    /// The value of a new object and the value an invalid stored one reads as.
    pub default: ParamValue,
}

/// The kind of a parameter. Only the kinds a story uses exist (ADR 0034 §4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParamKind {
    /// A length in mm within `min..=max`.
    Length {
        /// The smallest value.
        min: Length,
        /// The largest value.
        max: Length,
    },
    /// A text the host shows and the maker does not edit (the placeholder machine name).
    Display,
}

/// A typed parameter value.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ParamValue {
    /// A length.
    Length(Length),
    /// A text.
    Text(String),
}

impl ParamValue {
    /// The stored encoding: a length is `{"mm": <number>}`, a text a plain string.
    #[must_use]
    pub fn to_stored(&self) -> Value {
        match self {
            Self::Length(length) => {
                let mut map = Map::new();
                map.insert(MM.to_owned(), Value::from(length.as_mm()));
                Value::Object(map)
            }
            Self::Text(text) => Value::String(text.clone()),
        }
    }

    /// The value a stored encoding holds, `None` for any other shape or a number that is not
    /// finite.
    #[must_use]
    pub fn from_stored(value: &Value) -> Option<Self> {
        match value {
            Value::Object(map) if map.len() == 1 => {
                let mm = map.get(MM)?.as_f64().filter(|mm| mm.is_finite())?;
                Some(Self::Length(Length::from_mm(mm)))
            }
            Value::String(text) => Some(Self::Text(text.clone())),
            _ => None,
        }
    }

    /// The length this value holds, if it is one.
    #[must_use]
    pub const fn length(&self) -> Option<Length> {
        match self {
            Self::Length(length) => Some(*length),
            Self::Text(_) => None,
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

    /// An edited value as it is written: a length clamped into the bounds (UX notes §4, permit
    /// within bounds), `None` for a value of the wrong kind or a non-finite number.
    #[must_use]
    pub fn accept(&self, value: ParamValue) -> Option<ParamValue> {
        match (self.kind, value) {
            (ParamKind::Length { min, max }, ParamValue::Length(length))
                if length.as_mm().is_finite() =>
            {
                Some(ParamValue::Length(Length::from_mm(
                    length.as_mm().clamp(min.as_mm(), max.as_mm()),
                )))
            }
            (ParamKind::Display, ParamValue::Text(text)) => Some(ParamValue::Text(text)),
            _ => None,
        }
    }

    /// The stored values of a new object: every parameter at its default.
    #[must_use]
    pub fn defaults(params: &[Self]) -> StoredParams {
        params
            .iter()
            .filter(|param| HostKey::from_key(param.key).is_none())
            .map(|param| (param.key.to_owned(), param.default.to_stored()))
            .collect()
    }
}

/// The generic manufacturing values a block may show, owned by the host rather than stored in
/// the extension's registers (ADR 0033 §1: the work area is the owned page's size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKey {
    /// The placeholder machine name (`mfg_machine_name`).
    MachineName,
    /// The work area width (the output page's width).
    WorkWidth,
    /// The work area height (the output page's height).
    WorkHeight,
}

impl HostKey {
    /// The parameter key that binds this host value.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::MachineName => "host.machine_name",
            Self::WorkWidth => "host.work_width",
            Self::WorkHeight => "host.work_height",
        }
    }

    /// The host value a parameter key binds, `None` for an extension's own key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        [Self::MachineName, Self::WorkWidth, Self::WorkHeight]
            .into_iter()
            .find(|host| host.key() == key)
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
            },
            default: ParamValue::Length(Length::from_mm(10.0)),
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
        assert_eq!(param.accept(ParamValue::Text("x".into())), None);
    }

    #[test]
    fn defaults_leave_out_host_values() {
        let mut host = spacing();
        host.key = HostKey::WorkWidth.key();
        let stored = Param::defaults(&[spacing(), host]);
        assert_eq!(stored.len(), 1);
        assert_eq!(
            HostKey::from_key("host.work_height"),
            Some(HostKey::WorkHeight)
        );
        assert_eq!(HostKey::from_key("spacing"), None);
    }
}
