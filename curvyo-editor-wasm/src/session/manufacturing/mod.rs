//! The session's manufacturing workbenches (ADR 0032 §1, §2): entering and leaving one with its
//! own view and tab, which manufacturings are available, and the extension switches. Each
//! workbench is session view state; switching writes nothing to the document.

mod commands;
mod frame;
mod machine;
mod machine_view;
mod roles;
#[cfg(test)]
mod roles_tests;
#[cfg(test)]
mod tests;
mod view_json;

use std::collections::HashMap;

use curvyo_document_core::{ManufacturingSnapshot, NodeId, Point};
use curvyo_ui_core::{ExtensionSettings, PanelTab, Viewport};

use super::{Session, Workbench};

/// The read-only notice of the Laser workbench (UX notes §5): shown for 2 s by the host; a new
/// one restarts it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadOnlyNotice {
    /// Changes with every notice.
    pub seq: u32,
    /// The press position in canvas CSS pixels; `None` for a key, which anchors to the canvas.
    pub at: Option<(f64, f64)>,
}

/// The text of the read-only notice (29 characters, criterion 7).
pub const READ_ONLY_NOTICE: &str = "Read-only here. Edit in Vector.";

/// The free area left of and above an opened output page, CSS pixels: the tool rail and the
/// bar row (the same allowance a template load makes).
const FIT_FREE_LEFT_PX: f64 = 64.0;
const FIT_FREE_TOP_PX: f64 = 24.0;

/// What the session keeps for the manufacturing workbenches.
#[derive(Debug, Default)]
pub(super) struct ManufacturingState {
    /// The extensions the maker switched off (`extensions.toml`), per user.
    pub(super) extensions: ExtensionSettings,
    /// The drawing workbenches' view while a manufacturing is shown.
    drawing_view: Option<Viewport>,
    /// The tab the drawing workbench showed when a manufacturing was entered.
    drawing_tab: Option<PanelTab>,
    /// Each manufacturing's own view, kept while the session lasts.
    views: HashMap<NodeId, Viewport>,
    /// Where an undo of Add manufacturing returns to.
    return_to: Workbench,
    /// The manufacturing deleted from its own workbench: the undo that restores it returns there.
    deleted_from: Option<NodeId>,
    /// The last read-only notice.
    notice: Option<ReadOnlyNotice>,
    /// The members of each placement for the drawing read they were computed from.
    pub(super) placed: frame::PlacedCache,
    /// The maker's machine profiles and the last pick (`specs/0026-machine-profile`).
    pub(super) machine: machine::MachineState,
    /// The leaf-to-layer map and the Role view switch (`specs/0027-manufacturing-roles`).
    pub(super) roles: roles::RoleState,
}

impl Session {
    /// The active manufacturing, `None` in a drawing workbench or when it is gone.
    pub(super) fn active_manufacturing(&self) -> Option<ManufacturingSnapshot> {
        match self.workbench() {
            Workbench::Manufacturing(id) => self.document.manufacturing(id),
            _ => None,
        }
    }

    /// Whether the extension of `process` is compiled in and switched on.
    #[must_use]
    pub fn process_enabled(&self, process: &str) -> bool {
        curvyo_ui_core::manifest(process).is_some()
            && self.manufacturing.extensions.is_enabled(process)
    }

    /// Whether the manufacturing `id` exists and its workbench can open.
    #[must_use]
    pub fn manufacturing_available(&self, id: NodeId) -> bool {
        self.document
            .manufacturing(id)
            .is_some_and(|m| self.process_enabled(&m.process))
    }

