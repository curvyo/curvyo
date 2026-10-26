//! The value fields of the text layout block (`specs/0079-text-layout-controls` criteria 7, 10,
//! 12, 13, 16, 26 to 28, 27b; UX notes §0, §6 and §10.3): their scales relative to Size, what they show
//! for one or several subjects, and the values a change sets.

use std::ops::RangeInclusive;

use curvyo_document_core::{
    Angle, CharValues, KERN_X_FACTORS, KERN_Y_FACTORS, LETTER_SPACING_FACTORS,
    LINE_SPACING_FACTORS, TAPER_FRACTIONS, TextCharsLabel, TextLayoutLabel, TextLayoutValues,
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
    /// Taper, percent (criterion 16; in the block since the customer's decision of 2026-10-25).
    Taper,
    /// Horizontal kerning of single characters, mm (criterion 26).
    HorizontalKerning,
    /// Vertical kerning of single characters, mm (criterion 26).
    VerticalKerning,
    /// The own rotation of single characters, degrees (criterion 27b).
    CharRotation,
}

/// One subject as the fields see it: a text, or for the kerning fields one character of a text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSubject {
    /// The text's Size, mm.
    pub size_mm: f64,
    /// Its layout values.
    pub values: TextLayoutValues,
    /// Its face's natural line spacing at that size, mm (the Auto readout).
    pub natural_line_mm: f64,
    /// The character's values (the kerning fields).
    pub kern: CharValues,
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
    /// Each subject's new character values, in order.
    pub kerns: Vec<CharValues>,
    /// How many texts were held to their own limit.
    pub limited: usize,
    /// When any was: whether at the top (`true`) or the bottom of the range, and that text's
    /// limit in mm.
    pub limit: Option<(bool, f64)>,
}

impl LayoutField {
    /// Every field, in the block's order.
    pub const ALL: [Self; 8] = [
        Self::LetterSpacing,
        Self::WordSpacing,
        Self::LineSpacing,
        Self::CharacterRotation,
        Self::Taper,
        Self::HorizontalKerning,
        Self::VerticalKerning,
        Self::CharRotation,
    ];

