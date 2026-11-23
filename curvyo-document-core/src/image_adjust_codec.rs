//! The stored shape of an image's adjustments (ADR 0072 §1, §5, amendment A1, A5), read with damage
//! and written here.
//!
//! ```text
//! meta map of an image (and of a clone with `own_adjustments`), besides its own registers:
//!   image_preset      : "<text>"        absent = "expert"; known values are the extension's
//!   image_strength    : i64 0..=100     absent = 50
//!   image_adjustments : [ { kind: "<text>", enabled: bool, fields: { <key>: i64, ... },
//!                           result?: { asset, model, model_version, input_hash: "<64 hex>",
//!                                      width_px, height_px: i64 } }, ... ]
//!                       ONE plain list value, written whole; absent or empty = no stack; the
//!                       `result` of an AI step names its derived asset (ADR 0077 §2)
//!   image_dither      : { mode: "<text>", <key>: i64, ... }
//!                       ONE plain map, written whole; absent = Floyd-Steinberg with the `0123`
//!                       defaults; the mode names, keys and ranges are the extension's
//! ```
//!
//! Reads never repair and never refuse the file: a register that does not read reads as its
//! default and is counted, an entry that is not of the step shape is kept as stored.

use std::collections::BTreeMap;

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::assets::AssetHash;
use crate::image_adjust_model::{
    AdjustDamage, AdjustEntry, AdjustStep, AdjustmentsWrite, ImageAdjustments, MAX_IMAGE_STRENGTH,
    RanStep, StepResult, StoredDither,
};
use crate::manufacturing_codec::{json_of, loro_of};

/// The preset register.
pub(crate) const KEY_PRESET: &str = "image_preset";
/// The Strength register.
pub(crate) const KEY_STRENGTH: &str = "image_strength";
/// The stack register.
pub(crate) const KEY_ADJUSTMENTS: &str = "image_adjustments";
/// The Dither register (ADR 0072 amendment A1).
pub(crate) const KEY_DITHER: &str = "image_dither";
/// The four registers, in the order a copy writes them.
pub(crate) const ADJUST_KEYS: [&str; 4] = [KEY_PRESET, KEY_STRENGTH, KEY_ADJUSTMENTS, KEY_DITHER];

const DITHER_MODE: &str = "mode";

const ENTRY_KIND: &str = "kind";
const ENTRY_ENABLED: &str = "enabled";
const ENTRY_FIELDS: &str = "fields";
const ENTRY_RESULT: &str = "result";

const RESULT_ASSET: &str = "asset";
const RESULT_MODEL: &str = "model";
const RESULT_VERSION: &str = "model_version";
const RESULT_INPUT: &str = "input_hash";
const RESULT_WIDTH: &str = "width_px";
const RESULT_HEIGHT: &str = "height_px";
const RESULT_KEYS: [&str; 6] = [
    RESULT_ASSET,
    RESULT_MODEL,
    RESULT_VERSION,
    RESULT_INPUT,
    RESULT_WIDTH,
    RESULT_HEIGHT,
];

/// One register as stored.
enum Stored {
    /// The key is absent.
    Absent,
    /// A plain value.
    Plain(LoroValue),
    /// A container, which no register here ever is (damage).
    Container,
}

fn stored(meta: &LoroMap, key: &str) -> Stored {
    match meta.get(key) {
        None => Stored::Absent,
        Some(ValueOrContainer::Value(value)) => Stored::Plain(value),
        Some(ValueOrContainer::Container(_)) => Stored::Container,
    }
}

/// One stored entry as a step, `None` when it is not of the step shape.
fn read_step(value: &LoroValue) -> Option<AdjustStep> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    if map
        .keys()
        .any(|key| ![ENTRY_KIND, ENTRY_ENABLED, ENTRY_FIELDS, ENTRY_RESULT].contains(&key.as_str()))
    {
        return None;
    }
    let LoroValue::String(kind) = map.get(ENTRY_KIND)? else {
        return None;
    };
    let LoroValue::Bool(enabled) = map.get(ENTRY_ENABLED)? else {
        return None;
    };
    let LoroValue::Map(stored) = map.get(ENTRY_FIELDS)? else {
        return None;
    };
    let mut fields = BTreeMap::new();
    for (key, value) in stored.iter() {
        let LoroValue::I64(number) = value else {
            return None;
        };
        fields.insert(key.clone(), *number);
    }
    Some(AdjustStep {
        kind: kind.to_string(),
        enabled: *enabled,
        fields,
    })
}

