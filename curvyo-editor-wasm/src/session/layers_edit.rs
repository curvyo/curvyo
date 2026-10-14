//! `Session`'s writes from the Layers panel: the eye and the lock, rename, the Opacity field,
//! New layer, Delete layer and Delete on a row, and moves by drag and by Alt+arrow keys (the
//! toolbar's Move up, Move down and Duplicate are in `layers_toolbar.rs`)
//! (`specs/0050-layers-and-objects-panel` criteria 16, 18, 20, 26, 29 to 32, 34, 36 and 37). Each
//! is one step; all pass the Pen gate of `layers_panel.rs`.

use std::sync::Arc;

use curvyo_document_core::{
    MovePlan, MoveTarget, NodeId, ObjectEditError, ObjectSnapshot, Opacity,
};
use curvyo_render_core::drawn_leaves_with;
use curvyo_ui_core::{
    Grid, OpacityTarget, Reorder, RowMark, ValueScale, delete_locked_layer_notice,
    deleted_layer_notice, locked_object_notice, move_refusal_text, moved_announcement,
    opacity_caption, percent, unchanged_text,
};
use serde_json::{Value, json};

use super::Session;
use super::layers::ActiveLayer;
use super::layers_panel::{PanelReply, layer_name, parse_id};

/// Which own flag a row button toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowFlag {
    /// The eye (criterion 18).
    Visible,
    /// The lock (criterion 20).
    Locked,
}

impl RowFlag {
    /// The flag called `name` (`"visible"`, `"locked"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "visible" => Some(Self::Visible),
            "locked" => Some(Self::Locked),
            _ => None,
        }
    }
}

/// Where a drop puts the dragged rows, relative to the row under the pointer (criterion 29).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPlace {
    /// On the line above the row: in front of it.
    Above,
    /// On the line below the row: behind it.
    Below,
    /// On the container row: inside it, as its front-most children.
    Into,
}

impl DropPlace {
    /// The place called `name` (`"above"`, `"below"`, `"into"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "above" => Some(Self::Above),
            "below" => Some(Self::Below),
            "into" => Some(Self::Into),
            _ => None,
        }
    }
}

impl Session {
    /// The nodes a press on the row `id` acts on: every selected object when the row is
    /// selected, else the row's node (criteria 18 and 29).
    fn row_targets(&self, id: NodeId) -> Vec<NodeId> {
        let rows = self.layer_rows();
        if rows
            .row(id)
            .is_some_and(|row| row.mark == RowMark::Selected)
        {
            self.selected_units()
        } else {
            vec![id]
        }
    }

