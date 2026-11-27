//! The edits of the tabs of the selected items (`specs/0139` criteria 5, 6, 12; `ux-notes.md` 4,
//! 5.1, 7, 8): the Tabs tool's add, a handle's drag, delete and width, the Own and Operation's
//! presses, each one step.

use curvyo_document_core::{
    ADD_TAB_COMMIT_LABEL, DELETE_TAB_COMMIT_LABEL, MOVE_TAB_COMMIT_LABEL, NodeId,
    OPERATIONS_TABS_COMMIT_LABEL, OWN_TABS_COMMIT_LABEL, Point, TAB_WIDTH_COMMIT_LABEL,
};
use curvyo_laser_core::settings::CutLength;
use curvyo_laser_core::tabs::{
    Measured, OwnTab, OwnTabs, TabSpan, add, read_own_tabs, slide, with_width,
};
use serde::Serialize;

use super::super::Session;
use super::tab_outlines::TabOutline;

/// The pointer within this of a cut path adds a tab there (UX 4.3).
const ADD_PX: f64 = 6.0;
/// A press and release closer than this on screen is a click, not a drag (UX 4.3).
const DEAD_ZONE_PX: f64 = 3.0;
/// A handle snaps to a node within this along the path (criterion 5).
const SNAP_PX: f64 = 8.0;

/// The current handle: an outline and a tab of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Current {
    pub(super) op: NodeId,
    pub(super) key: String,
    pub(super) reference: String,
    pub(super) index: usize,
}

/// A tab drag in flight.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TabDrag {
    pub(super) outline: TabOutline,
    pub(super) index: usize,
    pub(super) centre: f64,
    pub(super) snapped: bool,
    pub(super) limited: bool,
    /// Where the press was, and whether the pointer has left the 3 px dead zone since: a plain
    /// click selects the handle and moves nothing (UX 4.3, 5.1).
    pub(super) from: Point,
    pub(super) live: bool,
}

/// The Tabs tool and the current handle (session state, no step).
#[derive(Debug, Default)]
pub(in crate::session) struct TabsState {
    pub(super) tool: bool,
    pub(super) current: Option<Current>,
    pub(super) drag: Option<TabDrag>,
}

/// The width chip of the current tab (UX 5.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabChipView {
    /// The effective width in mm.
    pub width_mm: f64,
    /// Whether the tab has an own width (the reset button shows).
    pub own_width: bool,
    /// The tab's centre in document mm.
    pub at: Point,
}

/// The Tabs row of the item part (UX 3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemTabsView {
    /// Own pressed (`true`), Operation's (`false`), mixed (`None`).
    pub own: Option<bool>,
    /// "4 tabs", "Own tabs (3)", "4 to 6 tabs", "Own tabs on 2 of 3 items".
    pub readout: String,
}

impl Session {
    /// Whether the Tabs tool is on.
    #[must_use]
    pub fn tabs_tool(&self) -> bool {
        self.operations.tabs.tool && self.tabs_tool_shown()
    }

    /// Whether the Tabs toggle shows: a selected item runs in a Cut or Cut-out and the Toolpath
    /// view is off (UX 4.2).
    #[must_use]
    pub fn tabs_tool_shown(&self) -> bool {
        !self.toolpath_view_on() && !self.tab_outlines(None).is_empty()
    }

    /// Turns the Tabs tool on or off; whether it changed (no step).
    pub fn set_tabs_tool(&mut self, on: bool) -> bool {
        let on = on && self.tabs_tool_shown();
        if self.operations.tabs.tool == on {
            return false;
        }
        self.operations.tabs.tool = on;
        if !on {
            self.operations.tabs.current = None;
        }
        true
    }

    /// Writes the own record of the item of `outline` with `outline`'s tabs replaced by
    /// `spans` and `widths`, one step `label`: the first change writes every listed outline of
    /// the item with what it shows now (all or nothing, criterion 5).
    fn write_tabs(
        &mut self,
        outline: &TabOutline,
        spans: &[TabSpan],
        widths: &[Option<CutLength>],
        label: &str,
    ) -> bool {
        let records = self.document.operation_tabs(outline.op).unwrap_or_default();
        let mut own = records
            .get(&outline.key)
            .and_then(|value| read_own_tabs(value).ok())
            .unwrap_or_default();
        if !records.contains_key(&outline.key) {
            for other in self.tab_outlines(Some(outline.op)) {
                if other.key == outline.key && other.reference != outline.reference {
                    own.set(
                        other.reference.clone(),
                        own_tabs(&other, &other.spans, &other.widths),
                    );
                }
            }
        }
        own.set(outline.reference.clone(), own_tabs(outline, spans, widths));
        let serde_json::Value::Object(map) = own.to_value() else {
            return false;
        };
        let (op, key) = (outline.op, outline.key.clone());
        self.recording_unguarded(|this| {
            this.document
                .write_operation_tabs(op, &key, Some(&map), label)
                .unwrap_or(false)
        })
    }

