//! The gear's parameter groups (UX notes of `0141` §5; keys are file format, ADR 0034 §5).

use curvyo_extension_core::{ChoiceOption, ParamGroup, ShownWhen};

use crate::schema::{angle, choice, count, length, switch, when};

pub(crate) const WEB: &[ChoiceOption] = &[
    ChoiceOption {
        id: "solid",
        label: "Solid",
    },
    ChoiceOption {
        id: "spokes",
        label: "Spokes",
    },
];

/// The module range in mm.
pub(crate) const MODULE_RANGE: (f64, f64) = (0.1, 25.0);
/// The teeth range.
pub(crate) const TEETH_RANGE: (u32, u32) = (6, 200);

pub(crate) fn groups() -> Vec<ParamGroup> {
    vec![
        ParamGroup {
            label: "Teeth",
            params: vec![
                length("module", "Module", MODULE_RANGE, 2.0, (0.1, 4)),
                count("teeth", "Teeth", TEETH_RANGE.0, TEETH_RANGE.1, 24),
                length(
                    "pitch_diameter",
                    "Pitch diameter",
                    (
                        MODULE_RANGE.0 * f64::from(TEETH_RANGE.0),
                        MODULE_RANGE.1 * f64::from(TEETH_RANGE.1),
                    ),
                    48.0,
                    (1.0, 2),
                ),
                angle("pressure_angle", "Pressure angle", (10.0, 30.0), 20.0),
            ],
        },
        ParamGroup {
            label: "Centre",
            params: vec![
                length("bore", "Bore", (0.0, 1000.0), 6.0, (0.5, 2)),
                when(
                    switch("keyway", "Keyway"),
                    ShownWhen::Positive { key: "bore" },
                ),
                when(
                    length("keyway_width", "Keyway width", (0.5, 100.0), 2.0, (0.1, 2)),
                    ShownWhen::On { key: "keyway" },
                ),
                when(
                    length("keyway_depth", "Keyway depth", (0.2, 50.0), 1.0, (0.1, 2)),
                    ShownWhen::On { key: "keyway" },
                ),
            ],
        },
        ParamGroup {
            label: "Web",
            params: vec![
                choice("web", "Web", WEB, "solid"),
                when(
                    count("spokes", "Spokes", 2, 12, 5),
                    ShownWhen::Is {
                        key: "web",
                        choice: "spokes",
                    },
                ),
                when(
                    length("rim_width", "Rim width", (0.5, 200.0), 4.0, (0.5, 2)),
                    ShownWhen::Is {
                        key: "web",
                        choice: "spokes",
                    },
                ),
                when(
                    length(
                        "hub_diameter",
                        "Hub diameter",
                        (1.0, 1000.0),
                        12.0,
                        (0.5, 2),
                    ),
                    ShownWhen::Is {
                        key: "web",
                        choice: "spokes",
                    },
                ),
            ],
        },
    ]
}
