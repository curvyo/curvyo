//! The box's parameter groups (UX notes of `0141` §5).

use curvyo_extension_core::{ChoiceOption, ParamGroup};

use crate::schema::{CENTRED, advanced, choice, length, tracked};
use curvyo_extension_core::Curve::{Linear, Log};

pub(crate) const LID: &[ChoiceOption] = &[
    ChoiceOption {
        id: "none",
        label: "None",
    },
    ChoiceOption {
        id: "flat",
        label: "Flat",
    },
    ChoiceOption {
        id: "fingers",
        label: "Fingers",
    },
];

pub(crate) const INNER_RANGE: (f64, f64) = (10.0, 2000.0);

pub(crate) fn groups() -> Vec<ParamGroup> {
    vec![
        ParamGroup {
            label: "Size",
            params: vec![
                tracked(
                    length("inner_width", "Inner width", INNER_RANGE, 100.0, (1.0, 2)),
                    10.0,
                    500.0,
                    Log,
                ),
                tracked(
                    length("inner_depth", "Inner depth", INNER_RANGE, 80.0, (1.0, 2)),
                    10.0,
                    500.0,
                    Log,
                ),
                tracked(
                    length("inner_height", "Inner height", INNER_RANGE, 50.0, (1.0, 2)),
                    10.0,
                    500.0,
                    Log,
                ),
            ],
        },
        ParamGroup {
            label: "Material",
            params: vec![
                tracked(
                    length("thickness", "Thickness", (0.1, 50.0), 3.0, (0.1, 2)),
                    0.1,
                    12.0,
                    Linear,
                ),
                tracked(
                    length("kerf", "Kerf compensation", (0.0, 2.0), 0.0, (0.01, 3)),
                    0.0,
                    0.5,
                    Linear,
                ),
            ],
        },
        ParamGroup {
            label: "Joints",
            params: vec![tracked(
                length("clearance", "Clearance", (0.0, 2.0), 0.0, (0.01, 3)),
                0.0,
                0.5,
                Linear,
            )],
        },
        ParamGroup {
            label: "Lid",
            params: vec![choice("lid", "Lid", LID, "none")],
        },
        ParamGroup {
            label: "Layout",
            params: vec![tracked(
                length("gap", "Gap", (0.0, 100.0), 5.0, (1.0, 2)),
                0.0,
                50.0,
                Linear,
            )],
        },
        joint_layout(),
    ]
}

/// The Advanced group of the tabs' layout with Finger width as its Joint size (spec `0155`
/// criterion 25): the key stays `finger_width`, shown once under its new name.
fn joint_layout() -> ParamGroup {
    let mut group = advanced(None, CENTRED);
    for param in &mut group.params {
        if param.key == "joint_width" {
            *param = tracked(
                length("finger_width", "Joint size", (0.5, 200.0), 6.0, (0.5, 2)),
                0.5,
                50.0,
                Log,
            );
        }
    }
    group
}
