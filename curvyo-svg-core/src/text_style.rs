//! The text properties of the cascade (`specs/0024-svg-import-export` criteria 80, 81, 83, 85,
//! 88 to 91, 96, 97): parsed per declaration into the computed style, inherited down the tree
//! as CSS says, lengths in user units by the fixed CSS factors of criterion 83.

use std::rc::Rc;

use svgtypes::{Length, LengthUnit};

use crate::drawing_text::{
    Baseline, BaselineShift, FlowAlign, LineHeight, SvgOrientation, SvgWritingMode, TextAnchor,
};

/// The presentation attributes of text (SVG 1.1 and the SVG 2 ones Inkscape writes).
pub(crate) const TEXT_PROPERTIES: [&str; 21] = [
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "font-stretch",
    "letter-spacing",
    "word-spacing",
    "text-anchor",
    "writing-mode",
    "text-orientation",
    "glyph-orientation-vertical",
    "line-height",
    "white-space",
    "dominant-baseline",
    "alignment-baseline",
    "baseline-shift",
    "text-transform",
    "text-align",
    "inline-size",
    "shape-inside",
    "shape-padding",
];

/// The font size with no value (criterion 83): 16 px.
pub(crate) const DEFAULT_FONT_SIZE: f64 = 16.0;

/// `text-transform`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TextTransform {
    #[default]
    None,
    Upper,
    Lower,
    Capitalize,
}

/// The text properties of one element after the cascade.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextProps {
    /// The raw `font-family` value.
    pub(crate) family: Option<Rc<str>>,
    pub(crate) font_size: f64,
    pub(crate) weight: u16,
    pub(crate) italic: bool,
    pub(crate) stretch: u16,
    pub(crate) letter_spacing: f64,
    pub(crate) word_spacing: f64,
    pub(crate) anchor: TextAnchor,
    pub(crate) writing_mode: SvgWritingMode,
    pub(crate) orientation: SvgOrientation,
    pub(crate) line_height: LineHeight,
    /// `white-space: pre`, `pre-wrap` or `break-spaces`.
    pub(crate) pre: bool,
    pub(crate) dominant: Baseline,
    /// `alignment-baseline` of this element only (not inherited).
    pub(crate) alignment: Option<Baseline>,
    pub(crate) baseline_shift: BaselineShift,
    pub(crate) transform: TextTransform,
    pub(crate) flow_align: FlowAlign,
    /// `inline-size` of this element only, user units.
    pub(crate) inline_size: Option<f64>,
    /// `shape-inside: url(#id)` of this element only.
    pub(crate) shape_inside: Option<Rc<str>>,
    /// `shape-padding` of this element only, user units.
    pub(crate) shape_padding: f64,
}

impl Default for TextProps {
    fn default() -> Self {
        Self {
            family: None,
            font_size: DEFAULT_FONT_SIZE,
            weight: 400,
            italic: false,
            stretch: 5,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            anchor: TextAnchor::Start,
            writing_mode: SvgWritingMode::Horizontal,
            orientation: SvgOrientation::Auto,
            line_height: LineHeight::Normal,
            pre: false,
            dominant: Baseline::Alphabetic,
            alignment: None,
            baseline_shift: BaselineShift::None,
            transform: TextTransform::None,
            flow_align: FlowAlign::Start,
            inline_size: None,
            shape_inside: None,
            shape_padding: 0.0,
        }
    }
}

impl TextProps {
    /// The inherited part of `parent`: the per-element values start empty.
    pub(crate) fn inherited(parent: &Self) -> Self {
        Self {
            alignment: None,
            inline_size: None,
            shape_inside: None,
            shape_padding: 0.0,
            ..parent.clone()
        }
    }

    /// The baseline this element's characters sit on.
    pub(crate) fn baseline(&self) -> Baseline {
        self.alignment.unwrap_or(self.dominant)
    }

    /// Applies `font-size` first (other lengths depend on it); `parent` is the parent's props.
    pub(crate) fn apply_font_size(&mut self, value: &str, parent: &Self) {
        if let Some(size) = font_size(value, parent.font_size) {
            self.font_size = size;
        }
    }

