//! The stored shape of an image's trace mask (ADR 0061 §1, §7), read with damage and written here.
//!
//! ```text
//! meta map of an image, besides its own registers (image_codec.rs):
//!   trace_part.<part>          : { order: i64, outside: bool }
//!   trace_settings.<part>      : { <setting key>: <plain value>, ... }   opaque here
//!   trace_region.<part>.<reg>  : { order: i64, kind: "rect" | "lasso", subtract: bool,
//!                                  points: [u0, v0, u1, v1, ...] }       the unit square
//! ```
//!
//! Reads never repair: a region that does not read is left out and kept, a part key that does not
//! read makes its part Inside with order 0, a region or settings key without its part key is left
//! out (dangling), and unknown `trace_*` keys are kept.

use std::collections::{BTreeMap, HashMap};

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::manufacturing_codec::json_of;
use crate::path_codec::as_f64;
use crate::trace_mask_model::{
    NewTraceRegion, TraceMask, TraceMaskPart, TraceMaskRegion, TracePartSettings, TraceRegionKind,
    UnitPoint,
};

const PART_PREFIX: &str = "trace_part.";
const SETTINGS_PREFIX: &str = "trace_settings.";
const REGION_PREFIX: &str = "trace_region.";
const ANY_PREFIX: &str = "trace_";

/// The part every first gesture on an untouched image stores (ADR 0061 §3).
pub const FIRST_TRACE_PART: &str = "0";

/// The key of part `part`.
pub(crate) fn part_key(part: &str) -> String {
    format!("{PART_PREFIX}{part}")
}

/// The key of part `part`'s settings.
pub(crate) fn settings_key(part: &str) -> String {
    format!("{SETTINGS_PREFIX}{part}")
}

/// The key of region `region` of part `part`.
pub(crate) fn region_key(part: &str, region: &str) -> String {
    format!("{REGION_PREFIX}{part}.{region}")
}

/// The part and region ids of a region key.
fn region_ids(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix(REGION_PREFIX)?;
    let (part, region) = rest.split_once('.')?;
    (!part.is_empty() && !region.is_empty() && !region.contains('.')).then_some((part, region))
}

/// What one stored part key reads as: its order and mode, Inside at order 0 when it does not read.
fn read_part(value: &ValueOrContainer) -> (i64, bool) {
    let ValueOrContainer::Value(LoroValue::Map(map)) = value else {
        return (0, false);
    };
    match (map.get("order"), map.get("outside")) {
        (Some(LoroValue::I64(order)), Some(LoroValue::Bool(outside))) => (*order, *outside),
        _ => (0, false),
    }
}

/// A stored settings map as plain JSON; a value that is not plain (or a number that is not
/// finite) reads as `null`, a key that is not a map reads as no settings.
fn read_settings(value: &ValueOrContainer) -> TracePartSettings {
    let ValueOrContainer::Value(LoroValue::Map(map)) = value else {
        return TracePartSettings::new();
    };
    map.iter()
        .map(|(key, value)| {
            (
                key.clone(),
                json_of(value).unwrap_or(serde_json::Value::Null),
            )
        })
        .collect()
}

/// A stored region, `None` when it does not read (ADR 0061 §7).
fn read_region(id: &str, value: &ValueOrContainer) -> Option<TraceMaskRegion> {
    let ValueOrContainer::Value(LoroValue::Map(map)) = value else {
        return None;
    };
    let LoroValue::I64(order) = map.get("order")? else {
        return None;
    };
    let LoroValue::String(tag) = map.get("kind")? else {
        return None;
    };
    let kind = TraceRegionKind::from_tag(tag)?;
    let LoroValue::Bool(subtract) = map.get("subtract")? else {
        return None;
    };
    let LoroValue::List(numbers) = map.get("points")? else {
        return None;
    };
    if numbers.len() % 2 != 0 || !kind.accepts(numbers.len() / 2) {
        return None;
    }
    let mut points = Vec::with_capacity(numbers.len() / 2);
    for pair in numbers.as_chunks::<2>().0 {
        let point = UnitPoint::new(as_f64(&pair[0])?, as_f64(&pair[1])?);
        if !point.is_storable() {
            return None;
        }
        points.push(point);
    }
    Some(TraceMaskRegion {
        id: id.to_owned(),
        order: *order,
        kind,
        subtract: *subtract,
        points,
    })
}

