//! The notice sentences of the values a generator clamped, and the numbers as a field shows them.

use curvyo_extension_core::{Bound, Clamp, GeneratorKind, Param, ParamKind, ParamValue};

/// A number as the field shows it: at most `decimals` places, trailing zeros trimmed.
#[must_use]
pub fn shown(value: f64, decimals: u8) -> String {
    let text = format!("{value:.*}", usize::from(decimals));
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

/// A millimetre or degree number as a notice says it: "120 mm" or "30°".
pub(crate) fn say_number(angle: bool, number: f64) -> String {
    if angle {
        format!("{}°", shown(number, 1))
    } else {
        format!("{} mm", shown(number, 2))
    }
}

/// The value as the notice says it, with its unit.
fn say(param: &Param, value: &ParamValue) -> String {
    match (param.kind, value) {
        (ParamKind::Length { decimals, .. }, ParamValue::Length(length)) => {
            format!("{} mm", shown(length.as_mm(), decimals))
        }
        (ParamKind::Angle { decimals, .. }, ParamValue::Angle(degrees)) => {
            format!("{}°", shown(degrees.0, decimals))
        }
        (_, ParamValue::Count(count)) => count.to_string(),
        (ParamKind::Percent { min, .. }, ParamValue::Percent(percent)) if min < 0 => {
            // A signed percentage says its sign: "+30 %", "-5 %".
            format!("{percent:+} %")
        }
        (_, ParamValue::Percent(percent)) => format!("{percent} %"),
        (ParamKind::Ratio { decimals, .. }, ParamValue::Ratio(ratio)) => {
            format!("{} : 1", shown(*ratio, decimals))
        }
        (ParamKind::Choice { options }, ParamValue::Choice(id)) => options
            .iter()
            .find(|option| option.id == id)
            // "Q 25 %" is said "Q": the first word names the option.
            .and_then(|option| option.label.split_whitespace().next())
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    }
}

/// A kind's label inside a sentence: "gear", "rounded box", but "QR code" keeps its acronym and
/// "Code 128" its number, "Data Matrix" its second capital (a name, not a noun).
#[must_use]
pub fn noun(label: &str) -> String {
    let mut chars = label.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if first.is_uppercase() && second.is_uppercase() => {
            label.to_owned()
        }
        _ if label.chars().any(|c| c.is_ascii_digit()) => label.to_owned(),
        // "Data Matrix" is a name of two capitalised words.
        _ if label.chars().skip(1).any(char::is_uppercase) => label.to_owned(),
        _ => label.to_lowercase(),
    }
}

/// The notice sentence of one clamp (UX notes of `0141` §6), e.g. "Teeth set to 6, the minimum."
#[must_use]
pub fn clamp_notice(kind: &GeneratorKind, clamp: &Clamp) -> Option<String> {
    let param = kind.params().find(|param| param.key == clamp.key)?;
    let value = say(param, &clamp.value);
    let name = noun(kind.label);
    Some(match clamp.bound {
        Bound::DependentFor(subject) => {
            // "Logo size" (group and row), but "Error correction" once.
            let group = kind
                .groups
                .iter()
                .find(|group| group.params.iter().any(|p| p.key == clamp.key))
                .map_or("", |group| group.label);
            let subject_of_clamp = if group == param.label || group.is_empty() {
                param.label.to_owned()
            } else {
                format!("{group} {}", param.label.to_lowercase())
            };
            format!("{subject_of_clamp} set to {value}, the most {subject} allows.")
        }
        Bound::Minimum => format!("{} set to {value}, the minimum.", param.label),
        Bound::Maximum => format!("{} set to {value}, the maximum.", param.label),
        Bound::Dependent => format!(
            "{} set to {value}, the most this {name} allows.",
            param.label
        ),
        Bound::DependentMinimumFor(subject) => {
            format!(
                "{} set to {value}, the least the {subject} allows.",
                param.label
            )
        }
        Bound::DependentMinimum => format!(
            "{} set to {value}, the least this {name} allows.",
            param.label
        ),
    })
}

pub(crate) fn notices(kind: &GeneratorKind, clamps: &[Clamp]) -> Vec<String> {
    clamps
        .iter()
        .filter_map(|clamp| clamp_notice(kind, clamp))
        .collect()
}
