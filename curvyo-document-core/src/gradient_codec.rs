//! The stored form of a gradient fill (ADR 0066 §1, §7): the registers `fill_gradient` and
//! `fill_gradient_stops` of an object's meta map, read leniently, written only when changed, and
//! checked strictly when a file is opened. Each register holds one compact text whose grammar is
//! `gradient_text`'s.
//!
//! The paint value `"gradient"` of `fill_paint` lives in `pattern_codec` with the other paints.

use loro::{LoroMap, LoroValue};

use crate::gradient_model::{GradientFill, GradientGeometry, GradientStops};
use crate::gradient_text::{geometry_text, parse_geometry, parse_stops, stops_text};
use crate::pattern_model::FillPaint;
use crate::style_codec::read_value;

pub(crate) const KEY_FILL_GRADIENT: &str = "fill_gradient";
pub(crate) const KEY_FILL_GRADIENT_STOPS: &str = "fill_gradient_stops";

/// The registers a reset of a node's style keys leaves in place (an own style turned off or
/// rewritten): no write ever removes a gradient, so a `fill_paint` of `"gradient"` that wins a
/// merge always finds both (ADR 0066 §2, §7).
pub(crate) const KEPT_ON_RESET: [&str; 2] = [KEY_FILL_GRADIENT, KEY_FILL_GRADIENT_STOPS];

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

fn geometry_from_value(value: &LoroValue) -> Option<GradientGeometry> {
    match value {
        LoroValue::String(text) => parse_geometry(text),
        _ => None,
    }
}

fn stops_from_value(value: &LoroValue) -> Option<GradientStops> {
    match value {
        LoroValue::String(text) => parse_stops(text),
        _ => None,
    }
}

