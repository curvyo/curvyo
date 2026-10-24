//! The stored shape of a text on path (ADR 0022 §1, `specs/0080-text-on-path/adrs.md` decision 1),
//! read and written here.
//!
//! ```text
//! meta map of a text on path (format 22), every key absent on a straight text:
//!   text_carrier        { "node": "<peer>:<counter>", "outline": 0 }   one atomic value
//!   text_path_position  f64 mm, arc length of the text's middle         written with text_carrier
//!   text_path_side      "below"                                         absent: above
//!   text_path_line      "top"                                           absent: baseline
//!   text_path_distance  f64 mm                                          absent: 0
//!   text_path_mirror    true                                            absent: false
//!   text_carrier_kept   { "closed": bool, "anchors": [[x, y, in_x, in_y, out_x, out_y], ...] }
//!                       document mm, one atomic value, written when the carrier is removed
//! ```
//!
//! A default is never written. Read rules (criterion 35): an unknown side or line code is the
//! default; a missing or non-finite position or distance reads the text as straight; an outline
//! index past the carrier's outlines is its last outline (`text_carrier`); a kept outline that is
//! not well formed is ignored.

use std::collections::HashMap;

use loro::{LoroMap, LoroValue, TreeID, ValueOrContainer};

use crate::history::{decode_node, encode_node};
use crate::path_codec::as_f64;
use crate::path_model::{AnchorKind, NodeId};
use crate::primitive_outline::OutlineAnchor;
use crate::text_path_model::{
    CarrierCurve, CarrierRef, LineOnPath, OnPathValues, PathSide, TextOnPath,
};
use crate::units::{Length, Point, Vec2};

const KEY_CARRIER: &str = "text_carrier";
const KEY_POSITION: &str = "text_path_position";
const KEY_SIDE: &str = "text_path_side";
const KEY_LINE: &str = "text_path_line";
const KEY_DISTANCE: &str = "text_path_distance";
const KEY_MIRROR: &str = "text_path_mirror";
const KEY_KEPT: &str = "text_carrier_kept";
const KEY_NODE: &str = "node";
const KEY_OUTLINE: &str = "outline";
const KEY_CLOSED: &str = "closed";
const KEY_ANCHORS: &str = "anchors";
const SIDE_BELOW: &str = "below";
const LINE_TOP: &str = "top";
/// Two lengths closer than this (mm) are the same stored value.
const SAME_VALUE_MM: f64 = 1e-12;

/// Every key of a text on path, for Release from path (it deletes them all).
const KEYS: [&str; 7] = [
    KEY_CARRIER,
    KEY_POSITION,
    KEY_SIDE,
    KEY_LINE,
    KEY_DISTANCE,
    KEY_MIRROR,
    KEY_KEPT,
];

fn put(meta: &LoroMap, key: &str, value: Option<LoroValue>) -> bool {
    let present = meta.get(key).is_some();
    // invariant: inserting or deleting a plain value under a known key on an attached map cannot
    // fail.
    #[allow(clippy::unwrap_used)]
    match value {
        Some(value) => meta.insert(key, value).unwrap(),
        None if present => meta.delete(key).unwrap(),
        None => return false,
    }
    true
}

fn number(meta: &LoroMap, key: &str) -> Option<f64> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(value)) => as_f64(&value),
        _ => None,
    }
}

fn code(meta: &LoroMap, key: &str) -> Option<String> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(code))) => Some(code.to_string()),
        _ => None,
    }
}

/// Writes the carrier link as one value.
pub(crate) fn write_carrier(meta: &LoroMap, carrier: CarrierRef) {
    let value: HashMap<String, LoroValue> = HashMap::from([
        (
            KEY_NODE.to_owned(),
            LoroValue::from(encode_node(TreeID::new(
                carrier.node.peer,
                carrier.node.counter,
            ))),
        ),
        (
            KEY_OUTLINE.to_owned(),
            LoroValue::I64(i64::from(carrier.outline)),
        ),
    ]);
    put(meta, KEY_CARRIER, Some(LoroValue::from(value)));
}

/// The carrier link as stored, or `None`.
pub(crate) fn read_carrier(meta: &LoroMap) -> Option<CarrierRef> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_CARRIER) else {
        return None;
    };
    let LoroValue::String(node) = map.get(KEY_NODE)? else {
        return None;
    };
    let node = decode_node(node)?;
    let outline = match map.get(KEY_OUTLINE) {
        Some(LoroValue::I64(index)) => u32::try_from(*index).unwrap_or(u32::MAX),
        _ => 0,
    };
    Some(CarrierRef {
        node: NodeId::from_parts(node.peer, node.counter),
        outline,
    })
}

