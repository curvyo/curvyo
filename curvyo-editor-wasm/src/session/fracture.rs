//! The session's Fracture and Flatten commands (`specs/0037-fracture-and-flatten`): both write
//! their plan through the one write `Document::replace_with_pieces`, differing only in the plan.
//! All new objects are selected afterwards, together with the operands that stayed.

use curvyo_document_core::{ObjectSnapshot, PieceFill};
use curvyo_ui_core::{
    AnchorIdMinter, BooleanRefusal, FLATTEN_COMMIT_LABEL, FRACTURE_COMMIT_LABEL, ObjectSelection,
    StackPlan, StackRefusal, plan_flatten, plan_fracture,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_fracture`] or [`Session::apply_flatten`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackOutcome {
    /// Nothing: the Select tool is not active, a drag is in flight or a typed entry is open, or
    /// the write could not be made.
    Ignored,
    /// Operands were replaced or removed.
    Applied {
        /// How many objects were selected.
        operands: usize,
        /// How many operands were replaced or removed.
        removed: usize,
        /// How many new objects were made.
        pieces: usize,
        /// Flatten: how many operands were covered completely and removed.
        hidden: usize,
        /// Flatten: a trimmed operand has a stroke, which now follows the trim.
        strokes_follow: bool,
        /// How many open outlines were closed with a straight line.
        closed_open: usize,
        /// How many open outlines enclosed no area and were removed.
        removed_open: usize,
    },
    /// Refused; nothing was changed.
    Refused(StackRefusal),
}

/// The planner of one of the two commands.
type Planner =
    fn(&[ObjectSnapshot], &ObjectSelection, &mut AnchorIdMinter) -> Result<StackPlan, StackRefusal>;

impl Session {
    /// Cuts the selected objects into the pieces their overlaps make, in one commit labelled
    /// `fracture` (criteria 6 to 9). The tool stays Select. A refusal changes nothing.
    pub fn apply_fracture(&mut self) -> StackOutcome {
        self.apply_stack(plan_fracture, FRACTURE_COMMIT_LABEL)
    }

    /// Trims each selected object to what no selected object above it covers, in one commit
    /// labelled `flatten` (criteria 10 to 14). The tool stays Select. A refusal changes nothing.
    pub fn apply_flatten(&mut self) -> StackOutcome {
        self.apply_stack(plan_flatten, FLATTEN_COMMIT_LABEL)
    }

    fn apply_stack(&mut self, planner: Planner, label: &str) -> StackOutcome {
        self.recording(|this| -> StackOutcome {
            if !this.rail_command_ready() {
                return StackOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            // The commands do not look through groups (`0023` criterion 28).
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                this.command_refusal = Some(RefusalMarks::new(in_groups.clone(), &selection));
                return StackOutcome::Refused(StackRefusal::Operands(BooleanRefusal::Groups {
                    offenders: in_groups,
                }));
            }
            let plan = match planner(&objects, &selection, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    this.command_refusal = match &refusal {
                        StackRefusal::Operands(
                            BooleanRefusal::NoArea { offenders, .. }
                            | BooleanRefusal::OutOfRange { offenders, .. }
                            | BooleanRefusal::Groups { offenders },
                        ) => Some(RefusalMarks::new(offenders.clone(), &selection)),
                        _ => None,
                    };
                    return StackOutcome::Refused(refusal);
                }
            };
            let written = this.document.replace_with_pieces(
                &plan.removed,
                &plan.pieces,
                PieceFill::Keep,
                label,
            );
            let Ok(created) = written else {
                // invariant: the plan was made from the objects just read, every piece's source is
                // removed, every piece has an outline with nodes and the anchor ids are fresh.
                debug_assert!(false, "a planned {label} could not be written");
                return StackOutcome::Ignored;
            };
            this.select_after_replace(&plan.kept, &created);
            StackOutcome::Applied {
                operands: plan.operands,
                removed: plan.removed.len(),
                pieces: created.len(),
                hidden: plan.hidden,
                strokes_follow: plan.strokes_follow,
                closed_open: plan.closed_open,
                removed_open: plan.removed_open,
            }
        })
    }
}