/// Reads both registers; `None` unless both read (lenient: a register that does not read is
/// absent, and a gradient paint without both paints no fill, ADR 0066 §7).
pub(crate) fn read_gradient(meta: &LoroMap) -> Option<GradientFill> {
    Some(GradientFill {
        geometry: geometry_from_value(&read_value(meta, KEY_FILL_GRADIENT)?)?,
        stops: stops_from_value(&read_value(meta, KEY_FILL_GRADIENT_STOPS)?)?,
    })
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

fn geometry_to_value(geometry: &GradientGeometry) -> LoroValue {
    LoroValue::from(geometry_text(geometry))
}

fn stops_to_value(stops: &GradientStops) -> LoroValue {
    LoroValue::from(stops_text(stops))
}

fn insert(meta: &LoroMap, key: &str, value: LoroValue) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

/// Writes the register that changed, each on its own, so a drag on one peer and a stop edit on
/// another both survive a merge (ADR 0066 §2); returns whether anything was written. A gradient
/// is never removed by a write: switching the paint away keeps it.
pub(crate) fn write_changes(
    meta: &LoroMap,
    old: Option<&GradientFill>,
    new: Option<&GradientFill>,
) -> bool {
    let Some(new) = new else {
        return false;
    };
    let mut wrote = false;
    if old.is_none_or(|old| old.geometry != new.geometry) {
        insert(meta, KEY_FILL_GRADIENT, geometry_to_value(&new.geometry));
        wrote = true;
    }
    if old.is_none_or(|old| old.stops != new.stops) {
        insert(meta, KEY_FILL_GRADIENT_STOPS, stops_to_value(&new.stops));
        wrote = true;
    }
    wrote
}

/// Writes only `fill_gradient`, with `new`'s geometry, where a stored gradient's geometry differs
/// (a transform carries the picture, ADR 0066 §5); returns whether it wrote. Nothing is written
/// without `new` or without a stored gradient.
pub(crate) fn write_carried_geometry(meta: &LoroMap, new: Option<&GradientFill>) -> bool {
    let (Some(new), Some(old)) = (new, read_gradient(meta)) else {
        return false;
    };
    if old.geometry == new.geometry {
        return false;
    }
    insert(meta, KEY_FILL_GRADIENT, geometry_to_value(&new.geometry));
    true
}

// ---------------------------------------------------------------------------
// Strict validation
// ---------------------------------------------------------------------------

/// Whether the two registers of an opened file are valid (ADR 0066 §7): each, if present, reads
/// exactly, and a `fill_paint` of `"gradient"` has both.
pub(crate) fn keys_are_valid(meta: &LoroMap, paint: FillPaint) -> bool {
    let geometry = read_value(meta, KEY_FILL_GRADIENT);
    let stops = read_value(meta, KEY_FILL_GRADIENT_STOPS);
    let geometry_ok = geometry
        .as_ref()
        .is_none_or(|v| geometry_from_value(v).is_some());
    let stops_ok = stops.as_ref().is_none_or(|v| stops_from_value(v).is_some());
    let paint_ok = paint != FillPaint::Gradient || (geometry.is_some() && stops.is_some());
    geometry_ok && stops_ok && paint_ok
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::gradient_model::{AxisRatio, BoxPoint};
    use crate::path_model::Color;

    fn meta() -> (loro::LoroDoc, LoroMap) {
        let doc = loro::LoroDoc::new();
        let map = doc.get_map("meta");
        (doc, map)
    }

    #[test]
    fn absent_registers_read_as_no_gradient_and_are_valid() {
        let (_doc, meta) = meta();
        assert_eq!(read_gradient(&meta), None);
        assert!(keys_are_valid(&meta, FillPaint::Colour));
        assert!(
            !keys_are_valid(&meta, FillPaint::Gradient),
            "a gradient paint needs both registers"
        );
    }

    #[test]
    fn a_written_gradient_reads_back_and_is_valid() {
        let (_doc, meta) = meta();
        for geometry in [
            GradientGeometry::DEFAULT_LINEAR,
            GradientGeometry::DEFAULT_RADIAL,
            GradientGeometry::Radial {
                centre: BoxPoint::new(-0.25, 2.0).unwrap(),
                radius: BoxPoint::new(0.5, 0.125).unwrap(),
                ratio: AxisRatio::new(0.4).unwrap(),
            },
        ] {
            let fill = GradientFill {
                geometry,
                stops: GradientStops::two((Color { r: 1, g: 2, b: 171 }, 205), (Color::WHITE, 0)),
            };
            write_changes(&meta, None, Some(&fill));
            assert_eq!(read_gradient(&meta), Some(fill));
            assert!(keys_are_valid(&meta, FillPaint::Gradient));
        }
        assert_eq!(
            read_value(&meta, KEY_FILL_GRADIENT_STOPS),
            Some(LoroValue::from("#0102ABCD 0;#FFFFFF00 1")),
            "one text, upper case"
        );
        assert_eq!(
            read_value(&meta, KEY_FILL_GRADIENT),
            Some(LoroValue::from("radial -0.25 2 0.5 0.125 0.4"))
        );
    }

    #[test]
    fn only_the_changed_register_is_written() {
        let (doc, meta) = meta();
        let fill = GradientFill::first_for(Color::BLACK, 255);
        assert!(write_changes(&meta, None, Some(&fill)));
        doc.commit();
        let ops = doc.len_ops();
        let mut dragged = fill.clone();
        dragged.geometry = GradientGeometry::DEFAULT_RADIAL;
        assert!(write_changes(&meta, Some(&fill), Some(&dragged)));
        doc.commit();
        assert_eq!(doc.len_ops() - ops, 1, "one register");
        assert!(!write_changes(&meta, Some(&dragged), Some(&dragged)));
        assert!(!write_changes(&meta, Some(&dragged), None));
    }

    /// ADR 0066 §2: a drag on one peer and a stop edit on another both survive the merge.
    #[test]
    fn a_drag_and_a_stop_edit_merge_without_loss() {
        let (a, meta_a) = meta();
        a.set_peer_id(1).unwrap();
        let fill = GradientFill::first_for(Color::BLACK, 255);
        write_changes(&meta_a, None, Some(&fill));
        a.commit();
        let b = loro::LoroDoc::new();
        b.set_peer_id(2).unwrap();
        b.import(&a.export(loro::ExportMode::Snapshot).unwrap())
            .unwrap();
        let meta_b = b.get_map("meta");
        let mut dragged = fill.clone();
        dragged.geometry = GradientGeometry::DEFAULT_RADIAL;
        write_changes(&meta_a, Some(&fill), Some(&dragged));
        a.commit();
        let mut recoloured = fill.clone();
        recoloured.stops = GradientStops::two((Color::WHITE, 255), (Color::BLACK, 255));
        write_changes(&meta_b, Some(&fill), Some(&recoloured));
        b.commit();
        a.import(&b.export(loro::ExportMode::all_updates()).unwrap())
            .unwrap();
        b.import(&a.export(loro::ExportMode::all_updates()).unwrap())
            .unwrap();
        let merged = GradientFill {
            geometry: dragged.geometry,
            stops: recoloured.stops,
        };
        assert_eq!(read_gradient(&meta_a), Some(merged.clone()));
        assert_eq!(read_gradient(&meta_b), Some(merged));
    }

    /// ADR 0066 §7: a value that is not the documented text refuses the file, also the map and
    /// list forms of the first draft.
    #[test]
    fn damaged_values_are_refused() {
        let pair = LoroValue::from(vec![LoroValue::Double(0.0), LoroValue::Double(0.5)]);
        let geometries = [
            LoroValue::from("conic 0 0 1 0"),
            LoroValue::from("linear abc 0.5 1 0.5"),
            LoroValue::from("radial 0.5 0.5 1 0.5 0"),
            LoroValue::Double(1.0),
            LoroValue::from(HashMap::from([
                ("kind".to_owned(), LoroValue::from("linear")),
                ("start".to_owned(), pair.clone()),
                ("end".to_owned(), pair.clone()),
            ])),
        ];
        for geometry in geometries {
            let (_doc, meta) = meta();
            meta.insert(KEY_FILL_GRADIENT, geometry.clone()).unwrap();
            assert!(!keys_are_valid(&meta, FillPaint::Colour), "{geometry:?}");
            assert_eq!(read_gradient(&meta), None);
        }
        let stop_lists = [
            LoroValue::from("#000000 0"),
            LoroValue::from("#00000 0;#FFFFFF 1"),
            LoroValue::from("#000000 0;#808080 0.7;#808080 0.3;#FFFFFF 1"),
            LoroValue::from(vec![
                LoroValue::from(vec![LoroValue::from("#000000FF"), LoroValue::Double(0.0)]),
                LoroValue::from(vec![LoroValue::from("#FFFFFFFF"), LoroValue::Double(1.0)]),
            ]),
        ];
        for stops in stop_lists {
            let (_doc, meta) = meta();
            meta.insert(KEY_FILL_GRADIENT_STOPS, stops.clone()).unwrap();
            assert!(!keys_are_valid(&meta, FillPaint::Colour), "{stops:?}");
        }
        let (_doc, meta) = meta();
        meta.insert(KEY_FILL_GRADIENT, "linear 0 0.5 1 0.5")
            .unwrap();
        assert!(
            !keys_are_valid(&meta, FillPaint::Gradient),
            "a gradient paint without its stops"
        );
        assert!(
            keys_are_valid(&meta, FillPaint::Colour),
            "kept while not painted"
        );
    }
}