    /// The label of the manufacturing workbench's tab, from its manifest.
    pub(super) fn manufacturing_tab_label(&self, id: NodeId) -> &'static str {
        self.document
            .manufacturing(id)
            .and_then(|m| curvyo_ui_core::manifest(&m.process))
            .and_then(|manifest| {
                manifest
                    .workbench()
                    .and_then(|w| w.tabs.first())
                    .map(|t| t.label)
            })
            .unwrap_or("Workbench")
    }

    /// Saves the view and tab of the workbench being left and sets those of the one entered
    /// (UX notes §3: each workbench keeps its own view; entering selects the manifest tab and
    /// leaving restores the drawing's tab). Called by `set_workbench` before the switch.
    pub(super) fn swap_workbench_view(&mut self, from: Workbench, to: Workbench) {
        if let Workbench::Manufacturing(left) = from {
            self.manufacturing.views.insert(left, self.viewport);
        }
        match (from, to) {
            (_, Workbench::Manufacturing(entered)) => {
                if !matches!(from, Workbench::Manufacturing(_)) {
                    self.manufacturing.drawing_view = Some(self.viewport);
                    self.manufacturing.drawing_tab = Some(self.panel_tabs.get().active());
                }
                let canvas = self.viewport.canvas_size();
                self.viewport = self
                    .manufacturing
                    .views
                    .get(&entered)
                    .copied()
                    .unwrap_or_else(|| {
                        let mut view = Viewport::new();
                        view.resize(canvas.0, canvas.1);
                        view
                    });
                if !self.manufacturing.views.contains_key(&entered) {
                    self.fit_output_page_of(entered);
                }
                self.press_panel_tab(PanelTab::Workbench.name());
            }
            (Workbench::Manufacturing(_), _) => {
                let canvas = self.viewport.canvas_size();
                if let Some(view) = self.manufacturing.drawing_view.take() {
                    self.viewport = view;
                    self.viewport.resize(canvas.0, canvas.1);
                }
                let tab = self
                    .manufacturing
                    .drawing_tab
                    .take()
                    .unwrap_or(PanelTab::Document);
                // A tab that cannot be pressed now (Style with nothing in scope) gives Document.
                if !self.press_panel_tab(tab.name()) {
                    self.press_panel_tab(PanelTab::Document.name());
                }
            }
            _ => {}
        }
    }

    /// Fit page (the View card): shows the active output page whole.
    pub fn fit_output_page(&mut self) {
        if let Workbench::Manufacturing(id) = self.workbench() {
            self.fit_output_page_of(id);
        }
    }

    fn fit_output_page_of(&mut self, id: NodeId) {
        let Some(manufacturing) = self.document.manufacturing(id) else {
            return;
        };
        let area = manufacturing.work_area();
        self.viewport.fit_area(
            Point::new(0.0, 0.0),
            Point::new(area.width.as_mm(), area.height.as_mm()),
            FIT_FREE_LEFT_PX,
            FIT_FREE_TOP_PX,
        );
    }

    /// After an undo or redo: the active manufacturing that is gone hands over to the workbench
    /// it was added from; a manufacturing deleted from its own workbench that is back opens it
    /// again (UX notes §2, §8).
    pub(super) fn after_history_manufacturing(&mut self) {
        if let Workbench::Manufacturing(id) = self.workbench()
            && !self.manufacturing_available(id)
        {
            let back = match self.manufacturing.return_to {
                Workbench::Manufacturing(other) if !self.manufacturing_available(other) => {
                    Workbench::Vector
                }
                other => other,
            };
            self.manufacturing.deleted_from = None;
            self.set_workbench(back);
            return;
        }
        if let Some(id) = self.manufacturing.deleted_from
            && self.manufacturing_available(id)
        {
            self.manufacturing.deleted_from = None;
            self.set_workbench(Workbench::Manufacturing(id));
        }
    }

    /// Switches the extension `id` on or off (criterion 11; not an undo step). A manufacturing
    /// workbench of a process switched off hands over to Vector. Returns the settings file's new
    /// text for the host to keep.
    pub fn set_extension_enabled(&mut self, id: &str, enabled: bool) -> String {
        self.manufacturing.extensions.set_enabled(id, enabled);
        if let Workbench::Manufacturing(active) = self.workbench()
            && !self.manufacturing_available(active)
        {
            self.set_workbench(Workbench::Vector);
        }
        self.manufacturing.extensions.to_text()
    }

    /// Reads the per-user settings file `extensions.toml` the host kept (ADR 0034 §6).
    pub fn load_extension_settings(&mut self, text: &str) {
        self.manufacturing.extensions = ExtensionSettings::parse(text);
        if let Workbench::Manufacturing(active) = self.workbench()
            && !self.manufacturing_available(active)
        {
            self.set_workbench(Workbench::Vector);
        }
    }

    /// The last read-only notice, for the host.
    #[must_use]
    pub const fn read_only_notice(&self) -> Option<ReadOnlyNotice> {
        self.manufacturing.notice
    }

    /// Shows the read-only notice at `at` (a press, in document mm) or at the canvas (a key).
    pub(super) fn show_read_only_notice(&mut self, at: Option<Point>) {
        let seq = self
            .manufacturing
            .notice
            .map_or(1, |n| n.seq.wrapping_add(1));
        let at = at.map(|point| self.view().document_to_screen(point));
        self.manufacturing.notice = Some(ReadOnlyNotice { seq, at });
    }

    /// A press on the canvas of a manufacturing workbench (criterion 7): nothing is selected or
    /// changed; a press on the placed content or the output page (where the fixtures lie) shows
    /// the read-only notice.
    pub(super) fn manufacturing_pointer_down(&mut self, point: Point) {
        let Some(manufacturing) = self.active_manufacturing() else {
            return;
        };
        let area = manufacturing.work_area();
        let on_page = (0.0..=area.width.as_mm()).contains(&point.x)
            && (0.0..=area.height.as_mm()).contains(&point.y);
        let on_content = self
            .placed_bounds_of(&manufacturing)
            .is_some_and(|(min, max)| {
                (min.x..=max.x).contains(&point.x) && (min.y..=max.y).contains(&point.y)
            });
        if on_page || on_content {
            self.show_read_only_notice(Some(point));
        }
    }

    /// A key in a manufacturing workbench (UX notes §3, §5): undo and redo act; Delete,
    /// Backspace, Ctrl+A, a nudge key and Ctrl+V show the read-only notice; every other key,
    /// the drawing tools' letters included, does nothing.
    pub(super) fn manufacturing_key(
        &mut self,
        input: &super::KeyInput<'_>,
    ) -> Option<super::KeyOutcome> {
        if input.dom_blocked {
            return Some(super::KeyOutcome::Ignored);
        }
        let key = input.key.to_ascii_lowercase();
        if input.ctrl && matches!(key.as_str(), "z" | "y") {
            return None;
        }
        let edit = matches!(
            input.key,
            "Delete" | "Backspace" | "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
        ) || (input.ctrl && matches!(key.as_str(), "a" | "v"));
        if edit {
            self.show_read_only_notice(None);
            return Some(super::KeyOutcome::Consumed);
        }
        Some(super::KeyOutcome::Ignored)
    }
}
