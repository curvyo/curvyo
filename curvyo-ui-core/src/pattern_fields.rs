//! The value fields of the Pattern block (`specs/0071-pattern-fill` criterion 2, UX notes §5): each
//! parameter's scale, rounding grid, text, typed range, and the style edit a value makes.

use curvyo_document_core::{
    GeneratorKind, Length, ParamKind, PatternFill, PatternGenerator, PatternParam, StyleEdit,
    pattern_params,
};

use crate::transform_entry::parse_entry_number;
use crate::value_scale::Grid;

/// A field of the Pattern block: a generator parameter or the margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternField {
    /// A parameter of the current generator.
    Param(PatternParam),
    /// The margin.
    Margin,
}

/// The field's scale (UX notes §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scale {
    /// Log 0.1 to 50 mm, typed 0.05 to 1000.
    Length,
    /// The stroke width curve, 0 to 20 mm, typed 0 to 1000.
    Margin,
    /// Signed -180 to 180 degrees, any finite value typed.
    Angle,
    /// 0 to 100 percent.
    Percent,
    /// 1 (or 2) to 50, typed to 500.
    Count,
    /// 0 to 180 degrees.
    Jitter,
}

/// What a field shows.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternFieldView {
    /// The host's name of the field (`"spacing"`, `"margin"`).
    pub name: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// The value as text without the unit; empty when mixed.
    pub text: String,
    /// The shown value (`aria-valuenow`).
    pub value: f64,
    /// The shapes differ.
    pub mixed: bool,
    /// The bar from `bar_from` to `bar`.
    pub bar: f64,
    /// Where the bar starts (the zero mark of a signed field).
    pub bar_from: f64,
    /// The zero mark of a signed field.
    pub zero: Option<f64>,
    /// The unit shown (`"mm"`, `"°"`, `"%"`, empty for a count).
    pub unit: &'static str,
    /// The unit in words.
    pub unit_words: &'static str,
    /// The track's ends.
    pub value_min: f64,
    /// See `value_min`.
    pub value_max: f64,
    /// The reset target as text.
    pub default_text: String,
    /// The reset icon shows.
    pub resettable: bool,
}

const LENGTH_MIN: f64 = 0.1;
const LENGTH_BASE: f64 = 500.0;
const MARGIN_MAX: f64 = 20.0;
const TYPED_MAX_MM: f64 = 1000.0;