    /// The field named by the host (`"letter-spacing"`, `"word-spacing"`, `"line-spacing"`,
    /// `"character-rotation"`, `"taper"`, `"horizontal-kerning"`, `"vertical-kerning"`,
    /// `"char-rotation"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "letter-spacing" => Some(Self::LetterSpacing),
            "word-spacing" => Some(Self::WordSpacing),
            "line-spacing" => Some(Self::LineSpacing),
            "character-rotation" => Some(Self::CharacterRotation),
            "taper" => Some(Self::Taper),
            "horizontal-kerning" => Some(Self::HorizontalKerning),
            "vertical-kerning" => Some(Self::VerticalKerning),
            "char-rotation" => Some(Self::CharRotation),
            _ => None,
        }
    }

    /// Whether the field acts on single characters (criteria 26, 27b).
    #[must_use]
    pub const fn is_per_character(self) -> bool {
        matches!(
            self,
            Self::HorizontalKerning | Self::VerticalKerning | Self::CharRotation
        )
    }

    /// Whether the field holds degrees.
    const fn is_rotation(self) -> bool {
        matches!(self, Self::CharacterRotation | Self::CharRotation)
    }

    /// The fixed ends of a field not relative to Size: the rotations in degrees, the taper in
    /// percent; `None` for the lengths.
    const fn fixed_range(self) -> Option<(f64, f64)> {
        match self {
            Self::CharacterRotation | Self::CharRotation => Some((-180.0, 180.0)),
            Self::Taper => Some((
                *TAPER_FRACTIONS.start() * 100.0,
                *TAPER_FRACTIONS.end() * 100.0,
            )),
            _ => None,
        }
    }

    /// The step a change (or, with `clear`, the reset outside edit mode) of a per-character field
    /// is: "Change kerning", "Clear kerning", "Change per-character rotation", "Clear
    /// per-character rotation"; `None` for the other fields.
    #[must_use]
    pub const fn chars_label(self, clear: bool) -> Option<TextCharsLabel> {
        match (self, clear) {
            (Self::HorizontalKerning | Self::VerticalKerning, false) => {
                Some(TextCharsLabel::Kerning)
            }
            (Self::HorizontalKerning | Self::VerticalKerning, true) => {
                Some(TextCharsLabel::ClearKerning)
            }
            (Self::CharRotation, false) => Some(TextCharsLabel::CharRotation),
            (Self::CharRotation, true) => Some(TextCharsLabel::ClearCharRotation),
            _ => None,
        }
    }

    /// The step a change of this field is; `None` for the per-character fields
    /// ([`Self::chars_label`]).
    #[must_use]
    pub const fn label(self) -> Option<TextLayoutLabel> {
        match self {
            Self::LetterSpacing => Some(TextLayoutLabel::LetterSpacing),
            Self::WordSpacing => Some(TextLayoutLabel::WordSpacing),
            Self::LineSpacing => Some(TextLayoutLabel::LineSpacing),
            Self::CharacterRotation => Some(TextLayoutLabel::CharacterRotation),
            Self::Taper => Some(TextLayoutLabel::Taper),
            Self::HorizontalKerning | Self::VerticalKerning | Self::CharRotation => None,
        }
    }

    /// The typed range as factors of Size (criterion 7); `None` for the rotations and the taper.
    fn typed_factors(self) -> Option<RangeInclusive<f64>> {
        match self {
            Self::LetterSpacing => Some(LETTER_SPACING_FACTORS),
            Self::WordSpacing => Some(WORD_SPACING_FACTORS),
            Self::LineSpacing => Some(LINE_SPACING_FACTORS),
            Self::HorizontalKerning => Some(KERN_X_FACTORS),
            Self::VerticalKerning => Some(KERN_Y_FACTORS),
            Self::CharacterRotation | Self::CharRotation | Self::Taper => None,
        }
    }

    /// The track in mm at the smallest subject Size, in degrees or in percent (UX notes §0.4).
    fn track(self, size_mm: f64) -> (f64, f64) {
        if let Some(range) = self.fixed_range() {
            return range;
        }
        match self {
            Self::LetterSpacing => (-0.25 * size_mm, size_mm),
            Self::WordSpacing => (-0.5 * size_mm, 2.0 * size_mm),
            Self::LineSpacing => (0.0, 3.0 * size_mm),
            Self::CharacterRotation | Self::CharRotation | Self::Taper => (-180.0, 180.0),
            Self::HorizontalKerning => (-0.25 * size_mm, 0.25 * size_mm),
            Self::VerticalKerning => (-0.5 * size_mm, 0.5 * size_mm),
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
            Self::Taper => Some(values.taper * 100.0),
            Self::HorizontalKerning => Some(subject.kern.kern_x * subject.size_mm),
            Self::VerticalKerning => Some(subject.kern.kern_y * subject.size_mm),
            Self::CharRotation => Some(subject.kern.rotation),
        }
    }

    /// The value the field shows for `subject`: Auto shows the natural spacing.
    fn shown_of(self, subject: &LayoutSubject) -> f64 {
        self.value_of(subject).unwrap_or(subject.natural_line_mm)
    }

    /// The value as the field shows it: mm with up to two decimals, degrees and percent with
    /// one, trailing zeros trimmed, U+2212 for minus.
    #[must_use]
    pub fn text(self, value: f64) -> String {
        let scale = if self.fixed_range().is_some() {
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
/// track span per field width, Shift ten times, Ctrl a tenth; rotation 1°, Shift 15°, Ctrl 0.1°;
/// taper 1 %, Shift 10 %, Ctrl 0.1 % (criterion 16).
fn drag_grid(field: LayoutField, subjects: &[LayoutSubject], grid: Grid) -> f64 {
    if field.fixed_range().is_some() {
        return fixed_steps(field, grid);
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

/// The step of a field with fixed ends: rotation 1°, Shift 15°, Ctrl 0.1°; taper 1 %, Shift
/// 10 %, Ctrl 0.1 %.
fn fixed_steps(field: LayoutField, grid: Grid) -> f64 {
    let coarse = if field.is_rotation() { 15.0 } else { 10.0 };
    match grid {
        Grid::Normal => 1.0,
        Grid::Coarse => coarse,
        Grid::Fine => 0.1,
    }
}

/// The value `steps` arrow-key steps from `shown` (UX notes §0.1): spacing 0.1 mm, Shift 1 mm,
/// Ctrl 0.01 mm; rotation and taper as their drag grid, held to their ends.
#[must_use]
pub fn stepped(field: LayoutField, shown: f64, steps: i32, grid: Grid) -> f64 {
    let step = if field.fixed_range().is_some() {
        fixed_steps(field, grid)
    } else {
        match grid {
            Grid::Normal => 0.1,
            Grid::Coarse => 1.0,
            Grid::Fine => 0.01,
        }
    };
    let value = snapped(shown + f64::from(steps) * step, step);
    match field.fixed_range() {
        Some((low, high)) => value.clamp(low, high),
        None => value,
    }
}

/// A typed value in mm (the units of `0052`: "2", "2 mm", "0.1 in"), degrees ("12", "12°") or
/// percent ("12", "12 %"); `None` when it is not a number.
#[must_use]
pub fn parse_field_text(field: LayoutField, text: &str) -> Option<f64> {
    match field {
        LayoutField::CharacterRotation | LayoutField::CharRotation => {
            parse_entry_number(text, true)
        }
        LayoutField::Taper => {
            let trimmed = text.trim();
            parse_entry_number(trimmed.strip_suffix('%').unwrap_or(trimmed), false)
        }
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
    let mut kerns = Vec::with_capacity(subjects.len());
    let values = subjects
        .iter()
        .map(|subject| {
            let mut values = subject.values;
            let mut kern = subject.kern;
            if field == LayoutField::CharacterRotation {
                let degrees = value.unwrap_or(0.0);
                values.rotation = Angle::from_radians(degrees.to_radians()).normalized();
                kerns.push(kern);
                return values;
            }
            if field == LayoutField::Taper {
                let asked = value.unwrap_or(0.0) / 100.0;
                let held = asked.clamp(*TAPER_FRACTIONS.start(), *TAPER_FRACTIONS.end());
                if (held - asked).abs() > LIMITED {
                    limited += 1;
                    limit = Some((asked > held, held * 100.0));
                }
                values.taper = held;
                kerns.push(kern);
                return values;
            }
            if field == LayoutField::CharRotation {
                kern.rotation = value.unwrap_or(0.0);
                kerns.push(kern.held());
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
                LayoutField::HorizontalKerning => kern.kern_x = factor.unwrap_or(0.0),
                LayoutField::VerticalKerning => kern.kern_y = factor.unwrap_or(0.0),
                LayoutField::CharacterRotation | LayoutField::CharRotation | LayoutField::Taper => {
                }
            }
            kerns.push(kern);
            values
        })
        .collect();
    LayoutChange {
        values,
        kerns,
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
            kern: CharValues::DEFAULT,
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

    /// Criteria 26 and 27: the kerning tracks and typed ranges relative to Size; characters with
    /// different kerns show Mixed; a change sets every subject, held to its range.
    #[test]
    fn ac26_kerning_fields() {
        let at = |x: f64, y: f64| LayoutSubject {
            kern: CharValues {
                kern_x: x,
                kern_y: y,
                rotation: 0.0,
            },
            ..subject(10.0)
        };
        let view = field_view(LayoutField::HorizontalKerning, &[at(-0.05, 0.0)]);
        assert!((view.track_min + 2.5).abs() < 1e-9 && (view.track_max - 2.5).abs() < 1e-9);
        assert_eq!(view.text, "\u{2212}0.5");
        assert!(view.resettable && view.zero.is_some());
        let vertical = field_view(LayoutField::VerticalKerning, &[at(0.0, 0.0)]);
        assert!((vertical.track_max - 5.0).abs() < 1e-9 && !vertical.resettable);
        assert!(
            field_view(
                LayoutField::HorizontalKerning,
                &[at(0.1, 0.0), at(0.0, 0.0)]
            )
            .mixed
        );
        let change = set_field(
            LayoutField::VerticalKerning,
            &[at(0.1, 0.0), at(0.0, 0.3)],
            Some(30.0),
        );
        assert_eq!(change.limited, 2);
        assert_eq!(
            change.kerns,
            [
                CharValues {
                    kern_x: 0.1,
                    kern_y: 2.0,
                    rotation: 0.0,
                },
                CharValues {
                    kern_x: 0.0,
                    kern_y: 2.0,
                    rotation: 0.0,
                }
            ]
        );
        assert_eq!(change.limit, Some((true, 20.0)));
    }

    /// Criterion 27b: the per-character rotation reads and steps in degrees as Character rotation,
    /// a change wraps into (-180°, 180°] and keeps the kerns; its steps have their own labels.
    #[test]
    fn ac27b_char_rotation_field() {
        let mut turned = subject(10.0);
        turned.kern = CharValues {
            kern_x: 0.1,
            rotation: -12.34,
            ..CharValues::DEFAULT
        };
        let view = field_view(LayoutField::CharRotation, &[turned]);
        assert_eq!(view.text, "\u{2212}12.3");
        assert!((view.track_min + 180.0).abs() < 1e-9 && view.resettable);
        assert!((stepped(LayoutField::CharRotation, 10.0, 1, Grid::Coarse) - 30.0).abs() < 1e-9);
        let change = set_field(LayoutField::CharRotation, &[turned], Some(270.0));
        assert!((change.kerns[0].rotation + 90.0).abs() < 1e-9);
        assert!((change.kerns[0].kern_x - 0.1).abs() < 1e-12);
        assert_eq!(change.values[0], turned.values);
        assert_eq!(
            LayoutField::CharRotation.chars_label(true),
            Some(TextCharsLabel::ClearCharRotation)
        );
        assert_eq!(
            LayoutField::from_name("char-rotation"),
            Some(LayoutField::CharRotation)
        );
    }

    /// Criterion 16: the taper reads in percent with one decimal, −50 % to 50 % with a zero
    /// mark, steps 1 %, Shift 10 %, Ctrl 0.1 %; a typed value beyond the range is set to the
    /// nearest end and reported; Mixed for differing texts.
    #[test]
    fn ac16_taper_field() {
        let mut tapered = subject(10.0);
        tapered.values.taper = 0.125;
        let view = field_view(LayoutField::Taper, &[tapered]);
        assert_eq!(view.text, "12.5");
        assert!((view.track_min + 50.0).abs() < 1e-9 && (view.track_max - 50.0).abs() < 1e-9);
        assert!((view.zero.unwrap() - 0.5).abs() < 1e-9 && view.resettable);
        assert_eq!((view.drag_min, view.drag_limit), (0.0, 1.0));
        assert!(!field_view(LayoutField::Taper, &[subject(10.0)]).resettable);
        assert!(field_view(LayoutField::Taper, &[tapered, subject(20.0)]).mixed);
        assert!((stepped(LayoutField::Taper, 12.5, 1, Grid::Normal) - 14.0).abs() < 1e-9);
        assert!((stepped(LayoutField::Taper, 45.0, 1, Grid::Coarse) - 50.0).abs() < 1e-9);
        assert!((stepped(LayoutField::Taper, 1.0, -1, Grid::Fine) - 0.9).abs() < 1e-9);
        assert_eq!(parse_field_text(LayoutField::Taper, "20 %"), Some(20.0));
        assert_eq!(
            parse_field_text(LayoutField::Taper, "\u{2212}5"),
            Some(-5.0)
        );
        let change = set_field(LayoutField::Taper, &[tapered, subject(20.0)], Some(80.0));
        assert!(change.values.iter().all(|v| (v.taper - 0.5).abs() < 1e-12));
        assert_eq!(change.limited, 2);
        assert_eq!(change.limit, Some((true, 50.0)));
        let set = set_field(LayoutField::Taper, &[tapered], Some(-20.0));
        assert!((set.values[0].taper + 0.2).abs() < 1e-12 && set.limit.is_none());
        assert_eq!(set.values[0].rotation, tapered.values.rotation);
        assert_eq!(LayoutField::Taper.label(), Some(TextLayoutLabel::Taper));
        assert_eq!(LayoutField::from_name("taper"), Some(LayoutField::Taper));
    }
}
