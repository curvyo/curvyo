//! The parameter groups of the 1D kinds (UX notes of `0144` §11.5; the keys are file format, ADR
//! 0092 §2): shared by every 1D kind, with a few rows that only some kinds have.

use curvyo_extension_core::{Curve, Param, ParamGroup, ParamKind, ParamValue, ShownWhen};

use super::Symbology;
use crate::rows::{length, percent, row, shown, switch, tracked};

/// The module width range in mm (as the QR module size).
pub(crate) const MODULE_RANGE: (f64, f64) = (0.1, 20.0);
/// The bar height range in mm.
pub(crate) const HEIGHT_RANGE: (f64, f64) = (1.0, 500.0);
/// The text size range in mm.
pub(crate) const TEXT_RANGE: (f64, f64) = (0.5, 50.0);
/// The default module width in mm (the X dimension of EAN at 100 % is 0.33).
pub(crate) const DEFAULT_MODULE: f64 = 0.33;
/// The default bar height in mm.
pub(crate) const DEFAULT_HEIGHT: f64 = 15.0;
/// The default text size in mm.
pub(crate) const DEFAULT_TEXT: f64 = 3.0;

const TEXT_ON: ShownWhen = ShownWhen::On { key: "text" };

fn decimals(mut param: Param, decimals: u8) -> Param {
    if let ParamKind::Length { decimals: d, .. } = &mut param.kind {
        *d = decimals;
    }
    param
}

/// The width the defaults give: the outer width with the quiet zones at the default module width.
fn default_width(symbology: Symbology) -> f64 {
    let code = super::encode(symbology, symbology.default_content(), 3.0);
    code.map_or(0.0, |code| {
        let (left, right) = code.fixed_quiet.unwrap_or((10.0, 10.0));
        (left + code.width + right) * DEFAULT_MODULE
    })
}

fn size_group(symbology: Symbology) -> ParamGroup {
    let mut size = vec![
        tracked(
            decimals(
                length(
                    "module_width",
                    "Module width",
                    MODULE_RANGE,
                    DEFAULT_MODULE,
                    0.01,
                ),
                3,
            ),
            0.2,
            2.0,
            Curve::Log,
        ),
        tracked(
            length(
                "bar_height",
                "Bar height",
                HEIGHT_RANGE,
                DEFAULT_HEIGHT,
                1.0,
            ),
            5.0,
            100.0,
            Curve::Log,
        ),
        // Derived: the outer width with both quiet zones (the Pitch diameter rule).
        tracked(
            length(
                "width",
                "Width",
                (1.0, 4000.0),
                default_width(symbology),
                1.0,
            ),
            10.0,
            300.0,
            Curve::Log,
        ),
    ];
    if symbology == Symbology::Code39 {
        size.push(row(
            "wide_narrow",
            "Wide:narrow",
            ParamKind::Ratio {
                min: 2.0,
                max: 3.0,
                step: 0.1,
                decimals: 1,
            },
            ParamValue::Ratio(3.0),
        ));
    }
    ParamGroup {
        label: "Size",
        params: size,
    }
}

fn text_group(symbology: Symbology) -> ParamGroup {
    let mut text_switch = switch("text", "Human-readable text");
    text_switch.default = ParamValue::Switch(symbology.text_default());
    ParamGroup {
        label: "Text",
        params: vec![
            text_switch,
            shown(
                row("font", "Font", ParamKind::Font, ParamValue::Font(None)),
                TEXT_ON,
            ),
            shown(
                tracked(
                    length("text_size", "Size", TEXT_RANGE, DEFAULT_TEXT, 0.1),
                    1.0,
                    10.0,
                    Curve::Log,
                ),
                TEXT_ON,
            ),
        ],
    }
}

fn advanced_group(symbology: Symbology) -> ParamGroup {
    let mut advanced = Vec::new();
    if !symbology.fixed_quiet() {
        advanced.push(row(
            "bar_quiet_zone",
            "Quiet zone",
            ParamKind::Count {
                min: 0,
                max: 20,
                automatic: false,
            },
            ParamValue::Count(10),
        ));
    }
    advanced.push(percent("bar_adjust", "Bar width adjust", (-30, 30), 0));
    ParamGroup {
        label: "Advanced",
        params: advanced,
    }
}

pub(crate) fn groups(symbology: Symbology) -> Vec<ParamGroup> {
    vec![
        ParamGroup {
            label: "Content",
            params: vec![row(
                "content",
                "Content",
                ParamKind::Text { multiline: false },
                ParamValue::Text(symbology.default_content().to_owned()),
            )],
        },
        size_group(symbology),
        text_group(symbology),
        ParamGroup {
            label: "Colours",
            params: vec![switch("inverted", "Inverted")],
        },
        advanced_group(symbology),
    ]
}
