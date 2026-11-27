//! The small constructors of parameter rows the codes' schemas share.

use curvyo_document_core::Length;
use curvyo_extension_core::{ChoiceOption, Curve, Param, ParamKind, ParamValue, ShownWhen, Track};

pub(crate) fn row(
    key: &'static str,
    label: &'static str,
    kind: ParamKind,
    default: ParamValue,
) -> Param {
    Param {
        key,
        label,
        tooltip: label,
        kind,
        default,
        shown_when: None,
        track: None,
        initial: None,
    }
}

pub(crate) fn length(
    key: &'static str,
    label: &'static str,
    (min, max): (f64, f64),
    default: f64,
    step: f64,
) -> Param {
    row(
        key,
        label,
        ParamKind::Length {
            min: Length::from_mm(min),
            max: Length::from_mm(max),
            step,
            decimals: 2,
            automatic: false,
        },
        ParamValue::Length(Length::from_mm(default)),
    )
}

pub(crate) fn percent(
    key: &'static str,
    label: &'static str,
    (min, max): (i32, i32),
    default: i32,
) -> Param {
    row(
        key,
        label,
        ParamKind::Percent { min, max },
        ParamValue::Percent(default),
    )
}

pub(crate) fn choice(
    key: &'static str,
    label: &'static str,
    options: &'static [ChoiceOption],
    default: &str,
) -> Param {
    row(
        key,
        label,
        ParamKind::Choice { options },
        ParamValue::Choice(default.to_owned()),
    )
}

pub(crate) fn switch(key: &'static str, label: &'static str) -> Param {
    row(key, label, ParamKind::Switch, ParamValue::Switch(false))
}

pub(crate) fn shown(mut param: Param, condition: ShownWhen) -> Param {
    param.shown_when = Some(condition);
    param
}

pub(crate) fn tracked(mut param: Param, from: f64, to: f64, curve: Curve) -> Param {
    param.track = Some(Track { from, to, curve });
    param
}