/// The stored result of an entry, `None` when it is not a map of the six keys with two hashes,
/// two texts and two sizes from 1 to `u32::MAX`.
fn read_result(value: &LoroValue) -> Option<StepResult> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    if map.len() != RESULT_KEYS.len() || map.keys().any(|k| !RESULT_KEYS.contains(&k.as_str())) {
        return None;
    }
    let text = |key: &str| match map.get(key) {
        Some(LoroValue::String(text)) => Some(text.to_string()),
        _ => None,
    };
    let size = |key: &str| match map.get(key) {
        Some(LoroValue::I64(n)) => u32::try_from(*n).ok().filter(|n| *n > 0),
        _ => None,
    };
    Some(StepResult {
        asset: AssetHash::parse(&text(RESULT_ASSET)?)?,
        model: text(RESULT_MODEL)?,
        model_version: text(RESULT_VERSION)?,
        input_hash: AssetHash::parse(&text(RESULT_INPUT)?)?,
        width_px: size(RESULT_WIDTH)?,
        height_px: size(RESULT_HEIGHT)?,
    })
}

fn result_value(result: &StepResult) -> LoroValue {
    let map: std::collections::HashMap<String, LoroValue> = [
        (RESULT_ASSET, LoroValue::from(result.asset.as_str())),
        (RESULT_MODEL, LoroValue::from(result.model.as_str())),
        (
            RESULT_VERSION,
            LoroValue::from(result.model_version.as_str()),
        ),
        (RESULT_INPUT, LoroValue::from(result.input_hash.as_str())),
        (RESULT_WIDTH, LoroValue::I64(i64::from(result.width_px))),
        (RESULT_HEIGHT, LoroValue::I64(i64::from(result.height_px))),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    LoroValue::from(map)
}

/// The plain form of an unreadable entry; `null` for a value that has none (bytes, a NaN).
fn unreadable_json(value: &LoroValue) -> serde_json::Value {
    json_of(value).unwrap_or(serde_json::Value::Null)
}

/// The stored Dither, `None` when it is not a map with a text `mode` and integer fields.
fn read_dither(value: &LoroValue) -> Option<StoredDither> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    let LoroValue::String(mode) = map.get(DITHER_MODE)? else {
        return None;
    };
    let mut fields = BTreeMap::new();
    for (key, value) in map.iter().filter(|(key, _)| key.as_str() != DITHER_MODE) {
        let LoroValue::I64(number) = value else {
            return None;
        };
        fields.insert(key.clone(), *number);
    }
    Some(StoredDither {
        mode: mode.to_string(),
        fields,
    })
}

fn dither_value(dither: &StoredDither) -> LoroValue {
    let map: std::collections::HashMap<String, LoroValue> = dither
        .fields
        .iter()
        .map(|(key, value)| (key.clone(), LoroValue::I64(*value)))
        .chain([(
            DITHER_MODE.to_owned(),
            LoroValue::from(dither.mode.as_str()),
        )])
        .collect();
    LoroValue::from(map)
}

/// One stored entry: a step, a step with its stored result, or (not of the step shape, or a
/// `result` that does not read) unreadable.
fn read_entry(value: &LoroValue) -> AdjustEntry {
    let unreadable = || AdjustEntry::Unreadable(unreadable_json(value));
    let Some(step) = read_step(value) else {
        return unreadable();
    };
    let LoroValue::Map(map) = value else {
        return unreadable();
    };
    match map.get(ENTRY_RESULT) {
        None => AdjustEntry::Step(step),
        Some(stored) => read_result(stored).map_or_else(unreadable, |result| {
            AdjustEntry::Ran(RanStep { step, result })
        }),
    }
}

/// Whether a stored entry is unreadable (kept as stored by a write).
fn is_unreadable(value: &LoroValue) -> bool {
    matches!(read_entry(value), AdjustEntry::Unreadable(_))
}