    /// A press at `point`: on a handle it becomes current and starts its drag; with the Tabs
    /// tool on, near a selected cut path it adds a tab there ("Add tab"). Whether it took the
    /// press.
    pub(in crate::session) fn tab_press(&mut self, point: Point) -> bool {
        if let Some((outline, index)) = self.handle_at(point) {
            self.operations.tabs.current = Some(Current {
                op: outline.op,
                key: outline.key.clone(),
                reference: outline.reference.clone(),
                index,
            });
            let centre = outline.spans[index].centre;
            self.operations.tabs.drag = Some(TabDrag {
                outline,
                index,
                centre,
                snapped: false,
                limited: false,
                from: point,
                live: false,
            });
            return true;
        }
        if !self.tabs_tool() {
            self.operations.tabs.current = None;
            return false;
        }
        let scale = self.view().scale();
        let Some((outline, arc)) = self.addable_at(point) else {
            return false;
        };
        let snap = SNAP_PX / scale * 1000.0;
        let arc = slide_snap(&outline.measured, arc, snap);
        let width = outline.settings.tab_width.mm() * 1000.0;
        let Some((spans, index)) = add(&outline.measured, &outline.spans, arc, width) else {
            // No room: nothing changes and the press is the Tabs tool's (UX 4.3).
            return true;
        };
        let mut widths = outline.widths.clone();
        widths.insert(index, None);
        if self.write_tabs(&outline, &spans, &widths, ADD_TAB_COMMIT_LABEL) {
            self.operations.tabs.current = Some(Current {
                op: outline.op,
                key: outline.key,
                reference: outline.reference,
                index,
            });
        }
        true
    }

    /// The selected cut path within 6 px of `point` and the arc there (UX 4.3).
    fn addable_at(&self, point: Point) -> Option<(TabOutline, f64)> {
        let scale = self.view().scale();
        self.tab_outlines(None).into_iter().find_map(|outline| {
            let (arc, d2) = outline.measured.nearest(outline.to_page_um(point));
            (d2.sqrt() / 1000.0 * scale <= ADD_PX).then_some((outline, arc))
        })
    }