/// Writes the placement values that differ from the stored ones; a default deletes its key.
/// Returns whether anything was written. The caller commits.
pub(crate) fn write_values(meta: &LoroMap, values: &OnPathValues) -> bool {
    let stored = read_values(meta);
    let mut wrote = false;
    let same = |a: Length, b: Length| (a.as_mm() - b.as_mm()).abs() <= SAME_VALUE_MM;
    if stored.is_none_or(|s| !same(s.position, values.position)) {
        wrote |= put(
            meta,
            KEY_POSITION,
            Some(LoroValue::Double(values.position.as_mm())),
        );
    }
    let stored = stored.unwrap_or(OnPathValues::at(values.position));
    if stored.side != values.side {
        wrote |= put(
            meta,
            KEY_SIDE,
            (values.side == PathSide::Below).then(|| LoroValue::from(SIDE_BELOW)),
        );
    }
    if stored.line != values.line {
        wrote |= put(
            meta,
            KEY_LINE,
            (values.line == LineOnPath::TopEdge).then(|| LoroValue::from(LINE_TOP)),
        );
    }
    if !same(stored.distance, values.distance) {
        let mm = values.distance.as_mm();
        wrote |= put(
            meta,
            KEY_DISTANCE,
            (mm.abs() > SAME_VALUE_MM).then_some(LoroValue::Double(mm)),
        );
    }
    if stored.mirror_letters != values.mirror_letters {
        wrote |= put(
            meta,
            KEY_MIRROR,
            values.mirror_letters.then_some(LoroValue::Bool(true)),
        );
    }
    wrote
}

/// The placement values as stored, or `None` when the position or the distance is missing or
/// not finite (the text then reads straight, criterion 35).
pub(crate) fn read_values(meta: &LoroMap) -> Option<OnPathValues> {
    let position = number(meta, KEY_POSITION).filter(|mm| mm.is_finite())?;
    let distance = match meta.get(KEY_DISTANCE) {
        None => 0.0,
        Some(_) => number(meta, KEY_DISTANCE).filter(|mm| mm.is_finite())?,
    };
    Some(OnPathValues {
        side: if code(meta, KEY_SIDE).as_deref() == Some(SIDE_BELOW) {
            PathSide::Below
        } else {
            PathSide::Above
        },
        line: if code(meta, KEY_LINE).as_deref() == Some(LINE_TOP) {
            LineOnPath::TopEdge
        } else {
            LineOnPath::Baseline
        },
        position: Length::from_mm(position),
        distance: Length::from_mm(distance),
        mirror_letters: matches!(
            meta.get(KEY_MIRROR),
            Some(ValueOrContainer::Value(LoroValue::Bool(true)))
        ),
    })
}

/// Writes the kept copy of a removed carrier's outline (criterion 25).
pub(crate) fn write_kept(meta: &LoroMap, curve: &CarrierCurve) {
    let anchors: Vec<LoroValue> = curve
        .anchors
        .iter()
        .map(|a| {
            LoroValue::from(vec![
                a.point.x,
                a.point.y,
                a.handle_in.x,
                a.handle_in.y,
                a.handle_out.x,
                a.handle_out.y,
            ])
        })
        .collect();
    let value: HashMap<String, LoroValue> = HashMap::from([
        (KEY_CLOSED.to_owned(), LoroValue::Bool(curve.closed)),
        (KEY_ANCHORS.to_owned(), LoroValue::from(anchors)),
    ]);
    put(meta, KEY_KEPT, Some(LoroValue::from(value)));
}

/// The kept copy as stored, or `None` when absent or not well formed: not a map, no anchors, an
/// anchor that is not six finite numbers.
pub(crate) fn read_kept(meta: &LoroMap) -> Option<CarrierCurve> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_KEPT) else {
        return None;
    };
    let LoroValue::Bool(closed) = map.get(KEY_CLOSED)? else {
        return None;
    };
    let LoroValue::List(list) = map.get(KEY_ANCHORS)? else {
        return None;
    };
    let mut anchors = Vec::with_capacity(list.len());
    for entry in list.iter() {
        let LoroValue::List(numbers) = entry else {
            return None;
        };
        if numbers.len() != 6 {
            return None;
        }
        let mut n = [0.0; 6];
        for (slot, value) in n.iter_mut().zip(numbers.iter()) {
            *slot = as_f64(value).filter(|v| v.is_finite())?;
        }
        anchors.push(OutlineAnchor {
            point: Point::new(n[0], n[1]),
            handle_in: Vec2::new(n[2], n[3]),
            handle_out: Vec2::new(n[4], n[5]),
            kind: AnchorKind::Corner,
        });
    }
    (!anchors.is_empty()).then_some(CarrierCurve {
        closed: *closed,
        anchors,
        ellipse: None,
    })
}