    /// The eye or the lock of the row `id`: its own flag toggles, on every selected row with it
    /// when it is selected, in one step (criteria 18, 20 and 22).
    pub fn toggle_layer_row_flag(&mut self, id: &str, flag: RowFlag) -> PanelReply {
        let Some(id) = parse_id(id) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            let Some(registers) = this.document.node_registers(id) else {
                return PanelReply::Quiet;
            };
            let targets = this.row_targets(id);
            let _ = match flag {
                RowFlag::Visible => this.document.set_node_visible(&targets, !registers.visible),
                RowFlag::Locked => this.document.set_node_locked(&targets, !registers.locked),
            };
            PanelReply::Quiet
        })
    }

    /// Commits a rename of the row `id` (criterion 16). Right after New layer it joins that
    /// layer's step (criterion 34).
    pub fn rename_layer_row(&mut self, id: &str, name: &str) -> PanelReply {
        let Some(id) = parse_id(id) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            if this.panel.fresh_layer.take() == Some(id) {
                this.document.continue_step();
            }
            let current = this.document.node_registers(id).and_then(|r| r.name);
            if current == curvyo_document_core::clean_name(name) {
                return PanelReply::Quiet;
            }
            if this.document.is_layer(id) && curvyo_document_core::clean_name(name).is_none() {
                // A layer always has a name: an empty rename keeps it.
                return PanelReply::Quiet;
            }
            let _ = this.document.rename_node(id, name);
            PanelReply::Quiet
        })
    }

    /// What the Opacity field acts on, and its words (criterion 26).
    fn opacity_target(&self) -> (Vec<NodeId>, OpacityTarget) {
        if self.panel.target_layer
            && let Some(layer) = self.active_layer()
        {
            let locked = self
                .document
                .node_registers(layer)
                .is_some_and(|registers| registers.locked);
            if locked {
                return (Vec::new(), OpacityTarget::Locked);
            }
            return (vec![layer], OpacityTarget::Layer(layer_name(self, layer)));
        }
        let units = self.selected_units();
        if units.is_empty() {
            return (Vec::new(), OpacityTarget::Nothing);
        }
        let count = units.len();
        (units, OpacityTarget::Objects(count))
    }

    /// The Opacity header as the host reads it: the caption, whether the field is in the tree,
    /// whether the target is a layer, and the value (`null` for Mixed).
    pub(super) fn layer_opacity_json(&self) -> Value {
        let (targets, target) = self.opacity_target();
        let values: Vec<u8> = targets
            .iter()
            .filter_map(|&id| self.document.node_registers(id))
            .map(|registers| percent(registers.opacity))
            .collect();
        let stored = values
            .first()
            .filter(|&&first| values.iter().all(|&v| v == first))
            .copied();
        // While the field is dragged it shows the dragged value (customer feedback 2026-10-14).
        let value = match &self.panel.opacity_preview {
            Some((_, opacity)) if !targets.is_empty() => Some(percent(*opacity)),
            _ => stored,
        };
        let bar = value.map_or(0.0, |value| {
            ValueScale::Opacity.position_of(f64::from(value))
        });
        json!({
            "caption": opacity_caption(&target),
            "field": !targets.is_empty(),
            "layer": matches!(target, OpacityTarget::Layer(_)),
            "value": value,
            "bar": bar,
        })
    }

    /// The Opacity field is dragged to `percent`: the canvas shows it, nothing is written
    /// (criterion 26, "live while dragging"). It checks the Pen itself instead of going through
    /// `panel_action`: a preview writes nothing and records no step, so it needs no recording;
    /// with an unfinished path it shows nothing.
    pub fn preview_layer_opacity(&mut self, value: f64) {
        if self.pen_path_unfinished() {
            return;
        }
        let (targets, _) = self.opacity_target();
        if let Ok(opacity) = Opacity::new((value / 100.0).clamp(0.0, 1.0)) {
            self.panel.opacity_preview = Some((targets.into_iter().collect(), opacity));
        }
    }

    /// The Opacity field is dragged to position `p` of its scale (`ValueScale::Opacity`, the
    /// Style tab's), rounded to `grid`: [`Session::preview_layer_opacity`] of that value.
    pub fn preview_layer_opacity_at(&mut self, p: f64, grid: Grid) {
        let scale = ValueScale::Opacity;
        self.preview_layer_opacity(scale.round(scale.value_at_drag(p), grid));
    }

    /// Escape in the Opacity field: the preview ends and the stored value shows again.
    pub fn cancel_layer_opacity(&mut self) {
        self.panel.opacity_preview = None;
    }

    /// The Opacity field commits `percent` to every target, in one step (criterion 26).
    pub fn commit_layer_opacity(&mut self, value: f64) -> PanelReply {
        self.panel.opacity_preview = None;
        self.panel_action(|this| {
            let (targets, _) = this.opacity_target();
            let Ok(opacity) = Opacity::new((value / 100.0).clamp(0.0, 1.0)) else {
                return PanelReply::Quiet;
            };
            if targets.is_empty() {
                return PanelReply::Quiet;
            }
            let unchanged = targets.iter().all(|&id| {
                this.document
                    .node_registers(id)
                    .is_some_and(|registers| percent(registers.opacity) == percent(opacity))
            });
            if !unchanged {
                let _ = this.document.set_node_opacity(&targets, opacity);
            }
            PanelReply::Quiet
        })
    }

    /// The Opacity field's release: the previewed value is committed, once (criterion 26).
    pub fn commit_layer_opacity_preview(&mut self) -> PanelReply {
        match self
            .panel
            .opacity_preview
            .as_ref()
            .map(|(_, opacity)| *opacity)
        {
            Some(opacity) => self.commit_layer_opacity(opacity.get() * 100.0),
            None => PanelReply::Quiet,
        }
    }

    /// The panel hint while the Pen has an unfinished path, else empty: what a press that would
    /// open a rename or start an Opacity drag shows instead (criterion 53).
    #[must_use]
    pub fn layers_pen_hint(&self) -> String {
        if self.pen_path_unfinished() {
            curvyo_ui_core::FINISH_PEN_FIRST.to_owned()
        } else {
            String::new()
        }
    }

    /// The drawn leaves with the Opacity preview applied, while one runs.
    pub(super) fn previewed_drawn(&self) -> Option<Arc<[ObjectSnapshot]>> {
        let (targets, opacity) = self.panel.opacity_preview.as_ref()?;
        let read = self.document_read();
        Some(drawn_leaves_with(&read.nodes, &|id| targets.contains(&id), *opacity).into())
    }

    /// New layer: an empty "Layer N" directly in front of the active layer, which becomes the
    /// active layer and the panel target (criterion 34). Its rename right after joins the step.
    pub fn new_layer_row(&mut self) -> PanelReply {
        self.panel_action(|this| {
            if this.context_group().is_some() {
                this.leave_to_level(0);
            }
            let layer = this.document.new_layer(this.active_layer());
            this.selection.clear();
            this.layers.active = ActiveLayer::Chosen(Some(layer));
            this.panel.target_layer = true;
            this.panel.fresh_layer = Some(layer);
            PanelReply::Quiet
        })
    }

    /// Delete layer (the footer button): the active layer goes with its content (criterion 36).
    pub fn delete_active_layer(&mut self) -> PanelReply {
        match self.active_layer() {
            Some(layer) => self.panel_action(|this| this.delete_layer_now(layer)),
            None => PanelReply::Quiet,
        }
    }

    /// Delete on the focused row: a layer row deletes the layer, an object row the selection as
    /// the canvas Delete does (criteria 36 and 37).
    pub fn delete_layer_row(&mut self, id: &str) -> PanelReply {
        let Some(id) = parse_id(id) else {
            return PanelReply::Quiet;
        };
        if self.document.is_layer(id) {
            return self.panel_action(|this| this.delete_layer_now(id));
        }
        // Delete acts on what the focus shows (criterion 37, amended 2026-10-12): the whole
        // selection when the row's node is part of it, else only that node.
        let leaves = self.document.leaves_of(id);
        let in_selection =
            !leaves.is_empty() && leaves.iter().all(|&leaf| self.selection.contains(leaf));
        self.panel_action(|this| {
            if in_selection {
                this.delete_selected();
                return PanelReply::Quiet;
            }
            let locked = this
                .document_read()
                .nodes
                .iter()
                .find(|node| node.id == id)
                .is_some_and(|node| node.locked);
            if locked {
                let name = this
                    .layer_rows()
                    .row(id)
                    .map_or_else(|| id.short_id(), |row| row.name.clone());
                return PanelReply::Refusal(locked_object_notice(&name));
            }
            let _ = this.document.delete_objects(&[id]);
            PanelReply::Quiet
        })
    }

    fn delete_layer_now(&mut self, layer: NodeId) -> PanelReply {
        self.panel.fresh_layer = None;
        let name = layer_name(self, layer);
        let layers: Vec<NodeId> = self
            .document
            .children(None)
            .into_iter()
            .filter(|&id| self.document.is_layer(id))
            .collect();
        let at = layers.iter().position(|&id| id == layer);
        match self.document.delete_layer(layer) {
            Ok(objects) => {
                let next = at.and_then(|at| {
                    at.checked_sub(1)
                        .and_then(|back| layers.get(back))
                        .or_else(|| layers.get(at + 1))
                        .copied()
                });
                self.layers.active = ActiveLayer::Chosen(next);
                self.panel.target_layer = next.is_some();
                PanelReply::Notice(deleted_layer_notice(&name, objects))
            }
            Err(ObjectEditError::Locked) => PanelReply::Refusal(delete_locked_layer_notice(&name)),
            Err(_) => PanelReply::Quiet,
        }
    }

    /// The move a drop of the row `pressed` at `place` of the row `target` makes.
    fn drop_plan(&self, pressed: NodeId, target: NodeId, place: DropPlace) -> MovePlan {
        MovePlan {
            nodes: self.row_targets(pressed),
            target: match place {
                DropPlace::Above => MoveTarget::Above(target),
                DropPlace::Below => MoveTarget::Below(target),
                DropPlace::Into => MoveTarget::FrontOf(Some(target)),
            },
        }
    }

    /// Why a drop would be refused, in the maker's words, or empty when it is allowed: the
    /// cursor and the tooltip during a drag (criterion 30).
    pub fn check_layer_drop(&self, pressed: &str, target: &str, place: &str) -> String {
        let (Some(pressed), Some(target), Some(place)) = (
            parse_id(pressed),
            parse_id(target),
            DropPlace::from_name(place),
        ) else {
            return String::new();
        };
        match self
            .document
            .check_move(&self.drop_plan(pressed, target, place))
        {
            Ok(()) => String::new(),
            Err(refusal) => move_refusal_text(refusal).to_owned(),
        }
    }

    /// A drop of the row `pressed` (with the selection when it is selected) at `place` of the
    /// row `target`: one step (criteria 29 to 31).
    pub fn drop_layer_rows(&mut self, pressed: &str, target: &str, place: &str) -> PanelReply {
        let (Some(pressed), Some(target), Some(place)) = (
            parse_id(pressed),
            parse_id(target),
            DropPlace::from_name(place),
        ) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            let plan = this.drop_plan(pressed, target, place);
            match this.document.move_nodes(&plan) {
                Ok(()) => PanelReply::Quiet,
                Err(refusal) => PanelReply::Refusal(move_refusal_text(refusal).to_owned()),
            }
        })
    }

    /// Alt+Up, Alt+Down, Alt+Home, Alt+End, Alt+Left and Alt+Right on the focused row: the
    /// selected rows (or the focused one) move, one step, and a screen reader hears where to
    /// (criterion 32).
    pub fn reorder_layer_rows(&mut self, focused: &str, how: Reorder) -> PanelReply {
        let Some(focused) = parse_id(focused) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            let nodes = this.row_targets(focused);
            match this.run_reorder(&nodes, how) {
                Ok(true) => {
                    if let Some(parent) = this.document.parent_node(focused) {
                        let kind = if this.document.is_layer(parent) {
                            curvyo_ui_core::RowKind::Layer
                        } else {
                            curvyo_ui_core::RowKind::Group
                        };
                        this.panel.expansion.set(parent, kind, true);
                    }
                    let rows = this.layer_rows();
                    rows.row(focused).map_or(PanelReply::Quiet, |row| {
                        PanelReply::Announce(moved_announcement(
                            &row.name,
                            row.position,
                            row.siblings,
                        ))
                    })
                }
                // A press that changes nothing is said (criterion 32, amended 2026-10-12).
                Ok(false) => unchanged_text(how).map_or(PanelReply::Quiet, |text| {
                    PanelReply::Announce(text.to_owned())
                }),
                Err(refusal) => PanelReply::Refusal(move_refusal_text(refusal).to_owned()),
            }
        })
    }
}