/// The trace mask stored in `meta`, parts and regions ordered by (`order`, id). Never by the map's
/// iteration order, so every peer reads the same (ADR 0061 §4).
pub(crate) fn read_trace_mask(meta: &LoroMap) -> TraceMask {
    let mut parts: BTreeMap<String, (i64, bool)> = BTreeMap::new();
    let mut settings: HashMap<String, TracePartSettings> = HashMap::new();
    let mut regions: HashMap<String, Vec<TraceMaskRegion>> = HashMap::new();
    meta.for_each(|key, value| {
        if let Some(part) = key.strip_prefix(PART_PREFIX) {
            parts.insert(part.to_owned(), read_part(&value));
        } else if let Some(part) = key.strip_prefix(SETTINGS_PREFIX) {
            settings.insert(part.to_owned(), read_settings(&value));
        } else if let Some((part, region)) = region_ids(key)
            && let Some(read) = read_region(region, &value)
        {
            regions.entry(part.to_owned()).or_default().push(read);
        }
    });
    let mut out: Vec<TraceMaskPart> = parts
        .into_iter()
        .map(|(id, (order, outside))| {
            let mut own = regions.remove(&id).unwrap_or_default();
            own.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
            TraceMaskPart {
                settings: settings.remove(&id).unwrap_or_default(),
                id,
                order,
                outside,
                regions: own,
            }
        })
        .collect();
    out.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
    TraceMask { parts: out }
}

/// The highest stored `order` of any part key, and of any region key that reads; `None` when
/// there is none. A dangling region counts, so a new region still lands on top of it.
pub(crate) fn highest_orders(meta: &LoroMap) -> (Option<i64>, Option<i64>) {
    let (mut parts, mut regions) = (None::<i64>, None::<i64>);
    meta.for_each(|key, value| {
        if key.starts_with(PART_PREFIX) {
            let (order, _) = read_part(&value);
            parts = Some(parts.map_or(order, |o| o.max(order)));
        } else if let Some((_, region)) = region_ids(key)
            && let Some(read) = read_region(region, &value)
        {
            regions = Some(regions.map_or(read.order, |o| o.max(read.order)));
        }
    });
    (parts, regions)
}

/// Writes part `part`'s key.
pub(crate) fn write_part(meta: &LoroMap, part: &str, order: i64, outside: bool) {
    let value: HashMap<String, LoroValue> = [
        ("order".to_owned(), LoroValue::I64(order)),
        ("outside".to_owned(), LoroValue::Bool(outside)),
    ]
    .into_iter()
    .collect();
    // invariant: inserting a plain map value into an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(&part_key(part), LoroValue::from(value))
        .unwrap();
}

/// Writes part `part`'s settings whole.
pub(crate) fn write_settings(meta: &LoroMap, part: &str, settings: &TracePartSettings) {
    crate::manufacturing_codec::write_plain_map(meta, &settings_key(part), settings);
}

/// Writes region `id` of part `part` whole.
pub(crate) fn write_region(
    meta: &LoroMap,
    part: &str,
    id: &str,
    order: i64,
    region: &NewTraceRegion,
) {
    let points: Vec<LoroValue> = region
        .points
        .iter()
        .flat_map(|p| [LoroValue::Double(p.u), LoroValue::Double(p.v)])
        .collect();
    let value: HashMap<String, LoroValue> = [
        ("order".to_owned(), LoroValue::I64(order)),
        ("kind".to_owned(), LoroValue::from(region.kind.tag())),
        ("subtract".to_owned(), LoroValue::Bool(region.subtract)),
        ("points".to_owned(), LoroValue::from(points)),
    ]
    .into_iter()
    .collect();
    // invariant: see `write_part`.
    #[allow(clippy::unwrap_used)]
    meta.insert(&region_key(part, id), LoroValue::from(value))
        .unwrap();
}