/// The adjustment registers stored in `meta`, each damaged one read as its default and named in
/// [`ImageAdjustments::damage`].
pub(crate) fn read_adjustments(meta: &LoroMap) -> ImageAdjustments {
    let mut read = ImageAdjustments::default();
    let mut damage = AdjustDamage::default();
    match stored(meta, KEY_PRESET) {
        Stored::Absent => {}
        Stored::Plain(LoroValue::String(text)) => read.preset = Some(text.to_string()),
        Stored::Plain(_) | Stored::Container => damage.preset = true,
    }
    match stored(meta, KEY_STRENGTH) {
        Stored::Absent => {}
        Stored::Plain(LoroValue::I64(n)) if (0..=i64::from(MAX_IMAGE_STRENGTH)).contains(&n) => {
            // invariant: 0 to 100 fits a u8.
            read.strength = u8::try_from(n).unwrap_or(MAX_IMAGE_STRENGTH);
        }
        Stored::Plain(_) | Stored::Container => damage.strength = true,
    }
    match stored(meta, KEY_ADJUSTMENTS) {
        Stored::Absent => {}
        Stored::Plain(LoroValue::List(list)) => read.steps = list.iter().map(read_entry).collect(),
        Stored::Plain(_) | Stored::Container => damage.stack = true,
    }
    match stored(meta, KEY_DITHER) {
        Stored::Absent => {}
        Stored::Plain(value) => match read_dither(&value) {
            Some(dither) => read.dither = Some(dither),
            None => damage.dither = true,
        },
        Stored::Container => damage.dither = true,
    }
    read.damage = damage;
    read
}

/// One entry as its stored value.
fn entry_value(entry: &AdjustEntry) -> LoroValue {
    match entry {
        AdjustEntry::Step(step) | AdjustEntry::Ran(RanStep { step, .. }) => {
            let fields: std::collections::HashMap<String, LoroValue> = step
                .fields
                .iter()
                .map(|(key, value)| (key.clone(), LoroValue::I64(*value)))
                .collect();
            let mut map: std::collections::HashMap<String, LoroValue> = [
                (ENTRY_KIND.to_owned(), LoroValue::from(step.kind.as_str())),
                (ENTRY_ENABLED.to_owned(), LoroValue::Bool(step.enabled)),
                (ENTRY_FIELDS.to_owned(), LoroValue::from(fields)),
            ]
            .into_iter()
            .collect();
            if let Some(result) = entry.result() {
                map.insert(ENTRY_RESULT.to_owned(), result_value(result));
            }
            LoroValue::from(map)
        }
        AdjustEntry::Unreadable(value) => loro_of(value),
    }
}

fn insert(meta: &LoroMap, key: &str, value: LoroValue) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

/// Writes each register `write` names, uncommitted; the others stay as stored.
pub(crate) fn write_adjustments(meta: &LoroMap, write: &AdjustmentsWrite) {
    if let Some(preset) = &write.preset {
        insert(meta, KEY_PRESET, LoroValue::from(preset.as_str()));
    }
    if let Some(strength) = write.strength {
        insert(meta, KEY_STRENGTH, LoroValue::I64(i64::from(strength)));
    }
    if let Some(steps) = &write.steps {
        // An unreadable entry is written back from its stored value, so one with no plain form
        // (bytes, a NaN) is kept as stored and never turned into `null` by the next edit.
        let mut kept: Vec<Option<LoroValue>> = match stored(meta, KEY_ADJUSTMENTS) {
            Stored::Plain(LoroValue::List(list)) => list
                .iter()
                .filter(|value| is_unreadable(value))
                .cloned()
                .map(Some)
                .collect(),
            _ => Vec::new(),
        };
        let list: Vec<LoroValue> = steps
            .iter()
            .map(|entry| match entry {
                AdjustEntry::Unreadable(json) => kept
                    .iter_mut()
                    .find(|raw| {
                        raw.as_ref()
                            .is_some_and(|raw| unreadable_json(raw) == *json)
                    })
                    .and_then(Option::take)
                    .unwrap_or_else(|| entry_value(entry)),
                AdjustEntry::Step(_) | AdjustEntry::Ran(_) => entry_value(entry),
            })
            .collect();
        insert(meta, KEY_ADJUSTMENTS, LoroValue::from(list));
    }
    if let Some(dither) = &write.dither {
        insert(meta, KEY_DITHER, dither_value(dither));
    }
}