impl PatternField {
    /// The field the host names: a parameter's stored name or `"margin"`.
    #[must_use]
    pub fn from_name(kind: GeneratorKind, name: &str) -> Option<Self> {
        if name == "margin" {
            return Some(Self::Margin);
        }
        pattern_params(kind)
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| Self::Param(spec.param))
    }

    fn spec(self, kind: GeneratorKind) -> Option<&'static curvyo_document_core::ParamSpec> {
        match self {
            Self::Param(param) => curvyo_document_core::spec_of(kind, param),
            Self::Margin => None,
        }
    }

    fn scale(self, kind: GeneratorKind) -> Scale {
        match self.spec(kind).map(|spec| spec.kind) {
            None => Scale::Margin,
            Some(ParamKind::Length) => Scale::Length,
            Some(ParamKind::Angle) => Scale::Angle,
            Some(ParamKind::Percent) => Scale::Percent,
            Some(ParamKind::Count) => Scale::Count,
            Some(ParamKind::Jitter) => Scale::Jitter,
        }
    }

    /// The smallest count (Interpolation waves draws at least 2 lines).
    fn count_min(self, kind: GeneratorKind) -> f64 {
        self.spec(kind).map_or(1.0, |spec| spec.min.max(1.0))
    }

    /// The shown value of the field in `pattern` (mm, degrees, percent, count).
    #[must_use]
    pub fn shown(self, pattern: &PatternFill) -> Option<f64> {
        let kind = pattern.generator.kind();
        match self {
            Self::Margin => Some(pattern.margin.as_mm()),
            Self::Param(param) => {
                let stored = pattern.generator.get(param)?;
                Some(if self.scale(kind) == Scale::Percent {
                    stored * 100.0
                } else {
                    stored
                })
            }
        }
    }

    /// The edit that sets the shown `value`.
    #[must_use]
    pub fn edit(self, kind: GeneratorKind, value: f64) -> StyleEdit {
        match self {
            Self::Margin => StyleEdit::PatternMargin(Length::from_mm(value)),
            Self::Param(param) => {
                let stored = match self.scale(kind) {
                    Scale::Percent => value / 100.0,
                    Scale::Angle => normalized_degrees(value),
                    _ => value,
                };
                StyleEdit::PatternParam(param, stored)
            }
        }
    }

    /// The default's shown value.
    #[must_use]
    pub fn default_value(self, kind: GeneratorKind) -> f64 {
        match self.spec(kind) {
            None => 0.0,
            Some(spec) if spec.kind == ParamKind::Percent => spec.default * 100.0,
            Some(spec) => spec.default,
        }
    }

    /// The value at track position `p` (0 to 1), rounded to `grid`.
    #[must_use]
    pub fn value_at(self, kind: GeneratorKind, p: f64, grid: Grid) -> f64 {
        let p = if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let raw = match self.scale(kind) {
            Scale::Length => LENGTH_MIN * LENGTH_BASE.powf(p),
            Scale::Margin => MARGIN_MAX * (100.0_f64.powf(p) - 1.0) / 99.0,
            Scale::Angle => -180.0 + 360.0 * p,
            Scale::Percent => 100.0 * p,
            Scale::Count => self.count_min(kind) + (50.0 - self.count_min(kind)) * p,
            Scale::Jitter => 180.0 * p,
        };
        self.round(kind, raw, grid)
    }

    /// The track position of `value`, 0 to 1.
    #[must_use]
    pub fn position_of(self, kind: GeneratorKind, value: f64) -> f64 {
        let p = match self.scale(kind) {
            Scale::Length => (value.max(LENGTH_MIN) / LENGTH_MIN).ln() / LENGTH_BASE.ln(),
            Scale::Margin => (1.0 + value.max(0.0) * 99.0 / MARGIN_MAX).ln() / 100.0_f64.ln(),
            Scale::Angle => (normalized_degrees(value) + 180.0) / 360.0,
            Scale::Percent => value / 100.0,
            Scale::Count => (value - self.count_min(kind)) / (50.0 - self.count_min(kind)),
            Scale::Jitter => value / 180.0,
        };
        if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The spacing of the field's grid: Shift coarse, Ctrl fine (UX notes §5).
    fn grid_step(self, kind: GeneratorKind, grid: Grid) -> f64 {
        match (self.scale(kind), grid) {
            (Scale::Length | Scale::Margin, Grid::Normal) => 0.01,
            (Scale::Length | Scale::Margin, Grid::Coarse)
            | (Scale::Angle | Scale::Jitter, Grid::Fine) => 0.1,
            (Scale::Length | Scale::Margin, Grid::Fine) => 0.001,
            (Scale::Angle | Scale::Jitter, Grid::Coarse) => 15.0,
            (Scale::Percent | Scale::Count, Grid::Coarse) => 10.0,
            _ => 1.0,
        }
    }

    /// `value` rounded to the field's grid and held in its typed range.
    #[must_use]
    pub fn round(self, kind: GeneratorKind, value: f64, grid: Grid) -> f64 {
        let step = self.grid_step(kind, grid);
        let rounded = ((value / step).round() * step * 1000.0).round() / 1000.0;
        let (min, max) = self.typed_range(kind);
        if rounded.is_finite() {
            rounded.clamp(min, max) + 0.0
        } else {
            min
        }
    }

    /// `steps` arrow-key steps from `value`.
    #[must_use]
    pub fn step(self, kind: GeneratorKind, value: f64, steps: i32, grid: Grid) -> f64 {
        let unit = self.grid_step(kind, grid);
        self.round(kind, value + f64::from(steps) * unit, grid)
    }

    /// The smallest and largest value that may be typed.
    #[must_use]
    pub fn typed_range(self, kind: GeneratorKind) -> (f64, f64) {
        match self.scale(kind) {
            Scale::Length => (
                self.spec(kind)
                    .map_or(curvyo_document_core::MIN_LENGTH_MM, |s| s.min),
                TYPED_MAX_MM,
            ),
            Scale::Margin => (0.0, TYPED_MAX_MM),
            Scale::Angle => (-180.0, 180.0),
            Scale::Percent => (0.0, 100.0),
            Scale::Count => (self.count_min(kind), 500.0),
            Scale::Jitter => (0.0, 180.0),
        }
    }

    /// The field's value as typed text (an optional unit), or why not.
    ///
    /// # Errors
    /// `"range"` for a number outside the typed range, `"number"` for text that is no number.
    pub fn parse(self, kind: GeneratorKind, text: &str) -> Result<f64, &'static str> {
        let trimmed = text.trim();
        let number = trimmed
            .trim_end_matches(['°', '%'])
            .trim_end_matches("mm")
            .trim();
        let value = parse_entry_number(number, true).ok_or("number")?;
        if self.scale(kind) == Scale::Angle {
            return if value.is_finite() {
                Ok(normalized_degrees(value))
            } else {
                Err("number")
            };
        }
        if self.scale(kind) == Scale::Count && value.fract() != 0.0 {
            return Err("range");
        }
        let (min, max) = self.typed_range(kind);
        if (min..=max).contains(&value) {
            Ok(value)
        } else {
            Err("range")
        }
    }

    /// The host's message key for a value of this field that `pattern`'s other fields refuse
    /// (Joint or Wall not smaller than its cell): the field it collides with. The texts are the
    /// host's (`PatternBlock.tsx`).
    #[must_use]
    pub fn misfit(self, pattern: &PatternFill) -> &'static str {
        match (self, pattern.generator) {
            (Self::Param(PatternParam::CellSize), _) => "wall",
            (Self::Param(PatternParam::BrickWidth | PatternParam::BrickHeight), _) => "joint",
            (
                Self::Param(PatternParam::Joint),
                PatternGenerator::BrickWall { width, height, .. },
            ) if width.as_mm() < height.as_mm() => "brick_width",
            (Self::Param(PatternParam::Joint), _) => "brick_height",
            _ => "cell_size",
        }
    }

    /// The field's unit and its words.
    const fn units(scale: Scale) -> (&'static str, &'static str) {
        match scale {
            Scale::Length | Scale::Margin => ("mm", "millimetres"),
            Scale::Angle | Scale::Jitter => ("°", "degrees"),
            Scale::Percent => ("%", "percent"),
            Scale::Count => ("", ""),
        }
    }

    /// `value` as the field shows it.
    fn text(self, kind: GeneratorKind, value: f64) -> String {
        match self.scale(kind) {
            Scale::Length | Scale::Margin => format!("{}", (value * 1000.0).round() / 1000.0 + 0.0),
            Scale::Angle | Scale::Jitter => format!("{}", (value * 10.0).round() / 10.0 + 0.0),
            Scale::Percent | Scale::Count => format!("{}", value.round() + 0.0),
        }
    }

    /// What the field shows for `values` of the edited shapes (one value, or several: mixed).
    #[must_use]
    pub fn view(self, kind: GeneratorKind, values: &[f64]) -> PatternFieldView {
        let scale = self.scale(kind);
        let (name, label) = match self.spec(kind) {
            Some(spec) => (spec.name, spec.label),
            None => ("margin", "Margin"),
        };
        let first = values.first().copied().unwrap_or(0.0);
        let mixed = values.iter().any(|v| (v - first).abs() > 1e-9);
        let (unit, unit_words) = Self::units(scale);
        let default = self.default_value(kind);
        let zero = (scale == Scale::Angle).then_some(0.5_f64);
        let at = if mixed {
            0.0
        } else {
            self.position_of(kind, first)
        };
        // A signed field's bar runs between its zero mark and the value.
        let (bar_from, bar) = match zero {
            Some(z) if !mixed => (z.min(at), z.max(at)),
            Some(z) => (z, z),
            None => (0.0, at),
        };
        let (value_min, value_max) = match scale {
            Scale::Length => (LENGTH_MIN, LENGTH_MIN * LENGTH_BASE),
            Scale::Margin => (0.0, MARGIN_MAX),
            Scale::Count => (self.count_min(kind), 50.0),
            _ => self.typed_range(kind),
        };
        PatternFieldView {
            name,
            label,
            text: if mixed {
                String::new()
            } else {
                self.text(kind, first)
            },
            value: first,
            mixed,
            bar,
            bar_from,
            zero,
            unit,
            unit_words,
            value_min,
            value_max,
            default_text: format!("{} {unit}", self.text(kind, default))
                .trim_end()
                .to_owned(),
            resettable: mixed || (first - default).abs() > 1e-9,
        }
    }
}

