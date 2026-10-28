//! `Session`'s Layers list: the rows as the panel reads them, a window at a time, and the presses
//! that select from it (`specs/0050-layers-and-objects-panel` criteria 5 to 15, 38 to 40, 48, 49
//! and 52). The writes of the panel (eye, lock, rename, opacity, layers, moves) are in
//! `layers_edit.rs`; the rules on the canvas in `layers.rs`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{NodeId, NodeRead};
use curvyo_ui_core::{
    Expansion, FINISH_PEN_FIRST, LayerRow, LayerRows, NothingToSelect, RowFacts, RowKind, RowMark,
    inherited_lines, nothing_to_select_notice, objects_count_text, row_accessible_name,
    row_refusal, row_state_line,
};
use serde_json::{Value, json};

use super::Session;
use super::layers::ActiveLayer;

/// What a panel action asks the host to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelReply {
    /// Nothing.
    Quiet,
    /// The Pen has an unfinished path: the panel hint of criterion 53, 2 s, `role="status"`.
    Hint(String),
    /// A line for the canvas notice slot (a status).
    Notice(String),
    /// A refusal for the canvas notice slot.
    Refusal(String),
    /// A line for the panel's visually hidden status region (a reorder by key).
    Announce(String),
}

impl PanelReply {
    /// `{"kind": "hint" | "notice" | "refusal" | "announce", "text": ...}` for the host, or the
    /// empty string for [`PanelReply::Quiet`]: an empty notice is never serialised (customer
    /// feedback 2026-10-14).
    #[must_use]
    pub fn to_json(&self) -> String {
        let (kind, text) = match self {
            Self::Quiet => return String::new(),
            Self::Hint(text) => ("hint", text.as_str()),
            Self::Notice(text) => ("notice", text.as_str()),
            Self::Refusal(text) => ("refusal", text.as_str()),
            Self::Announce(text) => ("announce", text.as_str()),
        };
        json!({ "kind": kind, "text": text }).to_string()
    }
}

/// How a row was pressed (criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowPress {
    /// A plain click, Enter or Space: the node becomes the selection.
    Plain,
    /// Ctrl+click or Ctrl+Space: the node is added or removed.
    Toggle,
    /// Shift+click or Shift+arrow: the range from the anchor among siblings.
    Range,
}

impl RowPress {
    /// The press called `name` (`"plain"`, `"toggle"`, `"range"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(Self::Plain),
            "toggle" => Some(Self::Toggle),
            "range" => Some(Self::Range),
            _ => None,
        }
    }
}

/// The rows of one state of the document and the session, and that state.
#[derive(Debug, Clone)]
struct RowsKey {
    nodes: Arc<[NodeRead]>,
    expansion: Expansion,
    selected: Vec<NodeId>,
    active: Option<NodeId>,
    entered: Option<NodeId>,
}

impl PartialEq for RowsKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.nodes, &other.nodes)
            && self.expansion == other.expansion
            && self.selected == other.selected
            && self.active == other.active
            && self.entered == other.entered
    }
}

/// The panel's view state: not saved, no step (criterion 8).
#[derive(Debug, Default)]
pub(super) struct PanelState {
    pub(super) expansion: Expansion,
    /// The last clicked row: the anchor of a Shift-click.
    anchor: Option<NodeId>,
    /// The maker last clicked a layer row, so the Opacity field and the footer act on the active
    /// layer (criterion 13).
    pub(super) target_layer: bool,
    /// The selection was last made from the list: no automatic scroll (criterion 12).
    from_list: bool,
    /// The first selected object the list last scrolled to.
    scrolled_to: Option<NodeId>,
    /// The row the host should scroll into view, until it takes it.
    pending_scroll: Option<usize>,
    /// The row under the pointer or the focus and its leaves, read once when it is set
    /// (criterion 15), so a frame tests each object against a set.
    pub(super) hover: Option<(NodeId, HashSet<NodeId>)>,
    /// A layer New layer just created, whose rename joins that step (criterion 34).
    pub(super) fresh_layer: Option<NodeId>,
    /// The Opacity field's drag in flight: the targets and the value drawn, never stored.
    pub(super) opacity_preview: Option<(HashSet<NodeId>, curvyo_document_core::Opacity)>,
    rows: RefCell<Option<(RowsKey, Arc<LayerRows>)>>,
}

/// The id a host sends back, as `document.json` writes it.
pub(super) fn parse_id(id: &str) -> Option<NodeId> {
    serde_json::from_str(id).ok()
}

