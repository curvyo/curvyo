//! The stored shape of a dimension object (ADR 0027 §2 and §8), read, written and validated here.
//!
//! ```text
//! meta map of a dimension (format 31):
//!   shape            : "dimension"
//!   dimension_kind   : "aligned" | "horizontal" | "vertical" | "angle" | "radius" | "diameter"
//!   dimension_ref_a  : one slot, a list:
//!                        ["anchor", "<peer>:<counter>", "<anchor hex>"]
//!                        ["segment_mid", "<peer>:<counter>", "<anchor hex>"]
//!                        ["shape_point", "<peer>:<counter>", "center" | "corner" | "vertex"
//!                         | "edge_mid", i]
//!                        ["free", x, y]
//!                        ["circle", "<peer>:<counter>"]
//!                        ["arc", "<peer>:<counter>", "<anchor hex>"]
//!                        ["round_corner", "<peer>:<counter>", i]
//!                        ["free_circle", centre x, centre y, radius mm > 0]
//!   dimension_ref_b  : a point slot (distance and angle kinds)
//!   dimension_ref_c  : a point slot (angle kind only)
//!   dimension_place  : [offset mm, along fraction, leader rad, sector 0..3]
//!   dimension_kept   : [value (mm or rad), x0, y0, x1, y1(, x2, y2)]
//!   + `name`, `visible`, `locked`; no style keys, no `guide` (a guide by kind)
//! ```
//!
//! Damage (refused at open): an unknown kind, a slot count that does not fit the kind, a
//! malformed slot, a place or kept value that is not finite or of the wrong length.

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::dimension_model::{
    CurveSlot, DimensionKind, DimensionPlace, DimensionSlots, DimensionSnapshot, DimensionValue,
    Measured, PointSlot, ShapeFeature,
};
use crate::history::{decode_node, encode_node};
use crate::path_codec::as_f64;
use crate::path_model::{AnchorId, NodeId};
use crate::units::{Angle, Length, Point};

/// The `shape` tag of a dimension node.
pub(crate) const SHAPE_DIMENSION: &str = "dimension";
const KEY_KIND: &str = "dimension_kind";
const KEY_REFS: [&str; 3] = ["dimension_ref_a", "dimension_ref_b", "dimension_ref_c"];
const KEY_PLACE: &str = "dimension_place";
const KEY_KEPT: &str = "dimension_kept";

/// Whether `meta` is a dimension node's.
pub(crate) fn is_dimension_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_DIMENSION
    )
}

fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

fn node_text(node: NodeId) -> LoroValue {
    encode_node(crate::paths::tree_id_of(node)).into()
}

fn point_slot_value(slot: &PointSlot) -> LoroValue {
    let list: Vec<LoroValue> = match *slot {
        PointSlot::Anchor { node, anchor } => {
            vec!["anchor".into(), node_text(node), anchor.to_hex().into()]
        }
        PointSlot::SegmentMid { node, anchor } => {
            vec![
                "segment_mid".into(),
                node_text(node),
                anchor.to_hex().into(),
            ]
        }
        PointSlot::ShapePoint { node, feature } => {
            let (name, index) = match feature {
                ShapeFeature::Center => ("center", 0),
                ShapeFeature::Corner(i) => ("corner", i),
                ShapeFeature::Vertex(i) => ("vertex", i),
                ShapeFeature::EdgeMid(i) => ("edge_mid", i),
            };
            vec![
                "shape_point".into(),
                node_text(node),
                name.into(),
                i64::from(index).into(),
            ]
        }
        PointSlot::Free { point } => vec!["free".into(), point.x.into(), point.y.into()],
    };
    LoroValue::List(list.into())
}

fn curve_slot_value(slot: &CurveSlot) -> LoroValue {
    let list: Vec<LoroValue> = match *slot {
        CurveSlot::Circle { node } => vec!["circle".into(), node_text(node)],
        CurveSlot::Arc { node, anchor } => {
            vec!["arc".into(), node_text(node), anchor.to_hex().into()]
        }
        CurveSlot::RoundCorner { node, corner } => vec![
            "round_corner".into(),
            node_text(node),
            i64::from(corner).into(),
        ],
        CurveSlot::FreeCircle { center, radius } => vec![
            "free_circle".into(),
            center.x.into(),
            center.y.into(),
            radius.as_mm().into(),
        ],
    };
    LoroValue::List(list.into())
}

