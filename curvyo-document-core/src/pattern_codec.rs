//! The stored form of a pattern fill (ADR 0029 §1, §2): the keys `fill_paint` and
//! `fill_pattern` of an object's meta map, read leniently, written only when changed, and checked
//! strictly when a file is opened.
//!
//! ```text
//!   fill_paint   : "colour" | "pattern"            absent = "colour"
//!   fill_pattern : { generator: string id,          absent = no stored pattern
//!                    margin: f64 mm >= 0,
//!                    mode: "lines" | "shapes",
//!                    seed: integer in u32,
//!                    whole_pieces: bool,            absent = false
//!                    params: { name: number } }     an absent parameter = its default
//! ```

use std::collections::HashMap;

use loro::{LoroMap, LoroValue};

use crate::path_codec::as_f64;
use crate::pattern_model::{
    FillPaint, GeneratorKind, PatternFill, PatternGenerator, PatternMode, PatternSeed,
};
use crate::pattern_schema::{ParamKind, specs};
use crate::style_codec::read_value;
use crate::units::Length;

pub(crate) const KEY_FILL_PAINT: &str = "fill_paint";
pub(crate) const KEY_FILL_PATTERN: &str = "fill_pattern";

const PAINT_COLOUR: &str = "colour";
const PAINT_PATTERN: &str = "pattern";
const MODE_LINES: &str = "lines";
const MODE_SHAPES: &str = "shapes";
const FIELD_GENERATOR: &str = "generator";
const FIELD_MARGIN: &str = "margin";
const FIELD_MODE: &str = "mode";
const FIELD_SEED: &str = "seed";
const FIELD_PARAMS: &str = "params";
const FIELD_WHOLE_PIECES: &str = "whole_pieces";

/// Reads `fill_paint`: anything but `"pattern"` is a colour (lenient).
pub(crate) fn read_paint(meta: &LoroMap) -> FillPaint {
    match read_value(meta, KEY_FILL_PAINT) {
        Some(LoroValue::String(s)) if s.as_str() == PAINT_PATTERN => FillPaint::Pattern,
        _ => FillPaint::Colour,
    }
}

/// Reads `fill_pattern`; a value that is not a valid pattern reads as none (lenient: only a
/// merged document can hold one, open validation refuses a file's).
pub(crate) fn read_pattern(meta: &LoroMap) -> Option<PatternFill> {
    pattern_from_value(&read_value(meta, KEY_FILL_PATTERN)?)
}

fn pattern_from_value(value: &LoroValue) -> Option<PatternFill> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    let kind = match map.get(FIELD_GENERATOR) {
        Some(LoroValue::String(id)) => GeneratorKind::from_id(id)?,
        _ => return None,
    };
    let margin = as_f64(map.get(FIELD_MARGIN)?).filter(|mm| mm.is_finite() && *mm >= 0.0)?;
    let mode = match map.get(FIELD_MODE) {
        Some(LoroValue::String(s)) if s.as_str() == MODE_LINES => PatternMode::Lines,
        Some(LoroValue::String(s)) if s.as_str() == MODE_SHAPES => PatternMode::Shapes,
        _ => return None,
    };
    let seed = match map.get(FIELD_SEED) {
        Some(LoroValue::I64(n)) => PatternSeed::new(u32::try_from(*n).ok()?),
        _ => return None,
    };
    let whole_pieces = match map.get(FIELD_WHOLE_PIECES) {
        None => false,
        Some(LoroValue::Bool(on)) => *on,
        Some(_) => return None,
    };
    let Some(LoroValue::Map(params)) = map.get(FIELD_PARAMS) else {
        return None;
    };
    let values = specs(kind)
        .iter()
        .map(|spec| match params.get(spec.name) {
            None => Some(spec.default),
            Some(v) => as_f64(v),
        })
        .collect::<Option<Vec<f64>>>()?;
    let generator = PatternGenerator::from_values(kind, &values).ok()?;
    Some(PatternFill {
        generator,
        margin: Length::from_mm(margin),
        mode,
        seed,
        whole_pieces,
    })
}