impl Session {
    /// The selected objects of the context, leaves and groups: what the rows mark as selected.
    pub(super) fn selected_units(&self) -> Vec<NodeId> {
        self.units()
            .selected(self.selection.ids())
            .into_iter()
            .map(|unit| unit.id)
            .collect()
    }

    /// The rows of the list now, made once per document read, expansion, selection, active
    /// layer and context (`adrs.md` decision 7).
    pub(super) fn layer_rows(&self) -> Arc<LayerRows> {
        let key = RowsKey {
            nodes: Arc::clone(&self.document_read().nodes),
            expansion: self.panel.expansion.clone(),
            selected: self.selected_units(),
            active: self.active_layer(),
            entered: self.context_group(),
        };
        if let Some((held, rows)) = self.panel.rows.borrow().as_ref()
            && *held == key
        {
            return Arc::clone(rows);
        }
        let selected: HashSet<NodeId> = key.selected.iter().copied().collect();
        let rows = Arc::new(LayerRows::build(
            &key.nodes,
            &key.expansion,
            RowFacts {
                selected: &selected,
                active_layer: key.active,
                entered: key.entered,
            },
        ));
        *self.panel.rows.borrow_mut() = Some((key, Arc::clone(&rows)));
        rows
    }

    /// The collapsed ancestors of the first selected object open, once per change of it, unless
    /// the selection came from the list (criterion 12). The row to scroll to waits for the host.
    fn follow_selection_in_list(&mut self) {
        if let Some(to) = self.opened_for_selection() {
            self.panel.pending_scroll = Some(to);
        }
    }

    /// Opens the ancestors of a newly selected first object and returns its row.
    fn opened_for_selection(&mut self) -> Option<usize> {
        let first = self.selected_units().first().copied();
        if first == self.panel.scrolled_to {
            return None;
        }
        self.panel.scrolled_to = first;
        if std::mem::take(&mut self.panel.from_list) {
            return None;
        }
        let first = first?;
        let mut ancestors = Vec::new();
        let mut current = first;
        while let Some(parent) = self.document.parent_node(current) {
            ancestors.push(parent);
            current = parent;
        }
        for ancestor in ancestors {
            let kind = if self.document.is_layer(ancestor) {
                RowKind::Layer
            } else {
                RowKind::Group
            };
            self.panel.expansion.set(ancestor, kind, true);
        }
        self.layer_rows().index_of(first)
    }

    /// The row a canvas selection asks the list to scroll into view, once (criterion 12).
    pub fn take_layer_scroll(&mut self) -> Option<usize> {
        self.panel.pending_scroll.take()
    }

    /// The subject line of the Layers tab: "40 objects" (criterion 4).
    #[must_use]
    pub fn layers_subject(&self) -> String {
        objects_count_text(self.layer_rows().objects())
    }

    /// The Layers tab's state and the rows `first` to `first + count` as JSON (criterion 48: only
    /// the window crosses to the host): the row count (exact scrollbar), the subject line, the
    /// Opacity header, the footer and the rows. A canvas selection opens the ancestors of its
    /// first row first; the row to scroll to waits in [`Session::take_layer_scroll`]. A second
    /// call at the same state returns the same text.
    pub fn layers_view_json(&mut self, first: usize, count: usize) -> String {
        self.follow_selection_in_list();
        let rows = self.layer_rows();
        let first_selected = self
            .selected_units()
            .first()
            .and_then(|&id| self.shown_row_of(&rows, id));
        let window: Vec<Value> = rows
            .window(first, count)
            .iter()
            .enumerate()
            .map(|(offset, row)| {
                let mut json = row_json(row, first + offset);
                // The trace group's tooltip repeats the success notice (`0025` criteria 30, 68).
                json["traced"] = self
                    .traced_counts(row.id)
                    .map_or(Value::Null, |(paths, nodes)| json!([paths, nodes]));
                json
            })
            .collect();
        json!({
            "count": rows.len(),
            "subject": objects_count_text(rows.objects()),
            "opacity": self.layer_opacity_json(),
            "activeLayer": self.active_layer(),
            "canDeleteLayer": self.active_layer().is_some(),
            "toolbar": self.layer_toolbar_json(),
            "firstSelected": first_selected,
            "rows": window,
        })
        .to_string()
    }

