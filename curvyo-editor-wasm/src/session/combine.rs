//! The session's Combine command and what the Path card shows: the write
//! (`Document::replace_with_path`) and the red outline of the objects a refusal names
//! (`specs/0035-combine-and-break-apart`). Split and Break apart are in `split.rs`, Cut in
//! `cut.rs`.

use curvyo_ui_core::{
    COMBINE_COMMIT_LABEL, CombineRefusal, PathAvailability, path_availability, plan_combine,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_combine`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombineOutcome {
    /// Nothing: the Select tool is not active, a drag is in flight or a typed entry is open, or
    /// the write could not be made.
    Ignored,
    /// The operands were replaced by one compound path.
    Applied {
        /// How many objects took part.
        operands: usize,
        /// How many of the result's outlines are holes.
        holes: usize,
        /// How many of the result's outlines are open.
        open: usize,
        /// How many closed outlines touch or cross another closed outline or themselves.
        touching: usize,
        /// How many holes touch or cross another hole.
        touching_holes: usize,
        /// How many outlines with a single node were left out.
        single_nodes_removed: usize,
        /// Whether an operand had a marker on (the result draws none).
        markers: bool,
        /// Whether the operands' styles were not all equal (the result has the lowest one).
        styles_differ: bool,
    },
    /// Refused; nothing was changed. The offending objects, if any, are outlined until the
    /// selection or the tool changes or the host clears them.
    Refused(CombineRefusal),
}

impl Session {
    /// What the Path card of the tool rail shows now. Read it after every pointer release and tool
    /// or selection change.
    #[must_use]
    pub fn path_availability(&self) -> PathAvailability {
        path_availability(&self.objects(), &self.boolean_selection())
    }

    /// Combines the selected objects into one compound path in one commit labelled
    /// `combine_paths`, and selects the result alone. The tool stays Select. A refusal changes
    /// nothing (no commit, same selection) and outlines its offenders.
    pub fn apply_combine(&mut self) -> CombineOutcome {
        self.recording(|this| -> CombineOutcome {
            if !this.rail_command_ready() {
                return CombineOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            // The command does not look through groups (`0023` criterion 28).
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                this.command_refusal = Some(RefusalMarks::new(in_groups.clone(), &selection));
                return CombineOutcome::Refused(CombineRefusal::Groups {
                    offenders: in_groups,
                });
            }
            let plan = match plan_combine(&objects, &selection, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    this.command_refusal = match &refusal {
                        CombineRefusal::OutOfRange { offenders, .. }
                        | CombineRefusal::Groups { offenders }
                        | CombineRefusal::Texts(curvyo_ui_core::TextRefusal {
                            offenders, ..
                        }) => Some(RefusalMarks::new(offenders.clone(), &selection)),
                        CombineRefusal::NeedsTwo | CombineRefusal::SingleNodesOnly => None,
                    };
                    return CombineOutcome::Refused(refusal);
                }
            };
            let replaced = this.document.replace_with_path(
                &plan.operands,
                plan.base,
                &plan.outlines,
                COMBINE_COMMIT_LABEL,
            );
            let Ok(result) = replaced else {
                // invariant: the plan was made from the objects just read, with a base among the
                // operands, at least one outline and fresh anchor ids, so the write cannot fail.
                debug_assert!(false, "a planned combine could not be written");
                return CombineOutcome::Ignored;
            };
            this.command_refusal = None;
            // The operands are gone, and so are the nodes a Node-tool selection held.
            this.node.clear_selection();
            this.selection.select_single(result);
            this.hovered_object = None;
            CombineOutcome::Applied {
                operands: plan.operands.len(),
                holes: plan.holes,
                open: plan.open,
                touching: plan.touching,
                touching_holes: plan.touching_holes,
                single_nodes_removed: plan.single_nodes_removed,
                markers: plan.markers,
                styles_differ: plan.styles_differ,
            }
        })
    }
}
