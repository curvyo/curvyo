//! The generators' manifest: the kinds `gear` and `box` and their parameter groups.

use curvyo_extension_core::{Contribution, GeneratorKind, Manifest};

use crate::{boxes, gear, rounded_box};

/// The extension id; generated objects store `<id>/<kind>`.
pub const PROCESS_ID: &str = "curvyo.generators";
/// The kind id of the gear.
pub const GEAR: &str = "gear";
/// The kind id of the box.
pub const BOX: &str = "box";
/// The kind id of the rounded box.
pub const ROUNDED_BOX: &str = "rounded-box";

/// The generators' manifest, kinds in the registry order (by name: Box, Gear, Rounded box; `0065` criterion 3).
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
            }),
            Contribution::Generator(GeneratorKind {
                id: GEAR,
                label: "Gear",
                icon: "Cog",
                groups: gear::groups(),
            }),
            Contribution::Generator(GeneratorKind {
                id: ROUNDED_BOX,
                label: "Rounded box",
                icon: "SquareRoundCorner",
                groups: rounded_box::groups(),
            }),
        ],
    }
}