    /// Applies one declaration other than `font-size`; an unreadable value changes nothing.
    pub(crate) fn apply(&mut self, name: &str, value: &str, parent: &Self) {
        let size = self.font_size;
        match name {
            "font-family" => {
                if !value.is_empty() {
                    self.family = Some(Rc::from(value));
                }
            }
            "font-weight" => {
                if let Some(weight) = font_weight(value, parent.weight) {
                    self.weight = weight;
                }
            }
            "font-style" => match value {
                "italic" | "oblique" => self.italic = true,
                "normal" => self.italic = false,
                _ if value.starts_with("oblique") => self.italic = true,
                _ => {}
            },
            "font-stretch" => {
                if let Some(class) = font_stretch(value) {
                    self.stretch = class;
                }
            }
            "letter-spacing" => {
                if let Some(v) = spacing(value, size) {
                    self.letter_spacing = v;
                }
            }
            "word-spacing" => {
                if let Some(v) = spacing(value, size) {
                    self.word_spacing = v;
                }
            }
            _ => self.apply_layout(name, value, size),
        }
    }

    /// Applies a declaration of the line, its alignment and baselines and the flowed frame.
    fn apply_layout(&mut self, name: &str, value: &str, size: f64) {
        match name {
            "text-anchor" => {
                self.anchor = match value {
                    "start" => TextAnchor::Start,
                    "middle" => TextAnchor::Middle,
                    "end" => TextAnchor::End,
                    _ => self.anchor,
                };
            }
            "writing-mode" => {
                self.writing_mode = match value {
                    "horizontal-tb" | "lr" | "lr-tb" | "rl" | "rl-tb" => SvgWritingMode::Horizontal,
                    "vertical-rl" | "tb-rl" | "tb" => SvgWritingMode::VerticalRl,
                    "vertical-lr" => SvgWritingMode::VerticalLr,
                    _ => self.writing_mode,
                };
            }
            "text-orientation" => {
                self.orientation = match value {
                    "mixed" => SvgOrientation::Auto,
                    "upright" => SvgOrientation::Upright,
                    "sideways" | "sideways-right" => SvgOrientation::Sideways,
                    _ => self.orientation,
                };
            }
            "glyph-orientation-vertical" => {
                self.orientation = match value.trim_end_matches("deg") {
                    "auto" => SvgOrientation::Auto,
                    "0" => SvgOrientation::Upright,
                    "90" => SvgOrientation::Sideways,
                    _ => self.orientation,
                };
            }
            "line-height" => {
                if let Some(height) = line_height(value, size) {
                    self.line_height = height;
                }
            }
            "white-space" => match value {
                "pre" | "pre-wrap" | "break-spaces" => self.pre = true,
                "normal" | "nowrap" | "pre-line" => self.pre = false,
                _ => {}
            },
            "dominant-baseline" => {
                if let Some(b) = baseline(value) {
                    self.dominant = b;
                }
            }
            "alignment-baseline" => self.alignment = baseline(value),
            "baseline-shift" => {
                if let Some(shift) = baseline_shift(value, size) {
                    self.baseline_shift = shift;
                }
            }
            "text-transform" => {
                self.transform = match value {
                    "uppercase" => TextTransform::Upper,
                    "lowercase" => TextTransform::Lower,
                    "capitalize" => TextTransform::Capitalize,
                    "none" => TextTransform::None,
                    _ => self.transform,
                };
            }
            "text-align" => {
                self.flow_align = match value {
                    "start" | "left" => FlowAlign::Start,
                    "center" => FlowAlign::Centre,
                    "end" | "right" => FlowAlign::End,
                    "justify" => FlowAlign::Justify,
                    _ => self.flow_align,
                };
            }
            "inline-size" => self.inline_size = css_length(value, size).filter(|w| *w > 0.0),
            "shape-inside" => {
                self.shape_inside = value
                    .strip_prefix("url(")
                    .and_then(|rest| rest.strip_suffix(')'))
                    .map(|id| id.trim().trim_matches(['"', '\'']))
                    .and_then(|id| id.strip_prefix('#'))
                    .map(Rc::from);
            }
            "shape-padding" => {
                if let Some(padding) = css_length(value, size) {
                    self.shape_padding = padding.max(0.0);
                }
            }
            _ => {}
        }
    }

