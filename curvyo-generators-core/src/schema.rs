//! Small constructors for the parameter rows of the two generators.

use curvyo_document_core::Length;
use curvyo_extension_core::{
    ChoiceOption, Curve, Degrees, Param, ParamGroup, ParamKind, ParamValue, ShownWhen, Track,
};

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
            automatic: false,
        },
        default: ParamValue::Length(Length::from_mm(default)),
        shown_when: None,
        track: None,
        initial: None,
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
        kind: ParamKind::Count {
            min,
            max,
            automatic: false,
        },
        default: ParamValue::Count(default),
        shown_when: None,
        track: None,
        initial: None,
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
        track: None,
        initial: None,
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
        track: None,
        initial: None,
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
        track: None,
        initial: None,
    }
}

/// `param` drawn only while `condition` holds.
pub(crate) fn when(mut param: Param, condition: ShownWhen) -> Param {
    param.shown_when = Some(condition);
    param
}

/// `param` drawn on a slider whose track runs from `from` to `to` (a drag past the ends goes on up
/// to the parameter's own bounds).
pub(crate) fn tracked(mut param: Param, from: f64, to: f64, curve: Curve) -> Param {
    param.track = Some(Track { from, to, curve });
    param
}

/// `param` stores 0 for automatic: the generator derives the value and reports it.
pub(crate) fn automatic(mut param: Param) -> Param {
    match &mut param.kind {
        ParamKind::Length { automatic, .. } | ParamKind::Count { automatic, .. } => {
            *automatic = true;
        }
        _ => {}
    }
    param
}

/// A switch row, on by default.
pub(crate) fn switch_on(key: &'static str, label: &'static str) -> Param {
    let mut param = switch(key, label);
    param.default = ParamValue::Switch(true);
    param
}

/// `param` is written as the choice `id` in a new object while a stored one without the key still
/// reads its default (ADR 0089 decision 2).
pub(crate) fn initial(mut param: Param, id: &'static str) -> Param {
    param.initial = Some(ParamValue::Choice(id.to_owned()));
    param
}

/// How the tabs of an edge are spread (spec `0155` criterion 19).
pub(crate) const JOINT_PATTERN: &[ChoiceOption] = &[
    ChoiceOption {
        id: "even",
        label: "Even",
    },
    ChoiceOption {
        id: "centred",
        label: "Centred",
    },
];

/// The condition of the gap and the spread: the pattern is Centred.
pub(crate) const CENTRED: ShownWhen = ShownWhen::Is {
    key: "joint_pattern",
    choice: "centred",
};

/// The Advanced group: the joint layout of the tabs (spec `0155` criteria 18 to 20). `shown` is
/// the condition every row hangs on and `centred` the one of the gap and the spread (the rounded
/// box shows the group with the flat top only).
pub(crate) fn advanced(shown: Option<ShownWhen>, centred: ShownWhen) -> ParamGroup {
    let on = |param: Param, for_centred: bool| {
        if for_centred {
            when(param, centred)
        } else {
            shown.map_or(param.clone(), |only| when(param, only))
        }
    };
    ParamGroup {
        label: "Advanced",
        params: vec![
            on(
                choice("joint_pattern", "Joint pattern", JOINT_PATTERN, "even"),
                false,
            ),
            on(
                tracked(
                    automatic(count("joint_count", "Joint count", 1, 200, 0)),
                    1.0,
                    20.0,
                    Curve::Linear,
                ),
                false,
            ),
            on(
                tracked(
                    automatic(length(
                        "joint_width",
                        "Joint size",
                        (1.0, 200.0),
                        0.0,
                        (0.5, 2),
                    )),
                    1.0,
                    30.0,
                    Curve::Linear,
                ),
                false,
            ),
            on(
                tracked(
                    automatic(length(
                        "joint_gap",
                        "Joint gap",
                        (1.0, 400.0),
                        0.0,
                        (0.5, 2),
                    )),
                    1.0,
                    60.0,
                    Curve::Linear,
                ),
                true,
            ),
            on(
                tracked(
                    count("joint_spread", "Joint spread", 0, 100, 0),
                    0.0,
                    100.0,
                    Curve::Linear,
                ),
                true,
            ),
        ],
    }
}