/// Writes the slot registers of `slots`, removing the ones it does not use.
pub(crate) fn write_slots(meta: &LoroMap, slots: &DimensionSlots) {
    let values: Vec<LoroValue> = match slots {
        DimensionSlots::Points(points) => points.iter().map(point_slot_value).collect(),
        DimensionSlots::Curve(curve) => vec![curve_slot_value(curve)],
    };
    for (i, key) in KEY_REFS.into_iter().enumerate() {
        match values.get(i) {
            Some(value) => insert(meta, key, value.clone()),
            None if meta.get(key).is_some() => {
                // invariant: deleting a present key of an attached map cannot fail.
                #[allow(clippy::unwrap_used)]
                meta.delete(key).unwrap();
            }
            None => {}
        }
    }
}

/// Writes the place register.
pub(crate) fn write_place(meta: &LoroMap, place: DimensionPlace) {
    insert(
        meta,
        KEY_PLACE,
        vec![
            place.offset.as_mm(),
            place.along,
            place.leader.as_radians(),
            f64::from(place.sector),
        ],
    );
}

/// Writes the kept register.
pub(crate) fn write_kept(meta: &LoroMap, kept: &Measured) {
    let mut values = vec![kept.value.raw()];
    for point in &kept.points {
        values.extend([point.x, point.y]);
    }
    insert(meta, KEY_KEPT, values);
}

/// Writes every register of a new dimension node.
pub(crate) fn write_dimension(meta: &LoroMap, dimension: &DimensionSnapshot) {
    crate::shape_codec::write_shape_tag(meta, SHAPE_DIMENSION);
    insert(meta, KEY_KIND, dimension.kind.code());
    write_slots(meta, &dimension.slots);
    write_place(meta, dimension.place);
    write_kept(meta, &dimension.kept);
}

fn list(meta: &LoroMap, key: &str) -> Option<Vec<LoroValue>> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::List(list))) => Some(list.to_vec()),
        _ => None,
    }
}

fn numbers(meta: &LoroMap, key: &str) -> Option<Vec<f64>> {
    list(meta, key)?
        .iter()
        .map(|value| as_f64(value).filter(|n| n.is_finite()))
        .collect()
}

fn text(value: &LoroValue) -> Option<&str> {
    match value {
        LoroValue::String(text) => Some(text.as_str()),
        _ => None,
    }
}

fn node_of(value: &LoroValue) -> Option<NodeId> {
    decode_node(text(value)?).map(|id| NodeId::from_parts(id.peer, id.counter))
}

fn anchor_of(value: &LoroValue) -> Option<AnchorId> {
    AnchorId::from_hex(text(value)?)
}

fn index_of(value: &LoroValue) -> Option<u32> {
    match value {
        LoroValue::I64(n) => u32::try_from(*n).ok(),
        LoroValue::Double(n) if n.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(n) => {
            // invariant: a whole number within the u32 range.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(*n as u32)
        }
        _ => None,
    }
}

fn read_point_slot(value: &[LoroValue]) -> Option<PointSlot> {
    match (text(value.first()?)?, value.len()) {
        ("anchor", 3) => Some(PointSlot::Anchor {
            node: node_of(&value[1])?,
            anchor: anchor_of(&value[2])?,
        }),
        ("segment_mid", 3) => Some(PointSlot::SegmentMid {
            node: node_of(&value[1])?,
            anchor: anchor_of(&value[2])?,
        }),
        ("shape_point", 4) => {
            let index = index_of(&value[3])?;
            let feature = match text(&value[2])? {
                "center" => ShapeFeature::Center,
                "corner" => ShapeFeature::Corner(index),
                "vertex" => ShapeFeature::Vertex(index),
                "edge_mid" => ShapeFeature::EdgeMid(index),
                _ => return None,
            };
            Some(PointSlot::ShapePoint {
                node: node_of(&value[1])?,
                feature,
            })
        }
        ("free", 3) => {
            let (x, y) = (as_f64(&value[1])?, as_f64(&value[2])?);
            (x.is_finite() && y.is_finite()).then_some(PointSlot::Free {
                point: Point::new(x, y),
            })
        }
        _ => None,
    }
}