/// The derived assets the stack stored in `meta` names (ADR 0077 §5), in stack order.
pub(crate) fn derived_assets(meta: &LoroMap) -> Vec<AssetHash> {
    let Stored::Plain(LoroValue::List(list)) = stored(meta, KEY_ADJUSTMENTS) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|value| match read_entry(value) {
            AdjustEntry::Ran(ran) => Some(ran.result.asset),
            _ => None,
        })
        .collect()
}

/// Removes the Dither register from `meta`, uncommitted (Restore original, amendment A3).
pub(crate) fn remove_dither(meta: &LoroMap) {
    remove(meta, KEY_DITHER);
}

/// The four registers of `meta` exactly as stored, damage included (`None`: absent).
pub(crate) fn raw_registers(meta: &LoroMap) -> [Option<LoroValue>; 4] {
    ADJUST_KEYS.map(|key| match stored(meta, key) {
        Stored::Plain(value) => Some(value),
        Stored::Absent | Stored::Container => None,
    })
}

/// Makes the four registers of `meta` exactly `raw` (absent ones removed), uncommitted.
pub(crate) fn write_raw_registers(meta: &LoroMap, raw: &[Option<LoroValue>; 4]) {
    for (key, value) in ADJUST_KEYS.iter().zip(raw) {
        match value {
            Some(value) => insert(meta, key, value.clone()),
            None => remove(meta, key),
        }
    }
}

/// Removes the four registers from `meta`, uncommitted.
pub(crate) fn remove_all(meta: &LoroMap) {
    for key in ADJUST_KEYS {
        remove(meta, key);
    }
}

