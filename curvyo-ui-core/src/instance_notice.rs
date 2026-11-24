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
        _ => String::new(),
    }
}

/// The notice sentence of one clamp (UX notes of `0141` §6), e.g. "Teeth set to 6, the minimum."
#[must_use]
pub fn clamp_notice(kind: &GeneratorKind, clamp: &Clamp) -> Option<String> {
    let param = kind.params().find(|param| param.key == clamp.key)?;
    let value = say(param, &clamp.value);
    let name = kind.label.to_lowercase();
    Some(match clamp.bound {
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
