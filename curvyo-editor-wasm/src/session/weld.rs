//! The session's Weld command (`specs/0068-laserable-lettering` criteria 1, 3 and 17 to 24): the
//! selected objects, groups and texts included, become one path in one commit labelled `weld`,
//! written by `Document::replace_with_path_at`; the result is selected alone.

use curvyo_document_core::Length;
use curvyo_ui_core::{
    WELD_COMMIT_LABEL, WeldAvailability, WeldRefusal, format_status_length, plan_weld,
    weld_availability,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_weld`] did.
#[derive(Debug, Clone, PartialEq)]
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
        /// The result's total line length in millimetres.
        length_mm: f64,
        /// That length in the display unit with the status bar's decimals and the unit, as
        /// "140.0 mm" (UX notes, "Notices").
        length_text: String,
        /// Operands covered completely and removed.
        hidden: usize,
        /// Open outlines closed with a straight line.
        closed_open: usize,
        /// Open outlines that enclosed no area and were removed.
        removed_open: usize,
        /// The operands' styles differed.
        mixed_styles: bool,
        /// The lowest object had Fill on, which the result switches off.
        fill_switched_off: bool,
        /// The lowest object had Stroke off, which the result switches on.
        stroke_switched_on: bool,
    },
    /// Refused; nothing was changed. The offenders are outlined.
    Refused(WeldRefusal),
}

impl Session {
    /// What the Weld button shows now: dimmed without a selection or with another tool than
    /// Select, and how many open outlines a press closes.
    #[must_use]
    pub fn weld_availability(&self) -> WeldAvailability {
        let selection = self.boolean_selection();
        let units = self.units().selected(selection.ids());
        weld_availability(&self.objects(), &units)
    }

    /// Welds the selected objects in one commit labelled `weld`. The tool stays Select. A refusal
    /// changes nothing and outlines its offenders.
    pub fn apply_weld(&mut self) -> WeldOutcome {
        self.recording(|this| -> WeldOutcome {
            if !this.rail_command_ready() || this.selection.is_empty() {
                return WeldOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let selection = this.boolean_selection();
            let units = this.units().selected(selection.ids());
            let plan = match plan_weld(&objects, &units, &mut this.minter) {
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
                &plan.style,
                plan.place,
                &plan.outlines,
                WELD_COMMIT_LABEL,
            );
            let Ok(result) = written else {
                // invariant: the plan was made from the objects just read, with the place one of
                // them or a group of them only, at least one outline and fresh anchor ids, so the
                // write cannot fail.
                debug_assert!(false, "a planned weld could not be written");
                return WeldOutcome::Ignored;
            };
            let unit = this.display_unit();
            let length_text = format!(
                "{} {}",
                format_status_length(Length::from_mm(plan.length_mm), unit),
                unit.symbol()
            );
            this.command_refusal = None;
            this.node.clear_selection();
            this.selection.select_single(result);
            this.hovered_object = None;
            WeldOutcome::Applied {
                objects: plan.objects,
                texts_only: plan.texts_only,
                compound: plan.outlines.len() > 1,
                length_mm: plan.length_mm,
                length_text,
                hidden: plan.hidden,
                closed_open: plan.closed_open,
                removed_open: plan.removed_open,
                mixed_styles: plan.mixed_styles,
                fill_switched_off: plan.fill_switched_off,
                stroke_switched_on: plan.stroke_switched_on,
            }
        })
    }
}
