//! The rounded box's parameter groups (UX notes of `0147` §3, §4; keys are file format).

use curvyo_extension_core::Curve::{Linear, Log};
use curvyo_extension_core::{ChoiceOption, ParamGroup, ShownWhen};

use crate::schema::{
    CENTRED as CENTRED_PATTERN, advanced, automatic, choice, count, initial, length, switch_on,
    tracked, when,
};

pub(crate) const PATTERN: &[ChoiceOption] = &[
    ChoiceOption {
        id: "straight",
        label: "Straight",
    },
    ChoiceOption {
        id: "offset",
        label: "Offset",
    },
    ChoiceOption {
        id: "wave",
        label: "Wave",
    },
];

pub(crate) const WALL_SLITS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "corners",
        label: "Corners only",
    },
    ChoiceOption {
        id: "all",
        label: "All round",
    },
];

/// How a tab on a bend meets the wall's slits (ADR 0089 decision 2).
pub(crate) const BEND_TABS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "window",
        label: "Flat window",
    },
    ChoiceOption {
        id: "curve",
        label: "Curve",
    },
];

pub(crate) const TOP_SHAPE: &[ChoiceOption] = &[
    ChoiceOption {
        id: "flat",
        label: "Flat",
    },
    ChoiceOption {
        id: "curved",
        label: "Curved",
    },
];

pub(crate) const INNER: (f64, f64) = (10.0, 2000.0);
const THICKNESS: (f64, f64) = (0.1, 50.0);

fn size() -> ParamGroup {
    ParamGroup {
        label: "Size",
        params: vec![
            tracked(
                length("inner_width", "Inner width", INNER, 120.0, (1.0, 2)),
                10.0,
                500.0,
                Log,
            ),
            tracked(
                length("inner_depth", "Inner depth", INNER, 80.0, (1.0, 2)),
                10.0,
                500.0,
                Log,
            ),
            tracked(
                length("inner_height", "Inner height", INNER, 50.0, (1.0, 2)),
                10.0,
                500.0,
                Log,
            ),
            tracked(
                length(
                    "corner_radius",
                    "Corner radius",
                    (1.0, 1000.0),
                    15.0,
                    (0.5, 2),
                ),
                1.0,
                100.0,
                Log,
            ),
            choice("top_shape", "Top shape", TOP_SHAPE, "flat"),
        ],
    }
}

