//! The four value fields of the text layout block (`specs/0079-text-layout-controls` criteria 7,
//! 10, 12, 13; UX notes §0 and §6): their scales relative to Size, what they show for one or
//! several texts, and the per-text values a change sets.

use std::ops::RangeInclusive;

use curvyo_document_core::{
    Angle, LETTER_SPACING_FACTORS, LINE_SPACING_FACTORS, TextLayoutLabel, TextLayoutValues,
    WORD_SPACING_FACTORS,
};

use crate::display_unit_text::parse_length;
use crate::transform_entry::parse_entry_number;
use crate::value_scale::Grid;

/// The field's drawn width (design system, "Value field"): the 1-2-5 drag grid is at least one
/// track span per this many pixels.
const FIELD_WIDTH_PX: f64 = 244.0;
/// Two values closer than this (mm or degrees) are the same: one value, not "Mixed".
const SAME_VALUE: f64 = 1e-6;
/// A clamped factor further than this from the asked one counts as limited.
const LIMITED: f64 = 1e-9;

/// One field of the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutField {
    /// Letter spacing, mm.
    LetterSpacing,
    /// Word spacing, mm.
    WordSpacing,
    /// Line spacing, mm, or Auto.
    LineSpacing,
    /// Character rotation, degrees.
    CharacterRotation,
}

/// One subject text as the fields see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSubject {
    /// The text's Size, mm.
    pub size_mm: f64,
    /// Its layout values.
    pub values: TextLayoutValues,
    /// Its face's natural line spacing at that size, mm (the Auto readout).
    pub natural_line_mm: f64,
}

/// What a field shows (criteria 7, 10, 13).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutFieldView {
    /// The value as text without the unit (U+2212 minus); empty when mixed.
    pub text: String,
    /// The value in mm or degrees (`aria-valuenow`).
    pub value: f64,
    /// The subject texts differ.
    pub mixed: bool,
    /// Line spacing is Auto: the natural spacing is shown, muted.
    pub auto: bool,
    /// The filled bar from `bar_from` to `bar_to`, shares of the field's width.
    pub bar_from: f64,
    /// See `bar_from`.
    pub bar_to: f64,
    /// Where 0 is on a track that crosses it (the zero mark), else `None`.
    pub zero: Option<f64>,
    /// Where a drag starts from on the scale extended beyond the track.
    pub drag_base: f64,
    /// The lowest drag position (below 0 for a left overshoot).
    pub drag_min: f64,
    /// The highest drag position (above 1 for a right overshoot).
    pub drag_limit: f64,
    /// The track's ends in mm or degrees (`aria-valuemin`, `aria-valuemax`).
    pub track_min: f64,
    /// See `track_min`.
    pub track_max: f64,
    /// The reset icon shows.
    pub resettable: bool,
}

/// What a change did to the subject texts.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutChange {
    /// Each subject's new values, in order.
    pub values: Vec<TextLayoutValues>,
    /// How many texts were held to their own limit.
    pub limited: usize,
    /// When any was: whether at the top (`true`) or the bottom of the range, and that text's
    /// limit in mm.
    pub limit: Option<(bool, f64)>,
}

impl LayoutField {
    /// Every field, in the block's order.
    pub const ALL: [Self; 4] = [
        Self::LetterSpacing,
        Self::WordSpacing,
        Self::LineSpacing,
        Self::CharacterRotation,
    ];

