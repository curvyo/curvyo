//! The generators' manifest: the kinds `gear` and `box` and their parameter groups.

use curvyo_extension_core::{Contribution, GeneratorKind, Manifest, Resize, Source};

use crate::{boxes, gear, path_box, rounded_box};

/// The extension id; generated objects store `<id>/<kind>`.
pub const PROCESS_ID: &str = "curvyo.generators";
/// The kind id of the gear.
pub const GEAR: &str = "gear";
/// The kind id of the box.
pub const BOX: &str = "box";
/// The kind id of the rounded box.
pub const ROUNDED_BOX: &str = "rounded-box";
/// The kind id of the path box.
pub const PATH_BOX: &str = "path-box";

/// The generators' manifest, kinds in the registry order (by the text of the segment: Box, Box from path, Gear, Rounded box; `0065` criterion 3).
#[must_use]
pub fn manifest() -> Manifest {
    Manifest {
        id: PROCESS_ID,
        name: "Generators",
        version: env!("CARGO_PKG_VERSION"),
        licence: "AGPL-3.0-or-later",
        contributions: vec![
            Contribution::Generator(GeneratorKind {
                id: BOX,
                label: "Box",
                icon: "Box",
                groups: boxes::groups(),
                source: None,
                scanned: false,
                family: None,
                resize: Resize::Proportional,
            }),
            Contribution::Generator(GeneratorKind {
                id: PATH_BOX,
                label: "Path box",
                icon: "Spline",
                groups: path_box::groups(),
                source: Some(Source {
                    param: "outline",
                    command: "Box from path",
                }),
                scanned: false,
                family: None,
                resize: Resize::Proportional,
            }),
            Contribution::Generator(GeneratorKind {
                id: GEAR,
                label: "Gear",
                icon: "Cog",
                groups: gear::groups(),
                source: None,
                scanned: false,
                family: None,
                resize: Resize::Proportional,
            }),
            Contribution::Generator(GeneratorKind {
                id: ROUNDED_BOX,
                label: "Rounded box",
                icon: "SquareRoundCorner",
                groups: rounded_box::groups(),
                source: None,
                scanned: false,
                family: None,
                resize: Resize::Proportional,
            }),
        ],
    }
}
