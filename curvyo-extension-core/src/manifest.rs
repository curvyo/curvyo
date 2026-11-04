//! The manifest an extension declares and the front end renders (ADR 0034 §4).

use serde::Serialize;

use crate::param::Param;

/// What one compiled-in extension declares: identity, licence and contributions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Manifest {
    /// Dotted id (`curvyo.laser`; third parties use `<author>.<name>`). It is the process id of
    /// the manufacturings its workbench makes (ADR 0033 §1).
    pub id: &'static str,
    /// The name in the extension list ("Laser workbench").
    pub name: &'static str,
    /// The crate version.
    pub version: &'static str,
    /// The SPDX licence id.
    pub licence: &'static str,
    /// What the extension adds to the shell.
    pub contributions: Vec<Contribution>,
}

impl Manifest {
    /// The workbench this extension contributes, if any.
    #[must_use]
    pub fn workbench(&self) -> Option<&WorkbenchContribution> {
        self.contributions
            .iter()
            .map(|Contribution::Workbench(workbench)| workbench)
            .next()
    }
}

/// One thing an extension adds to the shell. Only the kinds a story uses exist (ADR 0034 §4).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Contribution {
    /// A manufacturing workbench: its tabs, toolbox cards and fixture kinds.
    Workbench(WorkbenchContribution),
}

/// A manufacturing workbench the shell draws from data (`ManifestWorkbench`, ADR 0032 §3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchContribution {
    /// The workbench id; equal to the manifest id, so the process id names both.
    pub id: &'static str,
    /// The process name ("Laser"): the Add manufacturing choice and the first part of a new
    /// manufacturing's name ("Laser 1").
    pub label: &'static str,
    /// The Lucide glyph name of the switcher entry ("Zap").
    pub icon: &'static str,
    /// The Properties tabs, in order.
    pub tabs: Vec<Tab>,
    /// The toolbox cards, in order.
    pub toolbox_cards: Vec<ToolboxCard>,
    /// The fixture kinds the machine setup can hold.
    pub fixture_kinds: Vec<FixtureKind>,
}

impl WorkbenchContribution {
    /// The fixture kind `id` (the part after `<process>/`).
    #[must_use]
    pub fn fixture_kind(&self, id: &str) -> Option<&FixtureKind> {
        self.fixture_kinds.iter().find(|kind| kind.id == id)
    }
}

/// One Properties tab: a list of blocks drawn top to bottom.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tab {
    /// Stable id.
    pub id: &'static str,
    /// The tab name ("Laser").
    pub label: &'static str,
    /// The Lucide glyph name of the strip entry.
    pub icon: &'static str,
    /// The blocks, in order.
    pub blocks: Vec<Block>,
}

/// A block of a schema-rendered tab: the shell knows exactly these four kinds.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// The machine profile picker and the copy's fields, drawn by the shell from the session's
    /// manufacturing view (`specs/0026-machine-profile` `adrs.md` note 4); the extension only says
    /// where it sits.
    MachineProfile {
        /// The group heading ("Machine").
        label: &'static str,
    },
    /// One row per drawing layer with its role in this manufacturing, the role view switch, its
    /// legend and notice, drawn by the shell from the session's manufacturing view
    /// (`specs/0027-manufacturing-roles` `adrs.md` note 1); the extension only says where it sits.
    Layers {
        /// The group heading ("Layers").
        label: &'static str,
    },
    /// The installed fixtures with add, hide and remove.
    FixtureList {
        /// The group heading ("Fixtures").
        label: &'static str,
    },
    /// A short text the extension computes; shown only while it applies.
    Notice {
        /// Which notice the extension computes for it.
        id: &'static str,
    },
}

/// A toolbox card: a row of command buttons.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolboxCard {
    /// The card's name ("View").
    pub label: &'static str,
    /// The buttons, in order.
    pub commands: Vec<ToolboxCommand>,
}

/// One button of a toolbox card; the id names a shell command (`pan`, `zoom`, `fit_page`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolboxCommand {
    /// The shell command it runs.
    pub id: &'static str,
    /// Tooltip and accessible name.
    pub label: &'static str,
    /// The Lucide glyph name.
    pub icon: &'static str,
}

/// A kind of fixture the machine setup can hold (ADR 0032 §8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureKind {
    /// The kind id inside the process (`engraved_grid`); stored as `<process>/<id>`.
    pub id: &'static str,
    /// The name of a new fixture ("Engraved grid").
    pub label: &'static str,
    /// The Lucide glyph name of its row.
    pub icon: &'static str,
    /// Its parameters, in order.
    pub params: Vec<Param>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy() -> Manifest {
        Manifest {
            id: "test.dummy",
            name: "Dummy",
            version: "0.0.1",
            licence: "AGPL-3.0-or-later",
            contributions: vec![Contribution::Workbench(WorkbenchContribution {
                id: "test.dummy",
                label: "Dummy",
                icon: "Box",
                tabs: vec![Tab {
                    id: "dummy",
                    label: "Dummy",
                    icon: "Box",
                    blocks: vec![Block::FixtureList { label: "Fixtures" }],
                }],
                toolbox_cards: Vec::new(),
                fixture_kinds: vec![FixtureKind {
                    id: "pin",
                    label: "Pin",
                    icon: "Pin",
                    params: Vec::new(),
                }],
            })],
        }
    }

    #[test]
    fn a_manifest_finds_its_workbench_and_fixture_kinds() {
        let manifest = dummy();
        let workbench = manifest.workbench().unwrap();
        assert_eq!(workbench.label, "Dummy");
        assert!(workbench.fixture_kind("pin").is_some());
        assert!(workbench.fixture_kind("grid").is_none());
    }

    #[test]
    fn the_manifest_json_tags_contributions_and_blocks_by_kind() {
        let json = serde_json::to_value(dummy()).unwrap();
        assert_eq!(json["contributions"][0]["kind"], "workbench");
        assert_eq!(
            json["contributions"][0]["tabs"][0]["blocks"][0]["kind"],
            "fixture_list"
        );
    }
}
