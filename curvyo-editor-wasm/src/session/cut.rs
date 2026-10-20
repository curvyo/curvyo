//! The session's Cut command (`specs/0036-cut-at-crossings`): the selected paths are cut open
//! where they cross, touch or overlap, through the one write `Document::replace_with_pieces`
//! with the fill of the pieces turned off. All pieces are selected afterwards, together with the
//! operands that were not divided.

use curvyo_document_core::PieceFill;
use curvyo_ui_core::{CUT_COMMIT_LABEL, CutRefusal, plan_cut};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_cut`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CutOutcome {
    /// Nothing: the Select tool is not active, a drag is in flight or a typed entry is open, or
    /// the write could not be made.
    Ignored,
    /// Paths were replaced by their pieces.
    Applied {
        /// How many paths were divided.
        paths: usize,
        /// How many pieces they became.
        pieces: usize,
    },
    /// Refused; nothing was changed. The offending objects, if any, are outlined until the
    /// selection or the tool changes or the host clears them.
    Refused(CutRefusal),
}

impl Session {
    /// Cuts the selected paths open where they cross, touch or overlap, in one commit labelled
    /// `cut_at_crossings`, and selects all the pieces together with the paths that were not
    /// divided. The pieces have Fill None. The tool stays Select. A refusal changes nothing.
    pub fn apply_cut(&mut self) -> CutOutcome {
        self.recording(|this| -> CutOutcome {
            if !this.rail_command_ready() {
                return CutOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                this.command_refusal = Some(RefusalMarks::new(in_groups.clone(), &selection));
                return CutOutcome::Refused(CutRefusal::Groups {
                    offenders: in_groups,
                });
            }
            let plan = match plan_cut(&objects, &selection, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    this.command_refusal = match &refusal {
                        CutRefusal::Compound { offenders }
                        | CutRefusal::TextObjects { offenders }
                        | CutRefusal::OutOfRange { offenders, .. }
                        | CutRefusal::Texts(curvyo_ui_core::TextRefusal { offenders, .. }) => {
                            Some(RefusalMarks::new(offenders.clone(), &selection))
                        }
                        _ => None,
                    };
                    return CutOutcome::Refused(refusal);
                }
            };
            let written = this.document.replace_with_pieces(
                &plan.removed,
                &plan.pieces,
                PieceFill::None,
                CUT_COMMIT_LABEL,
            );
            let Ok(created) = written else {
                // invariant: the plan was made from the objects just read, each divided operand
                // still exists, every piece has at least two nodes and the anchor ids are fresh.
                debug_assert!(false, "a planned cut could not be written");
                return CutOutcome::Ignored;
            };
            this.select_after_replace(&plan.kept, &created);
            CutOutcome::Applied {
                paths: plan.removed.len(),
                pieces: created.len(),
            }
        })
    }
}
