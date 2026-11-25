//! The registers of a sketch instance (`specs/0099-sketch-pages/adrs.md` A3, A13): the sketch's
//! `exposed_params` list, a clone's `instance_params` and `instance_fallback` maps and its
//! `instance_unlinked` list, each one LWW value written whole, and the one-key encoding
//! `{"mm": n}` or `{"deg": n}` of a number.
//!
//! ```text
//! exposed_params    : [ { "key": "p:width", "min": {"mm": 20}, "max": {"mm": 200} },
//!                       { "key": "d:c7", "name": "Radius 1", "link": "d:c7" },
//!                       { "key": "d:c8", "name": "Radius 2", "link": "d:c7" } ]
//! instance_params   : { "p:width": {"mm": 82}, "d:c7": {"deg": 30} }
//! instance_fallback : { "p:width": {"mm": 150} }
//! instance_unlinked : ["d:c8"]
//! ```
//!
//! Damage is a wrong register shape only; a value that does not decode is ignored and kept.

use loro::{LoroMap, LoroValue, ValueOrContainer};
use serde_json::{Map, Value};

use crate::instance_value::{ExposedKey, InstanceValue, KeySet, ValueSet};

/// The sketch register.
pub(crate) const KEY_EXPOSED_PARAMS: &str = "exposed_params";
/// The clone register of own values.
pub(crate) const KEY_INSTANCE_PARAMS: &str = "instance_params";
/// The clone register of the last value set that solved.
pub(crate) const KEY_INSTANCE_FALLBACK: &str = "instance_fallback";
/// The clone register of the keys it changed alone, unlinked from their group.
pub(crate) const KEY_INSTANCE_UNLINKED: &str = "instance_unlinked";

const MM: &str = "mm";
const DEG: &str = "deg";
const LIMIT: f64 = 1e7;
const NAME_MAX: usize = 64;

/// The unit word of a stored number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredUnit {
    /// Millimetres, `{"mm": n}`.
    Millimetres,
    /// Degrees, `{"deg": n}`.
    Degrees,
}

/// The stored one-key encoding of `number` in `unit`; the one encoding of a length or an angle
/// that sketch instances and generated objects share (ADR 0080 §2).
#[must_use]
pub fn encode_number(unit: StoredUnit, number: f64) -> Value {
    let key = match unit {
        StoredUnit::Millimetres => MM,
        StoredUnit::Degrees => DEG,
    };
    let mut map = Map::new();
    map.insert(key.to_owned(), Value::from(number));
    Value::Object(map)
}

/// The unit and number of a one-key encoding of a length or an angle, `None` for any other shape
/// or a number that is not finite.
#[must_use]
pub fn decode_number(value: &Value) -> Option<(StoredUnit, f64)> {
    let Value::Object(map) = value else {
        return None;
    };
    if map.len() != 1 {
        return None;
    }
    let (key, inner) = map.iter().next()?;
    let number = inner.as_f64().filter(|number| number.is_finite())?;
    match key.as_str() {
        MM => Some((StoredUnit::Millimetres, number)),
        DEG => Some((StoredUnit::Degrees, number)),
        _ => None,
    }
}

/// The stored one-key encoding of `value`.
#[must_use]
pub fn encode_value(value: InstanceValue) -> Value {
    let unit = match value {
        InstanceValue::Length(_) => StoredUnit::Millimetres,
        InstanceValue::Angle(_) => StoredUnit::Degrees,
    };
    encode_number(unit, value.number())
}

/// The value a one-key encoding holds, `None` for any other shape or a number that is not finite.
#[must_use]
pub fn decode_value(value: &Value) -> Option<InstanceValue> {
    match decode_number(value)? {
        (StoredUnit::Millimetres, number) => Some(InstanceValue::from_mm(number)),
        (StoredUnit::Degrees, number) => Some(InstanceValue::from_deg(number)),
    }
}

/// A decoded entry of `exposed_params`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExposedEntry {
    pub(crate) key: ExposedKey,
    pub(crate) name: Option<String>,
    pub(crate) min: Option<InstanceValue>,
    pub(crate) max: Option<InstanceValue>,
    /// Its link group's string, `None` without a valid one.
    pub(crate) link: Option<String>,
}

impl ExposedEntry {
    /// The stored JSON of the entry.
    pub(crate) fn to_json(&self) -> Value {
        self.merged_into(Map::new())
    }

