//! The stored shape of a generated object (ADR 0080 §4, `specs/0141-parametric-generators`): the
//! leaf `shape = "generated"` with its two registers, read, written and validated here.
//!
//! ```text
//! meta map of a generated object (format 54):
//!   shape            : "generated"
//!   generated        : ONE map value, written whole by create, a parameter edit and a scale:
//!     kind           : "<extension id>/<kind>"            e.g. "curvyo.generators/gear"
//!     seed           : 0 ..= 4294967295
//!     params         : { key: { "mm": n } | { "deg": n } | { "count": n } | { "on": b } | { "choice": "id" } | { "path": [x0, y0, c1x, c1y, c2x, c2y, x, y, ...] } }
//!     parts          : [ part, ... ]   one part per piece (a gear one, a box one per plate)
//!       part         : [ subpath, ... ]   the outline and its holes
//!       subpath      : [x0, y0, c1x, c1y, c2x, c2y, x, y, ...]   a start point, then cubics
//!   generated_frame  : [a, b, c, d, e, f]   rotation, mirror and translation only
//!   + style keys as on a path, `name`, `visible`, `locked`, `opacity`
//! ```
//!
//! The parameters are opaque to this crate: the extension reads them (a bad value reads as its
//! default, shown, never rewritten on read, ADR 0033 §3). The object reads as one compound path,
//! `frame x parts`, in every build, so a build that does not know the kind still draws, cuts,
//! moves, rotates, copies and deletes it from the stored outline.
//!
//! Damage (refused at open): `generated` not a map; a `kind` that is not `<id>/<kind>`; a seed
//! outside `u32`; `params` not a map; no part or an empty part; a subpath that is not `2 + 6n`
//! finite numbers within ±1e7 (n at least 1); a frame that is not six finite numbers or holds
//! scale or shear beyond 1e-9.

use std::collections::HashMap;

use loro::{LoroMap, LoroValue, ValueOrContainer};
use serde_json::Value;

use crate::affine::Affine;
use crate::generated_outline::subpath_anchors;
use crate::path_codec::as_f64;
use crate::path_model::{NodeId, SubpathSnapshot};
use crate::units::Vec2;

/// The `shape` tag of a generated node.
pub(crate) const SHAPE_GENERATED: &str = "generated";
/// The atomic record register.
pub(crate) const KEY_GENERATED: &str = "generated";
/// The frame register.
pub(crate) const KEY_FRAME: &str = "generated_frame";

/// Every key a generated node carries besides the style; "Convert to paths" strips them.
pub(crate) const ALL_GENERATED_KEYS: [&str; 3] =
    [crate::shape_codec::KEY_SHAPE, KEY_GENERATED, KEY_FRAME];

/// The largest coordinate a stored outline may hold (±1e7 mm, the matrix limit).
const COORDINATE_LIMIT: f64 = 1e7;
/// How far from a unit vector a frame column may be.
const FRAME_TOLERANCE: f64 = 1e-9;

/// What the extension generated and the host stores: kind, seed, parameters and the outline,
/// frame-local.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedRecord {
    /// `<extension id>/<kind>`.
    pub kind: String,
    /// The seed, set once at creation (ADR 0029 §5).
    pub seed: u32,
    /// The stored parameter values, opaque here.
    pub params: serde_json::Map<String, Value>,
    /// Parts, each a list of subpaths of `2 + 6n` numbers (start, then `c1 c2 to` per cubic).
    pub parts: Vec<Vec<Vec<f64>>>,
}

/// A generated object as stored: its record and frame.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedSnapshot {
    /// Persistent object identity.
    pub id: NodeId,
    /// The record.
    pub record: GeneratedRecord,
    /// Frame-local to document placement: rotation, mirror and translation.
    pub frame: Affine,
}