    /// The row of `id`, or of its nearest ancestor that is shown when `id` is inside a collapsed
    /// container (UX review item 11: Shift+Ctrl+L focuses a row that can be seen).
    fn shown_row_of(&self, rows: &LayerRows, id: NodeId) -> Option<usize> {
        let mut current = Some(id);
        while let Some(node) = current {
            if let Some(index) = rows.index_of(node) {
                return Some(index);
            }
            current = self.document.parent_node(node);
        }
        None
    }

    /// The index of the row of `id` in the whole list, `None` when it is not shown.
    #[must_use]
    pub fn layer_row_index(&self, id: &str) -> Option<usize> {
        parse_id(id).and_then(|id| self.layer_rows().index_of(id))
    }

    /// Opens or closes the container `id` (chevron, Right, Left; criterion 8). Not gated by the
    /// Pen (criterion 53) and no step.
    pub fn set_layer_expanded(&mut self, id: &str, expanded: bool) -> bool {
        let Some(id) = parse_id(id) else {
            return false;
        };
        let rows = self.layer_rows();
        let Some(row) = rows.row(id).filter(|row| row.container.is_some()) else {
            return false;
        };
        self.panel.expansion.set(id, row.kind, expanded)
    }

    /// The row under the pointer or the focus: the canvas draws its hover box (criterion 15). An
    /// empty or unknown id, or a layer, clears it.
    pub fn set_layer_hover(&mut self, id: &str) {
        let hovered = parse_id(id).filter(|&id| !self.document.is_layer(id));
        if self.panel.hover.as_ref().map(|(held, _)| *held) == hovered {
            return;
        }
        self.panel.hover =
            hovered.map(|id| (id, self.document.leaves_of(id).into_iter().collect()));
    }

    /// Runs a panel action that writes or selects, unless the Pen has an unfinished path
    /// (criterion 53): then nothing changes and the hint is returned.
    pub(super) fn panel_action(
        &mut self,
        action: impl FnOnce(&mut Self) -> PanelReply,
    ) -> PanelReply {
        if self.pen_path_unfinished() {
            return PanelReply::Hint(FINISH_PEN_FIRST.to_owned());
        }
        self.recording(|this| {
            let reply = action(this);
            this.sync_context();
            reply
        })
    }

