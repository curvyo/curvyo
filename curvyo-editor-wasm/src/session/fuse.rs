//! The session's Bridge command (`specs/0156-fused-lettering` criteria 1 to 4, 11, 14 and 17 to
//! 20; `adrs.md` decisions 5 and 7): on the active laser page only, the selected objects become
//! one group "Bridged lettering" or "Bridged stencil" in one commit labelled `fuse`, written by
//! `Document::replace_with_group`; the result is selected alone.

use curvyo_document_core::{NewGroup, NodeId};
use curvyo_laser_core::operations::{ChainScope, OperationList};
use curvyo_ui_core::{
    FUSE_COMMIT_LABEL, FuseRefusal, FuseSettings, Reach, RowKind, object_bounds, plan_fuse,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What the Bridge buttons show (criteria 1 and 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuseAvailability {
    /// The active page is no laser page with its extension on: no Bridge anywhere.
    Absent,
    /// Present and dimmed: nothing selected, or not the Select tool.
    NeedsOne,
    /// Present and enabled.
    Ready,
}

/// Why the session refused Bridge before planning it. Nothing was changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuseSessionRefusal {
    /// The selection holds leaves off the active laser page, or a placement (criterion 4).
    OnePage,
    /// The plan refused.
    Plan(FuseRefusal),
}

/// What [`Session::apply_fuse`] did.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FuseOutcome {
    /// Nothing: no laser page, nothing selected, the Select tool not active, a drag in flight or
    /// a typed entry open, or the write could not be made.
    #[default]
    Ignored,
    /// The selected objects became one group with this many bridges.
    Applied {
        /// The bridges made.
        bridges: usize,
    },
    /// Refused; nothing was changed. The plan's offenders are outlined.
    Refused(FuseSessionRefusal),
}

impl Session {
    /// Whether Bridge exists on the active page: a laser page with its extension on (criteria 1
    /// and 2, `adrs.md` decision 7).
    #[must_use]
    pub fn fuse_available(&self) -> bool {
        self.operations_available()
    }

    /// What the Bridge buttons show now.
    #[must_use]
    pub fn fuse_availability(&self) -> FuseAvailability {
        if !self.fuse_available() {
            FuseAvailability::Absent
        } else if self.boolean_selection().is_empty() {
            FuseAvailability::NeedsOne
        } else {
            FuseAvailability::Ready
        }
    }

    /// Whether every selected leaf is a member of the active laser page and of no other page, and
    /// nothing selected is a placement (criterion 4).
    fn fuse_on_one_page(&self, leaves: &[NodeId]) -> bool {
        let Some(page) = self.active_manufacturing().and_then(|m| m.page) else {
            return false;
        };
        if self
            .selection
            .ids()
            .iter()
            .any(|&id| self.document.is_placement(id))
        {
            return false;
        }
        let membership = self.page_membership();
        let pages = self.document.pages();
        let objects = self.objects();
        leaves.iter().all(|&leaf| {
            objects
                .iter()
                .find(|o| o.id() == leaf)
                .is_some_and(|object| {
                    let mut owners =
                        membership.pages_of(&pages, leaf, object_bounds(object), Reach::Touches);
                    owners.next().is_some_and(|p| p.id == page.id) && owners.next().is_none()
                })
        })
    }

    /// The effective assignment of `leaf` by the nearest rule (criterion 14), as operation ids.
    /// The Bridges get none of their own and inherit what encloses the result (Frage 404 A).
    fn fuse_operations(&self, leaf: NodeId) -> Vec<NodeId> {
        let Some(active) = self.active_operations() else {
            return Vec::new();
        };
        let list = OperationList::new(&active.operations, active.cuts_last);
        list.nearest(&self.document.key_chain(&[leaf]), ChainScope::Whole)
            .map(|a| {
                let mut at = a.operations;
                at.sort_unstable();
                at.into_iter().map(|k| active.operations[k].id).collect()
            })
            .unwrap_or_default()
    }

    /// Bridges the selected objects with `settings` in one commit labelled `fuse`. The tool stays
    /// Select. A refusal changes nothing and outlines the plan's offenders. A selection off the
    /// active laser page is refused before the edit guard, so it gets its own sentence
    /// (criterion 4).
    pub fn apply_fuse(&mut self, settings: FuseSettings) -> FuseOutcome {
        if !self.fuse_available() || !self.rail_command_ready() || self.selection.is_empty() {
            return FuseOutcome::Ignored;
        }
        let selection = self.boolean_selection();
        let leaves: Vec<NodeId> = self
            .units()
            .selected(selection.ids())
            .into_iter()
            .flat_map(|unit| unit.leaves)
            .collect();
        if !self.fuse_on_one_page(&leaves) {
            return FuseOutcome::Refused(FuseSessionRefusal::OnePage);
        }
        self.recording(|this| this.fuse_selection(settings))
    }

    /// The planned write of [`Session::apply_fuse`], inside the recorded step.
    fn fuse_selection(&mut self, settings: FuseSettings) -> FuseOutcome {
        self.flush_select_bar_preview();
        self.end_colour_pick();
        let objects = self.objects();
        let selection = self.boolean_selection();
        let units = self.units().selected(selection.ids());
        let plan = match plan_fuse(&objects, &units, settings, &mut self.minter) {
            Ok(plan) => plan,
            Err(refusal) => {
                self.command_refusal = Some(RefusalMarks::new(offenders(&refusal), &selection));
                return FuseOutcome::Refused(FuseSessionRefusal::Plan(refusal));
            }
        };
        let operations = self.fuse_operations(plan.bottom);
        let written = self.document.replace_with_group(
            &plan.removed,
            plan.place,
            &NewGroup {
                name: plan.group_name,
                style: &plan.style,
                outline: ("Outline", &plan.outline),
                operations: &operations,
                parts_name: "Bridges",
                parts: &plan.bridges,
            },
            FUSE_COMMIT_LABEL,
        );
        let Ok(ids) = written else {
            // invariant: the plan was made from the objects just read, with the place one of them
            // or a group of them only, fresh anchor ids and live operations, so the write cannot
            // fail.
            debug_assert!(false, "a planned bridge could not be written");
            return FuseOutcome::Ignored;
        };
        self.command_refusal = None;
        self.node.clear_selection();
        self.selection.select_single(ids.group);
        self.hovered_object = None;
        // The result is expanded one level, its Bridges collapsed (criterion 11).
        self.panel.expansion.set(ids.group, RowKind::Group, true);
        self.panel.expansion.set(ids.parts, RowKind::Group, false);
        FuseOutcome::Applied {
            bridges: plan.bridges.len(),
        }
    }
}

/// The leaves a plan refusal outlines.
fn offenders(refusal: &FuseRefusal) -> Vec<NodeId> {
    match refusal {
        FuseRefusal::Texts(texts) => texts.offenders.clone(),
        FuseRefusal::ExtendedPath { offenders, .. }
        | FuseRefusal::NoArea { offenders, .. }
        | FuseRefusal::OutOfRange { offenders, .. } => offenders.clone(),
        FuseRefusal::NothingToJoin | FuseRefusal::TooManyBridges | FuseRefusal::NoIslands => {
            Vec::new()
        }
    }
}