/// The stored value of `pattern`: every parameter written, counts as integers.
fn pattern_to_value(pattern: &PatternFill) -> LoroValue {
    let kind = pattern.generator.kind();
    let params: HashMap<String, LoroValue> = specs(kind)
        .iter()
        .zip(pattern.generator.values())
        .map(|(spec, value)| {
            #[allow(clippy::cast_possible_truncation)] // a count is a whole number
            let stored = if spec.kind == ParamKind::Count {
                LoroValue::I64(value as i64)
            } else {
                LoroValue::Double(value)
            };
            (spec.name.to_owned(), stored)
        })
        .collect();
    let mode = match pattern.mode {
        PatternMode::Lines => MODE_LINES,
        PatternMode::Shapes => MODE_SHAPES,
    };
    let map: HashMap<String, LoroValue> = HashMap::from([
        (FIELD_GENERATOR.to_owned(), LoroValue::from(kind.id())),
        (
            FIELD_MARGIN.to_owned(),
            LoroValue::Double(pattern.margin.as_mm()),
        ),
        (FIELD_MODE.to_owned(), LoroValue::from(mode)),
        (
            FIELD_SEED.to_owned(),
            LoroValue::I64(i64::from(pattern.seed.get())),
        ),
        (
            FIELD_WHOLE_PIECES.to_owned(),
            LoroValue::Bool(pattern.whole_pieces),
        ),
        (FIELD_PARAMS.to_owned(), LoroValue::from(params)),
    ]);
    LoroValue::from(map)
}

/// Writes the paint and the pattern where they changed; returns whether anything was written.
pub(crate) fn write_changes(
    meta: &LoroMap,
    (old_paint, old_pattern): (FillPaint, Option<&PatternFill>),
    (new_paint, new_pattern): (FillPaint, Option<&PatternFill>),
) -> bool {
    let mut wrote = false;
    if old_paint != new_paint {
        let text = match new_paint {
            FillPaint::Colour => PAINT_COLOUR,
            FillPaint::Pattern => PAINT_PATTERN,
        };
        // invariant: inserting a plain value under a known key cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert(KEY_FILL_PAINT, text).unwrap();
        wrote = true;
    }
    if old_pattern != new_pattern {
        match new_pattern {
            // invariant: see above.
            #[allow(clippy::unwrap_used)]
            Some(pattern) => meta
                .insert(KEY_FILL_PATTERN, pattern_to_value(pattern))
                .unwrap(),
            None => {
                if meta.get(KEY_FILL_PATTERN).is_some() {
                    // invariant: deleting a present key cannot fail.
                    #[allow(clippy::unwrap_used)]
                    meta.delete(KEY_FILL_PATTERN).unwrap();
                }
            }
        }
        wrote = true;
    }
    wrote
}

