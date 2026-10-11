//! The session's Split and Break apart commands (`specs/0048-split-compound-path`): both take the
//! selected compound paths apart through the one write `Document::replace_with_pieces`, differing
//! only in the plan. All pieces are selected afterwards, together with the objects that stayed.

use curvyo_document_core::PieceFill;
use curvyo_ui_core::{
    BREAK_APART_COMMIT_LABEL, BreakApartRefusal, SPLIT_COMMIT_LABEL, SplitRefusal,
    plan_break_apart, plan_split,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_split`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitOutcome {
    /// Nothing: the Select tool is not active, a drag is in flight or a typed entry is open, or
    /// the write could not be made.
    Ignored,
    /// Compound paths were replaced by their parts.
    Applied {
        /// How many compound paths were split.
        compounds: usize,
        /// How many objects they became.
        pieces: usize,
        /// Whether a piece has a hole.
        with_holes: bool,
        /// How many selected compound paths were one piece and left as they are.
        left_alone: usize,
    },
    /// Refused; nothing was changed.
    Refused(SplitRefusal),
}

/// What [`Session::apply_break_apart`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakApartOutcome {
    /// Nothing, as for [`SplitOutcome::Ignored`].
    Ignored,
    /// Compound paths were replaced by one closed path per outline.
    Applied {
        /// How many compound paths were broken apart.
        compounds: usize,
        /// How many objects they became.
        pieces: usize,
        /// Whether an outline was a hole and is now a filled shape.
        with_holes: bool,
    },
    /// Refused; nothing was changed.
    Refused(BreakApartRefusal),
}

impl Session {
    /// Replaces each selected compound path of more than one part by one object per part, in one
    /// commit labelled `split`, and selects all the pieces together with the selected objects
    /// that stayed. The tool stays Select. A refusal changes nothing.
    pub fn apply_split(&mut self) -> SplitOutcome {
        self.recording(|this| -> SplitOutcome {
            if !this.rail_command_ready() {
                return SplitOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                this.command_refusal = Some(RefusalMarks::new(in_groups.clone(), &selection));
                return SplitOutcome::Refused(SplitRefusal::Groups {
                    offenders: in_groups,
                });
            }
            let plan = match plan_split(&objects, &selection, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    this.command_refusal = None;
                    return SplitOutcome::Refused(refusal);
                }
            };
            let written = this.document.replace_with_pieces(
                &plan.removed,
                &plan.pieces,
                PieceFill::Keep,
                SPLIT_COMMIT_LABEL,
            );
            let Ok(created) = written else {
                // invariant: the plan was made from the objects just read, each compound still
                // exists, every piece has an outline with nodes and the anchor ids are fresh.
                debug_assert!(false, "a planned split could not be written");
                return SplitOutcome::Ignored;
            };
            this.select_after_replace(&plan.kept, &created);
            SplitOutcome::Applied {
                compounds: plan.removed.len(),
                pieces: created.len(),
                with_holes: plan.with_holes,
                left_alone: plan.left_alone,
            }
        })
    }

    /// Replaces each selected compound path by one closed path per outline, in one commit
    /// labelled `break_apart`, and selects all the pieces together with the selected objects that
    /// stayed. The tool stays Select. A refusal changes nothing.
    pub fn apply_break_apart(&mut self) -> BreakApartOutcome {
        self.recording(|this| -> BreakApartOutcome {
            if !this.rail_command_ready() {
                return BreakApartOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                this.command_refusal = Some(RefusalMarks::new(in_groups.clone(), &selection));
                return BreakApartOutcome::Refused(BreakApartRefusal::Groups {
                    offenders: in_groups,
                });
            }
            let plan = match plan_break_apart(&objects, &selection, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    this.command_refusal = None;
                    return BreakApartOutcome::Refused(refusal);
                }
            };
            let written = this.document.replace_with_pieces(
                &plan.removed,
                &plan.pieces,
                PieceFill::Keep,
                BREAK_APART_COMMIT_LABEL,
            );
            let Ok(created) = written else {
                // invariant: as for Split.
                debug_assert!(false, "a planned break apart could not be written");
                return BreakApartOutcome::Ignored;
            };
            this.select_after_replace(&plan.kept, &created);
            BreakApartOutcome::Applied {
                compounds: plan.removed.len(),
                pieces: created.len(),
                with_holes: plan.with_holes,
            }
        })
    }

    /// The state after a command replaced objects by pieces: no refusal outline, no Node-tool
    /// selection (its nodes are gone), the pieces and the objects that stayed selected.
    pub(super) fn select_after_replace(
        &mut self,
        kept: &[curvyo_document_core::NodeId],
        created: &[curvyo_document_core::NodeId],
    ) {
        self.command_refusal = None;
        self.node.clear_selection();
        self.selection.clear();
        for id in kept.iter().chain(created) {
            self.selection.add(*id);
        }
        self.hovered_object = None;
    }
}