/// The on-path part of a text as stored, the outline not yet resolved; `None` for a straight
/// text and for one whose values cannot be read (criterion 35).
pub(crate) fn read_on_path(meta: &LoroMap) -> Option<TextOnPath> {
    let carrier = read_carrier(meta)?;
    let values = read_values(meta)?;
    Some(TextOnPath {
        carrier,
        values,
        curve: None,
        kept: false,
        kept_curve: read_kept(meta).map(std::sync::Arc::new),
    })
}

/// Deletes every on-path key (Release from path). Returns whether anything was deleted.
pub(crate) fn delete_all(meta: &LoroMap) -> bool {
    let mut wrote = false;
    for key in KEYS {
        wrote |= put(meta, key, None);
    }
    wrote
}

/// Writes every on-path key of `on_path` on a new node (a copy): the link, the values and the
/// kept copy.
pub(crate) fn write_on_path(meta: &LoroMap, on_path: &TextOnPath) {
    write_carrier(meta, on_path.carrier);
    write_values(meta, &on_path.values);
    if let Some(kept) = &on_path.kept_curve {
        write_kept(meta, kept);
    }
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;

    use super::*;

    fn meta() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let meta = doc.get_map("meta");
        (doc, meta)
    }

    fn keys(meta: &LoroMap) -> Vec<String> {
        let mut keys: Vec<String> = meta.keys().map(|k| k.to_string()).collect();
        keys.sort();
        keys
    }

    /// Criterion 34: the values round-trip; defaults are not written.
    #[test]
    fn the_keys_round_trip_and_defaults_are_absent() {
        let (_doc, meta) = meta();
        let carrier = CarrierRef {
            node: NodeId::from_parts(7, 3),
            outline: 2,
        };
        write_carrier(&meta, carrier);
        assert!(write_values(
            &meta,
            &OnPathValues::at(Length::from_mm(12.5))
        ));
        assert_eq!(keys(&meta), vec![KEY_CARRIER, KEY_POSITION]);
        let read = read_on_path(&meta).unwrap();
        assert_eq!(read.carrier, carrier);
        assert_eq!(read.values, OnPathValues::at(Length::from_mm(12.5)));
        let all = OnPathValues {
            side: PathSide::Below,
            line: LineOnPath::TopEdge,
            position: Length::from_mm(-3.0),
            distance: Length::from_mm(-1.5),
            mirror_letters: true,
        };
        assert!(write_values(&meta, &all));
        assert_eq!(read_values(&meta), Some(all));
        assert!(!write_values(&meta, &all), "unchanged writes nothing");
        assert!(write_values(
            &meta,
            &OnPathValues::at(Length::from_mm(-3.0))
        ));
        assert_eq!(keys(&meta), vec![KEY_CARRIER, KEY_POSITION]);
        assert!(delete_all(&meta));
        assert_eq!(keys(&meta), Vec::<String>::new());
    }

    /// Criterion 35: unknown codes are defaults; a non-finite position reads the text straight;
    /// a malformed kept copy is ignored.
    #[test]
    fn damaged_keys_read_leniently() {
        let (_doc, meta) = meta();
        write_carrier(
            &meta,
            CarrierRef {
                node: NodeId::from_parts(1, 1),
                outline: 0,
            },
        );
        meta.insert(KEY_POSITION, 5.0).unwrap();
        meta.insert(KEY_SIDE, "sideways").unwrap();
        meta.insert(KEY_LINE, 3.0).unwrap();
        let read = read_values(&meta).unwrap();
        assert_eq!(
            (read.side, read.line),
            (PathSide::Above, LineOnPath::Baseline)
        );
        meta.insert(KEY_KEPT, "not a map").unwrap();
        assert_eq!(read_kept(&meta), None);
        meta.insert(KEY_DISTANCE, f64::INFINITY).unwrap();
        assert_eq!(read_on_path(&meta), None);
        meta.insert(KEY_DISTANCE, 1.0).unwrap();
        meta.insert(KEY_POSITION, f64::NAN).unwrap();
        assert_eq!(read_on_path(&meta), None);
        meta.insert(KEY_CARRIER, "1:1").unwrap();
        meta.insert(KEY_POSITION, 1.0).unwrap();
        assert_eq!(read_on_path(&meta), None, "a carrier that is not a map");
    }

    #[test]
    fn the_kept_copy_round_trips() {
        let (_doc, meta) = meta();
        let curve = CarrierCurve {
            closed: true,
            anchors: vec![
                OutlineAnchor {
                    point: Point::new(1.0, 2.0),
                    handle_in: Vec2::new(0.5, 0.0),
                    handle_out: Vec2::new(-0.5, 0.0),
                    kind: AnchorKind::Corner,
                },
                OutlineAnchor {
                    point: Point::new(10.0, 2.0),
                    handle_in: Vec2::ZERO,
                    handle_out: Vec2::ZERO,
                    kind: AnchorKind::Corner,
                },
            ],
            ellipse: None,
        };
        write_kept(&meta, &curve);
        assert_eq!(read_kept(&meta), Some(curve));
    }
}
