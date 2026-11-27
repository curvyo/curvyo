//! The embroidery workbench's manifest: one Embroidery tab and a View card (ADR 0097 §1).

use curvyo_extension_core::{
    Contribution, Manifest, Tab, ToolboxCard, ToolboxCommand, WorkbenchContribution,
};

/// The process id of a embroidery manufacturing and the id of this extension (ADR 0097 §1).
pub const PROCESS_ID: &str = "curvyo.embroidery";

/// The page word: Add page offers it, and a new page is named "Embroidery N".
pub const WORD: &str = "Embroidery";

/// The commit label of Add page with this process ("Add embroidery page" in the history).
pub const ADD_PAGE_COMMIT_LABEL: &str = "add_embroidery_page";

/// The embroidery workbench's manifest.
#[must_use]
pub fn manifest() -> Manifest {
    Manifest {
        id: PROCESS_ID,
        name: "Embroidery workbench",
        version: env!("CARGO_PKG_VERSION"),
        licence: "AGPL-3.0-or-later",
        contributions: vec![Contribution::Workbench(WorkbenchContribution {
            id: PROCESS_ID,
            label: WORD,
            icon: "Spool",
            tabs: vec![Tab {
                id: "embroidery",
                label: WORD,
                icon: "Spool",
                blocks: Vec::new(),
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
            fixture_kinds: Vec::new(),
        })],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_declares_one_embroidery_workbench_named_by_its_word() {
        let manifest = manifest();
        assert_eq!(manifest.id, "curvyo.embroidery");
        let workbench = manifest.workbench().unwrap();
        assert_eq!(workbench.id, PROCESS_ID);
        assert_eq!(workbench.label, "Embroidery");
        assert_eq!(workbench.tabs.len(), 1);
        assert!(
            workbench.tabs[0].blocks.is_empty(),
            "no laser Machine block"
        );
    }
}
