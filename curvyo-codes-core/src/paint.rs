//! The Background and the Finder colour of a code: parts with their own paint (ADR 0091 "Per-part
//! paint" P3). The parts exist only while their switch is on, so with both off a generation is the
//! one part of slices 1 to 3, byte for byte.

use std::borrow::Cow;

use curvyo_document_core::{Color, PartPaint};
use curvyo_extension_core::{ClosedPath, Param, ParamKind, ParamValue, Part, ShownWhen};

use crate::rows::{row, shown, switch};
use crate::values::Values;

/// The key of the Background part (never reaches a machine).
pub(crate) const BACKGROUND: &str = "background";
/// The key of the Finder part (QR only; ink, it runs with the code).
pub(crate) const FINDER: &str = "finder";

const BACKGROUND_ON: ShownWhen = ShownWhen::On { key: "background" };
const NOT_INVERTED: ShownWhen = ShownWhen::Off { key: "inverted" };
const FINDER_ON: ShownWhen = ShownWhen::All {
    conditions: &[
        ShownWhen::On {
            key: "finder_colour_on",
        },
        NOT_INVERTED,
    ],
};

/// The Colours rows after Inverted: the Finder colour pair (QR only) and the Background pair.
pub(crate) fn rows(finder: bool) -> Vec<Param> {
    let mut rows = vec![switch("inverted", "Inverted")];
    if finder {
        rows.push(shown(
            switch("finder_colour_on", "Finder colour"),
            NOT_INVERTED,
        ));
        rows.push(shown(
            row(
                "finder_colour",
                "Finder colour",
                ParamKind::Colour,
                ParamValue::Colour(Color::BLACK),
            ),
            FINDER_ON,
        ));
    }
    rows.push(switch("background", "Background"));
    rows.push(shown(
        row(
            "background_colour",
            "Background",
            ParamKind::Colour,
            ParamValue::Colour(Color::WHITE),
        ),
        BACKGROUND_ON,
    ));
    rows
}

/// The Background part when its switch is on: `ground` (the symbol and its quiet zone) painted in
/// the chosen colour, never reaching a machine.
pub(crate) fn background(values: &Values<'_>, ground: ClosedPath) -> Option<Part> {
    values.switch("background").then(|| Part {
        key: Cow::Borrowed(BACKGROUND),
        paths: vec![ground],
        paint: Some(PartPaint {
            fill: values.colour("background_colour"),
            job: false,
        }),
    })
}

/// The Finder part when the Finder colour is on and not inverted: `finders` painted in the chosen
/// colour, running with the code.
pub(crate) fn finder(values: &Values<'_>, finders: Vec<ClosedPath>) -> Option<Part> {
    (values.switch("finder_colour_on") && !values.switch("inverted") && !finders.is_empty()).then(
        || Part {
            key: Cow::Borrowed(FINDER),
            paths: finders,
            paint: Some(PartPaint {
                fill: values.colour("finder_colour"),
                job: true,
            }),
        },
    )
}