    /// `inherit` for a text property.
    pub(crate) fn inherit(&mut self, name: &str, parent: &Self) {
        match name {
            "font-family" => self.family.clone_from(&parent.family),
            "font-size" => self.font_size = parent.font_size,
            "font-weight" => self.weight = parent.weight,
            "font-style" => self.italic = parent.italic,
            "font-stretch" => self.stretch = parent.stretch,
            "letter-spacing" => self.letter_spacing = parent.letter_spacing,
            "word-spacing" => self.word_spacing = parent.word_spacing,
            "text-anchor" => self.anchor = parent.anchor,
            "writing-mode" => self.writing_mode = parent.writing_mode,
            "text-orientation" | "glyph-orientation-vertical" => {
                self.orientation = parent.orientation;
            }
            "line-height" => self.line_height = parent.line_height,
            "white-space" => self.pre = parent.pre,
            "dominant-baseline" => self.dominant = parent.dominant,
            "baseline-shift" => self.baseline_shift = parent.baseline_shift,
            "text-transform" => self.transform = parent.transform,
            "text-align" => self.flow_align = parent.flow_align,
            _ => {}
        }
    }
}

/// A CSS length in user units (criterion 83): `px` and a bare number 1, `pt` 4/3, `pc` 16, `in`
/// 96, `cm` 96/2.54, `mm` 96/25.4; `em` and `%` of `font_size`, `ex` half of it.
pub(crate) fn css_length(text: &str, font_size: f64) -> Option<f64> {
    let length: Length = text.trim().parse().ok()?;
    let n = length.number;
    let value = match length.unit {
        LengthUnit::None | LengthUnit::Px => n,
        LengthUnit::Pt => n * 4.0 / 3.0,
        LengthUnit::Pc => n * 16.0,
        LengthUnit::In => n * 96.0,
        LengthUnit::Cm => n * 96.0 / 2.54,
        LengthUnit::Mm => n * 96.0 / 25.4,
        LengthUnit::Em => n * font_size,
        LengthUnit::Percent => n / 100.0 * font_size,
        LengthUnit::Ex => n * font_size / 2.0,
    };
    value.is_finite().then_some(value)
}

/// `font-size` in user units under a parent of `parent` (criterion 83).
fn font_size(value: &str, parent: f64) -> Option<f64> {
    let keyword = match value {
        "xx-small" => Some(9.0),
        "x-small" => Some(10.0),
        "small" => Some(13.0),
        "medium" => Some(16.0),
        "large" => Some(18.0),
        "x-large" => Some(24.0),
        "xx-large" => Some(32.0),
        "larger" => Some(parent * 1.2),
        "smaller" => Some(parent / 1.2),
        _ => None,
    };
    keyword.or_else(|| css_length(value, parent))
}

/// `font-weight` (criterion 81): `bolder` and `lighter` by the CSS table.
fn font_weight(value: &str, parent: u16) -> Option<u16> {
    match value {
        "normal" => Some(400),
        "bold" => Some(700),
        "bolder" => Some(match parent {
            0..350 => 400,
            350..550 => 700,
            _ => 900,
        }),
        "lighter" => Some(match parent {
            0..550 => 100,
            550..750 => 400,
            _ => 700,
        }),
        number => {
            let n: f64 = number.parse().ok()?;
            // invariant: the value is clamped to 1..=1000 before the cast.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            (n.is_finite() && (1.0..=1000.0).contains(&n)).then(|| n.round() as u16)
        }
    }
}

/// `font-stretch` as a width class 1 to 9.
fn font_stretch(value: &str) -> Option<u16> {
    const KEYWORDS: [&str; 9] = [
        "ultra-condensed",
        "extra-condensed",
        "condensed",
        "semi-condensed",
        "normal",
        "semi-expanded",
        "expanded",
        "extra-expanded",
        "ultra-expanded",
    ];
    const PERCENT: [f64; 9] = [50.0, 62.5, 75.0, 87.5, 100.0, 112.5, 125.0, 150.0, 200.0];
    if let Some(at) = KEYWORDS.iter().position(|k| *k == value) {
        return u16::try_from(at + 1).ok();
    }
    let percent: f64 = value.strip_suffix('%')?.trim().parse().ok()?;
    let nearest = PERCENT
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - percent).abs().total_cmp(&(b.1 - percent).abs()))?
        .0;
    u16::try_from(nearest + 1).ok()
}

/// `letter-spacing` or `word-spacing`: `normal` is 0.
fn spacing(value: &str, font_size: f64) -> Option<f64> {
    if value == "normal" {
        return Some(0.0);
    }
    css_length(value, font_size)
}