fn read_curve_slot(value: &[LoroValue]) -> Option<CurveSlot> {
    match (text(value.first()?)?, value.len()) {
        ("circle", 2) => Some(CurveSlot::Circle {
            node: node_of(&value[1])?,
        }),
        ("arc", 3) => Some(CurveSlot::Arc {
            node: node_of(&value[1])?,
            anchor: anchor_of(&value[2])?,
        }),
        ("round_corner", 3) => Some(CurveSlot::RoundCorner {
            node: node_of(&value[1])?,
            corner: index_of(&value[2]).filter(|&i| i < 4)?,
        }),
        ("free_circle", 4) => {
            let (x, y, r) = (as_f64(&value[1])?, as_f64(&value[2])?, as_f64(&value[3])?);
            (x.is_finite() && y.is_finite() && r.is_finite() && r > 0.0).then_some(
                CurveSlot::FreeCircle {
                    center: Point::new(x, y),
                    radius: Length::from_mm(r),
                },
            )
        }
        _ => None,
    }
}

fn read_kind(meta: &LoroMap) -> Option<DimensionKind> {
    match meta.get(KEY_KIND) {
        Some(ValueOrContainer::Value(LoroValue::String(code))) => DimensionKind::from_code(&code),
        _ => None,
    }
}

fn read_slots(meta: &LoroMap, kind: DimensionKind) -> Option<DimensionSlots> {
    let present: Vec<Vec<LoroValue>> = KEY_REFS.iter().map_while(|key| list(meta, key)).collect();
    let stray = KEY_REFS[present.len()..]
        .iter()
        .any(|key| meta.get(key).is_some());
    if stray {
        return None;
    }
    let slots = if kind.is_curve() {
        match present.as_slice() {
            [one] => DimensionSlots::Curve(read_curve_slot(one)?),
            _ => return None,
        }
    } else {
        DimensionSlots::Points(
            present
                .iter()
                .map(|value| read_point_slot(value))
                .collect::<Option<_>>()?,
        )
    };
    slots.fit(kind).then_some(slots)
}

fn read_place(meta: &LoroMap) -> Option<DimensionPlace> {
    let [offset, along, leader, sector] = numbers(meta, KEY_PLACE)?[..] else {
        return None;
    };
    if sector.fract() != 0.0 || !(0.0..4.0).contains(&sector) {
        return None;
    }
    // invariant: a whole number in 0..4.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let place = DimensionPlace {
        offset: Length::from_mm(offset),
        along,
        leader: Angle::from_radians(leader),
        sector: sector as u8,
    };
    place.is_valid().then_some(place)
}

fn read_kept(meta: &LoroMap, kind: DimensionKind) -> Option<Measured> {
    let values = numbers(meta, KEY_KEPT)?;
    let count = if kind == DimensionKind::Angle { 3 } else { 2 };
    if values.len() != 1 + 2 * count {
        return None;
    }
    let value = if kind == DimensionKind::Angle {
        DimensionValue::Angle(Angle::from_radians(values[0]))
    } else {
        DimensionValue::Length(Length::from_mm(values[0]))
    };
    let points = values[1..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[x, y]| Point::new(x, y))
        .collect();
    Some(Measured { points, value })
}

/// The dimension `id` as stored, or `None` when a register cannot be read.
pub(crate) fn read_dimension_snapshot(id: NodeId, meta: &LoroMap) -> Option<DimensionSnapshot> {
    let kind = read_kind(meta)?;
    Some(DimensionSnapshot::new(
        id,
        kind,
        read_slots(meta, kind)?,
        read_place(meta)?,
        read_kept(meta, kind)?,
    ))
}

/// Whether a dimension node's registers can be read (open-file validation).
pub(crate) fn validate_dimension_node(meta: &LoroMap) -> bool {
    read_dimension_snapshot(NodeId::from_parts(0, 0), meta).is_some()
}

/// Moves a dimension's free points, free circle and kept points by `offset` in place (a copy's shift; a move
/// of an unlinked dimension, criterion 13). Linked slots follow their geometry on read.
pub(crate) fn translate_dimension_meta(meta: &LoroMap, offset: crate::units::Vec2) {
    let Some(mut dimension) = read_dimension_snapshot(NodeId::from_parts(0, 0), meta) else {
        return;
    };
    let slots = dimension
        .slots
        .mapped_free(|point| point.translated(offset));
    if slots != dimension.slots {
        write_slots(meta, &slots);
    }
    for point in &mut dimension.kept.points {
        *point = point.translated(offset);
    }
    write_kept(meta, &dimension.kept);
}