    /// The field named by the host (`"letter-spacing"`, `"word-spacing"`, `"line-spacing"`,
    /// `"character-rotation"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "letter-spacing" => Some(Self::LetterSpacing),
            "word-spacing" => Some(Self::WordSpacing),
            "line-spacing" => Some(Self::LineSpacing),
            "character-rotation" => Some(Self::CharacterRotation),
            _ => None,
        }
    }

    /// The step a change of this field is.
    #[must_use]
    pub const fn label(self) -> TextLayoutLabel {
        match self {
            Self::LetterSpacing => TextLayoutLabel::LetterSpacing,
            Self::WordSpacing => TextLayoutLabel::WordSpacing,
            Self::LineSpacing => TextLayoutLabel::LineSpacing,
            Self::CharacterRotation => TextLayoutLabel::CharacterRotation,
        }
    }

    /// The typed range as factors of Size (criterion 7); `None` for the rotation.
    fn typed_factors(self) -> Option<RangeInclusive<f64>> {
        match self {
            Self::LetterSpacing => Some(LETTER_SPACING_FACTORS),
            Self::WordSpacing => Some(WORD_SPACING_FACTORS),
            Self::LineSpacing => Some(LINE_SPACING_FACTORS),
            Self::CharacterRotation => None,
        }
    }

    /// The track in mm at the smallest subject Size, or in degrees (UX notes §0.4).
    fn track(self, size_mm: f64) -> (f64, f64) {
        match self {
            Self::LetterSpacing => (-0.25 * size_mm, size_mm),
            Self::WordSpacing => (-0.5 * size_mm, 2.0 * size_mm),
            Self::LineSpacing => (0.0, 3.0 * size_mm),
            Self::CharacterRotation => (-180.0, 180.0),
        }
    }

    /// The value of `subject` in mm or degrees; `None` for Line spacing Auto.
    fn value_of(self, subject: &LayoutSubject) -> Option<f64> {
        let values = &subject.values;
        match self {
            Self::LetterSpacing => Some(values.letter_spacing * subject.size_mm),
            Self::WordSpacing => Some(values.word_spacing * subject.size_mm),
            Self::LineSpacing => values.line_spacing.map(|f| f * subject.size_mm),
            Self::CharacterRotation => Some(values.rotation.as_radians().to_degrees()),
        }
    }

    /// The value the field shows for `subject`: Auto shows the natural spacing.
    fn shown_of(self, subject: &LayoutSubject) -> f64 {
        self.value_of(subject).unwrap_or(subject.natural_line_mm)
    }

    /// The value as the field shows it: mm with up to two decimals, degrees with one, trailing
    /// zeros trimmed, U+2212 for minus.
    #[must_use]
    pub fn text(self, value: f64) -> String {
        let scale = if self == Self::CharacterRotation {
            10.0
        } else {
            100.0
        };
        let rounded = (value * scale).round() / scale + 0.0;
        format!("{rounded}").replace('-', "\u{2212}")
    }
}

fn smallest_size(subjects: &[LayoutSubject]) -> f64 {
    subjects
        .iter()
        .map(|subject| subject.size_mm)
        .fold(f64::INFINITY, f64::min)
}

/// The field's view for `subjects` (none: an empty, unmixed field at 0).
#[must_use]
pub fn field_view(field: LayoutField, subjects: &[LayoutSubject]) -> LayoutFieldView {
    let size = smallest_size(subjects);
    let size = if size.is_finite() { size } else { 1.0 };
    let (low, high) = field.track(size);
    let span = high - low;
    let position = |value: f64| (value - low) / span;
    let first = subjects.first();
    let shown = first.map_or(0.0, |s| field.shown_of(s));
    let auto = first.is_some_and(|s| field.value_of(s).is_none());
    let mixed = subjects.iter().any(|s| {
        (field.shown_of(s) - shown).abs() > SAME_VALUE || field.value_of(s).is_none() != auto
    });
    let (drag_min, drag_limit) = match field.typed_factors() {
        Some(range) => (
            position(range.start() * size).min(0.0),
            position(range.end() * size).max(1.0),
        ),
        None => (0.0, 1.0),
    };
    let zero = (low < 0.0).then(|| position(0.0));
    let at = position(shown).clamp(0.0, 1.0);
    let (bar_from, bar_to) = match zero {
        Some(zero) => (zero.min(at), zero.max(at)),
        None => (0.0, at),
    };
    let at_default = match field {
        LayoutField::LineSpacing => auto,
        _ => shown.abs() <= SAME_VALUE,
    };
    LayoutFieldView {
        text: if mixed {
            String::new()
        } else {
            field.text(shown)
        },
        value: shown,
        mixed,
        auto: auto && !mixed,
        bar_from,
        bar_to,
        zero,
        drag_base: position(shown).clamp(drag_min, drag_limit),
        drag_min,
        drag_limit,
        track_min: low,
        track_max: high,
        resettable: mixed || !at_default,
    }
}

/// The smallest 1-2-5 step that is at least `at_least`.
fn one_two_five(at_least: f64) -> f64 {
    let decade = 10_f64.powf(at_least.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * decade)
        .find(|step| *step >= at_least * (1.0 - 1e-12))
        .unwrap_or(10.0 * decade)
}

/// The drag grid of `field` (UX notes §0.1): spacing on the smallest 1-2-5 step of at least one
/// track span per field width, Shift ten times, Ctrl a tenth; rotation 1°, Shift 15°, Ctrl 0.1°.
fn drag_grid(field: LayoutField, subjects: &[LayoutSubject], grid: Grid) -> f64 {
    if field == LayoutField::CharacterRotation {
        return match grid {
            Grid::Normal => 1.0,
            Grid::Coarse => 15.0,
            Grid::Fine => 0.1,
        };
    }
    let (low, high) = field.track(smallest_size(subjects));
    let step = one_two_five((high - low) / FIELD_WIDTH_PX);
    match grid {
        Grid::Normal => step,
        Grid::Coarse => step * 10.0,
        Grid::Fine => step / 10.0,
    }
}

