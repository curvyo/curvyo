//! The QR code's parameter groups (UX notes of `0144` §3; the keys are file format, ADR 0034 §5).

use curvyo_document_core::Length;
use curvyo_extension_core::{
    ChoiceOption, Curve, Param, ParamGroup, ParamKind, ParamValue, ShownWhen, Track,
};

/// The module size range in mm.
pub(crate) const MODULE_RANGE: (f64, f64) = (0.1, 20.0);
/// The quiet zone range in modules.
pub(crate) const QUIET_RANGE: (u32, u32) = (0, 10);
/// The default content (a visible, harmless placeholder).
pub(crate) const DEFAULT_CONTENT: &str = "https://example.com";

const SHAPES: &[ChoiceOption] = &[
    ChoiceOption {
        id: "square",
        label: "Square",
    },
    ChoiceOption {
        id: "rounded",
        label: "Rounded square",
    },
    ChoiceOption {
        id: "dot",
        label: "Dot",
    },
    ChoiceOption {
        id: "diamond",
        label: "Diamond",
    },
];
const FINDERS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "square",
        label: "Square",
    },
    ChoiceOption {
        id: "rounded",
        label: "Rounded",
    },
    ChoiceOption {
        id: "circle",
        label: "Circle",
    },
];
/// The outer ring has no Circle in slice 1: the independent decoder cannot read it (spec 0144 criterion 6).
const FINDERS_OUTER: &[ChoiceOption] = &[
    ChoiceOption {
        id: "square",
        label: "Square",
    },
    ChoiceOption {
        id: "rounded",
        label: "Rounded",
    },
];
const LEVELS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "l",
        label: "L 7 %",
    },
    ChoiceOption {
        id: "m",
        label: "M 15 %",
    },
    ChoiceOption {
        id: "q",
        label: "Q 25 %",
    },
    ChoiceOption {
        id: "h",
        label: "H 30 %",
    },
];
const OUTPUTS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "merged",
        label: "Merged",
    },
    ChoiceOption {
        id: "separate",
        label: "Separate",
    },
];

fn row(key: &'static str, label: &'static str, kind: ParamKind, default: ParamValue) -> Param {
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

fn length(
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

fn percent(key: &'static str, label: &'static str, (min, max): (u32, u32), default: u32) -> Param {
    row(
        key,
        label,
        ParamKind::Percent { min, max },
        ParamValue::Percent(default),
    )
}

fn choice(
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

fn switch(key: &'static str, label: &'static str) -> Param {
    row(key, label, ParamKind::Switch, ParamValue::Switch(false))
}

fn shown(mut param: Param, condition: ShownWhen) -> Param {
    param.shown_when = Some(condition);
    param
}

fn tracked(mut param: Param, from: f64, to: f64, curve: Curve) -> Param {
    param.track = Some(Track { from, to, curve });
    param
}

const LOGO_ON: ShownWhen = ShownWhen::On { key: "logo" };

pub(crate) fn groups() -> Vec<ParamGroup> {
    vec![
        content(),
        size(),
        modules(),
        finder(),
        ParamGroup {
            label: "Error correction",
            params: vec![choice("ec", "Error correction", LEVELS, "m")],
        },
        logo(),
        ParamGroup {
            label: "Colours",
            params: vec![switch("inverted", "Inverted")],
        },
        advanced(),
    ]
}

fn content() -> ParamGroup {
    ParamGroup {
        label: "Content",
        params: vec![row(
            "content",
            "Content",
            ParamKind::Text { multiline: true },
            ParamValue::Text(DEFAULT_CONTENT.to_owned()),
        )],
    }
}

fn size() -> ParamGroup {
    ParamGroup {
        label: "Size",
        params: vec![
            tracked(
                length("module_size", "Module size", MODULE_RANGE, 1.0, 0.05),
                0.2,
                5.0,
                Curve::Log,
            ),
            // Derived: the outer width including the quiet zone (the Pitch diameter rule).
            tracked(
                length("width", "Width", (2.1, 4000.0), 29.0, 1.0),
                5.0,
                300.0,
                Curve::Log,
            ),
        ],
    }
}

fn modules() -> ParamGroup {
    ParamGroup {
        label: "Modules",
        params: vec![
            choice("module_shape", "Module shape", SHAPES, "square"),
            shown(
                percent("rounding", "Rounding", (0, 50), 30),
                ShownWhen::Is {
                    key: "module_shape",
                    choice: "rounded",
                },
            ),
            percent("module_fill", "Module fill", (50, 100), 100),
        ],
    }
}

fn finder() -> ParamGroup {
    ParamGroup {
        label: "Finder",
        params: vec![
            choice("finder_outer", "Outer", FINDERS_OUTER, "square"),
            choice("finder_inner", "Inner", FINDERS, "square"),
        ],
    }
}

fn logo() -> ParamGroup {
    ParamGroup {
        label: "Logo",
        params: vec![
            switch("logo", "Clear area"),
            shown(choice("logo_shape", "Shape", FINDERS, "circle"), LOGO_ON),
            shown(percent("logo_size", "Size", (5, 35), 20), LOGO_ON),
            shown(
                row(
                    "logo_margin",
                    "Margin",
                    ParamKind::Count {
                        min: 0,
                        max: 2,
                        automatic: false,
                    },
                    ParamValue::Count(1),
                ),
                LOGO_ON,
            ),
        ],
    }
}

fn advanced() -> ParamGroup {
    ParamGroup {
        label: "Advanced",
        params: vec![
            row(
                "quiet_zone",
                "Quiet zone",
                ParamKind::Count {
                    min: QUIET_RANGE.0,
                    max: QUIET_RANGE.1,
                    automatic: false,
                },
                ParamValue::Count(4),
            ),
            shown(
                choice("output", "Output", OUTPUTS, "merged"),
                ShownWhen::All {
                    conditions: &[
                        ShownWhen::Any {
                            conditions: &[
                                ShownWhen::Is {
                                    key: "module_shape",
                                    choice: "square",
                                },
                                ShownWhen::Is {
                                    key: "module_shape",
                                    choice: "rounded",
                                },
                            ],
                        },
                        ShownWhen::Full { key: "module_fill" },
                    ],
                },
            ),
        ],
    }
}
