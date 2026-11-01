//! The session's Stroke to path command (`specs/0085-stroke-to-path` criteria 1, 2, 17 to 22,
//! 25 and 26): the selected leaves, through groups at any depth, have their strokes replaced by
//! filled outlines in one commit labelled `stroke_to_path`; the fill parts and the outlines are
//! selected after.

use std::sync::Arc;

use curvyo_document_core::NodeId;
use curvyo_ui_core::{
    StrokeToPathRefusal, TextRefusal, plan_stroke_to_path, stroke_to_path_available,
};

use super::Session;
use super::refusal::RefusalMarks;

/// What [`Session::apply_stroke_to_path`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeToPathOutcome {
    /// Nothing: no subject, the Select tool is not active, a drag is in flight or a typed entry
    /// is open, or the write could not be made.
    Ignored,
    /// Strokes became filled outlines.
    Applied {
        /// Strokes converted (a text is one).
        strokes: usize,
        /// Selected objects without a stroke, left as they are.
        unstroked: usize,
        /// Texts converted to paths.
        texts: usize,
        /// Clones unlinked.
        unlinked: usize,
    },
    /// Refused; nothing was changed. The offenders are outlined.
    Refused(StrokeToPathRefusal),
}

impl Session {
    /// The leaves that are never subjects and not counted: hidden, locked, guides.
    fn stroke_skips(&self) -> impl Fn(NodeId) -> bool + use<> {
        let read = self.document_read();
        let (inert, guides) = (Arc::clone(&read.inert), Arc::clone(&read.guides));
        move |id| inert.contains(&id) || guides.contains(&id)
    }

    /// Whether the Stroke to path button is in the Select bar: something is selected and not only
    /// dimensions (`specs/0070-measure-and-dimension` UI review item 3).
    #[must_use]
    pub fn stroke_to_path_shown(&self) -> bool {
        !self.selection.is_empty() && !self.selection_only_dimensions()
    }

    /// Whether the Stroke to path button is enabled: the Select tool is active and some selected
    /// leaf draws a stroke (criterion 1).
    #[must_use]
    pub fn stroke_to_path_available(&self) -> bool {
        let selection = self.boolean_selection();
        if selection.is_empty() {
            return false;
        }
        let leaves = Arc::clone(&self.document_read().leaves);
        stroke_to_path_available(&leaves, selection.ids(), &self.stroke_skips())
    }

    /// Stroke to path on the selected objects, in one commit. The tool stays Select. A refusal
    /// changes nothing and outlines its offenders.
    pub fn apply_stroke_to_path(&mut self) -> StrokeToPathOutcome {
        self.recording(|this| -> StrokeToPathOutcome {
            if !this.rail_command_ready() || this.selection.is_empty() {
                return StrokeToPathOutcome::Ignored;
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let read = this.document_read();
            let leaves = Arc::clone(&read.leaves);
            let nodes = Arc::clone(&read.nodes);
            let names = |id: NodeId| {
                nodes
                    .iter()
                    .find(|node| node.id == id)
                    .and_then(|node| node.registers.name.clone())
            };
            let selection = this.selection.clone();
            let skips = this.stroke_skips();
            let plan = match plan_stroke_to_path(
                &leaves,
                selection.ids(),
                &skips,
                &names,
                &mut this.minter,
            ) {
                Ok(plan) => plan,
                Err(refusal) => {
                    let offenders = match &refusal {
                        StrokeToPathRefusal::Texts(TextRefusal { offenders, .. })
                        | StrokeToPathRefusal::OutOfRange { offenders } => offenders,
                    };
                    this.command_refusal = Some(RefusalMarks::new(offenders.clone(), &selection));
                    return StrokeToPathOutcome::Refused(refusal);
                }
            };
            if plan.write.outlines.is_empty() {
                return StrokeToPathOutcome::Ignored;
            }
            let Ok(selected) = this.document.stroke_to_path(&plan.write) else {
                // invariant: the plan was made from the objects just read with fresh anchor ids,
                // so the write cannot fail.
                debug_assert!(false, "a planned stroke to path could not be written");
                return StrokeToPathOutcome::Ignored;
            };
            this.command_refusal = None;
            this.node.clear_selection();
            this.selection.set(&selected);
            this.hovered_object = None;
            StrokeToPathOutcome::Applied {
                strokes: plan.strokes,
                unstroked: plan.unstroked,
                texts: plan.texts,
                unlinked: plan.unlinked,
            }
        })
    }
}
