//! The laser workbench's manifest: one Laser tab, a View card and the Engraved grid fixture
//! (`specs/0093-laser-workbench-shell` UX notes §3, §4).

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Block, Contribution, FixtureKind, HostKey, Manifest, Param, ParamKind, ParamValue, Tab,
    ToolboxCard, ToolboxCommand, WorkbenchContribution,
};

use crate::engraved_grid::{ENGRAVED_GRID, grid_params};
use crate::overhang::OVERHANG_NOTICE;

/// The process id of a laser manufacturing and the id of this extension (ADR 0033 §1).
pub const PROCESS_ID: &str = "curvyo.laser";

/// The work area bounds of the Width and Height fields (UX notes §4).
const MIN_WORK_MM: f64 = 1.0;
const MAX_WORK_MM: f64 = 10_000.0;

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
            machine_name: "Generic laser",
            work_width_mm: 400.0,
            work_height_mm: 300.0,
            tabs: vec![Tab {
                id: "laser",
                label: "Laser",
                icon: "Zap",
                blocks: vec![
                    Block::Notice {
                        id: OVERHANG_NOTICE,
                    },
                    Block::Params {
                        label: "Machine",
                        params: machine_params(),
                    },
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

fn machine_params() -> Vec<Param> {
    let work = |key: HostKey, label, default| Param {
        key: key.key(),
        label,
        tooltip: label,
        kind: ParamKind::Length {
            min: Length::from_mm(MIN_WORK_MM),
            max: Length::from_mm(MAX_WORK_MM),
        },
        default: ParamValue::Length(Length::from_mm(default)),
    };
    vec![
        Param {
            key: HostKey::MachineName.key(),
            label: "Machine",
            tooltip: "Machine",
            kind: ParamKind::Display,
            default: ParamValue::Text("Generic laser".to_owned()),
        },
        work(HostKey::WorkWidth, "Width", 400.0),
        work(HostKey::WorkHeight, "Height", 300.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_declares_one_laser_workbench_with_its_placeholder_machine() {
        let manifest = manifest();
        assert_eq!(manifest.id, PROCESS_ID);
        assert_eq!(manifest.name, "Laser workbench");
        assert_eq!(manifest.version, "0.1.0");
        let workbench = manifest.workbench().unwrap();
        assert_eq!(workbench.label, "Laser");
        assert_eq!(workbench.machine_name, "Generic laser");
        assert_eq!(
            (workbench.work_width_mm, workbench.work_height_mm),
            (400.0, 300.0)
        );
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
            Block::Params {
                label: "Machine",
                ..
            }
        ));
        assert!(matches!(blocks[2], Block::FixtureList { .. }));
    }
}