fn remove(meta: &LoroMap, key: &str) {
    if meta.get(key).is_some() {
        // invariant: deleting a key just confirmed present on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(key).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;
    use serde_json::json;

    use super::*;

    fn meta() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let map = doc.get_map("meta");
        (doc, map)
    }

    fn levels(enabled: bool) -> AdjustEntry {
        AdjustEntry::Step(AdjustStep {
            kind: "levels".into(),
            enabled,
            fields: [("black_pct", 2), ("white_pct", 98), ("gamma_centi", 120)]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        })
    }

    fn halftone() -> StoredDither {
        StoredDither {
            mode: "halftone".into(),
            fields: [
                ("threshold_pct", 50),
                ("screen_size_um", 1_600),
                ("screen_angle_deg", 30),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        }
    }

    /// ADR 0072 §1: absent registers read as Expert, 50 and no stack.
    #[test]
    fn absent_registers_read_as_their_defaults() {
        let (_doc, map) = meta();
        let read = read_adjustments(&map);
        assert_eq!(read, ImageAdjustments::default());
        assert_eq!(read.strength, 50);
        assert_eq!(read.dither, None);
        assert!(!read.is_adjusted());
        assert!(!read.is_restorable());
    }

    /// Amendment A3: a stored Dither alone enables Restore original, never `is_adjusted`.
    #[test]
    fn a_stored_dither_alone_is_restorable_and_not_adjusted() {
        let (_doc, map) = meta();
        write_adjustments(
            &map,
            &AdjustmentsWrite {
                dither: Some(halftone()),
                ..AdjustmentsWrite::default()
            },
        );
        let read = read_adjustments(&map);
        assert!(read.is_restorable());
        assert!(!read.is_adjusted());
        let LoroValue::Map(stored) = map.get(KEY_DITHER).unwrap().into_value().unwrap() else {
            panic!("one plain map, never a container");
        };
        assert_eq!(stored.len(), 4, "mode and every field");
        remove_dither(&map);
        assert!(!read_adjustments(&map).is_restorable());
    }

    /// Amendment A5, the shape cases document-core sees: not a map, a mode that is no text or
    /// missing, a field that is no integer; each reads as no Dither, is named and kept.
    #[test]
    fn a_dither_of_the_wrong_shape_is_damage_and_kept() {
        let (_doc, map) = meta();
        for value in [
            json!("halftone"),
            json!({"threshold_pct": 50}),
            json!({"mode": 3, "threshold_pct": 50}),
            json!({"mode": "halftone", "threshold_pct": 50.5}),
        ] {
            map.insert(KEY_DITHER, loro_of(&value)).unwrap();
            let read = read_adjustments(&map);
            assert_eq!(read.dither, None, "{value}");
            assert!(read.damage.dither, "{value}");
            assert!(read.is_restorable(), "{value}");
            assert_eq!(read.damage.count(), 1);
            assert_eq!(
                json_of(&map.get(KEY_DITHER).unwrap().into_value().unwrap()),
                Some(value),
                "kept as stored"
            );
        }
        let later = json!({"mode": "halftone_x", "screen_size_um": 6_000});
        map.insert(KEY_DITHER, loro_of(&later)).unwrap();
        let read = read_adjustments(&map);
        assert!(
            !read.damage.dither,
            "the mode and the ranges are the extension's"
        );
        assert_eq!(read.dither.unwrap().mode, "halftone_x");
    }

    /// ADR 0072 §1: every register reads back as written, fields as integers in their keys.
    #[test]
    fn the_registers_round_trip() {
        let (_doc, map) = meta();
        let write = AdjustmentsWrite {
            preset: Some("photo".into()),
            strength: Some(70),
            steps: Some(vec![levels(true), levels(false)]),
            dither: Some(halftone()),
        };
        write_adjustments(&map, &write);
        let read = read_adjustments(&map);
        assert_eq!(read.preset.as_deref(), Some("photo"));
        assert_eq!(read.strength, 70);
        assert_eq!(read.steps, vec![levels(true), levels(false)]);
        assert_eq!(read.dither, Some(halftone()));
        assert_eq!(read.damage, AdjustDamage::default());
        assert!(read.is_adjusted());
        let LoroValue::List(list) = map.get(KEY_ADJUSTMENTS).unwrap().into_value().unwrap() else {
            panic!("a plain list value, never a container");
        };
        assert_eq!(list.len(), 2);
    }

    /// A write of one register leaves the other two as stored (merge safety, ADR 0072 §4).
    #[test]
    fn a_write_touches_only_the_registers_it_names() {
        let (_doc, map) = meta();
        write_adjustments(
            &map,
            &AdjustmentsWrite {
                preset: Some("graphics".into()),
                steps: Some(vec![levels(true)]),
                ..AdjustmentsWrite::default()
            },
        );
        write_adjustments(
            &map,
            &AdjustmentsWrite {
                strength: Some(10),
                ..AdjustmentsWrite::default()
            },
        );
        let read = read_adjustments(&map);
        assert_eq!(read.preset.as_deref(), Some("graphics"));
        assert_eq!(read.steps.len(), 1);
        assert_eq!(read.strength, 10);
    }

    /// ADR 0072 §5, one case per reason: each damaged register reads as its default and is
    /// named; an entry not of the step shape is kept and written back unchanged.
    #[test]
    fn damage_reads_as_the_adr_says_and_is_kept() {
        let (_doc, map) = meta();
        map.insert(KEY_PRESET, 7_i64).unwrap();
        map.insert(KEY_STRENGTH, 101_i64).unwrap();
        map.insert(KEY_ADJUSTMENTS, "levels").unwrap();
        let read = read_adjustments(&map);
        assert_eq!(read.preset, None);
        assert_eq!(read.strength, 50);
        assert_eq!(read.steps, []);
        assert_eq!(
            read.damage,
            AdjustDamage {
                preset: true,
                strength: true,
                stack: true,
                dither: false,
            }
        );
        assert_eq!(read.damage.count(), 3);
        for strength in [LoroValue::Double(50.0), LoroValue::I64(-1), "50".into()] {
            map.insert(KEY_STRENGTH, strength).unwrap();
            assert!(read_adjustments(&map).damage.strength);
        }
        let entries = [
            json!("levels"),
            json!({"kind": 3, "enabled": true, "fields": {}}),
            json!({"kind": "levels", "enabled": 1, "fields": {}}),
            json!({"kind": "levels", "fields": {}}),
            json!({"kind": "levels", "enabled": true, "fields": {"black_pct": 2.5}}),
            json!({"kind": "levels", "enabled": true, "fields": [1, 2]}),
            json!({"kind": "levels", "enabled": true, "fields": {}, "later": 1}),
        ];
        let list: Vec<LoroValue> = entries.iter().map(loro_of).collect();
        map.insert(KEY_ADJUSTMENTS, LoroValue::from(list)).unwrap();
        let read = read_adjustments(&map);
        assert!(!read.damage.stack);
        assert_eq!(read.steps.len(), entries.len());
        for (entry, stored) in read.steps.iter().zip(&entries) {
            assert_eq!(entry, &AdjustEntry::Unreadable(stored.clone()));
        }
        let before = map.get(KEY_ADJUSTMENTS).unwrap().into_value().unwrap();
        write_adjustments(
            &map,
            &AdjustmentsWrite {
                steps: Some(read.steps),
                ..AdjustmentsWrite::default()
            },
        );
        assert_eq!(
            map.get(KEY_ADJUSTMENTS).unwrap().into_value().unwrap(),
            before,
            "kept and written back unchanged"
        );
    }

    /// An entry with no plain form (bytes, a NaN) is kept as stored by the next edit of the
    /// stack, not written as `null`, wherever the edit moves it.
    #[test]
    fn an_entry_with_no_plain_form_survives_the_next_edit() {
        let (_doc, map) = meta();
        let bytes = LoroValue::from(vec![1_u8, 2, 3]);
        let nan: LoroValue = [("kind".to_owned(), LoroValue::Double(f64::NAN))]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>()
            .into();
        let step = entry_value(&levels(true));
        map.insert(
            KEY_ADJUSTMENTS,
            LoroValue::from(vec![bytes.clone(), step, nan.clone()]),
        )
        .unwrap();
        let mut steps = read_adjustments(&map).steps;
        assert_eq!(steps[0], AdjustEntry::Unreadable(serde_json::Value::Null));
        steps.swap(0, 1);
        steps.push(levels(false));
        write_adjustments(
            &map,
            &AdjustmentsWrite {
                steps: Some(steps),
                ..AdjustmentsWrite::default()
            },
        );
        let LoroValue::List(list) = map.get(KEY_ADJUSTMENTS).unwrap().into_value().unwrap() else {
            panic!("a list");
        };
        assert_eq!(list[1], bytes, "the bytes stay bytes");
        let LoroValue::Map(kept) = &list[2] else {
            panic!("the map stays a map: {:?}", list[2]);
        };
        assert!(matches!(kept.get("kind"), Some(LoroValue::Double(d)) if d.is_nan()));
        assert_eq!(list.len(), 4);
    }

    /// An unknown kind is of the step shape: the extension decides that it does not run.
    #[test]
    fn an_unknown_kind_reads_as_a_step() {
        let (_doc, map) = meta();
        let later = json!([{"kind": "vignette", "enabled": true, "fields": {"amount_pct": 5}}]);
        map.insert(KEY_ADJUSTMENTS, loro_of(&later)).unwrap();
        let read = read_adjustments(&map);
        assert_eq!(read.steps[0].step().unwrap().kind, "vignette");
    }

    #[test]
    fn raw_registers_copy_damage_and_absence_exactly() {
        let (_doc, from) = meta();
        let doc2 = LoroDoc::new();
        let to = doc2.get_map("meta");
        from.insert(KEY_STRENGTH, "x").unwrap();
        write_adjustments(
            &from,
            &AdjustmentsWrite {
                steps: Some(vec![levels(true)]),
                ..AdjustmentsWrite::default()
            },
        );
        from.insert(KEY_DITHER, loro_of(&json!({"mode": "grey"})))
            .unwrap();
        to.insert(KEY_PRESET, "photo").unwrap();
        write_raw_registers(&to, &raw_registers(&from));
        assert_eq!(read_adjustments(&to), read_adjustments(&from));
        assert!(to.get(KEY_PRESET).is_none());
        remove_all(&to);
        assert_eq!(to.len(), 0);
    }
}
