//! A stored fixture's values read as typed values with a count of those that did not read
//! (`specs/0102-laser-fixtures` criteria 33 and 34; ADR 0049 §6, ADR 0050 §2).

use curvyo_document_core::{Length, OriginCorner, Point};
use curvyo_extension_core::StoredParams;
use serde_json::Value;

use crate::fixture_kind::{
    FixtureKind, MAJOR_EVERY_RANGE, MmRange, SlotShape, board_side_range, grid_spacing_range,
    origin_offset_range, screw_head_range, screw_thread_range, slot_size_range, stop_leg_range,
    stop_thickness_range,
};
use crate::fixture_slot::Slot;
use crate::grid_layout::{GridSpec, TapeEdges};

/// The stored text of a rectangle slot's shape.
pub const SLOT_RECT: &str = "rect";
/// The stored text of a circle slot's shape.
pub const SLOT_CIRCLE: &str = "circle";

/// A length value and whether it did not read: absent reads as the default and is no damage; a
/// value that is not a finite number inside the range reads as the default and is damage.
fn length(params: &StoredParams, key: &str, range: MmRange) -> (Length, usize) {
    match params.get(key) {
        None => (range.default, 0),
        Some(value) => match value.as_f64() {
            Some(mm) if (range.min.as_mm()..=range.max.as_mm()).contains(&mm) => {
                (Length::from_mm(mm), 0)
            }
            _ => (range.default, 1),
        },
    }
}

/// A `0093` length of the form `{"mm": n}`: `Some` with its damage when the key is present.
fn legacy_length(params: &StoredParams, key: &str) -> Option<(f64, usize)> {
    let value = params.get(key)?;
    Some(
        match value
            .get("mm")
            .and_then(Value::as_f64)
            .filter(|mm| mm.is_finite())
        {
            Some(mm) => (mm, 0),
            None => (0.0, 1),
        },
    )
}

/// How many of the kind's stored values do not read. A key the kind does not use is kept and is
/// no damage; an unknown kind has no damage of its own.
#[must_use]
pub fn params_damage(kind: FixtureKind, params: &StoredParams) -> usize {
    match kind {
        FixtureKind::EngravedGrid => grid_spec(params, OriginCorner::default()).1,
        FixtureKind::Holder | FixtureKind::Clamp => {
            let side = board_side_range(1.0);
            length(params, "width_mm", side).1 + length(params, "height_mm", side).1
        }
        FixtureKind::Stop => {
            length(params, "leg_a_mm", stop_leg_range()).1
                + length(params, "leg_b_mm", stop_leg_range()).1
                + length(params, "thickness_mm", stop_thickness_range()).1
        }
        FixtureKind::Screw => screw_spec(params).2,
        FixtureKind::Unknown => 0,
    }
}

/// A screw's thread and head diameters with the number of values that did not read: each reads as
/// its default when damaged, and a head below the thread reads as the thread's value and counts as
/// one damaged value (ADR 0049 §6).
#[must_use]
pub fn screw_spec(params: &StoredParams) -> (Length, Length, usize) {
    let (thread, thread_damage) = length(params, "thread_diameter_mm", screw_thread_range());
    let (head, head_damage) = length(params, "head_diameter_mm", screw_head_range());
    if head.as_mm() < thread.as_mm() {
        (thread, thread, thread_damage + head_damage + 1)
    } else {
        (thread, head, thread_damage + head_damage)
    }
}

/// The grid's layout values over the 0090 `grid_origin` corner `corner` (Frage 174 B), with the
/// number of values that did not read. The `0093` keys (`spacing`, `origin_x`, `origin_y` as
/// `{"mm": n}`) are read when the new ones are absent, so a format 39 grid draws its lines as
/// before; absent tapes are all four (Frage 174 C).
#[must_use]
pub fn grid_spec(params: &StoredParams, corner: OriginCorner) -> (GridSpec, usize) {
    let mut damaged = 0;
    let mut mm = |key: &str, legacy: &str, range: MmRange| {
        if !params.contains_key(key)
            && let Some((value, damage)) = legacy_length(params, legacy)
        {
            damaged += damage;
            return if damage == 0 {
                Length::from_mm(value)
            } else {
                range.default
            };
        }
        let (value, damage) = length(params, key, range);
        damaged += damage;
        value
    };
    let spacing = mm("spacing_mm", "spacing", grid_spacing_range());
    let origin_x = mm("origin_x_mm", "origin_x", origin_offset_range());
    let origin_y = mm("origin_y_mm", "origin_y", origin_offset_range());
    let (min, max, default) = MAJOR_EVERY_RANGE;
    let major_every = match params.get("major_every") {
        None => default,
        Some(value) => match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
            Some(n) if (min..=max).contains(&n) => n,
            _ => {
                damaged += 1;
                default
            }
        },
    };
    let (tapes, tape_damage) = read_tapes(params.get("tapes"));
    let spec = GridSpec {
        spacing,
        major_every,
        tapes,
        corner,
        origin_x,
        origin_y,
    };
    (spec, damaged + tape_damage)
}

/// The tape switches: a list of edge names; absent is all four, anything else all four and
/// damage.
fn read_tapes(value: Option<&Value>) -> (TapeEdges, usize) {
    let Some(value) = value else {
        return (TapeEdges::all(), 0);
    };
    let Some(names) = value.as_array() else {
        return (TapeEdges::all(), 1);
    };
    let mut tapes = TapeEdges::default();
    for name in names {
        match name.as_str() {
            Some("bottom") => tapes.bottom = true,
            Some("top") => tapes.top = true,
            Some("left") => tapes.left = true,
            Some("right") => tapes.right = true,
            _ => return (TapeEdges::all(), 1),
        }
    }
    (tapes, 0)
}