/// Degrees held in `(-180, 180]`.
fn normalized_degrees(value: f64) -> f64 {
    let wrapped = value.rem_euclid(360.0);
    (if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }) + 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_scale_ends_and_inverse() {
        let field = PatternField::Param(PatternParam::Spacing);
        let kind = GeneratorKind::Hatch;
        assert_eq!(field.value_at(kind, 0.0, Grid::Normal), 0.1);
        assert_eq!(field.value_at(kind, 1.0, Grid::Normal), 50.0);
        for v in [0.5, 2.0, 10.0] {
            let p = field.position_of(kind, v);
            assert!(
                (field.value_at(kind, p, Grid::Fine) - v).abs() < 0.002,
                "{v}"
            );
        }
    }

    #[test]
    fn typed_values_are_checked_and_angles_wrap() {
        let kind = GeneratorKind::Hatch;
        let spacing = PatternField::Param(PatternParam::Spacing);
        assert_eq!(spacing.parse(kind, "2,5 mm"), Ok(2.5));
        assert_eq!(spacing.parse(kind, "0.01"), Err("range"));
        assert_eq!(spacing.parse(kind, "abc"), Err("number"));
        let angle = PatternField::Param(PatternParam::Angle);
        assert_eq!(angle.parse(kind, "270°"), Ok(-90.0));
        let lines = PatternField::Param(PatternParam::Lines);
        assert_eq!(
            lines.parse(GeneratorKind::InterpolationWaves, "1"),
            Err("range")
        );
        assert_eq!(
            lines.parse(GeneratorKind::InterpolationWaves, "2.5"),
            Err("range")
        );
    }

    /// UI review 2026-11-02, item 5: the cross-field message names the field the value collides
    /// with: the cell size for Wall, the smaller brick side for Joint, Joint or Wall for the cell.
    #[test]
    fn a_cross_field_refusal_names_the_other_field() {
        let bricks = |width: f64, height: f64| PatternFill {
            generator: PatternGenerator::BrickWall {
                width: Length::from_mm(width),
                height: Length::from_mm(height),
                joint: Length::from_mm(0.5),
            },
            ..PatternFill::default()
        };
        let comb = PatternFill {
            generator: PatternGenerator::defaults(GeneratorKind::Honeycomb),
            ..PatternFill::default()
        };
        let field = PatternField::Param;
        assert_eq!(field(PatternParam::Wall).misfit(&comb), "cell_size");
        assert_eq!(field(PatternParam::CellSize).misfit(&comb), "wall");
        assert_eq!(
            field(PatternParam::Joint).misfit(&bricks(10.0, 5.0)),
            "brick_height"
        );
        assert_eq!(
            field(PatternParam::Joint).misfit(&bricks(4.0, 5.0)),
            "brick_width"
        );
        assert_eq!(
            field(PatternParam::BrickWidth).misfit(&bricks(10.0, 5.0)),
            "joint"
        );
        assert_eq!(
            field(PatternParam::BrickHeight).misfit(&bricks(10.0, 5.0)),
            "joint"
        );
    }

    #[test]
    fn a_percent_field_edits_the_share() {
        let kind = GeneratorKind::LeafForest;
        let density = PatternField::Param(PatternParam::Density);
        let pattern = PatternFill {
            generator: PatternGenerator::defaults(kind),
            ..PatternFill::default()
        };
        assert_eq!(density.shown(&pattern), Some(50.0));
        assert_eq!(
            density.edit(kind, 25.0),
            StyleEdit::PatternParam(PatternParam::Density, 0.25)
        );
        let view = density.view(kind, &[50.0]);
        assert_eq!(
            (view.text.as_str(), view.unit, view.resettable),
            ("50", "%", false)
        );
    }
}