    /// A press on the row `id` (criteria 10, 11, 13 and 14).
    pub fn press_layer_row(&mut self, id: &str, press: RowPress) -> PanelReply {
        let Some(id) = parse_id(id) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            this.panel.fresh_layer = None;
            if this.document.is_layer(id) {
                this.press_layer(id);
                return PanelReply::Quiet;
            }
            let read = this.document_read();
            let Some(node) = read.nodes.iter().find(|node| node.id == id) else {
                return PanelReply::Quiet;
            };
            if !node.visible || node.locked {
                return PanelReply::Quiet;
            }
            let rows = this.layer_rows();
            this.press_object(id, press, &rows);
            PanelReply::Quiet
        })
    }

    /// A press on a layer row: the layer becomes the active layer and the panel target, and
    /// nothing is selected (criterion 13).
    fn press_layer(&mut self, layer: NodeId) {
        if self.context_group().is_some() {
            self.leave_to_level(0);
        }
        self.selection.clear();
        self.layers.active = ActiveLayer::Chosen(Some(layer));
        self.panel.target_layer = true;
        self.panel.anchor = Some(layer);
    }

    /// A press on the row of a leaf, clone or group.
    fn press_object(&mut self, id: NodeId, press: RowPress, rows: &LayerRows) {
        let parent = self.document.parent_group(id);
        let same_context = parent == self.context_group();
        let press = if same_context { press } else { RowPress::Plain };
        if !same_context {
            match parent {
                Some(group) => {
                    self.enter_group(group, None);
                }
                None => {
                    self.leave_to_level(0);
                }
            }
        }
        let units = self.units();
        match press {
            RowPress::Plain => self.selection.set(&units.leaves_of_unit(id)),
            RowPress::Toggle => {
                let leaves = units.leaves_of_unit(id);
                let selected = leaves.iter().all(|&leaf| self.selection.contains(leaf));
                let mut now: Vec<NodeId> = self.selection.ids().to_vec();
                if selected {
                    now.retain(|leaf| !leaves.contains(leaf));
                } else {
                    now.extend(leaves);
                }
                self.selection.set(&now);
            }
            RowPress::Range => {
                let range = self
                    .panel
                    .anchor
                    .map_or_else(|| vec![id], |anchor| rows.sibling_range(anchor, id));
                let leaves: Vec<NodeId> = range
                    .into_iter()
                    .filter(|&id| rows.row(id).is_some_and(|r| row_refusal(r).is_none()))
                    .flat_map(|id| units.leaves_of_unit(id))
                    .collect();
                self.selection.set(&leaves);
            }
        }
        if press != RowPress::Range {
            self.panel.anchor = Some(id);
        }
        self.panel.target_layer = false;
        self.panel.from_list = true;
    }

    /// Select original on the clone row `id` (`0050` criterion 38): one step up its chain, under
    /// the hidden-or-locked rule of `0049` criterion 25.
    pub fn select_row_original(&mut self, id: &str) -> PanelReply {
        let Some(clone) = parse_id(id) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            this.selection.set(&[clone]);
            let result = this.select_original();
            if result.notice.is_empty() {
                PanelReply::Quiet
            } else if result.kind == super::clones::CloneKind::Refused {
                // The hidden-or-locked refusal is an alert (UX review should-fix 17).
                PanelReply::Refusal(result.notice)
            } else {
                PanelReply::Notice(result.notice)
            }
        })
    }

    /// Select contents on the layer row `id`: its top-level children that are not hidden or
    /// locked, at once (criterion 13); a notice when there is nothing to select.
    pub fn select_layer_contents(&mut self, id: &str) -> PanelReply {
        let Some(layer) = parse_id(id).filter(|&id| self.document.is_layer(id)) else {
            return PanelReply::Quiet;
        };
        self.panel_action(|this| {
            let name = layer_name(this, layer);
            let Some(registers) = this.document.node_registers(layer) else {
                return PanelReply::Quiet;
            };
            if !registers.visible {
                return PanelReply::Notice(nothing_to_select_notice(
                    &name,
                    NothingToSelect::Hidden,
                ));
            }
            if registers.locked {
                return PanelReply::Notice(nothing_to_select_notice(
                    &name,
                    NothingToSelect::Locked,
                ));
            }
            if this.context_group().is_some() {
                this.leave_to_level(0);
            }
            let units = this.units();
            let leaves: Vec<NodeId> = this
                .document
                .children(Some(layer))
                .into_iter()
                .filter(|&child| !units.is_inert(child))
                .flat_map(|child| units.leaves_of_unit(child))
                .collect();
            if leaves.is_empty() {
                return PanelReply::Notice(nothing_to_select_notice(&name, NothingToSelect::Empty));
            }
            this.selection.set(&leaves);
            this.panel.from_list = true;
            this.panel.target_layer = false;
            PanelReply::Quiet
        })
    }
}

/// The name of the layer `layer` as the list shows it.
pub(super) fn layer_name(session: &Session, layer: NodeId) -> String {
    session
        .document
        .node_registers(layer)
        .and_then(|registers| registers.name)
        .unwrap_or_else(|| format!("Layer {}", layer.short_id()))
}

/// One row as the host reads it.
fn row_json(row: &LayerRow, index: usize) -> Value {
    let (hidden_by, locked_by) = inherited_lines(row);
    let container = row.container;
    json!({
        "index": index,
        "id": row.id,
        "level": row.depth + 1,
        "kind": row.kind.code(),
        "name": row.name,
        "named": row.named,
        "visible": row.own.visible,
        "locked": row.own.locked,
        "effectiveVisible": row.effective.visible,
        "effectiveLocked": row.effective.locked,
        "hiddenBy": hidden_by,
        "lockedBy": locked_by,
        "readout": row.opacity_readout(),
        "container": container.is_some(),
        "expanded": row.expanded(),
        "containsSelection": container.is_some_and(|c| c.contains_selection),
        "mark": match row.mark {
            RowMark::None => "",
            RowMark::Selected => "selected",
            RowMark::ActiveLayer => "active",
            RowMark::Entered => "entered",
        },
        "posinset": row.position,
        "setsize": row.siblings,
        "label": row_accessible_name(row),
        "stateLine": row_state_line(row),
        "chainLine": row.chain_line,
        "overridesLine": row.overrides_line,
        "refusal": row_refusal(row).unwrap_or(""),
        "rowAction": match row.kind {
            RowKind::Layer => "select-contents",
            // `0050` criterion 38: a clone row's action is Select original (`0049`).
            RowKind::Clone => "select-original",
            _ => "",
        },
    })
}