    /// The entry written over the stored map `raw`: its own fields replace theirs (an absent one
    /// is removed), every field it does not know is kept.
    pub(crate) fn merged_into(&self, raw: Map<String, Value>) -> Value {
        let mut map = raw;
        for field in ["key", "name", "min", "max", "link"] {
            map.remove(field);
        }
        map.insert("key".into(), Value::from(self.key.as_str()));
        if let Some(name) = &self.name {
            map.insert("name".into(), Value::from(name.as_str()));
        }
        if let Some(min) = self.min {
            map.insert("min".into(), encode_value(min));
        }
        if let Some(max) = self.max {
            map.insert("max".into(), encode_value(max));
        }
        if let Some(link) = &self.link {
            map.insert("link".into(), Value::from(link.as_str()));
        }
        Value::Object(map)
    }
}

fn bound(value: Option<&Value>) -> Option<InstanceValue> {
    let decoded = decode_value(value?)?;
    (decoded.number().abs() <= LIMIT).then_some(decoded)
}

fn name_is_valid(name: &str) -> bool {
    let length = name.chars().count();
    (1..=NAME_MAX).contains(&length) && !name.chars().any(char::is_control)
}

/// Decodes one stored entry; `None` when it does not decode (it is then ignored and kept).
pub(crate) fn decode_entry(value: &Value) -> Option<ExposedEntry> {
    let map = value.as_object()?;
    let key = ExposedKey::parse(map.get("key")?.as_str()?)?;
    let name = match (key.as_str().starts_with("d:"), map.get("name")) {
        (true, Some(name)) => {
            let name = name.as_str().filter(|name| name_is_valid(name))?;
            Some(name.to_owned())
        }
        _ => None,
    };
    let (mut min, mut max) = (bound(map.get("min")), bound(map.get("max")));
    if let (Some(low), Some(high)) = (min, max)
        && (low.kind() != high.kind() || low.number() > high.number())
    {
        min = None;
        max = None;
    }
    let link = map
        .get("link")
        .and_then(Value::as_str)
        .filter(|link| name_is_valid(link))
        .map(str::to_owned);
    Some(ExposedEntry {
        key,
        name,
        min,
        max,
        link,
    })
}

/// The stored link strings of `exposed_params`, valid or not, so a new group never takes one an
/// entry already uses (`adrs.md` A13).
pub(crate) fn used_links(items: &[Value]) -> std::collections::HashSet<String> {
    items
        .iter()
        .filter_map(|item| item.get("link"))
        .map(|link| match link {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect()
}

/// The register's items as stored, `None` when it is absent or not a list value.
fn raw_list(meta: &LoroMap) -> Option<Vec<Value>> {
    match meta.get(KEY_EXPOSED_PARAMS) {
        Some(ValueOrContainer::Value(value @ LoroValue::List(_))) => match Value::from(value) {
            Value::Array(items) => Some(items),
            _ => None,
        },
        _ => None,
    }
}

/// The stored items of `exposed_params`, undecodable ones included, empty when absent.
pub(crate) fn read_exposed_raw(meta: &LoroMap) -> Vec<Value> {
    raw_list(meta).unwrap_or_default()
}

/// The decoded entries of `exposed_params`, in order; an undecodable entry and a repeated key
/// (the first wins) are left out.
pub(crate) fn read_exposed(meta: &LoroMap) -> Vec<ExposedEntry> {
    let mut seen = std::collections::HashSet::new();
    read_exposed_raw(meta)
        .iter()
        .filter_map(decode_entry)
        .filter(|entry| seen.insert(entry.key.clone()))
        .collect()
}

/// Writes the whole list; an empty list is the absent key.
pub(crate) fn write_exposed_raw(meta: &LoroMap, items: &[Value]) {
    write_value(
        meta,
        KEY_EXPOSED_PARAMS,
        (!items.is_empty()).then(|| Value::Array(items.to_vec())),
    );
}

/// Reads a clone's value map under `key`: the keys and values that decode, kind not checked.
pub(crate) fn read_values(meta: &LoroMap, key: &str) -> ValueSet {
    let mut set = ValueSet::default();
    let Some(ValueOrContainer::Value(value @ LoroValue::Map(_))) = meta.get(key) else {
        return set;
    };
    let Value::Object(map) = Value::from(value) else {
        return set;
    };
    for (name, item) in &map {
        if let (Some(key), Some(value)) = (ExposedKey::parse(name), bound(Some(item))) {
            set.insert(key, value);
        }
    }
    set
}

/// The raw stored map under `key`, to keep undecodable entries when one is written.
pub(crate) fn read_values_raw(meta: &LoroMap, key: &str) -> Map<String, Value> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(value @ LoroValue::Map(_))) => match Value::from(value) {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        _ => Map::new(),
    }
}

