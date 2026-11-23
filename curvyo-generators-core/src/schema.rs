//! Small constructors for the parameter rows of the two generators.

use curvyo_document_core::Length;
use curvyo_extension_core::{ChoiceOption, Degrees, Param, ParamKind, ParamValue, ShownWhen};

/// A length row.
pub(crate) fn length(
    key: &'static str,
    label: &'static str,
    (min, max): (f64, f64),
    default: f64,
    (step, decimals): (f64, u8),
) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Length {
            min: Length::from_mm(min),
            max: Length::from_mm(max),
            step,
            decimals,
        },
        default: ParamValue::Length(Length::from_mm(default)),
        shown_when: None,
    }
}

/// A whole-number row.
pub(crate) fn count(
    key: &'static str,
    label: &'static str,
    min: u32,
    max: u32,
    default: u32,
) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Count { min, max },
        default: ParamValue::Count(default),
        shown_when: None,
    }
}

/// An angle row in degrees.
pub(crate) fn angle(
    key: &'static str,
    label: &'static str,
    (min, max): (f64, f64),
    default: f64,
) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Angle {
            min: Degrees(min),
            max: Degrees(max),
            step: 0.5,
            decimals: 1,
        },
        default: ParamValue::Angle(Degrees(default)),
        shown_when: None,
    }
}

/// A switch row, off by default.
pub(crate) fn switch(key: &'static str, label: &'static str) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Switch,
        default: ParamValue::Switch(false),
        shown_when: None,
    }
}

/// A choice row; the default is the first option.
pub(crate) fn choice(
    key: &'static str,
    label: &'static str,
    options: &'static [ChoiceOption],
    default: &'static str,
) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Choice { options },
        default: ParamValue::Choice(default.to_owned()),
        shown_when: None,
    }
}

/// `param` drawn only while `condition` holds.
pub(crate) fn when(mut param: Param, condition: ShownWhen) -> Param {
    param.shown_when = Some(condition);
    param
}
