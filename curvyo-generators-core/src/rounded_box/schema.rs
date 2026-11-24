//! The rounded box's parameter groups (UX notes of `0147` §3, §4; keys are file format).

use curvyo_extension_core::Curve::{Linear, Log};
use curvyo_extension_core::{ChoiceOption, ParamGroup, ShownWhen};

use crate::schema::{automatic, choice, count, length, switch_on, tracked, when};

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

const INNER: (f64, f64) = (10.0, 2000.0);
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
        ],
    }
}

fn material() -> ParamGroup {
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

fn slits() -> ParamGroup {
    ParamGroup {
        label: "Slits",
        params: vec![
            choice("slit_pattern", "Slit pattern", PATTERN, "straight"),
            choice("wall_slits", "Wall slits", WALL_SLITS, "corners"),
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
    }
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

fn base() -> ParamGroup {
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

fn lid() -> ParamGroup {
    let on = |key| ShownWhen::On { key };
    let seated = ShownWhen::AllOn {
        keys: &["lid", "lid_seat"],
    };
    ParamGroup {
        label: "Lid",
        params: vec![
            switch_on("lid", "Lid"),
            when(following("lid_thickness", "Lid thickness"), on("lid")),
            when(switch_on("lid_seat", "Lid seat"), on("lid")),
            when(following("ring_thickness", "Ring thickness"), seated),
            when(switch_on("finger_hollow", "Finger hollow"), seated),
            when(
                tracked(
                    length("hollow_width", "Hollow width", (5.0, 200.0), 16.0, (1.0, 2)),
                    5.0,
                    60.0,
                    Linear,
                ),
                ShownWhen::AllOn {
                    keys: &["lid", "lid_seat", "finger_hollow"],
                },
            ),
            when(
                tracked(
                    length("hollow_depth", "Hollow depth", (2.0, 100.0), 8.0, (0.5, 2)),
                    2.0,
                    30.0,
                    Linear,
                ),
                ShownWhen::AllOn {
                    keys: &["lid", "lid_seat", "finger_hollow"],
                },
            ),
        ],
    }
}

fn layout() -> ParamGroup {
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
    vec![size(), material(), slits(), base(), lid(), layout()]
}