/// Removes `key` from `meta`.
pub(crate) fn delete(meta: &LoroMap, key: &str) {
    // invariant: deleting a key of an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.delete(key).unwrap();
}

/// Every key of `meta` that `doomed` accepts.
fn keys_where(meta: &LoroMap, doomed: impl Fn(&str) -> bool) -> Vec<String> {
    let mut keys = Vec::new();
    meta.for_each(|key, _| {
        if doomed(key) {
            keys.push(key.to_owned());
        }
    });
    keys
}

/// The region and settings keys whose part key is absent (one peer removed the part while the
/// other drew in it, ADR 0061 §4).
pub(crate) fn dangling_keys(meta: &LoroMap) -> Vec<String> {
    let parts: Vec<String> = keys_where(meta, |key| key.starts_with(PART_PREFIX))
        .into_iter()
        .filter_map(|key| key.strip_prefix(PART_PREFIX).map(str::to_owned))
        .collect();
    keys_where(meta, |key| {
        let part = key
            .strip_prefix(SETTINGS_PREFIX)
            .or_else(|| region_ids(key).map(|(part, _)| part));
        part.is_some_and(|part| !parts.iter().any(|p| p == part))
    })
}

/// The region keys of part `part`, whether they read or not.
pub(crate) fn region_keys_of(meta: &LoroMap, part: &str) -> Vec<String> {
    keys_where(meta, |key| region_ids(key).is_some_and(|(p, _)| p == part))
}

/// The key of region `region`, in whichever part holds it.
pub(crate) fn find_region_key(meta: &LoroMap, region: &str) -> Option<String> {
    keys_where(meta, |key| {
        region_ids(key).is_some_and(|(_, r)| r == region)
    })
    .into_iter()
    .next()
}