fn snapped(value: f64, step: f64) -> f64 {
    (value / step).round() * step + 0.0
}

/// The value (mm or degrees) at drag position `p`, rounded to the drag grid: the track continued
/// linearly up to the typed range at either end; the rotation stops at its ends.
#[must_use]
pub fn value_at(field: LayoutField, subjects: &[LayoutSubject], p: f64, grid: Grid) -> f64 {
    let view = field_view(field, subjects);
    let p = if p.is_finite() { p } else { 0.0 };
    let p = p.clamp(view.drag_min, view.drag_limit);
    let value = view.track_min + p * (view.track_max - view.track_min);
    snapped(value, drag_grid(field, subjects, grid))
}

/// The value `steps` arrow-key steps from `shown` (UX notes §0.1): spacing 0.1 mm, Shift 1 mm,
/// Ctrl 0.01 mm; rotation 1°, Shift 15°, Ctrl 0.1°.
#[must_use]
pub fn stepped(field: LayoutField, shown: f64, steps: i32, grid: Grid) -> f64 {
    let (normal, coarse, fine) = if field == LayoutField::CharacterRotation {
        (1.0, 15.0, 0.1)
    } else {
        (0.1, 1.0, 0.01)
    };
    let step = match grid {
        Grid::Normal => normal,
        Grid::Coarse => coarse,
        Grid::Fine => fine,
    };
    let value = snapped(shown + f64::from(steps) * step, step);
    if field == LayoutField::CharacterRotation {
        value.clamp(-180.0, 180.0)
    } else {
        value
    }
}

/// A typed value in mm (the units of `0052`: "2", "2 mm", "0.1 in") or degrees ("12", "12°");
/// `None` when it is not a number.
#[must_use]
pub fn parse_field_text(field: LayoutField, text: &str) -> Option<f64> {
    match field {
        LayoutField::CharacterRotation => parse_entry_number(text, true),
        _ => parse_length(text, curvyo_document_core::DisplayUnit::Mm)
            .map(curvyo_document_core::Length::as_mm),
    }
    .filter(|value| value.is_finite())
}

