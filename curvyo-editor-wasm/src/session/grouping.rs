//! The Group and Ungroup commands of the session with their notices (`specs/0023-groups`). The
//! context they act in is in `context.rs`.

use curvyo_document_core::{NodeId, ObjectEditError};
use curvyo_ui_core::{GroupingAvailability, TOO_DEEP_NOTICE, grouping_availability};

use super::{Session, Tool};

/// What a Group or Ungroup did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingKind {
    /// Nothing: not available (fewer than two objects, no group, another tool, a drag or an open
    /// entry).
    Ignored,
    /// Objects were bound into one group.
    Grouped,
    /// Groups were released.
    Ungrouped,
    /// Refused; nothing was changed.
    Refused,
}

/// The outcome of Group or Ungroup, with the one-line notice for the canvas notice slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupingResult {
    /// What happened.
    pub kind: GroupingKind,
    /// The sentence to show: only a refusal has one (no success notice, customer rule of 2026-10-31).
    pub notice: String,
}

impl GroupingResult {
    fn ignored() -> Self {
        Self {
            kind: GroupingKind::Ignored,
            notice: String::new(),
        }
    }

    /// The code the host keys on: `"ignored"`, `"grouped"`, `"ungrouped"` or `"refused"`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self.kind {
            GroupingKind::Ignored => "ignored",
            GroupingKind::Grouped => "grouped",
            GroupingKind::Ungrouped => "ungrouped",
            GroupingKind::Refused => "refused",
        }
    }
}

impl Session {
    /// What the Grouping card of the rail shows (criteria 2 and 3): Group needs two objects of the
    /// context, Ungroup a selected group, both only with the Select tool.
    #[must_use]
    pub fn grouping_availability(&self) -> GroupingAvailability {
        if self.tool != Tool::Select {
            return GroupingAvailability {
                group: false,
                ungroup: false,
            };
        }
        grouping_availability(&self.units(), &self.selection)
    }

    /// The result of the last Group or Ungroup, once: the chord reports its outcome through
    /// [`super::KeyOutcome`] and the host reads the notice here.
    pub fn take_grouping_result(&mut self) -> Option<GroupingResult> {
        self.grouping_result.take()
    }

    /// Binds the selected objects of the context into one group in one commit labelled `group`
    /// and selects it (criterion 1). Refused with the depth notice when the group would nest more
    /// than 32 levels deep (criterion 5). Nothing happens unless two objects are selected, the
    /// Select tool is active, no drag runs and no entry is open.
    pub fn group_selection(&mut self) -> GroupingResult {
        self.recording(|this| {
            if !this.rail_command_ready() || !this.grouping_availability().group {
                return GroupingResult::ignored();
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let selected = this.units().selected(this.selection.ids());
            let ids: Vec<NodeId> = selected.iter().map(|unit| unit.id).collect();
            match this.document.group_objects(&ids) {
                Ok(group) => {
                    this.command_refusal = None;
                    this.node.clear_selection();
                    let leaves = this.document.leaves_of(group);
                    this.selection.set(&leaves);
                    this.hovered_object = None;
                    GroupingResult {
                        kind: GroupingKind::Grouped,
                        notice: String::new(),
                    }
                }
                Err(ObjectEditError::TooDeep) => GroupingResult {
                    kind: GroupingKind::Refused,
                    notice: TOO_DEEP_NOTICE.to_owned(),
                },
                Err(_) => GroupingResult::ignored(),
            }
        })
    }

    /// Replaces each selected group by its children in one commit labelled `ungroup`; the children
    /// are selected together with the other selected objects (criterion 3). One level only.
    pub fn ungroup_selection(&mut self) -> GroupingResult {
        self.recording(|this| {
            if !this.rail_command_ready() || !this.grouping_availability().ungroup {
                return GroupingResult::ignored();
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let selected = this.units().selected(this.selection.ids());
            let groups: Vec<NodeId> = selected
                .iter()
                .filter(|unit| unit.is_group)
                .map(|unit| unit.id)
                .collect();
            let others: Vec<NodeId> = selected
                .iter()
                .filter(|unit| !unit.is_group)
                .flat_map(|unit| unit.leaves.iter().copied())
                .collect();
            let Ok(released) = this.document.ungroup(&groups) else {
                return GroupingResult::ignored();
            };
            this.command_refusal = None;
            this.node.clear_selection();
            let mut now = others;
            for child in &released {
                now.extend(this.document.leaves_of(*child));
            }
            this.selection.set(&now);
            this.hovered_object = None;
            GroupingResult {
                kind: GroupingKind::Ungrouped,
                notice: String::new(),
            }
        })
    }

    /// The leaves of the groups in the selection, if it holds any: what a command that does not
    /// look through groups outlines when it refuses (criterion 28).
    pub(super) fn selected_group_leaves(&self) -> Vec<NodeId> {
        self.units()
            .selected(self.selection.ids())
            .into_iter()
            .filter(|unit| unit.is_group)
            .flat_map(|unit| unit.leaves)
            .collect()
    }
}