/// Writes `values` whole under `key`, keeping the raw entries that did not decode; an empty map is
/// the absent key.
pub(crate) fn write_values(meta: &LoroMap, key: &str, values: &ValueSet) {
    let mut map: Map<String, Value> = read_values_raw(meta, key)
        .into_iter()
        .filter(|(name, item)| ExposedKey::parse(name).is_none() || bound(Some(item)).is_none())
        .collect();
    for (name, value) in values.iter() {
        map.insert(name.as_str().to_owned(), encode_value(value));
    }
    write_value(meta, key, (!map.is_empty()).then_some(Value::Object(map)));
}

fn write_value(meta: &LoroMap, key: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            // invariant: a plain value under a known key on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.insert(key, LoroValue::from(value)).unwrap();
        }
        None if meta.get(key).is_some() => {
            // invariant: deleting a key just confirmed present on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(key).unwrap();
        }
        None => {}
    }
}

/// Reads a clone's `instance_unlinked`: the keys that parse, nothing else checked.
pub(crate) fn read_unlinked(meta: &LoroMap) -> KeySet {
    read_unlinked_raw(meta)
        .iter()
        .filter_map(Value::as_str)
        .filter_map(ExposedKey::parse)
        .collect()
}

/// The stored items of `instance_unlinked`, empty when absent or not a list value.
pub(crate) fn read_unlinked_raw(meta: &LoroMap) -> Vec<Value> {
    match meta.get(KEY_INSTANCE_UNLINKED) {
        Some(ValueOrContainer::Value(value @ LoroValue::List(_))) => match Value::from(value) {
            Value::Array(items) => items,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Writes `keys` whole, keeping the stored items that do not parse; an empty list is the absent
/// key.
pub(crate) fn write_unlinked(meta: &LoroMap, keys: &KeySet) {
    let mut items: Vec<Value> = read_unlinked_raw(meta)
        .into_iter()
        .filter(|item| item.as_str().and_then(ExposedKey::parse).is_none())
        .collect();
    items.extend(keys.iter().map(|key| Value::from(key.as_str())));
    write_value(
        meta,
        KEY_INSTANCE_UNLINKED,
        (!items.is_empty()).then_some(Value::Array(items)),
    );
}

/// Whether `exposed_params` on a sketch node is absent or a list value.
pub(crate) fn sketch_shape_ok(meta: &LoroMap) -> bool {
    matches!(
        meta.get(KEY_EXPOSED_PARAMS),
        None | Some(ValueOrContainer::Value(LoroValue::List(_)))
    )
}

/// Whether the two clone value registers are absent or map values and `instance_unlinked` is
/// absent or a list value.
pub(crate) fn clone_shape_ok(meta: &LoroMap) -> bool {
    [KEY_INSTANCE_PARAMS, KEY_INSTANCE_FALLBACK]
        .into_iter()
        .all(|key| {
            matches!(
                meta.get(key),
                None | Some(ValueOrContainer::Value(LoroValue::Map(_)))
            )
        })
        && matches!(
            meta.get(KEY_INSTANCE_UNLINKED),
            None | Some(ValueOrContainer::Value(LoroValue::List(_)))
        )
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;
    use serde_json::json;

    use super::*;

    fn meta() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("t");
        let node = tree.create(None).unwrap();
        let meta = tree.get_meta(node).unwrap();
        (doc, meta)
    }

    #[test]
    fn a_value_reads_back_the_stored_number() {
        let angle = decode_value(&json!({"deg": 30.0})).unwrap();
        assert_eq!(encode_value(angle), json!({"deg": 30.0}));
        let length = decode_value(&json!({"mm": 82})).unwrap();
        assert_eq!(length, InstanceValue::from_mm(82.0));
        assert_eq!(decode_value(&json!({"mm": 1, "deg": 2})), None);
        assert_eq!(decode_value(&json!({"cm": 1})), None);
        assert_eq!(decode_value(&json!(5)), None);
    }

    #[test]
    fn exposed_entries_decode_and_bad_ones_are_ignored_but_kept() {
        let (_doc, meta) = meta();
        let items = vec![
            json!({"key": "p:width", "min": {"mm": 20}, "max": {"mm": 200}}),
            json!({"key": "d:c7", "name": "Radius 1"}),
            json!({"key": "x:nope"}),
            json!({"key": "d:c8", "name": ""}),
            json!({"key": "p:width", "min": {"mm": 1}}),
            json!({"key": "p:flip", "min": {"mm": 9}, "max": {"mm": 3}}),
        ];
        write_exposed_raw(&meta, &items);
        let read = read_exposed(&meta);
        assert_eq!(
            read.len(),
            3,
            "two ignored, the repeated key's second left out"
        );
        assert_eq!(read[0].min, Some(InstanceValue::from_mm(20.0)));
        assert_eq!(read[1].name.as_deref(), Some("Radius 1"));
        assert_eq!(
            (read[2].min, read[2].max),
            (None, None),
            "min above max is unlimited"
        );
        assert_eq!(read_exposed_raw(&meta), items, "kept as stored");
    }

    #[test]
    fn value_maps_keep_what_does_not_decode() {
        let (_doc, meta) = meta();
        meta.insert(
            KEY_INSTANCE_PARAMS,
            LoroValue::from(json!({"p:width": {"mm": 5}, "zz": 1, "d:c": {"cm": 2}})),
        )
        .unwrap();
        let mut set = read_values(&meta, KEY_INSTANCE_PARAMS);
        assert_eq!(set.len(), 1);
        set.insert(
            ExposedKey::parse("p:width").unwrap(),
            InstanceValue::from_mm(6.0),
        );
        write_values(&meta, KEY_INSTANCE_PARAMS, &set);
        let raw = read_values_raw(&meta, KEY_INSTANCE_PARAMS);
        assert_eq!(raw.len(), 3);
        assert_eq!(raw["p:width"], json!({"mm": 6.0}));
    }

    /// Criterion 37: a link is read from its entry, an invalid one is ignored and kept, and the
    /// entry written over its stored map keeps the fields it does not know.
    #[test]
    fn a_link_decodes_and_a_rewrite_keeps_unknown_fields() {
        let linked =
            decode_entry(&json!({"key": "d:c7", "name": "Radius 1", "link": "d:c7"})).unwrap();
        assert_eq!(linked.link.as_deref(), Some("d:c7"));
        let bad = decode_entry(&json!({"key": "d:c8", "name": "Radius 2", "link": 5})).unwrap();
        assert_eq!(bad.link, None, "ignored");
        let raw = json!({"key": "d:c7", "name": "Radius 1", "link": "g", "later": 1});
        let mut entry = decode_entry(&raw).unwrap();
        entry.min = Some(InstanceValue::from_mm(2.0));
        let written = entry.merged_into(raw.as_object().unwrap().clone());
        assert_eq!(written["link"], json!("g"));
        assert_eq!(written["later"], json!(1));
        assert_eq!(written["min"], json!({"mm": 2.0}));
        assert_eq!(used_links(&[raw, json!({"link": 5})]).len(), 2);
    }

    /// Criterion 37: the unlinked list keeps items that do not parse; a container refuses.
    #[test]
    fn the_unlinked_list_keeps_what_does_not_parse_and_a_container_is_damage() {
        let (_doc, meta) = meta();
        meta.insert(
            KEY_INSTANCE_UNLINKED,
            LoroValue::from(json!(["d:c8", 3, "zz"])),
        )
        .unwrap();
        let keys = read_unlinked(&meta);
        assert!(keys.contains(&ExposedKey::parse("d:c8").unwrap()));
        write_unlinked(&meta, &KeySet::default());
        assert_eq!(read_unlinked_raw(&meta), vec![json!(3), json!("zz")]);
        assert!(clone_shape_ok(&meta));
        meta.insert_container(KEY_INSTANCE_UNLINKED, LoroMap::new())
            .unwrap();
        assert!(!clone_shape_ok(&meta));
    }

    #[test]
    fn a_container_in_a_register_is_the_wrong_shape() {
        let (_doc, meta) = meta();
        assert!(clone_shape_ok(&meta) && sketch_shape_ok(&meta));
        meta.insert_container(KEY_INSTANCE_PARAMS, LoroMap::new())
            .unwrap();
        assert!(!clone_shape_ok(&meta));
        meta.insert(KEY_EXPOSED_PARAMS, "text").unwrap();
        assert!(!sketch_shape_ok(&meta));
    }
}