    /// The cursor over the tabs: `pointer` on a handle or while one is held, `crosshair` where
    /// the Tabs tool would add a tab (UX 4.3); `None` elsewhere.
    pub(in crate::session) fn tab_cursor(&self) -> Option<&'static str> {
        if self.operations.tabs.drag.is_some() {
            return Some("pointer");
        }
        let point = self.pointer_position?;
        if self.handle_at(point).is_some() {
            return Some("pointer");
        }
        (self.tabs_tool() && self.addable_at(point).is_some()).then_some("crosshair")
    }

    /// The pointer moved to `point` with a tab held: it slides along the path. Whether a tab is
    /// held.
    pub(in crate::session) fn tab_drag_move(&mut self, point: Point) -> bool {
        let scale = self.view().scale();
        let alt = self.held.alt;
        let Some(drag) = self.operations.tabs.drag.as_mut() else {
            return false;
        };
        if !drag.live {
            if drag.from.vector_to(point).length() * scale < DEAD_ZONE_PX {
                return true;
            }
            drag.live = true;
        }
        let (target, _) = drag
            .outline
            .measured
            .nearest(drag.outline.to_page_um(point));
        let snap = if alt { 0.0 } else { SNAP_PX / scale * 1000.0 };
        let moved = slide(
            &drag.outline.measured,
            &drag.outline.spans,
            drag.index,
            target,
            snap,
        );
        drag.centre = moved.centre;
        drag.snapped = moved.snapped;
        drag.limited = moved.limited;
        true
    }

    /// The release of a tab drag: one step "Move tab" when it moved beyond the dead zone.
    pub(in crate::session) fn tab_drag_end(&mut self, point: Point) -> bool {
        if !self.tab_drag_move(point) {
            return false;
        }
        let Some(drag) = self.operations.tabs.drag.take() else {
            return false;
        };
        let mut spans = drag.outline.spans.clone();
        if (spans[drag.index].centre - drag.centre).abs() < 0.5 {
            return true;
        }
        spans[drag.index].centre = drag.centre;
        let mut order: Vec<usize> = (0..spans.len()).collect();
        order.sort_by(|&a, &b| spans[a].centre.total_cmp(&spans[b].centre));
        let sorted: Vec<TabSpan> = order.iter().map(|&i| spans[i]).collect();
        let widths: Vec<Option<CutLength>> =
            order.iter().map(|&i| drag.outline.widths[i]).collect();
        self.write_tabs(&drag.outline, &sorted, &widths, MOVE_TAB_COMMIT_LABEL);
        true
    }

    /// Escape or a lost pointer during a tab drag: nothing is written.
    pub(in crate::session) fn cancel_tab_drag(&mut self) -> bool {
        self.operations.tabs.drag.take().is_some()
    }

    /// The readout of a tab drag: "42.30 mm" along the path, " · node", " · limit".
    pub(in crate::session) fn tab_readout(&self) -> Option<super::super::shapes::LiveReadout> {
        let drag = self.operations.tabs.drag.as_ref()?;
        let mut text = format!("{:.2} mm", drag.centre / 1000.0);
        if drag.limited {
            text.push_str(" · limit");
        } else if drag.snapped {
            text.push_str(" · node");
        }
        Some(super::super::shapes::LiveReadout {
            text,
            anchor: self.pointer_position?,
        })
    }

    fn current_outline(&self) -> Option<(TabOutline, usize)> {
        let current = self.operations.tabs.current.as_ref()?;
        self.tab_outlines(Some(current.op))
            .into_iter()
            .find(|o| o.key == current.key && o.reference == current.reference)
            .filter(|o| current.index < o.spans.len())
            .map(|o| (o, current.index))
    }

    /// Delete or Backspace with a current tab: it goes ("Delete tab") and the next becomes
    /// current. Whether a tab was current.
    pub(in crate::session) fn delete_current_tab(&mut self) -> bool {
        let Some((outline, index)) = self.current_outline() else {
            return false;
        };
        let mut spans = outline.spans.clone();
        let mut widths = outline.widths.clone();
        spans.remove(index);
        widths.remove(index);
        self.write_tabs(&outline, &spans, &widths, DELETE_TAB_COMMIT_LABEL);
        self.operations.tabs.current = if spans.is_empty() {
            None
        } else {
            self.operations.tabs.current.take().map(|mut c| {
                c.index = index % spans.len();
                c
            })
        };
        true
    }

    /// Delete or Backspace with a current tab removes it and never the object (UX 5.1).
    pub(in crate::session) fn tab_key(
        &mut self,
        input: &super::super::keys::KeyInput<'_>,
    ) -> Option<super::super::keys::KeyOutcome> {
        if input.dom_blocked || input.ctrl || input.alt {
            return None;
        }
        (matches!(input.key, "Delete" | "Backspace") && self.delete_current_tab())
            .then_some(super::super::keys::KeyOutcome::Deleted)
    }

    /// Escape: clears the current handle, else turns the Tabs tool off; whether it did either.
    pub(in crate::session) fn tabs_escape(&mut self) -> bool {
        if self.operations.tabs.current.take().is_some() {
            return true;
        }
        std::mem::take(&mut self.operations.tabs.tool)
    }

    /// The width chip of the current tab, `None` without one.
    #[must_use]
    pub fn current_tab_chip(&self) -> Option<TabChipView> {
        let (outline, index) = self.current_outline()?;
        let span = outline.spans[index];
        Some(TabChipView {
            width_mm: span.width / 1000.0,
            own_width: outline.widths[index].is_some(),
            at: outline.to_document(span.centre),
        })
    }

    /// Types `mm` as the current tab's width (`None`: back to the operation's width), one step
    /// "Tab width" (criterion 6). `Ok(false)` without a current tab or when nothing changes.
    ///
    /// # Errors
    /// "Width is 0.10 to 20.00 mm." or "No room for this width."
    pub fn set_current_tab_width(&mut self, mm: Option<f64>) -> Result<bool, String> {
        let Some((outline, index)) = self.current_outline() else {
            return Ok(false);
        };
        let own = match mm {
            Some(mm) => Some(CutLength::typed_tab(mm).map_err(str::to_owned)?),
            None => None,
        };
        let width = own.unwrap_or(outline.settings.tab_width).mm() * 1000.0;
        let spans =
            with_width(&outline.measured, &outline.spans, index, width).map_err(str::to_owned)?;
        let mut widths = outline.widths.clone();
        if widths[index] == own && outline.own {
            return Ok(false);
        }
        widths[index] = own;
        Ok(self.write_tabs(&outline, &spans, &widths, TAB_WIDTH_COMMIT_LABEL))
    }

    /// The Tabs pill of operation `op` for the selected items (UX 3.2): Own copies what they
    /// show now ("Own tabs"), Operation's drops their records ("Back to the operation's").
    pub fn set_selection_tabs_own(&mut self, op: NodeId, own: bool) -> bool {
        let outlines = self.tab_outlines(Some(op));
        let mut keys: Vec<String> = outlines.iter().map(|o| o.key.clone()).collect();
        keys.sort();
        keys.dedup();
        let records = self.document.operation_tabs(op).unwrap_or_default();
        let mut writes: Vec<(String, Option<serde_json::Map<String, serde_json::Value>>)> =
            Vec::new();
        for key in keys {
            let present = records.contains_key(&key);
            if own && !present {
                let mut record = OwnTabs::default();
                for o in outlines.iter().filter(|o| o.key == key) {
                    record.set(o.reference.clone(), own_tabs(o, &o.spans, &o.widths));
                }
                if let serde_json::Value::Object(map) = record.to_value() {
                    writes.push((key, Some(map)));
                }
            } else if !own && present {
                writes.push((key, None));
            }
        }
        if writes.is_empty() {
            return false;
        }
        let label = if own {
            OWN_TABS_COMMIT_LABEL
        } else {
            OPERATIONS_TABS_COMMIT_LABEL
        };
        self.recording_unguarded(|this| {
            this.document
                .write_operation_tabs_many(op, &writes, label)
                .unwrap_or(false)
        })
    }

    /// The Tabs row of the item part of operation `op`, `None` when no selected item runs in it.
    pub(in crate::session) fn item_tabs_view(&self, op: NodeId) -> Option<ItemTabsView> {
        let outlines = self.tab_outlines(Some(op));
        if outlines.is_empty() {
            return None;
        }
        let mut items: Vec<(String, bool, usize)> = Vec::new();
        for o in &outlines {
            match items.iter_mut().find(|i| i.0 == o.key) {
                Some(item) => item.2 += o.spans.len(),
                None => items.push((o.key.clone(), o.own, o.spans.len())),
            }
        }
        let owns = items.iter().filter(|i| i.1).count();
        let own = if owns == 0 {
            Some(false)
        } else if owns == items.len() {
            Some(true)
        } else {
            None
        };
        let counts: Vec<usize> = items.iter().map(|i| i.2).collect();
        let low = counts.iter().copied().min().unwrap_or(0);
        let high = counts.iter().copied().max().unwrap_or(0);
        let readout = match own {
            None => format!("Own tabs on {owns} of {} items", items.len()),
            Some(true) if items.len() == 1 => format!("Own tabs ({low})"),
            _ if low == high => format!("{low} {}", if low == 1 { "tab" } else { "tabs" }),
            _ => format!("{low} to {high} tabs"),
        };
        Some(ItemTabsView { own, readout })
    }
}

/// The own tabs of `outline` with `spans` and `widths` as stored fractions.
fn own_tabs(outline: &TabOutline, spans: &[TabSpan], widths: &[Option<CutLength>]) -> Vec<OwnTab> {
    spans
        .iter()
        .zip(widths)
        .map(|(span, width)| OwnTab {
            at: outline.measured.fraction_of(span.centre),
            width: *width,
        })
        .collect()
}

/// `arc` snapped to the nearest node within `snap` µm of arc.
fn slide_snap(measured: &Measured, arc: f64, snap: f64) -> f64 {
    measured
        .vertex_arcs()
        .iter()
        .copied()
        .find(|node| (node - arc).abs() <= snap)
        .unwrap_or(arc)
}