fn line_height(value: &str, font_size: f64) -> Option<LineHeight> {
    if value == "normal" {
        return Some(LineHeight::Normal);
    }
    if let Some(percent) = value.strip_suffix('%') {
        let n: f64 = percent.trim().parse().ok()?;
        return (n.is_finite() && n >= 0.0).then_some(LineHeight::Factor(n / 100.0));
    }
    if let Ok(n) = value.parse::<f64>() {
        return (n.is_finite() && n >= 0.0).then_some(LineHeight::Factor(n));
    }
    css_length(value, font_size)
        .filter(|n| *n >= 0.0)
        .map(LineHeight::Length)
}

fn baseline(value: &str) -> Option<Baseline> {
    Some(match value {
        "middle" => Baseline::Middle,
        "central" => Baseline::Central,
        "hanging" => Baseline::Hanging,
        "text-top" | "text-before-edge" => Baseline::TextTop,
        "text-bottom" | "text-after-edge" | "ideographic" => Baseline::TextBottom,
        "mathematical" => Baseline::Mathematical,
        "auto" | "alphabetic" | "baseline" | "use-script" | "no-change" | "reset-size"
        | "inherit" => Baseline::Alphabetic,
        _ => return None,
    })
}

fn baseline_shift(value: &str, font_size: f64) -> Option<BaselineShift> {
    match value {
        "baseline" => Some(BaselineShift::None),
        "super" => Some(BaselineShift::Super),
        "sub" => Some(BaselineShift::Sub),
        other => css_length(other, font_size).map(BaselineShift::Up),
    }
}

/// The `font-family` list of `value`: comma-separated, quotes optional, generic names mapped;
/// a name is cut at 128 characters (criterion 98). Never empty.
pub(crate) fn families(value: Option<&str>) -> Vec<crate::drawing_text::FontFamily> {
    use crate::drawing_text::FontFamily;
    let mut out = Vec::new();
    for entry in value.unwrap_or("").split(',') {
        let entry = entry.trim();
        let quoted = entry.len() >= 2
            && ((entry.starts_with('"') && entry.ends_with('"'))
                || (entry.starts_with('\'') && entry.ends_with('\'')));
        let name = if quoted {
            &entry[1..entry.len() - 1]
        } else {
            entry
        };
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            continue;
        }
        let family = match (quoted, name.to_ascii_lowercase().as_str()) {
            (false, "sans-serif" | "system-ui" | "monospace" | "fantasy") => FontFamily::SansSerif,
            (false, "serif") => FontFamily::Serif,
            (false, "cursive") => FontFamily::Cursive,
            _ => FontFamily::Named(name.chars().take(128).collect()),
        };
        out.push(family);
    }
    if out.is_empty() {
        out.push(FontFamily::SansSerif);
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::drawing_text::FontFamily;

    #[test]
    fn ac83_font_sizes_in_user_units() {
        assert_eq!(font_size("10.5833px", 16.0), Some(10.5833));
        assert_eq!(font_size("12pt", 16.0), Some(16.0));
        assert_eq!(font_size("2em", 10.0), Some(20.0));
        assert_eq!(font_size("50%", 10.0), Some(5.0));
        assert_eq!(font_size("x-large", 10.0), Some(24.0));
        assert!((font_size("smaller", 12.0).unwrap() - 10.0).abs() < 1e-12);
        assert!((font_size("1in", 16.0).unwrap() - 96.0).abs() < 1e-12);
    }

    #[test]
    fn ac81_weights_and_widths() {
        assert_eq!(font_weight("bold", 400), Some(700));
        assert_eq!(font_weight("bolder", 300), Some(400));
        assert_eq!(font_weight("bolder", 400), Some(700));
        assert_eq!(font_weight("bolder", 600), Some(900));
        assert_eq!(font_weight("lighter", 700), Some(400));
        assert_eq!(font_weight("lighter", 800), Some(700));
        assert_eq!(font_weight("600", 400), Some(600));
        assert_eq!(font_weight("0", 400), None);
        assert_eq!(font_stretch("condensed"), Some(3));
        assert_eq!(font_stretch("80%"), Some(3));
    }

    #[test]
    fn ac80_family_lists() {
        assert_eq!(
            families(Some("'Lobster', cursive")),
            [FontFamily::Named("Lobster".into()), FontFamily::Cursive]
        );
        assert_eq!(families(None), [FontFamily::SansSerif]);
        assert_eq!(
            families(Some("\"serif\", monospace")),
            [FontFamily::Named("serif".into()), FontFamily::SansSerif]
        );
        let long = "x".repeat(300);
        assert_eq!(families(Some(&long)), [FontFamily::Named("x".repeat(128))]);
    }
}