/// Removes every `trace_*` key of `meta`, uncommitted (an asset change, ADR 0061 §8).
pub(crate) fn remove_all(meta: &LoroMap) {
    for key in keys_where(meta, |key| key.starts_with(ANY_PREFIX)) {
        delete(meta, &key);
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

    fn rect(subtract: bool, a: (f64, f64), b: (f64, f64)) -> NewTraceRegion {
        NewTraceRegion {
            kind: TraceRegionKind::Rect,
            subtract,
            points: vec![UnitPoint::new(a.0, a.1), UnitPoint::new(b.0, b.1)],
        }
    }

    fn insert(map: &LoroMap, key: &str, value: &serde_json::Value) {
        let serde_json::Value::Object(object) = json!({ "v": value }) else {
            unreachable!()
        };
        crate::manufacturing_codec::write_plain_map(map, "scratch", &object);
        let ValueOrContainer::Value(LoroValue::Map(scratch)) = map.get("scratch").unwrap() else {
            unreachable!()
        };
        map.insert(key, scratch.get("v").unwrap().clone()).unwrap();
        map.delete("scratch").unwrap();
    }

    /// ADR 0061 §1: a round trip of parts, settings and regions, ordered by (order, id) whatever
    /// order the keys were written in.
    #[test]
    fn a_mask_reads_back_as_written_in_order_of_order_then_id() {
        let (_doc, map) = meta();
        write_part(&map, "7@2", 1, true);
        write_part(&map, FIRST_TRACE_PART, 0, false);
        write_part(&map, "3@9", 1, false);
        let settings: TracePartSettings =
            serde_json::from_value(json!({"smooth": 40, "threshold": null})).unwrap();
        write_settings(&map, "7@2", &settings);
        write_region(&map, "0", "5@1", 2, &rect(false, (0.0, 0.0), (0.5, 1.0)));
        write_region(&map, "0", "4@1", 2, &rect(true, (0.1, 0.1), (0.2, 0.2)));
        write_region(&map, "0", "9@1", 1, &rect(false, (0.3, 0.3), (0.4, 0.4)));
        let mask = read_trace_mask(&map);
        let ids: Vec<&str> = mask.parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["0", "3@9", "7@2"]);
        assert!(mask.parts[2].outside);
        assert_eq!(mask.parts[2].settings, settings);
        let regions: Vec<&str> = mask.parts[0]
            .regions
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(regions, ["9@1", "4@1", "5@1"]);
        assert!(mask.parts[0].regions[1].subtract);
        assert_eq!(
            mask.parts[0].regions[2].points,
            vec![UnitPoint::new(0.0, 0.0), UnitPoint::new(0.5, 1.0)]
        );
    }

    /// ADR 0061 §7: each kind of damage is read as stated and nothing is removed on read.
    #[test]
    fn damaged_keys_read_as_the_adr_says_and_stay() {
        let (_doc, map) = meta();
        write_part(&map, "0", 0, true);
        insert(&map, "trace_part.1@1", &json!("not a map"));
        insert(
            &map,
            "trace_part.2@1",
            &json!({"order": 1.5, "outside": true}),
        );
        let bad = [
            json!({"order": 0, "kind": "lasso", "subtract": false, "points": [0.0, 0.0, 1.0, 1.0]}),
            json!({"order": 0, "kind": "rect", "subtract": false, "points": [0.0, 0.0, 1001.0, 1.0]}),
            json!({"order": 0, "kind": "ellipse", "subtract": false, "points": [0.0, 0.0, 1.0, 1.0]}),
            json!({"order": 0, "kind": "rect", "subtract": false, "points": "0 0 1 1"}),
            json!({"order": 0.5, "kind": "rect", "subtract": false, "points": [0.0, 0.0, 1.0, 1.0]}),
            json!({"order": 0, "kind": "rect", "subtract": 1, "points": [0.0, 0.0, 1.0, 1.0]}),
            json!({"order": 0, "kind": "rect", "subtract": false, "points": [0.0, 0.0, 1.0]}),
            json!(12),
        ];
        for (n, value) in bad.into_iter().enumerate() {
            insert(&map, &format!("trace_region.0.{n}@3"), &value);
        }
        map.insert(
            "trace_region.0.nan@3",
            LoroValue::Map(
                [
                    ("order".to_owned(), LoroValue::I64(0)),
                    ("kind".to_owned(), LoroValue::from("rect")),
                    ("subtract".to_owned(), LoroValue::Bool(false)),
                    (
                        "points".to_owned(),
                        LoroValue::from(vec![
                            LoroValue::Double(f64::NAN),
                            LoroValue::Double(0.0),
                            LoroValue::Double(1.0),
                            LoroValue::Double(1.0),
                        ]),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>()
                .into(),
            ),
        )
        .unwrap();
        write_region(&map, "0", "ok@1", 3, &rect(false, (0.0, 0.0), (1.0, 1.0)));
        write_region(&map, "gone", "d@1", 0, &rect(false, (0.0, 0.0), (1.0, 1.0)));
        insert(&map, "trace_settings.gone", &json!({"smooth": 1}));
        insert(&map, "trace_future.thing", &json!(true));
        let keys_before = map.len();
        let mask = read_trace_mask(&map);
        assert_eq!(map.len(), keys_before, "a read removes nothing");
        let ids: Vec<(&str, i64, bool)> = mask
            .parts
            .iter()
            .map(|p| (p.id.as_str(), p.order, p.outside))
            .collect();
        assert_eq!(ids, [("0", 0, true), ("1@1", 0, false), ("2@1", 0, false)]);
        let regions: Vec<&str> = mask.parts[0]
            .regions
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(regions, ["ok@1"], "only the region that reads");
        let mut dangling = dangling_keys(&map);
        dangling.sort();
        assert_eq!(dangling, ["trace_region.gone.d@1", "trace_settings.gone"]);
        remove_all(&map);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn the_highest_orders_count_dangling_regions_and_every_part_key() {
        let (_doc, map) = meta();
        assert_eq!(highest_orders(&map), (None, None));
        write_part(&map, "0", 4, false);
        write_region(&map, "x", "r@1", 9, &rect(false, (0.0, 0.0), (1.0, 1.0)));
        assert_eq!(highest_orders(&map), (Some(4), Some(9)));
        assert_eq!(
            find_region_key(&map, "r@1").as_deref(),
            Some("trace_region.x.r@1")
        );
        assert_eq!(region_keys_of(&map, "x"), ["trace_region.x.r@1"]);
    }
}
