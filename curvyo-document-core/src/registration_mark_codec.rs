//! The stored shape of a registration mark and of the marks group (ADR 0081 §1, §2, §9), read,
//! written and validated here.
//!
//! ```text
//! meta map of a mark (format 55):
//!   shape          : "registration_mark"
//!   mark_number    : integer 1..4
//!   mark_centre    : [x, y] document mm, finite
//!   mark_diameter  : mm, 3..20
//!   mark_line      : mm, 0.05..1
//!   + `name`, `visible`, `locked`; no style keys
//! meta map of a marks object: an ordinary group (`group = true`) with
//!   registration_marks : true
//! ```
//!
//! Damage (refused at open): a number outside 1 to 4, a centre that is not two finite numbers, a
//! diameter or line out of range. Two marks of one number, or a group of 1 or 5 marks after a
//! merge, are read as they are (the Print and cut reading reports them).

use loro::{LoroMap, LoroTree, LoroValue, TreeID, ValueOrContainer};

use crate::path_codec::as_f64;
use crate::path_model::NodeId;
use crate::registration_mark_model::{
    MARK_MAX_COUNT, RegistrationMarkSnapshot, mark_diameter_is_valid, mark_line_is_valid,
};
use crate::units::{Length, Point, Vec2};

/// The `shape` tag of a registration mark node.
pub(crate) const SHAPE_REGISTRATION_MARK: &str = "registration_mark";
/// The marker on a group that makes it a marks object.
pub(crate) const KEY_REGISTRATION_MARKS: &str = "registration_marks";
const KEY_NUMBER: &str = "mark_number";
const KEY_CENTRE: &str = "mark_centre";
const KEY_DIAMETER: &str = "mark_diameter";
const KEY_LINE: &str = "mark_line";

/// Whether `meta` is a registration mark node's.
pub(crate) fn is_registration_mark_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_REGISTRATION_MARK
    )
}

/// Whether the node `id` is a group tagged as a marks object.
pub(crate) fn is_marks_group(tree: &LoroTree, id: TreeID) -> bool {
    crate::group_tree::is_group_node(tree, id)
        && tree.get_meta(id).is_ok_and(|meta| {
            matches!(
                meta.get(KEY_REGISTRATION_MARKS),
                Some(ValueOrContainer::Value(LoroValue::Bool(true)))
            )
        })
}

fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

/// Tags the group `meta` as a marks object.
pub(crate) fn write_marks_tag(meta: &LoroMap) {
    insert(meta, KEY_REGISTRATION_MARKS, true);
}

/// Writes every register of `mark` (the `shape` tag included).
pub(crate) fn write_mark(meta: &LoroMap, mark: &RegistrationMarkSnapshot) {
    insert(meta, crate::shape_codec::KEY_SHAPE, SHAPE_REGISTRATION_MARK);
    insert(meta, KEY_NUMBER, i64::from(mark.number));
    write_centre(meta, mark.centre);
    write_size(meta, mark.diameter, mark.line);
}

/// Writes the centre register.
pub(crate) fn write_centre(meta: &LoroMap, centre: Point) {
    insert(
        meta,
        KEY_CENTRE,
        LoroValue::List(vec![centre.x.into(), centre.y.into()].into()),
    );
}

/// Writes the diameter and line registers.
pub(crate) fn write_size(meta: &LoroMap, diameter: Length, line: Length) {
    insert(meta, KEY_DIAMETER, diameter.as_mm());
    insert(meta, KEY_LINE, line.as_mm());
}

fn value(meta: &LoroMap, key: &str) -> Option<LoroValue> {
    match meta.get(key)? {
        ValueOrContainer::Value(value) => Some(value),
        ValueOrContainer::Container(_) => None,
    }
}

fn read_centre(meta: &LoroMap) -> Option<Point> {
    let LoroValue::List(list) = value(meta, KEY_CENTRE)? else {
        return None;
    };
    let [x, y] = list.as_slice() else {
        return None;
    };
    let (x, y) = (as_f64(x)?, as_f64(y)?);
    (x.is_finite() && y.is_finite()).then(|| Point::new(x, y))
}

/// The mark `meta` stores, `None` when a register is missing or damaged.
pub(crate) fn read_mark_snapshot(id: NodeId, meta: &LoroMap) -> Option<RegistrationMarkSnapshot> {
    let LoroValue::I64(number) = value(meta, KEY_NUMBER)? else {
        return None;
    };
    let number = u8::try_from(number)
        .ok()
        .filter(|n| (1..=MARK_MAX_COUNT).contains(n))?;
    let diameter = as_f64(&value(meta, KEY_DIAMETER)?).filter(|&d| mark_diameter_is_valid(d))?;
    let line = as_f64(&value(meta, KEY_LINE)?).filter(|&l| mark_line_is_valid(l))?;
    Some(RegistrationMarkSnapshot::new(
        id,
        number,
        read_centre(meta)?,
        Length::from_mm(diameter),
        Length::from_mm(line),
    ))
}

/// Whether a mark node's registers can be read (open-file validation).
pub(crate) fn validate_mark_node(meta: &LoroMap) -> bool {
    read_mark_snapshot(NodeId::from_parts(0, 0), meta).is_some()
}

/// Moves a mark's centre by `offset` (a copy's shift).
pub(crate) fn translate_mark_meta(meta: &LoroMap, offset: Vec2) {
    if let Some(centre) = read_centre(meta) {
        write_centre(meta, centre.translated(offset));
    }
}

/// Writes the centre of `moved` (a transform result) where it differs from the stored one.
pub(crate) fn write_moved(meta: &LoroMap, moved: &RegistrationMarkSnapshot) {
    if read_centre(meta) != Some(moved.centre) {
        write_centre(meta, moved.centre);
    }
}