/// Sets `field` to `value` (mm or degrees; `None`: Line spacing Auto, the other fields 0) on
/// every subject, each held to its own Size's range (criteria 7, 13); a rotation wraps into
/// (-180°, 180°].
#[must_use]
pub fn set_field(
    field: LayoutField,
    subjects: &[LayoutSubject],
    value: Option<f64>,
) -> LayoutChange {
    let mut limited = 0;
    let mut limit = None;
    let values = subjects
        .iter()
        .map(|subject| {
            let mut values = subject.values;
            if field == LayoutField::CharacterRotation {
                let degrees = value.unwrap_or(0.0);
                values.rotation = Angle::from_radians(degrees.to_radians()).normalized();
                return values;
            }
            let factor = value.map(|mm| {
                let asked = mm / subject.size_mm;
                let range = field.typed_factors().unwrap_or(0.0..=0.0);
                let held = asked.clamp(*range.start(), *range.end());
                if (held - asked).abs() > LIMITED {
                    limited += 1;
                    limit = Some((asked > held, held * subject.size_mm));
                }
                held
            });
            match field {
                LayoutField::LetterSpacing => values.letter_spacing = factor.unwrap_or(0.0),
                LayoutField::WordSpacing => values.word_spacing = factor.unwrap_or(0.0),
                LayoutField::LineSpacing => values.line_spacing = factor,
                LayoutField::CharacterRotation => {}
            }
            values
        })
        .collect();
    LayoutChange {
        values,
        limited,
        limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(size: f64) -> LayoutSubject {
        LayoutSubject {
            size_mm: size,
            values: TextLayoutValues::DEFAULT,
            natural_line_mm: size * 1.25,
        }
    }

    /// Criterion 7: the tracks scale with Size, a spacing track crosses 0 with a zero mark and
    /// overshoots to the typed range at either end; the rotation does not.
    #[test]
    fn ac07_tracks_relative_to_size() {
        let view = field_view(LayoutField::LetterSpacing, &[subject(20.0)]);
        assert!((view.track_min + 5.0).abs() < 1e-9 && (view.track_max - 20.0).abs() < 1e-9);
        assert!((view.zero.unwrap() - 0.2).abs() < 1e-9);
        assert!(!view.resettable && view.text == "0");
        // The typed range is -10 mm to 40 mm: below the track's start and past its end.
        assert!(view.drag_min < 0.0 && view.drag_limit > 1.0);
        let at = |p| {
            value_at(
                LayoutField::LetterSpacing,
                &[subject(20.0)],
                p,
                Grid::Normal,
            )
        };
        assert!((at(view.drag_limit + 5.0) - 40.0).abs() < 1e-9);
        assert!((at(view.drag_min - 5.0) + 10.0).abs() < 1e-9);
        let rotation = field_view(LayoutField::CharacterRotation, &[subject(20.0)]);
        assert_eq!((rotation.drag_min, rotation.drag_limit), (0.0, 1.0));
        let line = field_view(LayoutField::LineSpacing, &[subject(10.0)]);
        assert!(line.auto && line.zero.is_none() && !line.resettable);
        assert_eq!(line.text, "12.5", "Auto shows the natural spacing");
    }

    /// UX notes §0.1: the drag lands on 1-2-5 steps of the track per field width; the arrows step
    /// 0.1 mm (Shift 1, Ctrl 0.01), the rotation 1° (Shift 15°, Ctrl 0.1°).
    #[test]
    fn the_drag_grid_and_the_steps() {
        assert!((one_two_five(0.103) - 0.2).abs() < 1e-12);
        assert!((one_two_five(0.02) - 0.02).abs() < 1e-12);
        assert!((one_two_five(0.3) - 0.5).abs() < 1e-12);
        // Size 20: track 25 mm over 244 px, 0.102 mm a pixel: a 0.2 mm grid.
        let value = value_at(
            LayoutField::LetterSpacing,
            &[subject(20.0)],
            0.3,
            Grid::Normal,
        );
        assert!(((value / 0.2).round() * 0.2 - value).abs() < 1e-9);
        assert!((stepped(LayoutField::LetterSpacing, 1.0, 1, Grid::Normal) - 1.1).abs() < 1e-9);
        assert!((stepped(LayoutField::LetterSpacing, 1.0, -1, Grid::Coarse)).abs() < 1e-9);
        assert!(
            (stepped(LayoutField::CharacterRotation, 179.0, 1, Grid::Coarse) - 180.0).abs() < 1e-9
        );
    }

    /// Criterion 13 and UX notes §0.4: equal mm values show, different ones are Mixed; a change
    /// sets the same mm on every text, held to each text's own range, and counts the limited.
    #[test]
    fn ac13_mixed_and_per_text_limits() {
        let mut a = subject(10.0);
        a.values.letter_spacing = 0.2;
        let mut b = subject(20.0);
        b.values.letter_spacing = 0.1;
        let view = field_view(LayoutField::LetterSpacing, &[a, b]);
        assert!(!view.mixed && view.text == "2", "2 mm on both");
        assert!((view.track_max - 10.0).abs() < 1e-9, "the smallest Size");
        b.values.letter_spacing = 0.2;
        let view = field_view(LayoutField::LetterSpacing, &[a, b]);
        assert!(view.mixed && view.resettable && view.text.is_empty());

        let change = set_field(LayoutField::LetterSpacing, &[a, b], Some(30.0));
        assert!(
            (change.values[0].letter_spacing - 2.0).abs() < 1e-9,
            "10 mm text held"
        );
        assert!(
            (change.values[1].letter_spacing - 1.5).abs() < 1e-9,
            "30 mm on the 20 mm one"
        );
        assert_eq!(change.limited, 1);
        assert_eq!(change.limit, Some((true, 20.0)));
        let auto = set_field(LayoutField::LineSpacing, &[a], None);
        assert_eq!(auto.values[0].line_spacing, None);
        let turned = set_field(LayoutField::CharacterRotation, &[a], Some(270.0));
        assert!((turned.values[0].rotation.as_radians().to_degrees() + 90.0).abs() < 1e-9);
    }

    #[test]
    fn readouts_and_typed_values() {
        assert_eq!(LayoutField::LetterSpacing.text(-1.2), "\u{2212}1.2");
        assert_eq!(LayoutField::WordSpacing.text(12.345), "12.35");
        assert_eq!(
            LayoutField::CharacterRotation.text(-179.46),
            "\u{2212}179.5"
        );
        assert!(
            (parse_field_text(LayoutField::LetterSpacing, "0.1 in").unwrap() - 2.54).abs() < 1e-9
        );
        assert_eq!(
            parse_field_text(LayoutField::CharacterRotation, "12°"),
            Some(12.0)
        );
        assert_eq!(parse_field_text(LayoutField::WordSpacing, "wide"), None);
    }
}