/// Whether the two keys of an opened file are valid (ADR 0029 §2, "damage"): a known paint; a
/// pattern map with a known generator, a margin that is a finite number of at least 0, a known
/// mode, a seed that is an integer in u32, a whole-pieces switch that is a bool if present, and
/// parameters of the generator that are numbers in
/// their range (absent ones read as their default; unknown names are damage).
pub(crate) fn keys_are_valid(meta: &LoroMap) -> bool {
    let paint_ok = read_value(meta, KEY_FILL_PAINT).is_none_or(|v| {
        matches!(&v, LoroValue::String(s) if [PAINT_COLOUR, PAINT_PATTERN].contains(&s.as_str()))
    });
    let pattern_ok = read_value(meta, KEY_FILL_PATTERN).is_none_or(|v| {
        let known_names = match &v {
            LoroValue::Map(map) => match (map.get(FIELD_GENERATOR), map.get(FIELD_PARAMS)) {
                (Some(LoroValue::String(id)), Some(LoroValue::Map(params))) => {
                    GeneratorKind::from_id(id).is_some_and(|kind| {
                        params
                            .keys()
                            .all(|name| specs(kind).iter().any(|spec| spec.name == name))
                    })
                }
                _ => false,
            },
            _ => false,
        };
        known_names && pattern_from_value(&v).is_some()
    });
    paint_ok && pattern_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_model::PatternParam;

    fn meta() -> (loro::LoroDoc, LoroMap) {
        let doc = loro::LoroDoc::new();
        let map = doc.get_map("meta");
        (doc, map)
    }

    #[test]
    fn absent_keys_read_as_a_colour_paint_without_a_pattern() {
        let (_doc, map) = meta();
        assert_eq!(read_paint(&map), FillPaint::Colour);
        assert_eq!(read_pattern(&map), None);
        assert!(keys_are_valid(&map));
    }

    #[test]
    fn every_generator_round_trips_through_the_stored_value() {
        let (_doc, map) = meta();
        for kind in GeneratorKind::ALL {
            let pattern = PatternFill {
                generator: PatternGenerator::defaults(kind),
                margin: Length::from_mm(1.5),
                mode: PatternMode::Shapes,
                seed: PatternSeed::new(u32::MAX),
                whole_pieces: kind.has_whole_pieces(),
            };
            assert!(write_changes(
                &map,
                (FillPaint::Colour, None),
                (FillPaint::Pattern, Some(&pattern))
            ));
            assert_eq!(read_pattern(&map), Some(pattern), "{kind:?}");
            assert_eq!(read_paint(&map), FillPaint::Pattern);
            assert!(keys_are_valid(&map), "{kind:?}");
        }
    }

    #[test]
    fn an_absent_parameter_reads_as_its_default() {
        let (_doc, map) = meta();
        let value = LoroValue::from(HashMap::from([
            (FIELD_GENERATOR.to_owned(), LoroValue::from("dots")),
            (FIELD_MARGIN.to_owned(), LoroValue::Double(0.0)),
            (FIELD_MODE.to_owned(), LoroValue::from("lines")),
            (FIELD_SEED.to_owned(), LoroValue::I64(3)),
            (
                FIELD_PARAMS.to_owned(),
                LoroValue::from(HashMap::from([(
                    "spacing".to_owned(),
                    LoroValue::Double(4.0),
                )])),
            ),
        ]));
        map.insert(KEY_FILL_PATTERN, value).unwrap();
        let pattern = read_pattern(&map).unwrap();
        assert_eq!(pattern.generator.get(PatternParam::Spacing), Some(4.0));
        assert_eq!(pattern.generator.get(PatternParam::Diameter), Some(1.0));
        assert!(!pattern.whole_pieces, "an absent switch reads as off");
        assert!(keys_are_valid(&map));
    }

    #[test]
    fn damaged_values_are_refused() {
        let base = || {
            HashMap::from([
                (FIELD_GENERATOR.to_owned(), LoroValue::from("hatch")),
                (FIELD_MARGIN.to_owned(), LoroValue::Double(0.0)),
                (FIELD_MODE.to_owned(), LoroValue::from("lines")),
                (FIELD_SEED.to_owned(), LoroValue::I64(3)),
                (
                    FIELD_PARAMS.to_owned(),
                    LoroValue::from(HashMap::<String, LoroValue>::new()),
                ),
            ])
        };
        let params = |name: &str, value: LoroValue| {
            let mut map = base();
            map.insert(
                FIELD_PARAMS.to_owned(),
                LoroValue::from(HashMap::from([(name.to_owned(), value)])),
            );
            map
        };
        let with = |field: &str, value: LoroValue| {
            let mut map = base();
            map.insert(field.to_owned(), value);
            map
        };
        let damaged = [
            with(FIELD_GENERATOR, LoroValue::from("voronoi")),
            with(FIELD_MARGIN, LoroValue::Double(-1.0)),
            with(FIELD_MODE, LoroValue::from("dashes")),
            with(FIELD_SEED, LoroValue::I64(-1)),
            with(FIELD_SEED, LoroValue::I64(1 << 33)),
            with(FIELD_SEED, LoroValue::Double(1.5)),
            params("spacing", LoroValue::Double(0.0)),
            params("spacing", LoroValue::from("2")),
            params("spacing", LoroValue::Double(f64::NAN)),
            params("diameter", LoroValue::Double(1.0)),
            with(FIELD_WHOLE_PIECES, LoroValue::from("yes")),
            with(FIELD_WHOLE_PIECES, LoroValue::I64(1)),
        ];
        for map in damaged {
            let (_doc, meta) = meta();
            meta.insert(KEY_FILL_PATTERN, LoroValue::from(map.clone()))
                .unwrap();
            assert!(!keys_are_valid(&meta), "{map:?}");
        }
        let (_doc, meta) = meta();
        meta.insert(KEY_FILL_PAINT, "gradient").unwrap();
        assert!(!keys_are_valid(&meta));
    }
}