impl GeneratedSnapshot {
    /// Every part as its subpaths with the frame applied, closed, with synthesised anchor ids
    /// that were never written: what "Convert to paths" replaces ids on (ADR 0080 §6).
    #[must_use]
    pub fn outlines(&self) -> Vec<Vec<SubpathSnapshot>> {
        let mut ordinal = 0;
        self.record
            .parts
            .iter()
            .map(|part| {
                part.iter()
                    .map(|subpath| {
                        ordinal += 1;
                        SubpathSnapshot {
                            closed: true,
                            anchors: subpath_anchors(subpath, self.frame, ordinal),
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// Whether `meta` is a generated node's.
pub(crate) fn is_generated_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_GENERATED
    )
}

fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

fn json_to_loro(value: &Value) -> LoroValue {
    match value {
        Value::Null => LoroValue::Null,
        Value::Bool(flag) => LoroValue::Bool(*flag),
        Value::Number(number) => number.as_i64().map_or_else(
            || LoroValue::Double(number.as_f64().unwrap_or(0.0)),
            LoroValue::I64,
        ),
        Value::String(text) => LoroValue::String(text.as_str().into()),
        Value::Array(items) => {
            LoroValue::List(items.iter().map(json_to_loro).collect::<Vec<_>>().into())
        }
        Value::Object(map) => LoroValue::Map(
            map.iter()
                .map(|(key, value)| (key.clone(), json_to_loro(value)))
                .collect::<HashMap<String, LoroValue>>()
                .into(),
        ),
    }
}

fn loro_to_json(value: &LoroValue) -> Value {
    match value {
        LoroValue::Bool(flag) => Value::Bool(*flag),
        LoroValue::I64(n) => Value::from(*n),
        LoroValue::Double(n) => Value::from(*n),
        LoroValue::String(text) => Value::String(text.to_string()),
        LoroValue::List(items) => Value::Array(items.iter().map(loro_to_json).collect()),
        LoroValue::Map(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), loro_to_json(value)))
                .collect(),
        ),
        _ => Value::Null,
    }
}

fn numbers(values: &[f64]) -> LoroValue {
    LoroValue::List(
        values
            .iter()
            .map(|value| LoroValue::Double(*value))
            .collect::<Vec<_>>()
            .into(),
    )
}

/// The record as one map value.
fn record_value(record: &GeneratedRecord) -> LoroValue {
    let parts: Vec<LoroValue> = record
        .parts
        .iter()
        .map(|part| {
            LoroValue::List(
                part.iter()
                    .map(|subpath| numbers(subpath))
                    .collect::<Vec<_>>()
                    .into(),
            )
        })
        .collect();
    let mut map: HashMap<String, LoroValue> = HashMap::new();
    map.insert(
        "kind".to_owned(),
        LoroValue::String(record.kind.as_str().into()),
    );
    map.insert("seed".to_owned(), LoroValue::I64(i64::from(record.seed)));
    map.insert(
        "params".to_owned(),
        json_to_loro(&Value::Object(record.params.clone())),
    );
    map.insert("parts".to_owned(), LoroValue::List(parts.into()));
    LoroValue::Map(map.into())
}

/// Writes the record register whole.
pub(crate) fn write_record(meta: &LoroMap, record: &GeneratedRecord) {
    insert(meta, KEY_GENERATED, record_value(record));
}

/// Writes the frame register.
pub(crate) fn write_frame(meta: &LoroMap, frame: Affine) {
    insert(meta, KEY_FRAME, frame.to_array().to_vec());
}

/// Writes a new generated node's tag, record and frame.
pub(crate) fn write_generated(meta: &LoroMap, record: &GeneratedRecord, frame: Affine) {
    crate::shape_codec::write_shape_tag(meta, SHAPE_GENERATED);
    write_record(meta, record);
    write_frame(meta, frame);
}

/// Removes every generated register, including the `shape` tag. Uncommitted.
pub(crate) fn strip(meta: &LoroMap) {
    for key in ALL_GENERATED_KEYS {
        if meta.get(key).is_some() {
            // invariant: deleting a key just confirmed present on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(key).unwrap();
        }
    }
}

/// Whether a frame is a rotation, mirror and translation (columns orthonormal).
#[must_use]
pub(crate) fn frame_is_valid(frame: Affine) -> bool {
    frame.is_within_limit()
        && (frame.a * frame.a + frame.b * frame.b - 1.0).abs() <= FRAME_TOLERANCE
        && (frame.c * frame.c + frame.d * frame.d - 1.0).abs() <= FRAME_TOLERANCE
        && (frame.a * frame.c + frame.b * frame.d).abs() <= FRAME_TOLERANCE
}

/// `frame` made exactly orthonormal again: the first column normalised, the second turned
/// square to it on the side the mirror state says. Written frames always pass this, so a long
/// chain of turns never drifts out of the limit the reader enforces.
#[must_use]
pub fn orthonormalised(frame: Affine) -> Affine {
    let length = frame.a.hypot(frame.b);
    if length < f64::EPSILON {
        return Affine::new(1.0, 0.0, 0.0, 1.0, frame.e, frame.f);
    }
    let (a, b) = (frame.a / length, frame.b / length);
    let sign = if frame.determinant() < 0.0 { -1.0 } else { 1.0 };
    Affine::new(a, b, -sign * b, sign * a, frame.e, frame.f)
}

fn value_of(meta: &LoroMap, key: &str) -> Option<LoroValue> {
    Some(meta.get(key)?.get_deep_value())
}

/// The frame register when it holds six finite numbers that form a valid frame.
pub(crate) fn read_frame(meta: &LoroMap) -> Option<Affine> {
    let LoroValue::List(list) = value_of(meta, KEY_FRAME)? else {
        return None;
    };
    if list.len() != 6 {
        return None;
    }
    let mut entries = [0.0; 6];
    for (entry, value) in entries.iter_mut().zip(list.iter()) {
        *entry = as_f64(value)?;
    }
    let frame = Affine::from_array(entries);
    frame_is_valid(frame).then_some(frame)
}

fn subpath_from(value: &LoroValue) -> Option<Vec<f64>> {
    let LoroValue::List(list) = value else {
        return None;
    };
    if list.len() < 8 || (list.len() - 2) % 6 != 0 {
        return None;
    }
    list.iter()
        .map(|entry| as_f64(entry).filter(|n| n.is_finite() && n.abs() <= COORDINATE_LIMIT))
        .collect()
}

fn part_from(value: &LoroValue) -> Option<Vec<Vec<f64>>> {
    let LoroValue::List(list) = value else {
        return None;
    };
    if list.is_empty() {
        return None;
    }
    list.iter().map(subpath_from).collect()
}

/// The record register when every rule of the module header holds.
pub(crate) fn read_record(meta: &LoroMap) -> Option<GeneratedRecord> {
    let LoroValue::Map(map) = value_of(meta, KEY_GENERATED)? else {
        return None;
    };
    let LoroValue::String(kind) = map.get("kind")? else {
        return None;
    };
    let kind = kind.to_string();
    match kind.split_once('/') {
        Some((extension, name)) if !extension.is_empty() && !name.is_empty() => {}
        _ => return None,
    }
    let LoroValue::I64(seed) = map.get("seed")? else {
        return None;
    };
    let seed = u32::try_from(*seed).ok()?;
    let Value::Object(params) = loro_to_json(map.get("params")?) else {
        return None;
    };
    let LoroValue::List(parts) = map.get("parts")? else {
        return None;
    };
    if parts.is_empty() {
        return None;
    }
    let parts = parts.iter().map(part_from).collect::<Option<Vec<_>>>()?;
    Some(GeneratedRecord {
        kind,
        seed,
        params,
        parts,
    })
}

/// The snapshot of the generated node `id`, `None` when a register cannot be read.
pub(crate) fn read_snapshot(id: NodeId, meta: &LoroMap) -> Option<GeneratedSnapshot> {
    Some(GeneratedSnapshot {
        id,
        record: read_record(meta)?,
        frame: read_frame(meta)?,
    })
}

/// Whether a generated node's registers can be read (open-file validation).
pub(crate) fn validate_generated_node(meta: &LoroMap) -> bool {
    read_record(meta).is_some()
        && read_frame(meta).is_some()
        && crate::style_validation::style_is_valid(meta)
}

/// Moves a generated node's frame by `offset` in place (paste, duplicate).
pub(crate) fn translate_generated_meta(meta: &LoroMap, offset: Vec2) {
    if let Some(frame) = read_frame(meta) {
        write_frame(
            meta,
            orthonormalised(Affine::translation(offset).compose(frame)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_outline::outline;

    fn square() -> Vec<f64> {
        vec![
            0.0, 0.0, // start
            0.0, 0.0, 4.0, 0.0, 4.0, 0.0, // line to (4, 0)
            4.0, 0.0, 4.0, 4.0, 4.0, 4.0, // line to (4, 4)
            4.0, 4.0, 0.0, 4.0, 0.0, 4.0, // line to (0, 4)
            0.0, 4.0, 0.0, 0.0, 0.0, 0.0, // back to the start
        ]
    }

    fn record() -> GeneratedRecord {
        let mut params = serde_json::Map::new();
        params.insert("module".into(), serde_json::json!({"mm": 2.0}));
        params.insert("teeth".into(), serde_json::json!({"count": 24}));
        GeneratedRecord {
            kind: "curvyo.generators/test".into(),
            seed: 7,
            params,
            parts: vec![vec![square()]],
        }
    }

    #[test]
    fn a_record_and_its_frame_round_trip_through_the_meta_map() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("meta");
        let frame = Affine::translation(Vec2::new(3.0, -2.0));
        write_generated(&meta, &record(), frame);
        assert!(is_generated_meta(&meta));
        let read = read_snapshot(NodeId::from_parts(1, 1), &meta).unwrap();
        assert_eq!(read.record, record());
        assert_eq!(read.frame, frame);
        assert!(validate_generated_node(&meta));
    }

    #[test]
    fn the_outline_is_the_frame_applied_to_the_parts_with_closed_anchors() {
        let snapshot = GeneratedSnapshot {
            id: NodeId::from_parts(1, 1),
            record: record(),
            frame: Affine::translation(Vec2::new(10.0, 0.0)),
        };
        let path = outline(&snapshot, crate::style_model::Style::default());
        assert!(path.closed && path.extra_subpaths.is_empty());
        let points: Vec<(f64, f64)> = path
            .anchors
            .iter()
            .map(|a| (a.point.x, a.point.y))
            .collect();
        assert_eq!(
            points,
            vec![(10.0, 0.0), (14.0, 0.0), (14.0, 4.0), (10.0, 4.0)]
        );
    }

    #[test]
    fn damage_is_detected() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("meta");
        let frame = Affine::IDENTITY;
        let mut bad = record();
        bad.kind = "gear".into();
        write_generated(&meta, &bad, frame);
        assert!(!validate_generated_node(&meta));
        let mut bad = record();
        bad.parts = vec![vec![vec![0.0, 0.0, 1.0]]];
        write_generated(&meta, &bad, frame);
        assert!(!validate_generated_node(&meta));
        let mut bad = record();
        bad.parts = vec![vec![]];
        write_generated(&meta, &bad, frame);
        assert!(!validate_generated_node(&meta));
        write_generated(&meta, &record(), Affine::scale(2.0, 2.0));
        assert!(!validate_generated_node(&meta));
        write_generated(&meta, &record(), frame);
        assert!(validate_generated_node(&meta));
    }

    #[test]
    fn a_frame_is_made_exactly_orthonormal_again() {
        let drifted = Affine::new(1.0 + 1e-12, 1e-11, -1e-11, 1.0, 5.0, 6.0);
        let fixed = orthonormalised(drifted);
        assert!(frame_is_valid(fixed));
        let mirrored = orthonormalised(Affine::new(1.0, 0.0, 0.0, -1.0 + 1e-12, 0.0, 0.0));
        assert!(frame_is_valid(mirrored) && mirrored.determinant() < 0.0);
    }
}