pub(crate) fn material() -> ParamGroup {
    ParamGroup {
        label: "Material",
        params: vec![
            tracked(
                length("thickness", "Wall thickness", THICKNESS, 3.0, (0.1, 2)),
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
    }
}

pub(crate) fn slits(wall_slits: bool) -> ParamGroup {
    let mut group = ParamGroup {
        label: "Slits",
        params: vec![
            choice("slit_pattern", "Slit pattern", PATTERN, "straight"),
            choice("wall_slits", "Wall slits", WALL_SLITS, "corners"),
            initial(
                choice("bend_tabs", "Bend through tabs", BEND_TABS, "window"),
                "curve",
            ),
            tracked(
                automatic(length(
                    "slit_width",
                    "Slit width",
                    (0.2, 5.0),
                    0.0,
                    (0.05, 3),
                )),
                0.2,
                2.0,
                Linear,
            ),
            tracked(
                automatic(length(
                    "slit_spacing",
                    "Slit spacing",
                    (0.5, 50.0),
                    0.0,
                    (0.1, 2),
                )),
                0.5,
                20.0,
                Log,
            ),
            when(
                tracked(
                    automatic(count("slit_rows", "Slit rows", 1, 20, 0)),
                    1.0,
                    10.0,
                    Linear,
                ),
                ShownWhen::Is {
                    key: "slit_pattern",
                    choice: "offset",
                },
            ),
        ],
    };
    if !wall_slits {
        group.params.retain(|param| param.key != "wall_slits");
    }
    group
}

/// A thickness that follows the wall thickness while stored 0.
fn following(key: &'static str, label: &'static str) -> curvyo_extension_core::Param {
    tracked(
        automatic(length(key, label, THICKNESS, 0.0, (0.1, 2))),
        0.1,
        12.0,
        Linear,
    )
}

pub(crate) fn base() -> ParamGroup {
    ParamGroup {
        label: "Base",
        params: vec![
            following("plate_thickness", "Inner base thickness"),
            tracked(
                length("base_inset", "Base inset", (0.0, 50.0), 0.0, (0.5, 2)),
                0.0,
                50.0,
                Linear,
            ),
            switch_on("base_cover", "Base cover"),
            when(
                following("cover_thickness", "Cover thickness"),
                ShownWhen::On { key: "base_cover" },
            ),
        ],
    }
}

/// The Lid rows' conditions, which the curved top's rows build on.
const LID_ON: ShownWhen = ShownWhen::On { key: "lid" };
const SEATED: ShownWhen = ShownWhen::AllOn {
    keys: &["lid", "lid_seat"],
};
const HOLLOW_ON: ShownWhen = ShownWhen::AllOn {
    keys: &["lid", "lid_seat", "finger_hollow"],
};

pub(crate) fn lid() -> ParamGroup {
    ParamGroup {
        label: "Lid",
        params: vec![
            switch_on("lid", "Lid"),
            when(following("lid_thickness", "Lid thickness"), LID_ON),
            when(switch_on("lid_seat", "Lid seat"), LID_ON),
            when(following("ring_thickness", "Ring thickness"), SEATED),
            when(switch_on("finger_hollow", "Finger hollow"), SEATED),
            when(
                tracked(
                    length("hollow_width", "Hollow width", (5.0, 200.0), 16.0, (1.0, 2)),
                    5.0,
                    60.0,
                    Linear,
                ),
                HOLLOW_ON,
            ),
            when(
                tracked(
                    length("hollow_depth", "Hollow depth", (2.0, 100.0), 8.0, (0.5, 2)),
                    2.0,
                    30.0,
                    Linear,
                ),
                HOLLOW_ON,
            ),
        ],
    }
}

/// The rounded box's Material: Wall thickness, then the two plates only the curved top has.
fn rounded_material() -> ParamGroup {
    let mut group = material();
    let mut extra = vec![
        when(
            following("end_plate_thickness", "End plate thickness"),
            CURVED,
        ),
        when(following("front_thickness", "Front thickness"), CURVED),
    ];
    group.params.splice(1..1, extra.drain(..));
    group
}

const FLAT: ShownWhen = ShownWhen::Is {
    key: "top_shape",
    choice: "flat",
};
const CURVED: ShownWhen = ShownWhen::Is {
    key: "top_shape",
    choice: "curved",
};
const FLAT_COVER: ShownWhen = ShownWhen::All {
    conditions: &[ShownWhen::On { key: "base_cover" }, FLAT],
};
const FLAT_LID: ShownWhen = ShownWhen::All {
    conditions: &[LID_ON, FLAT],
};
const FLAT_RING: ShownWhen = ShownWhen::All {
    conditions: &[SEATED, FLAT],
};
const HOLLOW_SWITCH: ShownWhen = ShownWhen::Any {
    conditions: &[SEATED, CURVED],
};
const HOLLOW_SIZE: ShownWhen = ShownWhen::Any {
    conditions: &[
        HOLLOW_ON,
        ShownWhen::All {
            conditions: &[
                CURVED,
                ShownWhen::On {
                    key: "finger_hollow",
                },
            ],
        },
    ],
};

/// `group` with the conditions of the curved top on the rows `rules` name (ADR 0082 §11): only the
/// rounded box's groups carry them, so the path box's manifest does not change.
fn conditioned(mut group: ParamGroup, rules: &[(&str, ShownWhen)]) -> ParamGroup {
    for param in &mut group.params {
        if let Some((_, condition)) = rules.iter().find(|(key, _)| *key == param.key) {
            param.shown_when = Some(*condition);
        }
    }
    group
}

pub(crate) fn layout() -> ParamGroup {
    ParamGroup {
        label: "Layout",
        params: vec![tracked(
            length("gap", "Gap", (0.0, 100.0), 5.0, (1.0, 2)),
            0.0,
            50.0,
            Linear,
        )],
    }
}

pub(crate) fn groups() -> Vec<ParamGroup> {
    vec![
        size(),
        rounded_material(),
        conditioned(slits(true), &[("wall_slits", FLAT), ("bend_tabs", FLAT)]),
        conditioned(
            base(),
            &[
                ("base_inset", FLAT),
                ("base_cover", FLAT),
                ("cover_thickness", FLAT_COVER),
            ],
        ),
        conditioned(
            lid(),
            &[
                ("lid", FLAT),
                ("lid_thickness", FLAT_LID),
                ("lid_seat", FLAT_LID),
                ("ring_thickness", FLAT_RING),
                ("finger_hollow", HOLLOW_SWITCH),
                ("hollow_width", HOLLOW_SIZE),
                ("hollow_depth", HOLLOW_SIZE),
            ],
        ),
        layout(),
        advanced(None, CENTRED_PATTERN),
    ]
}
