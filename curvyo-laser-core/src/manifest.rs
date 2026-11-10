//! The laser workbench's manifest: one Laser tab, a View card and the Engraved grid fixture
//! (`specs/0093-laser-workbench-shell` UX notes §3, §4).

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Block, Contribution, FixtureKind, Manifest, Param, ParamKind, ParamValue, Tab, ToolboxCard,
    ToolboxCommand, WorkbenchContribution,
};

use crate::overhang::OVERHANG_NOTICE;

/// The process id of a laser manufacturing and the id of this extension (ADR 0033 §1).
pub const PROCESS_ID: &str = "curvyo.laser";

/// The kind id of the Engraved grid inside the laser process.
pub const ENGRAVED_GRID: &str = "engraved_grid";

/// The parameters of the Engraved grid, with their defaults and bounds (UX notes §4).
#[must_use]
pub(crate) fn grid_params() -> Vec<Param> {
    let length = |key, label, min: f64, max: f64, default: f64| Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Length {
            min: Length::from_mm(min),
            max: Length::from_mm(max),
        },
        default: ParamValue::Length(Length::from_mm(default)),
    };
    vec![
        length("spacing", "Spacing", 0.5, 1000.0, 10.0),
        length("line_width", "Line width", 0.01, 10.0, 0.1),
        length("origin_x", "Origin X", -10_000.0, 10_000.0, 0.0),
        length("origin_y", "Origin Y", -10_000.0, 10_000.0, 0.0),
    ]
}

/// The laser workbench's manifest.
#[must_use]
pub fn manifest() -> Manifest {
    Manifest {
        id: PROCESS_ID,
        name: "Laser workbench",
        version: env!("CARGO_PKG_VERSION"),
        licence: "AGPL-3.0-or-later",
        contributions: vec![Contribution::Workbench(WorkbenchContribution {
            id: PROCESS_ID,
            label: "Laser",
            icon: "Zap",
            tabs: vec![Tab {
                id: "laser",
                label: "Laser",
                icon: "Zap",
                blocks: vec![
                    Block::Notice {
                        id: OVERHANG_NOTICE,
                    },
                    Block::MachineProfile { label: "Machine" },
                    Block::FixtureList { label: "Fixtures" },
                ],
            }],
            toolbox_cards: vec![ToolboxCard {
                label: "View",
                commands: vec![
                    ToolboxCommand {
                        id: "pan",
                        label: "Pan (H)",
                        icon: "Hand",
                    },
                    ToolboxCommand {
                        id: "zoom",
                        label: "Zoom (Z)",
                        icon: "ZoomIn",
                    },
                    ToolboxCommand {
                        id: "fit_page",
                        label: "Fit page",
                        icon: "Maximize",
                    },
                ],
            }],
            fixture_kinds: vec![FixtureKind {
                id: ENGRAVED_GRID,
                label: "Engraved grid",
                icon: "Grid3x3",
                params: grid_params(),
            }],
        })],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_declares_one_laser_workbench() {
        let manifest = manifest();
        assert_eq!(manifest.id, PROCESS_ID);
        assert_eq!(manifest.name, "Laser workbench");
        assert_eq!(manifest.version, "0.1.0");
        let workbench = manifest.workbench().unwrap();
        assert_eq!(workbench.label, "Laser");
        assert_eq!(workbench.tabs.len(), 1);
        assert_eq!(workbench.fixture_kinds[0].label, "Engraved grid");
    }

    #[test]
    fn the_laser_tab_starts_with_the_notice_then_machine_then_fixtures() {
        let manifest = manifest();
        let blocks = &manifest.workbench().unwrap().tabs[0].blocks;
        assert!(matches!(blocks[0], Block::Notice { .. }));
        assert!(matches!(
            blocks[1],
            Block::MachineProfile { label: "Machine" }
        ));
        assert!(matches!(blocks[2], Block::FixtureList { .. }));
    }
}
