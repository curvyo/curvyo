//! The typed fields of fixtures and slots: each key's range from `curvyo-laser-core`, its word and
//! unit, and the clamping notice of a value set to its nearest limit (criteria 15, 19, 20, 23, 37).

use curvyo_document_core::{FixtureParams, Length};
use curvyo_laser_core::{
    FixtureKind, MAJOR_EVERY_RANGE, MmRange, grid_spacing_range, origin_offset_range,
    screw_head_range, screw_thread_range, slot_size_range, stop_leg_range, stop_thickness_range,
};
use serde_json::Value;

use super::placed::{CLAMP_SIZE_MM, HOLDER_SIZE_MM};

/// One length field: its stored key, the word the notice names and its range.
#[derive(Debug, Clone, Copy)]
pub(super) struct LengthField {
    pub key: &'static str,
    pub word: &'static str,
    pub range: MmRange,
}

/// The length field `key` of a grid, the one kind with block fields (criterion 15); the `0093`
/// names `spacing`, `origin_x` and `origin_y` name its fields too. `None` for any other kind or
/// key: holder, clamp, stop and screw are sized by their handles and chips (D27).
pub(super) fn fixture_field(kind: FixtureKind, key: &str) -> Option<LengthField> {
    let field = |key, word, range| Some(LengthField { key, word, range });
    match (kind, key) {
        (FixtureKind::EngravedGrid, "spacing" | "spacing_mm") => {
            field("spacing_mm", "Spacing", grid_spacing_range())
        }
        (FixtureKind::EngravedGrid, "origin_x" | "origin_x_mm") => {
            field("origin_x_mm", "Origin X", origin_offset_range())
        }
        (FixtureKind::EngravedGrid, "origin_y" | "origin_y_mm") => {
            field("origin_y_mm", "Origin Y", origin_offset_range())
        }
        _ => None,
    }
}

/// The length field `key` of a slot: centre X and Y, width, height, diameter.
pub(super) fn slot_field(key: &str) -> Option<LengthField> {
    let field = |key, word, range| Some(LengthField { key, word, range });
    match key {
        "cx_mm" => field("cx_mm", "Centre X", origin_offset_range()),
        "cy_mm" => field("cy_mm", "Centre Y", origin_offset_range()),
        "w_mm" => field("w_mm", "Width", slot_size_range()),
        "h_mm" => field("h_mm", "Height", slot_size_range()),
        "d_mm" => field("d_mm", "Diameter", slot_size_range()),
        _ => None,
    }
}

/// A millimetre or count value as the notices write it: whole numbers with thousands commas,
/// otherwise up to two decimals.
pub(super) fn number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.trunc()).abs() > f64::EPSILON {
        return format!("{rounded}");
    }
    #[allow(clippy::cast_possible_truncation)] // bounded by the field ranges (±10,000)
    let whole = rounded as i64;
    let digits = whole.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if whole < 0 { format!("-{out}") } else { out }
}

/// The notice of a value set to its nearest limit (UX notes §4.1), at most 70 characters.
pub(super) fn clamp_notice(word: &str, min: f64, max: f64, unit: &str, set: f64) -> String {
    format!(
        "{word} is {} to {} {unit}. Set to {} {unit}.",
        number(min),
        number(max),
        number(set)
    )
}

impl LengthField {
    /// `value` inside the range, and the notice when it had to move.
    pub(super) fn accept(self, value: Length) -> (f64, Option<String>) {
        let (kept, moved) = self.range.clamp(value);
        let notice = moved.then(|| {
            clamp_notice(
                self.word,
                self.range.min.as_mm(),
                self.range.max.as_mm(),
                "mm",
                kept.as_mm(),
            )
        });
        (kept.as_mm(), notice)
    }
}

/// The grid's Major every as a whole count inside its range, and the notice when it moved.
pub(super) fn accept_major_every(value: f64) -> (u32, Option<String>) {
    let (min, max, default) = MAJOR_EVERY_RANGE;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
    let kept = if value.is_finite() {
        value.round().clamp(f64::from(min), f64::from(max)) as u32
    } else {
        default
    };
    let moved = !value.is_finite() || (value.round() - f64::from(kept)).abs() > f64::EPSILON;
    let notice = moved.then(|| {
        clamp_notice(
            "Major every",
            f64::from(min),
            f64::from(max),
            "lines",
            f64::from(kept),
        )
    });
    (kept, notice)
}

/// `params` with `key` set to the number `value`.
pub(super) fn with_number(params: &FixtureParams, key: &str, value: f64) -> FixtureParams {
    let mut out = params.clone();
    out.insert(key.to_owned(), Value::from(value));
    out
}

/// The parameters of a new fixture of `kind` (criterion 2, Proposal sizes; the grid at its
/// criterion 15 defaults with all four tapes).
pub(super) fn new_params(kind: FixtureKind) -> FixtureParams {
    let pairs: Vec<(&str, Value)> = match kind {
        FixtureKind::EngravedGrid => vec![
            (
                "spacing_mm",
                Value::from(grid_spacing_range().default.as_mm()),
            ),
            ("major_every", Value::from(MAJOR_EVERY_RANGE.2)),
            (
                "tapes",
                serde_json::json!(["bottom", "left", "top", "right"]),
            ),
            ("origin_x_mm", Value::from(0.0)),
            ("origin_y_mm", Value::from(0.0)),
        ],
        FixtureKind::Holder => vec![
            ("width_mm", Value::from(HOLDER_SIZE_MM.0)),
            ("height_mm", Value::from(HOLDER_SIZE_MM.1)),
        ],
        FixtureKind::Clamp => vec![
            ("width_mm", Value::from(CLAMP_SIZE_MM.0)),
            ("height_mm", Value::from(CLAMP_SIZE_MM.1)),
        ],
        FixtureKind::Stop => vec![
            ("leg_a_mm", Value::from(stop_leg_range().default.as_mm())),
            ("leg_b_mm", Value::from(stop_leg_range().default.as_mm())),
            (
                "thickness_mm",
                Value::from(stop_thickness_range().default.as_mm()),
            ),
        ],
        FixtureKind::Screw => vec![
            (
                "thread_diameter_mm",
                Value::from(screw_thread_range().default.as_mm()),
            ),
            (
                "head_diameter_mm",
                Value::from(screw_head_range().default.as_mm()),
            ),
        ],
        FixtureKind::Unknown => Vec::new(),
    };
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_the_spec_writes_them() {
        assert_eq!(number(1000.0), "1,000");
        assert_eq!(number(0.5), "0.5");
        assert_eq!(number(-10_000.0), "-10,000");
        assert_eq!(
            clamp_notice("Spacing", 1.0, 100.0, "mm", 100.0),
            "Spacing is 1 to 100 mm. Set to 100 mm."
        );
    }

    #[test]
    fn major_every_is_rounded_and_clamped_with_a_notice() {
        assert_eq!(accept_major_every(5.0), (5, None));
        assert_eq!(
            accept_major_every(40.0),
            (
                20,
                Some("Major every is 2 to 20 lines. Set to 20 lines.".to_owned())
            )
        );
    }
}