/// A slot's stored `slot_params` as a slot; `None` (the slot is skipped) when its shape is not
/// `rect` or `circle`, its centre is not two finite numbers, or the size of its shape is not
/// within 0.5 to 1,000 mm. The sizes its shape does not use are not read.
#[must_use]
pub fn read_slot(params: &StoredParams) -> Option<Slot> {
    let shape = match params.get("shape")?.as_str()? {
        SLOT_RECT => SlotShape::Rectangle,
        SLOT_CIRCLE => SlotShape::Circle,
        _ => return None,
    };
    let centre = origin_offset_range();
    let coordinate = |key: &str| {
        let (value, damage) = length(params, key, centre);
        (params.contains_key(key) && damage == 0).then_some(value.as_mm())
    };
    let centre = Point::new(coordinate("cx_mm")?, coordinate("cy_mm")?);
    let size = |key: &str| {
        let (value, damage) = length(params, key, slot_size_range());
        (params.contains_key(key) && damage == 0).then_some(value)
    };
    let none = Length::from_mm(0.0);
    let (width, height, diameter) = match shape {
        SlotShape::Rectangle => (size("w_mm")?, size("h_mm")?, none),
        SlotShape::Circle => (none, none, size("d_mm")?),
    };
    Some(Slot {
        shape,
        centre,
        width,
        height,
        diameter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(value: serde_json::Value) -> StoredParams {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_0093_grid_reads_its_spacing_and_origin_with_all_four_tapes() {
        let params = stored(serde_json::json!({
            "spacing": {"mm": 25.0}, "line_width": {"mm": 0.1},
            "origin_x": {"mm": 5.0}, "origin_y": {"mm": 0.0}
        }));
        let (spec, damaged) = grid_spec(&params, OriginCorner::BottomLeft);
        assert_eq!(damaged, 0);
        assert_eq!(spec.spacing, Length::from_mm(25.0));
        assert_eq!(spec.origin_x, Length::from_mm(5.0));
        assert_eq!(spec.tapes, TapeEdges::all());
        assert_eq!(spec.major_every, 5);
        assert_eq!(spec.corner, OriginCorner::BottomLeft, "the 0090 corner");
    }

    #[test]
    fn a_grid_of_format_40_reads_its_keys_and_counts_each_damaged_value() {
        let params = stored(serde_json::json!({
            "spacing_mm": "dense", "major_every": 50, "tapes": ["bottom", "middle"],
            "origin_x_mm": 3.0
        }));
        let (spec, damaged) = grid_spec(&params, OriginCorner::TopLeft);
        assert_eq!(damaged, 3);
        assert_eq!(spec.spacing, Length::from_mm(10.0));
        assert_eq!(spec.major_every, 5);
        assert_eq!(spec.tapes, TapeEdges::all());
        assert_eq!(spec.origin_x, Length::from_mm(3.0));
        let only = stored(serde_json::json!({"tapes": ["left"]}));
        let (spec, damaged) = grid_spec(&only, OriginCorner::TopLeft);
        assert_eq!(
            (spec.tapes.left, spec.tapes.bottom, damaged),
            (true, false, 0)
        );
    }

    #[test]
    fn board_and_stop_values_outside_their_range_are_damage() {
        let holder = stored(serde_json::json!({"width_mm": "wide", "height_mm": 60.0}));
        assert_eq!(params_damage(FixtureKind::Holder, &holder), 1);
        let stop = stored(serde_json::json!({"leg_a_mm": 0.0, "thickness_mm": 51.0}));
        assert_eq!(params_damage(FixtureKind::Stop, &stop), 2);
        assert_eq!(params_damage(FixtureKind::Unknown, &holder), 0);
    }

    #[test]
    fn a_screw_reads_its_diameters_and_a_head_below_its_thread_is_damage() {
        let mm = Length::from_mm;
        let fine = stored(serde_json::json!({"thread_diameter_mm": 8.0, "head_diameter_mm": 13.0}));
        assert_eq!(screw_spec(&fine), (mm(8.0), mm(13.0), 0));
        let text =
            stored(serde_json::json!({"thread_diameter_mm": "M6", "head_diameter_mm": 10.0}));
        assert_eq!(screw_spec(&text), (mm(6.0), mm(10.0), 1));
        let small_head =
            stored(serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 4.0}));
        assert_eq!(screw_spec(&small_head), (mm(6.0), mm(6.0), 1));
        assert_eq!(params_damage(FixtureKind::Screw, &small_head), 1);
        assert_eq!(screw_spec(&StoredParams::new()), (mm(6.0), mm(10.0), 0));
    }

    #[test]
    fn a_slot_reads_when_its_shape_and_its_sizes_do() {
        let circle = stored(serde_json::json!({
            "shape": "circle", "cx_mm": 30.0, "cy_mm": 30.0, "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0
        }));
        let slot = read_slot(&circle).unwrap();
        assert_eq!(slot.shape, SlotShape::Circle);
        assert_eq!(slot.diameter, Length::from_mm(12.0));
        for broken in [
            serde_json::json!({"cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 5.0, "h_mm": 5.0}),
            serde_json::json!({"shape": "star", "cx_mm": 5.0, "cy_mm": 5.0}),
            serde_json::json!({"shape": "circle", "cx_mm": 5.0, "cy_mm": 5.0, "d_mm": "big"}),
            serde_json::json!({"shape": "rect", "cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 0.1, "h_mm": 5.0}),
        ] {
            assert_eq!(read_slot(&stored(broken.clone())), None, "{broken}");
        }
    }
}