/// Writes the free points and the kept points of `moved` (a transform result) where they differ
/// from the stored ones; the kind, the linked slots and the place stay.
pub(crate) fn write_moved(meta: &LoroMap, moved: &DimensionSnapshot) {
    let Some(stored) = read_dimension_snapshot(moved.id, meta) else {
        return;
    };
    if stored.slots != moved.slots && stored.slots.fit(moved.kind) {
        write_slots(meta, &moved.slots);
    }
    if stored.kept != moved.kept {
        write_kept(meta, &moved.kept);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> LoroMap {
        let doc = loro::LoroDoc::new();
        doc.get_map("m")
    }

    fn sample() -> DimensionSnapshot {
        DimensionSnapshot::new(
            NodeId::from_parts(5, 1),
            DimensionKind::Angle,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: NodeId::from_parts(1, 2),
                    anchor: AnchorId::new(3, 4),
                },
                PointSlot::ShapePoint {
                    node: NodeId::from_parts(1, 3),
                    feature: ShapeFeature::EdgeMid(2),
                },
                PointSlot::Free {
                    point: Point::new(1.5, -2.0),
                },
            ]),
            DimensionPlace {
                offset: Length::from_mm(7.0),
                along: 0.25,
                leader: Angle::from_radians(0.5),
                sector: 3,
            },
            Measured {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(1.0, 0.0),
                    Point::new(0.0, 1.0),
                ],
                value: DimensionValue::Angle(Angle::from_radians(1.0)),
            },
        )
    }

    #[test]
    fn a_dimension_round_trips_through_its_registers() {
        let meta = meta();
        let dimension = sample();
        write_dimension(&meta, &dimension);
        assert!(is_dimension_meta(&meta));
        assert_eq!(
            read_dimension_snapshot(dimension.id, &meta),
            Some(dimension)
        );
    }

    #[test]
    fn a_curve_slot_round_trips() {
        let meta = meta();
        let mut dimension = sample();
        dimension.kind = DimensionKind::Radius;
        dimension.slots = DimensionSlots::Curve(CurveSlot::RoundCorner {
            node: NodeId::from_parts(9, 9),
            corner: 3,
        });
        dimension.kept = Measured {
            points: vec![Point::new(0.0, 0.0), Point::new(4.0, 0.0)],
            value: DimensionValue::Length(Length::from_mm(4.0)),
        };
        dimension.shown = dimension.kept.clone();
        write_dimension(&meta, &dimension);
        assert_eq!(
            read_dimension_snapshot(dimension.id, &meta),
            Some(dimension)
        );
    }

    #[test]
    fn damage_is_refused() {
        let damaged = |edit: &dyn Fn(&LoroMap)| {
            let meta = meta();
            write_dimension(&meta, &sample());
            edit(&meta);
            validate_dimension_node(&meta)
        };
        assert!(damaged(&|_| {}));
        assert!(!damaged(&|m| insert(m, KEY_KIND, "ordinate")));
        assert!(!damaged(&|m| insert(m, KEY_KIND, "aligned")), "three slots");
        assert!(!damaged(&|m| insert(
            m,
            KEY_REFS[1],
            LoroValue::List(vec!["anchor".into()].into())
        )));
        assert!(!damaged(&|m| insert(m, KEY_KEPT, vec![f64::NAN; 7])));
        assert!(!damaged(&|m| insert(
            m,
            KEY_PLACE,
            vec![1.0, 0.5, 0.0, 4.0]
        )));
    }

    #[test]
    fn translating_moves_free_and_kept_points_only() {
        let meta = meta();
        write_dimension(&meta, &sample());
        translate_dimension_meta(&meta, crate::units::Vec2::new(10.0, 0.0));
        let moved = read_dimension_snapshot(NodeId::from_parts(5, 1), &meta).unwrap();
        let DimensionSlots::Points(points) = &moved.slots else {
            unreachable!()
        };
        assert_eq!(
            points[2],
            PointSlot::Free {
                point: Point::new(11.5, -2.0)
            }
        );
        assert_eq!(points[0], sample_slots()[0]);
        assert_eq!(moved.kept.points[0], Point::new(10.0, 0.0));
    }

    fn sample_slots() -> Vec<PointSlot> {
        match sample().slots {
            DimensionSlots::Points(points) => points,
            DimensionSlots::Curve(_) => Vec::new(),
        }
    }
}
