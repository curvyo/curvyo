//! `Session`'s Layers toolbar on the panel target (`specs/0050-layers-and-objects-panel`, customer
//! feedback 2026-10-14): which of its buttons apply, and Move up, Move down and Duplicate. New
//! layer and Delete layer are in `layers_edit.rs`. Each action is one step and passes the Pen gate
//! of `layers_panel.rs`.

use curvyo_document_core::{CopySource, NodeId, Vec2};
use curvyo_ui_core::{
    DUPLICATE_FAILED_NOTICE, Reorder, move_refusal_text, reorder_plans, unchanged_text,
};
use serde_json::{Value, json};

use super::Session;
use super::layers::ActiveLayer;
use super::layers_panel::PanelReply;

impl Session {
    /// What the toolbar acts on: the active layer when a layer row was clicked last, else the
    /// selected objects of the context (criterion 13).
    fn panel_targets(&self) -> Vec<NodeId> {
        if self.panel.target_layer
            && let Some(layer) = self.active_layer()
        {
            return vec![layer];
        }
        self.selected_units()
    }

    /// Whether a one-place step `how` of `targets` would move anything and is allowed.
    fn can_step(&self, targets: &[NodeId], how: Reorder) -> bool {
        let plans = reorder_plans(&self.document, targets, how);
        !plans.is_empty()
            && plans
                .iter()
                .all(|plan| self.document.check_move(plan).is_ok())
    }

    /// Which toolbar buttons apply now; the host disables the others and never hides one.
    pub(super) fn layer_toolbar_json(&self) -> Value {
        let targets = self.panel_targets();
        json!({
            "newLayer": true,
            "deleteLayer": self.active_layer().is_some(),
            "moveUp": self.can_step(&targets, Reorder::Forward),
            "moveDown": self.can_step(&targets, Reorder::Backward),
            "duplicate": !targets.is_empty(),
        })
    }

    /// Move up (`Reorder::Forward`) or Move down (`Reorder::Backward`): the panel target steps
    /// one place among its siblings, one step (`move_nodes`, criterion 32's rules).
    pub fn step_layer_target(&mut self, how: Reorder) -> PanelReply {
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            let targets = this.panel_targets();
            if targets.is_empty() {
                return PanelReply::Quiet;
            }
            match this.run_reorder(&targets, how) {
                Ok(true) => PanelReply::Quiet,
                Ok(false) => unchanged_text(how).map_or(PanelReply::Quiet, |text| {
                    PanelReply::Announce(text.to_owned())
                }),
                Err(refusal) => PanelReply::Refusal(move_refusal_text(refusal).to_owned()),
            }
        })
    }

    /// Duplicate: the selected objects are copied in place, each directly above its original,
    /// and the copies become the selection; a target layer is copied with its content directly
    /// in front of it and the copy becomes the active layer. One step (`duplicate_objects`).
    pub fn duplicate_layer_target(&mut self) -> PanelReply {
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            let targets = this.panel_targets();
            if targets.is_empty() {
                return PanelReply::Quiet;
            }
            let sources: Vec<CopySource> = targets
                .iter()
                .map(|&id| CopySource {
                    id,
                    anchor_ids: (0..this.document.anchor_count(id))
                        .map(|_| this.minter.mint())
                        .collect(),
                })
                .collect();
            let Ok(copies) = this.document.duplicate_objects(&sources, Vec2::ZERO) else {
                return duplicate_refused();
            };
            if this.panel.target_layer && this.active_layer().is_some() {
                this.layers.active = ActiveLayer::Chosen(copies.first().copied());
            } else {
                let leaves: Vec<NodeId> = copies
                    .iter()
                    .flat_map(|&copy| this.document.leaves_of(copy))
                    .collect();
                this.selection.set(&leaves);
            }
            PanelReply::Quiet
        })
    }
}

/// What a refused Duplicate tells the maker: the alert "Could not duplicate. Nothing was changed."
/// (the document writes nothing when it refuses, `Document::duplicate_objects`).
fn duplicate_refused() -> PanelReply {
    PanelReply::Refusal(DUPLICATE_FAILED_NOTICE.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_duplicate_shows_an_alert_and_never_an_empty_notice() {
        let reply = duplicate_refused();
        assert_eq!(
            reply,
            PanelReply::Refusal("Could not duplicate. Nothing was changed.".into())
        );
        assert!(reply.to_json().contains(r#""kind":"refusal""#));
    }
}
