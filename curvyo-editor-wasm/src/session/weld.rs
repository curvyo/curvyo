//! The session's Weld command (`specs/0068-laserable-lettering` criteria 1 and 19 to 23): the
//! selected objects, groups and texts included, become one path in one commit labelled `weld`,
//! written by `Document::replace_with_path_at`; the result is selected alone.

use curvyo_ui_core::{WELD_COMMIT_LABEL, WeldRefusal, clamp_weld_gap, plan_weld};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_weld`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeldOutcome {
    /// Nothing: nothing is selected, the Select tool is not active, a drag is in flight or a
    /// typed entry is open, or the write could not be made.
    Ignored,
    /// The selected objects became one path.
    Applied {
        /// How many objects were selected (a group is one).
        objects: usize,
        /// Every selected object was a text.
        texts_only: bool,
        /// The result is a compound path.
        compound: bool,
        /// How many pieces the result has.
        pieces: usize,
        /// A dashed open outline was welded as a solid band.
        dashed: bool,
        /// Open outlines closed with a straight line.
        closed_open: usize,
        /// Open outlines that enclosed no area and were removed.
        removed_open: usize,
        /// The operands' styles differed.
        mixed_styles: bool,
    },
    /// Refused; nothing was changed. The offenders are outlined.
    Refused(WeldRefusal),
}

impl Session {
    /// Welds the selected objects at the gap `gap_mm` (clamped to 0 to 1 mm), in one commit
    /// labelled `weld`. The tool stays Select. A refusal changes nothing and outlines its
    /// offenders.
    pub fn apply_weld(&mut self, gap_mm: f64) -> WeldOutcome {
        self.recording(|this| -> WeldOutcome {
            if !this.rail_command_ready() || this.selection.is_empty() {
                return WeldOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            let units = this.units().selected(selection.ids());
            let (gap, _) = clamp_weld_gap(gap_mm);
            let plan = match plan_weld(&objects, &units, gap, &mut this.minter) {
                Ok(plan) => plan,
                Err(refusal) => {
                    let offenders = match &refusal {
                        WeldRefusal::Texts(texts) => &texts.offenders,
                        WeldRefusal::NoArea { offenders, .. }
                        | WeldRefusal::OutOfRange { offenders, .. } => offenders,
                    };
                    this.command_refusal = Some(RefusalMarks::new(offenders.clone(), &selection));
                    return WeldOutcome::Refused(refusal);
                }
            };
            let written = this.document.replace_with_path_at(
                &plan.removed,
                plan.style_from,
                plan.place,
                plan.name.as_deref(),
                &plan.outlines,
                WELD_COMMIT_LABEL,
            );
            let Ok(result) = written else {
                // invariant: the plan was made from the objects just read, with the base among the
                // operands, the place one of them or a group of them only, at least one outline
                // and fresh anchor ids, so the write cannot fail.
                debug_assert!(false, "a planned weld could not be written");
                return WeldOutcome::Ignored;
            };
            this.command_refusal = None;
            this.node.clear_selection();
            this.selection.select_single(result);
            this.hovered_object = None;
            WeldOutcome::Applied {
                objects: plan.objects,
                texts_only: plan.texts_only,
                compound: plan.outlines.len() > 1,
                pieces: plan.pieces,
                dashed: plan.dashed,
                closed_open: plan.closed_open,
                removed_open: plan.removed_open,
                mixed_styles: plan.mixed_styles,
            }
        })
    }
}
